//! Break the code and read-only data sections apart into individual symbols.
//!
//! Mach-O symbols carry no size, so sizes come from sorting by address within a
//! section and taking the delta to the next symbol. ELF records a real size, so
//! that is preferred when present.
//!
//! Under `lto = "fat"` an inlined function has no symbol at all and its bytes
//! land on whatever inlined it, so this shows where code ended up rather than
//! where it was written.

use std::{
    collections::HashMap,
    hash::{BuildHasher, Hash},
};

use object::{Object, ObjectSection, ObjectSymbol, SectionIndex, SymbolSection};
use serde::Serialize;

use crate::{
    name::{
        defining_crate, demangle, generic_family, instantiating_crate, trait_method_of, trait_of,
    },
    sections::Category,
};

#[derive(Debug, Serialize)]
pub struct SymbolReport {
    pub code: SymbolSet,
    pub data: SymbolSet,

    /// The patterns the size literature keeps naming as causes of bloat. These
    /// overlap and do not sum to the code total. Each carries its largest
    /// individual offenders, so a category points at code to change.
    pub patterns: Vec<PatternGroup>,

    /// Every impl of one trait method, summed. The axis that concentrates best:
    /// oxlint's code is flat enough that its twenty largest functions are 4.9%
    /// of the binary, while `Rule::run` across 596 impls is 7.4% on its own.
    pub trait_methods: Vec<Group>,

    /// Every method of one trait, summed — the trait-method axis one step
    /// coarser. It gathers what the per-method and by-crate views scatter: an
    /// AST visitor is one `Visit` impl spread over ~200 `visit_*` methods, each
    /// attributed to the implementing rule's crate, so no other view adds the
    /// trait's mass into a single number. Each carries its largest impls.
    pub traits: Vec<TraitGroup>,

    /// Rollups cover code only. Inferred data sizes are upper bounds and would
    /// swamp them — `httparse::TOKEN_MAP` is a 256-byte table that absorbs
    /// 149 KiB of the anonymous constants following it.
    pub crates: Vec<Group>,
    pub generics: Vec<GenericFamily>,

