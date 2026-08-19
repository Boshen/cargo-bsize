//! Build the linked target to analyze.
//!
//! Debug info is forced on and stripping off so the symbol table survives for
//! attribution. That changes the profile fingerprint, so the build goes to its
//! own target directory rather than invalidating the project's `target/release`.
//!
//! The final crate also emits its assembly. Under `lto = "fat"` that is the
//! whole program after link-time optimization, so it is asked of the final
//! crate alone through `cargo rustc`; a dependency's own assembly is discarded
//! by LTO and would only cost time to write.

use std::{
    env, fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{Context, Result, anyhow, bail};
use cargo_metadata::{Artifact, Message, Metadata, Target};
use object::{Object, ObjectKind};

/// The Cargo target kinds that produce a linked artifact we can analyze.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    Bin,
    Cdylib,
}

/// One linked target, named the way Cargo needs to build just that one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildTarget {
    pub package: String,
    pub name: String,
    pub kind: TargetKind,
}

impl BuildTarget {
    /// Cargo's selector for this target. A `cdylib` is one crate type of the
    /// package's library target, so Cargo selects it with `--lib`.
    pub fn cargo_args(&self) -> Vec<&str> {
        match self.kind {
            TargetKind::Bin => vec!["--bin", &self.name],
            TargetKind::Cdylib => vec!["--lib"],
        }
    }

    /// Whether a compiler artifact belongs to this target.
    fn matches(&self, target: &Target) -> bool {
        target.name == self.name
            && match self.kind {
                TargetKind::Bin => target.is_bin(),
                TargetKind::Cdylib => target.is_cdylib(),
            }
    }

    /// The linked file Cargo reported for this target. Executables have a
    /// dedicated field; cdylibs are one of the target's filenames alongside
    /// its rlib and platform-specific linker files.
    pub fn linked_artifact(&self, artifact: &Artifact) -> Option<PathBuf> {
        if !self.matches(&artifact.target) {
            return None;
        }

        match self.kind {
            TargetKind::Bin => {
                artifact.executable.as_ref().map(|path| path.as_std_path().to_owned())
            }
            TargetKind::Cdylib => artifact
                .filenames
                .iter()
                .map(|path| path.as_std_path())
                .find(|path| is_dynamic_library(path))
                .map(Path::to_owned),
        }
    }
}

/// What one build produced.
pub struct Build {
    pub binary: PathBuf,

    /// The assembly rustc emitted for the final crate: one file, or one per
    /// codegen unit. Empty when none could be found.
    pub assembly: Vec<PathBuf>,

    /// The LLVM IR of every crate, when `--emit=llvm-ir` was requested. Empty
    /// otherwise.
    pub llvm_ir: Vec<PathBuf>,
}

/// Pick the linked target to analyze. `None` means the workspace has neither a
/// binary nor a cdylib, which is not an error — a library-only workspace still
/// gets the dependency analysis.
///
/// # Errors
///
/// Errors when a requested target does not exist, both selectors are used, or
/// when the workspace has several targets of the default kind.
pub fn select_target(
    metadata: &Metadata,
    requested_bin: Option<&str>,
    requested_cdylib: Option<&str>,
) -> Result<Option<BuildTarget>> {
    if requested_bin.is_some() && requested_cdylib.is_some() {
        bail!("--bin and --cdylib cannot be used together");
    }

    let mut bins = Vec::new();
    let mut cdylibs = Vec::new();
    for package in metadata.workspace_packages() {
        for target in &package.targets {
            let kind = if target.is_bin() {
                Some(TargetKind::Bin)
            } else if target.is_cdylib() {
                Some(TargetKind::Cdylib)
            } else {
                None
            };
            let Some(kind) = kind else { continue };
            let selected = BuildTarget {
                package: package.name.as_str().to_owned(),
                name: target.name.clone(),
                kind,
            };
            match kind {
                TargetKind::Bin => bins.push(selected),
                TargetKind::Cdylib => cdylibs.push(selected),
            }
        }
    }
    bins.sort_by(|a, b| a.name.cmp(&b.name));
    cdylibs.sort_by(|a, b| a.name.cmp(&b.name));

    if let Some(name) = requested_bin {
        let available = names(&bins);
        return bins
            .into_iter()
            .find(|target| target.name == name)
            .map(Some)
            .ok_or_else(|| anyhow!("no bin target named `{name}`; available: {available}"));
    }
    if let Some(name) = requested_cdylib {
        let available = names(&cdylibs);
        return cdylibs
            .into_iter()
            .find(|target| target.name == name)
            .map(Some)
            .ok_or_else(|| anyhow!("no cdylib target named `{name}`; available: {available}"));
    }

    if bins.is_empty() {
        match cdylibs.len() {
            0 => Ok(None),
            1 => Ok(cdylibs.pop()),
            _ => bail!(
                "workspace has several cdylib targets, pick one with --cdylib: {}",
                names(&cdylibs)
            ),
        }
    } else {
        match bins.len() {
            1 => Ok(bins.pop()),
            _ => bail!("workspace has several bin targets, pick one with --bin: {}", names(&bins)),
        }
    }
}

