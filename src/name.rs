//! Reading structure out of Rust symbol names.
//!
//! Every rollup in the report is some projection of a name: the crate that
//! defined it, the crate that instantiated it, the module it sits in, the trait
//! method it implements, the generic it is one instantiation of. All of it is
//! string work over demangled paths and v0 mangling, with no binary involved.

/// Demangle a symbol, dropping crate disambiguator hashes.
pub fn demangle(mangled: &str) -> String {
    // Mach-O prefixes every symbol with an underscore.
    let trimmed = mangled
        .strip_prefix('_')
        .filter(|rest| rest.starts_with("_R") || rest.starts_with("_Z"))
        .unwrap_or(mangled);

    format!("{:#}", rustc_demangle::demangle(trimmed))
}

/// The crate a symbol is defined in — the first crate named in the mangled path.
pub fn defining_crate(mangled: &str, demangled: &str) -> Option<String> {
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
pub fn instantiating_crate(mangled: &str) -> Option<String> {
    (0..mangled.len()).rev().find_map(|index| {
        crate_at(mangled, index)
            .filter(|&(_, end)| end == mangled.len())
            .map(|(name, _)| name.to_owned())
    })
}

/// One method of one trait, so every impl of it sums into a single row.
pub fn trait_method_of(name: &str) -> Option<String> {
    let (_, trait_name, path) = split_qualified(name);
    let method = path.split("::").next().filter(|method| !method.is_empty())?;

    Some(format!("<{}>::{method}", strip_generics(trait_name?)))
}

/// One trait, so every method of every impl of it sums into a single row —
/// `trait_method_of` one axis coarser. `<Foo as Bar<T>>::baz` yields `Bar`.
/// `None` unless the symbol is a trait-method impl, matching what
/// `trait_method_of` counts.
pub fn trait_of(name: &str) -> Option<String> {
    let (_, trait_name, path) = split_qualified(name);
    let trait_name = trait_name?;
    // Require a method segment too, so this is exactly the trait-method impls
    // and not a bare qualified path.
    path.split("::").next().filter(|method| !method.is_empty())?;

    Some(strip_generics(trait_name))
}

/// Drop turbofish arguments so every instantiation of one generic shares a name.
pub fn generic_family(name: &str) -> String {
    let mut family = String::with_capacity(name.len());
    let mut rest = name;

    while let Some(start) = rest.find("::<") {
        family.push_str(&rest[..start]);

        match closing_bracket(&rest[start + 2..]) {
            Some(close) => rest = &rest[start + 2 + close + 1..],
            None => return family,
        }
    }

    family.push_str(rest);
    family
}

/// Split a demangled name into `(self type, trait, remaining path)`.
///
/// `<Foo as Bar>::baz` yields `(Foo, Bar, baz)`, `<Foo>::baz` yields
/// `(Foo, None, baz)`, and a plain path yields `(None, None, path)`.
fn split_qualified(name: &str) -> (Option<&str>, Option<&str>, &str) {
    if !name.starts_with('<') {
        return (None, None, name);
    }

    let Some(close) = closing_bracket(name) else { return (None, None, name) };
    let rest = name[close + 1..].trim_start_matches(':');

    match name[1..close].split_once(" as ") {
        Some((self_type, trait_name)) => (Some(self_type), Some(trait_name), rest),
        None => (Some(&name[1..close]), None, rest),
    }
}

/// Offset of the `>` closing the first `<` in `name`, honouring nesting. The
/// `>` of an fn-pointer arrow (`fn(u32) -> u32`) is not a bracket; counting it
/// would close the turbofish one `>` early.
fn closing_bracket(name: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut previous = '\0';

    for (offset, character) in name.char_indices() {
        match character {
            '<' => depth += 1,
            '>' if previous != '-' => {
                depth -= 1;
                if depth == 0 {
                    return Some(offset);
                }
            }
            _ => {}
        }
        previous = character;
    }

    None
}

/// Drop every generic argument, so `Router<Backend, Error>` becomes `Router`.
/// Without this the `::` inside a type argument is mistaken for a module split.
fn strip_generics(name: &str) -> String {
    let mut stripped = String::with_capacity(name.len());
    let mut depth = 0usize;
    let mut previous = '\0';

    for character in name.chars() {
        match character {
            '<' => depth += 1,
            // The `>` of an fn-pointer arrow is not a bracket.
            '>' if previous != '-' => depth = depth.saturating_sub(1),
            _ if depth == 0 => stripped.push(character),
            _ => {}
        }
        previous = character;
    }

    stripped
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

#[cfg(test)]
mod tests {
    use super::{generic_family, trait_method_of, trait_of};

    #[test]
    fn brackets_ignore_fn_pointer_arrows() {
        // The arrow's `>` must not close the turbofish early: this used to
        // yield the malformed family `core::ptr::drop_glue>`.
        assert_eq!(
            generic_family("core::ptr::drop_glue::<alloc::vec::Vec<fn(u32) -> u32>>"),
            "core::ptr::drop_glue"
        );
        // Nested turbofish still collapses cleanly.
        assert_eq!(generic_family("a::f::<b::G::<u8>>"), "a::f");
        // A trait with an fn-pointer parameter keeps its name intact through
        // `strip_generics`.
        assert_eq!(
            trait_of("<X as tower::Layer<fn(A) -> B>>::layer").as_deref(),
            Some("tower::Layer")
        );
    }

    #[test]
    fn trait_of_combines_every_method_of_a_trait() {
        let visit = "<oxc_linter::rules::Foo as oxc_ast_visit::VisitJs>::visit_expression";
        assert_eq!(
            trait_method_of(visit).as_deref(),
            Some("<oxc_ast_visit::VisitJs>::visit_expression")
        );
        assert_eq!(trait_of(visit).as_deref(), Some("oxc_ast_visit::VisitJs"));

        // Generic arguments on the trait are dropped so every impl shares a row.
        let call = "<Svc as tower_service::Service<Req>>::call";
        assert_eq!(trait_of(call).as_deref(), Some("tower_service::Service"));

        // An inherent method (no `as Trait`) and a free function have no trait.
        assert_eq!(trait_of("<oxc_linter::Foo>::run"), None);
        assert_eq!(trait_of("oxc_linter::rules::run"), None);
    }
}
