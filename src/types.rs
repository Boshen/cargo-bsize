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

use crate::{
    dwarf::{FunctionRange, Site, UnitInfo, file_path, with_dwarf},
    inlined::source,
    name::demangle,
};

#[derive(Debug, Serialize)]
pub struct TypeReport {
    /// The largest named types, by declared byte size.
    pub largest: Vec<NamedType>,
}

#[derive(Debug, Serialize)]
pub struct NamedType {
    pub name: String,
    pub size: u64,

    /// Bytes the fields account for, and the padding between them, for a
    /// struct whose every field has a known size.
    pub fields: Option<u64>,
    pub padding: Option<u64>,

    /// An enum's variants by their fields' bytes, largest first (the largest
    /// few), and how many there are.
    pub variants: Vec<Variant>,
    pub variant_count: usize,

    /// What boxing the largest variant would save on every value: the gap to
    /// the next variant, less the box's pointer.
    pub boxing_saves: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct Variant {
    pub name: String,
    pub bytes: u64,
}

/// How many of an enum's variants the report names.
const VARIANTS: usize = 3;

/// What the walk produced: the report, plus a map from a static's demangled name
/// to its exact byte size, for the symbol view to size read-only data exactly,
/// and — since one walk over the DIEs is expensive and this is the one that
/// visits them all — the compile units and every out-of-line function's address
/// range, for the provenance view.
pub struct Types {
    pub report: TypeReport,
    pub static_sizes: FxHashMap<String, u64>,
    pub units: Vec<UnitInfo>,
    pub functions: Vec<FunctionRange>,

    /// Where each function is defined, by demangled name — the
    /// `DW_AT_decl_file`/`decl_line` of its subprogram — for the views that
    /// name functions.
    pub sites: FxHashMap<String, Site>,
}

/// What the DIE walk accumulates.
#[derive(Default)]
struct Walk {
    kinds: Kinds,
    variables: Vec<(String, u64)>,

    /// The largest reading of each named type, and where it was read — the
    /// unit's offset and the DIE's, so the largest few can be reopened for
    /// their layout once every type is known.
    largest: FxHashMap<String, (u64, u64, u64)>,
    units: Vec<UnitInfo>,
    functions: Vec<FunctionRange>,
    sites: FxHashMap<String, Site>,

    /// The current unit's file table, resolved as needed: `DW_AT_decl_file`
    /// indexes it, and one file is named by many subprograms.
    files: FxHashMap<u64, Option<(String, bool)>>,
    comp_dir: Option<String>,
}

/// Read the largest types and exact static sizes from the DWARF at `debug`,
/// along with the compile units, function ranges, and definition sites the
/// same walk passes; `workspace` is the workspace root, which classifies the
/// sites.
///
/// # Errors
///
/// Errors when the debug info cannot be read or parsed.
pub fn analyze(debug: &Path, workspace: &Path, limit: usize) -> Result<Types> {
    with_dwarf(debug, |dwarf| {
        // Types reference each other across compilation units — a shared
        // primitive is a `DW_FORM_ref_addr` — so every type is keyed by its
        // global `.debug_info` offset in one pass and resolved in a second.
        let mut walk = Walk::default();
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
            walk.files.clear();
            walk.comp_dir = unit.comp_dir.as_ref().map(|dir| dir.to_string_lossy().into_owned());
            let mut tree = unit.entries_tree(None).context("failed to read the DWARF tree")?;
            let root = tree.root().context("failed to read the DWARF root")?;
            collect(unit, root, base, false, workspace, &mut walk)?;
        }

        let Walk { kinds, variables, largest, units, functions, sites, .. } = walk;
        let mut static_sizes: FxHashMap<String, u64> = FxHashMap::default();
        for (name, ty) in variables {
            if let Some(size) = kinds.size(ty, address_size, 0) {
                static_sizes.entry(name).or_insert(size);
            }
        }

        let mut ranked: Vec<(String, (u64, u64, u64))> = largest.into_iter().collect();
        ranked.sort_by(|(a, (x, ..)), (b, (y, ..))| y.cmp(x).then_with(|| a.cmp(b)));
        ranked.truncate(limit);

        // Only the ranked few are reopened for their layout: fields, padding,
        // and variants — the `-Zprint-type-sizes` reading, on stable.
        let largest = ranked
            .into_iter()
            .map(|(name, (size, base, offset))| {
                let layout = layout(dwarf, base, offset, &kinds, address_size).unwrap_or_default();
                let mut variants = layout.variants;
                variants.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
                let boxing_saves = boxing_saves(&variants, address_size);
                let variant_count = variants.len();
                variants.truncate(VARIANTS);
                NamedType {
                    name,
                    size,
                    fields: layout.fields,
                    padding: layout.fields.map(|fields| size.saturating_sub(fields)),
                    variants,
                    variant_count,
                    boxing_saves,
                }
            })
            .collect();

        Ok(Types { report: TypeReport { largest }, static_sizes, units, functions, sites })
    })
}

