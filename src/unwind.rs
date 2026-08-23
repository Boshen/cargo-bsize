//! Attribute exception-frame and cleanup-table bytes to linked functions.
//!
//! An `.eh_frame` FDE names the address range it describes, so its encoded
//! record can be joined to the final symbol containing that address. When the
//! FDE carries an LSDA pointer, adjacent pointers delimit the corresponding
//! record in `.gcc_except_table`. CIEs, compact-unwind indexes, and records
//! whose code has no final symbol deliberately remain unattributed.

use gimli::{BaseAddresses, CieOrFde, EhFrame, Pointer, ReaderOffset, UnwindSection};
use object::{Object, ObjectSection};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    name::{defining_crate, demangle},
    sections::Category,
    symbols::{Sized, sized_in},
};

#[derive(Debug)]
pub struct UnwindReport {
    /// File bytes in `.eh_frame` / `__eh_frame`.
    pub frame_bytes: u64,

    /// File bytes in `.gcc_except_table` / `__gcc_except_tab`.
    pub exception_bytes: u64,

    /// FDE and LSDA bytes joined to final named functions.
    pub attributed_bytes: u64,

    /// FDEs parsed, including ones whose function has no final symbol.
    pub fdes: usize,

    /// FDEs joined to a named final function.
    pub named_fdes: usize,

    /// Named functions represented by those FDEs.
    pub functions: usize,

    /// Named functions whose FDE points to language-specific cleanup data.
    pub cleanup_functions: usize,

    pub largest: Vec<UnwindFunction>,
    pub crates: Vec<UnwindCrate>,
}

#[derive(Debug)]
pub struct UnwindFunction {
    pub name: String,
    pub bytes: u64,
    pub frame_bytes: u64,
    pub exception_bytes: u64,
    pub code_bytes: u64,
    pub fdes: usize,
    pub cleanups: usize,
    pub defined_at: Option<String>,
}

#[derive(Debug)]
pub struct UnwindCrate {
    pub name: String,
    pub bytes: u64,
    pub frame_bytes: u64,
    pub exception_bytes: u64,
    pub functions: usize,
    pub cleanup_functions: usize,
}

#[derive(Default)]
struct FunctionTotal {
    name: String,
    krate: Option<String>,
    frame_bytes: u64,
    exception_bytes: u64,
    code_bytes: u64,
    fdes: usize,
    code_addresses: FxHashSet<u64>,
    lsdas: FxHashSet<u64>,
}

struct FdeRef {
    owner: Option<String>,
    lsda: Option<u64>,
}

#[derive(Clone, Copy)]
struct SectionSpan {
    begin: u64,
    end: u64,
    bytes: u64,
}

#[derive(Default)]
struct CrateTotal {
    frame_bytes: u64,
    exception_bytes: u64,
    functions: usize,
    cleanup_functions: usize,
}