    /// Generic code charged to the crate that caused the instantiation rather
    /// than the one that defined it.
    pub instantiated_by: Vec<Group>,
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

impl Symbol {
    fn new(mangled: &str, size: u64, exact: bool) -> Self {
        let name = demangle(mangled);
        Self {
            krate: defining_crate(mangled, &name),
            instantiated_by: instantiating_crate(mangled),
            name,
            size,
            exact,
            copies: 1,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Group {
    pub name: String,
    pub size: u64,
    pub symbols: usize,
}

#[derive(Debug, Serialize)]
pub struct PatternGroup {
    pub name: String,
    pub size: u64,
    pub symbols: usize,

    /// The largest individual offenders behind the total — each generic's
    /// instantiations summed — so a matched pattern names code to change rather
    /// than only a category.
    pub largest: Vec<Member>,
}

#[derive(Debug, Serialize)]
pub struct Member {
    pub name: String,
    pub bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct TraitGroup {
    pub name: String,
    pub size: u64,
    pub methods: usize,

    /// The largest individual impls behind the total — each type's
    /// instantiations summed — the concrete targets behind the trait's mass.
    pub largest: Vec<Member>,
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
/// `static_sizes` maps a data static's demangled name to its exact byte size
/// from DWARF; it replaces the gap inference where present. Empty when there is
/// no debug info.
pub fn analyze<S: BuildHasher>(
    file: &object::File<'_>,
    static_sizes: &HashMap<String, u64, S>,
    limit: usize,
) -> SymbolReport {
    let (code, data) = sized_symbols(file, static_sizes);
    let (code_bytes, data_bytes) = section_bytes(file);

    let patterns = patterns(&code);
    let trait_methods = rollup(&code, limit, |symbol| trait_method_of(&symbol.name));
    let traits = trait_groups(&code, limit);
    let crates = rollup(&code, limit, |symbol| symbol.krate.as_deref());
    let generics = generic_families(&code, limit);
    let instantiated_by = rollup(&code, limit, |symbol| symbol.instantiated_by.as_deref());

    SymbolReport {
        code: rank(code, code_bytes, limit),
        data: rank(data, data_bytes, limit),
        patterns,
        trait_methods,
        traits,
        crates,
        generics,
        instantiated_by,
    }
}

/// Every symbol in a code or read-only data section, split into `(code, data)`.
pub(crate) fn sized_symbols<S: BuildHasher>(
    file: &object::File<'_>,
    static_sizes: &HashMap<String, u64, S>,
) -> (Vec<Symbol>, Vec<Symbol>) {
    let mut code = Vec::new();
    let mut data = Vec::new();
    for sized in sized(file) {
        let mut symbol = Symbol::new(sized.mangled, sized.size, sized.exact);
        // DWARF gives read-only data an exact size where the gap inference could
        // only bound it.
        if !symbol.exact
            && let Some(&size) = static_sizes.get(&symbol.name)
        {
            symbol.size = size;
            symbol.exact = true;
        }
        if sized.category == Category::Code { code.push(symbol) } else { data.push(symbol) }
    }

    (code, data)
}

/// The bytes each code symbol occupies, by mangled name — the name assembly
/// and object files agree on.
pub(crate) fn code_sizes(file: &object::File<'_>) -> HashMap<String, u64> {
    sized(file)
        .into_iter()
        .filter(|sized| sized.category == Category::Code)
        .map(|sized| (sized.mangled.to_owned(), sized.size))
        .collect()
}

struct Sized<'data> {
    mangled: &'data str,
    size: u64,
    category: Category,

    /// `false` when the size came from the distance to the next symbol.
    exact: bool,
}

/// Every symbol in a code or read-only data section, with a size and whether
/// that size is exact.
///
/// Sizes inferred from the distance to the next symbol are only trustworthy
/// where symbols are dense. They are in code — oxlint names 14,668 symbols
/// across 11.3 MiB of `__text` — but not in the constant sections, where a
/// hundred-odd names cover a megabyte and each one absorbs the anonymous data
/// that follows it.
fn sized<'data>(file: &object::File<'data>) -> Vec<Sized<'data>> {
    let wanted: HashMap<SectionIndex, (u64, Category)> = file
        .sections()
        .filter_map(|section| {
            let category = Category::of(section.name().ok()?);
            matches!(category, Category::Code | Category::ReadOnlyData)
                .then(|| (section.index(), (section.address() + section.size(), category)))
        })
        .collect();

    let mut by_section: HashMap<SectionIndex, Vec<(u64, u64, &'data str)>> = HashMap::new();
    for symbol in file.symbols() {
        let SymbolSection::Section(index) = symbol.section() else { continue };
        let (Some(_), Ok(name)) = (wanted.get(&index), symbol.name()) else { continue };
        by_section.entry(index).or_default().push((symbol.address(), symbol.size(), name));
    }

    let mut sized = Vec::new();
    for (index, mut symbols) in by_section {
        let (end, category) = wanted[&index];
        symbols.sort_by_key(|&(address, ..)| address);
        symbols.dedup_by_key(|&mut (address, ..)| address);

        for (position, &(address, declared, mangled)) in symbols.iter().enumerate() {
            let next = symbols.get(position + 1).map_or(end, |&(address, ..)| address);
            let exact = declared > 0;
            let size = if exact { declared } else { next.saturating_sub(address) };
            sized.push(Sized { mangled, size, category, exact });
        }
    }

    sized
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

    merged.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.name.cmp(&b.name)));
    merged.truncate(limit);

    SymbolSet { bytes, section_bytes, count, largest: merged }
}

/// A running total, so accumulation reads as fields rather than tuple indices.
#[derive(Default)]
pub(crate) struct Total {
    pub bytes: u64,
    pub count: usize,

    /// The biggest single contribution, which is what a collapsed generic
    /// family would be left with.
    pub largest: u64,
}

impl Total {
    pub(crate) fn add(&mut self, bytes: u64) {
        self.bytes += bytes;
        self.count += 1;
        self.largest = self.largest.max(bytes);
    }
}

/// Sum `symbols` by `key`, skipping those without one, largest groups first.
fn rollup<'a, K>(
    symbols: &'a [Symbol],
    limit: usize,
    key: impl Fn(&'a Symbol) -> Option<K>,
) -> Vec<Group>
where
    K: Eq + Hash + Into<String>,
{
    let mut totals: HashMap<K, Total> = HashMap::new();
    for symbol in symbols {
        if let Some(name) = key(symbol) {
            totals.entry(name).or_default().add(symbol.size);
        }
    }

    let mut groups: Vec<Group> = totals
        .into_iter()
        .map(|(name, total)| Group { name: name.into(), size: total.bytes, symbols: total.count })
        .collect();
    groups.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.name.cmp(&b.name)));
    groups.truncate(limit);
    groups
}

