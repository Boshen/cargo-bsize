//! Which types the generic code was instantiated over.
//!
//! A turbofish names the types a generic was specialized to —
//! `core::ptr::drop_glue::<oxc_ast::ast::js::Statement>` is drop code *for the
//! AST*, `core::slice::sort::…::quicksort::<serde_json::Value, …>` is sort code
//! for JSON values — but every other rollup discards it: the generic-families
//! view strips the turbofish, and the by-crate views key on the *defining*
//! crate, which for all of the above is `core`. This view flips the key: each
//! instantiation's bytes — out-of-line symbols plus inlined instances — count
//! toward the crates its type arguments name, with the largest generic
//! families under each. "Do the AST types drop a lot?" is the `oxc_ast` row's
//! `core::ptr::drop_glue` line, and the same row answers it for sorts, copies,
//! and visitors at once.
//!
//! Distinct from "by crate, which one caused the instantiation": that reads
//! v0 mangling's instantiating-crate suffix — who *asked* for the code. This
//! is what the code is specialized *to*.

use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    inlined::InlinedFunction,
    name::{generic_family, turbofish},
    symbols::{Member, sized_symbols},
};

/// How many generic families to name under each crate's total.
const FAMILIES: usize = 3;

#[derive(Debug)]
pub struct InstantiationReport {
    /// Out-of-line bytes of symbols carrying a turbofish.
    pub bytes: u64,
    pub symbols: usize,

    /// The same for inlined instances; zero without debug info.
    pub inlined_bytes: u64,
    pub instances: usize,

    /// Ranked by combined bytes. A single instantiation counts toward every
    /// crate its type arguments name, so rows overlap and do not sum.
    pub crates: Vec<TypeUse>,
}

#[derive(Debug)]
pub struct TypeUse {
    /// The crate the type arguments name.
    pub name: String,

    /// Combined out-of-line and inlined bytes.
    pub bytes: u64,
    pub instantiations: usize,

    /// The largest generic families behind the total — which generic code the
    /// crate's types are paying for.
    pub largest: Vec<Member>,
}

/// Attribute every turbofished instantiation in `file` — and the `inlined`
/// instances — to the crates its type arguments name, keeping the `limit`
/// largest crates.
pub fn analyze(
    file: &object::File<'_>,
    static_sizes: &FxHashMap<String, u64>,
    inlined: &[InlinedFunction],
    limit: usize,
) -> InstantiationReport {
    let (code, _) = sized_symbols(file, static_sizes);

    build(
        code.iter().map(|symbol| (symbol.name.as_str(), symbol.size)),
        inlined.iter().map(|function| (function.name.as_str(), function.bytes, function.sites)),
        limit,
    )
}

#[derive(Default)]
struct Use {
    bytes: u64,
    instantiations: usize,
    families: FxHashMap<String, u64>,
}

