//! Duplicate read-only data — the data analog of the assembly view's identical
//! function bodies.
//!
//! Two constants with the same bytes under different names cost twice. The
//! linker's identical-code-folding collapses them when enabled; so does sharing
//! one `const`. This reads the actual bytes of the constant sections — the one
//! place the tool looks past sizes and symbol names — hashes each named region,
//! and reports what collapsing the duplicates would return. The usual sources
//! are a crate linked at several versions, each embedding the same table, and a
//! package's lib and bin crates shipping the same static twice.

use std::hash::Hasher;

use object::{Object, ObjectSection, ObjectSymbol, SectionIndex, SymbolSection};
use rustc_hash::{FxHashMap, FxHasher};
use serde::Serialize;

use crate::{name::demangle, sections::Category};

#[derive(Debug, Serialize)]
pub struct DupDataReport {
    pub groups: usize,
    pub symbols: usize,

    /// Bytes deduplication (or the linker's `--icf`) would return: every copy in
    /// a group but one.
    pub recoverable: u64,

    pub largest: Vec<DupGroup>,
}

#[derive(Debug, Serialize)]
pub struct DupGroup {
    pub names: Vec<String>,

    /// Bytes of one copy.
    pub size: u64,

    pub recoverable: u64,
}

/// Find byte-identical read-only-data symbols in `file`, keeping the `limit`
/// groups that would return the most.
///
/// `static_sizes` supplies exact sizes from DWARF: without them two identical
/// tables can be compared over different gap-inferred extents and never match,
/// so exact sizing is what makes the duplicates line up.
pub fn analyze(
    file: &object::File<'_>,
    static_sizes: &FxHashMap<String, u64>,
    limit: usize,
) -> DupDataReport {
    // The read-only-data sections we dedup within, each with its base address,
    // its end, and its bytes.
    let mut sections: FxHashMap<SectionIndex, (u64, u64, Vec<u8>)> = FxHashMap::default();
    for section in file.sections() {
        let Ok(name) = section.name() else { continue };
        if Category::of(name) != Category::ReadOnlyData {
            continue;
        }
        let Ok(data) = section.uncompressed_data() else { continue };
        let base = section.address();
        sections.insert(section.index(), (base, base + section.size(), data.into_owned()));
    }

    let mut by_section: FxHashMap<SectionIndex, Vec<(u64, String)>> = FxHashMap::default();
    for symbol in file.symbols() {
        let SymbolSection::Section(index) = symbol.section() else { continue };
        let (true, Ok(name)) = (sections.contains_key(&index), symbol.name()) else { continue };
        by_section.entry(index).or_default().push((symbol.address(), name.to_owned()));
    }

    // Hash each symbol's byte range. The size is exact where DWARF names it,
    // else the gap to the next symbol, so a gap match means the whole range —
    // trailing anonymous bytes included — is identical, which is conservative.
    let mut entries: Vec<(u64, u64, String)> = Vec::new();
    for (index, mut symbols) in by_section {
        let (base, end, data) = &sections[&index];
        symbols.sort_by_key(|&(address, _)| address);
        symbols.dedup_by_key(|&mut (address, _)| address);

        for position in 0..symbols.len() {
            let (address, name) = &symbols[position];
            let demangled = demangle(name);

            // Exact where DWARF names the type, else the gap to the next symbol.
            // Two identical tables only match when compared over the same extent.
            let next = symbols.get(position + 1).map_or(*end, |&(address, _)| address);
            let size = static_sizes
                .get(&demangled)
                .copied()
                .unwrap_or_else(|| next.saturating_sub(*address));

            let (Ok(start), Ok(len)) = (usize::try_from(address - base), usize::try_from(size))
            else {
                continue;
            };
            let Some(slice) = data.get(start..start.saturating_add(len)).filter(|s| !s.is_empty())
            else {
                continue;
            };

            let mut hasher = FxHasher::default();
            hasher.write(slice);
            entries.push((hasher.finish(), size, demangled));
        }
    }

    group(entries, limit)
}

/// Group `(content hash, size, name)` entries that share a hash and size, and
/// rank the groups by what folding them would return.
fn group(entries: Vec<(u64, u64, String)>, limit: usize) -> DupDataReport {
    let mut groups: FxHashMap<(u64, u64), Vec<String>> = FxHashMap::default();
    for (hash, size, name) in entries {
        groups.entry((hash, size)).or_default().push(name);
    }

    let mut largest: Vec<DupGroup> = groups
        .into_iter()
        .filter(|(_, names)| names.len() > 1)
        .map(|((_, size), mut names)| {
            names.sort();
            let recoverable = size * (names.len() as u64 - 1);
            DupGroup { names, size, recoverable }
        })
        .collect();
    largest.sort_by(|a, b| b.recoverable.cmp(&a.recoverable).then_with(|| a.names.cmp(&b.names)));

    let groups = largest.len();
    let symbols = largest.iter().map(|group| group.names.len()).sum();
    let recoverable = largest.iter().map(|group| group.recoverable).sum();
    largest.truncate(limit);

    DupDataReport { groups, symbols, recoverable, largest }
}

#[cfg(test)]
mod tests {
    use super::group;

    #[test]
    fn groups_identical_content_and_ignores_singletons_and_near_misses() {
        let entries = vec![
            (0xAB, 100, "a::T".to_owned()),
            (0xAB, 100, "b::T".to_owned()), // same hash + size as a::T
            (0xAB, 200, "c::T".to_owned()), // same hash, different size — not a dup
            (0xCD, 100, "d::T".to_owned()), // unique
        ];

        let report = group(entries, 20);

        assert_eq!(report.groups, 1);
        assert_eq!(report.symbols, 2);
        assert_eq!(report.recoverable, 100); // one 100-byte copy freed
        assert_eq!(report.largest[0].names, ["a::T", "b::T"]);
    }
}
