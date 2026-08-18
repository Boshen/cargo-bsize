//! Read type and static-variable sizes from DWARF.
//!
//! Two things the linked binary cannot give exactly. Mach-O symbols carry no
//! size, so the symbol view infers a data static's size from the gap to the
//! next symbol — an upper bound. DWARF records the real `DW_AT_byte_size` of the
//! static's type, so this recovers exact sizes and retires the `≤` guessing when
//! debug info is present. It also ranks the largest named types — the
//! `-Zprint-type-sizes` insight without a nightly compiler — since a large type
//! is what drives the moves, copies, and drop glue the other views measure.

use std::path::Path;

use anyhow::{Context, Result};
use gimli::ReaderOffset;
use rustc_hash::{FxHashMap, FxHashSet};
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
    pub static_sizes: FxHashMap<String, u64>,
}

/// Read the largest types and exact static sizes from the DWARF at `debug`.
///
/// # Errors
///
/// Errors when the debug info cannot be read or parsed.
pub fn analyze(debug: &Path, limit: usize) -> Result<Types> {
    with_dwarf(debug, |dwarf| {
        // Types reference each other across compilation units — a shared
        // primitive is a `DW_FORM_ref_addr` — so every type is keyed by its
        // global `.debug_info` offset in one pass and resolved in a second.
        let mut kinds = Kinds::default();
        let mut variables: Vec<(String, u64)> = Vec::new();
        let mut largest: FxHashMap<String, u64> = FxHashMap::default();
        let mut address_size = 8;

        let mut units = dwarf.units();
        while let Some(header) = units.next().context("failed to read a DWARF unit")? {
            let base = header.debug_info_offset().map_or(0, |offset| offset.0.into_u64());
            let unit = dwarf.unit(header).context("failed to parse a DWARF unit")?;
            let unit = unit.unit_ref(dwarf);
            address_size = unit.encoding().address_size.into();

            // Walk the DIE tree, not a flat list, so a type knows its parent.
            // An enum's variant field-structs are emitted as child structure DIEs
            // of the enum, each repeating the enum's byte size under the variant's
            // name (`Panicked`, `Suspend0`, …); they are not distinct types, and
            // skipping them from the ranking needs that parent link.
            let mut tree = unit.entries_tree(None).context("failed to read the DWARF tree")?;
            let root = tree.root().context("failed to read the DWARF root")?;
            collect(unit, root, base, false, &mut kinds, &mut variables, &mut largest)?;
        }

        let mut static_sizes: FxHashMap<String, u64> = FxHashMap::default();
        for (name, ty) in variables {
            if let Some(size) = kinds.size(ty, address_size, 0) {
                static_sizes.entry(name).or_insert(size);
            }
        }

        let mut largest: Vec<NamedType> =
            largest.into_iter().map(|(name, size)| NamedType { name, size }).collect();
        largest.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.name.cmp(&b.name)));
        largest.truncate(limit);

        Ok(Types { report: TypeReport { largest }, static_sizes })
    })
}

/// Walk a DIE and its children, recording every type by global offset and every
/// static variable. `parent_is_type` is true when the parent DIE is a struct,
/// enum, or union — the mark of an enum variant field-struct, kept out of the
/// largest-types ranking so a variant cannot masquerade as a distinct type the
/// size of its whole enum.
fn collect<R: gimli::Reader>(
    unit: gimli::UnitRef<'_, R>,
    node: gimli::EntriesTreeNode<'_, '_, R>,
    base: u64,
    parent_is_type: bool,
    kinds: &mut Kinds,
    variables: &mut Vec<(String, u64)>,
    largest: &mut FxHashMap<String, u64>,
) -> Result<()> {
    let entry = node.entry();
    let offset = base + entry.offset().0.into_u64();
    let tag = entry.tag();
    match tag {
        gimli::DW_TAG_structure_type
        | gimli::DW_TAG_enumeration_type
        | gimli::DW_TAG_union_type => {
            if let Some(size) = byte_size(entry) {
                kinds.sizes.insert(offset, size);
                if let Some(name) = string(unit, entry, gimli::DW_AT_name)
                    && ranks_as_named_type(&name, parent_is_type)
                {
                    // One type appears in many units; keep the largest reading.
                    let slot = largest.entry(name).or_default();
                    *slot = (*slot).max(size);
                }
            }
        }
        gimli::DW_TAG_base_type => {
            if let Some(size) = byte_size(entry) {
                kinds.sizes.insert(offset, size);
            }
        }
        gimli::DW_TAG_array_type => {
            if let (Some(element), Some(count)) = (type_ref(entry, base), array_count(unit, entry))
            {
                kinds.arrays.insert(offset, (element, count));
            }
        }
        gimli::DW_TAG_typedef
        | gimli::DW_TAG_const_type
        | gimli::DW_TAG_volatile_type
        | gimli::DW_TAG_restrict_type
        | gimli::DW_TAG_atomic_type => {
            if let Some(target) = type_ref(entry, base) {
                kinds.aliases.insert(offset, target);
            }
        }
        gimli::DW_TAG_pointer_type | gimli::DW_TAG_reference_type => {
            kinds.pointers.insert(offset);
        }
        gimli::DW_TAG_variable => {
            if let (Some(name), Some(ty)) =
                (string(unit, entry, gimli::DW_AT_linkage_name), type_ref(entry, base))
            {
                variables.push((demangle(&name), ty));
            }
        }
        _ => {}
    }

    // Only a struct, enum, or union can be the parent of a variant field-struct.
    let this_is_type = matches!(
        tag,
        gimli::DW_TAG_structure_type | gimli::DW_TAG_enumeration_type | gimli::DW_TAG_union_type
    );
    let mut children = node.children();
    while let Some(child) = children.next().context("failed to read a DWARF child")? {
        collect(unit, child, base, this_is_type, kinds, variables, largest)?;
    }
    Ok(())
}

