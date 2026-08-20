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
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{Context, Result, anyhow, bail};
use cargo_metadata::{Artifact, Message, Metadata, Target};

/// The Cargo target kinds that produce a linked artifact we can analyze.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    Bin,
    Cdylib,
    Staticlib,
}

impl TargetKind {
    fn matches(self, target: &Target) -> bool {
        match self {
            Self::Bin => target.is_bin(),
            Self::Cdylib => target.is_cdylib(),
            Self::Staticlib => target.is_staticlib(),
        }
    }

    const fn flag(self) -> &'static str {
        match self {
            Self::Bin => "--bin",
            Self::Cdylib => "--cdylib",
            Self::Staticlib => "--staticlib",
        }
    }

    const fn cargo_flag(self) -> &'static str {
        match self {
            Self::Bin => "--bin",
            Self::Cdylib | Self::Staticlib => "--lib",
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Bin => "bin",
            Self::Cdylib => "cdylib",
            Self::Staticlib => "staticlib",
        }
    }
}

/// One linked target, named the way Cargo needs to build just that one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildTarget {
    pub package: String,
    pub name: String,
    pub kind: TargetKind,
}

impl BuildTarget {
    /// Cargo's package and target selector. A `cdylib` or `staticlib` is a
    /// crate type of the library target, so Cargo selects it with `--lib`.
    pub fn cargo_args(&self) -> impl Iterator<Item = &str> {
        ["--package", self.package.as_str(), self.kind.cargo_flag()]
            .into_iter()
            .chain((self.kind == TargetKind::Bin).then_some(self.name.as_str()))
    }

    fn matches(&self, target: &Target) -> bool {
        target.name == self.name && self.kind.matches(target)
    }

    /// The file Cargo reported for this target. Executables have a dedicated
    /// field; cdylibs and staticlibs are one of the target's filenames
    /// alongside its rlib and platform-specific linker files.
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
                .find(|path| is_cdylib(path))
                .map(Path::to_owned),
            TargetKind::Staticlib => artifact
                .filenames
                .iter()
                .map(|path| path.as_std_path())
                .find(|path| is_staticlib(path))
                .map(Path::to_owned),
        }
    }

    /// The linked image to analyze for the artifact Cargo produced. A bin or
    /// cdylib is already linked; a staticlib is an archive of unlinked objects,
    /// so it is linked into a shared library first — the way its consumer would
    /// link it — with every member kept and unresolved symbols allowed.
    ///
    /// # Errors
    ///
    /// Errors when a staticlib cannot be linked.
    pub fn linked_image(&self, artifact: &Path, target_dir: &Path) -> Result<PathBuf> {
        match self.kind {
            TargetKind::Bin | TargetKind::Cdylib => Ok(artifact.to_owned()),
            TargetKind::Staticlib => link_staticlib(artifact, target_dir),
        }
    }
}

