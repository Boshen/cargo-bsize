//! Rendering of analysis results, as Markdown.
//!
//! The report is read by people and by agents, so it is structured rather than
//! streamed: a title, the standing instructions, a table of contents, and one
//! `##` section per view, each with a one-line note on what it means and a
//! table whose columns separate the size, its share of the shipped binary, the
//! name, and whatever else the row carries (a definition site, a count, the
//! source text). Names go in code spans so `<T as Trait>` is never read as
//! HTML, and every table cell is one line.
//!
//! Sizes are `KiB`/`MiB`; shares are of the shipped size — the binary without
//! symbols and debug info. `~` marks a size converted from an instruction
//! count, `≤` an upper bound inferred from the gap to the next symbol.

use std::{fmt::Write as _, io};

use crate::{
    assembly::{AssemblyReport, COPY_RUN, Caller, Copies, Formatting, Identical, Panics},
    categories::CategoryReport,
    constants::{self, ConstantsReport},
    diff::{DiffReport, NamedDelta},
    dispatch::DispatchReport,
    dupdata::DupDataReport,
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
    types::{NamedType, TypeReport},
    whatif::{self, WhatIfReport},
};

/// What an agent reading this report is meant to do with it.
pub const AGENT_INSTRUCTIONS: &str =
    "Only propose source-code changes. Do not propose configuration changes.";

/// Everything the analyses produced; each view is absent when its input was.
#[derive(Debug)]
pub struct Report {
    /// Standing instructions for an agent consuming the report.
    pub instructions: &'static str,

