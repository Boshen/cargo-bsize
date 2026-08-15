//! Rendering of analysis results.

use std::{fmt, io, str::FromStr};

use serde::Serialize;

use crate::duplicates::Duplicate;

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

/// An object rather than a bare array, so later analyses can be added without
/// breaking the schema.
#[derive(Serialize)]
struct JsonReport<'a> {
    duplicates: &'a [Duplicate],
}

/// # Errors
///
/// Errors when writing to `writer` fails.
pub fn render<W: io::Write>(
    writer: &mut W,
    duplicates: &[Duplicate],
    format: OutputFormat,
) -> io::Result<()> {
    match format {
        OutputFormat::Text => render_text(writer, duplicates),
        OutputFormat::Json => render_json(writer, duplicates),
    }
}

fn render_json<W: io::Write>(writer: &mut W, duplicates: &[Duplicate]) -> io::Result<()> {
    serde_json::to_writer_pretty(&mut *writer, &JsonReport { duplicates })
        .map_err(io::Error::other)?;
    writeln!(writer)
}

fn render_text<W: io::Write>(writer: &mut W, duplicates: &[Duplicate]) -> io::Result<()> {
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
