//! Rendering of analysis results.

use std::{fmt, io, str::FromStr};

use serde::Serialize;

use crate::{
    assembly::{AssemblyReport, COPY_RUN, Caller, Copies, Formatting, Identical, Line, Panics},
    categories::CategoryReport,
    constants::{self, ConstantsReport},
    diff::{DiffReport, NamedDelta},
    dispatch::DispatchReport,
    dupdata::DupDataReport,
    duplicates::Duplicate,
    features::FeatureReport,
    graph::GraphReport,
    inlined::{CallSite, InlineReport},
    instantiations::InstantiationReport,
    llvm_ir::IrReport,
    mono::MonoReport,
    overhead::OverheadReport,
    provenance::ProvenanceReport,
    relocations::RelocationReport,
    remarks::RemarksReport,
    sections::BinaryReport,
    symbols::{Group, Symbol, SymbolReport},
    types::TypeReport,
    whatif::{self, WhatIfReport},
};

/// What an agent reading this report is meant to do with it.
pub const AGENT_INSTRUCTIONS: &str =
    "Only propose source-code changes. Do not propose configuration changes.";

/// An object rather than a bare array, so later analyses can be added without
/// breaking the schema.
#[derive(Debug, Serialize)]
pub struct Report {
    /// Standing instructions for an agent consuming the report.
    pub instructions: &'static str,

    pub duplicates: Vec<Duplicate>,
    pub features: Option<FeatureReport>,
    pub binary: Option<BinaryReport>,
    pub symbols: Option<SymbolReport>,
    pub instantiations: Option<InstantiationReport>,
    pub overhead: Option<OverheadReport>,
    pub provenance: Option<ProvenanceReport>,
    pub dupdata: Option<DupDataReport>,
    pub dispatch: Option<DispatchReport>,
    pub categories: Option<CategoryReport>,
    pub types: Option<TypeReport>,
    pub inlined: Option<InlineReport>,
    pub assembly: Option<AssemblyReport>,
    pub constants: Option<ConstantsReport>,
    pub relocations: Option<RelocationReport>,
    pub graph: Option<GraphReport>,
    pub diff: Option<DiffReport>,
    pub llvm_ir: Option<IrReport>,
    pub mono: Option<MonoReport>,
    pub remarks: Option<RemarksReport>,
    pub whatif: Option<WhatIfReport>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

impl FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("text") {
            Ok(Self::Text)
        } else if s.eq_ignore_ascii_case("json") {
            Ok(Self::Json)
        } else {
            Err(format!("unknown format: {s}, expected: text, json"))
        }
    }
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Text => "text",
            Self::Json => "json",
        })
    }
}

/// # Errors
///
/// Errors when writing to `writer` fails.
pub fn render<W: io::Write>(
    writer: &mut W,
    report: &Report,
    format: OutputFormat,
    limit: usize,
) -> io::Result<()> {
    match format {
        OutputFormat::Text => render_text(writer, report, limit),
        OutputFormat::Json => {
            serde_json::to_writer_pretty(&mut *writer, report).map_err(io::Error::other)?;
            writeln!(writer)
        }
    }
}

fn render_text<W: io::Write>(writer: &mut W, report: &Report, limit: usize) -> io::Result<()> {
    writeln!(writer, "agent instructions")?;
    writeln!(writer, "  {}", report.instructions)?;
    writeln!(writer)?;

    if let Some(binary) = &report.binary {
        render_binary(writer, binary, limit)?;

        if let Some(diff) = &report.diff {
            render_diff(writer, diff, binary.shipped)?;
        }

        if let Some(symbols) = &report.symbols {
            render_symbols(
                writer,
                symbols,
                &report.duplicates,
                report.provenance.as_ref(),
                binary.shipped,
            )?;
        }

        if let Some(features) = &report.features {
            render_features(writer, features, binary.shipped)?;
        }

        if let Some(instantiations) = &report.instantiations {
            render_instantiations(writer, instantiations, binary.shipped)?;
        }

        if let Some(overhead) = &report.overhead {
            render_overhead(writer, overhead, binary.shipped)?;
        }

        if let Some(dupdata) = &report.dupdata {
            render_dupdata(writer, dupdata, binary.shipped)?;
        }

        if let Some(dispatch) = &report.dispatch {
            render_dispatch(writer, dispatch, binary.shipped)?;
        }

        if let Some(graph) = &report.graph {
            render_graph(writer, graph, binary.shipped)?;
        }

        if let Some(categories) = &report.categories {
            render_categories(writer, categories, binary.shipped)?;
        }

        if let Some(types) = &report.types {
            render_types(writer, types)?;
        }

        if let Some(inlined) = &report.inlined {
            render_inlined(writer, inlined, binary.shipped)?;
        }

        if let Some(assembly) = &report.assembly {
            render_assembly(writer, assembly, binary.shipped)?;
        }

        if let (Some(constants), Some(symbols)) = (&report.constants, &report.symbols) {
            render_constants(writer, constants, symbols.data.section_bytes, binary.shipped)?;
        }

        if let Some(relocations) = &report.relocations {
            render_relocations(writer, relocations, binary.shipped)?;
        }

        if let Some(ir) = &report.llvm_ir {
            render_llvm_ir(writer, ir)?;
        }

        if let Some(mono) = &report.mono {
            render_mono(writer, mono)?;
        }

        if let Some(remarks) = &report.remarks {
            render_remarks(writer, remarks)?;
        }

        if let Some(whatif) = &report.whatif {
            render_whatif(writer, whatif, binary.shipped)?;
        }
    } else {
        // No binary to break down; the dependency graph is all there is.
        render_duplicates(writer, &report.duplicates)?;
    }

    Ok(())
}

