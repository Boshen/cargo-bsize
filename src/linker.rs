//! What the linker retained, from its map of the final link.
//!
//! The linked image says how many bytes survived but has forgotten which input
//! object or archive member supplied many of them. A linker map preserves that
//! boundary. Mach-O maps also name local literals and anonymous regions that
//! are absent from the final symbol table, so they close part of that coverage
//! gap. Exported definitions are listed alongside the map: they are retention
//! roots for a dynamic library, even when no call in the emitted assembly
//! reaches them.

use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};
use object::{Object, ObjectSymbol, SymbolScope};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::name::demangle;
use crate::sections::Category;

/// Attribution recovered from one platform linker map.
#[derive(Debug)]
pub struct LinkerReport {
    pub format: &'static str,
    pub path: String,
    pub live_bytes: u64,
    pub dead_bytes: u64,
    pub objects: usize,
    pub archive_members: usize,
    pub largest_objects: Vec<Contribution>,
    pub largest_archives: Vec<ArchiveContribution>,
    pub exported_roots: Vec<ExportedRoot>,
    pub linker_only_bytes: u64,
    pub linker_only: Vec<LinkerRegion>,
}

/// Bytes the map attributes to one object file or archive member.
#[derive(Debug)]
pub struct Contribution {
    pub name: String,
    pub bytes: u64,
    pub regions: usize,
}

/// Archive members combined under the archive that supplied them.
#[derive(Debug)]
pub struct ArchiveContribution {
    pub name: String,
    pub bytes: u64,
    pub members: usize,
    pub regions: usize,
}

/// An externally visible definition: a root the linker must retain.
#[derive(Debug)]
pub struct ExportedRoot {
    pub name: String,
    pub bytes: u64,
}

/// A region named by the linker but absent from the final symbol table.
#[derive(Debug)]
pub struct LinkerRegion {
    pub name: String,
    pub object: String,
    pub bytes: u64,
}

#[derive(Default)]
struct ParsedMap {
    format: Option<&'static str>,
    objects: FxHashMap<String, ObjectBytes>,
    live: Vec<MapRegion>,
}

#[derive(Default)]
struct ObjectBytes {
    live: u64,
    dead: u64,
    live_regions: usize,
}

struct MapRegion {
    address: Option<u64>,
    bytes: u64,
    object: String,
    name: String,
}

/// A stable location within cargo-bsize's private target directory.
pub(crate) fn map_path(target_dir: &Path, target: &str) -> Result<PathBuf> {
    let dir = target_dir.join("linker");
    fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;
    Ok(dir.join(format!("{target}.map")))
}

/// rustc arguments that pass a map filename through its linker driver.
pub(crate) fn rustc_args(path: &Path) -> Vec<OsString> {
    if cfg!(windows) {
        let mut arg = OsString::from("-Clink-arg=/MAP:");
        arg.push(path);
        return vec![arg];
    }

    let option = if cfg!(target_os = "macos") { "-map" } else { "-Map" };
    let mut filename = OsString::from("-Clink-arg=");
    filename.push(path);
    vec![
        OsString::from("-Clink-arg=-Xlinker"),
        OsString::from(format!("-Clink-arg={option}")),
        OsString::from("-Clink-arg=-Xlinker"),
        filename,
    ]
}

/// Add the equivalent arguments to the `cc` link used for static libraries.
pub(crate) fn configure_cc(command: &mut Command, path: &Path) {
    let option = if cfg!(target_os = "macos") { "-map" } else { "-Map" };
    command.arg("-Xlinker").arg(option).arg("-Xlinker").arg(path);
}

