//! Break the code and read-only data sections apart into individual symbols.
//!
//! Mach-O symbols carry no size, so sizes come from sorting by address within a
//! section and taking the delta to the next symbol. ELF records a real size, so
//! that is preferred when present.
//!
//! Under `lto = "fat"` an inlined function has no symbol at all and its bytes
//! land on whatever inlined it, so this shows where code ended up rather than
//! where it was written.

use std::{cmp::Reverse, collections::HashMap};

use object::{Object, ObjectSection, ObjectSymbol, SectionIndex, SymbolSection};
use serde::Serialize;

use crate::sections::Category;

#[derive(Debug, Serialize)]
pub struct SymbolReport {
    /// Bytes landing on a named symbol. The rest of the section is padding and
    /// anonymous data.
    pub attributed: u64,

    pub symbols: Vec<Symbol>,
    pub crates: Vec<CrateSize>,
    pub generics: Vec<GenericFamily>,

    /// Generic code charged to the crate that caused the instantiation rather
    /// than the one that defined it.
    pub instantiated_by: Vec<CrateSize>,
}

#[derive(Debug, Serialize)]
pub struct Symbol {
    pub name: String,
    pub size: u64,
    pub krate: Option<String>,

    /// Set only for a generic instantiated outside its defining crate; v0
    /// mangling omits it when a crate instantiates its own generic.
    pub instantiated_by: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CrateSize {
    pub name: String,
    pub size: u64,
    pub symbols: usize,
}

#[derive(Debug, Serialize)]
pub struct GenericFamily {
    pub name: String,
    pub size: u64,
    pub instantiations: usize,
}

/// Rank the symbols in `file`, keeping the `limit` largest of each rollup.
pub fn analyze(file: &object::File<'_>, limit: usize) -> SymbolReport {
    let mut symbols: Vec<Symbol> = sized_symbols(file)
        .into_iter()
        .map(|(mangled, size)| {
            let name = demangle(&mangled);
            Symbol {
                krate: defining_crate(&mangled, &name),
                instantiated_by: instantiating_crate(&mangled),
                name,
                size,
            }
        })
        .collect();
    symbols.sort_by_key(|symbol| Reverse(symbol.size));

    let attributed = symbols.iter().map(|symbol| symbol.size).sum();
    let crates = rollup(symbols.iter().filter_map(|s| s.krate.as_deref().zip(Some(s.size))), limit);
    let instantiated_by = rollup(
        symbols.iter().filter_map(|s| s.instantiated_by.as_deref().zip(Some(s.size))),
        limit,
    );
    let generics = generic_families(&symbols, limit);

    symbols.truncate(limit);

    SymbolReport { attributed, symbols, crates, generics, instantiated_by }
}

/// Every symbol in a code or read-only data section, with a size.
fn sized_symbols(file: &object::File<'_>) -> Vec<(String, u64)> {
    let ends: HashMap<SectionIndex, u64> = file
        .sections()
        .filter(|section| {
            section.name().is_ok_and(|name| {
                matches!(Category::of(name), Category::Code | Category::ReadOnlyData)
            })
        })
        .map(|section| (section.index(), section.address() + section.size()))
        .collect();

    let mut by_section: HashMap<SectionIndex, Vec<(u64, u64, String)>> = HashMap::new();
    for symbol in file.symbols() {
        let SymbolSection::Section(index) = symbol.section() else { continue };
        let (Some(_), Ok(name)) = (ends.get(&index), symbol.name()) else { continue };
        by_section.entry(index).or_default().push((
            symbol.address(),
            symbol.size(),
            name.to_owned(),
        ));
    }

    let mut sized = Vec::new();
    for (index, mut symbols) in by_section {
        let end = ends[&index];
        symbols.sort_by_key(|&(address, ..)| address);
        symbols.dedup_by_key(|&mut (address, ..)| address);

        for position in 0..symbols.len() {
            let (address, declared, name) = &symbols[position];
            let next = symbols.get(position + 1).map_or(end, |&(address, ..)| address);
            let size = if *declared > 0 { *declared } else { next.saturating_sub(*address) };
            sized.push((name.clone(), size));
        }
    }

    sized
}

fn rollup<'a>(sizes: impl Iterator<Item = (&'a str, u64)>, limit: usize) -> Vec<CrateSize> {
    let mut totals: HashMap<&str, (u64, usize)> = HashMap::new();
    for (name, size) in sizes {
        let entry = totals.entry(name).or_default();
        entry.0 += size;
        entry.1 += 1;
    }

    let mut rollup: Vec<CrateSize> = totals
        .into_iter()
        .map(|(name, (size, symbols))| CrateSize { name: name.to_owned(), size, symbols })
        .collect();
    rollup.sort_by_key(|entry| Reverse(entry.size));
    rollup.truncate(limit);
    rollup
}

