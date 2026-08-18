//! What the read-only data is made of, from the assembly.
//!
//! The symbol table names a few dozen statics and leaves the rest of the
//! constant sections — usually most of them — as anonymous bytes between them.
//! The assembly names every one: `l_anon.<hash>.<n>` on Mach-O,
//! `.Lanon.<hash>.<n>` on ELF, each with its bytes spelled out. Read by shape
//! rather than by name, they sort into a handful of things a reader can act on:
//! the `core::panic::Location` record every panic site loads (a pointer to a
//! source path, its length, a line and a column: 24 bytes, plus the path
//! itself), the type and variant names `#[derive(Debug)]` writes, the messages
//! `expect` and `panic!` carry, the `&str` slices a `format_args!` is built
//! from, the vtables behind `dyn Trait`, the lookup and jump tables a large
//! `match` compiles to, and the byte tables everything else is.
//!
//! The parser in `assembly` streams the `.s` once and hands each directive to
//! the collector here; the analysis runs afterwards. Constants are emitted after
//! the code that loads them, so a function's references are resolved at the
//! end. Only what a linked function reaches is counted, mirroring the assembly
//! view's `linked` line, and under `lto = "fat"` that is the whole program.

use std::path::Path;

use rustc_hash::{FxHashMap, FxHashSet};
use serde::Serialize;

use crate::{
    assembly::{Origin, source},
    name::demangle,
};

/// The most bytes of a constant kept for classification. A pointer slot, a
/// length, a line and a column fit in 24; 32 leaves room to check what follows.
const HEAD: usize = 32;

/// The longest text preview kept for a string.
const PREVIEW_CHARS: usize = 64;

/// The most source lines listed per file of panic locations.
const LINES_PER_FILE: usize = 5;

/// The most functions named per string.
const FUNCTIONS_PER_STRING: usize = 2;

#[derive(Debug, Serialize)]
pub struct ConstantsReport {
    /// Constants defined in the assembly's constant sections.
    pub constants: usize,

    /// Of those, the ones some linked function reaches, directly or through
    /// another constant's pointer. Everything below counts only these.
    pub linked: usize,

    /// Bytes of the linked constants.
    pub bytes: u64,

    /// The linked bytes by what the constant is; a partition of `bytes`.
    pub classes: Vec<ConstantClass>,

    pub locations: Locations,

    /// Strings and string-slice arrays loaded only on the way to a panic:
    /// `expect` and `panic!` messages, and the pieces of a formatted one.
    pub panic_messages: ConstantClass,

    /// The largest strings — messages and names; paths have their own view —
    /// with the functions that load them.
    pub strings: Vec<StringConstant>,

    /// Functions by the lookup and jump tables their `match`es compiled to.
    pub tables: Vec<FunctionTables>,

    /// Functions by the constant bytes they reach, directly or through a
    /// table's pointers; `exclusive` is what only they reach, the bytes
    /// rewriting the function alone would free.
    pub functions: Vec<ConstantCarrier>,

    /// Functions by the size of their exception tables — the landing pads a
    /// `panic = "unwind"` build keeps for the values with destructors held
    /// across calls. An estimate: LEB128 fields are counted at one byte.
    pub unwind: Vec<UnwindTable>,
}

#[derive(Debug, Serialize)]
pub struct ConstantClass {
    pub kind: Kind,
    pub bytes: u64,
    pub count: usize,
}

/// `core::panic::Location` records: one per panic site, each 24 bytes plus the
/// source path it points at.
#[derive(Debug, Serialize)]
pub struct Locations {
    pub records: usize,

    /// The records themselves plus the path strings they point at, each path
    /// once.
    pub bytes: u64,

    /// Files by the panic sites recorded in them.
    pub files: Vec<LocationFile>,

    /// The same, for files in this workspace.
    pub workspace_files: Vec<LocationFile>,

    /// Functions by the records they load.
    pub functions: Vec<LocationCaller>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocationFile {
    pub file: String,
    pub records: usize,

    /// The records plus the path string once.
    pub bytes: u64,

    /// The lines with the most records, most first.
    pub lines: Vec<u64>,
}

#[derive(Debug, Serialize)]
pub struct LocationCaller {
    pub name: String,
    pub records: usize,
    pub bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct StringConstant {
    pub bytes: u64,
    pub kind: Kind,

    /// The text, cut to a preview.
    pub preview: String,

    /// The functions that reach it, up to a few.
    pub functions: Vec<String>,

    /// How many functions reach it.
    pub references: usize,
}

#[derive(Debug, Serialize)]
pub struct FunctionTables {
    pub name: String,
    pub bytes: u64,
    pub switch_tables: usize,
    pub jump_tables: usize,
}

#[derive(Debug, Serialize)]
pub struct ConstantCarrier {
    pub name: String,

    /// Bytes of every constant it reaches.
    pub bytes: u64,