/// What boxing the largest of `variants` (largest first) saves on every value:
/// the enum shrinks to its next variant, or to the box's pointer if that is
/// bigger. `None` when nothing would be saved.
fn boxing_saves(variants: &[Variant], address_size: u64) -> Option<u64> {
    let saves = match variants {
        [first, second, ..] => first.bytes.saturating_sub(second.bytes.max(address_size)),
        [first] => first.bytes.saturating_sub(address_size),
        [] => return None,
    };
    (saves > 0).then_some(saves)
}

/// What reopening a type's DIE says about its layout.
#[derive(Default)]
struct Layout {
    /// The fields' bytes, when every field's size is known and the type is a
    /// struct (an enum's fields live in its variants).
    fields: Option<u64>,
    variants: Vec<Variant>,
}

/// Reopen the DIE at `offset` in the unit at `base` and read its members: a
/// struct's fields, or an enum's variants and each variant's fields.
fn layout<R: gimli::Reader>(
    dwarf: &gimli::Dwarf<R>,
    base: u64,
    offset: u64,
    kinds: &Kinds,
    address_size: u64,
) -> Option<Layout> {
    let header = dwarf
        .debug_info
        .header_from_offset(gimli::DebugInfoOffset(R::Offset::from_u64(base).ok()?))
        .ok()?;
    let unit = dwarf.unit(header).ok()?;
    let unit = unit.unit_ref(dwarf);
    let mut tree = unit
        .entries_tree(Some(gimli::UnitOffset(R::Offset::from_u64(offset - base).ok()?)))
        .ok()?;
    let root = tree.root().ok()?;

    let mut layout = Layout::default();
    let mut fields = Some(0);
    let mut is_enum = false;
    let mut children = root.children();
    while let Ok(Some(child)) = children.next() {
        let entry = child.entry();
        match entry.tag() {
            gimli::DW_TAG_member => {
                let size = type_ref(entry, base).and_then(|ty| kinds.size(ty, address_size, 0));
                fields = match (fields, size) {
                    (Some(total), Some(size)) => Some(total + size),
                    _ => None,
                };
            }
            gimli::DW_TAG_variant_part => {
                is_enum = true;
                let mut variants = child.children();
                while let Ok(Some(variant)) = variants.next() {
                    if variant.entry().tag() != gimli::DW_TAG_variant {
                        continue;
                    }
                    // Each variant holds one member naming it, whose type is
                    // the variant's field struct.
                    let mut members = variant.children();
                    while let Ok(Some(member)) = members.next() {
                        let entry = member.entry();
                        if entry.tag() != gimli::DW_TAG_member {
                            continue;
                        }
                        let name = string(unit, entry, gimli::DW_AT_name).unwrap_or_default();
                        let bytes = type_ref(entry, base)
                            .and_then(|ty| {
                                variant_fields(unit, ty - base, base, kinds, address_size)
                            })
                            .unwrap_or(0);
                        layout.variants.push(Variant { name, bytes });
                    }
                }
            }
            _ => {}
        }
    }

    if !is_enum {
        layout.fields = fields;
    }
    Some(layout)
}

