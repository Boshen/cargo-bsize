//! Report where a binary's file size goes, by section and by category.
//!
//! Sizes are file ranges, never virtual sizes — Mach-O's `__PAGEZERO` claims
//! 4 GB of address space and no file bytes. Sections do not cover a whole file
//! either, so sectionless segments are reported too; `__LINKEDIT` is routinely
//! a quarter of a Mach-O binary. ELF program headers always overlap sections,
//! so nothing is double counted there.

use std::{cmp::Reverse, collections::BTreeMap, fmt, fs, path::Path};

use anyhow::{Context, Result};
use object::{Object, ObjectSection, ObjectSegment};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct BinaryReport {
    pub path: String,
    pub format: String,
    pub total: u64,
    pub accounted: u64,

    /// Headers, alignment padding, and code signature.
    pub other: u64,

    /// `total` without the symbol and debug bytes this build keeps but a
    /// shipped release strips.
    pub shipped: u64,

    pub categories: Vec<CategorySize>,
    pub sections: Vec<SectionSize>,
}

#[derive(Debug, Serialize)]
pub struct CategorySize {
    pub category: Category,
    pub size: u64,
}

#[derive(Debug, Serialize)]
pub struct SectionSize {
    pub name: String,
    pub category: Category,
    pub size: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Category {
    Code,
    ReadOnlyData,
    Unwind,
    Data,
    Symbols,
    Debug,
    Uncategorized,
}

impl Category {
    /// Names carry more meaning here than `SectionKind`, which lumps unwind
    /// tables in with read-only data.
    fn of(name: &str) -> Self {
        // `.rodata.foo` and `.debug_info.dwo` categorize as `.rodata` / `.debug_info`.
        let stem = match name.match_indices('.').nth(1) {
            Some((index, _)) => &name[..index],
            None => name,
        };

        match stem {
            "__text" | "__text_startup" | "__stubs" | "__stub_helper" | ".text" | ".plt"
            | ".init" | ".fini" => Self::Code,
            "__const" | "__cstring" | "__literal4" | "__literal8" | "__literal16" | ".rodata"
            | ".rdata" => Self::ReadOnlyData,
            "__eh_frame" | "__unwind_info" | "__gcc_except_tab" | ".eh_frame" | ".eh_frame_hdr"
            | ".gcc_except_table" => Self::Unwind,
            "__data" | "__got" | "__la_symbol_ptr" | "__mod_init_func" | "__bss" | "__common"
            | ".data" | ".bss" | ".got" | ".got_plt" | ".init_array" => Self::Data,
            "__LINKEDIT" | ".symtab" | ".strtab" | ".dynsym" | ".dynstr" => Self::Symbols,
            _ if stem.starts_with("__thread_") => Self::Data,
            _ if stem.starts_with("__debug_") || stem.starts_with(".debug_") => Self::Debug,
            _ => Self::Uncategorized,
        }
    }

    /// Whether a shipped release build would strip these bytes.
    const fn is_stripped(self) -> bool {
        matches!(self, Self::Symbols | Self::Debug)
    }
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Code => "code",
            Self::ReadOnlyData => "read-only data",
            Self::Unwind => "unwind",
            Self::Data => "data",
            Self::Symbols => "symbols",
            Self::Debug => "debug",
            Self::Uncategorized => "uncategorized",
        })
    }
}

/// Break `path` down into sections, largest first.
///
/// # Errors
///
/// Errors when the file cannot be read or is not a recognized object file.
pub fn analyze(path: &Path) -> Result<BinaryReport> {
    let data = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let file = object::File::parse(&*data)
        .with_context(|| format!("failed to parse {}", path.display()))?;

    let mut sections: Vec<SectionSize> = file
        .sections()
        .filter_map(|section| {
            let (_, size) = section.file_range().filter(|&(_, size)| size > 0)?;
            let name = section.name().ok()?;
            let category = Category::of(name);

            // Mach-O names are unique only within a segment: `__const` exists in
            // both `__TEXT` and `__DATA_CONST`. ELF returns `None`.
            let name = match section.segment_name().ok().flatten() {
                Some(segment) => format!("{segment},{name}"),
                None => name.to_owned(),
            };

            Some(SectionSize { name, category, size })
        })
        .collect();

    sections.extend(sectionless_segments(&file));
    sections.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.name.cmp(&b.name)));

    let total = data.len() as u64;
    let accounted: u64 = sections.iter().map(|section| section.size).sum();
    let stripped: u64 = sections
        .iter()
        .filter(|section| section.category.is_stripped())
        .map(|section| section.size)
        .sum();

    Ok(BinaryReport {
        path: path.display().to_string(),
        format: format!("{:?}", file.format()).to_lowercase(),
        total,
        accounted,
        other: total.saturating_sub(accounted),
        shipped: total.saturating_sub(stripped),
        categories: categories(&sections),
        sections,
    })
}

/// Segments holding file bytes that no section claims — `__LINKEDIT` on Mach-O.
fn sectionless_segments(file: &object::File<'_>) -> Vec<SectionSize> {
    let claimed: Vec<(u64, u64)> =
        file.sections().filter_map(|section| section.file_range()).collect();

    file.segments()
        .filter_map(|segment| {
            let (start, size) = segment.file_range();
            let covered = claimed.iter().any(|&(section_start, section_size)| {
                section_start < start + size && start < section_start + section_size
            });

            if size == 0 || covered {
                return None;
            }

            let name = segment.name().ok()??.to_owned();
            Some(SectionSize { category: Category::of(&name), name, size })
        })
        .collect()
}

fn categories(sections: &[SectionSize]) -> Vec<CategorySize> {
    let mut totals: BTreeMap<Category, u64> = BTreeMap::new();
    for section in sections {
        *totals.entry(section.category).or_default() += section.size;
    }

    let mut categories: Vec<CategorySize> =
        totals.into_iter().map(|(category, size)| CategorySize { category, size }).collect();
    categories.sort_by_key(|entry| Reverse(entry.size));
    categories
}