    /// Bytes of the constants only it reaches.
    pub exclusive: u64,

    pub constants: usize,
}

#[derive(Debug, Serialize)]
pub struct UnwindTable {
    pub name: String,
    pub bytes: u64,
}

/// What a constant is, read from its shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    /// A source file path: what a panic location points at.
    Path,
    /// A `core::panic::Location`: path pointer, length, line, column.
    Location,
    /// A type, variant, or field name — what `#[derive(Debug)]` writes.
    Name,
    /// Any other text: messages, format literals.
    Message,
    /// An array of `&str`: the pieces of a `format_args!`, a table of names.
    Pieces,
    /// A trait object's vtable: drop, size, align, then the methods.
    Vtable,
    /// A table of function pointers.
    PointerTable,
    /// A table of records that point at other constants: `&[&[T]]`,
    /// `&[(char, &str)]`, and the like.
    SliceTable,
    /// A lookup table LLVM built for a `match`, named for its function.
    SwitchTable,
    /// A jump table for a `match`, named for its function.
    JumpTable,
    /// Bytes that are not text and hold no pointers: numeric tables.
    Bytes,
    /// Anything else with pointers in it.
    Other,
    /// A function's exception table (landing pads); counted separately.
    Lsda,
}

impl Kind {
    /// The row label in the text report.
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Path => "source paths (what panic locations point at)",
            Self::Location => "panic locations (24 B records)",
            Self::Name => "type, variant, and field names",
            Self::Message => "messages and other text",
            Self::Pieces => "string slices and format pieces",
            Self::Vtable => "vtables",
            Self::PointerTable => "function pointer tables",
            Self::SliceTable => "tables of slices and records",
            Self::SwitchTable => "lookup tables for `match`",
            Self::JumpTable => "jump tables for `match`",
            Self::Bytes => "byte tables",
            Self::Other => "other pointer data",
            Self::Lsda => "exception tables",
        }
    }
}

/// Which section a constant sits in, as far as the classification cares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Section {
    /// Read-only and relocated-read-only data.
    Constants,
    /// `__gcc_except_tab` / `.gcc_except_table`.
    Unwind,
}

/// One constant, as the parser saw it.
struct Constant {
    label: u32,
    section: Section,
    bytes: u64,

    /// Bytes decoded from `.ascii`/`.asciz`, the terminating NUL excluded.
    text: u64,

    /// Whether every string chunk was valid, printable UTF-8.
    text_ok: bool,

    /// The first bytes, as laid out: strings decoded, integers little-endian,
    /// a pointer slot as zeros.
    head: [u8; HEAD],
    head_len: u8,

    /// Pointer slots: byte offset and target label or symbol.
    slots: Vec<(u32, u32)>,

    /// The text, cut to a preview; the whole of it for a path, which a
    /// location record needs spelled out.
    preview: Option<Box<str>>,

    /// Whether any directive has landed since the label; a second label before
    /// one arrives is an alias.
    touched: bool,
}

impl Constant {
    fn new(label: u32, section: Section) -> Self {
        Self {
            label,
            section,
            bytes: 0,
            text: 0,
            text_ok: true,
            head: [0; HEAD],
            head_len: 0,
            slots: Vec::new(),
            preview: None,
            touched: false,
        }
    }

    fn push_head(&mut self, bytes: &[u8]) {
        let room = HEAD - usize::from(self.head_len);
        let take = bytes.len().min(room);
        self.head[usize::from(self.head_len)..usize::from(self.head_len) + take]
            .copy_from_slice(&bytes[..take]);
        self.head_len += take as u8;
    }

    fn u64_at(&self, offset: usize) -> Option<u64> {
        let end = offset + 8;
        (end <= usize::from(self.head_len))
            .then(|| u64::from_le_bytes(self.head[offset..end].try_into().unwrap_or([0; 8])))
    }

    fn u32_at(&self, offset: usize) -> Option<u32> {
        let end = offset + 4;
        (end <= usize::from(self.head_len))
            .then(|| u32::from_le_bytes(self.head[offset..end].try_into().unwrap_or([0; 4])))
    }
}

/// What the parser gathers while the assembly streams by.
#[derive(Default)]
pub(crate) struct Collected {
    /// Every label and symbol seen, interned; ids are per name, and the
    /// per-file map below restarts with each `.s` file, since assembler-local
    /// names do.
    names: Vec<String>,
    ids: FxHashMap<String, u32>,

    constants: Vec<Constant>,
    /// Which constant a label id opened or aliases.
    by_label: FxHashMap<u32, usize>,
    current: Option<usize>,

    /// Function symbol → constant label, from the instructions.
    references: Vec<(u32, u32)>,
    /// The same, for loads in a block that ends in a panic call.
    panic_references: Vec<(u32, u32)>,
    /// Function symbol → exception table label.
    lsda: Vec<(u32, u32)>,
}