/// The bytes of a variant's fields: the members of its field struct, whose
/// own `DW_AT_byte_size` is the whole enum's.
fn variant_fields<R: gimli::Reader>(
    unit: gimli::UnitRef<'_, R>,
    offset: u64,
    base: u64,
    kinds: &Kinds,
    address_size: u64,
) -> Option<u64> {
    let mut tree =
        unit.entries_tree(Some(gimli::UnitOffset(R::Offset::from_u64(offset).ok()?))).ok()?;
    let root = tree.root().ok()?;
    let mut total = 0;
    let mut members = root.children();
    while let Ok(Some(member)) = members.next() {
        let entry = member.entry();
        if entry.tag() == gimli::DW_TAG_member {
            total += type_ref(entry, base).and_then(|ty| kinds.size(ty, address_size, 0))?;
        }
    }
    Some(total)
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
    workspace: &Path,
    walk: &mut Walk,
) -> Result<()> {
    let entry = node.entry();
    let offset = base + entry.offset().0.into_u64();
    let tag = entry.tag();
    match tag {
        gimli::DW_TAG_compile_unit => {
            let language = entry.attr_value(gimli::DW_AT_language);
            walk.units.push(UnitInfo {
                name: string(unit, entry, gimli::DW_AT_name).unwrap_or_default(),
                comp_dir: string(unit, entry, gimli::DW_AT_comp_dir),
                rust: language.is_none_or(|value| {
                    matches!(value, gimli::AttributeValue::Language(gimli::DW_LANG_Rust))
                }),
            });
        }
        gimli::DW_TAG_structure_type
        | gimli::DW_TAG_enumeration_type
        | gimli::DW_TAG_union_type => {
            if let Some(size) = byte_size(entry) {
                walk.kinds.sizes.insert(offset, size);
                if let Some(name) = string(unit, entry, gimli::DW_AT_name)
                    && ranks_as_named_type(&name, parent_is_type)
                {
                    // One type appears in many units; keep the largest reading,
                    // and where it was read.
                    let slot = walk.largest.entry(name).or_insert((0, base, offset));
                    if size > slot.0 {
                        *slot = (size, base, offset);
                    }
                }
            }
        }
        gimli::DW_TAG_base_type => {
            if let Some(size) = byte_size(entry) {
                walk.kinds.sizes.insert(offset, size);
            }
        }
        gimli::DW_TAG_array_type => {
            if let (Some(element), Some(count)) = (type_ref(entry, base), array_count(unit, entry))
            {
                walk.kinds.arrays.insert(offset, (element, count));
            }
        }
        gimli::DW_TAG_typedef
        | gimli::DW_TAG_const_type
        | gimli::DW_TAG_volatile_type
        | gimli::DW_TAG_restrict_type
        | gimli::DW_TAG_atomic_type => {
            if let Some(target) = type_ref(entry, base) {
                walk.kinds.aliases.insert(offset, target);
            }
        }
        gimli::DW_TAG_pointer_type | gimli::DW_TAG_reference_type => {
            walk.kinds.pointers.insert(offset);
        }
        gimli::DW_TAG_variable => {
            if let (Some(name), Some(ty)) =
                (string(unit, entry, gimli::DW_AT_linkage_name), type_ref(entry, base))
            {
                walk.variables.push((demangle(&name), ty));
            }
        }
        gimli::DW_TAG_subprogram => {
            // An out-of-line function's code, charged to the unit that emitted
            // it. Declarations and inlined-only origins carry no ranges.
            if let Some(index) = walk.units.len().checked_sub(1)
                && let Ok(mut ranges) = unit.die_ranges(entry)
            {
                while let Ok(Some(range)) = ranges.next() {
                    if range.end > range.begin {
                        walk.functions.push(FunctionRange {
                            unit: index,
                            begin: range.begin,
                            end: range.end,
                        });
                    }
                }
            }

            // Where it is defined, by the name the symbol views use. Inlined
            // functions have this too, on their abstract origins.
            if let Some(linkage) = string(unit, entry, gimli::DW_AT_linkage_name)
                && let Some(gimli::AttributeValue::FileIndex(file)) =
                    entry.attr_value(gimli::DW_AT_decl_file)
                && let Some(line) =
                    entry.attr_value(gimli::DW_AT_decl_line).and_then(|value| value.udata_value())
            {
                let comp_dir = walk.comp_dir.as_deref();
                let resolved = walk.files.entry(file).or_insert_with(|| {
                    file_path(unit, file).map(|path| source(&path, comp_dir, workspace))
                });
                if let Some((file, workspace)) = resolved {
                    walk.sites.entry(demangle(&linkage)).or_insert_with(|| Site {
                        file: file.clone(),
                        line,
                        workspace: *workspace,
                    });
                }
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
        collect(unit, child, base, this_is_type, workspace, walk)?;
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
    use super::{Kinds, Variant, boxing_saves, is_transparent_wrapper, ranks_as_named_type};

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
    fn boxing_the_largest_variant_saves_the_gap_to_the_next() {
        let variant = |name: &str, bytes: u64| Variant { name: name.to_owned(), bytes };
        // Big 96 over Rect 40: every value shrinks by 56.
        assert_eq!(boxing_saves(&[variant("Big", 96), variant("Rect", 40)], 8), Some(56));
        // A lone or unit-only payload shrinks to the pointer.
        assert_eq!(boxing_saves(&[variant("Some", 720), variant("None", 0)], 8), Some(712));
        assert_eq!(boxing_saves(&[variant("Only", 4)], 8), None);
        assert_eq!(boxing_saves(&[], 8), None);
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
