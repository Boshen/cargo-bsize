//! Attribute pre-optimization LLVM IR to the generics that generated it — the
//! `cargo-llvm-lines` technique.
//!
//! Every other view reads the linked binary, which shows only the code that
//! survived optimization. This reads the IR rustc handed to LLVM, before it
//! inlined or deleted anything, and rolls it up by generic family. That names
//! the SOURCE of monomorphization bloat: a generic instantiated a thousand ways
//! shows here as one family with a thousand instantiations and the IR they cost,
//! which the binary — where most of it was inlined away — cannot.
//!
//! IR lines are not binary bytes; the optimizer removes much of this. It
//! predicts where code comes from and what the compiler had to chew through, and
//! is read alongside the binary views, not instead of them. The IR is emitted
//! for every crate, so it is the whole program, not the final crate alone.

use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
};

use anyhow::{Context, Result, bail};
use rustc_hash::FxHashMap;
use serde::Serialize;

use crate::{
    name::{demangle, generic_family},
    symbols::Total,
};

#[derive(Debug, Serialize)]
pub struct IrReport {
    pub files: usize,
    pub functions: usize,
    pub lines: u64,

    /// The generic families that generated the most IR, largest first.
    pub families: Vec<IrFamily>,
}

#[derive(Debug, Serialize)]
pub struct IrFamily {
    pub name: String,
    pub lines: u64,
    pub instantiations: usize,
}

/// Rank the generic families in the IR files `paths` by the instruction lines
/// they generated, keeping the `limit` largest.
///
/// # Errors
///
/// Errors when there is no IR or a file cannot be read.
pub fn analyze(paths: &[PathBuf], limit: usize) -> Result<IrReport> {
    if paths.is_empty() {
        bail!("the build produced no LLVM IR");
    }

    let mut families: FxHashMap<String, Total> = FxHashMap::default();
    let mut functions = 0;
    let mut lines = 0;
    for path in paths {
        let file =
            File::open(path).with_context(|| format!("failed to read {}", path.display()))?;
        let mut current: Option<(String, u64)> = None;

        for line in BufReader::new(file).lines() {
            let line = line.with_context(|| format!("failed to read {}", path.display()))?;

            if let Some((_, count)) = &mut current {
                if line == "}" {
                    let (family, count) = current.take().expect("current is Some");
                    families.entry(family).or_default().add(count);
                    functions += 1;
                    lines += count;
                } else if is_instruction(&line) {
                    *count += 1;
                }
            } else if let Some(name) = line.strip_prefix("define").and_then(ir_name) {
                current = Some((generic_family(&demangle(name)), 0));
            }
        }
    }

    let mut families: Vec<IrFamily> = families
        .into_iter()
        .map(|(name, total)| IrFamily { name, lines: total.bytes, instantiations: total.count })
        .collect();
    families.sort_by(|a, b| b.lines.cmp(&a.lines).then_with(|| a.name.cmp(&b.name)));
    families.truncate(limit);

    Ok(IrReport { files: paths.len(), functions, lines, families })
}

/// The mangled name a `define` line declares, from its `@name`.
fn ir_name(define: &str) -> Option<&str> {
    let rest = &define[define.find('@')? + 1..];
    rest.strip_prefix('"').map_or_else(
        // `@name(...)` — bare, up to the parameter list.
        || rest.split('(').next().map(str::trim_end),
        // `@"..."` — a quoted name, closed by the next quote.
        |quoted| quoted.split('"').next(),
    )
}

/// Whether a function-body line is an instruction, not a label, comment, debug
/// record, or metadata — so the count tracks generated code, not debug info.
fn is_instruction(line: &str) -> bool {
    let line = line.trim();
    if line.is_empty() || line.starts_with([';', '#', '!']) {
        return false;
    }

    // A basic-block label is the first token ending in `:` (`bb1:`, `start:`).
    !line.split_whitespace().next().unwrap_or_default().ends_with(':')
}

#[cfg(test)]
mod tests {
    use super::{ir_name, is_instruction};

    #[test]
    fn reads_the_define_name() {
        assert_eq!(ir_name(" internal i1 @_RNvC4core3fmt(ptr %x) {"), Some("_RNvC4core3fmt"));
        assert_eq!(ir_name(r#" void @"weird name"(i8) {"#), Some("weird name"));
    }

    #[test]
    fn counts_instructions_not_noise() {
        assert!(is_instruction("  %0 = load i8, ptr %p, align 1"));
        assert!(is_instruction("  ret void"));
        assert!(!is_instruction("start:"));
        assert!(!is_instruction("bb1:                    ; preds = %start"));
        assert!(!is_instruction("    #dbg_value(ptr %x, !1, !DIExpression(), !2)"));
        assert!(!is_instruction("; call core::fmt::write"));
        assert!(!is_instruction("  "));
    }
}
