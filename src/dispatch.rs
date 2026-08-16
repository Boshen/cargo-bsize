//! The cost of dynamic dispatch and function-pointer coercion.
//!
//! A `dyn Trait` needs a vtable; coercing a function to a `fn` pointer or a
//! trait object needs a reify or drop shim. These are named
//! (`…::call_once::vtable`, `{shim:vtable#0}`), so they can be summed — a proxy
//! for how much the binary spends on indirection, and a lever in either
//! direction: fewer trait objects, or MORE `dyn` to collapse a generic family
//! the generic-families view flags.
//!
//! The vtables themselves — the arrays of method pointers — are anonymous in
//! Rust and cannot be attributed by name, so this is a floor.

use std::collections::HashMap;

use serde::Serialize;

use crate::symbols::{Total, sized_symbols};

#[derive(Debug, Serialize)]
pub struct DispatchReport {
    /// Named vtables (`…::vtable`).
    pub vtables: Group,

    /// Function-pointer coercion and drop shims (`{shim:…}`).
    pub shims: Group,

    pub largest: Vec<DispatchSymbol>,
}

#[derive(Debug, Serialize)]
pub struct Group {
    pub bytes: u64,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct DispatchSymbol {
    pub name: String,
    pub kind: String,
    pub size: u64,
}

/// Total the dynamic-dispatch machinery in `file`, keeping the `limit` largest
/// symbols. `static_sizes` supplies exact data sizes from DWARF.
pub fn analyze(
    file: &object::File<'_>,
    static_sizes: &HashMap<String, u64>,
    limit: usize,
) -> DispatchReport {
    let (code, data) = sized_symbols(file, static_sizes);

    let mut vtables = Total::default();
    let mut shims = Total::default();
    let mut largest: Vec<DispatchSymbol> = Vec::new();
    for symbol in code.iter().chain(&data) {
        let kind = if symbol.name.contains("vtable") {
            vtables.add(symbol.size);
            "vtable"
        } else if symbol.name.contains("{shim:") {
            shims.add(symbol.size);
            "shim"
        } else {
            continue;
        };
        largest.push(DispatchSymbol {
            name: symbol.name.clone(),
            kind: kind.to_owned(),
            size: symbol.size,
        });
    }

    largest.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.name.cmp(&b.name)));
    largest.truncate(limit);

    DispatchReport {
        vtables: Group { bytes: vtables.bytes, count: vtables.count },
        shims: Group { bytes: shims.bytes, count: shims.count },
        largest,
    }
}
