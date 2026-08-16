//! Read type and static-variable sizes from DWARF.
//!
//! Two things the linked binary cannot give exactly. Mach-O symbols carry no
//! size, so the symbol view infers a data static's size from the gap to the
//! next symbol — an upper bound. DWARF records the real `DW_AT_byte_size` of the
//! static's type, so this recovers exact sizes and retires the `≤` guessing when
//! debug info is present. It also ranks the largest named types — the
//! `-Zprint-type-sizes` insight without a nightly compiler — since a large type
//! is what drives the moves, copies, and drop glue the other views measure.

use std::{collections::HashMap, path::Path};

use anyhow::{Context, Result};
use serde::Serialize;

use crate::{dwarf::with_dwarf, name::demangle};

#[derive(Debug, Serialize)]
pub struct TypeReport {
    /// The largest named types, by declared byte size.
    pub largest: Vec<NamedType>,
}

#[derive(Debug, Serialize)]
pub struct NamedType {
    pub name: String,
    pub size: u64,
}

/// What the walk produced: the report, plus a map from a static's demangled name
/// to its exact byte size, for the symbol view to size read-only data exactly.
pub struct Types {
    pub report: TypeReport,
    pub static_sizes: HashMap<String, u64>,
}

/// Read the largest types and exact static sizes from the DWARF at `debug`.
///
/// # Errors
///
/// Errors when the debug info cannot be read or parsed.
pub fn analyze(debug: &Path, limit: usize) -> Result<Types> {
    with_dwarf(debug, |dwarf| {
        let mut largest: HashMap<String, u64> = HashMap::new();
        let mut static_sizes: HashMap<String, u64> = HashMap::new();

        let mut units = dwarf.units();
        while let Some(header) = units.next().context("failed to read a DWARF unit")? {
            let unit = dwarf.unit(header).context("failed to parse a DWARF unit")?;
            let unit = unit.unit_ref(dwarf);

            let mut entries = unit.entries();
            while let Some(entry) = entries.next_dfs().context("failed to read a DWARF DIE")? {
                match entry.tag() {
                    gimli::DW_TAG_structure_type
                    | gimli::DW_TAG_enumeration_type
                    | gimli::DW_TAG_union_type => {
                        if let (Some(name), Some(size)) =
                            (string(unit, entry, gimli::DW_AT_name), byte_size(entry))
                            && !is_transparent_wrapper(&name)
                        {
                            // The same type appears in many units; keep the
                            // largest reading (it may be forward-declared small
                            // elsewhere).
                            let slot = largest.entry(name).or_default();
                            *slot = (*slot).max(size);
                        }
                    }
                    gimli::DW_TAG_variable => {
                        if let (Some(name), Some(size)) = (
                            string(unit, entry, gimli::DW_AT_linkage_name),
                            variable_size(unit, entry),
                        ) {
                            static_sizes.entry(demangle(&name)).or_insert(size);
                        }
                    }
                    _ => {}
                }
            }
        }

        let mut largest: Vec<NamedType> =
            largest.into_iter().map(|(name, size)| NamedType { name, size }).collect();
        largest.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.name.cmp(&b.name)));
        largest.truncate(limit);

        Ok(Types { report: TypeReport { largest }, static_sizes })
    })
}

/// Transparent std wrappers that are the exact size of what they wrap, so in a
/// largest-types ranking they only clutter it with copies of the inner type.
fn is_transparent_wrapper(name: &str) -> bool {
    ["ManuallyDrop<", "MaybeUninit<", "MaybeDangling<", "Wrapping<", "Saturating<", "Reverse<"]
        .iter()
        .any(|wrapper| name.starts_with(wrapper))
}

/// A string attribute, if present.
fn string<R: gimli::Reader>(
    unit: gimli::UnitRef<'_, R>,
    entry: &gimli::DebuggingInformationEntry<R>,
    attribute: gimli::DwAt,
) -> Option<String> {
    let value = entry.attr_value(attribute)?;
    let string = unit.attr_string(value).ok()?;
    Some(string.to_string_lossy().ok()?.into_owned())
}

