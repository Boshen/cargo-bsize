//! # cargo-bsize
//!
//! Analyze Rust binary size and propose size-reducing changes.

pub mod duplicates;
pub mod output;
#[cfg(test)]
mod tests;

use std::{
    env,
    ffi::OsString,
    io::Write,
    path::PathBuf,
    process::{Command, ExitCode},
};

use anyhow::{Context, Result, anyhow};
use bpaf::Bpaf;
use cargo_metadata::{Metadata, MetadataCommand};

use crate::output::OutputFormat;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Analyze Rust binary size and propose size-reducing changes.
///
/// `options("bsize")` pulls in bpaf's `cargo_helper`, so `cargo bsize` and
/// `cargo-bsize` parse identically.
#[derive(Debug, Clone, Bpaf)]
#[bpaf(options("bsize"), version(VERSION))]
pub struct CargoBsizeOptions {
    /// Output format: text, json
    #[bpaf(long, fallback(OutputFormat::Text), display_fallback)]
    format: OutputFormat,

    /// Assert that `Cargo.lock` will remain unchanged.
    locked: bool,

    /// Run without accessing the network
    offline: bool,

    /// Equivalent to specifying both --locked and --offline
    frozen: bool,

    /// Path to the project directory, defaulting to the current directory.
    #[bpaf(positional("PATH"), fallback_with(default_path))]
    path: PathBuf,
}

impl CargoBsizeOptions {
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        Self { format: OutputFormat::default(), locked: false, offline: false, frozen: false, path }
    }

    #[must_use]
    pub const fn with_format(mut self, format: OutputFormat) -> Self {
        self.format = format;
        self
    }
}

fn default_path() -> Result<PathBuf> {
    env::current_dir().context("failed to read the current directory")
}

/// Runs the analyses and renders the report to `writer`.
pub struct CargoBsize<W> {
    writer: W,
    options: CargoBsizeOptions,
}

impl<W: Write> CargoBsize<W> {
    #[must_use]
    pub const fn new(writer: W, options: CargoBsizeOptions) -> Self {
        Self { writer, options }
    }

    /// Returns `0` whatever is found — this reports on a build, it does not gate
    /// one — or `2` on a fatal error.
    #[must_use]
    pub fn run(mut self) -> ExitCode {
        match self.analyze() {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                let _ = writeln!(self.writer, "error: {err:?}");
                ExitCode::from(2)
            }
        }
    }

    fn analyze(&mut self) -> Result<()> {
        let metadata = self.metadata()?;
        let duplicates = duplicates::find(&metadata)?;
        output::render(&mut self.writer, &duplicates, self.options.format)?;
        Ok(())
    }

    /// `--filter-platform` keeps the graph to what links on this host; without
    /// it, every platform's target-specific dependencies show up at once.
    fn metadata(&self) -> Result<Metadata> {
        let mut other_options = vec!["--filter-platform".to_owned(), host_triple()?];
        if self.options.locked {
            other_options.push("--locked".to_owned());
        }
        if self.options.offline {
            other_options.push("--offline".to_owned());
        }
        if self.options.frozen {
            other_options.push("--frozen".to_owned());
        }

        MetadataCommand::new()
            .current_dir(&self.options.path)
            .other_options(other_options)
            .exec()
            .map_err(|err| anyhow!("failed to run `cargo metadata`: {err}"))
    }
}

fn host_triple() -> Result<String> {
    let rustc = env::var_os("RUSTC").unwrap_or_else(|| OsString::from("rustc"));
    let output = Command::new(&rustc)
        .arg("-vV")
        .output()
        .with_context(|| format!("failed to run `{} -vV`", rustc.to_string_lossy()))?;
    let stdout = String::from_utf8(output.stdout).context("`rustc -vV` printed invalid UTF-8")?;

    stdout
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .map(str::to_owned)
        .context("`rustc -vV` did not report a host triple")
}