fn generic_families(symbols: &[Symbol], limit: usize) -> Vec<GenericFamily> {
    let mut totals: HashMap<String, Total> = HashMap::new();
    for symbol in symbols {
        totals.entry(generic_family(&symbol.name)).or_default().add(symbol.size);
    }

    let mut families: Vec<GenericFamily> = totals
        .into_iter()
        .filter(|(_, total)| total.count > 1)
        .map(|(name, total)| GenericFamily {
            name,
            size: total.bytes,
            // Collapsing a family onto one copy returns all but its largest.
            recoverable: total.bytes - total.largest,
            each: total.bytes / total.count as u64,
            instantiations: total.count,
        })
        .collect();
    families.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.name.cmp(&b.name)));
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

/// How many of a rollup's largest offenders to name under its total.
const OFFENDERS: usize = 3;

/// A symbol can match several patterns — a serde deserializer written as a
/// closure is both — so these are counted independently rather than partitioned.
fn patterns(symbols: &[Symbol]) -> Vec<PatternGroup> {
    let mut groups: Vec<PatternGroup> = PATTERNS
        .iter()
        .map(|&(name, matches)| {
            let mut total = Total::default();
            // The offenders behind the total, each generic's instantiations
            // summed by family so one heavily-instantiated item stands out.
            let mut families: HashMap<String, u64> = HashMap::new();
            for symbol in symbols.iter().filter(|symbol| matches(&symbol.name)) {
                total.add(symbol.size);
                *families.entry(generic_family(&symbol.name)).or_default() += symbol.size;
            }

            PatternGroup {
                name: name.to_owned(),
                size: total.bytes,
                symbols: total.count,
                largest: largest_members(families, OFFENDERS),
            }
        })
        .filter(|group| group.size > 0)
        .collect();

    groups.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.name.cmp(&b.name)));
    groups
}

/// Sum code by the trait it implements, largest first, each row carrying its
/// largest individual impls (each type's instantiations summed) as the concrete
/// targets behind the trait's mass.
fn trait_groups(symbols: &[Symbol], limit: usize) -> Vec<TraitGroup> {
    let mut totals: HashMap<String, Total> = HashMap::new();
    let mut members: HashMap<String, HashMap<String, u64>> = HashMap::new();
    for symbol in symbols {
        if let Some(trait_name) = trait_of(&symbol.name) {
            totals.entry(trait_name.clone()).or_default().add(symbol.size);
            *members
                .entry(trait_name)
                .or_default()
                .entry(generic_family(&symbol.name))
                .or_default() += symbol.size;
        }
    }

    let mut groups: Vec<TraitGroup> = totals
        .into_iter()
        .map(|(name, total)| TraitGroup {
            largest: largest_members(members.remove(&name).unwrap_or_default(), OFFENDERS),
            name,
            size: total.bytes,
            methods: total.count,
        })
        .collect();
    groups.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.name.cmp(&b.name)));
    groups.truncate(limit);
    groups
}

/// The `count` largest families in `families` (name → summed bytes), largest
/// first.
fn largest_members(families: HashMap<String, u64>, count: usize) -> Vec<Member> {
    let mut members: Vec<Member> =
        families.into_iter().map(|(name, bytes)| Member { name, bytes }).collect();
    members.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
    members.truncate(count);
    members
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{OFFENDERS, largest_members};

    #[test]
    fn ranks_the_largest_offenders() {
        let families = HashMap::from([
            ("a::{closure#0}".to_owned(), 100),
            ("b::{closure#0}".to_owned(), 400),
            ("c::{closure#0}".to_owned(), 200),
            ("d::{closure#0}".to_owned(), 50),
        ]);

        let largest = largest_members(families, OFFENDERS);

        assert_eq!(largest.len(), OFFENDERS);
        assert_eq!((largest[0].name.as_str(), largest[0].bytes), ("b::{closure#0}", 400));
        assert_eq!(largest[1].bytes, 200);
        assert_eq!(largest[2].bytes, 100);
    }
}
