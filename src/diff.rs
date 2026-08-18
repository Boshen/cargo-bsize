//! Compare the build against a baseline binary — what grew, and where.
//!
//! Every other view is a snapshot. This one answers the question a change
//! actually raises: did the binary grow, and which crate or function did it.
//! Code symbols are sized the same way in both (the gap inference, no DWARF), so
//! the deltas are method-consistent; the read-only data, whose sizes are only
//! bounds, is left out.

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
};

use anyhow::{Context, Result};
use serde::Serialize;

use crate::symbols::sized_symbols;

#[derive(Debug, Serialize)]
pub struct DiffReport {
    pub baseline: String,

    /// Total code bytes in the baseline and in this build.
    pub before: u64,
    pub after: u64,

    /// The crates whose code changed the most, by absolute delta.
    pub crates: Vec<NamedDelta>,

    /// The functions whose code changed the most, by absolute delta.
    pub symbols: Vec<NamedDelta>,
}

#[derive(Debug, Serialize)]
pub struct NamedDelta {
    pub name: String,
    pub before: u64,
    pub after: u64,
}

impl NamedDelta {
    /// How far the size moved, either direction, for ranking.
    const fn magnitude(&self) -> u64 {
        self.after.abs_diff(self.before)
    }
}

/// Diff `primary` against the binary at `baseline`, keeping the `limit` biggest
/// movers of each list.
///
/// # Errors
///
/// Errors when the baseline binary cannot be read or parsed.
pub fn analyze(primary: &object::File<'_>, baseline: &Path, limit: usize) -> Result<DiffReport> {
    let data =
        fs::read(baseline).with_context(|| format!("failed to read {}", baseline.display()))?;
    let file = object::File::parse(&*data)
        .with_context(|| format!("failed to parse {}", baseline.display()))?;

    let (after_names, after_crates, after) = code_sizes(primary);
    let (before_names, before_crates, before) = code_sizes(&file);

    Ok(DiffReport {
        baseline: baseline.display().to_string(),
        before,
        after,
        crates: deltas(&before_crates, &after_crates, limit),
        symbols: deltas(&before_names, &after_names, limit),
    })
}

/// Code bytes by function name, by defining crate, and in total. Data is left
/// out — its sizes are only upper bounds without DWARF, which the baseline lacks.
fn code_sizes(file: &object::File<'_>) -> (HashMap<String, u64>, HashMap<String, u64>, u64) {
    let (code, _) = sized_symbols(file, &HashMap::new());

    let mut by_name: HashMap<String, u64> = HashMap::new();
    let mut by_crate: HashMap<String, u64> = HashMap::new();
    let mut total = 0;
    for symbol in code {
        total += symbol.size;
        if let Some(krate) = &symbol.krate {
            *by_crate.entry(krate.clone()).or_default() += symbol.size;
        }
        *by_name.entry(symbol.name).or_default() += symbol.size;
    }

    (by_name, by_crate, total)
}

/// The entries that changed between two size maps, biggest mover first.
fn deltas(
    before: &HashMap<String, u64>,
    after: &HashMap<String, u64>,
    limit: usize,
) -> Vec<NamedDelta> {
    let names: HashSet<&str> = before.keys().chain(after.keys()).map(String::as_str).collect();

    let mut deltas: Vec<NamedDelta> = names
        .into_iter()
        .map(|name| NamedDelta {
            name: name.to_owned(),
            before: before.get(name).copied().unwrap_or_default(),
            after: after.get(name).copied().unwrap_or_default(),
        })
        .filter(|delta| delta.before != delta.after)
        .collect();

    deltas.sort_by(|a, b| b.magnitude().cmp(&a.magnitude()).then_with(|| a.name.cmp(&b.name)));
    deltas.truncate(limit);
    deltas
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::deltas;

    #[test]
    fn ranks_the_biggest_movers_and_ignores_the_unchanged() {
        let before =
            HashMap::from([("grew".into(), 100), ("same".into(), 50), ("gone".into(), 30)]);
        let after = HashMap::from([("grew".into(), 400), ("same".into(), 50), ("new".into(), 20)]);

        let deltas = deltas(&before, &after, 20);
        let names: Vec<&str> = deltas.iter().map(|delta| delta.name.as_str()).collect();

        // `same` dropped (unchanged); ordered by absolute delta: grew 300, gone 30, new 20.
        assert_eq!(names, ["grew", "gone", "new"]);
        assert_eq!(deltas[0].before, 100);
        assert_eq!(deltas[0].after, 400);
        assert_eq!(deltas[1].after, 0); // removed
        assert_eq!(deltas[2].before, 0); // added
    }
}