impl Collected {
    fn id(&mut self, name: &str) -> u32 {
        if let Some(&id) = self.ids.get(name) {
            return id;
        }
        let id = u32::try_from(self.names.len()).unwrap_or(u32::MAX);
        self.ids.insert(name.to_owned(), id);
        self.names.push(name.to_owned());
        id
    }

    /// A section change: whatever constant was open is complete.
    pub(crate) fn section(&mut self) {
        self.current = None;
    }

    /// A label in a constant or unwind section opens a constant — or, arriving
    /// before the last one saw any bytes, aliases it. In an exception table
    /// only the table's own labels count; the ones marking its parts do not.
    pub(crate) fn label(&mut self, label: &str, section: Section) {
        if section == Section::Unwind && !label.contains("except") {
            return;
        }
        let id = self.id(label);
        if let Some(index) = self.current
            && !self.constants[index].touched
            && self.constants[index].section == section
        {
            self.by_label.insert(id, index);
            return;
        }

        let index = self.constants.len();
        self.constants.push(Constant::new(id, section));
        self.by_label.insert(id, index);
        self.current = Some(index);
    }

    /// A data directive under the open constant.
    pub(crate) fn directive(&mut self, name: &str, arguments: &str, word: u64) {
        let Some(index) = self.current else { return };
        let mut slots = Vec::new();
        {
            let constant = &mut self.constants[index];
            constant.touched = true;
            match name {
                "ascii" | "asciz" => {
                    for chunk in quoted(arguments) {
                        let decoded = decode_quoted(chunk);
                        constant.text_ok &= is_text(&decoded);
                        if constant.text_ok
                            && constant
                                .preview
                                .as_ref()
                                .is_none_or(|preview| preview.chars().count() < PREVIEW_CHARS)
                        {
                            let text = String::from_utf8_lossy(&decoded);
                            let mut preview =
                                constant.preview.take().map(String::from).unwrap_or_default();
                            preview.push_str(&text);
                            constant.preview = Some(preview.into_boxed_str());
                        }
                        constant.push_head(&decoded);
                        constant.text += decoded.len() as u64;
                        constant.bytes += decoded.len() as u64;
                        if name == "asciz" {
                            constant.push_head(&[0]);
                            constant.bytes += 1;
                        }
                    }
                }
                "space" | "zero" | "fill" => {
                    let mut numbers = arguments.split(',').map(str::trim);
                    let count = numbers.next().and_then(|n| n.parse::<u64>().ok()).unwrap_or(0);
                    let (size, fill) = if name == "fill" {
                        let size = numbers.next().and_then(|n| n.parse::<u64>().ok()).unwrap_or(1);
                        (size, numbers.next().and_then(parse_int).unwrap_or(0))
                    } else {
                        (1, numbers.next().and_then(parse_int).unwrap_or(0))
                    };
                    let bytes = count.saturating_mul(size);
                    let fill = vec![fill as u8; bytes.min(HEAD as u64) as usize];
                    constant.push_head(&fill);
                    constant.bytes += bytes;
                }
                // Not knowable from the text: one byte is the floor.
                "uleb128" | "sleb128" => constant.bytes += 1,
                _ => {
                    // An integer directive: `.byte 1, 2`, `.quad label+8`.
                    if word == 0 {
                        return;
                    }
                    for value in arguments.split(',').map(str::trim) {
                        let offset = constant.bytes;
                        constant.bytes += word;
                        match parse_int(value) {
                            Some(number) => {
                                let bytes = number.to_le_bytes();
                                constant.push_head(&bytes[..word as usize]);
                            }
                            None => {
                                constant.push_head(&[0; 8][..word as usize]);
                                if word == 8
                                    && let Some(target) = target_of(value)
                                {
                                    slots.push((offset, target.to_owned()));
                                }
                            }
                        }
                    }
                }
            }
        }
        for (offset, target) in slots {
            let target = self.id(&target);
            self.constants[index].slots.push((offset as u32, target));
        }
    }

    /// ELF's `.size label, N`: the assembler's own count wins.
    pub(crate) fn size(&mut self, arguments: &str) {
        let Some((label, size)) = arguments.split_once(',') else { return };
        let Ok(size) = size.trim().parse::<u64>() else { return };
        let id = self.id(label.trim());
        if let Some(&index) = self.by_label.get(&id) {
            self.constants[index].bytes = size;
        }
    }

    /// `function` loads the constant at `label`.
    pub(crate) fn reference(&mut self, function: &str, label: &str) {
        let edge = (self.id(function), self.id(label));
        self.references.push(edge);
    }

    /// `function` loads the constant at `label` on the way to a panic.
    pub(crate) fn panic_reference(&mut self, function: &str, label: &str) {
        let edge = (self.id(function), self.id(label));
        self.panic_references.push(edge);
    }