/// Parse `map`, combine its input contributions, and find final-image exports.
///
/// # Errors
///
/// Errors when the map cannot be read or its linker format is unknown.
pub fn analyze(
    file: &object::File<'_>,
    map: &Path,
    workspace: &Path,
    target_dir: &Path,
    include_exported_roots: bool,
    limit: usize,
) -> Result<LinkerReport> {
    let data =
        fs::read(map).with_context(|| format!("failed to read linker map {}", map.display()))?;
    let text = String::from_utf8_lossy(&data);
    let mut parsed = parse(&text)?;
    let format = parsed.format.context("linker map has no format")?;

    let live_bytes = parsed.objects.values().map(|object| object.live).sum();
    let dead_bytes = parsed.objects.values().map(|object| object.dead).sum();
    let objects = parsed.objects.values().filter(|object| object.live > 0).count();
    let archive_members = parsed
        .objects
        .iter()
        .filter(|(name, object)| object.live > 0 && split_archive(name).is_some())
        .count();

    let mut largest_objects: Vec<Contribution> = parsed
        .objects
        .iter()
        .filter(|(_, object)| object.live > 0)
        .map(|(name, object)| Contribution {
            name: display_object(name, workspace, target_dir),
            bytes: object.live,
            regions: object.live_regions,
        })
        .collect();
    largest_objects.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
    largest_objects.truncate(limit);

    let mut archives: FxHashMap<String, (u64, usize, usize)> = FxHashMap::default();
    for (name, object) in &parsed.objects {
        let Some((archive, _)) = split_archive(name) else { continue };
        if object.live == 0 {
            continue;
        }
        let entry = archives.entry(archive.to_owned()).or_default();
        entry.0 += object.live;
        entry.1 += 1;
        entry.2 += object.live_regions;
    }
    let mut largest_archives: Vec<ArchiveContribution> = archives
        .into_iter()
        .map(|(name, (bytes, members, regions))| ArchiveContribution {
            name: compact_path(&name, workspace, target_dir),
            bytes,
            members,
            regions,
        })
        .collect();
    largest_archives.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
    largest_archives.truncate(limit);

    let exported_roots =
        if include_exported_roots { exports(file, &parsed.live, limit) } else { Vec::new() };
    let mut named_ranges: Vec<(u64, u64)> = crate::symbols::sized(file)
        .into_iter()
        .filter(|symbol| matches!(symbol.category, Category::Code) || symbol.exact)
        .map(|symbol| (symbol.address, symbol.address.saturating_add(symbol.size)))
        .collect();
    named_ranges.sort_unstable();
    let named_starts: FxHashSet<u64> = file
        .symbols()
        .chain(file.dynamic_symbols())
        .filter(|symbol| !symbol.is_undefined())
        .map(|symbol| symbol.address())
        .collect();
    let mut linker_only_bytes = 0;
    let mut linker_only = Vec::new();
    for region in parsed.live.drain(..) {
        if region.address.is_some_and(|address| {
            named_starts.contains(&address) || covered(&named_ranges, address)
        }) {
            continue;
        }
        linker_only_bytes += region.bytes;
        linker_only.push(LinkerRegion {
            name: shorten(&demangle(&region.name)),
            object: display_object(&region.object, workspace, target_dir),
            bytes: region.bytes,
        });
    }
    linker_only.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
    linker_only.truncate(limit);

    Ok(LinkerReport {
        format,
        path: map.display().to_string(),
        live_bytes,
        dead_bytes,
        objects,
        archive_members,
        largest_objects,
        largest_archives,
        exported_roots,
        linker_only_bytes,
        linker_only,
    })
}