/// Read unwind ownership from the final linked image, keeping the `limit`
/// largest function and crate lists.
#[must_use]
pub fn analyze(file: &object::File<'_>, limit: usize) -> Option<UnwindReport> {
    let section =
        file.section_by_name(".eh_frame").or_else(|| file.section_by_name("__eh_frame"))?;
    let data = section.uncompressed_data().ok()?;
    if data.is_empty() {
        return None;
    }

    let endian = if file.is_little_endian() {
        gimli::RunTimeEndian::Little
    } else {
        gimli::RunTimeEndian::Big
    };
    let mut frame = EhFrame::new(&data, endian);
    frame.set_address_size(if file.is_64() { 8 } else { 4 });

    let mut bases = BaseAddresses::default().set_eh_frame(section.address());
    if let Some(text) = file.section_by_name(".text").or_else(|| file.section_by_name("__text")) {
        bases = bases.set_text(text.address());
    }
    if let Some(got) = file.section_by_name(".got").or_else(|| file.section_by_name("__got")) {
        bases = bases.set_got(got.address());
    }

    let mut code = sized_in(file, |category| category == Category::Code);
    code.sort_by_key(|symbol| symbol.address);

    let mut totals: FxHashMap<String, FunctionTotal> = FxHashMap::default();
    let mut refs = Vec::new();
    let mut fdes = 0;
    let mut named_fdes = 0;
    let mut entries = frame.entries(&bases);
    while let Some(entry) = entries.next().ok()? {
        let CieOrFde::Fde(partial) = entry else { continue };
        fdes += 1;
        let frame_bytes =
            record_bytes(&data, partial.offset().into_u64(), partial.entry_len().into_u64());
        let fde = partial.parse(EhFrame::cie_from_offset).ok()?;
        let symbol = containing(&code, fde.initial_address());
        let owner = symbol.map(|symbol| {
            named_fdes += 1;
            let name = demangle(symbol.mangled);
            let total = totals.entry(name.clone()).or_insert_with(|| FunctionTotal {
                krate: defining_crate(symbol.mangled, &name),
                name: name.clone(),
                ..FunctionTotal::default()
            });
            total.frame_bytes += frame_bytes;
            total.fdes += 1;
            if total.code_addresses.insert(symbol.address) {
                total.code_bytes += symbol.size;
            }
            name
        });
        let lsda = match fde.lsda() {
            Some(Pointer::Direct(address)) => Some(address),
            _ => None,
        };
        refs.push(FdeRef { owner, lsda });
    }

    let exception_sections = exception_sections(file);
    attribute_lsdas(&refs, &exception_sections, &mut totals);

    let attributed_bytes: u64 =
        totals.values().map(|total| total.frame_bytes + total.exception_bytes).sum();
    let functions = totals.len();
    let cleanup_functions = totals.values().filter(|total| !total.lsdas.is_empty()).count();

    let mut by_crate: FxHashMap<String, CrateTotal> = FxHashMap::default();
    for total in totals.values() {
        let Some(krate) = &total.krate else { continue };
        let group = by_crate.entry(krate.clone()).or_default();
        group.frame_bytes += total.frame_bytes;
        group.exception_bytes += total.exception_bytes;
        group.functions += 1;
        group.cleanup_functions += usize::from(!total.lsdas.is_empty());
    }
    let mut crates: Vec<UnwindCrate> = by_crate
        .into_iter()
        .map(|(name, total)| UnwindCrate {
            name,
            bytes: total.frame_bytes + total.exception_bytes,
            frame_bytes: total.frame_bytes,
            exception_bytes: total.exception_bytes,
            functions: total.functions,
            cleanup_functions: total.cleanup_functions,
        })
        .collect();
    crates.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
    crates.truncate(limit);

    let mut largest: Vec<UnwindFunction> = totals
        .into_values()
        .map(|total| UnwindFunction {
            name: total.name,
            bytes: total.frame_bytes + total.exception_bytes,
            frame_bytes: total.frame_bytes,
            exception_bytes: total.exception_bytes,
            code_bytes: total.code_bytes,
            fdes: total.fdes,
            cleanups: total.lsdas.len(),
            defined_at: None,
        })
        .collect();
    largest.sort_by(|a, b| {
        b.bytes
            .cmp(&a.bytes)
            .then_with(|| b.exception_bytes.cmp(&a.exception_bytes))
            .then_with(|| a.name.cmp(&b.name))
    });
    largest.truncate(limit);

    Some(UnwindReport {
        frame_bytes: data.len() as u64,
        exception_bytes: exception_sections.iter().map(|section| section.bytes).sum(),
        attributed_bytes,
        fdes,
        named_fdes,
        functions,
        cleanup_functions,
        largest,
        crates,
    })
}

/// The FDE length excludes its four-byte DWARF32 or twelve-byte DWARF64 length
/// prefix. Reading the prefix also avoids charging the next entry's bytes.
fn record_bytes(data: &[u8], offset: u64, payload: u64) -> u64 {
    let Ok(offset) = usize::try_from(offset) else { return payload };
    let prefix =
        if data.get(offset..offset.saturating_add(4)).is_some_and(|bytes| bytes == [0xff; 4]) {
            12
        } else {
            4
        };
    payload.saturating_add(prefix)
}

fn containing<'a>(symbols: &'a [Sized<'_>], address: u64) -> Option<&'a Sized<'a>> {
    let index = symbols.partition_point(|symbol| symbol.address <= address).checked_sub(1)?;
    let symbol = &symbols[index];
    (address < symbol.address.saturating_add(symbol.size)).then_some(symbol)
}

