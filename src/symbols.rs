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

    /// The patterns the size literature keeps naming as causes of bloat. These
    /// overlap and do not sum to the code total.
    pub patterns: Vec<Group>,

    /// Every impl of one trait method, summed. The axis that concentrates best:
    /// oxlint's code is flat enough that its twenty largest functions are 4.9%
    /// of the binary, while `Rule::run` across 596 impls is 7.4% on its own.
    pub trait_methods: Vec<Group>,

    /// Code grouped by the module that defines it.
    pub modules: Vec<Group>,

    /// Rollups cover code only. Inferred data sizes are upper bounds and would
    /// swamp them — `httparse::TOKEN_MAP` is a 256-byte table that absorbs
    /// 149 KiB of the anonymous constants following it.
    pub crates: Vec<Group>,
    pub generics: Vec<GenericFamily>,

    /// Generic code charged to the crate that caused the instantiation rather
    /// than the one that defined it.
    pub instantiated_by: Vec<Group>,

    /// Generics the compiler stamped out in bulk that left few symbols behind.
    pub inlined_away: Vec<MonoFamily>,
}

/// A generic compared across the two things we can observe: what the compiler
/// monomorphized, and what survived to the linked binary.
#[derive(Debug, Serialize)]
pub struct MonoFamily {
    pub name: String,

    /// Instantiations the compiler generated.
    pub generated: usize,

    /// Instantiations still carrying a symbol in the binary.
    pub surviving: usize,
}

#[derive(Debug, Serialize)]
pub struct SymbolSet {
    /// Bytes attributed to a named symbol in these sections.
    pub bytes: u64,

    /// Total file bytes of the sections these symbols live in. What is left
    /// over after `bytes` is data no symbol names.
    pub section_bytes: u64,

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
pub struct Group {
    pub name: String,
    pub size: u64,
    pub symbols: usize,
}

#[derive(Debug, Serialize)]
pub struct GenericFamily {
    pub name: String,
    pub size: u64,
    pub instantiations: usize,

    /// What collapsing every instantiation onto one would return: the total
    /// minus the largest instance. An upper bound — dynamic dispatch is not
    /// free, and the surviving copy may grow.
    pub recoverable: u64,

    /// Mean bytes per instantiation. Ranking by total favours whatever is
    /// instantiated most; this favours whatever is expensive each time, and the
    /// two orders disagree — `register_lsp_methods` is 28 KiB per instance
    /// against `LintContext::create_fix` at 518 B.
    pub each: u64,
}

/// Rank the symbols in `file`, keeping the `limit` largest of each list.
///
/// `mono_items` are the instantiations the compiler generated, most of which
/// never reach the binary as a symbol of their own.
pub fn analyze(file: &object::File<'_>, mono_items: &[String], limit: usize) -> SymbolReport {
    let mut code = Vec::new();
    let mut data = Vec::new();
    let (code_sections, data_sections) = section_bytes(file);

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
    let trait_methods =
        rollup(code.iter().filter_map(|s| trait_method_of(&s.name).zip(Some(s.size))), limit);
    let modules = rollup(code.iter().filter_map(|s| module_of(&s.name).zip(Some(s.size))), limit);
    let generics = generic_families(&code, limit);

    let patterns = patterns(&code);
    let inlined_away = inlined_away(mono_items, &code, limit);

    SymbolReport {
        code: rank(code, code_sections, limit),
        data: rank(data, data_sections, limit),
        patterns,
        trait_methods,
        modules,
        crates,
        generics,
        instantiated_by,
        inlined_away,
    }
}

/// Generics the compiler generated many copies of that left few symbols behind.
///
/// Everything else in this report reads the linked binary, which sees only the
/// instantiations that survived: on cargo-bsize itself, 1,384 symbols out of
/// 31,635 monomorphized items. The rest were inlined into their callers, where
/// their bytes are counted against whoever inlined them, or dropped as dead
/// code. This is the one view that can see them at all.
fn inlined_away(mono_items: &[String], code: &[Symbol], limit: usize) -> Vec<MonoFamily> {
    let mut generated: HashMap<String, usize> = HashMap::new();
    for item in mono_items {
        *generated.entry(generic_family(item)).or_default() += 1;
    }

    let mut surviving: HashMap<String, usize> = HashMap::new();
    for symbol in code {
        *surviving.entry(generic_family(&symbol.name)).or_default() += 1;
    }

    let mut families: Vec<MonoFamily> = generated
        .into_iter()
        .map(|(name, generated)| {
            let surviving = surviving.get(&name).copied().unwrap_or_default();
            MonoFamily { name, generated, surviving }
        })
        .filter(|family| family.generated > family.surviving)
        .collect();

    families.sort_by_key(|family| Reverse(family.generated - family.surviving));
    families.truncate(limit);
    families
}

/// Shapes the size literature repeatedly blames, matched on the demangled name.
///
/// Closures lead because a method generic over a closure type gets a fresh
/// instantiation per call site — in oxlint they are 16% of the code, and no
/// crate, module, or trait rollup can see them.
type Pattern = (&'static str, fn(&str) -> bool);

const PATTERNS: [Pattern; 6] = [
    ("closures", |name| name.contains("{closure#")),
    ("serde", |name| name.contains("serde") && name.contains("erialize")),
    ("formatting", |name| name.contains("::fmt")),
    ("drop glue", |name| name.contains("drop_glue") || name.contains("drop_in_place")),
    ("iterators", |name| name.contains("::iter::") || name.contains("Iterator>::")),
    ("panic paths", |name| {
        name.contains("panic") || name.contains("unwrap_failed") || name.contains("expect_failed")
    }),
];

/// A symbol can match several patterns — a serde deserializer written as a
/// closure is both — so these are counted independently rather than partitioned.
fn patterns(symbols: &[Symbol]) -> Vec<Group> {
    let mut groups: Vec<Group> = PATTERNS
        .iter()
        .map(|(name, matches)| {
            let matched = symbols.iter().filter(|symbol| matches(&symbol.name));
            let (size, count) =
                matched.fold((0, 0), |(size, count), symbol| (size + symbol.size, count + 1));

            Group { name: (*name).to_owned(), size, symbols: count }
        })
        .filter(|group| group.size > 0)
        .collect();

    groups.sort_by_key(|group| Reverse(group.size));
    groups
}

/// Split a demangled name into `(self type, trait, remaining path)`.
///
/// `<Foo as Bar>::baz` yields `(Foo, Bar, baz)`, `<Foo>::baz` yields
/// `(Foo, None, baz)`, and a plain path yields `(None, None, path)`.
fn split_qualified(name: &str) -> (Option<&str>, Option<&str>, &str) {
    if !name.starts_with('<') {
        return (None, None, name);
    }

    let mut depth = 0usize;
    let mut close = None;
    for (index, character) in name.char_indices() {
        match character {
            '<' => depth += 1,
            '>' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(index);
                    break;
                }
            }
            _ => {}
        }
    }