fn parse(text: &str) -> Result<ParsedMap> {
    if text.lines().any(|line| line == "# Object files:") {
        return Ok(parse_macho(text));
    }
    if text.contains("Linker script and memory map") {
        return Ok(parse_gnu(text));
    }
    if text.lines().any(|line| {
        line.contains("VMA")
            && line.contains("LMA")
            && line.contains("Size")
            && line.contains("Align")
    }) {
        return Ok(parse_lld(text));
    }
    if text.contains("Publics by Value") {
        return Ok(parse_msvc(text));
    }
    bail!("unrecognized linker map format")
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MachState {
    Other,
    Objects,
    Live,
    Dead,
}

fn parse_macho(text: &str) -> ParsedMap {
    let mut parsed = ParsedMap { format: Some("macho"), ..ParsedMap::default() };
    let mut state = MachState::Other;
    let mut objects: FxHashMap<u32, String> = FxHashMap::default();

    for line in text.lines() {
        state = match line {
            "# Object files:" => MachState::Objects,
            "# Symbols:" => MachState::Live,
            "# Dead Stripped Symbols:" => MachState::Dead,
            _ if line.starts_with("# Sections:") => MachState::Other,
            _ if line.starts_with('#') => state,
            _ => state,
        };

        match state {
            MachState::Objects => {
                if let Some((index, name)) = bracketed(line) {
                    objects.insert(index, name.to_owned());
                }
            }
            MachState::Live | MachState::Dead => {
                let Some((address, bytes, index, name)) = macho_region(line, state) else {
                    continue;
                };
                let object =
                    objects.get(&index).cloned().unwrap_or_else(|| format!("object #{index}"));
                let stats = parsed.objects.entry(object.clone()).or_default();
                if state == MachState::Live {
                    stats.live += bytes;
                    stats.live_regions += 1;
                    parsed.live.push(MapRegion { address, bytes, object, name: name.to_owned() });
                } else {
                    stats.dead += bytes;
                }
            }
            MachState::Other => {}
        }
    }
    parsed
}

fn bracketed(line: &str) -> Option<(u32, &str)> {
    let line = line.trim_start();
    let close = line.find(']')?;
    let index = line.strip_prefix('[')?[..close - 1].trim().parse().ok()?;
    Some((index, line[close + 1..].trim()))
}

fn macho_region(line: &str, state: MachState) -> Option<(Option<u64>, u64, u32, &str)> {
    let mut fields = line.split_whitespace();
    let first = fields.next()?;
    let second = fields.next()?;
    let address = (state == MachState::Live).then(|| hex(first)).flatten();
    if state == MachState::Live && address.is_none()
        || state == MachState::Dead && first != "<<dead>>"
    {
        return None;
    }
    let bytes = hex(second)?;

    let open = line.find('[')?;
    let close = line[open..].find(']')? + open;
    let index = line[open + 1..close].trim().parse().ok()?;
    let name = line[close + 1..].trim();
    Some((address, bytes, index, name))
}

fn parse_gnu(text: &str) -> ParsedMap {
    let mut parsed = ParsedMap { format: Some("elf"), ..ParsedMap::default() };
    let mut active = false;
    let mut dead = false;
    for line in text.lines() {
        if line == "Discarded input sections" {
            active = true;
            dead = true;
            continue;
        }
        if line == "Memory Configuration" {
            active = false;
            continue;
        }
        if line == "Linker script and memory map" {
            active = true;
            dead = false;
            continue;
        }
        if !active {
            continue;
        }
        let Some((bytes, object)) = gnu_contribution(line) else { continue };
        record_contribution(&mut parsed, object, bytes, dead);
    }
    parsed
}

fn gnu_contribution(line: &str) -> Option<(u64, &str)> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    let index =
        fields.windows(2).position(|pair| hex(pair[0]).is_some() && hex(pair[1]).is_some())?;
    let bytes = hex(fields[index + 1])?;
    let object = *fields.get(index + 2)?;
    looks_like_object(object).then_some((bytes, object))
}

fn parse_lld(text: &str) -> ParsedMap {
    let mut parsed = ParsedMap { format: Some("elf/lld"), ..ParsedMap::default() };
    for line in text.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 5 || hex(fields[0]).is_none() || hex(fields[2]).is_none() {
            continue;
        }
        let Some(input) = fields[4..].iter().find(|field| field.contains(":(")) else {
            continue;
        };
        let object = input.split_once(":(").map_or(*input, |(object, _)| object);
        if looks_like_object(object) {
            record_contribution(&mut parsed, object, hex(fields[2]).unwrap_or(0), false);
        }
    }
    parsed
}

fn parse_msvc(text: &str) -> ParsedMap {
    let mut parsed = ParsedMap { format: Some("msvc"), ..ParsedMap::default() };
    let preferred = text
        .lines()
        .find_map(|line| line.trim().strip_prefix("Preferred load address is ").and_then(hex));
    let mut active = false;
    let mut symbols: Vec<(u32, u64, String, String)> = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.contains("Address") && trimmed.contains("Publics by Value")
            || trimmed == "Static symbols"
        {
            active = true;
            continue;
        }
        if !active {
            continue;
        }
        let fields: Vec<&str> = trimmed.split_whitespace().collect();
        let Some((segment, _)) = fields.first().and_then(|field| field.split_once(':')) else {
            continue;
        };
        let (Some(segment), Some(address), Some(object)) = (
            u32::from_str_radix(segment, 16).ok(),
            fields.get(2).and_then(|field| hex(field)),
            fields.last().filter(|field| looks_like_msvc_object(field)),
        ) else {
            continue;
        };
        let address = preferred.map_or(address, |base| address.saturating_sub(base));
        symbols.push((
            segment,
            address,
            msvc_object(object),
            fields.get(1).copied().unwrap_or("(unnamed)").to_owned(),
        ));
    }

    symbols.sort_by_key(|(segment, address, ..)| (*segment, *address));
    symbols.dedup_by_key(|(segment, address, ..)| (*segment, *address));
    for pair in symbols.windows(2) {
        let (segment, address, object, name) = &pair[0];
        let (next_segment, next, ..) = &pair[1];
        if segment != next_segment || next <= address {
            continue;
        }
        let bytes = next - address;
        record_contribution(&mut parsed, object, bytes, false);
        parsed.live.push(MapRegion {
            address: Some(*address),
            bytes,
            object: object.clone(),
            name: name.clone(),
        });
    }
    parsed
}

