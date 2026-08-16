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
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};
use object::{BinaryFormat, Object, ObjectSection};
use serde::Serialize;

use crate::{name::demangle, symbols::Total};

#[derive(Debug, Serialize)]
pub struct InlineReport {
    /// Bytes charged to an inlined instance rather than to a named symbol.
    pub bytes: u64,

    /// Inlined instances found.
    pub instances: usize,

    /// Instances whose extent DWARF did not record, contributing no bytes here.
    pub without_range: usize,

    pub functions: Vec<InlinedFunction>,

    /// The source lines those instances were inlined at. The only view in the
    /// report that names a line of code rather than a symbol.
    pub call_sites: Vec<CallSite>,
}

#[derive(Debug, Serialize)]
pub struct InlinedFunction {
    pub name: String,

    /// Bytes this function occupies across every site it was inlined into,
    /// counting only instructions not attributed to a deeper inline.
    pub bytes: u64,

    pub sites: usize,
}

#[derive(Debug, Serialize)]
pub struct CallSite {
    pub file: String,
    pub line: u64,

    /// Bytes of inlined code this line pulled in.
    pub bytes: u64,

    pub instances: usize,
}

/// What the DIE walk accumulates.
#[derive(Default)]
struct Tally {
    functions: HashMap<String, Total>,
    sites: HashMap<(String, u64), Total>,
    instances: usize,
    without_range: usize,
}

/// Find every inlined instance in the binary at `path`, whose object format is
/// `format`, totalled by inlined function and by the source line that inlined
/// it.
///
/// # Errors
///
/// Errors when the debug info cannot be produced, read, or parsed.
pub fn analyze(
    path: &Path,
    format: BinaryFormat,
    target_dir: &Path,
    limit: usize,
) -> Result<InlineReport> {
    let debug = debug_object(path, format, target_dir)?;
    let data = fs::read(&debug).with_context(|| format!("failed to read {}", debug.display()))?;
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

    let mut tally = Tally::default();
    let mut units = dwarf.units();
    while let Some(header) = units.next().context("failed to read a DWARF unit")? {
        let unit = dwarf.unit(header).context("failed to parse a DWARF unit")?;
        let unit = unit.unit_ref(&dwarf);
        let mut tree = unit.entries_tree(None).context("failed to walk DWARF entries")?;
        let root = tree.root().context("failed to read a DWARF root entry")?;

        walk(unit, root, &mut tally)?;
    }

    let bytes = tally.functions.values().map(|total| total.bytes).sum();

    let mut functions: Vec<InlinedFunction> = tally
        .functions
        .into_iter()
        .map(|(name, total)| InlinedFunction { name, bytes: total.bytes, sites: total.count })
        .collect();
    functions.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
    functions.truncate(limit);

    let mut call_sites: Vec<CallSite> = tally
        .sites
        .into_iter()
        .map(|((file, line), total)| CallSite {
            file,
            line,
            bytes: total.bytes,
            instances: total.count,
        })
        .collect();
    call_sites.sort_by(|a, b| {
        b.bytes.cmp(&a.bytes).then_with(|| (&a.file, a.line).cmp(&(&b.file, b.line)))
    });
    call_sites.truncate(limit);

    Ok(InlineReport {
        bytes,
        instances: tally.instances,
        without_range: tally.without_range,
        functions,
        call_sites,
    })
}

/// Walk the DIE tree, charging each inlined range to its innermost frame.
///
/// Returns the bytes this subtree's inlined children cover, so a parent can
/// subtract them from its own extent.
fn walk<R: gimli::Reader>(
    unit: gimli::UnitRef<'_, R>,
    node: gimli::EntriesTreeNode<'_, '_, R>,
    tally: &mut Tally,
) -> Result<u64> {
    let entry = node.entry();
    let inlined = entry.tag() == gimli::DW_TAG_inlined_subroutine;

    let mut extent = 0;
    let mut name = None;
    let mut site = None;
    if inlined {
        tally.instances += 1;
        extent = extent_of(unit, entry)?;
        if extent == 0 {
            tally.without_range += 1;
        }
        name = inlined_name(unit, entry)?;
        site = call_site(unit, entry);
    }

    let mut children = node.children();
    let mut nested = 0u64;
    while let Some(child) = children.next().context("failed to read a DWARF child entry")? {
        nested += walk(unit, child, tally)?;
    }

    // Instructions the children claim belong to them, not to this frame.
    let own = extent.saturating_sub(nested);

    if let Some(name) = name {
        tally.functions.entry(name).or_default().add(own);
    }
    if let Some(site) = site {
        tally.sites.entry(site).or_default().add(own);
    }

    Ok(if inlined { extent } else { nested })
}

