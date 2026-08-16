//! Rendering of analysis results.

use std::{fmt, io, str::FromStr};

use serde::Serialize;

use crate::{
    assembly::{AssemblyReport, COPY_RUN, Caller, Line},
    duplicates::Duplicate,
    inlined::InlineReport,
    sections::BinaryReport,
    symbols::{Group, Symbol, SymbolReport},
};

/// An object rather than a bare array, so later analyses can be added without
/// breaking the schema.
#[derive(Debug, Serialize)]
pub struct Report {
    pub duplicates: Vec<Duplicate>,
    pub binary: Option<BinaryReport>,
    pub symbols: Option<SymbolReport>,
    pub inlined: Option<InlineReport>,
    pub assembly: Option<AssemblyReport>,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

impl FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" => Ok(Self::Text),
            "json" => Ok(Self::Json),
            _ => Err(format!("unknown format: {s}, expected: text, json")),
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
    if let Some(binary) = &report.binary {
        render_binary(writer, binary, limit)?;

        if let Some(symbols) = &report.symbols {
            render_symbols(writer, symbols, binary.shipped)?;
        }

        if let Some(inlined) = &report.inlined {
            render_inlined(writer, inlined, binary.shipped)?;
        }

        if let Some(assembly) = &report.assembly {
            render_assembly(writer, assembly, binary.shipped)?;
        }
    }

    render_duplicates(writer, &report.duplicates)
}

fn render_symbols<W: io::Write>(
    writer: &mut W,
    symbols: &SymbolReport,
    total: u64,
) -> io::Result<()> {
    let named = format_args!("code in {} named symbols", symbols.code.count);
    row(writer, symbols.code.bytes, total, named)?;
    let named = format_args!("read-only data in {} named symbols", symbols.data.count);
    row(writer, symbols.data.bytes, total, named)?;

    let anonymous = (symbols.code.section_bytes + symbols.data.section_bytes)
        .saturating_sub(symbols.code.bytes + symbols.data.bytes);
    row(writer, anonymous, total, "in those sections, named by no symbol")?;

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
    writeln!(writer, "  (a symbol can match several, so these do not sum to the total)")?;
    groups(writer, &symbols.patterns, total, "symbols")?;

    writeln!(writer, "\nby trait method, every impl combined")?;
    groups(writer, &symbols.trait_methods, total, "impls")?;

    writeln!(writer, "\nby module")?;
    groups(writer, &symbols.modules, total, "symbols")?;

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

    writeln!(writer, "\nmonomorphized, left no symbol behind")?;
    writeln!(writer, "  (inlined into callers, or dropped as dead code)")?;
    writeln!(writer, "  (counts, not bytes \u{2014} compiler shims excluded)")?;
    writeln!(writer, "  {:>12}  {:>9}", "generated", "surviving")?;
    for family in &symbols.inlined_away {
        writeln!(writer, "  {:>12}  {:>9}  {}", family.generated, family.surviving, family.name)?;
    }

    writeln!(writer, "\nby crate, which one caused the instantiation")?;
    writeln!(
        writer,
        "  (generic code from the list above, re-attributed \u{2014} not additional)"
    )?;
    groups(writer, &symbols.instantiated_by, total, "symbols")?;

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

    writeln!(writer, "\nsource lines that pulled in the most inlined code")?;
    for site in &inlined.call_sites {
        row(
            writer,
            site.bytes,
            total,
            format_args!("{}:{} ({} inlined)", site.file, site.line, site.instances),
        )?;
    }

    writeln!(writer)
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

    let identical = &assembly.identical;
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

    let panics = &assembly.panics;
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
    callers(writer, &panics.functions, total, &approx)?;

    let formatting = &assembly.formatting;
    writeln!(writer, "\nformatting call sites, into core::fmt and alloc::fmt")?;
    writeln!(writer, "  (the block before each call builds the Arguments)")?;
    approx_row(
        writer,
        approx(formatting.instructions),
        total,
        format_args!("in the blocks of {} sites", formatting.sites),
    )?;
    writeln!(writer, "\nfunctions spending the most on formatting call sites")?;
    callers(writer, &formatting.functions, total, &approx)?;

    let copies = &assembly.copies;
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

    writeln!(writer, "\nsource lines compiled to the most instructions")?;
    writeln!(
        writer,
        "  (the line an instruction came from, after inlining, every instantiation summed)"
    )?;
    lines(writer, &assembly.lines, total, &approx)?;
    writeln!(writer, "\nsource lines in this workspace compiled to the most instructions")?;
    lines(writer, &assembly.workspace_lines, total, &approx)?;

    writeln!(writer)
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

    for duplicate in duplicates {
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

        writeln!(writer)?;
    }

    let count = duplicates.len();
    let noun = if count == 1 { "duplicate dependency" } else { "duplicate dependencies" };
    writeln!(writer, "{count} {noun}")
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