    pub duplicates: Vec<crate::duplicates::Duplicate>,
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

/// How many movers each what-if lever lists.
const WHATIF_MOVERS: usize = 10;

/// Write the report as Markdown.
///
/// # Errors
///
/// Errors when writing to `writer` fails.
pub fn render<W: io::Write>(writer: &mut W, report: &Report, limit: usize) -> io::Result<()> {
    let mut md = Md::default();
    let title = report
        .binary
        .as_ref()
        .and_then(|binary| binary.path.rsplit('/').next())
        .map_or_else(|| "cargo bsize".to_owned(), |name| format!("cargo bsize: {name}"));
    md.title(&format!("# {title}\n\n> {}\n\n", report.instructions));

    if let Some(binary) = &report.binary {
        let total = binary.shipped;
        summary(&mut md, report, binary, limit);
        if let Some(diff) = &report.diff {
            baseline(&mut md, diff, total);
        }
        dependencies(&mut md, report, total);
        if let Some(symbols) = &report.symbols {
            functions(&mut md, symbols, total);
        }
        if let Some(instantiations) = &report.instantiations {
            argument_types(&mut md, instantiations, total);
        }
        if let Some(overhead) = &report.overhead {
            overhead_section(&mut md, overhead, total);
        }
        if let Some(dupdata) = &report.dupdata {
            duplicate_data(&mut md, dupdata, total);
        }
        dispatch(&mut md, report.dispatch.as_ref(), report.graph.as_ref(), total);
        if let Some(graph) = &report.graph {
            reference_graph(&mut md, graph, total);
        }
        if let Some(categories) = &report.categories {
            derives(&mut md, categories, total);
        }
        if let Some(types) = &report.types {
            largest_types(&mut md, types);
        }
        if let Some(inlined) = &report.inlined {
            inlined_code(&mut md, inlined, total);
        }
        if let Some(assembly) = &report.assembly {
            assembly_section(&mut md, assembly, total);
        }
        if let (Some(constants), Some(symbols)) = (&report.constants, &report.symbols) {
            constant_data(&mut md, constants, symbols.data.section_bytes, total);
        }
        if let Some(relocations) = &report.relocations {
            dynamic_relocations(&mut md, relocations, total);
        }
        if let Some(ir) = &report.llvm_ir {
            llvm_ir(&mut md, ir);
        }
        if let Some(mono) = &report.mono {
            mono_stats(&mut md, mono);
        }
        if let Some(remarks) = &report.remarks {
            expanded_loops(&mut md, remarks);
        }
        if let Some(whatif) = &report.whatif {
            what_if(&mut md, whatif, total);
        }
    } else {
        // No binary to break down; the dependency graph is all there is.
        dependencies(&mut md, report, 0);
    }

    md.write(writer)
}

// ---------------------------------------------------------------- summary

fn summary(md: &mut Md, report: &Report, binary: &BinaryReport, limit: usize) {
    let total = binary.shipped;
    md.h2("Summary");
    md.line(&format!("- Binary: `{}` ({})", binary.path, binary.format));
    md.line(&format!(
        "- Total {}, shipped {} (without symbols and debug info). Shares below are of the shipped size.",
        bytes(binary.total),
        bytes(binary.shipped)
    ));
    md.blank();

    let mut table = Table::new(&[Col::Size, Col::Share, Col::Text("Category")]);
    for category in &binary.categories {
        if category.category.is_stripped() {
            table.row([
                bytes(category.size),
                "-".to_owned(),
                format!("{} (not shipped)", category.category),
            ]);
        } else {
            table.row([
                bytes(category.size),
                share(category.size, total),
                category.category.to_string(),
            ]);
        }
    }
    table.row([
        bytes(binary.other),
        share(binary.other, total),
        "overhead (headers, padding, code signature)".to_owned(),
    ]);
    md.table(table);

    md.h3("Sections");
    let mut table = Table::new(&[Col::Size, Col::Share, Col::Text("Section")]);
    for section in binary.sections.iter().take(limit) {
        let share = if section.category.is_stripped() {
            "-".to_owned()
        } else {
            share(section.size, total)
        };
        table.row([bytes(section.size), share, code(&section.name)]);
    }
    md.table(table);

    if let Some(symbols) = &report.symbols {
        md.h3("Coverage");
        md.note("what the symbol table names, and what it leaves anonymous");
        let mut table = Table::new(&[Col::Size, Col::Share, Col::Text("Bytes")]);
        table.row([
            bytes(symbols.code.bytes),
            share(symbols.code.bytes, total),
            format!("code in {} named symbols", symbols.code.count),
        ]);
        table.row([
            bytes(symbols.data.bytes),
            share(symbols.data.bytes, total),
            format!("read-only data in {} named symbols", symbols.data.count),
        ]);
        let anonymous = (symbols.code.section_bytes + symbols.data.section_bytes)
            .saturating_sub(symbols.code.bytes + symbols.data.bytes);
        table.row([
            bytes(anonymous),
            share(anonymous, total),
            "in those sections, named by no symbol".to_owned(),
        ]);
        if let Some(provenance) = &report.provenance
            && provenance.uncovered > 0
        {
            table.row([
                bytes(provenance.uncovered),
                share(provenance.uncovered, total),
                "code no compile unit claims (built without debug info: C, assembly, std's backtrace crates)".to_owned(),
            ]);
        }
        md.table(table);
    }
}

fn baseline(md: &mut Md, diff: &DiffReport, total: u64) {
    md.h2("Compared with the baseline");
    md.line(&format!(
        "- Baseline: `{}`; code {} \u{2192} {} ({})",
        diff.baseline,
        bytes(diff.before),
        bytes(diff.after),
        signed(diff.before, diff.after)
    ));
    md.blank();
    if !diff.crates.is_empty() {
        md.h3("By crate, largest change");
        md.table(deltas(&diff.crates, total, "Crate"));
    }
    if !diff.symbols.is_empty() {
        md.h3("By function, largest change");
        md.table(deltas(&diff.symbols, total, "Function"));
    }
}

fn deltas(deltas: &[NamedDelta], total: u64, what: &'static str) -> Table {
    let mut table =
        Table::new(&[Col::Right("Change"), Col::Share, Col::Text(what), Col::Text("Note")]);
    for delta in deltas {
        let note = match (delta.before, delta.after) {
            (0, _) => "new",
            (_, 0) => "removed",
            _ => "",
        };
        table.row([
            signed(delta.before, delta.after),
            share(delta.after.abs_diff(delta.before), total),
            code(&delta.name),
            note.to_owned(),
        ]);
    }
    table
}

// ----------------------------------------------------------- dependencies

fn dependencies(md: &mut Md, report: &Report, total: u64) {
    let features = report.features.as_ref().filter(|features| !features.crates.is_empty());
    md.h2("Dependencies");

    if report.duplicates.is_empty() {
        md.line("No duplicate dependencies.");
        md.blank();
    } else {
        let count = report.duplicates.len();
        md.h3(&format!("Duplicate versions ({count})"));
        md.note("the same crate at several versions; each ships its own copy of the code, costed here from the compile units when the debug info was read");
        let mut table = Table::new(&[
            Col::Text("Crate"),
            Col::Text("Version"),
            Col::Right("Code"),
            Col::Text("Used by"),
        ]);
        for duplicate in &report.duplicates {
            for (index, version) in duplicate.versions.iter().enumerate() {
                let dependents = version
                    .dependents
                    .iter()
                    .map(|dependent| format!("{} {}", dependent.name, dependent.version))
                    .collect::<Vec<_>>()
                    .join(", ");
                // Zero bytes (generics instantiated, or everything inlined,
                // in its users) reads the same as no unit at all: nothing.
                let cost = match version.bytes {
                    Some(size) if size > 0 => bytes(size),
                    _ => String::new(),
                };
                let name = if index == 0 { code(&duplicate.name) } else { String::new() };
                table.row([name, version.version.clone(), cost, dependents]);
            }
        }
        md.table(table);
    }

    if let Some(features) = features {
        md.h3("Features");
        md.note("each linked dependency's resolved features and who asked for them; the bytes are the crate's whole code, so a shorter feature list returns some part of them");
        let mut table = Table::new(&[
            Col::Size,
            Col::Share,
            Col::Text("Crate"),
            Col::Text("Features"),
            Col::Text("Requested by"),
        ]);
        for krate in &features.crates {
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
            let (size, pct) = match krate.bytes {
                Some(size) => (bytes(size), share(size, total)),
                None => (String::new(), String::new()),
            };
            table.row([
                size,
                pct,
                code(&format!("{} {}", krate.name, krate.version)),
                krate.features.join(", "),
                requesters,
            ]);
        }
        md.table(table);
    }
}

// --------------------------------------------------------------- functions

fn functions(md: &mut Md, symbols: &SymbolReport, total: u64) {
    md.h2("Functions and data symbols");

    md.h3("Largest functions");
    let mut table =
        Table::new(&[Col::Size, Col::Share, Col::Text("Function"), Col::Text("Defined at")]);
    for symbol in &symbols.code.largest {
        table.row([
            bytes(symbol.size),
            share(symbol.size, total),
            named(symbol),
            site(symbol.defined_at.as_deref()),
        ]);
    }
    md.table(table);

    md.h3("Largest data symbols");
    if symbols.data.largest.iter().any(|symbol| !symbol.exact) {
        md.note("\u{2264} marks an upper bound: the size runs to the next symbol, so it also counts the unnamed constants in between");
    }
    let mut table = Table::new(&[Col::Size, Col::Share, Col::Text("Symbol")]);
    for symbol in &symbols.data.largest {
        let size = if symbol.exact {
            bytes(symbol.size)
        } else {
            format!("\u{2264} {}", bytes(symbol.size))
        };
        table.row([size, share(symbol.size, total), named(symbol)]);
    }
    md.table(table);

    md.h3("By pattern");
    md.note("a symbol can match several patterns, so these do not sum to the total; the \u{21b3} rows are the largest single offenders");
    let mut table =
        Table::new(&[Col::Size, Col::Share, Col::Text("Pattern"), Col::Right("Symbols")]);
    for pattern in &symbols.patterns {
        table.row([
            bytes(pattern.size),
            share(pattern.size, total),
            pattern.name.clone(),
            pattern.symbols.to_string(),
        ]);
        if pattern.largest.len() > 1 {
            for member in &pattern.largest {
                table.row([
                    bytes(member.bytes),
                    share(member.bytes, total),
                    member_row(&member.name),
                    String::new(),
                ]);
            }
        }
    }
    md.table(table);

    md.h3("By trait method, every impl combined");
    md.note("attribution — one method summed over every impl, so where the bytes sit, not what one change removes");
    md.table(groups(&symbols.trait_methods, total, "Trait method", "Impls"));

    md.h3("By trait, every method of every impl combined");
    md.note("one axis coarser than above; the \u{21b3} rows are the trait's largest single impls — the concrete targets");
    let mut table = Table::new(&[Col::Size, Col::Share, Col::Text("Trait"), Col::Right("Methods")]);
    for group in &symbols.traits {
        table.row([
            bytes(group.size),
            share(group.size, total),
            code(&group.name),
            group.methods.to_string(),
        ]);
        if group.largest.len() > 1 {
            for member in &group.largest {
                table.row([
                    bytes(member.bytes),
                    share(member.bytes, total),
                    member_row(&member.name),
                    String::new(),
                ]);
            }
        }
    }
    md.table(table);

    md.h3("By crate, where the code is defined");
    md.table(groups(&symbols.crates, total, "Crate", "Symbols"));

    md.h3("Generic families");
    md.note("every instantiation of one generic summed; recoverable = the total less its largest instance, what collapsing the family onto one copy would return");
    let mut table = Table::new(&[
        Col::Size,
        Col::Share,
        Col::Text("Family"),
        Col::Right("Instances"),
        Col::Right("Each"),
        Col::Right("Recoverable"),
        Col::Text("Defined at"),
    ]);
    for family in &symbols.generics {
        table.row([
            bytes(family.size),
            share(family.size, total),
            code(&family.name),
            family.instantiations.to_string(),
            bytes(family.each),
            format!("~{}", bytes(family.recoverable)),
            site(family.defined_at.as_deref()),
        ]);
    }
    md.table(table);

    md.h3("By crate, which one caused the instantiation");
    md.note("generic code from the families above, re-attributed to the crate that instantiated it — not additional");
    md.table(groups(&symbols.instantiated_by, total, "Crate", "Symbols"));
}

/// One row per group, all of which differ only in what they count.
fn groups(groups: &[Group], total: u64, what: &'static str, unit: &'static str) -> Table {
    let mut table = Table::new(&[Col::Size, Col::Share, Col::Text(what), Col::Right(unit)]);
    for entry in groups {
        table.row([
            bytes(entry.size),
            share(entry.size, total),
            code(&entry.name),
            entry.symbols.to_string(),
        ]);
    }
    table
}

fn argument_types(md: &mut Md, instantiations: &InstantiationReport, total: u64) {
    if instantiations.crates.is_empty() {
        return;
    }
    md.h2("Generic code, by the types it is instantiated over");
    md.note("a turbofish names the types a generic was specialized to; bytes count toward every crate those types name, so rows overlap — the \u{21b3} rows are the largest generic families within each");
    let combined = instantiations.bytes + instantiations.inlined_bytes;
    let found = if instantiations.inlined_bytes > 0 {
        format!(
            "in {} symbols and {} inlined instances",
            instantiations.symbols, instantiations.instances
        )
    } else {
        format!("in {} symbols", instantiations.symbols)
    };
    md.line(&format!("- {} ({}) {found}", bytes(combined), share(combined, total)));
    md.blank();

    let mut table = Table::new(&[
        Col::Size,
        Col::Share,
        Col::Text("Argument crate"),
        Col::Right("Instantiations"),
    ]);
    for entry in &instantiations.crates {
        table.row([
            bytes(entry.bytes),
            share(entry.bytes, total),
            code(&entry.name),
            entry.instantiations.to_string(),
        ]);
        for family in &entry.largest {
            table.row([
                bytes(family.bytes),
                share(family.bytes, total),
                member_row(&family.name),
                String::new(),
            ]);
        }
    }
    md.table(table);
}

// ---------------------------------------------------------------- overhead

fn overhead_section(md: &mut Md, overhead: &OverheadReport, total: u64) {
    let data_bytes: u64 = overhead.data.iter().map(|group| group.bytes).sum();
    if overhead.unwind == 0 && data_bytes == 0 {
        return;
    }
    md.h2("Panic, format, and unwind overhead");
    md.note("infrastructure the code views only hint at; panic=\"abort\" drops the unwind tables, -Zbuild-std with panic_immediate_abort strips the panic locations, disabling tracing removes the callsite metadata");
    let mut table = Table::new(&[Col::Size, Col::Share, Col::Text("What"), Col::Right("Symbols")]);
    table.row([
        bytes(overhead.unwind),
        share(overhead.unwind, total),
        "unwind and exception tables".to_owned(),
        String::new(),
    ]);
    for group in &overhead.data {
        table.row([
            bytes(group.bytes),
            share(group.bytes, total),
            group.kind.clone(),
            group.symbols.to_string(),
        ]);
    }
    md.table(table);
}

fn duplicate_data(md: &mut Md, dupdata: &DupDataReport, total: u64) {
    if dupdata.largest.is_empty() {
        return;
    }
    md.h2("Duplicate read-only data");
    md.note(&format!(
        "byte-identical constants under different names; the linker's --icf or sharing one const collapses them — {} recoverable from {} groups ({} symbols)",
        bytes(dupdata.recoverable),
        dupdata.groups,
        dupdata.symbols
    ));
    let mut table = Table::new(&[
        Col::Right("Recoverable"),
        Col::Share,
        Col::Text("Constants"),
        Col::Right("Copies"),
        Col::Right("Each"),
    ]);
    for group in &dupdata.largest {
        table.row([
            bytes(group.recoverable),
            share(group.recoverable, total),
            same_names(&group.names),
            group.names.len().to_string(),
            bytes(group.size),
        ]);
    }
    md.table(table);
}

/// A group of names that are one item repeated, or a few distinct ones.
fn same_names(names: &[String]) -> String {
    let first = names.first().map(String::as_str).unwrap_or_default();
    if names.iter().all(|name| name == first) {
        return code(first);
    }
    let others: Vec<String> = names.iter().skip(1).take(2).map(|name| code(name)).collect();
    let more = names.len().saturating_sub(1 + others.len());
    let more = if more > 0 { format!(" \u{2261} {more} more") } else { String::new() };
    format!("{} \u{2261} {}{more}", code(first), others.join(" \u{2261} "))
}

// ---------------------------------------------------------------- dispatch

fn dispatch(
    md: &mut Md,
    dispatch: Option<&DispatchReport>,
    graph: Option<&GraphReport>,
    total: u64,
) {
    let named = dispatch.filter(|d| d.vtables.count > 0 || d.shims.count > 0);
    let by_trait = graph.filter(|g| !g.vtables.is_empty());
    if named.is_none() && by_trait.is_none() {
        return;
    }
    md.h2("Dynamic dispatch");

    if let Some(dispatch) = named {
        md.h3("Named vtables and shims");
        md.note("a proxy: the few vtables and fn-pointer shims that carry a symbol; most vtables are anonymous and counted below");
        md.line(&format!(
            "- {} ({}) in {} vtables; {} ({}) in {} coercion and drop shims.",
            bytes(dispatch.vtables.bytes),
            share(dispatch.vtables.bytes, total),
            dispatch.vtables.count,
            bytes(dispatch.shims.bytes),
            share(dispatch.shims.bytes, total),
            dispatch.shims.count,
        ));
        md.blank();
        let mut table = Table::new(&[Col::Size, Col::Share, Col::Text("Symbol")]);
        for symbol in &dispatch.largest {
            table.row([bytes(symbol.size), share(symbol.size, total), code(&symbol.name)]);
        }
        md.table(table);
    }

    if let Some(graph) = by_trait {
        md.h3("Vtables by trait object");
        md.note("recovered from the function pointers each anonymous vtable carries; bytes are the vtables themselves, not the methods they point at");
        let mut table =
            Table::new(&[Col::Size, Col::Share, Col::Text("Trait object"), Col::Right("Vtables")]);
        for group in &graph.vtables {
            table.row([
                bytes(group.bytes),
                share(group.bytes, total),
                code(&format!("dyn {}", group.name)),
                group.count.to_string(),
            ]);
        }
        md.table(table);
    }
}

// ---------------------------------------------------------- reference graph

fn reference_graph(md: &mut Md, graph: &GraphReport, total: u64) {
    if graph.single_callers.is_empty()
        && graph.retained.is_empty()
        && graph.unreachable.symbols == 0
    {
        return;
    }
    md.h2("Reference graph");
    md.note("who calls, addresses, and points at whom, read from the assembly; conservative, since an indirect call it cannot name may retain more");

    if !graph.single_callers.is_empty() {
        md.h3("Called from one place");
        md.note("nothing else reaches these — each exists for a single call site, where merging or inlining it would land");
        let mut table = Table::new(&[
            Col::Size,
            Col::Share,
            Col::Text("Function"),
            Col::Text("Only caller"),
            Col::Text("Defined at"),
        ]);
        for single in &graph.single_callers {
            table.row([
                bytes(single.bytes),
                share(single.bytes, total),
                code(&single.name),
                code(&single.caller),
                site(single.defined_at.as_deref()),
            ]);
        }
        md.table(table);
    }

    if !graph.retained.is_empty() {
        md.h3("Removing a function frees");
        md.note("itself plus everything only it reaches, code and constants alike — dominators of the reference graph from the entry point");
        let mut table = Table::new(&[
            Col::Right("Frees"),
            Col::Share,
            Col::Text("Function"),
            Col::Right("Itself"),
            Col::Right("Symbols with it"),
        ]);
        for entry in &graph.retained {
            table.row([
                bytes(entry.retained),
                share(entry.retained, total),
                code(&entry.name),
                bytes(entry.own),
                entry.dominated.to_string(),
            ]);
        }
        md.table(table);
    }

    if graph.unreachable.symbols > 0 {
        md.h3("Reached by no reference the graph can see");
        md.line(&format!(
            "- {} ({}) in {} functions: linked code with no path of calls, addresses, or data slots from the entry — kept by something the assembly does not name.",
            bytes(graph.unreachable.bytes),
            share(graph.unreachable.bytes, total),
            graph.unreachable.symbols
        ));
        md.blank();
    }
}

// ----------------------------------------------------------------- derives

fn derives(md: &mut Md, categories: &CategoryReport, total: u64) {
    if categories.derives.is_empty() && categories.cold == 0 {
        return;
    }
    md.h2("Derives");
    if !categories.derives.is_empty() {
        md.note("every impl of the trait, derived or hand-written alike; the total is attribution across many types, not a saving — the \u{21b3} rows are the largest single impls, the ones worth acting on");
        let mut table =
            Table::new(&[Col::Size, Col::Share, Col::Text("Derive"), Col::Right("Impls")]);
        for derive in &categories.derives {
            table.row([
                bytes(derive.bytes),
                share(derive.bytes, total),
                derive.name.clone(),
                derive.impls.to_string(),
            ]);
            if derive.largest.len() > 1 {
                for member in &derive.largest {
                    table.row([
                        bytes(member.bytes),
                        share(member.bytes, total),
                        member_row(&member.name),
                        String::new(),
                    ]);
                }
            }
        }
        md.table(table);
    }
    if categories.cold > 0 {
        md.h3("Cold code");
        md.line(&format!(
            "- {} ({}) split off for panic and error paths, in .text.unlikely.",
            bytes(categories.cold),
            share(categories.cold, total)
        ));
        md.blank();
    }
}

// ------------------------------------------------------------------- types

fn largest_types(md: &mut Md, types: &TypeReport) {
    if types.largest.is_empty() {
        return;
    }
    md.h2("Largest types");
    md.note("in-memory layout size, not a share of the binary; a large type drives the moves, copies, and drop glue, and an enum is as large as its largest variant, so boxing that one shrinks every value");
    let mut table = Table::new(&[Col::Size, Col::Text("Type"), Col::Text("Layout")]);
    for ty in &types.largest {
        table.row([bytes(ty.size), code(&ty.name), layout_note(ty)]);
    }
    md.table(table);
}

/// What a type's layout says: its largest variants and what boxing the
/// largest saves, or its padding.
fn layout_note(ty: &NamedType) -> String {
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
        return format!("variants {variants}{more}{boxing}");
    }
    match ty.padding {
        Some(padding) if padding >= 8 => format!("{} padding", bytes(padding)),
        _ => String::new(),
    }
}

