//! The pointers in constant data, and what the loader charges for them.
//!
//! A `&'static str` in a table, a method in a vtable, the path in a panic
//! location: every pointer a position-independent binary keeps in its data is a
//! slot the loader must fill at start, and on ELF each slot costs a relocation
//! record of its own — 24 bytes in `.rela.dyn` for a 64-bit target, on top of
//! the 8-byte slot, unless the linker packed them into `.relr.dyn`. Mach-O
//! chains its fixups through the slots themselves and pays almost nothing
//! extra, so there only the slot counts are reported. Either way the slots
//! name the data that could be built from offsets instead of pointers.

use object::{
    Architecture, Object, ObjectSection, ObjectSegment, RelocationFlags,
    read::macho::{MachHeader, MachOFile},
};
use rustc_hash::FxHashMap;
use serde::Serialize;

use crate::{
    name::demangle,
    sections::Category,
    symbols::{Sized, sized_in},
};

#[derive(Debug, Serialize)]
pub struct RelocationReport {
    /// Pointer slots the loader fills at start: relative relocations on ELF,
    /// rebase fixups on Mach-O.
    pub slots: usize,

    /// File bytes of the relocation records themselves: `.rela.dyn` and
    /// `.relr.dyn` on ELF; the compressed rebase opcodes on Mach-O, or nothing
    /// when its fixups are chained through the slots.
    pub bytes: u64,

    /// Whether the records are packed (`.relr.dyn`), at a fraction of a byte
    /// per slot.
    pub packed: bool,

    /// Bytes each slot's record costs — 24 for an unpacked 64-bit ELF entry,
    /// 0 when packed or on Mach-O, where the cost is amortized.
    pub record: u64,

    /// Sections by slots, most first.
    pub sections: Vec<SlotGroup>,

    /// Data symbols by slots, most first; anonymous constants fall to the
    /// section rows above.
    pub symbols: Vec<SlotGroup>,
}

#[derive(Debug, Serialize)]
pub struct SlotGroup {
    pub name: String,
    pub slots: usize,

    /// The records' bytes: slots × `record`.
    pub bytes: u64,
}

/// Count the pointer slots in `file`'s data and attribute them to sections and
/// symbols, keeping the `limit` largest of each.
#[must_use]
pub fn analyze(file: &object::File<'_>, limit: usize) -> Option<RelocationReport> {
    let (slots, bytes, packed, record) = match file {
        object::File::MachO64(macho) => macho_rebases(macho)?,
        object::File::Elf64(_) | object::File::Elf32(_) => elf_relative(file)?,
        _ => return None,
    };
    if slots.is_empty() {
        return None;
    }

    // Sections and named data symbols, by address, for the join. Mach-O
    // section names are unique only within a segment, so they carry it.
    let mut sections: Vec<(u64, u64, String)> = file
        .sections()
        .filter_map(|section| {
            let name = section.name().ok()?;
            let name = section
                .segment_name()
                .ok()
                .flatten()
                .map_or_else(|| name.to_owned(), |segment| format!("{segment},{name}"));
            (section.size() > 0).then(|| (section.address(), section.size(), name))
        })
        .collect();
    sections.sort_unstable();
    let mut symbols: Vec<Sized<'_>> =
        sized_in(file, |category| matches!(category, Category::ReadOnlyData | Category::Data));
    symbols.sort_by_key(|symbol| symbol.address);

    let mut by_section: FxHashMap<&str, usize> = FxHashMap::default();
    let mut by_symbol: FxHashMap<&str, usize> = FxHashMap::default();
    for &slot in &slots {
        if let Some((_, _, name)) = containing(&sections, slot, |&(begin, size, _)| (begin, size)) {
            *by_section.entry(name.as_str()).or_default() += 1;
        }
        if let Some(symbol) = containing(&symbols, slot, |symbol| (symbol.address, symbol.size)) {
            *by_symbol.entry(symbol.mangled).or_default() += 1;
        }
    }

    let group =
        |name: String, slots: usize| SlotGroup { name, slots, bytes: slots as u64 * record };
    let mut sections: Vec<SlotGroup> =
        by_section.into_iter().map(|(name, slots)| group(name.to_owned(), slots)).collect();
    let mut symbols: Vec<SlotGroup> =
        by_symbol.into_iter().map(|(name, slots)| group(demangle(name), slots)).collect();
    for list in [&mut sections, &mut symbols] {
        list.sort_by(|a, b| b.slots.cmp(&a.slots).then_with(|| a.name.cmp(&b.name)));
        list.truncate(limit);
    }

    Some(RelocationReport { slots: slots.len(), bytes, packed, record, sections, symbols })
}

/// The entry of `ranges` (sorted by start) whose `[start, start + size)` holds
/// `address`.
fn containing<T>(ranges: &[T], address: u64, span: impl Fn(&T) -> (u64, u64)) -> Option<&T> {
    let index = ranges.partition_point(|range| span(range).0 <= address).checked_sub(1)?;
    let (start, size) = span(&ranges[index]);
    (address < start + size).then(|| &ranges[index])
}

