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
    pub code: SymbolSet,
    pub data: SymbolSet,

    /// Rollups cover code only. Inferred data sizes are upper bounds and would
    /// swamp them — `httparse::TOKEN_MAP` is a 256-byte table that absorbs
    /// 149 KiB of the anonymous constants following it.
    pub crates: Vec<CrateSize>,
    pub generics: Vec<GenericFamily>,

    /// Generic code charged to the crate that caused the instantiation rather
    /// than the one that defined it.
    pub instantiated_by: Vec<CrateSize>,
}

#[derive(Debug, Serialize)]
pub struct SymbolSet {
    /// Bytes attributed to a named symbol in these sections.
    pub bytes: u64,
    pub count: usize,
    pub largest: Vec<Symbol>,
}

#[derive(Debug, Serialize)]
pub struct Symbol {
    pub name: String,
    pub size: u64,

    /// `false` when the size came from the distance to the next symbol, making
    /// it an upper bound that includes any anonymous bytes in between.
    pub exact: bool,

    /// How many symbols share this name. More than one means the same item was
    /// emitted repeatedly, and `size` is their total.
    pub copies: usize,

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

/// Rank the symbols in `file`, keeping the `limit` largest of each list.
pub fn analyze(file: &object::File<'_>, limit: usize) -> SymbolReport {
    let mut code = Vec::new();
    let mut data = Vec::new();

    for (mangled, size, category, exact) in sized_symbols(file) {
        let name = demangle(&mangled);
        let symbol = Symbol {
            krate: defining_crate(&mangled, &name),
            instantiated_by: instantiating_crate(&mangled),
            name,
            size,
            exact,
            copies: 1,
        };

        if category == Category::Code { code.push(symbol) } else { data.push(symbol) }
    }

    let crates = rollup(code.iter().filter_map(|s| s.krate.as_deref().zip(Some(s.size))), limit);
    let instantiated_by =
        rollup(code.iter().filter_map(|s| s.instantiated_by.as_deref().zip(Some(s.size))), limit);
    let generics = generic_families(&code, limit);

    SymbolReport {
        code: rank(code, limit),
        data: rank(data, limit),
        crates,
        generics,
        instantiated_by,
    }
}

fn rank(symbols: Vec<Symbol>, limit: usize) -> SymbolSet {
    let bytes = symbols.iter().map(|symbol| symbol.size).sum();
    let count = symbols.len();

    // Two symbols demangling to the same name are one item emitted twice —
    // oxlint carries `register_lsp_methods::<Backend>` once for its lib crate
    // and once for its bin. Merge them into a row with a copy count; left apart
    // they render as identical adjacent rows and read as a display bug.
    let mut merged: Vec<Symbol> = Vec::with_capacity(symbols.len());
    let mut seen: HashMap<String, usize> = HashMap::new();
    for symbol in symbols {
        if let Some(&index) = seen.get(&symbol.name) {
            merged[index].size += symbol.size;
            merged[index].copies += 1;
        } else {
            seen.insert(symbol.name.clone(), merged.len());
            merged.push(symbol);
        }
    }

    merged.sort_by_key(|symbol| Reverse(symbol.size));
    merged.truncate(limit);

    SymbolSet { bytes, count, largest: merged }
}

/// Every symbol in a code or read-only data section, with a size and whether
/// that size is exact.
///
/// Sizes inferred from the distance to the next symbol are only trustworthy
/// where symbols are dense. They are in code — oxlint names 14,668 symbols
/// across 11.3 MiB of `__text` — but not in the constant sections, where a
/// hundred-odd names cover a megabyte and each one absorbs the anonymous data
/// that follows it.
fn sized_symbols(file: &object::File<'_>) -> Vec<(String, u64, Category, bool)> {
    let wanted: HashMap<SectionIndex, (u64, Category)> = file
        .sections()
        .filter_map(|section| {
            let category = Category::of(section.name().ok()?);
            matches!(category, Category::Code | Category::ReadOnlyData)
                .then(|| (section.index(), (section.address() + section.size(), category)))
        })
        .collect();

    let mut by_section: HashMap<SectionIndex, Vec<(u64, u64, String)>> = HashMap::new();
    for symbol in file.symbols() {
        let SymbolSection::Section(index) = symbol.section() else { continue };
        let (Some(_), Ok(name)) = (wanted.get(&index), symbol.name()) else { continue };
        by_section.entry(index).or_default().push((
            symbol.address(),
            symbol.size(),
            name.to_owned(),
        ));
    }

    let mut sized = Vec::new();
    for (index, mut symbols) in by_section {
        let (end, category) = wanted[&index];
        symbols.sort_by_key(|&(address, ..)| address);
        symbols.dedup_by_key(|&mut (address, ..)| address);

        for position in 0..symbols.len() {
            let (address, declared, name) = &symbols[position];
            let next = symbols.get(position + 1).map_or(end, |&(address, ..)| address);
            let exact = *declared > 0;
            let size = if exact { *declared } else { next.saturating_sub(*address) };
            sized.push((name.clone(), size, category, exact));
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
