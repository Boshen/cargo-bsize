//! Reference edges retained in linker input objects.
//!
//! Assembly is the richest source because it still spells local labels, but it
//! may cover only the final crate when LTO is off. Relocatable objects retain
//! every cross-symbol call and pointer the linker had to resolve. The linker
//! map selects only inputs that contributed live bytes; final-image and
//! map-live symbol names then keep dead sections from creating false paths.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use object::{
    Object, ObjectSection, ObjectSymbol, Relocation, RelocationEncoding, RelocationTarget,
    SectionIndex, SectionKind, SymbolKind, read::archive::ArchiveFile,
};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    assembly::is_initializer_section,
    graph::Edges,
    linker::{self, LiveInput},
};

/// Add references from every live linker input that is still available.
/// Missing objects and malformed native archive members are skipped: the
/// assembly graph remains useful on its own, and the report states how many
/// inputs were read.
pub(crate) fn analyze(
    binary: &object::File<'_>,
    map: &Path,
    workspace: &Path,
    target_dir: &Path,
    sizes: &FxHashMap<String, u64>,
    edges: &mut Edges,
) -> Result<()> {
    let inputs = linker::live_inputs(map)?;
    let total = inputs.len();

    let mut linked: FxHashSet<String> = sizes.keys().cloned().collect();
    linked.extend(edges.data_labels().map(str::to_owned));
    for symbol in binary.symbols().chain(binary.dynamic_symbols()) {
        if !symbol.is_undefined()
            && let Ok(name) = symbol.name()
        {
            linked.insert(name.to_owned());
        }
    }
    for input in &inputs {
        linked.extend(input.symbols.iter().cloned());
    }

    let mut objects = 0;
    let mut archives: FxHashMap<PathBuf, Vec<LiveInput>> = FxHashMap::default();
    for input in inputs {
        let path = resolve(&input.path, workspace, target_dir);
        if let Some(member) = &input.member {
            // rustc normally leaves an extracted copy beside its rlib. It is
            // dramatically cheaper to read than a debug-heavy archive.
            let sibling = path.parent().unwrap_or(Path::new("")).join(member);
            if read_object_path(&sibling, &input, &linked, edges) {
                objects += 1;
            } else {
                archives.entry(path).or_default().push(input);
            }
        } else if read_object_path(&path, &input, &linked, edges) {
            objects += 1;
        }
    }

    for (path, inputs) in archives {
        let Ok(data) = fs::read(&path) else { continue };
        let Ok(archive) = ArchiveFile::parse(&*data) else { continue };
        for input in inputs {
            let Some(wanted) = input.member.as_deref() else { continue };
            let member = archive
                .members()
                .filter_map(Result::ok)
                .find(|member| String::from_utf8_lossy(member.name()) == wanted);
            let Some(member) = member else { continue };
            let Ok(data) = member.data(&*data) else { continue };
            if add_object(data, &input, &linked, edges).is_ok() {
                objects += 1;
            }
        }
    }

    edges.relocation_stats(total, objects);
    Ok(())
}

fn resolve(path: &Path, workspace: &Path, target_dir: &Path) -> PathBuf {
    if path.is_absolute() || path.exists() {
        return path.to_owned();
    }
    for base in [workspace, target_dir] {
        let candidate = base.join(path);
        if candidate.exists() {
            return candidate;
        }
    }
    path.to_owned()
}

fn read_object_path(
    path: &Path,
    input: &LiveInput,
    linked: &FxHashSet<String>,
    edges: &mut Edges,
) -> bool {
    fs::read(path).is_ok_and(|data| add_object(&data, input, linked, edges).is_ok())
}

#[derive(Clone, Copy)]
struct SymbolRange<'data> {
    begin: u64,
    end: u64,
    name: Option<&'data str>,
}

#[derive(Clone, Copy)]
struct RawSymbol<'data> {
    address: u64,
    size: u64,
    name: Option<&'data str>,
}

fn add_object(
    data: &[u8],
    input: &LiveInput,
    linked: &FxHashSet<String>,
    edges: &mut Edges,
) -> object::read::Result<()> {
    let file = object::File::parse(data)?;
    let ranges = symbol_ranges(&file, input, linked);

    for section in file.sections() {
        if !loadable(section.kind()) {
            continue;
        }
        let symbols = ranges.get(&section.index()).map(Vec::as_slice).unwrap_or_default();
        let initializer = section.name().is_ok_and(is_initializer_section);
        for (offset, relocation) in section.relocations() {
            let address = section.address().saturating_add(offset);
            let Some(to) = relocation_target(&file, &ranges, &relocation, input, linked) else {
                continue;
            };
            let Some(from) = containing(symbols, address).and_then(|symbol| symbol.name) else {
                if initializer {
                    edges.root(to);
                }
                continue;
            };
            if initializer {
                edges.root(from);
            }

            if section.kind() != SectionKind::Text {
                edges.relocation_slot(from, to);
            } else if matches!(
                relocation.encoding(),
                RelocationEncoding::AArch64Call | RelocationEncoding::X86Branch
            ) {
                edges.relocation_call(from, to);
            } else {
                edges.relocation_address(from, to);
            }
        }
    }
    Ok(())
}

