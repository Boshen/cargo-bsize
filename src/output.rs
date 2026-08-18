//! Rendering of analysis results.

use std::{fmt, io, str::FromStr};

use serde::Serialize;

use crate::{
    assembly::{AssemblyReport, COPY_RUN, Caller, Copies, Formatting, Identical, Line, Panics},
    categories::CategoryReport,
    diff::{DiffReport, NamedDelta},
    dispatch::DispatchReport,
    dupdata::DupDataReport,
    duplicates::Duplicate,
    graph::GraphReport,
    inlined::{CallSite, InlineReport},
    instantiations::InstantiationReport,
    llvm_ir::IrReport,
    overhead::OverheadReport,
    sections::BinaryReport,
    symbols::{Group, Symbol, SymbolReport},
    types::TypeReport,
    whatif::WhatIfReport,
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
    pub binary: Option<BinaryReport>,
    pub symbols: Option<SymbolReport>,
    pub instantiations: Option<InstantiationReport>,
    pub overhead: Option<OverheadReport>,
    pub dupdata: Option<DupDataReport>,
    pub dispatch: Option<DispatchReport>,
    pub categories: Option<CategoryReport>,
    pub types: Option<TypeReport>,
    pub inlined: Option<InlineReport>,
    pub assembly: Option<AssemblyReport>,
    pub graph: Option<GraphReport>,
    pub diff: Option<DiffReport>,
    pub llvm_ir: Option<IrReport>,
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
            render_symbols(writer, symbols, &report.duplicates, binary.shipped)?;
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

        if let Some(ir) = &report.llvm_ir {
            render_llvm_ir(writer, ir)?;
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
    total: u64,
) -> io::Result<()> {
    let named = format_args!("code in {} named symbols", symbols.code.count);
    row(writer, symbols.code.bytes, total, named)?;
    let named = format_args!("read-only data in {} named symbols", symbols.data.count);
    row(writer, symbols.data.bytes, total, named)?;

    let anonymous = (symbols.code.section_bytes + symbols.data.section_bytes)
        .saturating_sub(symbols.code.bytes + symbols.data.bytes);
    row(writer, anonymous, total, "in those sections, named by no symbol")?;

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
                "{} ({}\u{d7}, {} each, ~{} recoverable)",
                family.name,
                family.instantiations,
                bytes(family.each),
                bytes(family.recoverable)
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
            delta_row(writer, delta, total)?;
        }
    }

    if !diff.symbols.is_empty() {
        writeln!(writer, "\nby function, largest change")?;
        for delta in &diff.symbols {
            delta_row(writer, delta, total)?;
        }
    }

    writeln!(writer)
}

fn delta_row<W: io::Write>(writer: &mut W, delta: &NamedDelta, total: u64) -> io::Result<()> {
    let tag = match (delta.before, delta.after) {
        (0, _) => " (new)",
        (_, 0) => " (removed)",
        _ => "",
    };
    writeln!(
        writer,
        "  {:>12}  {:>4.1}%  {}{tag}",
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
            let label = format_args!("{} \u{2190} {}", single.name, single.caller);
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
    writeln!(writer, "  (the change in shipped size under each lever)")?;
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
    }

    writeln!(writer)
}

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

/// Types carry an in-memory layout size, not a share of the binary, so this
/// shows the size alone — no percentage.
fn render_types<W: io::Write>(writer: &mut W, types: &TypeReport) -> io::Result<()> {
    if types.largest.is_empty() {
        return Ok(());
    }

    writeln!(writer, "\nlargest types")?;
    writeln!(
        writer,
        "  (in-memory layout size; a large type drives the moves, copies, and drop glue above)"
    )?;
    for ty in &types.largest {
        writeln!(writer, "  {:>12}  {}", bytes(ty.size), ty.name)?;
    }

    writeln!(writer)
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
            format_args!("{} ({} sites)", function.name, function.sites),
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
    }

    Ok(())
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

/// Name a symbol, flagging when the binary carries more than one copy of it.
fn label(symbol: &Symbol) -> String {
    if symbol.copies > 1 {
        format!("{} ({}\u{d7})", symbol.name, symbol.copies)
    } else {
        symbol.name.clone()
    }
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

            if dependents.is_empty() {
                writeln!(writer, "  {}", version.version)?;
            } else {
                writeln!(writer, "  {} — used by {dependents}", version.version)?;
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