    /// `function`'s exception table is at `label`.
    pub(crate) fn lsda(&mut self, function: &str, label: &str) {
        let edge = (self.id(function), self.id(label));
        self.lsda.push(edge);
    }

    /// One `.s` file is done: its assembler-local names must not collide with
    /// the next file's.
    pub(crate) fn end_file(&mut self) {
        self.current = None;
        self.ids.clear();
    }
}

/// The quoted strings in a directive's arguments, without their quotes.
fn quoted(arguments: &str) -> impl Iterator<Item = &str> {
    let mut rest = arguments;
    std::iter::from_fn(move || {
        let start = rest.find('"')? + 1;
        let mut end = start;
        let bytes = rest.as_bytes();
        while end < bytes.len() {
            match bytes[end] {
                b'\\' => end += 2,
                b'"' => break,
                _ => end += 1,
            }
        }
        let end = end.min(bytes.len());
        let chunk = &rest[start..end];
        rest = &rest[(end + 1).min(rest.len())..];
        Some(chunk)
    })
}

/// The bytes of a quoted string as LLVM writes it: `\"`, `\\`, `\b`, `\f`,
/// `\n`, `\r`, `\t`, and up to three octal digits for everything else.
pub(crate) fn decode_quoted(text: &str) -> Vec<u8> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'\\' {
            out.push(bytes[i]);
            i += 1;
            continue;
        }
        i += 1;
        let Some(&next) = bytes.get(i) else { break };
        match next {
            b'0'..=b'7' => {
                let mut value: u32 = 0;
                let mut digits = 0;
                while digits < 3 && i < bytes.len() && (b'0'..=b'7').contains(&bytes[i]) {
                    value = value * 8 + u32::from(bytes[i] - b'0');
                    i += 1;
                    digits += 1;
                }
                out.push(value as u8);
            }
            b'b' => {
                out.push(8);
                i += 1;
            }
            b'f' => {
                out.push(12);
                i += 1;
            }
            b'n' => {
                out.push(b'\n');
                i += 1;
            }
            b'r' => {
                out.push(b'\r');
                i += 1;
            }
            b't' => {
                out.push(b'\t');
                i += 1;
            }
            other => {
                out.push(other);
                i += 1;
            }
        }
    }
    out
}

/// Whether bytes read as text: valid UTF-8 with no control characters beyond
/// whitespace.
fn is_text(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes)
        .is_ok_and(|text| text.chars().all(|c| !c.is_control() || matches!(c, '\n' | '\t' | '\r')))
}

/// An integer operand: decimal, negative, or `0x` hex.
fn parse_int(value: &str) -> Option<u64> {
    let value = value.trim();
    if let Some(hex) = value.strip_prefix("0x").or_else(|| value.strip_prefix("0X")) {
        return u64::from_str_radix(hex, 16).ok();
    }
    if let Some(negative) = value.strip_prefix('-') {
        return negative.parse::<u64>().ok().map(u64::wrapping_neg);
    }
    value.parse::<u64>().ok()
}

/// The label or symbol a pointer-sized operand names, without a `+offset`
/// or relocation suffix.
fn target_of(value: &str) -> Option<&str> {
    let end = value.find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '$')));
    let target = &value[..end.unwrap_or(value.len())];
    (!target.is_empty() && !target.starts_with(|c: char| c.is_ascii_digit())).then_some(target)
}