// ----------------------------------------------------------------- inlined

fn inlined_code(md: &mut Md, inlined: &InlineReport, total: u64) {
    md.h2("Inlined code");
    md.line(&format!(
        "- {} ({}) in {} inlined instances, charged to their callers.",
        bytes(inlined.bytes),
        share(inlined.bytes, total),
        inlined.instances
    ));
    md.blank();

    md.h3("Largest inlined functions");
    md.note("bytes across every site a function was inlined into, counting only instructions not attributed to a deeper inline");
    let mut table = Table::new(&[
        Col::Size,
        Col::Share,
        Col::Text("Function"),
        Col::Right("Sites"),
        Col::Text("Defined at"),
    ]);
    for function in &inlined.functions {
        table.row([
            bytes(function.bytes),
            share(function.bytes, total),
            code(&function.name),
            function.sites.to_string(),
            site(function.defined_at.as_deref()),
        ]);
    }
    md.table(table);

    if !inlined.workspace_call_sites.is_empty() {
        md.h3("Workspace lines that pulled in the most inlined code");
        md.table(call_sites(&inlined.workspace_call_sites, total));
    }
}

fn call_sites(sites: &[CallSite], total: u64) -> Table {
    let mut table = Table::new(&[
        Col::Size,
        Col::Share,
        Col::Text("Line"),
        Col::Right("Inlined"),
        Col::Text("Source"),
    ]);
    for site in sites {
        table.row([
            bytes(site.bytes),
            share(site.bytes, total),
            code(&format!("{}:{}", site.file, site.line)),
            site.instances.to_string(),
            site.snippet.as_deref().map(code).unwrap_or_default(),
        ]);
    }
    table
}

