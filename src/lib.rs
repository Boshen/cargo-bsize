//! # cargo-bsize
//!
//! Analyze Rust binary size and propose size-reducing changes.

pub mod assembly;
pub mod build;
pub mod diff;
pub mod dupdata;
pub mod duplicates;
pub mod dwarf;
pub mod inlined;
pub mod name;
pub mod output;
pub mod overhead;
pub mod sections;
pub mod symbols;
#[cfg(test)]
mod tests;
pub mod types;

use std::{
    env,
    ffi::OsString,
    fs,
    io::Write,
    path::PathBuf,
    process::{Command, ExitCode},
};

use anyhow::{Context, Result, anyhow};
use bpaf::Bpaf;
use cargo_metadata::{Metadata, MetadataCommand};

use crate::output::OutputFormat;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// How many entries each ranked list keeps unless `--limit` says otherwise.
const DEFAULT_LIMIT: usize = 20;

/// Analyze Rust binary size and propose size-reducing changes.
///
/// `options("bsize")` pulls in bpaf's `cargo_helper`, so `cargo bsize` and
/// `cargo-bsize` parse identically.
#[derive(Debug, Clone, Bpaf)]
#[bpaf(options("bsize"), version(VERSION))]
pub struct CargoBsizeOptions {
    /// Binary to analyze, required when the workspace has more than one.
    #[bpaf(long, argument("NAME"))]
    bin: Option<String>,

    /// Output format: text, json
    #[bpaf(long, fallback(OutputFormat::Text), display_fallback)]
    format: OutputFormat,

    /// How many entries to keep in each ranked list.
    #[bpaf(long, argument("N"), fallback(DEFAULT_LIMIT), display_fallback)]
    limit: usize,

    /// Compare against a previously-built binary and report what grew.
    #[bpaf(long, argument("PATH"))]
    baseline: Option<PathBuf>,

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
        Self {
            bin: None,
            format: OutputFormat::default(),
            limit: DEFAULT_LIMIT,
            baseline: None,
            locked: false,
            offline: false,
            frozen: false,
            path,
        }
    }

    #[must_use]
    pub const fn with_format(mut self, format: OutputFormat) -> Self {
        self.format = format;
        self
    }

    /// The resolution flags to forward to every cargo invocation.
    fn cargo_flags(&self) -> Vec<&'static str> {
        [(self.locked, "--locked"), (self.offline, "--offline"), (self.frozen, "--frozen")]
            .into_iter()
            .filter_map(|(enabled, flag)| enabled.then_some(flag))
            .collect()
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

        let mut report = output::Report {
            duplicates,
            binary: None,
            symbols: None,
            overhead: None,
            types: None,
            dupdata: None,
            inlined: None,
            assembly: None,
            diff: None,
        };

        if let Some(bin) = build::select_bin(&metadata, self.options.bin.as_deref())? {
            let target_dir = metadata.target_directory.join("bsize");
            let target_dir = target_dir.as_std_path();
            let flags = self.options.cargo_flags();
            let build::Build { executable, assembly } =
                build::release(&self.options.path, target_dir, &bin, &flags)?;

            let data = fs::read(&executable)
                .with_context(|| format!("failed to read {}", executable.display()))?;
            let file = object::File::parse(&*data)
                .with_context(|| format!("failed to parse {}", executable.display()))?;
            let workspace = metadata.workspace_root.as_std_path();

            // DWARF is the only place inlined code is named and exact static
            // sizes live. Reading it is best-effort — a project may strip it, or
            // `dsymutil` may be missing — so a failure only costs those views.
            // It is produced once and shared by the type and inlined analyses.
            let debug = dwarf::debug_object(&executable, file.format(), target_dir).ok();
            let limit = self.options.limit;
            let types = debug.as_deref().and_then(|debug| types::analyze(debug, limit).ok());
            let static_sizes = types.as_ref().map(|t| t.static_sizes.clone()).unwrap_or_default();

            report.binary = Some(sections::analyze(&file, &executable, data.len() as u64));
            report.symbols = Some(symbols::analyze(&file, &static_sizes, limit));
            report.overhead = Some(overhead::analyze(&file, &static_sizes));
            report.dupdata = Some(dupdata::analyze(&file, &static_sizes, limit));
            report.types = types.map(|t| t.report);
            report.inlined =
                debug.as_deref().and_then(|debug| inlined::analyze(debug, workspace, limit).ok());

            // Likewise the assembly: best-effort, since it may not be found in
            // `deps/` or the target may be one this parser does not know.
            report.assembly = assembly::analyze(&file, &assembly, workspace, limit).ok();

            if let Some(baseline) = &self.options.baseline {
                report.diff = diff::analyze(&file, baseline, limit).ok();
            }
        }

        output::render(&mut self.writer, &report, self.options.format, self.options.limit)?;
        Ok(())
    }

    /// `--filter-platform` keeps the graph to what links on this host; without
    /// it, every platform's target-specific dependencies show up at once.
    fn metadata(&self) -> Result<Metadata> {
        let mut other_options = vec!["--filter-platform".to_owned(), host_triple()?];
        other_options.extend(self.options.cargo_flags().into_iter().map(str::to_owned));

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