/// The addresses of every relative relocation in an ELF binary, the file bytes
/// of the sections that hold the records, whether they are packed, and the
/// bytes one record costs.
fn elf_relative(file: &object::File<'_>) -> Option<(Vec<u64>, u64, bool, u64)> {
    let relative = match file.architecture() {
        Architecture::X86_64 => object::elf::R_X86_64_RELATIVE,
        Architecture::Aarch64 => object::elf::R_AARCH64_RELATIVE,
        Architecture::I386 => object::elf::R_386_RELATIVE,
        Architecture::Arm => object::elf::R_ARM_RELATIVE,
        Architecture::Riscv64 | Architecture::Riscv32 => object::elf::R_RISCV_RELATIVE,
        Architecture::PowerPc64 => object::elf::R_PPC64_RELATIVE,
        Architecture::S390x => object::elf::R_390_RELATIVE,
        Architecture::LoongArch64 => object::elf::R_LARCH_RELATIVE,
        _ => return None,
    };

    let slots: Vec<u64> = file
        .dynamic_relocations()?
        .filter(|(_, relocation)| {
            matches!(relocation.flags(), RelocationFlags::Elf { r_type } if r_type == relative)
        })
        .map(|(address, _)| address)
        .collect();

    let section_bytes = |name: &str| {
        file.section_by_name(name)
            .and_then(|section| section.file_range())
            .map_or(0, |(_, size)| size)
    };
    let rela = section_bytes(".rela.dyn") + section_bytes(".rel.dyn");
    let relr = section_bytes(".relr.dyn");
    let packed = relr > 0;
    let record = if packed {
        0
    } else if file.is_64() {
        24
    } else {
        8
    };
    Some((slots, rela + relr, packed, record))
}

/// The addresses of every rebased pointer in a Mach-O binary — from the
/// classic `LC_DYLD_INFO` rebase opcodes (whose bytes are the only file cost,
/// run-length compressed) or from chained fixups (which live in the slots and
/// cost nothing more) — with the record bytes, never packed, and no cost per
/// slot.
fn macho_rebases<'data, Mach: MachHeader>(
    macho: &MachOFile<'data, Mach, &'data [u8]>,
) -> Option<(Vec<u64>, u64, bool, u64)> {
    let endian = macho.endian();
    let data = macho.data();
    let segments: Vec<(u64, u64, u64)> = macho
        .segments()
        .map(|segment| {
            let (offset, size) = segment.file_range();
            (segment.address(), offset, size)
        })
        .collect();
    let text = macho
        .segments()
        .find(|segment| segment.name() == Ok(Some("__TEXT")))
        .map_or(0, |segment| segment.address());
    let pointer_size = if macho.is_64() { 8 } else { 4 };

    let mut slots = Vec::new();
    let mut bytes = 0;
    let mut commands = macho.macho_load_commands().ok()?;
    while let Ok(Some(command)) = commands.next() {
        if let Ok(Some(info)) = command.dyld_info() {
            bytes += u64::from(info.rebase_size.get(endian));
            let mut rebases = info.rebases(endian, data, pointer_size).ok()?;
            while let Ok(Some(rebase)) = rebases.next() {
                if let Some(&(address, ..)) = segments.get(usize::from(rebase.segment_index)) {
                    slots.push(address + rebase.segment_offset);
                }
            }
        } else if let Ok(Some(linkedit)) = command.dyld_chained_fixups() {
            let fixups = linkedit.chained_fixups(endian, data).ok()?;
            let mut chained = fixups.segments(endian).ok()?;
            while let Ok(Some(segment)) = chained.next() {
                let Some(&(address, offset, size)) = segments.get(segment.index() as usize) else {
                    continue;
                };
                let Some(segment_data) = usize::try_from(offset)
                    .ok()
                    .zip(usize::try_from(size).ok())
                    .and_then(|(o, s)| data.get(o..o + s))
                else {
                    continue;
                };
                let mut entries = segment.fixups(endian, text, segment_data);
                while let Ok(Some((slot, fixup))) = entries.next() {
                    if matches!(fixup, object::read::macho::Fixup::Rebase(_)) {
                        slots.push(address + slot);
                    }
                }
            }
        }
    }
    Some((slots, bytes, false, 0))
}

#[cfg(test)]
mod tests {
    use super::containing;

    #[test]
    fn finds_the_range_holding_an_address() {
        let ranges = [(0x100u64, 0x10u64, "a"), (0x200, 0x08, "b")];
        let span = |&(start, size, _): &(u64, u64, &str)| (start, size);
        assert_eq!(containing(&ranges, 0x100, span).map(|r| r.2), Some("a"));
        assert_eq!(containing(&ranges, 0x10f, span).map(|r| r.2), Some("a"));
        assert_eq!(containing(&ranges, 0x110, span), None);
        assert_eq!(containing(&ranges, 0x204, span).map(|r| r.2), Some("b"));
        assert_eq!(containing(&ranges, 0x0ff, span), None);
    }
}
