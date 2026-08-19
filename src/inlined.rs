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

use std::{borrow::Cow, path::Path};

use anyhow::{Context, Result};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    dwarf::{attr_string, file_path, with_dwarf},
    name::{defining_crate, demangle},
    symbols::Total,
};

#[derive(Debug)]
pub struct InlineReport {
    /// Bytes charged to an inlined instance rather than to a named symbol.
    pub bytes: u64,

    /// Inlined instances found.
    pub instances: usize,

    /// Instances whose extent DWARF did not record, contributing no bytes here.
    pub without_range: usize,

    pub functions: Vec<InlinedFunction>,

    /// The functions holding the most inlined code, by the bytes of instances
    /// inside their ranges — how much of each body is other functions' code.
    pub callers: Vec<InlinedCaller>,

    /// The same bytes credited to the crate defining the inlined function —
    /// the footprint of crates whose code vanishes into their callers.
    pub origin_crates: Vec<InlinedCrate>,

    /// The source lines those instances were inlined at. The only view in the
    /// report that names a line of code rather than a symbol.
    pub call_sites: Vec<CallSite>,

    /// The same, restricted to lines in this workspace — the code you can edit,
    /// rather than the std and dependency lines that top the full list.
    pub workspace_call_sites: Vec<CallSite>,
}

#[derive(Debug, Clone)]
pub struct InlinedFunction {
    pub name: String,

    /// Bytes this function occupies across every site it was inlined into,
    /// counting only instructions not attributed to a deeper inline.
    pub bytes: u64,

    pub sites: usize,

    /// Where it is defined, `file:line`.
    pub defined_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct InlinedCaller {
    pub name: String,

    /// Bytes of inlined instances inside this function's ranges.
    pub bytes: u64,

    pub instances: usize,

    /// The function's whole extent, from the same DWARF ranges — `bytes` of it
    /// is inlined code.
    pub total: u64,
}

#[derive(Debug, Clone)]
pub struct InlinedCrate {
    pub name: String,
    pub bytes: u64,
    pub instances: usize,
}

/// What the walk produced: the report, plus the untruncated per-function tally
/// for analyses that read the whole list rather than the largest rows.
pub struct Inlines {
    pub report: InlineReport,
    pub(crate) functions: Vec<InlinedFunction>,
}

#[derive(Debug, Clone)]
pub struct CallSite {
    pub file: String,
    pub line: u64,

    /// Bytes of inlined code this line pulled in.
    pub bytes: u64,

    pub instances: usize,

    /// The line's source text, for lines in this workspace.
    pub snippet: Option<String>,
}

/// What the DIE walk accumulates.
#[derive(Default)]
struct Tally {
    functions: FxHashMap<String, Total>,
    sites: FxHashMap<(String, u64), Total>,

    /// Inlined bytes per enclosing function, and that function's own extent.
    callers: FxHashMap<String, (Total, u64)>,

    /// Inlined bytes per crate defining the inlined function.
    origins: FxHashMap<String, Total>,

    /// Call-site files that live in this workspace.
    workspace: FxHashSet<String>,