/// Read the collected constants. `sizes` maps a code symbol's raw label to its
/// bytes, as `symbols::code_sizes` produces; `workspace` is the workspace root.
pub(crate) fn analyze(
    collected: Collected,
    sizes: &FxHashMap<String, u64>,
    workspace: &Path,
    limit: usize,
) -> ConstantsReport {
    let Collected { names, constants, by_label, references, panic_references, lsda, .. } =
        collected;
    let is_code = |id: u32| sizes.contains_key(&names[id as usize]);

    // What a linked function reaches: its references, and every constant a
    // reached constant points at.
    let mut linked: Vec<bool> = vec![false; constants.len()];
    let mut queue: Vec<usize> = Vec::new();
    for &(_, label) in &references {
        if let Some(&index) = by_label.get(&label)
            && !std::mem::replace(&mut linked[index], true)
        {
            queue.push(index);
        }
    }
    while let Some(index) = queue.pop() {
        for &(_, target) in &constants[index].slots {
            if let Some(&next) = by_label.get(&target)
                && !std::mem::replace(&mut linked[next], true)
            {
                queue.push(next);
            }
        }
    }

    // Classify, resolving a location record's path through its slot.
    let kinds: Vec<Kind> = constants
        .iter()
        .map(|constant| classify(constant, &constants, &by_label, &names, &is_code))
        .collect();

    // References, deduplicated: which functions load each constant directly,
    // and which reach it at all — through a table's slots, a location record's
    // path pointer — for attributing what a function keeps alive.
    let mut references = references;
    references.sort_unstable();
    references.dedup();
    let mut loaders: FxHashMap<usize, Vec<u32>> = FxHashMap::default();
    for &(function, label) in &references {
        if let Some(&index) = by_label.get(&label) {
            loaders.entry(index).or_default().push(function);
        }
    }
    let reaching = reaching(&constants, &by_label, &loaders);

    let mut classes: FxHashMap<Kind, (u64, usize)> = FxHashMap::default();
    let mut bytes = 0;
    let mut linked_count = 0;
    let defined = constants.iter().filter(|constant| constant.section != Section::Unwind).count();
    for (index, constant) in constants.iter().enumerate() {
        if !linked[index] || constant.section == Section::Unwind {
            continue;
        }
        linked_count += 1;
        bytes += constant.bytes;
        let entry = classes.entry(kinds[index]).or_default();
        entry.0 += constant.bytes;
        entry.1 += 1;
    }
    let mut classes: Vec<ConstantClass> = classes
        .into_iter()
        .map(|(kind, (bytes, count))| ConstantClass { kind, bytes, count })
        .collect();
    classes.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.kind.cmp(&b.kind)));

    let locations =
        locations(&constants, &kinds, &linked, &by_label, &loaders, &names, workspace, limit);
    let panic_messages = panic_messages(&constants, &kinds, &by_label, panic_references);
    let strings = strings(&constants, &kinds, &linked, &reaching, &names, limit);
    let tables = tables(&constants, &kinds, &linked, &loaders, &names, limit);
    let functions = carriers(&constants, &linked, &reaching, &names, limit);
    let unwind = unwind(&constants, &by_label, &names, lsda, limit);

    ConstantsReport {
        constants: defined,
        linked: linked_count,
        bytes,
        classes,
        locations,
        panic_messages,
        strings,
        tables,
        functions,
        unwind,
    }
}

/// For each constant, every function that reaches it: its direct loaders and,
/// following pointer slots, the loaders of whatever points at it.
fn reaching(
    constants: &[Constant],
    by_label: &FxHashMap<u32, usize>,
    loaders: &FxHashMap<usize, Vec<u32>>,
) -> Vec<Vec<u32>> {
    let mut by_function: FxHashMap<u32, Vec<usize>> = FxHashMap::default();
    for (&index, functions) in loaders {
        for &function in functions {
            by_function.entry(function).or_default().push(index);
        }
    }

    let mut reaching: Vec<Vec<u32>> = vec![Vec::new(); constants.len()];
    let mut seen: FxHashSet<usize> = FxHashSet::default();
    let mut stack: Vec<usize> = Vec::new();
    for (function, roots) in by_function {
        seen.clear();
        stack.extend(roots);
        while let Some(index) = stack.pop() {
            if !seen.insert(index) {
                continue;
            }
            reaching[index].push(function);
            for &(_, target) in &constants[index].slots {
                if let Some(&next) = by_label.get(&target) {
                    stack.push(next);
                }
            }
        }
    }
    for functions in &mut reaching {
        functions.sort_unstable();
    }
    reaching
}

/// What a constant is, from its bytes and what it points at.
fn classify(
    constant: &Constant,
    constants: &[Constant],
    by_label: &FxHashMap<u32, usize>,
    names: &[String],
    is_code: &dyn Fn(u32) -> bool,
) -> Kind {
    if constant.section == Section::Unwind {
        return Kind::Lsda;
    }
    let label = &names[constant.label as usize];
    if label.contains("JTI") {
        return Kind::JumpTable;
    }
    if label.contains("switch.table") {
        return Kind::SwitchTable;
    }

    let text = |id: u32| by_label.get(&id).map(|&index| &constants[index]).filter(|c| is_string(c));
    let text_kind = |c: &Constant| classify_text(c.preview.as_deref().unwrap_or_default(), c.text);

    if constant.slots.is_empty() {
        if is_string(constant) {
            return text_kind(constant);
        }
        return Kind::Bytes;
    }

    // A location record: a source path pointer, its length, a line, and a
    // column, both counted from 1.
    if constant.bytes == 24
        && constant.slots.len() == 1
        && constant.slots[0].0 == 0
        && let Some(path) = text(constant.slots[0].1)
        && text_kind(path) == Kind::Path
        && constant.u64_at(8) == Some(path.text)
        && constant.u32_at(16).is_some_and(|line| line >= 1)
        && constant.u32_at(20).is_some_and(|column| (1..65_536).contains(&column))
    {
        return Kind::Location;
    }

    // A vtable: drop glue or none, size, a power-of-two align, then methods.
    if constant.bytes >= 32
        && constant.slots.iter().any(|&(offset, target)| offset >= 24 && is_code(target))
        && constant.slots.iter().all(|&(offset, _)| offset == 0 || offset >= 24)
        && constant.u64_at(16).is_some_and(|align| align.is_power_of_two() && align <= 4096)
    {
        let drop = constant.slots.iter().find(|&&(offset, _)| offset == 0);
        let dropless = drop.is_none() && constant.u64_at(0) == Some(0);
        let drops = drop.is_some_and(|&(_, target)| {
            let name = demangle(&names[target as usize]);
            name.contains("drop_in_place") || name.contains("drop_glue")
        });
        if dropless || drops {
            return Kind::Vtable;
        }
    }

    // A table: records of one stride, each with a pointer at the same place.
    // Every 16 bytes to strings is `&[&str]`; every 8 to code, `[fn; N]`;
    // anything else regular, a table of slices or records.
    let first = u64::from(constant.slots[0].0);
    let stride =
        constant.slots.get(1).map_or(constant.bytes, |&(offset, _)| u64::from(offset) - first);
    let regular = stride >= 8
        && constant
            .slots
            .iter()
            .enumerate()
            .all(|(n, &(offset, _))| u64::from(offset) == first + n as u64 * stride)
        && constant.slots.len() as u64 * stride == constant.bytes;
    if regular {
        if stride == 16
            && first == 0
            && constant.slots.iter().all(|&(_, target)| text(target).is_some())
        {
            return Kind::Pieces;
        }
        if stride == 8 && constant.slots.iter().all(|&(_, target)| is_code(target)) {
            return Kind::PointerTable;
        }
        return Kind::SliceTable;
    }

    Kind::Other
}