fn exception_sections(file: &object::File<'_>) -> Vec<SectionSpan> {
    let mut sections: Vec<SectionSpan> = file
        .sections()
        .filter_map(|section| {
            let name = section.name().ok()?;
            matches!(name, ".gcc_except_table" | "__gcc_except_tab").then(|| SectionSpan {
                begin: section.address(),
                end: section.address().saturating_add(section.size()),
                bytes: section.file_range().map_or(section.size(), |(_, bytes)| bytes),
            })
        })
        .collect();
    sections.sort_by_key(|section| section.begin);
    sections
}

/// Infer each LSDA record's encoded span from the next referenced LSDA in its
/// section. Shared LSDAs are divided between their named owners so the report's
/// total never exceeds the bytes in the table.
fn attribute_lsdas(
    refs: &[FdeRef],
    sections: &[SectionSpan],
    totals: &mut FxHashMap<String, FunctionTotal>,
) {
    let mut by_address: FxHashMap<u64, (u64, FxHashSet<String>)> = FxHashMap::default();
    for reference in refs {
        let Some(address) = reference.lsda else { continue };
        let Some(section) = section_containing(sections, address) else { continue };
        let entry =
            by_address.entry(address).or_insert_with(|| (section.end, FxHashSet::default()));
        if let Some(owner) = &reference.owner {
            entry.1.insert(owner.clone());
        }
    }

    let mut records: Vec<_> = by_address.into_iter().collect();
    records.sort_by_key(|&(address, _)| address);
    for index in 0..records.len() {
        let (address, (section_end, owners)) = &records[index];
        let next = records
            .get(index + 1)
            .map(|(address, _)| *address)
            .filter(|next| next < section_end)
            .unwrap_or(*section_end);
        let bytes = next.saturating_sub(*address);
        if owners.is_empty() {
            continue;
        }
        let each = bytes / owners.len() as u64;
        let mut remainder = bytes % owners.len() as u64;
        let mut owners: Vec<_> = owners.iter().collect();
        owners.sort();
        for owner in owners {
            if let Some(total) = totals.get_mut(owner) {
                let extra = u64::from(remainder > 0);
                remainder = remainder.saturating_sub(extra);
                total.exception_bytes += each + extra;
                total.lsdas.insert(*address);
            }
        }
    }
}

fn section_containing(sections: &[SectionSpan], address: u64) -> Option<SectionSpan> {
    let index = sections.partition_point(|section| section.begin <= address).checked_sub(1)?;
    let section = sections[index];
    (address < section.end).then_some(section)
}

#[cfg(test)]
mod tests {
    use rustc_hash::FxHashMap;

    use super::{
        FdeRef, FunctionTotal, SectionSpan, attribute_lsdas, record_bytes, section_containing,
    };

    #[test]
    fn counts_dwarf_record_prefixes() {
        assert_eq!(record_bytes(&[0, 0, 0, 0], 0, 20), 24);
        assert_eq!(record_bytes(&[0xff, 0xff, 0xff, 0xff], 0, 20), 32);
    }

    #[test]
    fn attributes_adjacent_and_shared_lsdas_once() {
        let refs = vec![
            FdeRef { owner: Some("a".to_owned()), lsda: Some(110) },
            FdeRef { owner: Some("b".to_owned()), lsda: Some(130) },
            FdeRef { owner: Some("c".to_owned()), lsda: Some(130) },
        ];
        let sections = vec![SectionSpan { begin: 100, end: 150, bytes: 50 }];
        let mut totals: FxHashMap<String, FunctionTotal> = ["a", "b", "c"]
            .into_iter()
            .map(|name| (name.to_owned(), FunctionTotal::default()))
            .collect();

        attribute_lsdas(&refs, &sections, &mut totals);

        assert_eq!(totals["a"].exception_bytes, 20);
        assert_eq!(totals["b"].exception_bytes + totals["c"].exception_bytes, 20);
        assert_eq!(section_containing(&sections, 149).map(|section| section.begin), Some(100));
        assert!(section_containing(&sections, 150).is_none());
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn reads_the_test_binarys_real_frame_records() {
        let path = std::env::current_exe().unwrap();
        let data = std::fs::read(path).unwrap();
        let file = object::File::parse(&*data).unwrap();
        let report = super::analyze(&file, 5).unwrap();

        assert!(report.fdes > 0);
        assert!(report.named_fdes > 0);
        assert!(report.attributed_bytes > 0);
        assert!(!report.largest.is_empty());
    }
}