fn generic_families(symbols: &[Symbol], limit: usize) -> Vec<GenericFamily> {
    let mut totals: HashMap<String, (u64, usize)> = HashMap::new();
    for symbol in symbols {
        let entry = totals.entry(generic_family(&symbol.name)).or_default();
        entry.0 += symbol.size;
        entry.1 += 1;
    }

    let mut families: Vec<GenericFamily> = totals
        .into_iter()
        .filter(|(_, (_, instantiations))| *instantiations > 1)
        .map(|(name, (size, instantiations))| GenericFamily { name, size, instantiations })
        .collect();
    families.sort_by_key(|family| Reverse(family.size));
    families.truncate(limit);
    families
}

/// Drop turbofish arguments so every instantiation of one generic shares a name.
fn generic_family(name: &str) -> String {
    let mut family = String::with_capacity(name.len());
    let mut rest = name;

    while let Some(start) = rest.find("::<") {
        family.push_str(&rest[..start]);

        let mut depth = 0usize;
        let mut close = None;
        for (offset, character) in rest[start + 2..].char_indices() {
            match character {
                '<' => depth += 1,
                '>' => {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(start + 2 + offset + 1);
                        break;
                    }
                }
                _ => {}
            }
        }

        match close {
            Some(close) => rest = &rest[close..],
            None => return family,
        }
    }

    family.push_str(rest);
    family
}

fn demangle(mangled: &str) -> String {
    // Mach-O prefixes every symbol with an underscore.
    let trimmed = mangled
        .strip_prefix('_')
        .filter(|rest| rest.starts_with("_R") || rest.starts_with("_Z"))
        .unwrap_or(mangled);

    // `{:#}` drops the crate disambiguator hashes.
    format!("{:#}", rustc_demangle::demangle(trimmed))
}

/// Parse `Cs<hash>_<len><name>` or `C<len><name>` at `index`, returning the
/// crate name and the offset just past it.
fn crate_at(symbol: &str, index: usize) -> Option<(&str, usize)> {
    let bytes = symbol.as_bytes();
    if bytes.get(index) != Some(&b'C') {
        return None;
    }

    let mut cursor = index + 1;
    if bytes.get(cursor) == Some(&b's') {
        cursor += 1;
        while bytes.get(cursor).is_some_and(u8::is_ascii_alphanumeric) {
            cursor += 1;
        }
        if bytes.get(cursor) != Some(&b'_') {
            return None;
        }
        cursor += 1;
    }

    let digits = cursor;
    while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
        cursor += 1;
    }

    let length: usize = symbol.get(digits..cursor)?.parse().ok()?;
    let end = cursor.checked_add(length)?;
    symbol.get(cursor..end).map(|name| (name, end))
}

/// The crate a symbol is defined in — the first crate named in the mangled path.
fn defining_crate(mangled: &str, demangled: &str) -> Option<String> {
    if mangled.trim_start_matches('_').starts_with('R')
        && let Some((name, _)) = (0..mangled.len()).find_map(|index| crate_at(mangled, index))
    {
        return Some(name.to_owned());
    }

    // Legacy `_ZN` symbols and anything unmangled.
    demangled.split("::").next().filter(|name| !name.is_empty()).map(str::to_owned)
}

/// The crate that caused a cross-crate generic instantiation.
///
/// v0 appends the instantiating crate as a trailing path. It has to be found by
/// scanning from the right: the first `C` in a symbol parses greedily and would
/// otherwise swallow the rest of the string.
pub(crate) fn instantiating_crate(mangled: &str) -> Option<String> {
    (0..mangled.len()).rev().find_map(|index| {
        crate_at(mangled, index)
            .filter(|&(_, end)| end == mangled.len())
            .map(|(name, _)| name.to_owned())
    })
}