/// A constant that is one string: only text directives, all of them readable.
fn is_string(constant: &Constant) -> bool {
    constant.text > 0
        && constant.text_ok
        && constant.slots.is_empty()
        && constant.bytes <= constant.text + 1
}

/// Which kind of text a string is: a source path, a bare name, or a message.
pub(crate) fn classify_text(preview: &str, length: u64) -> Kind {
    if preview.contains('/') && (preview.ends_with(".rs") || preview.contains("/library/"))
        || preview.starts_with("/rustc/")
        || preview.starts_with("/rust/deps/")
        || preview.starts_with("library/")
    {
        // The whole path is kept, so the ending is trustworthy.
        return Kind::Path;
    }
    let complete = u64::try_from(preview.chars().count()).is_ok_and(|shown| shown == length)
        || preview.len() as u64 == length;
    let name = complete
        && preview.starts_with(|c: char| c.is_alphabetic() || c == '_' || c == '<' || c == '&')
        && preview.chars().all(|c| {
            c.is_alphanumeric()
                || matches!(
                    c,
                    '_' | ':' | '<' | '>' | '&' | '\'' | '[' | ']' | ',' | '(' | ')' | '*' | '.'
                )
        });
    if name { Kind::Name } else { Kind::Message }
}

/// Panic locations by file and by function.
#[expect(clippy::too_many_arguments, reason = "one pass over the shared tables")]
fn locations(
    constants: &[Constant],
    kinds: &[Kind],
    linked: &[bool],
    by_label: &FxHashMap<u32, usize>,
    loaders: &FxHashMap<usize, Vec<u32>>,
    names: &[String],
    workspace: &Path,
    limit: usize,
) -> Locations {
    let mut records = 0;
    let mut files: FxHashMap<usize, (usize, FxHashMap<u32, usize>)> = FxHashMap::default();
    let mut callers: FxHashMap<u32, usize> = FxHashMap::default();
    for (index, constant) in constants.iter().enumerate() {
        if !linked[index] || kinds[index] != Kind::Location {
            continue;
        }
        let Some(&path) = by_label.get(&constant.slots[0].1) else { continue };
        records += 1;
        let file = files.entry(path).or_default();
        file.0 += 1;
        *file.1.entry(constant.u32_at(16).unwrap_or(0)).or_default() += 1;
        for &function in loaders.get(&index).into_iter().flatten() {
            *callers.entry(function).or_default() += 1;
        }
    }

    let mut all: Vec<(LocationFile, Origin)> = files
        .into_iter()
        .map(|(path, (count, lines))| {
            let text = constants[path].preview.as_deref().unwrap_or_default();
            let (file, origin) = source(text, workspace);
            let mut lines: Vec<(u32, usize)> = lines.into_iter().collect();
            lines.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            lines.truncate(LINES_PER_FILE);
            let bytes = 24 * count as u64 + constants[path].bytes;
            (
                LocationFile {
                    file,
                    records: count,
                    bytes,
                    lines: lines.into_iter().map(|(line, _)| u64::from(line)).collect(),
                },
                origin,
            )
        })
        .collect();
    all.sort_by(|(a, _), (b, _)| b.records.cmp(&a.records).then_with(|| a.file.cmp(&b.file)));
    let bytes = all.iter().map(|(file, _)| file.bytes).sum();

    let workspace_files: Vec<LocationFile> = all
        .iter()
        .filter(|(_, origin)| *origin == Origin::Workspace)
        .take(limit)
        .map(|(file, _)| file.clone())
        .collect();
    all.truncate(limit);

    let mut functions: Vec<LocationCaller> = callers
        .into_iter()
        .map(|(function, count)| LocationCaller {
            name: demangle(&names[function as usize]),
            records: count,
            bytes: 24 * count as u64,
        })
        .collect();
    functions.sort_by(|a, b| b.records.cmp(&a.records).then_with(|| a.name.cmp(&b.name)));
    functions.truncate(limit);

    Locations {
        records,
        bytes,
        files: all.into_iter().map(|(file, _)| file).collect(),
        workspace_files,
        functions,
    }
}

