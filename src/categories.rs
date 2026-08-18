//! Two rollups the per-symbol views do not name: the cost of each derivable
//! trait, and the fraction of code that is cold.
//!
//! Grouping impls of a derivable trait shows where `Debug`, `Clone`, … code
//! sits across the binary. The match is on the trait impl, so it counts derived
//! and hand-written impls alike — the symbol does not record which — and the
//! total is descriptive attribution spread over many independent types, not what
//! removing one `#[derive]` would save: a bound such as `Rule: Debug` keeps
//! every impl even after one is rewritten. Cold code is the `.text.unlikely`
//! the compiler splits off for panic and error paths (ELF only; Mach-O keeps it
//! in `__text`, so this reads zero there).

use object::{Object, ObjectSection};
use rustc_hash::FxHashMap;
use serde::Serialize;

use crate::{
    name::generic_family,
    symbols::{Total, sized_symbols},
};

/// How many of a derive's largest impls to name under its total.
const IMPLS_PER_DERIVE: usize = 3;

#[derive(Debug, Serialize)]
pub struct CategoryReport {
    /// Code grouped by the `#[derive]` it implements, largest first.
    pub derives: Vec<Derive>,

    /// Bytes of code the compiler marked cold (`.text.unlikely`).
    pub cold: u64,
}

#[derive(Debug, Serialize)]
pub struct Derive {
    pub name: String,
    pub bytes: u64,
    pub impls: usize,

    /// The largest individual impls behind `bytes` — each type's instantiations
    /// summed. The total is attribution across `impls` of these, so these named
    /// few are the ones worth acting on.
    pub largest: Vec<Impl>,
}

#[derive(Debug, Serialize)]
pub struct Impl {
    pub name: String,
    pub bytes: u64,
}

/// The derivable traits, matched on the `as <trait>>` of a demangled impl.
const DERIVES: [(&str, &str); 9] = [
    ("as core::fmt::Debug>", "Debug"),
    ("as core::clone::Clone>", "Clone"),
    ("as core::cmp::PartialEq>", "PartialEq"),
    ("as core::cmp::PartialOrd>", "PartialOrd"),
    ("as core::cmp::Ord>", "Ord"),
    ("as core::hash::Hash>", "Hash"),
    ("as core::default::Default>", "Default"),
    ("::ser::Serialize>", "Serialize"),
    ("::de::Deserialize>", "Deserialize"),
];

/// Group `file`'s code by derive and total its cold code. `static_sizes` is
/// unused for code but keeps the symbol sizing consistent with the other views.
pub fn analyze(
    file: &object::File<'_>,
    static_sizes: &FxHashMap<String, u64>,
    limit: usize,
) -> CategoryReport {
    let (code, _) = sized_symbols(file, static_sizes);

    let mut totals: FxHashMap<&'static str, Total> = FxHashMap::default();
    // Per derive, each distinct impl's code (its instantiations summed by type),
    // to rank the largest individual impls behind the total.
    let mut members: FxHashMap<&'static str, FxHashMap<String, u64>> = FxHashMap::default();
    for symbol in &code {
        if let Some(derive) = derive_of(&symbol.name) {
            totals.entry(derive).or_default().add(symbol.size);
            *members.entry(derive).or_default().entry(generic_family(&symbol.name)).or_default() +=
                symbol.size;
        }
    }

    let mut derives: Vec<Derive> = totals
        .into_iter()
        .map(|(name, total)| Derive {
            largest: largest_impls(members.remove(name).unwrap_or_default()),
            name: name.to_owned(),
            bytes: total.bytes,
            impls: total.count,
        })
        .collect();
    derives.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
    derives.truncate(limit);

    let cold = file
        .sections()
        .filter_map(|section| {
            let name = section.name().ok()?;
            let (_, size) = section.file_range()?;
            (name.contains(".text.unlikely") || name.contains("__text_cold")).then_some(size)
        })
        .sum();

    CategoryReport { derives, cold }
}

/// The derive a demangled impl name belongs to, if any.
fn derive_of(name: &str) -> Option<&'static str> {
    DERIVES.iter().find_map(|&(needle, derive)| name.contains(needle).then_some(derive))
}

/// The `IMPLS_PER_DERIVE` largest impls in `members` (impl name → summed bytes),
/// largest first.
fn largest_impls(members: FxHashMap<String, u64>) -> Vec<Impl> {
    let mut impls: Vec<Impl> =
        members.into_iter().map(|(name, bytes)| Impl { name, bytes }).collect();
    impls.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
    impls.truncate(IMPLS_PER_DERIVE);
    impls
}

#[cfg(test)]
mod tests {
    use rustc_hash::FxHashMap;

    use super::{IMPLS_PER_DERIVE, derive_of, largest_impls};

    #[test]
    fn matches_derivable_trait_impls() {
        assert_eq!(derive_of("<oxc_ast::Foo as core::fmt::Debug>::fmt"), Some("Debug"));
        assert_eq!(derive_of("<Foo as core::clone::Clone>::clone"), Some("Clone"));
        assert_eq!(
            derive_of("<Foo as serde_core::de::Deserialize>::deserialize"),
            Some("Deserialize")
        );
        assert_eq!(derive_of("<Foo as oxc_codegen::gen::Gen>::gen"), None);
    }

    #[test]
    fn ranks_the_largest_impls() {
        let members: FxHashMap<_, _> = [
            ("<A as Debug>::fmt".to_owned(), 100),
            ("<B as Debug>::fmt".to_owned(), 300),
            ("<C as Debug>::fmt".to_owned(), 200),
            ("<D as Debug>::fmt".to_owned(), 50),
        ]
        .into_iter()
        .collect();

        let largest = largest_impls(members);

        assert_eq!(largest.len(), IMPLS_PER_DERIVE);
        assert_eq!((largest[0].name.as_str(), largest[0].bytes), ("<B as Debug>::fmt", 300));
        assert_eq!(largest[1].bytes, 200);
        assert_eq!(largest[2].bytes, 100);
    }
}
