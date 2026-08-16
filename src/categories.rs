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

use std::collections::HashMap;

use object::{Object, ObjectSection};
use serde::Serialize;

use crate::symbols::{Total, sized_symbols};

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
    static_sizes: &HashMap<String, u64>,
    limit: usize,
) -> CategoryReport {
    let (code, _) = sized_symbols(file, static_sizes);

    let mut totals: HashMap<&'static str, Total> = HashMap::new();
    for symbol in &code {
        if let Some(derive) = derive_of(&symbol.name) {
            totals.entry(derive).or_default().add(symbol.size);
        }
    }

    let mut derives: Vec<Derive> = totals
        .into_iter()
        .map(|(name, total)| Derive {
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