/// Text loaded only on the way to a panic: messages and their pieces.
fn panic_messages(
    constants: &[Constant],
    kinds: &[Kind],
    by_label: &FxHashMap<u32, usize>,
    panic_references: Vec<(u32, u32)>,
) -> ConstantClass {
    let mut seen: FxHashSet<usize> = FxHashSet::default();
    let mut bytes = 0;
    let mut count = 0;
    let mut text = |index: usize, seen: &mut FxHashSet<usize>| {
        if matches!(kinds[index], Kind::Message | Kind::Name | Kind::Pieces) && seen.insert(index) {
            bytes += constants[index].bytes;
            count += 1;
        }
    };
    for (_, label) in panic_references {
        let Some(&index) = by_label.get(&label) else { continue };
        text(index, &mut seen);
        // The pieces of a formatted message point at their strings.
        if kinds[index] == Kind::Pieces {
            for &(_, target) in &constants[index].slots {
                if let Some(&piece) = by_label.get(&target) {
                    text(piece, &mut seen);
                }
            }
        }
    }
    ConstantClass { kind: Kind::Message, bytes, count }
}

/// The largest strings, with who loads them.
fn strings(
    constants: &[Constant],
    kinds: &[Kind],
    linked: &[bool],
    reaching: &[Vec<u32>],
    names: &[String],
    limit: usize,
) -> Vec<StringConstant> {
    let mut strings: Vec<(usize, &Constant)> = constants
        .iter()
        .enumerate()
        .filter(|&(index, _)| linked[index] && matches!(kinds[index], Kind::Name | Kind::Message))
        .collect();
    strings.sort_by(|(a, x), (b, y)| y.bytes.cmp(&x.bytes).then_with(|| a.cmp(b)));
    strings.truncate(limit);

    strings
        .into_iter()
        .map(|(index, constant)| {
            let functions = reaching[index].as_slice();
            StringConstant {
                bytes: constant.bytes,
                kind: kinds[index],
                preview: preview_of(constant),
                functions: functions
                    .iter()
                    .take(FUNCTIONS_PER_STRING)
                    .map(|&function| demangle(&names[function as usize]))
                    .collect(),
                references: functions.len(),
            }
        })
        .collect()
}

/// A string's preview, cut to length with an ellipsis and control characters
/// escaped.
fn preview_of(constant: &Constant) -> String {
    let text = constant.preview.as_deref().unwrap_or_default();
    let mut out = String::new();
    for c in text.chars().take(PREVIEW_CHARS) {
        match c {
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c.is_control() => out.extend(c.escape_default()),
            c => out.push(c),
        }
    }
    if text.chars().count() > PREVIEW_CHARS || (text.len() as u64) < constant.text {
        out.push('\u{2026}');
    }
    out
}

/// Functions by their lookup and jump tables.
fn tables(
    constants: &[Constant],
    kinds: &[Kind],
    linked: &[bool],
    loaders: &FxHashMap<usize, Vec<u32>>,
    names: &[String],
    limit: usize,
) -> Vec<FunctionTables> {
    let mut by_function: FxHashMap<String, FunctionTables> = FxHashMap::default();
    for (index, constant) in constants.iter().enumerate() {
        if !linked[index] || !matches!(kinds[index], Kind::SwitchTable | Kind::JumpTable) {
            continue;
        }
        // A switch table is named for its function; a jump table for the
        // function that loads it.
        let owner = if kinds[index] == Kind::SwitchTable {
            switch_table_owner(&names[constant.label as usize])
        } else {
            loaders
                .get(&index)
                .and_then(|functions| functions.first())
                .map(|&function| demangle(&names[function as usize]))
        };
        let Some(owner) = owner else { continue };
        let entry = by_function.entry(owner.clone()).or_insert_with(|| FunctionTables {
            name: owner,
            bytes: 0,
            switch_tables: 0,
            jump_tables: 0,
        });
        entry.bytes += constant.bytes;
        if kinds[index] == Kind::SwitchTable {
            entry.switch_tables += 1;
        } else {
            entry.jump_tables += 1;
        }
    }

    let mut tables: Vec<FunctionTables> = by_function.into_values().collect();
    tables.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
    tables.truncate(limit);
    tables
}

