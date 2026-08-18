//! # cargo-bsize
//!
//! Analyze Rust binary size and propose size-reducing changes.

pub mod assembly;
pub mod build;
pub mod categories;
pub mod constants;
pub mod diff;
pub mod dispatch;
pub mod dupdata;
pub mod duplicates;
pub mod dwarf;
pub mod features;
pub mod graph;
pub mod inlined;
pub mod instantiations;
pub mod llvm_ir;
pub mod name;
pub mod output;
pub mod overhead;
pub mod provenance;
pub mod relocations;
pub mod sections;
pub mod snippets;
pub mod symbols;
#[cfg(test)]
mod tests;
pub mod types;
pub mod whatif;

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
use rustc_hash::FxHashMap;

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

    /// Attribute LLVM IR to its generics (slow: a full rebuild, gigabytes of IR).
    llvm_ir: bool,

    /// Rebuild under each size lever and measure the saving (slow: a build each).
    what_if: bool,

    /// Which levers `--what-if` rebuilds with: `all`, or a comma-separated list
    /// (`opt-level=z,panic=abort,fmt-debug=none,build-std,min-size,…`).
    #[bpaf(long, argument("LEVERS"))]
    levers: Option<String>,

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
            llvm_ir: false,
            what_if: false,
            levers: None,
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
        // A misspelt lever should fail before the build, not after it.
        let levers = whatif::select(self.options.levers.as_deref())?;
        let metadata = self.metadata()?;
        let duplicates = duplicates::find(&metadata)?;

        let mut report = output::Report {
            instructions: output::AGENT_INSTRUCTIONS,
            duplicates,
            features: None,
            binary: None,
            symbols: None,
            instantiations: None,
            overhead: None,
            provenance: None,
            types: None,
            dupdata: None,
            dispatch: None,
            categories: None,
            inlined: None,
            assembly: None,
            constants: None,
            relocations: None,
            graph: None,
            diff: None,
            llvm_ir: None,
            whatif: None,
        };

        if let Some(bin) = build::select_bin(&metadata, self.options.bin.as_deref())? {
            let target_dir = metadata.target_directory.join("bsize");
            let target_dir = target_dir.as_std_path();
            let flags = self.options.cargo_flags();
            let build::Build { executable, assembly, llvm_ir } =
                build::release(&self.options.path, target_dir, &bin, &flags, self.options.llvm_ir)?;

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
            let types =
                debug.as_deref().and_then(|debug| types::analyze(debug, workspace, limit).ok());
            let mut sites = FxHashMap::default();
            let mut provenance = None;
            let (type_report, static_sizes) = match types {
                Some(types) => {
                    // The same walk read the compile units, which name each
                    // crate's checkout: a duplicated dependency's versions can
                    // be costed in bytes.
                    let read = provenance::analyze(
                        &file,
                        &types.units,
                        &types.functions,
                        workspace,
                        limit,
                    );
                    read.cost_duplicates(&mut report.duplicates);
                    provenance = Some(read);
                    sites = types.sites;
                    (Some(types.report), types.static_sizes)
                }
                None => (None, FxHashMap::default()),
            };
            report.features = features::analyze(&metadata, provenance.as_ref(), limit).ok();
            report.provenance = provenance.map(|provenance| provenance.report);

            report.binary = Some(sections::analyze(&file, &executable, data.len() as u64));
            report.symbols = Some(symbols::analyze(&file, &static_sizes, limit));
            report.overhead = Some(overhead::analyze(&file, &static_sizes));
            report.dupdata = Some(dupdata::analyze(&file, &static_sizes, limit));
            report.dispatch = Some(dispatch::analyze(&file, &static_sizes, limit));
            report.categories = Some(categories::analyze(&file, &static_sizes, limit));
            report.relocations = relocations::analyze(&file, limit);
            report.types = type_report;
            let inlines =
                debug.as_deref().and_then(|debug| inlined::analyze(debug, workspace, limit).ok());
            report.instantiations = Some(instantiations::analyze(
                &file,
                &static_sizes,
                inlines.as_ref().map_or(&[][..], |inlines| &inlines.functions),
                limit,
            ));
            report.inlined = inlines.map(|inlines| inlines.report);

            // Likewise the assembly: best-effort, since it may not be found in
            // `deps/` or the target may be one this parser does not know. The
            // reference graph reads what the same pass collected.
            if let Ok(analysis) = assembly::analyze(&file, &assembly, workspace, limit) {
                report.graph = Some(graph::analyze(analysis.edges, &analysis.sizes, limit));
                report.constants =
                    Some(constants::analyze(analysis.constants, &analysis.sizes, workspace, limit));
                report.assembly = Some(analysis.report);
            }

            if let Some(baseline) = &self.options.baseline {
                report.diff = diff::analyze(&file, baseline, limit).ok();
            }
            if self.options.llvm_ir {
                report.llvm_ir = llvm_ir::analyze(&llvm_ir, limit).ok();
            }
            // Every function the views name gets its definition site.
            provenance::attach(&mut report, &sites);

            if self.options.what_if
                && let Some(binary) = &report.binary
            {
                let primary = whatif::Primary { file: &file, shipped: binary.shipped };
                let host = host_triple()?;
                let job = whatif::Job {
                    path: &self.options.path,
                    target_dir,
                    bin: &bin,
                    flags: &flags,
                    host: &host,
                };
                report.whatif = Some(whatif::analyze(&job, &levers, &primary, limit));
            }
        }

        // The workspace rows that name lines get the text.
        snippets::attach(&mut report, metadata.workspace_root.as_std_path());

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