fn looks_like_msvc_object(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".obj") || lower.contains(".lib:")
}

fn msvc_object(name: &str) -> String {
    let lower = name.to_ascii_lowercase();
    let Some(index) = lower.rfind(".lib:") else { return name.to_owned() };
    let split = index + ".lib".len();
    format!("{}({})", &name[..split], &name[split + 1..])
}

fn record_contribution(parsed: &mut ParsedMap, object: &str, bytes: u64, dead: bool) {
    let stats = parsed.objects.entry(object.to_owned()).or_default();
    if dead {
        stats.dead += bytes;
    } else {
        stats.live += bytes;
        stats.live_regions += 1;
    }
}

fn exports(file: &object::File<'_>, map_regions: &[MapRegion], limit: usize) -> Vec<ExportedRoot> {
    let mut roots: FxHashMap<(String, u64), u64> = FxHashMap::default();
    let map_sizes: FxHashMap<u64, u64> = map_regions
        .iter()
        .filter_map(|region| region.address.map(|address| (address, region.bytes)))
        .collect();
    for symbol in file.symbols().chain(file.dynamic_symbols()) {
        if symbol.is_undefined() || symbol.address() == 0 {
            continue;
        }
        if symbol.scope() == SymbolScope::Dynamic
            && let Ok(name) = symbol.name()
        {
            let bytes =
                symbol.size().max(map_sizes.get(&symbol.address()).copied().unwrap_or_default());
            if bytes > 0 {
                roots.insert((demangle(name), symbol.address()), bytes);
            }
        }
    }

    let mut roots: Vec<ExportedRoot> =
        roots.into_iter().map(|((name, _), bytes)| ExportedRoot { name, bytes }).collect();
    roots.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
    roots.truncate(limit);
    roots
}

fn covered(ranges: &[(u64, u64)], address: u64) -> bool {
    let position = ranges.partition_point(|&(start, _)| start <= address);
    position > 0 && address < ranges[position - 1].1
}

fn split_archive(name: &str) -> Option<(&str, &str)> {
    let open = name.rfind('(')?;
    let member = name.get(open + 1..name.len().checked_sub(1)?)?;
    let with_index = &name[..open];
    let archive = with_index
        .rfind('[')
        .filter(|&index| {
            with_index.ends_with(']')
                && [".a", ".rlib", ".lib"]
                    .iter()
                    .any(|extension| with_index[..index].ends_with(extension))
        })
        .map_or(with_index, |index| &with_index[..index]);
    (name.ends_with(')')
        && [".a", ".rlib", ".lib"].iter().any(|extension| archive.ends_with(extension)))
    .then_some((archive, member))
}

fn display_object(name: &str, workspace: &Path, target_dir: &Path) -> String {
    if let Some((archive, member)) = split_archive(name) {
        return format!("{}({member})", compact_path(archive, workspace, target_dir));
    }
    compact_path(name, workspace, target_dir)
}

fn compact_path(name: &str, workspace: &Path, target_dir: &Path) -> String {
    let path = Path::new(name);
    if let Ok(relative) = path.strip_prefix(workspace) {
        return relative.display().to_string();
    }
    if let Ok(relative) = path.strip_prefix(target_dir) {
        return relative.display().to_string();
    }
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| name.to_owned())
}

fn looks_like_object(name: &str) -> bool {
    name.ends_with(".o")
        || name.ends_with(".obj")
        || split_archive(name).is_some()
        || name.contains(".a(")
        || name.contains(".rlib(")
        || name.contains(".lib(")
}

