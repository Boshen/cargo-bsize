//! Rendering of analysis results.

use std::{fmt, io, str::FromStr};

use serde::Serialize;

use crate::{duplicates::Duplicate, sections::BinaryReport};

/// An object rather than a bare array, so later analyses can be added without
/// breaking the schema.
#[derive(Debug, Serialize)]
pub struct Report {
    pub duplicates: Vec<Duplicate>,
    pub binaries: Vec<BinaryReport>,
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
) -> io::Result<()> {
    match format {
        OutputFormat::Text => render_text(writer, report),
        OutputFormat::Json => {
            serde_json::to_writer_pretty(&mut *writer, report).map_err(io::Error::other)?;
            writeln!(writer)
        }
    }
}

fn render_text<W: io::Write>(writer: &mut W, report: &Report) -> io::Result<()> {
    for binary in &report.binaries {
        render_binary(writer, binary)?;
    }

    render_duplicates(writer, &report.duplicates)
}

fn render_binary<W: io::Write>(writer: &mut W, binary: &BinaryReport) -> io::Result<()> {
    writeln!(writer, "{} ({})", binary.path, binary.format)?;
    writeln!(writer, "  {:>12}  total", bytes(binary.total))?;
    writeln!(writer, "  {:>12}  shipped, excluding symbols and debug info", bytes(binary.shipped))?;
    writeln!(writer)?;

    for category in &binary.categories {
        row(writer, category.size, binary.total, category.category)?;
    }
    row(writer, binary.other, binary.total, "other (headers, padding, code signature)")?;
    writeln!(writer)?;

    for section in &binary.sections {
        row(writer, section.size, binary.total, &section.name)?;
    }

    writeln!(writer)
}

fn row<W: io::Write, L: fmt::Display>(
    writer: &mut W,
    size: u64,
    total: u64,
    label: L,
) -> io::Result<()> {
    #[expect(clippy::cast_precision_loss, reason = "display only")]
    let percent = if total == 0 { 0.0 } else { size as f64 / total as f64 * 100.0 };

    writeln!(writer, "  {:>12}  {percent:>4.1}%  {label}", bytes(size))
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
