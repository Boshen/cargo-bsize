//! Rendering of analysis results.

use std::{fmt, io, str::FromStr};

use serde::Serialize;

use crate::{
    duplicates::Duplicate,
    sections::BinaryReport,
    symbols::{Symbol, SymbolReport},
};

/// An object rather than a bare array, so later analyses can be added without
/// breaking the schema.
#[derive(Debug, Serialize)]
pub struct Report {
    pub duplicates: Vec<Duplicate>,
    pub binary: Option<BinaryReport>,
    pub symbols: Option<SymbolReport>,
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
    for symbol in &symbols.data.largest {
        bounded_row(writer, symbol, total)?;
    }

    writeln!(writer, "\nby crate, where the code is defined")?;
    for entry in &symbols.crates {
        row(writer, entry.size, total, format_args!("{} ({} symbols)", entry.name, entry.symbols))?;
    }

    writeln!(writer, "\ngeneric families")?;
    for family in &symbols.generics {
        row(
            writer,
            family.size,
            total,
            format_args!("{} ({}\u{d7})", family.name, family.instantiations),
        )?;
    }

    writeln!(writer, "\nby crate, which one caused the instantiation")?;
    writeln!(
        writer,
        "  (generic code from the list above, re-attributed \u{2014} not additional)"
    )?;
    for entry in &symbols.instantiated_by {
        row(writer, entry.size, total, format_args!("{} ({} symbols)", entry.name, entry.symbols))?;
    }

    writeln!(writer)
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

/// A size inferred from the gap to the next symbol is an upper bound: it also
/// covers whatever anonymous data sits in between.
fn bounded_row<W: io::Write>(writer: &mut W, symbol: &Symbol, total: u64) -> io::Result<()> {
    if symbol.exact {
        return row(writer, symbol.size, total, label(symbol));
    }

    let size = format!("\u{2264} {}", bytes(symbol.size));
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