    instances: usize,
    without_range: usize,
}

/// Find every inlined instance in the DWARF at `debug`, totalled by inlined
/// function and by the source line that inlined it. `workspace` is the root the
/// editable-lines companion is kept to.
///
/// # Errors
///
/// Errors when the debug info cannot be read or parsed.
pub fn analyze(debug: &Path, workspace: &Path, limit: usize) -> Result<Inlines> {
    with_dwarf(debug, |dwarf| {
        let mut tally = Tally::default();
        let mut units = dwarf.units();
        while let Some(header) = units.next().context("failed to read a DWARF unit")? {
            let unit = dwarf.unit(header).context("failed to parse a DWARF unit")?;
            let unit = unit.unit_ref(dwarf);
            let mut tree = unit.entries_tree(None).context("failed to walk DWARF entries")?;
            let root = tree.root().context("failed to read a DWARF root entry")?;

            walk(unit, root, &mut tally, workspace, None)?;
        }

        let bytes = tally.functions.values().map(|total| total.bytes).sum();

        let mut functions: Vec<InlinedFunction> = tally
            .functions
            .into_iter()
            .map(|(name, total)| InlinedFunction {
                name,
                bytes: total.bytes,
                sites: total.count,
                defined_at: None,
            })
            .collect();
        functions.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
        // The whole list, before the report keeps only the largest: the names
        // carry their turbofish, which downstream analyses key on.
        let all = functions.clone();
        functions.truncate(limit);

        let mut callers: Vec<InlinedCaller> = tally
            .callers
            .into_iter()
            .filter(|(_, (inlined, _))| inlined.bytes > 0)
            .map(|(name, (inlined, total))| InlinedCaller {
                name,
                bytes: inlined.bytes,
                instances: inlined.count,
                total,
            })
            .collect();
        callers.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
        callers.truncate(limit);

        let mut origin_crates: Vec<InlinedCrate> = tally
            .origins
            .into_iter()
            .filter(|(_, total)| total.bytes > 0)
            .map(|(name, total)| InlinedCrate { name, bytes: total.bytes, instances: total.count })
            .collect();
        origin_crates.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
        origin_crates.truncate(limit);

        let mut call_sites: Vec<CallSite> = tally
            .sites
            .into_iter()
            .map(|((file, line), total)| CallSite {
                file,
                line,
                bytes: total.bytes,
                instances: total.count,
                snippet: None,
            })
            .collect();
        call_sites.sort_by(|a, b| {
            b.bytes.cmp(&a.bytes).then_with(|| (&a.file, a.line).cmp(&(&b.file, b.line)))
        });

        // Same ranking, kept to the editable lines. Taken before the full list
        // is truncated, since the workspace rarely tops it.
        let workspace_call_sites: Vec<CallSite> = call_sites
            .iter()
            .filter(|site| tally.workspace.contains(&site.file))
            .take(limit)
            .cloned()
            .collect();
        call_sites.truncate(limit);

        Ok(Inlines {
            report: InlineReport {
                bytes,
                instances: tally.instances,
                without_range: tally.without_range,
                functions,
                callers,
                origin_crates,
                call_sites,
                workspace_call_sites,
            },
            functions: all,
        })
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
    workspace: &Path,
    caller: Option<&str>,
) -> Result<u64> {
    let entry = node.entry();
    let inlined = entry.tag() == gimli::DW_TAG_inlined_subroutine;

    let (extent, name, site) = if inlined {
        tally.instances += 1;
        let extent = extent_of(unit, entry)?;
        if extent == 0 {
            tally.without_range += 1;
        }
        (extent, origin_name(unit, entry)?, call_site(unit, entry, workspace))
    } else {
        (0, None, None)
    };

    // A subprogram with code becomes the caller its inlined descendants are
    // charged to; anything else inherits the enclosing one.
    let enclosing: Option<(String, u64)> = if entry.tag() == gimli::DW_TAG_subprogram {
        let extent = extent_of(unit, entry)?;
        (extent > 0)
            .then(|| origin_name(unit, entry))
            .transpose()?
            .flatten()
            .map(|(name, _)| (name, extent))
    } else {
        None
    };
    let caller = enclosing.as_ref().map_or(caller, |(name, _)| Some(name));

    let mut children = node.children();
    let mut nested = 0u64;
    while let Some(child) = children.next().context("failed to read a DWARF child entry")? {
        nested += walk(unit, child, tally, workspace, caller)?;
    }

    if let Some((name, extent)) = &enclosing {
        tally.callers.entry(name.clone()).or_default().1 += extent;
    }

    // Instructions the children claim belong to them, not to this frame.
    let own = extent.saturating_sub(nested);

    if let Some((name, mangled)) = name {
        if let Some(caller) = caller {
            tally.callers.entry(caller.to_owned()).or_default().0.add(own);
        }
        if let Some(krate) = defining_crate(&mangled, &name) {
            tally.origins.entry(krate).or_default().add(own);
        }
        tally.functions.entry(name).or_default().add(own);
    }
    if let Some((file, line, in_workspace)) = site {
        if in_workspace {
            tally.workspace.insert(file.clone());
        }
        tally.sites.entry((file, line)).or_default().add(own);
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

/// A subroutine's name — on the entry itself, or through
/// `DW_AT_abstract_origin` to the declaration that carries it. Returns the
/// name as spelled (usually mangled, so the defining crate can be read off)
/// — demangle for display.
fn origin_name<R: gimli::Reader>(
    unit: gimli::UnitRef<'_, R>,
    entry: &gimli::DebuggingInformationEntry<R>,
) -> Result<Option<(String, String)>> {
    let attrs = [gimli::DW_AT_linkage_name, gimli::DW_AT_name];
    let mut name =
        attrs.into_iter().find_map(|attribute| attr_string(unit, entry.attr_value(attribute)?));

    if name.is_none()
        && let Some(gimli::AttributeValue::UnitRef(offset)) =
            entry.attr_value(gimli::DW_AT_abstract_origin)
    {
        let origin = unit.entry(offset).context("failed to resolve an abstract origin")?;
        name = attrs
            .into_iter()
            .find_map(|attribute| attr_string(unit, origin.attr_value(attribute)?));
    }

    Ok(name.map(|mangled| (demangle(&mangled), mangled)))
}

/// The source line the call was written on, from `DW_AT_call_file` and
/// `DW_AT_call_line`, with whether it lives in this workspace.
fn call_site<R: gimli::Reader>(
    unit: gimli::UnitRef<'_, R>,
    entry: &gimli::DebuggingInformationEntry<R>,
    workspace: &Path,
) -> Option<(String, u64, bool)> {
    let gimli::AttributeValue::FileIndex(index) = entry.attr_value(gimli::DW_AT_call_file)? else {
        return None;
    };
    let line = entry.attr_value(gimli::DW_AT_call_line)?.udata_value()?;
    let path = file_path(unit, index)?;

    // Files arrive relative to the unit's compilation directory, which is what
    // separates a workspace path from a dependency's: std reports `/rustc/<hash>`,
    // a dependency its registry checkout, a workspace crate a path under the root.
    let comp_dir: Option<String> =
        unit.comp_dir.as_ref().and_then(|dir| dir.to_string_lossy().ok()).map(Cow::into_owned);

    let (display, in_workspace) = source(&path, comp_dir.as_deref(), workspace);
    Some((display, line, in_workspace))
}

/// A source path's display form, and whether it is in this workspace — the code
/// the reader can edit — rather than std or a dependency.
pub(crate) fn source(path: &str, comp_dir: Option<&str>, workspace: &Path) -> (String, bool) {
    let absolute = match comp_dir {
        Some(dir) if !path.starts_with('/') => Cow::Owned(format!("{dir}/{path}")),
        _ => Cow::Borrowed(path),
    };

    Path::new(absolute.as_ref())
        .strip_prefix(workspace)
        .map_or_else(|_| (normalize(&absolute), false), |rest| (rest.display().to_string(), true))
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

    path.find("/library/").map_or_else(|| path.to_owned(), |index| path[index + 1..].to_owned())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::source;

    /// A path is a workspace path when it resolves, against the unit's compile
    /// directory, to somewhere under the workspace root — not by sniffing for
    /// `/library/` or `/registry/`, which mislabels std files that DWARF spells
    /// relative to a `/rustc/<hash>` directory.
    #[test]
    fn classifies_source_paths_by_the_compile_directory() {
        let workspace = Path::new("/work/space");

        // A workspace file, relative to its crate's compile directory.
        assert_eq!(
            source("src/lib.rs", Some("/work/space/crates/a"), workspace),
            ("crates/a/src/lib.rs".to_owned(), true)
        );

        // A workspace file, relative to a workspace-root compile directory.
        assert_eq!(
            source("crates/a/src/lib.rs", Some("/work/space"), workspace),
            ("crates/a/src/lib.rs".to_owned(), true)
        );

        // std: spelled relative to `/rustc/<hash>`, so it carries no absolute
        // `/library/` marker of its own — the regression this guards.
        assert_eq!(
            source("library/std/src/alloc.rs", Some("/rustc/abc123"), workspace),
            ("library/std/src/alloc.rs".to_owned(), false)
        );

        // A dependency, resolved into its registry checkout.
        assert_eq!(
            source("src/de.rs", Some("/home/u/.cargo/registry/src/index-1/serde-1.0.0"), workspace),
            ("serde-1.0.0/src/de.rs".to_owned(), false)
        );
    }
}