fn hex(value: &str) -> Option<u64> {
    let value = value.strip_prefix("0x").unwrap_or(value).trim_end_matches(['H', 'h']);
    (!value.is_empty() && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .then(|| u64::from_str_radix(value, 16).ok())
        .flatten()
}

fn shorten(name: &str) -> String {
    const MAX: usize = 160;
    if name.chars().count() <= MAX {
        return name.to_owned();
    }
    let mut short: String = name.chars().take(MAX - 1).collect();
    short.push('…');
    short
}

#[cfg(test)]
mod tests {
    use super::{parse, split_archive};

    #[test]
    fn parses_macho_objects_live_symbols_and_dead_symbols() {
        let text = r#"# Path: /tmp/libanswer.dylib
# Arch: arm64
# Object files:
[  0] linker synthesized
[  1] /tmp/answer.o
[  2] /tmp/libnative.a(helper.o)
# Sections:
# Address Size Segment Section
0x00001000 0x00000030 __TEXT __text
# Symbols:
# Address Size File Name
0x00001000 0x00000020 [  1] _answer
0x00001020 0x00000010 [  2] literal string: hello
# Dead Stripped Symbols:
<<dead>> 0x00000008 [  2] _unused
"#;
        let parsed = parse(text).unwrap();
        assert_eq!(parsed.format, Some("macho"));
        assert_eq!(parsed.objects["/tmp/answer.o"].live, 0x20);
        assert_eq!(parsed.objects["/tmp/libnative.a(helper.o)"].live, 0x10);
        assert_eq!(parsed.objects["/tmp/libnative.a(helper.o)"].dead, 8);
        assert_eq!(parsed.live.len(), 2);
    }

    #[test]
    fn parses_gnu_input_sections() {
        let text = r#"Discarded input sections

 .text.unused   0x0000000000000000 0x8 /tmp/a.o

Memory Configuration

Linker script and memory map

 .text.answer   0x0000000000001000 0x20 /tmp/a.o
 .rodata.value  0x0000000000001020 0x10 /tmp/libb.rlib(b.o)
"#;
        let parsed = parse(text).unwrap();
        assert_eq!(parsed.objects["/tmp/a.o"].live, 0x20);
        assert_eq!(parsed.objects["/tmp/a.o"].dead, 8);
        assert_eq!(parsed.objects["/tmp/libb.rlib(b.o)"].live, 0x10);
    }

    #[test]
    fn parses_lld_input_sections() {
        let text = r#"             VMA              LMA     Size Align Out     In      Symbol
          201000           201000       30    16 .text
          201000           201000       20     4         /tmp/a.o:(.text.answer)
          201020           201020       10     4         /tmp/libb.rlib(b.o):(.text.helper)
"#;
        let parsed = parse(text).unwrap();
        assert_eq!(parsed.format, Some("elf/lld"));
        assert_eq!(parsed.objects["/tmp/a.o"].live, 0x20);
        assert_eq!(parsed.objects["/tmp/libb.rlib(b.o)"].live, 0x10);
    }

    #[test]
    fn parses_msvc_public_symbol_spans() {
        let text = r#" Preferred load address is 0000000140000000

  Address         Publics by Value              Rva+Base               Lib:Object

 0001:00000000       answer                     0000000140001000 f   answer.obj
 0001:00000020       helper                     0000000140001020 f   native.lib:helper.obj
 0001:00000030       finish                     0000000140001030 f   answer.obj
"#;
        let parsed = parse(text).unwrap();
        assert_eq!(parsed.format, Some("msvc"));
        assert_eq!(parsed.objects["answer.obj"].live, 0x20);
        assert_eq!(parsed.objects["native.lib(helper.obj)"].live, 0x10);
        assert_eq!(parsed.live[0].address, Some(0x1000));
    }

    #[test]
    fn recognizes_archive_members() {
        assert_eq!(split_archive("/tmp/libfoo.rlib(foo.o)"), Some(("/tmp/libfoo.rlib", "foo.o")));
        assert_eq!(split_archive("/tmp/libfoo.a[42](foo.o)"), Some(("/tmp/libfoo.a", "foo.o")));
        assert_eq!(split_archive("/tmp/foo.o"), None);
    }
}
