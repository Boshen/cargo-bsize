//! # cargo-bsize
//!
//! Analyze Rust binary size and propose size-reducing changes.
//!
//! Today it reports crates that resolve to more than one version, since every
//! extra version is compiled and linked separately.

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
/// Parsed from `argv` by `bpaf`. The doc comments on individual fields become
/// the `--help` text users see, so keep them user-facing.
///
/// The "batteries" `cargo_helper` strips the leading `bsize` argument when
/// invoked as `cargo bsize`, so this struct sees the same shape either way.
/// See <https://docs.rs/bpaf/latest/bpaf/batteries/fn.cargo_helper.html>.
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

    /// Path to the project directory.
    ///
    /// Defaults to the current directory if not specified.
    #[bpaf(positional("PATH"), fallback_with(default_path))]
    path: PathBuf,
}

impl CargoBsizeOptions {
    /// Construct options with every flag at its default and the project rooted
    /// at `path`.
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        Self { format: OutputFormat::default(), locked: false, offline: false, frozen: false, path }
    }

    /// Pick the format results are rendered in.
    #[must_use]
    pub const fn with_format(mut self, format: OutputFormat) -> Self {
        self.format = format;
        self
    }
}

fn default_path() -> Result<PathBuf> {
    env::current_dir().context("failed to read the current directory")
}

/// Top-level entry point: resolves the dependency graph, runs the analyses,
/// and renders the report to `writer`.
pub struct CargoBsize<W> {
    /// Sink for the rendered report (typically `std::io::stdout()`).
    writer: W,

    /// Caller-supplied configuration; immutable once `run` starts.
    options: CargoBsizeOptions,
}

impl<W: Write> CargoBsize<W> {
    /// Build a runner that will write its report to `writer`.
    ///
    /// ```
    /// use cargo_bsize::{CargoBsize, CargoBsizeOptions};
    /// use std::path::PathBuf;
    ///
    /// let options = CargoBsizeOptions::new(PathBuf::from("."));
    /// let bsize = CargoBsize::new(std::io::stdout(), options);
    /// ```
    #[must_use]
    pub const fn new(writer: W, options: CargoBsizeOptions) -> Self {
        Self { writer, options }
    }

    /// Run every analysis and render the report.
    ///
    /// Returns:
    /// - `0` whether or not anything was found — this reports on a build, it
    ///   does not gate one.
    /// - `2` if a fatal error occurred (cargo failure, IO error, ...).
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

    /// Resolve the dependency graph for one concrete build.
    ///
    /// `--filter-platform` keeps the graph to what actually links on this host;
    /// without it, target-specific dependencies for every platform show up and
    /// duplicates get reported that no single build ever contains.
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

/// Ask rustc which target it builds for by default.
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