// ---------------------------------------------------------------- assembly

fn assembly_section(md: &mut Md, assembly: &AssemblyReport, total: u64) {
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

    md.h2("Assembly");
    let path = match assembly.paths.as_slice() {
        [] => String::new(),
        [path] => format!("`{path}`"),
        [path, rest @ ..] => format!("`{path}` and {} more", rest.len()),
    };
    md.line(&format!(
        "- {} ({}) of code in {} functions with assembly, {} instructions, {each:.1} B each; from {path}.",
        bytes(assembly.bytes),
        share(assembly.bytes, total),
        assembly.linked,
        assembly.instructions
    ));
    if assembly.functions > assembly.linked {
        md.line(&format!(
            "- {} more functions in the assembly never reached the binary.",
            assembly.functions - assembly.linked
        ));
    }
    md.line("- `~` sizes below are instruction counts converted at that rate.");
    md.blank();

    identical(md, &assembly.identical, total);
    panics(md, &assembly.panics, total, &approx);
    formatting(md, &assembly.formatting, total, &approx);
    copies(md, &assembly.copies, total, &approx);

    if !assembly.workspace_lines.is_empty() {
        md.h3("Workspace lines compiled to the most instructions");
        md.note("the line an instruction came from, after inlining, every instantiation summed");
        let mut table = Table::new(&[
            Col::Right("~Size"),
            Col::Share,
            Col::Text("Line"),
            Col::Right("Instructions"),
            Col::Text("Source"),
        ]);
        for line in &assembly.workspace_lines {
            let size = approx(line.instructions);
            table.row([
                approximate(size),
                share(size, total),
                code(&format!("{}:{}", line.file, line.line)),
                line.instructions.to_string(),
                line.snippet.as_deref().map(code).unwrap_or_default(),
            ]);
        }
        md.table(table);
    }
}

