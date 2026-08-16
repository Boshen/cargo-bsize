//! Attribute the code that inlining hid.
//!
//! Every other view reads the symbol table, which only names functions that
//! survived. A function inlined into all its callers has no symbol, and its
//! bytes are counted against whoever inlined it. DWARF records each inlined
//! instance with the range it occupies and the call site it came from, so this
//! recovers both, in bytes rather than counts.
//!
//! Nested inlines share address ranges — `String::deref` inlining `as_str`
//! inlining `Vec::as_slice` all cover the same instructions — so a range is
//! charged only to its innermost frame.

use std::{
    borrow::Cow,
    cmp::Reverse,
    collections::HashMap,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result};
use object::{Object, ObjectSection};
use serde::Serialize;

use crate::symbols::demangle;

#[derive(Debug, Serialize)]
pub struct InlineReport {
    /// Bytes charged to an inlined instance rather than to a named symbol.
    pub bytes: u64,

    /// Inlined instances found.
    pub instances: usize,

    /// Instances whose extent DWARF did not record, and which contribute no
    /// bytes here.
    pub without_range: usize,

    pub functions: Vec<InlinedFunction>,
}

#[derive(Debug, Serialize)]
pub struct InlinedFunction {
    pub name: String,

    /// Bytes this function occupies across every site it was inlined into,
    /// counting only instructions not attributed to a deeper inline.
    pub bytes: u64,

    pub sites: usize,
}

/// Find every inlined instance in `path` and total them by inlined function.
///
/// # Errors
///
/// Errors when the debug info cannot be produced, read, or parsed.
pub fn analyze(path: &Path, target_dir: &Path, limit: usize) -> Result<InlineReport> {
    let debug = debug_object(path, target_dir)?;
    let data =
        std::fs::read(&debug).with_context(|| format!("failed to read {}", debug.display()))?;
    let file = object::File::parse(&*data)
        .with_context(|| format!("failed to parse {}", debug.display()))?;

    let load = |id: gimli::SectionId| -> Result<Cow<'_, [u8]>, gimli::Error> {
        Ok(file
            .section_by_name(id.name())
            .and_then(|section| section.uncompressed_data().ok())
            .unwrap_or(Cow::Borrowed(&[])))
    };

    let endian = if file.is_little_endian() {
        gimli::RunTimeEndian::Little
    } else {
        gimli::RunTimeEndian::Big
    };
    let sections = gimli::DwarfSections::load(load).context("failed to load DWARF sections")?;
    let dwarf = sections.borrow(|section| gimli::EndianSlice::new(section, endian));

    let mut totals: HashMap<String, (u64, usize)> = HashMap::new();
    let mut instances = 0usize;
    let mut without_range = 0usize;

    let mut units = dwarf.units();
    while let Some(header) = units.next().context("failed to read a DWARF unit")? {
        let unit = dwarf.unit(header).context("failed to parse a DWARF unit")?;
        let mut tree = unit.entries_tree(None).context("failed to walk DWARF entries")?;
        let root = tree.root().context("failed to read a DWARF root entry")?;

        walk(&dwarf, &unit, root, &mut totals, &mut instances, &mut without_range)?;
    }

    let bytes = totals.values().map(|(bytes, _)| bytes).sum();
    let mut functions: Vec<InlinedFunction> = totals
        .into_iter()
        .map(|(name, (bytes, sites))| InlinedFunction { name, bytes, sites })
        .collect();
    functions.sort_by_key(|function| Reverse(function.bytes));
    functions.truncate(limit);

    Ok(InlineReport { bytes, instances, without_range, functions })
}

/// Walk the DIE tree, charging each inlined range to its innermost frame.
///
/// Returns the bytes this subtree's inlined children cover, so a parent can
/// subtract them from its own extent.
fn walk<R: gimli::Reader>(
    dwarf: &gimli::Dwarf<R>,
    unit: &gimli::Unit<R>,
    node: gimli::EntriesTreeNode<'_, '_, R>,
    totals: &mut HashMap<String, (u64, usize)>,
    instances: &mut usize,
    without_range: &mut usize,
) -> Result<u64> {
    let entry = node.entry();
    let inlined = entry.tag() == gimli::DW_TAG_inlined_subroutine;

    let extent = if inlined {
        *instances += 1;
        let extent = extent(dwarf, unit, entry)?;
        if extent == 0 {
            *without_range += 1;
        }
        extent
    } else {
        0
    };

    let name = if inlined { inlined_name(dwarf, unit, entry)? } else { None };

    let mut children = node.children();
    let mut nested = 0u64;
    while let Some(child) = children.next().context("failed to read a DWARF child entry")? {
        nested += walk(dwarf, unit, child, totals, instances, without_range)?;
    }

    if let Some(name) = name {
        // Instructions the children claim belong to them, not to this frame.
        let own = extent.saturating_sub(nested);
        let entry = totals.entry(name).or_default();
        entry.0 += own;
        entry.1 += 1;
    }

    Ok(if inlined { extent } else { nested })
}

/// Total bytes an entry covers, from either a contiguous pair or a range list.
fn extent<R: gimli::Reader>(
    dwarf: &gimli::Dwarf<R>,
    unit: &gimli::Unit<R>,
    entry: &gimli::DebuggingInformationEntry<R>,
) -> Result<u64> {
    let mut ranges = dwarf.die_ranges(unit, entry).context("failed to read DWARF ranges")?;
    let mut total = 0;

    while let Some(range) = ranges.next().context("failed to read a DWARF range")? {
        total += range.end.saturating_sub(range.begin);
    }

    Ok(total)
}

/// The name of the function that was inlined, followed through
/// `DW_AT_abstract_origin` to the declaration that carries it.
fn inlined_name<R: gimli::Reader>(
    dwarf: &gimli::Dwarf<R>,
    unit: &gimli::Unit<R>,
    entry: &gimli::DebuggingInformationEntry<R>,
) -> Result<Option<String>> {
    let Some(origin) = entry.attr_value(gimli::DW_AT_abstract_origin) else {
        return Ok(None);
    };
    let gimli::AttributeValue::UnitRef(offset) = origin else { return Ok(None) };

    let origin = unit.entry(offset).context("failed to resolve an abstract origin")?;
    for attribute in [gimli::DW_AT_linkage_name, gimli::DW_AT_name] {
        if let Some(value) = origin.attr_value(attribute)
            && let Ok(name) = dwarf.attr_string(unit, value)
            && let Ok(name) = name.to_string_lossy()
        {
            return Ok(Some(demangle(&name)));
        }
    }

    Ok(None)
}

/// Where the DWARF lives.
///
/// Mach-O leaves it in the object files and only records a pointer to them, so
/// `dsymutil` has to gather it first — a link of existing debug info, not a
/// recompile. Elsewhere it is already in the binary.
fn debug_object(path: &Path, target_dir: &Path) -> Result<PathBuf> {
    if !cfg!(target_os = "macos") {
        return Ok(path.to_owned());
    }

    let name = path.file_name().unwrap_or_default();
    let bundle = target_dir.join("bsize.dSYM");
    let status = Command::new("dsymutil")
        .arg(path)
        .arg("-o")
        .arg(&bundle)
        .status()
        .context("failed to run `dsymutil`")?;

    if !status.success() {
        anyhow::bail!("`dsymutil` failed with {status}");
    }

    Ok(bundle.join("Contents").join("Resources").join("DWARF").join(name))
}