/// Total bytes an entry covers, from either a contiguous pair or a range list.
fn extent_of<R: gimli::Reader>(
    unit: gimli::UnitRef<'_, R>,
    entry: &gimli::DebuggingInformationEntry<R>,
) -> Result<u64> {
    let mut ranges = unit.die_ranges(entry).context("failed to read DWARF ranges")?;
    let mut total = 0;

    while let Some(range) = ranges.next().context("failed to read a DWARF range")? {
        total += range.end.saturating_sub(range.begin);
    }

    Ok(total)
}

/// The name of the function that was inlined, followed through
/// `DW_AT_abstract_origin` to the declaration that carries it.
fn inlined_name<R: gimli::Reader>(
    unit: gimli::UnitRef<'_, R>,
    entry: &gimli::DebuggingInformationEntry<R>,
) -> Result<Option<String>> {
    let Some(gimli::AttributeValue::UnitRef(offset)) =
        entry.attr_value(gimli::DW_AT_abstract_origin)
    else {
        return Ok(None);
    };

    let origin = unit.entry(offset).context("failed to resolve an abstract origin")?;
    let name = [gimli::DW_AT_linkage_name, gimli::DW_AT_name]
        .into_iter()
        .find_map(|attribute| attr_string(unit, origin.attr_value(attribute)?));

    Ok(name.map(|name| demangle(&name)))
}

/// The source line the call was written on, from `DW_AT_call_file` and
/// `DW_AT_call_line`.
fn call_site<R: gimli::Reader>(
    unit: gimli::UnitRef<'_, R>,
    entry: &gimli::DebuggingInformationEntry<R>,
) -> Option<(String, u64)> {
    let gimli::AttributeValue::FileIndex(index) = entry.attr_value(gimli::DW_AT_call_file)? else {
        return None;
    };
    let line = entry.attr_value(gimli::DW_AT_call_line)?.udata_value()?;

    let header = unit.line_program.as_ref()?.header();
    let file = header.file(index)?;
    let name = attr_string(unit, file.path_name())?;

    // An absolute path is already complete; otherwise the file's directory
    // entry supplies the rest.
    let path = match file.directory(header).and_then(|directory| attr_string(unit, directory)) {
        Some(directory) if !name.starts_with('/') => format!("{directory}/{name}"),
        _ => name,
    };

    Some((normalize(&path), line))
}

/// Read a string attribute, from whichever string section it points into.
fn attr_string<R: gimli::Reader>(
    unit: gimli::UnitRef<'_, R>,
    value: gimli::AttributeValue<R>,
) -> Option<String> {
    let string = unit.attr_string(value).ok()?;
    Some(string.to_string_lossy().ok()?.into_owned())
}

/// Collapse the several ways compile units spell one file.
///
/// The same line of `core` arrives as an absolute rustup path, as
/// `/rustc/<hash>/library/…`, and as a bare `library/…`, which splits one
/// source line across three rows and understates every one of them.
pub(crate) fn normalize(path: &str) -> String {
    if let Some((_, rest)) = path.split_once("/registry/src/")
        && let Some((_, within)) = rest.split_once('/')
    {
        return within.to_owned();
    }

    // The crates vendored into std are spelled `/rust/deps/<crate>-<version>/…`.
    if let Some((_, within)) = path.split_once("/rust/deps/") {
        return within.to_owned();
    }

    match path.find("/library/") {
        Some(index) => path[index + 1..].to_owned(),
        None => path.to_owned(),
    }
}

/// Where the DWARF lives.
///
/// Mach-O leaves it in the object files and only records a pointer to them, so
/// `dsymutil` has to gather it first — a link of existing debug info, not a
/// recompile. Elsewhere it is already in the binary.
fn debug_object(path: &Path, format: BinaryFormat, target_dir: &Path) -> Result<PathBuf> {
    if format != BinaryFormat::MachO {
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
        bail!("`dsymutil` failed with {status}");
    }

    Ok(bundle.join("Contents").join("Resources").join("DWARF").join(name))
}