fn identical(md: &mut Md, identical: &Identical, total: u64) {
    md.h3("Identical function bodies");
    md.note(&format!(
        "the same instructions under different names; a linker folding identical code keeps one, and so does instantiating one — {} recoverable from {} groups ({} functions)",
        bytes(identical.recoverable),
        identical.groups,
        identical.functions
    ));
    let mut table = Table::new(&[
        Col::Right("Recoverable"),
        Col::Share,
        Col::Text("Functions"),
        Col::Right("Copies"),
        Col::Right("Each"),
    ]);
    for group in &identical.largest {
        table.row([
            bytes(group.recoverable),
            share(group.recoverable, total),
            same_names(&group.names),
            group.names.len().to_string(),
            bytes(group.bytes),
        ]);
    }
    md.table(table);
}

fn panics(md: &mut Md, panics: &Panics, total: u64, approx: &dyn Fn(u64) -> u64) {
    md.h3("Panic call sites");
    md.note("each is a compare, a branch, and a cold block that loads the location and calls; the location is 24 B more of read-only data");
    let size = approx(panics.instructions);
    md.line(&format!(
        "- {} ({}) in the blocks of {} sites: {} bounds checks, {} unwraps, {} allocation failures, {} other; {} distinct locations and messages loaded by them.",
        approximate(size),
        share(size, total),
        panics.sites,
        panics.bounds_checks,
        panics.unwraps,
        panics.allocation,
        panics.other,
        panics.constants
    ));
    md.blank();
    md.table(callers(&panics.functions, total, approx));
}

fn formatting(md: &mut Md, formatting: &Formatting, total: u64, approx: &dyn Fn(u64) -> u64) {
    md.h3("Formatting call sites");
    md.note("calls into core::fmt and alloc::fmt; the block before each builds the Arguments");
    let size = approx(formatting.instructions);
    md.line(&format!(
        "- {} ({}) in the blocks of {} sites.",
        approximate(size),
        share(size, total),
        formatting.sites
    ));
    md.blank();
    md.table(callers(&formatting.functions, total, approx));
}

fn callers(callers: &[Caller], total: u64, approx: &dyn Fn(u64) -> u64) -> Table {
    let mut table = Table::new(&[
        Col::Right("~Size"),
        Col::Share,
        Col::Text("Function"),
        Col::Right("Sites"),
        Col::Right("Instructions"),
    ]);
    for caller in callers {
        let size = approx(caller.instructions);
        table.row([
            approximate(size),
            share(size, total),
            code(&caller.name),
            caller.sites.to_string(),
            caller.instructions.to_string(),
        ]);
    }
    table
}

fn copies(md: &mut Md, copies: &Copies, total: u64, approx: &dyn Fn(u64) -> u64) {
    md.h3("Values copied through memory");
    md.note(&format!(
        "runs of {COPY_RUN} or more loads and stores back to back, and calls to memcpy for anything larger; boxing the value or passing it by reference removes them"
    ));
    let size = approx(copies.instructions);
    md.line(&format!(
        "- {} ({}) in {} runs, {} instructions, plus {} memcpy-family calls.",
        approximate(size),
        share(size, total),
        copies.runs,
        copies.instructions,
        copies.calls
    ));
    md.blank();
    let mut table = Table::new(&[
        Col::Right("~Size"),
        Col::Share,
        Col::Text("Function"),
        Col::Right("Instructions"),
        Col::Right("Runs"),
        Col::Right("Calls"),
    ]);
    for copier in &copies.functions {
        let size = approx(copier.instructions);
        table.row([
            approximate(size),
            share(size, total),
            code(&copier.name),
            copier.instructions.to_string(),
            copier.runs.to_string(),
            copier.calls.to_string(),
        ]);
    }
    md.table(table);
}

