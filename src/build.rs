//! Build the release binaries to analyze.
//!
//! Debug info is forced on and stripping off so the symbol table survives for
//! attribution. That changes the profile fingerprint, so the build goes to its
//! own target directory rather than invalidating the project's `target/release`.

use std::{
    io::BufReader,
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

/// Build one bin target in release mode and return the executable it produced.
///
/// # Errors
///
/// Errors when cargo cannot be spawned or the build fails.
pub fn release_executable(
    path: &Path,
    target_dir: &Path,
    bin: &BinTarget,
    flags: &[&str],
) -> Result<PathBuf> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut child = Command::new(cargo)
        .current_dir(path)
        .env("CARGO_TARGET_DIR", target_dir)
        .env("CARGO_PROFILE_RELEASE_DEBUG", "2")
        .env("CARGO_PROFILE_RELEASE_STRIP", "none")
        .args(["build", "--release", "--message-format=json-render-diagnostics"])
        .args(["--package", &bin.package, "--bin", &bin.name])
        .args(flags)
        .stdout(Stdio::piped())
        .spawn()
        .context("failed to run `cargo build`")?;

    let stdout = child.stdout.take().ok_or_else(|| anyhow!("`cargo build` produced no stdout"))?;
    // Build scripts are also reported as artifacts with an executable, so match
    // the bin target by name rather than taking the first executable seen.
    let executable =
        Message::parse_stream(BufReader::new(stdout)).filter_map(Result::ok).find_map(|message| {
            match message {
                Message::CompilerArtifact(artifact)
                    if artifact.target.is_bin() && artifact.target.name == bin.name =>
                {
                    artifact.executable.map(cargo_metadata::camino::Utf8PathBuf::into_std_path_buf)
                }
                _ => None,
            }
        });

    let status = child.wait().context("failed to wait for `cargo build`")?;
    if !status.success() {
        bail!("`cargo build --release` failed with {status}");
    }

    executable.ok_or_else(|| anyhow!("`cargo build` produced no executable for `{}`", bin.name))
}