/// The `DW_AT_byte_size` of an entry, if it declares one.
fn byte_size<R: gimli::Reader>(entry: &gimli::DebuggingInformationEntry<R>) -> Option<u64> {
    entry.attr_value(gimli::DW_AT_byte_size)?.udata_value()
}

/// The byte size of a `DW_TAG_variable`, from the type it points at.
fn variable_size<R: gimli::Reader>(
    unit: gimli::UnitRef<'_, R>,
    entry: &gimli::DebuggingInformationEntry<R>,
) -> Option<u64> {
    type_size(unit, type_ref(entry)?, 0)
}

/// The `DW_AT_type` reference of an entry, if it is a unit-local one.
fn type_ref<R: gimli::Reader>(
    entry: &gimli::DebuggingInformationEntry<R>,
) -> Option<gimli::UnitOffset<R::Offset>> {
    match entry.attr_value(gimli::DW_AT_type)? {
        gimli::AttributeValue::UnitRef(offset) => Some(offset),
        _ => None,
    }
}

/// The byte size of the type at `offset`, following qualifiers and typedefs.
///
/// Reads `DW_AT_byte_size` where the type declares one (structs, enums, base
/// types); returns the pointer size for pointers; gives up on arrays without a
/// declared size, which fall back to the symbol view's gap inference. `depth`
/// guards against a malformed cyclic type graph.
fn type_size<R: gimli::Reader>(
    unit: gimli::UnitRef<'_, R>,
    offset: gimli::UnitOffset<R::Offset>,
    depth: u32,
) -> Option<u64> {
    if depth > 16 {
        return None;
    }

    let entry = unit.entry(offset).ok()?;
    if let Some(size) = byte_size(&entry) {
        return Some(size);
    }

    match entry.tag() {
        gimli::DW_TAG_typedef
        | gimli::DW_TAG_const_type
        | gimli::DW_TAG_volatile_type
        | gimli::DW_TAG_restrict_type
        | gimli::DW_TAG_atomic_type => type_size(unit, type_ref(&entry)?, depth + 1),
        gimli::DW_TAG_pointer_type | gimli::DW_TAG_reference_type => {
            Some(unit.encoding().address_size.into())
        }
        // An array carries no byte size of its own: it is the element size times
        // the product of its dimensions. This is what makes the big lookup
        // tables exact.
        gimli::DW_TAG_array_type => {
            let element = type_size(unit, type_ref(&entry)?, depth + 1)?;
            let mut count = 1u64;
            let mut tree = unit.entries_tree(Some(offset)).ok()?;
            let mut dimensions = tree.root().ok()?.children();
            while let Some(dimension) = dimensions.next().ok()? {
                if dimension.entry().tag() == gimli::DW_TAG_subrange_type {
                    count = count.checked_mul(subrange_count(dimension.entry())?)?;
                }
            }
            element.checked_mul(count)
        }
        _ => None,
    }
}

/// The element count of one array dimension, from `DW_AT_count` or, failing
/// that, `DW_AT_upper_bound + 1`.
fn subrange_count<R: gimli::Reader>(entry: &gimli::DebuggingInformationEntry<R>) -> Option<u64> {
    if let Some(count) = entry.attr_value(gimli::DW_AT_count) {
        return count.udata_value();
    }
    entry.attr_value(gimli::DW_AT_upper_bound)?.udata_value().map(|bound| bound + 1)
}

#[cfg(test)]
mod tests {
    use super::is_transparent_wrapper;

    #[test]
    fn transparent_wrappers_are_skipped() {
        assert!(is_transparent_wrapper("ManuallyDrop<alloc::string::String>"));
        assert!(is_transparent_wrapper("MaybeUninit<u8>"));
        assert!(!is_transparent_wrapper("alloc::vec::Vec<u8>"));
        assert!(!is_transparent_wrapper("MyManuallyDropLookalike"));
    }
}