// --------------------------------------------------------------- constants

fn constant_data(md: &mut Md, constants: &ConstantsReport, section_bytes: u64, total: u64) {
    if constants.linked == 0 {
        return;
    }
    md.h2("Constant data");
    md.note("every constant the assembly spells out, sized from its directives and read by shape; only what a linked function reaches counts, and under lto=\"fat\" that is the whole program");
    md.line(&format!(
        "- {} ({}) in {} constants a linked function reaches ({} defined), against {} of read-only data sections.",
        bytes(constants.bytes),
        share(constants.bytes, total),
        constants.linked,
        constants.constants,
        bytes(section_bytes)
    ));
    md.blank();

    md.h3("By kind");
    let mut table =
        Table::new(&[Col::Size, Col::Share, Col::Text("Kind"), Col::Right("Constants")]);
    for class in &constants.classes {
        table.row([
            bytes(class.bytes),
            share(class.bytes, total),
            class.kind.label().to_owned(),
            class.count.to_string(),
        ]);
    }
    if constants.panic_messages.count > 0 {
        table.row([
            bytes(constants.panic_messages.bytes),
            share(constants.panic_messages.bytes, total),
            "of the text: loaded only on the way to a panic (messages and their pieces)".to_owned(),
            constants.panic_messages.count.to_string(),
        ]);
    }
    md.table(table);

    let locations = &constants.locations;
    if locations.records > 0 {
        md.h3("Panic locations");
        md.note(&format!(
            "a 24 B record per panic site — an unwrap, an index, an expect — plus the source path once: {} records in all, {}",
            locations.records,
            bytes(locations.bytes)
        ));
        if !locations.workspace_files.is_empty() {
            let mut table = Table::new(&[
                Col::Size,
                Col::Share,
                Col::Text("Workspace file"),
                Col::Right("Records"),
                Col::Text("Most on lines"),
            ]);
            for file in &locations.workspace_files {
                let lines = file.lines.iter().map(u64::to_string).collect::<Vec<_>>().join(", ");
                table.row([
                    bytes(file.bytes),
                    share(file.bytes, total),
                    code(&file.file),
                    file.records.to_string(),
                    lines,
                ]);
            }
            md.table(table);
        }
        let mut table = Table::new(&[
            Col::Size,
            Col::Share,
            Col::Text("Function loading the most records"),
            Col::Right("Records"),
        ]);
        for caller in &locations.functions {
            table.row([
                bytes(caller.bytes),
                share(caller.bytes, total),
                code(&caller.name),
                caller.records.to_string(),
            ]);
        }
        md.table(table);
    }

    if !constants.strings.is_empty() {
        md.h3("Largest strings");
        let mut table = Table::new(&[
            Col::Size,
            Col::Share,
            Col::Text("String"),
            Col::Text("Kind"),
            Col::Text("Loaded by"),
        ]);
        for string in &constants.strings {
            let loaders = match string.functions.as_slice() {
                [] => String::new(),
                [only] if string.references == 1 => code(only),
                [first, ..] => format!("{} and {} more", code(first), string.references - 1),
            };
            table.row([
                bytes(string.bytes),
                share(string.bytes, total),
                code(&format!("\"{}\"", string.preview)),
                kind_word(string.kind).to_owned(),
                loaders,
            ]);
        }
        md.table(table);
    }

    if !constants.tables.is_empty() {
        md.h3("Lookup and jump tables");
        md.note("by the function whose match built them");
        let mut table = Table::new(&[
            Col::Size,
            Col::Share,
            Col::Text("Function"),
            Col::Right("Lookup"),
            Col::Right("Jump"),
        ]);
        for entry in &constants.tables {
            table.row([
                bytes(entry.bytes),
                share(entry.bytes, total),
                code(&entry.name),
                entry.switch_tables.to_string(),
                entry.jump_tables.to_string(),
            ]);
        }
        md.table(table);
    }

    if !constants.functions.is_empty() {
        md.h3("Functions carrying the most constant data");
        md.note("ranked by what only that function reaches, directly or through a table's pointers — what rewriting it alone frees; the total counts shared constants too");
        let mut table = Table::new(&[
            Col::Right("Exclusive"),
            Col::Share,
            Col::Text("Function"),
            Col::Right("Constants"),
            Col::Right("Total"),
        ]);
        for carrier in &constants.functions {
            table.row([
                bytes(carrier.exclusive),
                share(carrier.exclusive, total),
                code(&carrier.name),
                carrier.constants.to_string(),
                bytes(carrier.bytes),
            ]);
        }
        md.table(table);
    }
}

/// A string's kind, as one word.
const fn kind_word(kind: constants::Kind) -> &'static str {
    match kind {
        constants::Kind::Path => "path",
        constants::Kind::Name => "name",
        _ => "message",
    }
}

/// Only where the records cost file bytes is there something to show: ELF
/// always, Mach-O for its rebase opcodes.
fn dynamic_relocations(md: &mut Md, relocations: &RelocationReport, total: u64) {
    if relocations.bytes == 0 {
        return;
    }
    md.h2("Dynamic relocations");
    if relocations.record > 0 {
        md.note(&format!(
            "every pointer kept in data is a slot the loader fills at start, and each costs a {} B record here on top of its 8 B; tables of &str, vtables, and panic locations are where they come from — offsets instead of pointers remove both",
            relocations.record
        ));
    } else {
        md.note("every pointer kept in data is a slot the loader fills at start; the records are compressed, so the slot's own 8 B is the cost — offsets instead of pointers remove it");
    }
    let packed = if relocations.packed { "packed " } else { "" };
    md.line(&format!(
        "- {} ({}) in {packed}relocation records for {} pointer slots ({} of slots).",
        bytes(relocations.bytes),
        share(relocations.bytes, total),
        relocations.slots,
        bytes(relocations.slots as u64 * 8)
    ));
    md.blank();
    if relocations.record > 0 && !relocations.symbols.is_empty() {
        let mut table =
            Table::new(&[Col::Size, Col::Share, Col::Text("Data symbol"), Col::Right("Slots")]);
        for group in &relocations.symbols {
            table.row([
                bytes(group.bytes),
                share(group.bytes, total),
                code(&group.name),
                group.slots.to_string(),
            ]);
        }
        md.table(table);
    }
}

// ----------------------------------------------------------------- opt-ins