    let Some(close) = close else { return (None, None, name) };
    let rest = name[close + 1..].trim_start_matches(':');

    match name[1..close].split_once(" as ") {
        Some((self_type, trait_name)) => (Some(self_type), Some(trait_name), rest),
        None => (Some(&name[1..close]), None, rest),
    }
}

/// The module a symbol is defined in — the owning type's path, or the function's
/// own path, minus its last segment.
fn module_of(name: &str) -> Option<String> {
    let (self_type, _, path) = split_qualified(name);
    let owner = strip_generics(self_type.unwrap_or(path));

    owner.rsplit_once("::").map(|(module, _)| module.to_owned())
}

/// One method of one trait, so every impl of it sums into a single row.
fn trait_method_of(name: &str) -> Option<String> {
    let (_, trait_name, path) = split_qualified(name);
    let method = path.split("::").next().filter(|method| !method.is_empty())?;

    Some(format!("<{}>::{method}", strip_generics(trait_name?)))
}

/// Drop every generic argument, so `Router<Backend, Error>` becomes `Router`.
/// Without this the `::` inside a type argument is mistaken for a module split.
fn strip_generics(name: &str) -> String {
    let mut stripped = String::with_capacity(name.len());
    let mut depth = 0usize;

    for character in name.chars() {
        match character {
            '<' => depth += 1,
            '>' => depth = depth.saturating_sub(1),
            _ if depth == 0 => stripped.push(character),
            _ => {}
        }
    }

    stripped
}

/// Total file bytes of the code and read-only data sections.
fn section_bytes(file: &object::File<'_>) -> (u64, u64) {
    let mut code = 0;
    let mut data = 0;

    for section in file.sections() {
        let Ok(name) = section.name() else { continue };
        let Some((_, size)) = section.file_range() else { continue };

        match Category::of(name) {
            Category::Code => code += size,
            Category::ReadOnlyData => data += size,
            _ => {}
        }
    }

    (code, data)
}

fn rank(symbols: Vec<Symbol>, section_bytes: u64, limit: usize) -> SymbolSet {
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

    SymbolSet { bytes, section_bytes, count, largest: merged }
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

fn rollup<K>(sizes: impl Iterator<Item = (K, u64)>, limit: usize) -> Vec<Group>
where
    K: Eq + std::hash::Hash + Into<String>,
{
    let mut totals: HashMap<K, (u64, usize)> = HashMap::new();
    for (name, size) in sizes {
        let entry = totals.entry(name).or_default();
        entry.0 += size;
        entry.1 += 1;
    }

    let mut rollup: Vec<Group> = totals
        .into_iter()
        .map(|(name, (size, symbols))| Group { name: name.into(), size, symbols })
        .collect();
    rollup.sort_by_key(|entry| Reverse(entry.size));
    rollup.truncate(limit);
    rollup
}

fn generic_families(symbols: &[Symbol], limit: usize) -> Vec<GenericFamily> {
    // Track the largest instance alongside the total: collapsing a family onto
    // one copy returns everything except that one.
    let mut totals: HashMap<String, (u64, usize, u64)> = HashMap::new();
    for symbol in symbols {
        let entry = totals.entry(generic_family(&symbol.name)).or_default();
        entry.0 += symbol.size;
        entry.1 += 1;
        entry.2 = entry.2.max(symbol.size);
    }

    let mut families: Vec<GenericFamily> = totals
        .into_iter()
        .filter(|(_, (_, instantiations, _))| *instantiations > 1)
        .map(|(name, (size, instantiations, largest))| GenericFamily {
            name,
            size,
            recoverable: size - largest,
            each: size / instantiations as u64,
            instantiations,
        })
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

pub(crate) fn demangle(mangled: &str) -> String {
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
