//! Measure, not guess, what a build lever would save.
//!
//! The other views point at levers — `panic="abort"` drops the unwind tables,
//! `opt-level="z"` trades speed for size. This one rebuilds the binary with each
//! lever set and reports the real change in shipped size, so the number is
//! measured rather than estimated. Each lever is a full build into its own
//! target directory (so the primary's cache is left intact), which is why it is
//! opt-in.

use std::{
    env,
    io::{BufRead, BufReader},
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{Context, Result, anyhow, bail};
use cargo_metadata::Message;
use serde::Serialize;

use crate::{build::BinTarget, sections};

#[derive(Debug, Serialize)]
pub struct WhatIfReport {
    pub levers: Vec<LeverResult>,
}

#[derive(Debug, Serialize)]
pub struct LeverResult {
    pub name: String,

    /// Shipped size now, and under the lever.
    pub before: u64,
    pub after: u64,
}

/// One profile setting to try, as the cargo env override that applies it.
const LEVERS: [(&str, &str, &str); 2] = [
    ("opt-level=\"z\"", "CARGO_PROFILE_RELEASE_OPT_LEVEL", "z"),
    ("panic=\"abort\"", "CARGO_PROFILE_RELEASE_PANIC", "abort"),
];

/// Rebuild `bin` under each lever and report the change in shipped size against
/// `before` (the primary build's shipped size).
///
/// A lever that fails to build (some crates reject `panic=abort`) is skipped
/// rather than sinking the report.
///
pub fn analyze(
    path: &Path,
    target_dir: &Path,
    bin: &BinTarget,
    flags: &[&str],
    before: u64,
) -> WhatIfReport {
    let mut levers = Vec::new();
    for (name, variable, value) in LEVERS {
        let target = target_dir.join(format!("whatif-{variable}"));
        if let Ok(after) = shipped(path, &target, bin, flags, (variable, value)) {
            levers.push(LeverResult { name: name.to_owned(), before, after });
        }
    }

    WhatIfReport { levers }
}

/// Build `bin` with one extra profile override and return its shipped size.
fn shipped(
    path: &Path,
    target_dir: &Path,
    bin: &BinTarget,
    flags: &[&str],
    (variable, value): (&str, &str),
) -> Result<u64> {
    let cargo = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut child = Command::new(cargo)
        .current_dir(path)
        .env("CARGO_TARGET_DIR", target_dir)
        .env("CARGO_PROFILE_RELEASE_DEBUG", "2")
        .env("CARGO_PROFILE_RELEASE_STRIP", "none")
        .env(variable, value)
        .args(["build", "--release", "--message-format=json-render-diagnostics"])
        .args(["--package", &bin.package, "--bin", &bin.name])
        .args(flags)
        .stdout(Stdio::piped())
        .spawn()
        .context("failed to run `cargo build`")?;

    let stdout = child.stdout.take().ok_or_else(|| anyhow!("`cargo build` produced no stdout"))?;
    let mut executable = None;
    for line in BufReader::new(stdout).lines() {
        let line = line.context("failed to read `cargo build` output")?;
        if let Ok(Message::CompilerArtifact(artifact)) = serde_json::from_str::<Message>(&line)
            && artifact.target.is_bin()
            && artifact.target.name == bin.name
            && let Some(path) = artifact.executable
        {
            executable = Some(path.into_std_path_buf());
        }
    }

    let status = child.wait().context("failed to wait for `cargo build`")?;
    if !status.success() {
        bail!("`cargo build` failed with {status}");
    }

    let executable = executable.ok_or_else(|| anyhow!("no executable for `{}`", bin.name))?;
    let data = std::fs::read(&executable)
        .with_context(|| format!("failed to read {}", executable.display()))?;
    let file = object::File::parse(&*data)
        .with_context(|| format!("failed to parse {}", executable.display()))?;

    Ok(sections::analyze(&file, &executable, data.len() as u64).shipped)
}