/// Whether a named type belongs in the largest-types ranking: not an enum
/// variant field-struct (whose parent is the enum type, so `parent_is_type`),
/// and not a transparent wrapper that is only ever the size of what it wraps.
fn ranks_as_named_type(name: &str, parent_is_type: bool) -> bool {
    !parent_is_type && !is_transparent_wrapper(name)
}

/// Every type, keyed by its global `.debug_info` offset, so a cross-unit
/// reference resolves like any other.
#[derive(Default)]
struct Kinds {
    /// Types that declare a `DW_AT_byte_size` directly (structs, enums, base
    /// types).
    sizes: FxHashMap<u64, u64>,

    /// Arrays, as `(element type, element count)`.
    arrays: FxHashMap<u64, (u64, u64)>,

    /// Typedefs and qualifiers, pointing at the type they wrap.
    aliases: FxHashMap<u64, u64>,

    /// Pointer and reference types, all the target's address size.
    pointers: FxHashSet<u64>,
}

impl Kinds {
    /// The byte size of the type at global offset `ty`. `depth` guards a
    /// malformed cyclic type graph.
    fn size(&self, ty: u64, address_size: u64, depth: u32) -> Option<u64> {
        if depth > 32 {
            return None;
        }
        if let Some(&size) = self.sizes.get(&ty) {
            return Some(size);
        }
        if self.pointers.contains(&ty) {
            return Some(address_size);
        }
        if let Some(&target) = self.aliases.get(&ty) {
            return self.size(target, address_size, depth + 1);
        }
        if let Some(&(element, count)) = self.arrays.get(&ty) {
            return self.size(element, address_size, depth + 1)?.checked_mul(count);
        }
        None
    }
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

/// The global `.debug_info` offset an entry's `DW_AT_type` points at, whether it
/// is a within-unit `DW_FORM_ref4` or a cross-unit `DW_FORM_ref_addr`. `base` is
/// the referring unit's offset in `.debug_info`.
fn type_ref<R: gimli::Reader>(
    entry: &gimli::DebuggingInformationEntry<R>,
    base: u64,
) -> Option<u64> {
    match entry.attr_value(gimli::DW_AT_type)? {
        gimli::AttributeValue::UnitRef(offset) => Some(base + offset.0.into_u64()),
        gimli::AttributeValue::DebugInfoRef(offset) => Some(offset.0.into_u64()),
        _ => None,
    }
}

/// The total element count of an array — the product of its dimensions — or
/// `None` if it declares none.
fn array_count<R: gimli::Reader>(
    unit: gimli::UnitRef<'_, R>,
    entry: &gimli::DebuggingInformationEntry<R>,
) -> Option<u64> {
    let mut tree = unit.entries_tree(Some(entry.offset())).ok()?;
    let mut dimensions = tree.root().ok()?.children();

    let mut count = 1u64;
    let mut any = false;
    while let Some(dimension) = dimensions.next().ok()? {
        if dimension.entry().tag() == gimli::DW_TAG_subrange_type {
            count = count.checked_mul(subrange_count(dimension.entry())?)?;
            any = true;
        }
    }

    any.then_some(count)
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
    use super::{Kinds, is_transparent_wrapper, ranks_as_named_type};

    #[test]
    fn resolves_sizes_across_units_and_through_wrappers() {
        let mut kinds = Kinds::default();
        kinds.sizes.insert(1, 1); // a `u8` base type at global offset 1
        kinds.arrays.insert(2, (1, 256)); // `[u8; 256]`, element cross-unit at 1
        kinds.aliases.insert(3, 2); // a typedef of the array
        kinds.pointers.insert(4); // a pointer

        assert_eq!(kinds.size(1, 8, 0), Some(1));
        assert_eq!(kinds.size(2, 8, 0), Some(256)); // 1 × 256 — the case ref_addr broke
        assert_eq!(kinds.size(3, 8, 0), Some(256)); // followed through the typedef
        assert_eq!(kinds.size(4, 8, 0), Some(8)); // the address size
        assert_eq!(kinds.size(99, 8, 0), None); // unknown offset
    }

    #[test]
    fn transparent_wrappers_are_skipped() {
        assert!(is_transparent_wrapper("ManuallyDrop<alloc::string::String>"));
        assert!(is_transparent_wrapper("MaybeUninit<u8>"));
        assert!(!is_transparent_wrapper("alloc::vec::Vec<u8>"));
        assert!(!is_transparent_wrapper("MyManuallyDropLookalike"));
    }

    #[test]
    fn enum_variant_field_structs_are_not_ranked() {
        // A real named type at namespace or unit scope ranks.
        assert!(ranks_as_named_type("Pow10SignificandsTable", false));
        // Variant field-structs sit under the enum type (`parent_is_type`) and
        // repeat its size under a variant name — never a distinct type.
        assert!(!ranks_as_named_type("Panicked", true)); // a coroutine state
        assert!(!ranks_as_named_type("Suspend0", true));
        assert!(!ranks_as_named_type("Consumed", true)); // a tokio Stage variant
        // Transparent wrappers never rank, wherever they sit.
        assert!(!ranks_as_named_type("ManuallyDrop<alloc::string::String>", false));
    }
}
