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
use cargo_metadata::{Artifact, Message};

/// Build `path` in release mode and return the executables it produced.
///
/// # Errors
///
/// Errors when cargo cannot be spawned or the build fails.
pub fn release_executables(path: &Path, target_dir: &Path, flags: &[&str]) -> Result<Vec<PathBuf>> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut child = Command::new(cargo)
        .current_dir(path)
        .env("CARGO_TARGET_DIR", target_dir)
        .env("CARGO_PROFILE_RELEASE_DEBUG", "2")
        .env("CARGO_PROFILE_RELEASE_STRIP", "none")
        .args(["build", "--release", "--message-format=json-render-diagnostics"])
        .args(flags)
        .stdout(Stdio::piped())
        .spawn()
        .context("failed to run `cargo build`")?;

    let stdout = child.stdout.take().ok_or_else(|| anyhow!("`cargo build` produced no stdout"))?;
    let executables: Vec<PathBuf> = Message::parse_stream(BufReader::new(stdout))
        .filter_map(Result::ok)
        .filter_map(|message| match message {
            Message::CompilerArtifact(Artifact { executable: Some(path), .. }) => {
                Some(path.into_std_path_buf())
            }
            _ => None,
        })
        .collect();

    let status = child.wait().context("failed to wait for `cargo build`")?;
    if !status.success() {
        bail!("`cargo build --release` failed with {status}");
    }

    Ok(executables)
}
