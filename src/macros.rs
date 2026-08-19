//! Macros by the source they expand to, before any of it compiles.
//!
//! Every symbol view sees code after macro expansion, so a macro-heavy
//! codebase reads as thousands of unrelated functions. `-Zmacro-stats` prints,
//! per crate, each macro with its use count and the bytes of source it
//! expanded to — the code a macro asks the compiler to build. Summed across
//! crates and ranked, it names the derives and declarative macros whose
//! expansions dominate, the ones to slim or to expand into shared functions.
//! Source bytes, not binary bytes, like the IR and mono views.

use std::{fs, path::Path};

use anyhow::{Context, Result};
use rustc_hash::FxHashMap;

#[derive(Debug)]
pub struct MacroReport {
    /// Crates whose statistics were read.
    pub crates: usize,

    /// Distinct macros, their uses, and the source bytes they expanded to.
    pub macros: usize,
    pub uses: u64,
    pub bytes: u64,

    /// Macros by their expanded bytes, largest first.
    pub largest: Vec<MacroStat>,
}

#[derive(Debug, Default)]
pub struct MacroStat {
    /// The macro as the compiler names it: `vec!`, `#[derive(Clone)]`.
    pub name: String,

    pub uses: u64,
    pub bytes: u64,

    /// Crates that used it, and the first few by name.
    pub crates: usize,
    pub crate_names: Vec<String>,
}

/// How many using crates a macro names.
const CRATE_NAMES: usize = 3;

/// Read every crate's statistics from `dir`, keeping the `limit` largest
/// macros.
///
/// # Errors
///
/// Errors when the directory cannot be listed or a file cannot be read.
pub fn analyze(dir: &Path, limit: usize) -> Result<MacroReport> {
    let mut macros: FxHashMap<String, MacroStat> = FxHashMap::default();
    let mut crates = 0;

    let mut entries: Vec<_> = fs::read_dir(dir)
        .with_context(|| format!("failed to read {}", dir.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name().and_then(|name| name.to_str()).is_some_and(|name| {
                name.ends_with(".macro-stats.txt")
                    && !name.starts_with("build_script")
                    && !name.contains("build script")
            })
        })
        .collect();
    entries.sort();

    for path in entries {
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let krate = path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.strip_suffix(".macro-stats.txt"))
            .unwrap_or_default();
        crates += 1;
        for (name, uses, bytes) in text.lines().filter_map(row) {
            let entry = macros
                .entry(name.clone())
                .or_insert_with(|| MacroStat { name, ..MacroStat::default() });
            entry.uses += uses;
            entry.bytes += bytes;
            entry.crates += 1;
            if entry.crate_names.len() < CRATE_NAMES {
                entry.crate_names.push(krate.to_owned());
            }
        }
    }

    let count = macros.len();
    let uses = macros.values().map(|stat| stat.uses).sum();
    let bytes = macros.values().map(|stat| stat.bytes).sum();
    let mut largest: Vec<MacroStat> = macros.into_values().collect();
    largest.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
    largest.truncate(limit);

    Ok(MacroReport { crates, macros: count, uses, bytes, largest })
}

/// One data row: the macro name, then uses, lines, average lines, bytes, and
/// average bytes. Headers and rules fail the number parses and fall away.
fn row(line: &str) -> Option<(String, u64, u64)> {
    // Counts arrive with underscore separators: `5_222_920`.
    let int = |field: &str| field.replace('_', "").parse::<u64>().ok();
    let float = |field: &str| field.replace('_', "").parse::<f64>().ok();

    let fields: Vec<&str> = line.split_whitespace().collect();
    let [name @ .., uses, lines, avg_lines, bytes, avg_bytes] = fields.as_slice() else {
        return None;
    };
    let _ = int(lines)?;
    let _ = float(avg_lines)?;
    let _ = float(avg_bytes)?;
    let uses = int(uses)?;
    let bytes = int(bytes)?;

    (!name.is_empty()).then(|| (name.join(" "), uses, bytes))
}

/// Split a captured cargo stderr into one `<crate>.macro-stats.txt` per crate
/// in `dir`, so cached crates — silent on later runs — keep their statistics.
///
/// # Errors
///
/// Errors when a file cannot be written.
pub fn persist(stderr: &str, dir: &Path) -> Result<()> {
    let mut krate: Option<String> = None;
    let mut block = String::new();

    let flush = |krate: &mut Option<String>, block: &mut String| -> Result<()> {
        if let Some(name) = krate.take() {
            let path = dir.join(format!("{name}.macro-stats.txt"));
            fs::write(&path, &block)
                .with_context(|| format!("failed to write {}", path.display()))?;
        }
        block.clear();
        Ok(())
    };

    for line in stderr.lines() {
        let Some(rest) = line.strip_prefix("macro-stats ") else { continue };
        if let Some(name) = rest.strip_prefix("MACRO EXPANSION STATS: ") {
            flush(&mut krate, &mut block)?;
            krate = Some(name.trim().replace(['/', '\\'], "_"));
        } else if krate.is_some() {
            block.push_str(rest);
            block.push('\n');
        }
    }
    flush(&mut krate, &mut block)
}

#[cfg(test)]
mod tests {
    use super::{analyze, persist};

    #[test]
    fn splits_captured_stats_and_sums_macros_across_crates() {
        let dir = std::env::temp_dir().join(format!("cargo-bsize-macros-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let stderr = "\
warning: unrelated
macro-stats ===========================================================================
macro-stats MACRO EXPANSION STATS: a
macro-stats Macro Name                 Uses      Lines  Avg Lines      Bytes  Avg Bytes
macro-stats ---------------------------------------------------------------------------
macro-stats #[derive(Clone)]              2         11        5.5        288      144.0
macro-stats big!                          2     10_500    5_250.0   22_20_2   11_101.0
macro-stats ===========================================================================
macro-stats ===========================================================================
macro-stats MACRO EXPANSION STATS: b
macro-stats Macro Name                 Uses      Lines  Avg Lines      Bytes  Avg Bytes
macro-stats ---------------------------------------------------------------------------
macro-stats #[derive(Clone)]              1          4        4.0        100      100.0
macro-stats ===========================================================================
";
        persist(stderr, &dir).unwrap();
        // A build script's statistics are not the program's, either way cargo
        // spells the unit.
        std::fs::write(dir.join("build_script_build.macro-stats.txt"), "").unwrap();
        std::fs::write(dir.join("a build script.macro-stats.txt"), "").unwrap();

        let report = analyze(&dir, 20).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();

        assert_eq!((report.crates, report.macros, report.uses, report.bytes), (2, 2, 5, 22590));
        assert_eq!((report.largest[0].name.as_str(), report.largest[0].bytes), ("big!", 22202));
        let derive = &report.largest[1];
        assert_eq!(
            (derive.name.as_str(), derive.uses, derive.bytes, derive.crates),
            ("#[derive(Clone)]", 3, 388, 2)
        );
        assert_eq!(derive.crate_names, ["a", "b"]);
    }
}
