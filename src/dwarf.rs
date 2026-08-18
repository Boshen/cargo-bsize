//! Locate and load the DWARF debug info shared by the inlined-code and type
//! analyses.
//!
//! Mach-O leaves DWARF in the object files and records only a pointer, so
//! `dsymutil` gathers it into a `.dSYM` first — a link of existing debug info,
//! not a recompile. Elsewhere it is already in the binary. Both consumers read
//! the same debug object, so it is produced once and passed in.

use std::{borrow::Cow, path::Path, path::PathBuf, process::Command};

use anyhow::{Context, Result, bail};
use object::{BinaryFormat, Object, ObjectSection};

/// The reader the loaded DWARF is parsed with.
pub(crate) type Reader<'data> = gimli::EndianSlice<'data, gimli::RunTimeEndian>;

/// What a compile unit's header says about where its code came from.
#[derive(Debug, Clone)]
pub struct UnitInfo {
    /// `DW_AT_name`: for a Rust unit, the crate root's source path, then
    /// `/@/<crate>.<hash>-cgu.N`.
    pub name: String,

    /// `DW_AT_comp_dir`, which completes a relative name.
    pub comp_dir: Option<String>,

    /// Whether `DW_AT_language` is Rust (or absent); a C or assembly object
    /// linked in says otherwise.
    pub rust: bool,
}

/// The address range of one out-of-line function, and the unit it belongs to
/// (an index into the units read in `.debug_info` order).
#[derive(Debug, Clone, Copy)]
pub struct FunctionRange {
    pub unit: usize,
    pub begin: u64,
    pub end: u64,
}

/// Where a function is defined, as the report shows it: a workspace-relative
/// or normalized path, and a line.
#[derive(Debug, Clone)]
pub struct Site {
    pub file: String,
    pub line: u64,
    pub workspace: bool,
}

impl Site {
    /// `file:line`.
    #[must_use]
    pub fn display(&self) -> String {
        format!("{}:{}", self.file, self.line)
    }
}

/// The path of file `index` in a unit's line-program table: the file's
/// directory entry joined with its name, unless the name is already absolute.
/// Relative to the unit's compilation directory, which the caller supplies.
pub(crate) fn file_path<R: gimli::Reader>(
    unit: gimli::UnitRef<'_, R>,
    index: u64,
) -> Option<String> {
    let header = unit.line_program.as_ref()?.header();
    let file = header.file(index)?;
    let name = attr_string(unit, file.path_name())?;

    match file.directory(header).and_then(|directory| attr_string(unit, directory)) {
        Some(directory) if !name.starts_with('/') => Some(format!("{directory}/{name}")),
        _ => Some(name),
    }
}

/// Read a string attribute, from whichever string section it points into.
pub(crate) fn attr_string<R: gimli::Reader>(
    unit: gimli::UnitRef<'_, R>,
    value: gimli::AttributeValue<R>,
) -> Option<String> {
    let string = unit.attr_string(value).ok()?;
    Some(string.to_string_lossy().ok()?.into_owned())
}

/// The debug object to read DWARF from — the binary itself, or the `.dSYM`
/// `dsymutil` gathers on macOS.
///
/// # Errors
///
/// Errors when `dsymutil` cannot be run or fails.
pub(crate) fn debug_object(
    executable: &Path,
    format: BinaryFormat,
    target_dir: &Path,
) -> Result<PathBuf> {
    if format != BinaryFormat::MachO {
        return Ok(executable.to_owned());
    }

    let name = executable.file_name().unwrap_or_default();
    let bundle = target_dir.join("bsize.dSYM");
    let status = Command::new("dsymutil")
        .arg(executable)
        .arg("-o")
        .arg(&bundle)
        .status()
        .context("failed to run `dsymutil`")?;

    if !status.success() {
        bail!("`dsymutil` failed with {status}");
    }

    Ok(bundle.join("Contents").join("Resources").join("DWARF").join(name))
}

/// Read and parse the DWARF at `debug`, then hand it to `f`.
///
/// The parsed `Dwarf` borrows a chain of stack-local buffers, so it cannot be
/// returned; callers do their whole walk inside the closure and return owned
/// results.
///
/// # Errors
///
/// Errors when the debug object cannot be read, parsed, or its DWARF loaded.
pub(crate) fn with_dwarf<T>(
    debug: &Path,
    f: impl for<'data> FnOnce(&gimli::Dwarf<Reader<'data>>) -> Result<T>,
) -> Result<T> {
    let data =
        std::fs::read(debug).with_context(|| format!("failed to read {}", debug.display()))?;
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

    f(&dwarf)
}