/// IR lines are not binary bytes — the optimizer deletes much of this — so
/// this shows line counts and instantiations, no size or share.
fn llvm_ir(md: &mut Md, ir: &IrReport) {
    md.h2("LLVM IR by generic family");
    md.note(&format!(
        "pre-optimization IR lines, not binary bytes — where the code comes from before the optimizer deletes it: {} lines across {} functions in {} crates",
        ir.lines, ir.functions, ir.files
    ));
    let mut table =
        Table::new(&[Col::Right("Lines"), Col::Text("Family"), Col::Right("Instantiations")]);
    for family in &ir.families {
        table.row([
            family.lines.to_string(),
            code(&family.name),
            family.instantiations.to_string(),
        ]);
    }
    md.table(table);
}

/// MIR statement estimates are not bytes either — the backend inlines and
/// deletes much of this — so this shows the estimates alone.
fn mono_stats(md: &mut Md, mono: &MonoReport) {
    if mono.largest.is_empty() {
        return;
    }
    md.h2("Generic definitions by estimated codegen cost");
    md.note(&format!(
        "MIR statements handed to the backend, every instantiation summed, before inlining — an estimate, not bytes; large and often instantiated is what to split into a small generic shell over a non-generic body: {} definitions, {} instantiations, ~{} statements across {} crates",
        mono.definitions, mono.instantiations, mono.estimate, mono.crates
    ));
    let mut table = Table::new(&[
        Col::Right("~Statements"),
        Col::Text("Definition"),
        Col::Right("Instantiations"),
        Col::Right("Each"),
        Col::Text("Instantiated in"),
    ]);
    for definition in &mono.largest {
        // A crate's own items are spelled without their crate, so name it.
        let more = definition.crates.saturating_sub(definition.crate_names.len());
        let crates = match (definition.crate_names.as_slice(), more) {
            ([], _) => String::new(),
            (names, 0) => names.join(", "),
            (names, more) => format!("{} and {more} more", names.join(", ")),
        };
        table.row([
            definition.estimate.to_string(),
            code(&definition.name),
            definition.instantiations.to_string(),
            definition.each.to_string(),
            crates,
        ]);
    }
    md.table(table);
}

/// Loop expansions are counted, not sized — the remark says what was done,
/// not how many bytes it made — so this shows counts alone.
fn expanded_loops(md: &mut Md, remarks: &RemarksReport) {
    if remarks.functions.is_empty() {
        return;
    }
    md.h2("Loops the optimizer expanded");
    md.note(&format!(
        "unrolled, peeled, or vectorized — body copies the source never wrote, from each crate's own optimization remarks; a simpler loop, #[inline(never)], or #[cold] takes them back: {} unrolled, {} peeled, {} vectorized, in {} remark files",
        remarks.unrolled, remarks.peeled, remarks.vectorized, remarks.files
    ));
    let mut table = Table::new(&[
        Col::Right("Loops"),
        Col::Text("Function"),
        Col::Right("Unrolled"),
        Col::Right("Body copies"),
        Col::Right("Peeled"),
        Col::Right("Vectorized"),
    ]);
    for function in &remarks.functions {
        table.row([
            (function.unrolled + function.peeled + function.vectorized).to_string(),
            code(&function.name),
            function.unrolled.to_string(),
            function.copies.to_string(),
            function.peeled.to_string(),
            function.vectorized.to_string(),
        ]);
    }
    md.table(table);

    if !remarks.workspace_sites.is_empty() {
        md.h3("Workspace loops the optimizer expanded");
        let mut table = Table::new(&[
            Col::Text("What"),
            Col::Text("Line"),
            Col::Text("Function"),
            Col::Text("Source"),
        ]);
        for site in &remarks.workspace_sites {
            table.row([
                site.detail.clone(),
                code(&format!("{}:{}", site.file, site.line)),
                code(&site.function),
                site.snippet.as_deref().map(code).unwrap_or_default(),
            ]);
        }
        md.table(table);
    }
}

fn what_if(md: &mut Md, whatif: &WhatIfReport, total: u64) {
    if whatif.levers.is_empty() && whatif.skipped.is_empty() {
        return;
    }
    md.h2("What-if, measured by rebuilding");
    md.note("the change in shipped size under each build lever — a measurement, not a proposal; beneath each, the functions that moved most, which is where that cost sits in the source");
    if !whatif.levers.is_empty() {
        let mut table = Table::new(&[
            Col::Right("Change"),
            Col::Share,
            Col::Text("Lever"),
            Col::Right("Before"),
            Col::Right("After"),
        ]);
        for lever in &whatif.levers {
            table.row([
                signed(lever.before, lever.after),
                share(lever.before.abs_diff(lever.after), total),
                code(&lever.name),
                bytes(lever.before),
                bytes(lever.after),
            ]);
        }
        md.table(table);
    }
    if !whatif.skipped.is_empty() {
        md.line(&format!("- Skipped, the build failed: {}.", whatif.skipped.join(", ")));
        md.blank();
    }
    for lever in &whatif.levers {
        if lever.diff.symbols.is_empty() {
            continue;
        }
        md.h3(&format!("Under `{}`", lever.name));
        if let Some(reading) = whatif::reading(&lever.name) {
            md.note(reading);
        }
        let movers: Vec<NamedDelta> = lever
            .diff
            .symbols
            .iter()
            .take(WHATIF_MOVERS)
            .map(|delta| NamedDelta {
                name: delta.name.clone(),
                before: delta.before,
                after: delta.after,
            })
            .collect();
        md.table(deltas(&movers, total, "Function"));
    }
}

// ----------------------------------------------------------------- helpers

/// The document being built: sections are written to a body while their
/// titles are collected, so the contents list can lead.
#[derive(Default)]
struct Md {
    head: String,
    body: String,
    contents: Vec<String>,
}

impl Md {
    /// The title block, before the contents.
    fn title(&mut self, text: &str) {
        self.head.push_str(text);
    }

    fn h2(&mut self, title: &str) {
        self.contents.push(title.to_owned());
        // One blank line before a section, whatever the last thing wrote.
        if !self.body.is_empty() && !self.body.ends_with("\n\n") {
            self.body.push('\n');
        }
        let _ = writeln!(self.body, "## {title}\n");
    }

    fn h3(&mut self, title: &str) {
        let _ = writeln!(self.body, "### {title}\n");
    }