fn names(targets: &[BuildTarget]) -> String {
    targets.iter().map(|target| target.name.as_str()).collect::<Vec<_>>().join(", ")
}

/// Whether `path` is a linked dynamic library rather than an rlib, import
/// library, debug companion, or another filename Cargo reports for a cdylib.
fn is_dynamic_library(path: &Path) -> bool {
    let Ok(data) = fs::read(path) else { return false };
    let Ok(file) = object::File::parse(&*data) else { return false };
    file.kind() == ObjectKind::Dynamic
}

/// What every crate is asked to emit, beyond the final crate's assembly. Each
/// goes through `RUSTFLAGS`, so it reaches the whole program — which is what
/// makes these runs heavy and why each is opt-in — and each changes the
/// build's fingerprint, so the first run with it rebuilds.
#[derive(Debug, Default, Clone)]
pub struct Extras {
    /// `--emit=llvm-ir`: every crate's IR beside its object in `deps/`.
    pub llvm_ir: bool,

    /// `-Zdump-mono-stats`: every crate's monomorphization statistics, as JSON
    /// files in this directory. Nightly-only, so the build runs with
    /// `RUSTC_BOOTSTRAP=1`.
    pub mono_stats: Option<PathBuf>,

    /// `-Cremark=loop-unroll -Cremark=loop-vectorize -Zremark-dir`: every
    /// crate's loop remarks, as YAML files in this directory. Nightly-only for
    /// the directory, so the build runs with `RUSTC_BOOTSTRAP=1`.
    pub remarks: Option<PathBuf>,
}

impl Extras {
    /// The `RUSTFLAGS` these ask for.
    fn rustflags(&self) -> Vec<String> {
        let mut flags = Vec::new();
        if self.llvm_ir {
            flags.push("--emit=llvm-ir".to_owned());
        }
        if let Some(dir) = &self.mono_stats {
            flags.push(format!("-Zdump-mono-stats={}", dir.display()));
            flags.push("-Zdump-mono-stats-format=json".to_owned());
        }
        if let Some(dir) = &self.remarks {
            flags.push("-Cremark=loop-unroll".to_owned());
            flags.push("-Cremark=loop-vectorize".to_owned());
            flags.push(format!("-Zremark-dir={}", dir.display()));
        }
        flags
    }

    /// Whether a `-Z` flag is among them.
    fn nightly(&self) -> bool {
        self.mono_stats.is_some() || self.remarks.is_some()
    }
}

/// Build one linked target in release mode.
///
/// # Errors
///
/// Errors when cargo cannot be spawned or the build fails.
pub fn release(
    path: &Path,
    target_dir: &Path,
    target: &BuildTarget,
    flags: &[&str],
    extras: &Extras,
) -> Result<Build> {
    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut command = Command::new(cargo);
    command
        .current_dir(path)
        .env("CARGO_TARGET_DIR", target_dir)
        .env("CARGO_PROFILE_RELEASE_DEBUG", "2")
        .env("CARGO_PROFILE_RELEASE_STRIP", "none")
        .args(["rustc", "--release", "--message-format=json-render-diagnostics"])
        .args(["--package", &target.package])
        .args(target.cargo_args())
        .args(flags)
        // For the final crate only, beside its object file in `deps/`.
        .args(["--", "--emit=asm"])
        .stdout(Stdio::piped());

    // Whatever every crate is asked to emit goes through `RUSTFLAGS`, merged
    // with any the environment set.
    let extra = extras.rustflags();
    if !extra.is_empty() {
        command.env("RUSTFLAGS", rustflags_with(&extra));
    }
    if extras.nightly() {
        command.env("RUSTC_BOOTSTRAP", "1");
    }

    let mut child = command.spawn().context("failed to run `cargo rustc`")?;

    let stdout = child.stdout.take().ok_or_else(|| anyhow!("`cargo rustc` produced no stdout"))?;

    // Read every line to the end: stopping early leaves cargo writing into a
    // closed pipe, which kills the build with a broken pipe.
    let mut binary = None;
    for line in BufReader::new(stdout).lines() {
        let line = line.context("failed to read `cargo rustc` output")?;

        // Build scripts and dependencies are reported too, so match the
        // selected target rather than taking the first linked file seen.
        if let Ok(Message::CompilerArtifact(artifact)) = serde_json::from_str::<Message>(&line)
            && let Some(path) = target.linked_artifact(&artifact)
        {
            binary = Some(path);
        }
    }

    let status = child.wait().context("failed to wait for `cargo rustc`")?;
    if !status.success() {
        bail!("`cargo rustc --release` failed with {status}");
    }

    let binary = binary.ok_or_else(|| {
        anyhow!("`cargo rustc` produced no linked artifact for `{}`", target.name)
    })?;
    let assembly = assembly_files(&binary, target);
    let llvm_ir = if extras.llvm_ir { ir_files(&binary) } else { Vec::new() };

    Ok(Build { binary, assembly, llvm_ir })
}