fn render_symbols<W: io::Write>(
    writer: &mut W,
    symbols: &SymbolReport,
    duplicates: &[Duplicate],
    provenance: Option<&ProvenanceReport>,
    total: u64,
) -> io::Result<()> {
    let named = format_args!("code in {} named symbols", symbols.code.count);
    row(writer, symbols.code.bytes, total, named)?;
    let named = format_args!("read-only data in {} named symbols", symbols.data.count);
    row(writer, symbols.data.bytes, total, named)?;

    let anonymous = (symbols.code.section_bytes + symbols.data.section_bytes)
        .saturating_sub(symbols.code.bytes + symbols.data.bytes);
    row(writer, anonymous, total, "in those sections, named by no symbol")?;

    // Code that came with no debug info: C and assembly objects, linker stubs.
    if let Some(provenance) = provenance
        && provenance.uncovered > 0
    {
        row(
            writer,
            provenance.uncovered,
            total,
            "code no compile unit claims (built without debug info: C, assembly, std's backtrace crates)",
        )?;
    }

    writeln!(writer)?;
    render_duplicates(writer, duplicates)?;

    writeln!(writer, "\nlargest functions")?;
    for symbol in &symbols.code.largest {
        row(writer, symbol.size, total, label(symbol))?;
    }

    writeln!(writer, "\nlargest data symbols")?;
    if symbols.data.largest.iter().any(|symbol| !symbol.exact) {
        writeln!(
            writer,
            "  (≤ marks an upper bound: the size runs to the next symbol, so it also counts the unnamed constants in between)"
        )?;
    }
    for symbol in &symbols.data.largest {
        bounded_row(writer, symbol, total)?;
    }

    writeln!(writer, "\nby pattern")?;
    writeln!(
        writer,
        "  (a symbol can match several, so these do not sum to the total; the indented rows are the largest single offenders)"
    )?;
    for pattern in &symbols.patterns {
        row(
            writer,
            pattern.size,
            total,
            format_args!("{} ({} symbols)", pattern.name, pattern.symbols),
        )?;

        // The total is spread over many symbols; name the largest few, the only
        // ones a single change could meaningfully shrink.
        if pattern.largest.len() > 1 {
            for member in &pattern.largest {
                row(writer, member.bytes, total, format_args!("    {}", member.name))?;
            }
        }
    }

    writeln!(writer, "\nby trait method, every impl combined")?;
    writeln!(
        writer,
        "  (attribution \u{2014} one method summed over every impl, so where the bytes sit, not what one change removes)"
    )?;
    groups(writer, &symbols.trait_methods, total, "impls")?;

    writeln!(writer, "\nby trait, every method of every impl combined")?;
    writeln!(
        writer,
        "  (one axis coarser than above; the indented rows are the trait's largest single impls \u{2014} the concrete targets)"
    )?;
    for group in &symbols.traits {
        row(writer, group.size, total, format_args!("{} ({} methods)", group.name, group.methods))?;

        if group.largest.len() > 1 {
            for member in &group.largest {
                row(writer, member.bytes, total, format_args!("    {}", member.name))?;
            }
        }
    }

    writeln!(writer, "\nby crate, where the code is defined")?;
    groups(writer, &symbols.crates, total, "symbols")?;

    writeln!(writer, "\ngeneric families")?;
    writeln!(writer, "  (recoverable = the total less its largest instance)")?;
    for family in &symbols.generics {
        row(
            writer,
            family.size,
            total,
            format_args!(
                "{} ({}\u{d7}, {} each, ~{} recoverable){}",
                family.name,
                family.instantiations,
                bytes(family.each),
                bytes(family.recoverable),
                at(family.defined_at.as_deref())
            ),
        )?;
    }

    writeln!(writer, "\nby crate, which one caused the instantiation")?;
    writeln!(
        writer,
        "  (generic code from the list above, re-attributed \u{2014} not additional)"
    )?;
    groups(writer, &symbols.instantiated_by, total, "symbols")?;

    writeln!(writer)
}