    /// A one-line note on what a section means.
    fn note(&mut self, text: &str) {
        let _ = writeln!(self.body, "_{text}_\n");
    }

    fn line(&mut self, text: &str) {
        let _ = writeln!(self.body, "{text}");
    }

    fn blank(&mut self) {
        self.body.push('\n');
    }

    fn table(&mut self, table: Table) {
        table.write(&mut self.body);
    }

    fn write<W: io::Write>(self, writer: &mut W) -> io::Result<()> {
        writer.write_all(self.head.as_bytes())?;
        if self.contents.len() > 1 {
            let links = self
                .contents
                .iter()
                .map(|title| format!("[{title}](#{})", anchor(title)))
                .collect::<Vec<_>>()
                .join(" \u{b7} ");
            writeln!(writer, "Contents: {links}\n")?;
        }
        writer.write_all(self.body.as_bytes())
    }
}

/// A GitHub-style heading anchor.
fn anchor(title: &str) -> String {
    title
        .chars()
        .filter_map(|c| match c {
            ' ' => Some('-'),
            c if c.is_alphanumeric() || c == '-' => Some(c.to_ascii_lowercase()),
            _ => None,
        })
        .collect()
}

/// A column: its header and how its cells align. Numbers align right and are
/// padded so the raw text reads as columns too; text is left as it comes.
#[derive(Clone, Copy)]
enum Col {
    /// A byte size.
    Size,
    /// A share of the shipped size.
    Share,
    Right(&'static str),
    Text(&'static str),
}

impl Col {
    fn header(self) -> &'static str {
        match self {
            Self::Size => "Size",
            Self::Share => "Share",
            Self::Right(name) | Self::Text(name) => name,
        }
    }

    const fn is_right(self) -> bool {
        matches!(self, Self::Size | Self::Share | Self::Right(_))
    }
}

/// A Markdown table.
struct Table {
    columns: Vec<Col>,
    rows: Vec<Vec<String>>,
}

impl Table {
    fn new(columns: &[Col]) -> Self {
        Self { columns: columns.to_vec(), rows: Vec::new() }
    }

    fn row<const N: usize>(&mut self, cells: [String; N]) {
        debug_assert_eq!(N, self.columns.len());
        self.rows.push(cells.into_iter().map(|cell| cell.replace('|', "\\|")).collect());
    }

    fn write(self, out: &mut String) {
        if self.rows.is_empty() {
            return;
        }
        let widths: Vec<usize> = self
            .columns
            .iter()
            .enumerate()
            .map(|(index, col)| {
                if !col.is_right() {
                    return 0;
                }
                self.rows
                    .iter()
                    .map(|row| row[index].chars().count())
                    .chain(std::iter::once(col.header().len()))
                    .max()
                    .unwrap_or(0)
            })
            .collect();

        out.push('|');
        for (col, width) in self.columns.iter().zip(&widths) {
            let _ = write!(out, " {:>width$} |", col.header(), width = *width);
        }
        out.push_str("\n|");
        for (col, width) in self.columns.iter().zip(&widths) {
            if col.is_right() {
                let _ = write!(out, "{}:|", "-".repeat((*width + 1).max(3)));
            } else {
                out.push_str("---|");
            }
        }
        out.push('\n');
        for row in &self.rows {
            out.push('|');
            for (cell, width) in row.iter().zip(&widths) {
                let _ = write!(out, " {cell:>width$} |", width = *width);
            }
            out.push('\n');
        }
        out.push('\n');
    }
}

/// A name in a code span, so `<T as Trait>` is never read as HTML.
fn code(text: &str) -> String {
    // A backtick inside the span needs a longer fence, padded with a space.
    let longest = text.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    if longest == 0 {
        format!("`{text}`")
    } else {
        let fence = "`".repeat(longest + 1);
        format!("{fence} {text} {fence}")
    }
}

/// A member row: the largest single item under a group.
fn member_row(name: &str) -> String {
    format!("\u{21b3} {}", code(name))
}

/// Name a symbol, flagging when the binary carries more than one copy of it.
fn named(symbol: &Symbol) -> String {
    if symbol.copies > 1 {
        format!("{} ({}\u{d7})", code(&symbol.name), symbol.copies)
    } else {
        code(&symbol.name)
    }
}

/// A definition site, or nothing.
fn site(site: Option<&str>) -> String {
    site.map(code).unwrap_or_default()
}

/// A size converted from an instruction count, so marked approximate.
fn approximate(size: u64) -> String {
    format!("~{}", bytes(size))
}

/// A byte delta with a leading sign, computed without an `i64` cast.
fn signed(before: u64, after: u64) -> String {
    if after >= before {
        format!("+{}", bytes(after - before))
    } else {
        format!("-{}", bytes(before - after))
    }
}

/// A share of `total`, as a percentage; a share too small to show as one
/// decimal reads `<0.1%` rather than nothing.
fn share(size: u64, total: u64) -> String {
    #[expect(clippy::cast_precision_loss, reason = "display only")]
    let ratio = if total == 0 { 0.0 } else { size as f64 / total as f64 };
    let percent = ratio * 100.0;
    if size > 0 && percent < 0.05 {
        return "<0.1%".to_owned();
    }
    format!("{percent:.1}%")
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
    use super::{Col, Table, anchor, code, share};

    #[test]
    fn tables_escape_pipes_and_align_numbers() {
        let mut table = Table::new(&[Col::Size, Col::Share, Col::Text("Name")]);
        table.row(["1.0 KiB".to_owned(), "50.0%".to_owned(), "`a|b`".to_owned()]);
        table.row(["12 B".to_owned(), "<0.1%".to_owned(), "c".to_owned()]);
        let mut out = String::new();
        table.write(&mut out);
        assert_eq!(
            out,
            "|    Size | Share | Name |\n|--------:|------:|---|\n| 1.0 KiB | 50.0% | `a\\|b` |\n|    12 B | <0.1% | c |\n\n"
        );

        assert_eq!(share(1, 100_000), "<0.1%");
        assert_eq!(share(0, 100), "0.0%");
        assert_eq!(share(1, 8), "12.5%");
        assert_eq!(
            anchor("Panic, format, and unwind overhead"),
            "panic-format-and-unwind-overhead"
        );
        assert_eq!(code("a"), "`a`");
        assert_eq!(code("run with `RUST_BACKTRACE=full`"), "`` run with `RUST_BACKTRACE=full` ``");
    }
}
