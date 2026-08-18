//! Generic definitions by what they cost to monomorphize, before inlining.
//!
//! The symbol table counts the instantiations that survived; the inlined view
//! charges the ones that did not. What neither says is how much MIR each generic
//! definition fed the backend across every crate that instantiated it — the
//! `cargo llvm-lines` question, answered a stage earlier and without gigabytes
//! of IR. `-Zdump-mono-stats` writes it per crate: each definition with its
//! instantiation count and a size estimate (MIR statements) per instantiation.
//! Summed across crates and ranked, it names the generics whose bodies are
//! large *and* instantiated often — the ones to split into a small generic
//! shell over a non-generic body. Estimates, not bytes, like the IR view.

use std::{fs, path::Path};

use anyhow::{Context, Result};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct MonoReport {
    /// Crates whose statistics were read.
    pub crates: usize,

    /// Distinct definitions, and the instantiations of them across every crate.
    pub definitions: usize,
    pub instantiations: u64,

    /// The estimated MIR statements of every instantiation, summed.
    pub estimate: u64,

    /// Definitions by their total estimate, largest first.
    pub largest: Vec<Definition>,
}

#[derive(Debug, Serialize)]
pub struct Definition {
    /// The definition, with its generic parameters as written.
    pub name: String,

    /// Instantiations across every crate.
    pub instantiations: u64,

    /// Estimated MIR statements per instantiation, and in total.
    pub each: u64,
    pub estimate: u64,

    /// Crates that instantiated it, and the first few by name — the
    /// statistics spell a crate's own items without their crate prefix.
    pub crates: usize,
    pub crate_names: Vec<String>,
}

/// How many instantiating crates a definition names.
const CRATE_NAMES: usize = 3;

/// One row of a crate's `<crate>.mono_items.json`.
#[derive(Debug, Deserialize)]
struct Item {
    name: String,
    instantiation_count: u64,
    size_estimate: u64,
    total_estimate: u64,
}

/// Read every crate's statistics from `dir`, keeping the `limit` largest
/// definitions.
///
/// # Errors
///
/// Errors when the directory cannot be listed or a file cannot be parsed.
pub fn analyze(dir: &Path, limit: usize) -> Result<MonoReport> {
    let mut definitions: FxHashMap<String, Definition> = FxHashMap::default();
    let mut crates = 0;

    let mut entries: Vec<_> = fs::read_dir(dir)
        .with_context(|| format!("failed to read {}", dir.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name().and_then(|name| name.to_str()).is_some_and(|name| {
                name.ends_with(".mono_items.json") && !name.starts_with("build_script")
            })
        })
        .collect();
    entries.sort();

    for path in entries {
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let items: Vec<Item> = serde_json::from_str(&text)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        let krate = path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.strip_suffix(".mono_items.json"))
            .unwrap_or_default()
            .to_owned();
        crates += 1;
        for item in items {
            let entry = definitions.entry(item.name.clone()).or_insert_with(|| Definition {
                name: item.name,
                instantiations: 0,
                each: item.size_estimate,
                estimate: 0,
                crates: 0,
                crate_names: Vec::new(),
            });
            entry.instantiations += item.instantiation_count;
            entry.estimate += item.total_estimate;
            entry.each = entry.each.max(item.size_estimate);
            entry.crates += 1;
            if entry.crate_names.len() < CRATE_NAMES {
                entry.crate_names.push(krate.clone());
            }
        }
    }

    let count = definitions.len();
    let instantiations = definitions.values().map(|d| d.instantiations).sum();
    let estimate = definitions.values().map(|d| d.estimate).sum();
    let mut largest: Vec<Definition> = definitions.into_values().collect();
    largest.sort_by(|a, b| b.estimate.cmp(&a.estimate).then_with(|| a.name.cmp(&b.name)));
    largest.truncate(limit);

    Ok(MonoReport { crates, definitions: count, instantiations, estimate, largest })
}

#[cfg(test)]
mod tests {
    use super::analyze;

    #[test]
    fn sums_a_definition_across_crates() {
        let dir = std::env::temp_dir().join(format!("cargo-bsize-mono-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("a.mono_items.json"),
            r#"[{"name":"core::ptr::drop_in_place::<T>","instantiation_count":3,"size_estimate":10,"total_estimate":30},
                {"name":"a::only","instantiation_count":1,"size_estimate":5,"total_estimate":5}]"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("b.mono_items.json"),
            r#"[{"name":"core::ptr::drop_in_place::<T>","instantiation_count":2,"size_estimate":12,"total_estimate":24}]"#,
        )
        .unwrap();
        // A build script's statistics are not the program's.
        std::fs::write(dir.join("build_script_build.mono_items.json"), "[]").unwrap();

        let report = analyze(&dir, 20).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();

        assert_eq!(
            (report.crates, report.definitions, report.instantiations, report.estimate),
            (2, 2, 6, 59)
        );
        let top = &report.largest[0];
        assert_eq!(
            (top.name.as_str(), top.instantiations, top.each, top.estimate, top.crates),
            ("core::ptr::drop_in_place::<T>", 5, 12, 54, 2)
        );
        assert_eq!(top.crate_names, ["a", "b"]);
    }
}