/// The function a `switch.table.<mangled>[.N]` label was built for.
pub(crate) fn switch_table_owner(label: &str) -> Option<String> {
    let (_, rest) = label.split_once("switch.table.")?;
    // LLVM numbers a function's second table `.1`, `.2`, …
    let mangled = match rest.rsplit_once('.') {
        Some((head, tail)) if tail.chars().all(|c| c.is_ascii_digit()) => head,
        _ => rest,
    };
    let demangled = demangle(mangled);
    (demangled != mangled || mangled.contains("::")).then_some(demangled)
}

/// Functions by the constant bytes they reach.
fn carriers(
    constants: &[Constant],
    linked: &[bool],
    reaching: &[Vec<u32>],
    names: &[String],
    limit: usize,
) -> Vec<ConstantCarrier> {
    let mut by_function: FxHashMap<u32, (u64, u64, usize)> = FxHashMap::default();
    for (index, constant) in constants.iter().enumerate() {
        if !linked[index] || constant.section == Section::Unwind {
            continue;
        }
        let functions = &reaching[index];
        for &function in functions {
            let entry = by_function.entry(function).or_default();
            entry.0 += constant.bytes;
            if functions.len() == 1 {
                entry.1 += constant.bytes;
            }
            entry.2 += 1;
        }
    }

    let mut carriers: Vec<ConstantCarrier> = by_function
        .into_iter()
        .map(|(function, (bytes, exclusive, count))| ConstantCarrier {
            name: demangle(&names[function as usize]),
            bytes,
            exclusive,
            constants: count,
        })
        .collect();
    carriers.sort_by(|a, b| {
        b.exclusive
            .cmp(&a.exclusive)
            .then_with(|| b.bytes.cmp(&a.bytes))
            .then_with(|| a.name.cmp(&b.name))
    });
    carriers.truncate(limit);
    carriers
}

/// Functions by the size of their exception tables.
fn unwind(
    constants: &[Constant],
    by_label: &FxHashMap<u32, usize>,
    names: &[String],
    lsda: Vec<(u32, u32)>,
    limit: usize,
) -> Vec<UnwindTable> {
    let mut tables: Vec<UnwindTable> = lsda
        .into_iter()
        .filter_map(|(function, label)| {
            let constant = &constants[*by_label.get(&label)?];
            (constant.bytes > 0).then(|| UnwindTable {
                name: demangle(&names[function as usize]),
                bytes: constant.bytes,
            })
        })
        .collect();
    tables.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
    tables.truncate(limit);
    tables
}

#[cfg(test)]
mod tests {
    use super::{Kind, classify_text, decode_quoted, quoted, switch_table_owner};

    #[test]
    fn decodes_llvm_quoted_strings() {
        assert_eq!(decode_quoted("src/main.rs"), b"src/main.rs");
        assert_eq!(
            decode_quoted("\\013\\000\\000\\000\\000\\000\\000\\000(\\000\\000\\000&\\000\\000"),
            [11, 0, 0, 0, 0, 0, 0, 0, b'(', 0, 0, 0, b'&', 0, 0]
        );
        assert_eq!(decode_quoted("a\\tb\\n\\\"q\\\"\\\\ \\303\\251"), "a\tb\n\"q\"\\ é".as_bytes());
        // Two strings in one directive.
        assert_eq!(quoted("\"ab\", \"c\\\"d\"").collect::<Vec<_>>(), ["ab", "c\\\"d"]);
    }

    #[test]
    fn tells_paths_names_and_messages_apart() {
        assert_eq!(classify_text("src/main.rs", 11), Kind::Path);
        assert_eq!(classify_text("/rustc/abc/library/core/src/fmt/mod.rs", 39), Kind::Path);
        assert_eq!(classify_text("Circle", 6), Kind::Name);
        assert_eq!(classify_text("alloc::vec::Vec<u8>", 19), Kind::Name);
        assert_eq!(classify_text("index must be a number", 22), Kind::Message);
        assert_eq!(classify_text("called `Option::unwrap()` on a `None` value", 44), Kind::Message);
        // A name whose text was cut short cannot be told from a message.
        assert_eq!(classify_text("Circle", 100), Kind::Message);
    }

    #[test]
    fn names_the_function_behind_a_switch_table() {
        assert_eq!(
            switch_table_owner("l_switch.table._ZN5probe4main17h0123456789abcdefE.1").as_deref(),
            Some("probe::main")
        );
        assert_eq!(
            switch_table_owner(".Lswitch.table._ZN5probe4main17h0123456789abcdefE").as_deref(),
            Some("probe::main")
        );
        assert_eq!(switch_table_owner("l_anon.abc.1"), None);
    }
}
