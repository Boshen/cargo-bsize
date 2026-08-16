//! Build the release binary to analyze.
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
use cargo_metadata::{Message, Metadata};

/// One bin target, named the way cargo needs to build just that one.
pub struct BinTarget {
    pub package: String,
    pub name: String,
}

/// What one build produced.
pub struct Build {
    pub executable: PathBuf,

    /// The assembly rustc emitted for the final crate: one file, or one per
    /// codegen unit. Empty when none could be found.
    pub assembly: Vec<PathBuf>,
}

/// Pick the bin target to analyze. `None` means the workspace has no binaries,
/// which is not an error — a library-only workspace still gets the rest of the
/// analysis.
///
/// # Errors
///
/// Errors when `requested` names no bin target, or when the workspace has
/// several and none was requested.
pub fn select_bin(metadata: &Metadata, requested: Option<&str>) -> Result<Option<BinTarget>> {
    let mut bins: Vec<BinTarget> = Vec::new();
    for package in metadata.workspace_packages() {
        for target in package.targets.iter().filter(|target| target.is_bin()) {
            bins.push(BinTarget {
                package: package.name.as_str().to_owned(),
                name: target.name.clone(),
            });
        }
    }
    bins.sort_by(|a, b| a.name.cmp(&b.name));

    let available = bins.iter().map(|bin| bin.name.as_str()).collect::<Vec<_>>().join(", ");

    if let Some(name) = requested {
        return bins
            .into_iter()
            .find(|bin| bin.name == name)
            .map(Some)
            .ok_or_else(|| anyhow!("no bin target named `{name}`; available: {available}"));
    }

    match bins.len() {
        0 => Ok(None),
        1 => Ok(bins.pop()),
        _ => bail!("workspace has several bin targets, pick one with --bin: {available}"),
    }
}

/// Build one bin target in release mode.
///
/// # Errors
///
/// Errors when cargo cannot be spawned or the build fails.
pub fn release(path: &Path, target_dir: &Path, bin: &BinTarget, flags: &[&str]) -> Result<Build> {
    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut child = Command::new(cargo)
        .current_dir(path)
        .env("CARGO_TARGET_DIR", target_dir)
        .env("CARGO_PROFILE_RELEASE_DEBUG", "2")
        .env("CARGO_PROFILE_RELEASE_STRIP", "none")
        .args(["rustc", "--release", "--message-format=json-render-diagnostics"])
        .args(["--package", &bin.package, "--bin", &bin.name])
        .args(flags)
        // For the final crate only, beside its object file in `deps/`.
        .args(["--", "--emit=asm"])
        .stdout(Stdio::piped())
        .spawn()
        .context("failed to run `cargo rustc`")?;

    let stdout = child.stdout.take().ok_or_else(|| anyhow!("`cargo rustc` produced no stdout"))?;

    // Read every line to the end: stopping early leaves cargo writing into a
    // closed pipe, which kills the build with a broken pipe.
    let mut executable = None;
    for line in BufReader::new(stdout).lines() {
        let line = line.context("failed to read `cargo rustc` output")?;

        // Build scripts are also reported as artifacts with an executable, so
        // match the bin target by name rather than taking the first one seen.
        if let Ok(Message::CompilerArtifact(artifact)) = serde_json::from_str::<Message>(&line)
            && artifact.target.is_bin()
            && artifact.target.name == bin.name
            && let Some(path) = artifact.executable
        {
            executable = Some(path.into_std_path_buf());
        }
    }

    let status = child.wait().context("failed to wait for `cargo rustc`")?;
    if !status.success() {
        bail!("`cargo rustc --release` failed with {status}");
    }

    let executable = executable
        .ok_or_else(|| anyhow!("`cargo rustc` produced no executable for `{}`", bin.name))?;
    let assembly = assembly_files(&executable, &bin.name);

    Ok(Build { executable, assembly })
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
fn assembly_files(executable: &Path, bin: &str) -> Vec<PathBuf> {
    let Some(deps) = executable.parent().map(|dir| dir.join("deps")) else { return Vec::new() };
    let Ok(wanted) = fs::read(executable) else { return Vec::new() };
    let mut entries: Vec<PathBuf> =
        fs::read_dir(deps).into_iter().flatten().flatten().map(|entry| entry.path()).collect();
    entries.sort();

    let name = |path: &Path| path.file_name().and_then(|name| name.to_str()).map(str::to_owned);
    let prefix = format!("{}-", bin.replace('-', "_"));
    let units = entries.iter().filter(|path| {
        path.extension() == executable.extension()
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
        let single = format!("{stem}.s");
        let per_unit = format!("{stem}.");
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