/// `RUSTFLAGS` with `extra` appended to whatever the environment set, so every
/// crate gets them without dropping the caller's flags.
fn rustflags_with(extra: &[String]) -> String {
    let mut flags = env::var("RUSTFLAGS").unwrap_or_default();
    for flag in extra {
        if !flags.is_empty() {
            flags.push(' ');
        }
        flags.push_str(flag);
    }
    flags
}

/// Check-build the program with `-Zmacro-stats` in its own target directory,
/// capturing the statistics into one file per crate under `dir`.
///
/// A check suffices — expansion happens before codegen — and the separate
/// directory leaves the primary cache alone. On later runs only changed
/// crates recompile and print; the per-crate files keep the rest.
///
/// # Errors
///
/// Errors when cargo cannot be spawned or the check fails.
pub fn macro_stats(
    path: &Path,
    dir: &Path,
    target: &BuildTarget,
    flags: &[&str],
    host: &str,
) -> Result<()> {
    fs::create_dir_all(dir).with_context(|| format!("failed to create {}", dir.display()))?;

    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(cargo)
        .current_dir(path)
        .env("CARGO_TARGET_DIR", dir)
        .env("RUSTC_BOOTSTRAP", "1")
        .env("RUSTFLAGS", rustflags_with(&["-Zmacro-stats".to_owned()]))
        .args(["check", "--release"])
        .args(["--package", &target.package])
        .args(target.cargo_args())
        // An explicit target keeps the flag off build scripts and proc macros
        // — their expansions compile for the host, not into the binary.
        .args(["--target", host])
        .args(flags)
        .stdout(Stdio::null())
        .output()
        .context("failed to run `cargo check`")?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        bail!("`cargo check -Zmacro-stats` failed:\n{}", stderr);
    }

    crate::macros::persist(&stderr, dir)
}

/// Every `.ll` file rustc left in `deps/` — the IR of the whole program. Unlike
/// the assembly, this is not one unit's file but all of them, so it is a plain
/// directory listing.
fn ir_files(executable: &Path) -> Vec<PathBuf> {
    let Some(parent) = executable.parent() else { return Vec::new() };
    let Ok(entries) = fs::read_dir(parent.join("deps")) else { return Vec::new() };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "ll"))
        .collect();
    files.sort();
    files
}

/// The assembly rustc emitted for the final crate: `deps/<crate>-<hash>.s`, or
/// one `deps/<crate>-<hash>.<cgu>.rcgu.s` per codegen unit when there are
/// several.
///
/// The hash is cargo's for the unit, and stale ones linger from earlier flags.
/// Nothing here can compute it, but cargo copies the unit's executable up out
/// of `deps/` unchanged, so the copy that matches byte for byte names the unit
/// — and the assembly beside it came from the same rustc run, whether that was
/// this build or the one cargo cached.
fn assembly_files(binary: &Path, target: &BuildTarget) -> Vec<PathBuf> {
    let Some(parent) = binary.parent() else { return Vec::new() };
    let Ok(wanted) = fs::read(binary) else { return Vec::new() };
    let Ok(entries) = fs::read_dir(parent.join("deps")) else { return Vec::new() };
    let mut entries: Vec<PathBuf> =
        entries.filter_map(Result::ok).map(|entry| entry.path()).collect();
    entries.sort();

    let name = |path: &Path| path.file_name().and_then(|name| name.to_str()).map(str::to_owned);
    let crate_name = target.name.replace('-', "_");
    let prefix = match target.kind {
        TargetKind::Bin => format!("{crate_name}-"),
        TargetKind::Cdylib => format!("lib{crate_name}"),
    };
    let units = entries.iter().filter(|path| {
        path.extension() == binary.extension()
            && path.file_stem().and_then(|stem| stem.to_str()).is_some_and(|stem| {
                stem.starts_with(&prefix) && !stem[prefix.len()..].contains('.')
            })
    });

    for unit in units {
        let same_size = fs::metadata(unit).is_ok_and(|meta| meta.len() == wanted.len() as u64);
        if !same_size || fs::read(unit).ok().as_deref() != Some(wanted.as_slice()) {
            continue;
        }

        let Some(stem) = unit.file_stem().and_then(|stem| stem.to_str()) else { continue };
        let rustc_stem = match target.kind {
            TargetKind::Bin => stem,
            TargetKind::Cdylib => stem.strip_prefix("lib").unwrap_or(stem),
        };
        let single = format!("{rustc_stem}.s");
        let per_unit = format!("{rustc_stem}.");
        let files: Vec<PathBuf> = entries
            .iter()
            .filter(|path| {
                name(path).is_some_and(|name| {
                    name == single || (name.starts_with(&per_unit) && name.ends_with(".rcgu.s"))
                })
            })
            .cloned()
            .collect();
        if !files.is_empty() {
            return files;
        }
    }

    Vec::new()
}