/// Link a staticlib into a shared library beside the analysis cache, with the
/// whole archive kept so nothing is lost for want of a referrer, dead code
/// stripped the way a consumer's link would, and the symbols the archive
/// expects its host to provide left unresolved.
fn link_staticlib(archive: &Path, target_dir: &Path) -> Result<PathBuf> {
    if cfg!(windows) {
        bail!("linking a staticlib for analysis is not supported on Windows yet");
    }
    let stem = archive.file_stem().and_then(|stem| stem.to_str()).unwrap_or("staticlib");
    let extension = if cfg!(target_os = "macos") { "dylib" } else { "so" };
    let output = target_dir.join(format!("{stem}.{extension}"));

    let cc = env::var_os("CC").unwrap_or_else(|| "cc".into());
    let mut command = Command::new(&cc);
    command.arg("-shared").arg("-o").arg(&output);
    if cfg!(target_os = "macos") {
        command
            .arg("-Wl,-force_load")
            .arg(archive)
            .args(["-Wl,-dead_strip", "-Wl,-undefined,dynamic_lookup"]);
    } else {
        command
            .arg("-Wl,--whole-archive")
            .arg(archive)
            .args(["-Wl,--no-whole-archive", "-Wl,--gc-sections"]);
    }

    let status =
        command.status().with_context(|| format!("failed to run `{}`", cc.to_string_lossy()))?;
    if !status.success() {
        bail!("linking {} into a shared library failed with {status}", archive.display());
    }
    Ok(output)
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

/// Pick the linked target to analyze. Binaries remain the default when present;
/// a workspace with no binaries falls back to its cdylibs, then its staticlibs.
/// `None` means no kind exists, which is not an error — a library-only
/// workspace still gets the dependency analysis.
///
/// # Errors
///
/// Errors when a requested target does not exist, several selectors are used,
/// or when the workspace has several targets of the selected kind.
pub fn select_target(
    metadata: &Metadata,
    requested_bin: Option<&str>,
    requested_cdylib: Option<&str>,
    requested_staticlib: Option<&str>,
) -> Result<Option<BuildTarget>> {
    let (mut kind, requested) = match (requested_bin, requested_cdylib, requested_staticlib) {
        (Some(name), None, None) => (TargetKind::Bin, Some(name)),
        (None, Some(name), None) => (TargetKind::Cdylib, Some(name)),
        (None, None, Some(name)) => (TargetKind::Staticlib, Some(name)),
        (None, None, None) => (TargetKind::Bin, None),
        _ => bail!("--bin, --cdylib and --staticlib cannot be used together"),
    };

    let mut targets = targets_of_kind(metadata, kind);
    if requested.is_none() {
        for fallback in [TargetKind::Cdylib, TargetKind::Staticlib] {
            if !targets.is_empty() {
                break;
            }
            kind = fallback;
            targets = targets_of_kind(metadata, kind);
        }
    }
    let available = names(&targets);

    if let Some(name) = requested {
        return targets.into_iter().find(|target| target.name == name).map(Some).ok_or_else(|| {
            anyhow!("no {} target named `{name}`; available: {available}", kind.name())
        });
    }

    match targets.len() {
        0 => Ok(None),
        1 => Ok(targets.pop()),
        _ => bail!(
            "workspace has several {} targets, pick one with {}: {available}",
            kind.name(),
            kind.flag()
        ),
    }
}

fn targets_of_kind(metadata: &Metadata, kind: TargetKind) -> Vec<BuildTarget> {
    let mut targets = metadata
        .workspace_packages()
        .into_iter()
        .flat_map(|package| {
            package.targets.iter().filter(move |target| kind.matches(target)).map(move |target| {
                BuildTarget {
                    package: package.name.as_str().to_owned(),
                    name: target.name.clone(),
                    kind,
                }
            })
        })
        .collect::<Vec<_>>();
    targets.sort_by(|a, b| a.name.cmp(&b.name));
    targets
}

fn names(targets: &[BuildTarget]) -> String {
    targets.iter().map(|target| target.name.as_str()).collect::<Vec<_>>().join(", ")
}

/// Whether this is the native library among the rlib and platform-specific
/// linker files Cargo may report for the same target.
fn is_cdylib(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| matches!(extension, "dylib" | "so" | "dll"))
}

/// Whether this is the static archive among the files Cargo may report for
/// the same target.
fn is_staticlib(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| matches!(extension, "a" | "lib"))
}

/// Read Cargo's JSON stream to the end and return the selected linked artifact.
pub fn find_artifact<R: Read>(stdout: R, target: &BuildTarget) -> Result<Option<PathBuf>> {
    let mut binary = None;
    for line in BufReader::new(stdout).lines() {
        let line = line.context("failed to read cargo output")?;
        if let Ok(Message::CompilerArtifact(artifact)) = serde_json::from_str::<Message>(&line)
            && let Some(path) = target.linked_artifact(&artifact)
        {
            binary = Some(path);
        }
    }
    Ok(binary)
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

    // Read every line to the end: stopping early leaves cargo writing into a
    // closed pipe, which kills the build with a broken pipe.
    let stdout = child.stdout.take().ok_or_else(|| anyhow!("`cargo rustc` produced no stdout"))?;
    let binary = find_artifact(stdout, target)?;

    let status = child.wait().context("failed to wait for `cargo rustc`")?;
    if !status.success() {
        bail!("`cargo rustc --release` failed with {status}");
    }

    let artifact = binary.ok_or_else(|| {
        anyhow!("`cargo rustc` produced no linked artifact for `{}`", target.name)
    })?;
    let binary = target.linked_image(&artifact, target_dir)?;
    let assembly = assembly_files(&artifact, target);
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
fn ir_files(binary: &Path) -> Vec<PathBuf> {
    let Some(parent) = binary.parent() else { return Vec::new() };
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
/// Nothing here can compute it, but cargo copies the unit's artifact up out
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
        TargetKind::Cdylib | TargetKind::Staticlib => format!("lib{crate_name}"),
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
            TargetKind::Cdylib | TargetKind::Staticlib => stem.strip_prefix("lib").unwrap_or(stem),
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
