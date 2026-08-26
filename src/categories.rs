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

use crate::{
    name::generic_family,
    symbols::{Member, Symbol, Total, largest_members},
};

/// How many of a derive's largest impls to name under its total.
const IMPLS_PER_DERIVE: usize = 3;

#[derive(Debug)]
pub struct CategoryReport {
    /// Code grouped by the `#[derive]` it implements, largest first.
    pub derives: Vec<Derive>,

    /// Bytes of code the compiler marked cold (`.text.unlikely`).
    pub cold: u64,
}

#[derive(Debug)]
pub struct Derive {
    pub name: String,
    pub bytes: u64,
    pub impls: usize,

    /// The largest individual impls behind `bytes` — each type's instantiations
    /// summed. The total is attribution across `impls` of these, so these named
    /// few are the ones worth acting on.
    pub largest: Vec<Member>,
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

/// Group the `code` symbols by derive and total `file`'s cold code.
pub fn analyze(file: &object::File<'_>, code: &[Symbol], limit: usize) -> CategoryReport {
    let mut totals: FxHashMap<&'static str, Total> = FxHashMap::default();
    // Per derive, each distinct impl's code (its instantiations summed by type),
    // to rank the largest individual impls behind the total.
    let mut members: FxHashMap<&'static str, FxHashMap<String, u64>> = FxHashMap::default();
    for symbol in code {
        if let Some(derive) = derive_of(&symbol.name) {
            totals.entry(derive).or_default().add(symbol.size);
            *members.entry(derive).or_default().entry(generic_family(&symbol.name)).or_default() +=
                symbol.size;
        }
    }

    let mut derives: Vec<Derive> = totals
        .into_iter()
        .map(|(name, total)| Derive {
            largest: largest_members(members.remove(name).unwrap_or_default(), IMPLS_PER_DERIVE),
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

#[cfg(test)]
mod tests {
    use super::derive_of;

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
}