fn symbol_ranges<'data>(
    file: &object::File<'data>,
    input: &LiveInput,
    linked: &FxHashSet<String>,
) -> FxHashMap<SectionIndex, Vec<SymbolRange<'data>>> {
    let mut sections: FxHashMap<SectionIndex, Vec<RawSymbol<'_>>> = FxHashMap::default();
    for symbol in file.symbols() {
        let Some(section) = symbol.section_index() else { continue };
        if !symbol.is_definition()
            || matches!(symbol.kind(), SymbolKind::Section | SymbolKind::File)
        {
            continue;
        }
        let name = symbol.name().ok().filter(|name| {
            linked.contains(*name) && (input.symbols.is_empty() || input.symbols.contains(*name))
        });
        sections.entry(section).or_default().push(RawSymbol {
            address: symbol.address(),
            size: symbol.size(),
            name,
        });
    }

    let ends: FxHashMap<SectionIndex, u64> = file
        .sections()
        .map(|section| (section.index(), section.address().saturating_add(section.size())))
        .collect();
    sections
        .into_iter()
        .map(|(section, mut symbols)| {
            symbols.sort_by_key(|symbol| (symbol.address, symbol.name.is_none()));
            let mut ranges = Vec::new();
            let mut position = 0;
            while position < symbols.len() {
                let begin = symbols[position].address;
                let mut next = position + 1;
                while next < symbols.len() && symbols[next].address == begin {
                    next += 1;
                }
                let name = symbols[position..next].iter().find_map(|symbol| symbol.name);
                let declared =
                    symbols[position..next].iter().map(|symbol| symbol.size).max().unwrap_or(0);
                let inferred = symbols.get(next).map_or_else(
                    || ends.get(&section).copied().unwrap_or(begin),
                    |symbol| symbol.address,
                );
                let end = if declared > 0 { begin.saturating_add(declared) } else { inferred };
                ranges.push(SymbolRange { begin, end, name });
                position = next;
            }
            (section, ranges)
        })
        .collect()
}

fn relocation_target<'data>(
    file: &object::File<'data>,
    ranges: &FxHashMap<SectionIndex, Vec<SymbolRange<'data>>>,
    relocation: &Relocation,
    input: &LiveInput,
    linked: &FxHashSet<String>,
) -> Option<&'data str> {
    match relocation.target() {
        RelocationTarget::Symbol(index) => {
            let symbol = file.symbol_by_index(index).ok()?;
            if let Ok(name) = symbol.name()
                && linked.contains(name)
                && (symbol.is_undefined()
                    || input.symbols.is_empty()
                    || input.symbols.contains(name))
            {
                return Some(name);
            }
            let section = symbol.section_index()?;
            let address = add_signed(symbol.address(), relocation.addend())?;
            containing(ranges.get(&section)?, address)?.name
        }
        RelocationTarget::Section(section) => {
            let base = file.section_by_index(section).ok()?.address();
            let address = add_signed(base, relocation.addend())?;
            containing(ranges.get(&section)?, address)?.name
        }
        _ => None,
    }
}

fn containing<'ranges, 'data>(
    ranges: &'ranges [SymbolRange<'data>],
    address: u64,
) -> Option<&'ranges SymbolRange<'data>> {
    let index = ranges.partition_point(|range| range.begin <= address).checked_sub(1)?;
    (address < ranges[index].end).then(|| &ranges[index])
}

fn add_signed(value: u64, addend: i64) -> Option<u64> {
    if addend >= 0 {
        value.checked_add(addend.unsigned_abs())
    } else {
        value.checked_sub(addend.unsigned_abs())
    }
}

const fn loadable(kind: SectionKind) -> bool {
    matches!(
        kind,
        SectionKind::Unknown
            | SectionKind::Text
            | SectionKind::Data
            | SectionKind::ReadOnlyData
            | SectionKind::ReadOnlyString
            | SectionKind::Tls
            | SectionKind::TlsVariables
    )
}

#[cfg(test)]
mod tests {
    use super::{SymbolRange, add_signed, containing};

    #[test]
    fn finds_symbol_ranges_and_applies_signed_addends() {
        let ranges = [
            SymbolRange { begin: 10, end: 20, name: Some("a") },
            SymbolRange { begin: 30, end: 40, name: Some("b") },
        ];
        assert_eq!(containing(&ranges, 10).and_then(|range| range.name), Some("a"));
        assert!(containing(&ranges, 20).is_none());
        assert_eq!(containing(&ranges, 39).and_then(|range| range.name), Some("b"));
        assert_eq!(add_signed(10, 5), Some(15));
        assert_eq!(add_signed(10, -5), Some(5));
        assert_eq!(add_signed(0, -1), None);
    }
}