fn render_features<W: io::Write>(
    writer: &mut W,
    features: &FeatureReport,
    total: u64,
) -> io::Result<()> {
    if features.crates.is_empty() {
        return Ok(());
    }

    writeln!(writer, "\ndependency features")?;
    writeln!(
        writer,
        "  (each linked dependency's resolved features and who asked for them; the bytes are the crate's whole code, so a shorter feature list returns some part of them)"
    )?;
    for krate in &features.crates {
        let features = krate.features.join(", ");
        let requesters = krate
            .requested_by
            .iter()
            .map(|requester| {
                let mut asked = Vec::new();
                if requester.default {
                    asked.push("default features".to_owned());
                }
                if !requester.features.is_empty() {
                    asked.push(requester.features.join(", "));
                }
                if asked.is_empty() {
                    requester.name.clone()
                } else {
                    format!("{} ({})", requester.name, asked.join("; "))
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        let label = format!("{} {}: {features} \u{2190} {requesters}", krate.name, krate.version);
        match krate.bytes {
            Some(size) => row(writer, size, total, label)?,
            None => writeln!(writer, "  {:>12}  {:>5}  {label}", "?", "-")?,
        }
    }

    writeln!(writer)
}

fn render_instantiations<W: io::Write>(
    writer: &mut W,
    instantiations: &InstantiationReport,
    total: u64,
) -> io::Result<()> {
    if instantiations.crates.is_empty() {
        return Ok(());
    }

    writeln!(writer, "\ngeneric code, by the types it is instantiated over")?;
    writeln!(
        writer,
        "  (a turbofish names the types a generic was specialized to; bytes count toward every crate those types name, so rows overlap \u{2014} the indented rows are the largest generic families within each)"
    )?;

    let combined = instantiations.bytes + instantiations.inlined_bytes;
    if instantiations.inlined_bytes > 0 {
        let label = format_args!(
            "in {} symbols and {} inlined instances",
            instantiations.symbols, instantiations.instances
        );
        row(writer, combined, total, label)?;
    } else {
        row(writer, combined, total, format_args!("in {} symbols", instantiations.symbols))?;
    }

    for entry in &instantiations.crates {
        let label = format_args!("{} ({} instantiations)", entry.name, entry.instantiations);
        row(writer, entry.bytes, total, label)?;

        for family in &entry.largest {
            row(writer, family.bytes, total, format_args!("    {}", family.name))?;
        }
    }

    writeln!(writer)
}

/// One row per group, all of which differ only in what they count.
fn groups<W: io::Write>(
    writer: &mut W,
    groups: &[Group],
    total: u64,
    unit: &str,
) -> io::Result<()> {
    for entry in groups {
        row(writer, entry.size, total, format_args!("{} ({} {unit})", entry.name, entry.symbols))?;
    }

    Ok(())
}

fn render_diff<W: io::Write>(writer: &mut W, diff: &DiffReport, total: u64) -> io::Result<()> {
    writeln!(writer, "\nvs baseline {}", diff.baseline)?;
    writeln!(
        writer,
        "  {:>12}  {:>4.1}%  code, from {} to {}",
        signed(diff.before, diff.after),
        percent(diff.after.abs_diff(diff.before), total),
        bytes(diff.before),
        bytes(diff.after)
    )?;

    if !diff.crates.is_empty() {
        writeln!(writer, "\nby crate, largest change")?;
        for delta in &diff.crates {
            delta_row(writer, delta, total, "")?;
        }
    }

    if !diff.symbols.is_empty() {
        writeln!(writer, "\nby function, largest change")?;
        for delta in &diff.symbols {
            delta_row(writer, delta, total, "")?;
        }
    }

    writeln!(writer)
}

fn delta_row<W: io::Write>(
    writer: &mut W,
    delta: &NamedDelta,
    total: u64,
    prefix: &str,
) -> io::Result<()> {
    let tag = match (delta.before, delta.after) {
        (0, _) => " (new)",
        (_, 0) => " (removed)",
        _ => "",
    };
    writeln!(
        writer,
        "  {:>12}  {:>4.1}%  {prefix}{}{tag}",
        signed(delta.before, delta.after),
        percent(delta.after.abs_diff(delta.before), total),
        delta.name
    )
}

/// A byte delta with a leading sign, computed without an `i64` cast.
fn signed(before: u64, after: u64) -> String {
    if after >= before {
        format!("+{}", bytes(after - before))
    } else {
        format!("-{}", bytes(before - after))
    }
}

fn render_categories<W: io::Write>(
    writer: &mut W,
    categories: &CategoryReport,
    total: u64,
) -> io::Result<()> {
    if !categories.derives.is_empty() {
        writeln!(writer, "\nby derive, every impl combined")?;
        writeln!(
            writer,
            "  (every impl of the trait, derived or hand-written alike; the total is attribution across many types, not a saving \u{2014} the indented rows are the largest single impls, the ones worth acting on)"
        )?;
        for derive in &categories.derives {
            let label = format_args!("{} ({} impls)", derive.name, derive.impls);
            row(writer, derive.bytes, total, label)?;

            // The total is spread over many types; name the largest few, which
            // are the only ones a single change could meaningfully shrink.
            if derive.largest.len() > 1 {
                for member in &derive.largest {
                    row(writer, member.bytes, total, format_args!("    {}", member.name))?;
                }
            }
        }
    }

    if categories.cold > 0 {
        writeln!(writer, "\ncold code, split off for panic and error paths")?;
        row(writer, categories.cold, total, "in .text.unlikely")?;
    }

    if !categories.derives.is_empty() || categories.cold > 0 {
        writeln!(writer)?;
    }

    Ok(())
}

fn render_dispatch<W: io::Write>(
    writer: &mut W,
    dispatch: &DispatchReport,
    total: u64,
) -> io::Result<()> {
    if dispatch.vtables.count == 0 && dispatch.shims.count == 0 {
        return Ok(());
    }

    writeln!(writer, "\ndynamic dispatch and coercion")?;
    writeln!(
        writer,
        "  (a proxy: named vtables and fn-pointer shims \u{2014} the indented rows name the few that carry a symbol; most vtables are anonymous)"
    )?;
    let vtables = format_args!("vtables ({} symbols)", dispatch.vtables.count);
    row(writer, dispatch.vtables.bytes, total, vtables)?;
    let shims = format_args!("coercion and drop shims ({} symbols)", dispatch.shims.count);
    row(writer, dispatch.shims.bytes, total, shims)?;

    for symbol in &dispatch.largest {
        row(writer, symbol.size, total, format_args!("    {}", symbol.name))?;
    }

    writeln!(writer)
}

fn render_graph<W: io::Write>(writer: &mut W, graph: &GraphReport, total: u64) -> io::Result<()> {
    if graph.vtables.is_empty()
        && graph.single_callers.is_empty()
        && graph.retained.is_empty()
        && graph.unreachable.symbols == 0
    {
        return Ok(());
    }

    if !graph.vtables.is_empty() {
        writeln!(writer, "\nvtables by trait object")?;
        writeln!(
            writer,
            "  (recovered from the function pointers each anonymous vtable carries \u{2014} the trait objects the named floor above cannot see)"
        )?;
        for group in &graph.vtables {
            let label = format_args!("dyn {} ({} vtables)", group.name, group.count);
            row(writer, group.bytes, total, label)?;
        }
    }

    if !graph.single_callers.is_empty() {
        writeln!(writer, "\ncalled from one place")?;
        writeln!(
            writer,
            "  (nothing else reaches these \u{2014} each exists for a single call site, named after the arrow, where merging or inlining it would land)"
        )?;
        for single in &graph.single_callers {
            let label = format_args!(
                "{} \u{2190} {}{}",
                single.name,
                single.caller,
                at(single.defined_at.as_deref())
            );
            row(writer, single.bytes, total, label)?;
        }
    }

    if !graph.retained.is_empty() {
        writeln!(writer, "\nremoving a function frees, with everything only it reaches")?;
        writeln!(
            writer,
            "  (dominators of the reference graph, from the entry point; conservative \u{2014} an indirect call the assembly cannot name may retain more)"
        )?;
        for entry in &graph.retained {
            let label = format_args!(
                "{} (itself {}, plus {} symbols)",
                entry.name,
                bytes(entry.own),
                entry.dominated
            );
            row(writer, entry.retained, total, label)?;
        }
    }

    if graph.unreachable.symbols > 0 {
        writeln!(writer, "\nreached by no reference the graph can see")?;
        writeln!(
            writer,
            "  (linked code with no path of calls, addresses, or data slots from the entry \u{2014} kept by something the assembly does not name)"
        )?;
        let label = format_args!("in {} functions", graph.unreachable.symbols);
        row(writer, graph.unreachable.bytes, total, label)?;
    }

    writeln!(writer)
}

fn render_dupdata<W: io::Write>(
    writer: &mut W,
    dupdata: &DupDataReport,
    total: u64,
) -> io::Result<()> {
    if dupdata.largest.is_empty() {
        return Ok(());
    }

    writeln!(writer, "\nduplicate read-only data")?;
    writeln!(
        writer,
        "  (byte-identical constants under different names; the linker's --icf or sharing a const collapses them)"
    )?;
    let summary =
        format_args!("recoverable from {} groups ({} symbols)", dupdata.groups, dupdata.symbols);
    row(writer, dupdata.recoverable, total, summary)?;

    for group in &dupdata.largest {
        let copies = group.names.len();
        let first = group.names.first().map(String::as_str).unwrap_or_default();
        let label = if group.names.iter().all(|name| name == first) {
            format!("{first} ({copies}\u{d7}, {} each)", bytes(group.size))
        } else {
            let others: Vec<&str> =
                group.names.iter().skip(1).take(2).map(String::as_str).collect();
            let more = copies.saturating_sub(1 + others.len());
            let more = if more > 0 { format!(" \u{2261} {more} more") } else { String::new() };
            format!(
                "{first} \u{2261} {}{more} ({copies} symbols, {} each)",
                others.join(" \u{2261} "),
                bytes(group.size)
            )
        };
        row(writer, group.recoverable, total, label)?;
    }

    writeln!(writer)
}

fn render_overhead<W: io::Write>(
    writer: &mut W,
    overhead: &OverheadReport,
    total: u64,
) -> io::Result<()> {
    let data_bytes: u64 = overhead.data.iter().map(|group| group.bytes).sum();
    if overhead.unwind == 0 && data_bytes == 0 {
        return Ok(());
    }

    writeln!(writer, "\npanic, format, and unwind overhead")?;
    writeln!(writer, "  (infrastructure the code views only hint at; the levers below remove it)")?;
    row(writer, overhead.unwind, total, "unwind and exception tables")?;
    for group in &overhead.data {
        let label = format_args!("{} ({} symbols)", group.kind, group.symbols);
        row(writer, group.bytes, total, label)?;
    }

    writeln!(
        writer,
        "\n  levers: panic=\"abort\" drops the unwind tables; -Zbuild-std with panic_immediate_abort strips the panic locations; disabling tracing removes the callsite metadata"
    )?;
    writeln!(writer)
}

fn render_whatif<W: io::Write>(
    writer: &mut W,
    whatif: &WhatIfReport,
    total: u64,
) -> io::Result<()> {
    if whatif.levers.is_empty() {
        return Ok(());
    }

    writeln!(writer, "\nwhat-if, measured by rebuilding")?;
    writeln!(
        writer,
        "  (the change in shipped size under each lever \u{2014} a measurement, not a proposal; beneath it, the functions that moved most, which is where that cost sits in the source)"
    )?;
    for lever in &whatif.levers {
        writeln!(
            writer,
            "  {:>12}  {:>4.1}%  {} ({} \u{2192} {})",
            signed(lever.before, lever.after),
            percent(lever.before.abs_diff(lever.after), total),
            lever.name,
            bytes(lever.before),
            bytes(lever.after)
        )?;
        if let Some(reading) = whatif::reading(&lever.name) {
            writeln!(writer, "{:>23}({reading})", "")?;
        }
        // The functions are the targets; the by-crate deltas stay in the JSON.
        for delta in lever.diff.symbols.iter().take(WHATIF_MOVERS) {
            delta_row(writer, delta, total, "    ")?;
        }
    }
    if !whatif.skipped.is_empty() {
        writeln!(writer, "  skipped, the build failed: {}", whatif.skipped.join(", "))?;
    }

    writeln!(writer)
}

/// How many movers each lever lists in text; the JSON keeps `--limit`.
const WHATIF_MOVERS: usize = 10;

/// IR lines are not binary bytes — the optimizer deletes much of this — so this
/// shows line counts and instantiations, no size or percentage.
fn render_llvm_ir<W: io::Write>(writer: &mut W, ir: &IrReport) -> io::Result<()> {
    writeln!(writer, "\nLLVM IR by generic family")?;
    writeln!(
        writer,
        "  (pre-optimization IR lines, not binary bytes \u{2014} where the code comes from, before the optimizer deletes it)"
    )?;
    writeln!(
        writer,
        "  {} lines across {} functions in {} crates",
        ir.lines, ir.functions, ir.files
    )?;
    for family in &ir.families {
        writeln!(
            writer,
            "  {:>12}  {} ({}\u{d7})",
            format!("{} lines", family.lines),
            family.name,
            family.instantiations
        )?;
    }

    writeln!(writer)
}

/// Loop expansions are counted, not sized — the remark says what was done, not
/// how many bytes it made — so this shows counts alone.
fn render_remarks<W: io::Write>(writer: &mut W, remarks: &RemarksReport) -> io::Result<()> {
    if remarks.functions.is_empty() {
        return Ok(());
    }

    writeln!(writer, "\nloops the optimizer expanded")?;
    writeln!(
        writer,
        "  (unrolled, peeled, or vectorized \u{2014} body copies the source never wrote, from each crate's own optimization remarks; a simpler loop, #[inline(never)], or #[cold] takes them back)"
    )?;
    writeln!(
        writer,
        "  {} unrolled, {} peeled, {} vectorized, in {} remark files",
        remarks.unrolled, remarks.peeled, remarks.vectorized, remarks.files
    )?;
    writeln!(writer, "\nfunctions with the most expanded loops")?;
    for function in &remarks.functions {
        writeln!(
            writer,
            "  {:>12}  {} ({} unrolled into {} copies, {} peeled, {} vectorized)",
            format!("{} loops", function.unrolled + function.peeled + function.vectorized),
            function.name,
            function.unrolled,
            function.copies,
            function.peeled,
            function.vectorized
        )?;
    }
    if !remarks.workspace_sites.is_empty() {
        writeln!(writer, "\nloops in this workspace the optimizer expanded")?;
        for site in &remarks.workspace_sites {
            writeln!(
                writer,
                "  {:>12}  {}:{} in {}",
                site.detail, site.file, site.line, site.function
            )?;
            snippet_row(writer, site.snippet.as_deref())?;
        }
    }

    writeln!(writer)
}

/// MIR statement estimates are not bytes either — the backend inlines and
/// deletes much of this — so this shows the estimates alone.
fn render_mono<W: io::Write>(writer: &mut W, mono: &MonoReport) -> io::Result<()> {
    if mono.largest.is_empty() {
        return Ok(());
    }

    writeln!(writer, "\ngeneric definitions by estimated codegen cost")?;
    writeln!(
        writer,
        "  (MIR statements handed to the backend, every instantiation summed, before inlining \u{2014} an estimate, not bytes; large and often instantiated is what to split into a small generic shell over a non-generic body)"
    )?;
    writeln!(
        writer,
        "  {} definitions, {} instantiations, ~{} statements across {} crates",
        mono.definitions, mono.instantiations, mono.estimate, mono.crates
    )?;
    for definition in &mono.largest {
        // A crate's own items are spelled without their crate, so name it.
        let more = definition.crates.saturating_sub(definition.crate_names.len());
        let crates = match (definition.crate_names.as_slice(), more) {
            ([], _) => String::new(),
            (names, 0) => format!(" in {}", names.join(", ")),
            (names, more) => format!(" in {} and {more} more", names.join(", ")),
        };
        writeln!(
            writer,
            "  {:>12}  {} ({}\u{d7}, ~{} each{crates})",
            format!("~{}", definition.estimate),
            definition.name,
            definition.instantiations,
            definition.each,
        )?;
    }

    writeln!(writer)
}

/// Types carry an in-memory layout size, not a share of the binary, so this
/// shows the size alone — no percentage.
fn render_types<W: io::Write>(writer: &mut W, types: &TypeReport) -> io::Result<()> {
    if types.largest.is_empty() {
        return Ok(());
    }

    writeln!(writer, "\nlargest types")?;
    writeln!(
        writer,
        "  (in-memory layout size; a large type drives the moves, copies, and drop glue above; an enum is as large as its largest variant, so boxing that one shrinks every value)"
    )?;
    for ty in &types.largest {
        writeln!(writer, "  {:>12}  {}{}", bytes(ty.size), ty.name, layout_note(ty))?;
    }

    writeln!(writer)
}

/// What a type's layout says: its largest variants and what boxing the
/// largest saves, or its padding.
fn layout_note(ty: &crate::types::NamedType) -> String {
    if !ty.variants.is_empty() {
        let variants = ty
            .variants
            .iter()
            .map(|variant| format!("{} {}", variant.name, bytes(variant.bytes)))
            .collect::<Vec<_>>()
            .join(", ");
        let more = ty.variant_count.saturating_sub(ty.variants.len());
        let more = if more > 0 { format!(" and {more} more") } else { String::new() };
        let boxing = match (ty.boxing_saves, ty.variants.first()) {
            (Some(saves), Some(largest)) => {
                format!("; boxing {} saves ~{} per value", largest.name, bytes(saves))
            }
            _ => String::new(),
        };
        return format!(" \u{2014} variants {variants}{more}{boxing}");
    }
    match ty.padding {
        Some(padding) if padding >= 8 => format!(" \u{2014} {} padding", bytes(padding)),
        _ => String::new(),
    }
}

fn render_inlined<W: io::Write>(
    writer: &mut W,
    inlined: &InlineReport,
    total: u64,
) -> io::Result<()> {
    let found =
        format_args!("in {} inlined instances, charged to their callers", inlined.instances);
    row(writer, inlined.bytes, total, found)?;

    writeln!(writer, "\nlargest inlined functions")?;
    for function in &inlined.functions {
        row(
            writer,
            function.bytes,
            total,
            format_args!(
                "{} ({} sites){}",
                function.name,
                function.sites,
                at(function.defined_at.as_deref())
            ),
        )?;
    }

    writeln!(writer, "\nsource lines in this workspace that pulled in the most inlined code")?;
    inlined_sites(writer, &inlined.workspace_call_sites, total)?;

    writeln!(writer)
}

fn inlined_sites<W: io::Write>(writer: &mut W, sites: &[CallSite], total: u64) -> io::Result<()> {
    for site in sites {
        row(
            writer,
            site.bytes,
            total,
            format_args!("{}:{} ({} inlined)", site.file, site.line, site.instances),
        )?;
        snippet_row(writer, site.snippet.as_deref())?;
    }

    Ok(())
}

/// The source text under a row that names a line, aligned with its label.
fn snippet_row<W: io::Write>(writer: &mut W, snippet: Option<&str>) -> io::Result<()> {
    match snippet {
        Some(text) => writeln!(writer, "{:>23}\u{2502} {text}", ""),
        None => Ok(()),
    }
}

fn render_assembly<W: io::Write>(
    writer: &mut W,
    assembly: &AssemblyReport,
    total: u64,
) -> io::Result<()> {
    // Instructions become bytes at the rate the linked functions show.
    let each = if assembly.instructions == 0 {
        0.0
    } else {
        #[expect(clippy::cast_precision_loss, reason = "display only")]
        let each = assembly.bytes as f64 / assembly.instructions as f64;
        each
    };
    let approx = |instructions: u64| {
        #[expect(clippy::cast_precision_loss, reason = "display only")]
        #[expect(clippy::cast_possible_truncation, reason = "instructions × bytes fits")]
        #[expect(clippy::cast_sign_loss, reason = "both factors are non-negative")]
        let bytes = (instructions as f64 * each) as u64;
        bytes
    };

    let path = match assembly.paths.as_slice() {
        [] => String::new(),
        [path] => path.clone(),
        [path, rest @ ..] => format!("{path} and {} more", rest.len()),
    };
    writeln!(writer, "assembly ({path})")?;
    row(
        writer,
        assembly.bytes,
        total,
        format_args!(
            "code in {} functions with assembly, {} instructions, {each:.1} B each",
            assembly.linked, assembly.instructions
        ),
    )?;
    if assembly.functions > assembly.linked {
        writeln!(
            writer,
            "  ({} more functions in the assembly never reached the binary)",
            assembly.functions - assembly.linked
        )?;
    }

    render_identical(writer, &assembly.identical, total)?;
    render_panics(writer, &assembly.panics, total, &approx)?;
    render_formatting(writer, &assembly.formatting, total, &approx)?;
    render_copies(writer, &assembly.copies, total, &approx)?;

    writeln!(writer, "\nsource lines in this workspace compiled to the most instructions")?;
    writeln!(
        writer,
        "  (the line an instruction came from, after inlining, every instantiation summed)"
    )?;
    lines(writer, &assembly.workspace_lines, total, &approx)?;

    writeln!(writer)
}

fn render_constants<W: io::Write>(
    writer: &mut W,
    constants: &ConstantsReport,
    section_bytes: u64,
    total: u64,
) -> io::Result<()> {
    if constants.linked == 0 {
        return Ok(());
    }

    writeln!(writer, "\nconstant data, from the assembly")?;
    writeln!(
        writer,
        "  (every constant the assembly spells out, sized from its directives and read by shape; only what a linked function reaches counts, and under lto=\"fat\" that is the whole program)"
    )?;
    row(
        writer,
        constants.bytes,
        total,
        format_args!(
            "in {} constants a linked function reaches ({} defined), against {} of read-only data sections",
            constants.linked,
            constants.constants,
            bytes(section_bytes)
        ),
    )?;
    for class in &constants.classes {
        row(writer, class.bytes, total, format_args!("{} ({})", class.kind.label(), class.count))?;
    }
    if constants.panic_messages.count > 0 {
        row(
            writer,
            constants.panic_messages.bytes,
            total,
            format_args!(
                "of the text, loaded only on the way to a panic: messages and their pieces ({})",
                constants.panic_messages.count
            ),
        )?;
    }

    let locations = &constants.locations;
    if locations.records > 0 {
        writeln!(writer, "\npanic locations by source file in this workspace")?;
        writeln!(
            writer,
            "  (a 24 B record per panic site \u{2014} an unwrap, an index, an expect \u{2014} plus the path once; {} records in all, {})",
            locations.records,
            bytes(locations.bytes)
        )?;
        for file in &locations.workspace_files {
            let lines = file.lines.iter().map(u64::to_string).collect::<Vec<_>>().join(", ");
            row(
                writer,
                file.bytes,
                total,
                format_args!("{} ({} records; lines {lines})", file.file, file.records),
            )?;
        }

        writeln!(writer, "\nfunctions loading the most panic locations")?;
        for caller in &locations.functions {
            row(
                writer,
                caller.bytes,
                total,
                format_args!("{} ({} records)", caller.name, caller.records),
            )?;
        }
    }

    if !constants.strings.is_empty() {
        writeln!(writer, "\nlargest strings")?;
        for string in &constants.strings {
            let loaders = match string.functions.as_slice() {
                [] => String::new(),
                [only] if string.references == 1 => format!(", loaded by {only}"),
                [first, ..] => {
                    format!(", loaded by {first} and {} more", string.references - 1)
                }
            };
            row(
                writer,
                string.bytes,
                total,
                format_args!("\"{}\" ({}{loaders})", string.preview, kind_word(string.kind)),
            )?;
        }
    }

    if !constants.tables.is_empty() {
        writeln!(writer, "\nlookup and jump tables, by the function whose match built them")?;
        for table in &constants.tables {
            row(
                writer,
                table.bytes,
                total,
                format_args!(
                    "{} ({} lookup, {} jump)",
                    table.name, table.switch_tables, table.jump_tables
                ),
            )?;
        }
    }

    if !constants.functions.is_empty() {
        writeln!(writer, "\nfunctions carrying the most constant data")?;
        writeln!(
            writer,
            "  (ranked by what only that function reaches, directly or through a table's pointers \u{2014} what rewriting it alone frees; the total counts shared constants too)"
        )?;
        for carrier in &constants.functions {
            row(
                writer,
                carrier.exclusive,
                total,
                format_args!(
                    "{} ({} constants, {} in all)",
                    carrier.name,
                    carrier.constants,
                    bytes(carrier.bytes)
                ),
            )?;
        }
    }

    writeln!(writer)
}

/// A string's kind, as one word after its preview.
const fn kind_word(kind: constants::Kind) -> &'static str {
    match kind {
        constants::Kind::Path => "path",
        constants::Kind::Name => "name",
        _ => "message",
    }
}

/// Only ELF pays for its pointer slots in the file, so only there is this a
/// text section; Mach-O keeps the slot counts in the JSON.
fn render_relocations<W: io::Write>(
    writer: &mut W,
    relocations: &RelocationReport,
    total: u64,
) -> io::Result<()> {
    if relocations.bytes == 0 {
        return Ok(());
    }

    writeln!(writer, "\ndynamic relocations")?;
    if relocations.record > 0 {
        writeln!(
            writer,
            "  (every pointer kept in data is a slot the loader fills at start, and each costs a {} B record here on top of its 8 B; tables of &str, vtables, and panic locations are where they come from \u{2014} offsets instead of pointers remove both)",
            relocations.record
        )?;
    } else {
        writeln!(
            writer,
            "  (every pointer kept in data is a slot the loader fills at start; the records are compressed, so the slot's own 8 B is the cost \u{2014} offsets instead of pointers remove it)"
        )?;
    }
    let packed = if relocations.packed { "packed " } else { "" };
    row(
        writer,
        relocations.bytes,
        total,
        format_args!(
            "in {packed}relocation records for {} pointer slots ({} of slots)",
            relocations.slots,
            bytes(relocations.slots as u64 * 8)
        ),
    )?;
    if relocations.record > 0 {
        writeln!(writer, "\ndata symbols with the most pointer slots")?;
        for group in &relocations.symbols {
            row(
                writer,
                group.bytes,
                total,
                format_args!("{} ({} slots)", group.name, group.slots),
            )?;
        }
    }

    writeln!(writer)
}

fn render_identical<W: io::Write>(
    writer: &mut W,
    identical: &Identical,
    total: u64,
) -> io::Result<()> {
    writeln!(writer, "\nidentical function bodies, by what folding each group would return")?;
    writeln!(
        writer,
        "  (the same instructions under different names; a linker folding identical code keeps one, and so does instantiating one)"
    )?;
    row(
        writer,
        identical.recoverable,
        total,
        format_args!(
            "recoverable from {} groups of identical functions ({} functions)",
            identical.groups, identical.functions
        ),
    )?;
    for group in &identical.largest {
        let copies = group.names.len();
        let mut names = group.names.iter();
        let first = names.next().map(String::as_str).unwrap_or_default();
        let label = if group.names.iter().all(|name| name == first) {
            format!("{first} ({copies}\u{d7}, {} each)", bytes(group.bytes))
        } else {
            let others: Vec<&str> = names.take(2).map(String::as_str).collect();
            let more = copies.saturating_sub(1 + others.len());
            let more = if more > 0 { format!(" \u{2261} {more} more") } else { String::new() };
            format!(
                "{first} \u{2261} {}{more} ({copies} functions, {} each)",
                others.join(" \u{2261} "),
                bytes(group.bytes)
            )
        };
        row(writer, group.recoverable, total, label)?;
    }

    Ok(())
}

fn render_panics<W: io::Write>(
    writer: &mut W,
    panics: &Panics,
    total: u64,
    approx: &dyn Fn(u64) -> u64,
) -> io::Result<()> {
    writeln!(writer, "\npanic call sites")?;
    writeln!(
        writer,
        "  (each is a compare, a branch, and a cold block that loads the location and calls; the location is 24 B more of read-only data)"
    )?;
    approx_row(
        writer,
        approx(panics.instructions),
        total,
        format_args!(
            "in the blocks of {} sites: {} bounds checks, {} unwraps, {} allocation failures, {} other",
            panics.sites, panics.bounds_checks, panics.unwraps, panics.allocation, panics.other
        ),
    )?;
    writeln!(
        writer,
        "  ({} distinct locations and messages loaded by those blocks)",
        panics.constants
    )?;
    writeln!(writer, "\nfunctions spending the most on panic call sites")?;
    callers(writer, &panics.functions, total, approx)
}

fn render_formatting<W: io::Write>(
    writer: &mut W,
    formatting: &Formatting,
    total: u64,
    approx: &dyn Fn(u64) -> u64,
) -> io::Result<()> {
    writeln!(writer, "\nformatting call sites, into core::fmt and alloc::fmt")?;
    writeln!(writer, "  (the block before each call builds the Arguments)")?;
    approx_row(
        writer,
        approx(formatting.instructions),
        total,
        format_args!("in the blocks of {} sites", formatting.sites),
    )?;
    writeln!(writer, "\nfunctions spending the most on formatting call sites")?;
    callers(writer, &formatting.functions, total, approx)
}

fn render_copies<W: io::Write>(
    writer: &mut W,
    copies: &Copies,
    total: u64,
    approx: &dyn Fn(u64) -> u64,
) -> io::Result<()> {
    writeln!(writer, "\nvalues copied through memory")?;
    writeln!(
        writer,
        "  (runs of {COPY_RUN} or more loads and stores back to back, and calls to memcpy for anything larger; boxing the value or passing it by reference removes them)"
    )?;
    approx_row(
        writer,
        approx(copies.instructions),
        total,
        format_args!(
            "in {} runs, {} instructions, plus {} memcpy-family calls",
            copies.runs, copies.instructions, copies.calls
        ),
    )?;
    writeln!(writer, "\nfunctions copying the most")?;
    for copier in &copies.functions {
        approx_row(
            writer,
            approx(copier.instructions),
            total,
            format_args!(
                "{} ({} instructions in {} runs, {} calls)",
                copier.name, copier.instructions, copier.runs, copier.calls
            ),
        )?;
    }

    Ok(())
}

fn callers<W: io::Write>(
    writer: &mut W,
    callers: &[Caller],
    total: u64,
    approx: &dyn Fn(u64) -> u64,
) -> io::Result<()> {
    for caller in callers {
        approx_row(
            writer,
            approx(caller.instructions),
            total,
            format_args!(
                "{} ({} sites, {} instructions)",
                caller.name, caller.sites, caller.instructions
            ),
        )?;
    }

    Ok(())
}

fn lines<W: io::Write>(
    writer: &mut W,
    lines: &[Line],
    total: u64,
    approx: &dyn Fn(u64) -> u64,
) -> io::Result<()> {
    for line in lines {
        approx_row(
            writer,
            approx(line.instructions),
            total,
            format_args!("{}:{} ({} instructions)", line.file, line.line, line.instructions),
        )?;
        snippet_row(writer, line.snippet.as_deref())?;
    }

    Ok(())
}

fn render_binary<W: io::Write>(
    writer: &mut W,
    binary: &BinaryReport,
    limit: usize,
) -> io::Result<()> {
    writeln!(writer, "{} ({})", binary.path, binary.format)?;
    writeln!(writer, "  {:>12}  total", bytes(binary.total))?;
    writeln!(writer, "  {:>12}  shipped, excluding symbols and debug info", bytes(binary.shipped))?;
    writeln!(writer)?;

    for category in &binary.categories {
        if category.category.is_stripped() {
            unshipped_row(
                writer,
                category.size,
                format_args!("{} (not shipped)", category.category),
            )?;
        } else {
            row(writer, category.size, binary.shipped, category.category)?;
        }
    }
    row(writer, binary.other, binary.shipped, "overhead (headers, padding, code signature)")?;
    writeln!(writer)?;

    for section in binary.sections.iter().take(limit) {
        if section.category.is_stripped() {
            unshipped_row(writer, section.size, &section.name)?;
        } else {
            row(writer, section.size, binary.shipped, &section.name)?;
        }
    }

    writeln!(writer)
}

fn row<W: io::Write, L: fmt::Display>(
    writer: &mut W,
    size: u64,
    total: u64,
    label: L,
) -> io::Result<()> {
    writeln!(writer, "  {:>12}  {:>4.1}%  {label}", bytes(size), percent(size, total))
}

/// Symbols and debug info are measured but stripped before release, so they are
/// not part of the denominator and a share of it would be meaningless.
fn unshipped_row<W: io::Write, L: fmt::Display>(
    writer: &mut W,
    size: u64,
    label: L,
) -> io::Result<()> {
    writeln!(writer, "  {:>12}  {:>5}  {label}", bytes(size), "-")
}

/// Name a symbol, flagging when the binary carries more than one copy of it,
/// and saying where it is defined when the debug info told.
fn label(symbol: &Symbol) -> String {
    let mut label = if symbol.copies > 1 {
        format!("{} ({}\u{d7})", symbol.name, symbol.copies)
    } else {
        symbol.name.clone()
    };
    label.push_str(&at(symbol.defined_at.as_deref()));
    label
}

/// ` @ file:line`, or nothing.
fn at(site: Option<&str>) -> String {
    site.map(|site| format!(" @ {site}")).unwrap_or_default()
}

/// A size converted from an instruction count at the binary's average bytes
/// per instruction, so marked approximate.
fn approx_row<W: io::Write, L: fmt::Display>(
    writer: &mut W,
    size: u64,
    total: u64,
    label: L,
) -> io::Result<()> {
    let size_text = format!("~{}", bytes(size));
    writeln!(writer, "  {size_text:>12}  {:>4.1}%  {label}", percent(size, total))
}

/// A size inferred from the gap to the next symbol is an upper bound: it also
/// covers whatever anonymous data sits in between.
fn bounded_row<W: io::Write>(writer: &mut W, symbol: &Symbol, total: u64) -> io::Result<()> {
    let bound = if symbol.exact { "" } else { "\u{2264} " };
    let size = format!("{bound}{}", bytes(symbol.size));
    writeln!(writer, "  {size:>12}  {:>4.1}%  {}", percent(symbol.size, total), label(symbol))
}

fn percent(size: u64, total: u64) -> f64 {
    #[expect(clippy::cast_precision_loss, reason = "display only")]
    let ratio = if total == 0 { 0.0 } else { size as f64 / total as f64 };

    ratio * 100.0
}

fn render_duplicates<W: io::Write>(writer: &mut W, duplicates: &[Duplicate]) -> io::Result<()> {
    if duplicates.is_empty() {
        return writeln!(writer, "no duplicate dependencies");
    }

    let count = duplicates.len();
    let noun = if count == 1 { "duplicate dependency" } else { "duplicate dependencies" };
    writeln!(writer, "{count} {noun}")?;
    writeln!(
        writer,
        "  (the same crate at several versions; each ships its own copy of the code)"
    )?;

    for (index, duplicate) in duplicates.iter().enumerate() {
        if index > 0 {
            writeln!(writer)?;
        }
        writeln!(writer, "{}", duplicate.name)?;

        for version in &duplicate.versions {
            let dependents = version
                .dependents
                .iter()
                .map(|dependent| format!("{} v{}", dependent.name, dependent.version))
                .collect::<Vec<_>>()
                .join(", ");

            // Bytes come from the compile units, once the binary's debug info
            // has been read. Zero is a version with no out-of-line code of its
            // own — generics instantiated, or everything inlined, in its users;
            // no unit at all shows nothing.
            let cost = match version.bytes {
                Some(0) => {
                    " \u{2014} no code of its own, instantiated or inlined in its users".to_owned()
                }
                Some(size) => format!(" \u{2014} {}", bytes(size)),
                None => String::new(),
            };
            if dependents.is_empty() {
                writeln!(writer, "  {}{cost}", version.version)?;
            } else {
                writeln!(writer, "  {}{cost} — used by {dependents}", version.version)?;
            }
        }
    }

    Ok(())
}

fn bytes(size: u64) -> String {
    #[expect(clippy::cast_precision_loss, reason = "display only")]
    let size = size as f64;

    for (unit, scale) in [("MiB", 1024.0 * 1024.0), ("KiB", 1024.0)] {
        if size >= scale {
            return format!("{:.1} {unit}", size / scale);
        }
    }

    format!("{size:.0} B")
}

#[cfg(test)]
mod tests {
    use super::OutputFormat;

    #[test]
    fn parses_output_formats_case_insensitively() {
        assert_eq!("text".parse(), Ok(OutputFormat::Text));
        assert_eq!("JSON".parse(), Ok(OutputFormat::Json));
        assert_eq!(
            "yaml".parse::<OutputFormat>(),
            Err("unknown format: yaml, expected: text, json".to_owned())
        );
    }
}