fn build<'a>(
    symbols: impl IntoIterator<Item = (&'a str, u64)>,
    inlined: impl IntoIterator<Item = (&'a str, u64, usize)>,
    limit: usize,
) -> InstantiationReport {
    let mut report = InstantiationReport {
        bytes: 0,
        symbols: 0,
        inlined_bytes: 0,
        instances: 0,
        crates: Vec::new(),
    };
    let mut uses: FxHashMap<String, Use> = FxHashMap::default();

    let mut attribute = |name: &str, bytes: u64| {
        let Some(arguments) = turbofish(name) else { return false };

        let found = argument_crates(arguments);
        let interesting: Vec<&str> = found
            .iter()
            .map(String::as_str)
            .filter(|krate| !matches!(*krate, "core" | "alloc" | "std"))
            .collect();
        let targets: Vec<&str> = if !interesting.is_empty() {
            interesting
        } else if !found.is_empty() {
            found.iter().map(String::as_str).collect()
        } else {
            // `::<u8, 336>` and friends name no crate at all.
            vec!["(primitives)"]
        };

        let family = generic_family(name);
        for target in targets {
            let entry = uses.entry(target.to_owned()).or_default();
            entry.bytes += bytes;
            entry.instantiations += 1;
            *entry.families.entry(family.clone()).or_default() += bytes;
        }
        true
    };

    for (name, bytes) in symbols {
        if attribute(name, bytes) {
            report.bytes += bytes;
            report.symbols += 1;
        }
    }
    for (name, bytes, sites) in inlined {
        if attribute(name, bytes) {
            report.inlined_bytes += bytes;
            report.instances += sites;
        }
    }

    let mut crates: Vec<TypeUse> = uses
        .into_iter()
        .map(|(name, r#use)| {
            let mut largest: Vec<Member> =
                r#use.families.into_iter().map(|(name, bytes)| Member { name, bytes }).collect();
            largest.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
            largest.truncate(FAMILIES);

            TypeUse { name, bytes: r#use.bytes, instantiations: r#use.instantiations, largest }
        })
        .collect();
    crates.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
    crates.truncate(limit);
    report.crates = crates;

    report
}

/// The crates the type arguments name, in order of first appearance: each
/// path's first segment, found where an identifier followed by `::` begins —
/// at the start, or after one of `< [ ( & , ; * ` or a space.
fn argument_crates(arguments: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    let mut seen: FxHashSet<&str> = FxHashSet::default();

    let mut run: Option<usize> = None;
    let mut before = '\0';
    let mut previous = '\0';
    for (offset, character) in arguments.char_indices() {
        let identifier = character.is_ascii_alphanumeric() || character == '_';
        match run {
            None if identifier => {
                run = Some(offset);
                before = previous;
            }
            Some(start) if !identifier => {
                let segment = &arguments[start..offset];
                if matches!(before, '\0' | '<' | '[' | '(' | '&' | ',' | ';' | '*' | ' ')
                    && arguments[offset..].starts_with("::")
                    && !segment.starts_with(|c: char| c.is_ascii_digit())
                    && seen.insert(segment)
                {
                    found.push(segment.to_owned());
                }
                run = None;
            }
            _ => {}
        }
        previous = character;
    }

    found
}

#[cfg(test)]
mod tests {
    use super::{argument_crates, build};

    #[test]
    fn finds_the_crates_the_arguments_name() {
        assert_eq!(argument_crates("oxc_ast::ast::js::Statement"), ["oxc_ast"]);
        assert_eq!(
            argument_crates("alloc::vec::Vec<oxc_ast::ast::js::Statement>"),
            ["alloc", "oxc_ast"]
        );
        assert_eq!(argument_crates("[serde_json::value::Value]"), ["serde_json"]);
        assert_eq!(argument_crates("(oxc_a::X, serde_b::Y)"), ["oxc_a", "serde_b"]);
        assert_eq!(argument_crates("&mut oxc_x::Y"), ["oxc_x"]);
        // The arrow's target is a path too; the fn keyword is not.
        assert_eq!(argument_crates("fn(u32) -> oxc_ast::X"), ["oxc_ast"]);
        // Deduplicated, in order of first appearance.
        assert_eq!(argument_crates("dyn core::fmt::Debug + core::marker::Send"), ["core"]);
        // Primitive-only arguments name nothing.
        assert_eq!(argument_crates("u8, 336"), Vec::<String>::new());
    }

    #[test]
    fn attributes_bytes_to_the_types_crates() {
        let symbols = [
            ("core::ptr::drop_glue::<oxc_ast::ast::js::Statement>", 300),
            (
                "core::slice::sort::unstable::quicksort::quicksort::<oxc_ast::ast::js::Statement>",
                500,
            ),
            // `alloc` counts only when nothing more interesting is named.
            ("core::ptr::drop_glue::<alloc::string::String>", 100),
            // No turbofish: not an instantiation.
            ("oxc_linter::run", 9000),
        ];
        let inlined = [("core::ptr::drop_glue::<[oxc_ast::ast::js::Statement]>", 200, 50)];

        let report = build(symbols, inlined, 10);

        assert_eq!(report.bytes, 900);
        assert_eq!(report.symbols, 3);
        assert_eq!(report.inlined_bytes, 200);
        assert_eq!(report.instances, 50);

        assert_eq!(report.crates.len(), 2);
        assert_eq!(report.crates[0].name, "oxc_ast");
        assert_eq!(report.crates[0].bytes, 1000);
        assert_eq!(report.crates[0].instantiations, 3);
        // The families behind the total, largest first.
        assert_eq!(report.crates[0].largest[0].name, "core::ptr::drop_glue");
        assert_eq!(report.crates[0].largest[0].bytes, 500);
        assert_eq!(report.crates[1].name, "alloc");
        assert_eq!(report.crates[1].bytes, 100);
    }
}
