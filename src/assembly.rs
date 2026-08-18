//! Read the assembly the compiler emitted, the way `cargo asm` does, for the
//! shapes a symbol table cannot show.
//!
//! Symbols give each function a size; the assembly says what is in it. Two
//! functions carrying the same body under different names, the compare and cold
//! call that guard every index, a run of loads and stores dragging a large
//! value through memory, the source line each instruction was compiled from —
//! none of that is visible from sizes, and it is what a person finds by reading
//! `cargo asm` output one function at a time.
//!
//! Only functions that reached the linked binary are counted, matched by mangled
//! name, so instructions and bytes here reconcile with the symbol view. Under
//! `lto = "fat"` the final crate's assembly is the whole program after LTO;
//! without it, only that crate's own code, and the coverage line says so.
//!
//! The same pass feeds two other views as it streams: the reference graph
//! (`graph`) from every call, taken address, and pointer slot, and the constant
//! data view (`constants`) from every directive under a label in a constant
//! section. Debug-info sections, most of the file by far, are skipped.

use std::{
    fs::File,
    hash::Hasher,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use object::{Architecture, Object};
use rustc_hash::{FxHashMap, FxHashSet, FxHasher};
use serde::Serialize;

use crate::{
    constants::{self, Collected},
    graph::Edges,
    inlined::normalize,
    name::demangle,
    sections::Category,
    symbols::code_sizes,
};

/// Loads and stores back to back from this many on are taken to be one value
/// moving through memory rather than ordinary field access.
pub(crate) const COPY_RUN: usize = 8;

#[derive(Debug, Serialize)]
pub struct AssemblyReport {
    pub paths: Vec<String>,

    /// Functions in the assembly.
    pub functions: usize,

    /// Of those, the functions with a symbol in the binary. Everything below
    /// counts only these.
    pub linked: usize,

    pub instructions: u64,

    /// Bytes the linked functions occupy, from the symbol table.
    pub bytes: u64,

    pub identical: Identical,
    pub panics: Panics,
    pub formatting: Formatting,
    pub copies: Copies,

    /// The source lines the most instructions were compiled from, after
    /// inlining and across every instantiation.
    pub lines: Vec<Line>,

    /// The same, for lines in this workspace rather than in std or a dependency.
    pub workspace_lines: Vec<Line>,
}

/// Functions whose bodies are the same instructions, one for one, once local
/// labels are renamed. A linker folding identical code keeps one of each group;
/// so does not instantiating the others.
#[derive(Debug, Serialize)]
pub struct Identical {
    pub groups: usize,
    pub functions: usize,

    /// Bytes folding would drop: every copy in a group but one.
    pub recoverable: u64,

    pub largest: Vec<IdenticalGroup>,
}

#[derive(Debug, Serialize)]
pub struct IdenticalGroup {
    pub names: Vec<String>,
    pub instructions: u64,

    /// Bytes of one copy.
    pub bytes: u64,

    pub recoverable: u64,
}

/// Calls into the panic machinery. Each is a compare, a branch, and a cold block
/// that loads a source location and calls; the location is another 24 bytes of
/// read-only data.
#[derive(Debug, Serialize)]
pub struct Panics {
    pub sites: usize,
    pub bounds_checks: usize,
    pub unwraps: usize,
    pub allocation: usize,
    pub other: usize,

    /// Instructions in the blocks those calls end.
    pub instructions: u64,

    /// Distinct anonymous constants those blocks load: the locations and
    /// messages they pass.
    pub constants: usize,

    pub functions: Vec<Caller>,
}

/// Calls into `core::fmt` and `alloc::fmt`. The block before each one builds
/// the `Arguments`.
#[derive(Debug, Serialize)]
pub struct Formatting {
    pub sites: usize,
    pub instructions: u64,
    pub functions: Vec<Caller>,
}

#[derive(Debug, Serialize)]
pub struct Caller {
    pub name: String,
    pub sites: usize,

    /// Instructions in the blocks ending in those calls.
    pub instructions: u64,
}

/// Values moved through memory: runs of back-to-back loads and stores, and
/// calls to `memcpy` and friends for anything too large to unroll.
#[derive(Debug, Serialize)]
pub struct Copies {
    pub runs: usize,
    pub instructions: u64,
    pub calls: usize,
    pub functions: Vec<Copier>,
}

#[derive(Debug, Serialize)]
pub struct Copier {
    pub name: String,
    pub runs: usize,
    pub instructions: u64,
    pub calls: usize,
}

#[derive(Debug, Serialize)]
pub struct Line {
    pub file: String,
    pub line: u64,
    pub instructions: u64,

    /// The line's source text, for lines in this workspace.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
}

/// What one pass over the assembly yields: the report, plus the reference
/// graph and symbol sizes the graph analysis reads.
pub struct Analysis {
    pub report: AssemblyReport,
    pub(crate) edges: Edges,
    pub(crate) constants: Collected,
    pub(crate) sizes: FxHashMap<String, u64>,
}

/// Rank what the assembly in `paths` shows, keeping the `limit` largest of each
/// list. `binary` is the linked executable the assembly was compiled into, and
/// `workspace` its workspace root.
///
/// # Errors
///
/// Errors when there is no assembly or it cannot be read.
pub fn analyze(
    binary: &object::File<'_>,
    paths: &[PathBuf],
    workspace: &Path,
    limit: usize,
) -> Result<Analysis> {
    if paths.is_empty() {
        bail!("the build produced no assembly");
    }

    let sizes = code_sizes(binary);
    let mut parser = Parser::new(Arch::of(binary.architecture()), &sizes, workspace);
    for path in paths {
        let file =
            File::open(path).with_context(|| format!("failed to read {}", path.display()))?;
        parser
            .parse(BufReader::new(file))
            .with_context(|| format!("failed to read {}", path.display()))?;
    }

    let (report, edges, constants) = parser.report(paths, limit);
    Ok(Analysis { report, edges, constants, sizes })
}

/// What the parser gathers about one function.
struct Function {
    name: String,

    /// The raw label, the spelling the graph and the symbol table join on.
    symbol: String,

    bytes: u64,
    instructions: u64,

    /// Hash of the body with local labels renamed, so identical code hashes
    /// alike whatever it was called.
    hasher: FxHasher,
    labels: FxHashMap<String, usize>,

    /// Constant pools and jump tables the body refers to. Their contents are
    /// part of the body: two functions differing only in a constant are not
    /// the same code.
    data: Vec<String>,

    /// Instructions since the last label, branch, or call: the block a call
    /// site's setup lives in. A block reached only by falling through has no
    /// label, so the branch before it has to count as a boundary too.
    block: u64,

    /// Anonymous constants the current block loads.
    block_constants: Vec<String>,

    /// Loads and stores in a row so far.
    run: usize,

    panics: [usize; 4],
    panic_instructions: u64,
    formatting: usize,
    formatting_instructions: u64,
    copy_runs: usize,
    copy_instructions: u64,
    copy_calls: usize,

    /// The label of its exception table, from `.cfi_lsda`.
    lsda: Option<String>,
}

impl Function {
    fn new(name: String, symbol: String, bytes: u64) -> Self {
        Self {
            name,
            symbol,
            bytes,
            instructions: 0,
            hasher: FxHasher::default(),
            labels: FxHashMap::default(),
            data: Vec::new(),
            block: 0,
            block_constants: Vec::new(),
            run: 0,
            panics: [0; 4],
            panic_instructions: 0,
            formatting: 0,
            formatting_instructions: 0,
            copy_runs: 0,
            copy_instructions: 0,
            copy_calls: 0,
            lsda: None,
        }
    }

    fn new_block(&mut self) {
        self.block = 0;
        self.block_constants.clear();
    }

    /// A label or a non-move instruction ends a run of loads and stores.
    const fn end_run(&mut self) {
        if self.run >= COPY_RUN {
            self.copy_runs += 1;
            self.copy_instructions += self.run as u64;
        }
        self.run = 0;
    }

    /// Feed `text` to the body hash with its local labels renamed.
    fn hash(&mut self, text: &str) {
        let mut normalized = String::with_capacity(text.len());
        for piece in pieces(text) {
            match piece {
                Piece::Local(label) => {
                    let next = self.labels.len();
                    let ordinal = *self.labels.entry(label.to_owned()).or_insert(next);
                    normalized.push_str("L#");
                    normalized.push_str(&ordinal.to_string());
                }
                Piece::Other(other) => normalized.push_str(other),
            }
        }

        self.hasher.write(normalized.as_bytes());
        self.hasher.write_u8(b'\n');
    }
}

/// A finished, linked function.
struct Linked {
    function: Function,
    hash: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Arch {
    Aarch64,
    X86,
    Other,
}

impl Arch {
    const fn of(architecture: Architecture) -> Self {
        match architecture {
            Architecture::Aarch64 | Architecture::Aarch64_Ilp32 => Self::Aarch64,
            Architecture::X86_64 | Architecture::X86_64_X32 | Architecture::I386 => Self::X86,
            _ => Self::Other,
        }
    }
}

/// Where a source path is: what the reader can edit, a dependency, or std.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Origin {
    Workspace,
    Dependency,
    Std,
}

/// What kind of section the parser is in, as far as its readers care.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    Text,
    /// Read-only and relocated-read-only data: the constants view's input.
    Constants,
    /// Exception tables.
    Unwind,
    /// Debug info: skipped as fast as possible — it is most of the file.
    Debug,
    Other,
}

impl Section {
    /// Which section a `.section` directive's arguments name: Mach-O spells
    /// `SEGMENT,section,…`, ELF `name,"flags",@type`.
    fn of(arguments: &str) -> Self {
        if is_text_section(arguments) {
            return Self::Text;
        }
        let mut parts = arguments.split(',').map(str::trim);
        let name = match parts.next() {
            // Everything in the `__DWARF` segment is debug info, the Apple
            // accelerator tables included.
            Some("__DWARF") => return Self::Debug,
            Some(segment) if segment.starts_with("__") => parts.next().unwrap_or(segment),
            Some(name) => name,
            None => return Self::Other,
        };
        Self::named(name)
    }

    /// The section a bare directive (`.text`, `.data`, `.const_data`) or a
    /// section name stands for.
    fn named(name: &str) -> Self {
        // Zero-filled sections have no file bytes to count.
        if matches!(name, "__bss" | "__common" | "__thread_bss" | ".bss" | ".tbss") {
            return Self::Other;
        }
        match Category::of(name) {
            Category::Code => Self::Text,
            Category::ReadOnlyData | Category::Data => Self::Constants,
            Category::Unwind => Self::Unwind,
            Category::Debug => Self::Debug,
            _ => Self::Other,
        }
    }
}

/// Where a call goes, when it is somewhere the report counts.
enum Sink {
    Panic(usize),
    Formatting,
    Copy,
}

const BOUNDS_CHECKS: usize = 0;
const UNWRAPS: usize = 1;
const ALLOCATION: usize = 2;
const OTHER: usize = 3;

struct Parser<'a> {
    arch: Arch,
    sizes: &'a FxHashMap<String, u64>,
    workspace: &'a Path,

    // State that lasts one `.s` file: labels and file numbers are per module.
    files: FxHashMap<u64, (String, Origin)>,
    counts: FxHashMap<(u64, u64), u64>,
    data: FxHashMap<String, Vec<String>>,
    reading: Option<String>,
    location: Option<(u64, u64)>,
    in_text: bool,
    section: Section,
    current: Option<Function>,

    /// The named constant or static the data section is emitting, whose
    /// pointer slots and size the reference graph records.
    data_symbol: Option<String>,

    functions: usize,
    linked: Vec<Linked>,
    lines: FxHashMap<(String, u64), (u64, Origin)>,
    constants: FxHashSet<String>,
    edges: Edges,

    /// Every constant's bytes and shape, for the constants view.
    collected: Collected,
}

impl<'a> Parser<'a> {
    fn new(arch: Arch, sizes: &'a FxHashMap<String, u64>, workspace: &'a Path) -> Self {
        Self {
            arch,
            sizes,
            workspace,
            files: FxHashMap::default(),
            counts: FxHashMap::default(),
            data: FxHashMap::default(),
            reading: None,
            location: None,
            in_text: false,
            section: Section::Other,
            current: None,
            data_symbol: None,
            functions: 0,
            linked: Vec::new(),
            lines: FxHashMap::default(),
            constants: FxHashSet::default(),
            edges: Edges::default(),
            collected: Collected::default(),
        }
    }

    /// Read one `.s` file.
    fn parse(&mut self, mut input: impl BufRead) -> Result<()> {
        let mut line = String::new();
        loop {
            line.clear();
            if input.read_line(&mut line)? == 0 {
                break;
            }
            self.line(line.trim());
        }

        // Everything numbered restarts with the next file.
        self.finish();
        for ((file, number), count) in self.counts.drain() {
            let Some((path, origin)) = self.files.get(&file) else { continue };
            let entry = self.lines.entry((path.clone(), number)).or_insert((0, *origin));
            entry.0 += count;
        }
        self.files.clear();
        self.data.clear();
        self.reading = None;
        self.data_symbol = None;
        self.location = None;
        self.in_text = false;
        self.section = Section::Other;
        self.collected.end_file();

        Ok(())
    }

    fn line(&mut self, text: &str) {
        if text.is_empty() || text.starts_with(['#', ';']) || text.starts_with("//") {
            return;
        }

        // Debug info is most of the file — every DIE, spelled out in `.byte`s
        // — and nothing here reads it; only the directive that leaves the
        // section matters.
        if self.section == Section::Debug && !changes_section(text) {
            return;
        }

        // A label before a directive: an ELF basic-block label is `.LBB0_1:`,
        // which starts with `.` like a directive but must not be read as one.
        // No directive ends in `:`, and a label is a lone `identifier:`.
        if let Some((label, rest)) = text.split_once(':')
            && rest.trim_start().is_empty()
            && is_identifier(label)
        {
            self.label(label);
            return;
        }

        if let Some(directive) = text.strip_prefix('.') {
            self.directive(directive);
            return;
        }

        if self.in_text {
            self.instruction(text);
        }
    }

    fn directive(&mut self, directive: &str) {
        let (name, arguments) =
            directive.split_once(char::is_whitespace).unwrap_or((directive, ""));
        let arguments = arguments.trim();

        match name {
            "section" => self.enter(Section::of(arguments)),
            "text" => self.enter(Section::Text),
            "data" | "rodata" | "const_data" | "const" | "cstring" | "literal4" | "literal8"
            | "literal16" => self.enter(Section::Constants),
            "bss" => self.enter(Section::Other),
            _ if self.section == Section::Debug => {}
            "file" => self.file(arguments),
            "loc" => {
                let mut numbers = arguments.split_whitespace().map_while(|word| word.parse().ok());
                self.location = match (numbers.next(), numbers.next()) {
                    (Some(file), Some(line)) if line > 0 => Some((file, line)),
                    _ => None,
                };
            }
            // Contents of a constant pool or jump table.
            "byte" | "short" | "hword" | "word" | "long" | "quad" | "xword" | "2byte" | "4byte"
            | "8byte" => {
                if let Some(label) = &self.reading {
                    self.data.entry(label.clone()).or_default().push(arguments.to_owned());
                }
                let width = directive_width(self.arch, name);
                if !self.in_text
                    && let Some(symbol) = &self.data_symbol
                {
                    let values = arguments.split(',').map(str::trim);
                    self.edges.data_bytes(symbol, width * values.clone().count() as u64);

                    // Pointer-width values naming a symbol are the slots the
                    // graph reads: a vtable's drop glue and methods.
                    if width == 8 {
                        for value in values {
                            if let Some(target) = symbol_target(value) {
                                self.edges.slot(symbol, target);
                            }
                        }
                    }
                }
                if matches!(self.section, Section::Constants | Section::Unwind) {
                    self.collected.directive(name, arguments, width);
                }
            }
            // A string, zero fill, or LEB128 field: bytes of a constant. A
            // vtable packs its size and align this way, so the graph's data
            // sizes count them too.
            "ascii" | "asciz" | "space" | "zero" | "fill" | "uleb128" | "sleb128" => {
                if !self.in_text
                    && let Some(symbol) = &self.data_symbol
                {
                    let bytes = match name {
                        "ascii" | "asciz" => arguments
                            .split('"')
                            .skip(1)
                            .step_by(2)
                            .map(|chunk| {
                                constants::decode_quoted(chunk).len() as u64
                                    + u64::from(name == "asciz")
                            })
                            .sum(),
                        "uleb128" | "sleb128" => 1,
                        _ => arguments
                            .split(',')
                            .next()
                            .and_then(|count| count.trim().parse::<u64>().ok())
                            .unwrap_or(0),
                    };
                    self.edges.data_bytes(symbol, bytes);
                }
                if matches!(self.section, Section::Constants | Section::Unwind) {
                    self.collected.directive(name, arguments, 0);
                }
            }
            "size" if self.section == Section::Constants => self.collected.size(arguments),
            "cfi_lsda" => {
                if let Some(function) = &mut self.current
                    && let Some((_, label)) = arguments.split_once(',')
                {
                    function.lsda = Some(label.trim().to_owned());
                }
            }
            _ => {}
        }
    }

    /// Enter a section: whatever was being read is done.
    fn enter(&mut self, section: Section) {
        self.reading = None;
        self.data_symbol = None;
        self.in_text = section == Section::Text;
        self.section = section;
        self.collected.section();
    }

    /// `.file N "dir" "name"` or `.file N "path"`, in either DWARF style.
    fn file(&mut self, arguments: &str) {
        let (number, rest) = arguments.split_once(char::is_whitespace).unwrap_or((arguments, ""));
        let Ok(number) = number.parse::<u64>() else { return };

        let mut strings = rest.split('"').skip(1).step_by(2);
        let (Some(first), second) = (strings.next(), strings.next()) else { return };
        let path = match second {
            Some(name) if !name.starts_with('/') && !first.is_empty() => format!("{first}/{name}"),
            Some(name) => name.to_owned(),
            None => first.to_owned(),
        };

        self.files.insert(number, self.source(&path));
    }

    /// Classify a source path and spell it the way the inlined view does.
    fn source(&self, path: &str) -> (String, Origin) {
        source(path, self.workspace)
    }

    fn label(&mut self, label: &str) {
        if !self.in_text {
            // Constant pools precede their function and jump tables follow it;
            // both are named for it, and both are part of its body.
            let pool = label.contains("JTI") || label.contains("CPI");
            self.reading = pool.then(|| label.to_owned());
            // Anything else is a constant or static in its own right — the
            // anonymous vtables live here — whose slots the graph records.
            self.data_symbol = (!pool).then(|| label.to_owned());
            match self.section {
                Section::Constants => self.collected.label(label, constants::Section::Constants),
                Section::Unwind => self.collected.label(label, constants::Section::Unwind),
                _ => {}
            }
            return;
        }

        if is_local(label) {
            // Only basic-block labels are branch targets. `Ltmp` and `Lloh`
            // annotate instructions in place — and how many there are depends
            // on debug locations, so they must not shift the numbering either.
            if let Some(function) = &mut self.current
                && is_block_label(label)
            {
                function.end_run();
                function.new_block();
                let next = function.labels.len();
                function.labels.entry(label.to_owned()).or_insert(next);
            }
            return;
        }

        self.finish();
        self.functions += 1;
        // Every function opens with its own `.loc`; none may inherit the last.
        self.location = None;
        // Only functions the linker kept are counted; the rest are consumed.
        self.current = self
            .sizes
            .get(label)
            .map(|&bytes| Function::new(demangle(label), label.to_owned(), bytes));
    }

    fn instruction(&mut self, text: &str) {
        let Some(function) = &mut self.current else { return };
        let (mnemonic, operands) = text.split_once(char::is_whitespace).unwrap_or((text, ""));
        let operands = operands.trim();

        function.instructions += 1;
        function.block += 1;
        function.hash(text);
        if let Some(location) = self.location {
            *self.counts.entry(location).or_default() += 1;
        }

        for piece in pieces(operands) {
            match piece {
                Piece::Local(label) if label.contains("JTI") || label.contains("CPI") => {
                    if !function.data.iter().any(|known| known == label) {
                        function.data.push(label.to_owned());
                    }
                }
                Piece::Other(word)
                    if word.contains("anon.")
                        || word.contains("__unnamed")
                        || word.contains("switch.table") =>
                {
                    self.collected.reference(&function.symbol, word);
                    function.block_constants.push(word.to_owned());
                }
                Piece::Local(_) | Piece::Other(_) => {}
            }
        }

        if is_move(self.arch, mnemonic, operands) {
            function.run += 1;
            return;
        }
        function.end_run();

        // `rep movs`/`rep stos` is a copy loop in one instruction.
        if matches!(mnemonic, "rep" | "repe" | "repne")
            && (operands.starts_with("movs") || operands.starts_with("stos"))
        {
            function.copy_runs += 1;
            function.copy_instructions += 1;
        }

        if let Some(target) = address_target(self.arch, mnemonic, operands) {
            self.edges.address(&function.symbol, target);
        }

        if let Some(callee) = call_target(mnemonic, operands) {
            self.edges.call(&function.symbol, callee);
            match sink(&demangle(callee)) {
                Some(Sink::Panic(kind)) => {
                    function.panics[kind] += 1;
                    function.panic_instructions += function.block;
                    for label in function.block_constants.drain(..) {
                        self.collected.panic_reference(&function.symbol, &label);
                        self.constants.insert(label);
                    }
                }
                Some(Sink::Formatting) => {
                    function.formatting += 1;
                    function.formatting_instructions += function.block;
                }
                Some(Sink::Copy) => function.copy_calls += 1,
                None => {}
            }
        }

        if transfers_control(self.arch, mnemonic) {
            function.new_block();
        }
    }

    /// Close the function being read, folding in the pools and tables it uses.
    fn finish(&mut self) {
        let Some(mut function) = self.current.take() else { return };
        function.end_run();

        for label in std::mem::take(&mut function.data) {
            for line in self.data.get(&label).into_iter().flatten() {
                function.hash(line);
            }
            // The pools and jump tables are the function's own constants.
            self.collected.reference(&function.symbol, &label);
        }
        if let Some(lsda) = function.lsda.take() {
            self.collected.lsda(&function.symbol, &lsda);
        }

        let hash = function.hasher.finish();
        self.linked.push(Linked { function, hash });
    }

    fn report(self, paths: &[PathBuf], limit: usize) -> (AssemblyReport, Edges, Collected) {
        let functions: Vec<&Function> = self.linked.iter().map(|linked| &linked.function).collect();
        let (lines, workspace_lines) = ranked_lines(self.lines, limit);

        let report = AssemblyReport {
            paths: paths.iter().map(|path| path.display().to_string()).collect(),
            functions: self.functions,
            linked: self.linked.len(),
            instructions: functions.iter().map(|function| function.instructions).sum(),
            bytes: functions.iter().map(|function| function.bytes).sum(),
            identical: identical(&self.linked, limit),
            panics: panics(&functions, self.constants.len(), limit),
            formatting: formatting(&functions, limit),
            copies: copies(&functions, limit),
            lines,
            workspace_lines,
        };
        drop(functions);

        (report, self.edges, self.collected)
    }
}

/// Classify a source path and spell it the way the inlined view does:
/// workspace-relative under `workspace`, otherwise normalized.
pub(crate) fn source(path: &str, workspace: &Path) -> (String, Origin) {
    // Under the workspace root wins: a project whose own path happens to
    // contain `/library/` or `/registry/src/` must not read as std or a
    // dependency.
    if let Ok(rest) = Path::new(path).strip_prefix(workspace) {
        return (rest.display().to_string(), Origin::Workspace);
    }
    if path.contains("/registry/src/") || path.contains("/git/checkouts/") {
        return (normalize(path), Origin::Dependency);
    }
    if path.contains("/library/") || path.contains("/rust/deps/") {
        return (normalize(path), Origin::Std);
    }

    // A relative path with no marker: std and dependencies are absolute
    // here, so this is a workspace file whose directory was left relative.
    (path.to_owned(), Origin::Workspace)
}

/// Identical linked functions, grouped by body hash and instruction count.
fn identical(linked: &[Linked], limit: usize) -> Identical {
    // The instruction count guards the hash.
    let mut groups: FxHashMap<(u64, u64), Vec<&Function>> = FxHashMap::default();
    for linked in linked {
        groups
            .entry((linked.hash, linked.function.instructions))
            .or_default()
            .push(&linked.function);
    }

    let mut identical: Vec<IdenticalGroup> = groups
        .into_values()
        .filter(|group| group.len() > 1)
        .map(|mut group| {
            group.sort_by(|a, b| a.name.cmp(&b.name));
            let bytes = group.iter().map(|function| function.bytes).min().unwrap_or_default();
            IdenticalGroup {
                names: group.iter().map(|function| function.name.clone()).collect(),
                instructions: group[0].instructions,
                bytes,
                recoverable: bytes * (group.len() as u64 - 1),
            }
        })
        .collect();
    identical.sort_by(|a, b| b.recoverable.cmp(&a.recoverable).then_with(|| a.names.cmp(&b.names)));

    Identical {
        groups: identical.len(),
        functions: identical.iter().map(|group| group.names.len()).sum(),
        recoverable: identical.iter().map(|group| group.recoverable).sum(),
        largest: {
            identical.truncate(limit);
            identical
        },
    }
}

fn panics(functions: &[&Function], constants: usize, limit: usize) -> Panics {
    let mut callers: Vec<Caller> = functions
        .iter()
        .filter(|function| function.panics.iter().any(|&count| count > 0))
        .map(|function| Caller {
            name: function.name.clone(),
            sites: function.panics.iter().sum(),
            instructions: function.panic_instructions,
        })
        .collect();
    rank_callers(&mut callers, limit);

    Panics {
        sites: functions.iter().map(|function| function.panics.iter().sum::<usize>()).sum(),
        bounds_checks: functions.iter().map(|function| function.panics[BOUNDS_CHECKS]).sum(),
        unwraps: functions.iter().map(|function| function.panics[UNWRAPS]).sum(),
        allocation: functions.iter().map(|function| function.panics[ALLOCATION]).sum(),
        other: functions.iter().map(|function| function.panics[OTHER]).sum(),
        instructions: functions.iter().map(|function| function.panic_instructions).sum(),
        constants,
        functions: callers,
    }
}

fn formatting(functions: &[&Function], limit: usize) -> Formatting {
    let mut callers: Vec<Caller> = functions
        .iter()
        .filter(|function| function.formatting > 0)
        .map(|function| Caller {
            name: function.name.clone(),
            sites: function.formatting,
            instructions: function.formatting_instructions,
        })
        .collect();
    rank_callers(&mut callers, limit);

    Formatting {
        sites: functions.iter().map(|function| function.formatting).sum(),
        instructions: functions.iter().map(|function| function.formatting_instructions).sum(),
        functions: callers,
    }
}

fn copies(functions: &[&Function], limit: usize) -> Copies {
    let mut copiers: Vec<Copier> = functions
        .iter()
        .filter(|function| function.copy_runs > 0 || function.copy_calls > 0)
        .map(|function| Copier {
            name: function.name.clone(),
            runs: function.copy_runs,
            instructions: function.copy_instructions,
            calls: function.copy_calls,
        })
        .collect();
    copiers.sort_by(|a, b| {
        b.instructions
            .cmp(&a.instructions)
            .then_with(|| b.calls.cmp(&a.calls))
            .then_with(|| a.name.cmp(&b.name))
    });
    copiers.truncate(limit);

    Copies {
        runs: functions.iter().map(|function| function.copy_runs).sum(),
        instructions: functions.iter().map(|function| function.copy_instructions).sum(),
        calls: functions.iter().map(|function| function.copy_calls).sum(),
        functions: copiers,
    }
}

fn ranked_lines(
    lines: FxHashMap<(String, u64), (u64, Origin)>,
    limit: usize,
) -> (Vec<Line>, Vec<Line>) {
    let mut lines: Vec<(Line, Origin)> = lines
        .into_iter()
        .map(|((file, line), (instructions, origin))| {
            (Line { file, line, instructions, snippet: None }, origin)
        })
        .collect();
    lines.sort_by(|(a, _), (b, _)| {
        b.instructions.cmp(&a.instructions).then_with(|| (&a.file, a.line).cmp(&(&b.file, b.line)))
    });

    let workspace = lines
        .iter()
        .filter(|(_, origin)| *origin == Origin::Workspace)
        .take(limit)
        .map(|(line, _)| Line {
            file: line.file.clone(),
            line: line.line,
            instructions: line.instructions,
            snippet: None,
        })
        .collect();
    lines.truncate(limit);

    (lines.into_iter().map(|(line, _)| line).collect(), workspace)
}

/// Largest first by the code the calls cost, as every other list is.
fn rank_callers(callers: &mut Vec<Caller>, limit: usize) {
    callers.sort_by(|a, b| {
        b.instructions
            .cmp(&a.instructions)
            .then_with(|| b.sites.cmp(&a.sites))
            .then_with(|| a.name.cmp(&b.name))
    });
    callers.truncate(limit);
}

/// A run of identifier characters, or the text between two.
enum Piece<'a> {
    Local(&'a str),
    Other(&'a str),
}

/// Split `text` so that local labels can be told from everything else.
fn pieces(text: &str) -> impl Iterator<Item = Piece<'_>> {
    let mut rest = text;
    std::iter::from_fn(move || {
        if rest.is_empty() {
            return None;
        }

        let identifier = rest.starts_with(is_identifier_char);
        let end = rest.find(|c| is_identifier_char(c) != identifier).unwrap_or(rest.len());
        let (piece, tail) = rest.split_at(end);
        rest = tail;

        Some(if identifier && is_local(piece) { Piece::Local(piece) } else { Piece::Other(piece) })
    })
}

const fn is_identifier_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '$')
}

fn is_identifier(text: &str) -> bool {
    !text.is_empty() && text.chars().all(is_identifier_char)
}

/// Assembler-local labels: `LBB0_1`, `Ltmp3`, `LJTI0_0`, `lCPI0_0`,
/// `Lfunc_begin0`, with a `.L` spelling on ELF. Rust functions never start
/// with `L`, and the module's private constants — `l_anon.<hash>.<n>`,
/// `l_switch.table.<fn>` — are deliberately not local: two functions loading
/// different constants are different code.
fn is_local(label: &str) -> bool {
    let label = label.strip_prefix('.').unwrap_or(label);
    let Some(rest) = label.strip_prefix(['L', 'l']) else { return false };

    rest.starts_with(|c: char| c.is_ascii_uppercase())
        || ["tmp", "func_", "loh", "exception", "set", "cfi", "xray"]
            .iter()
            .any(|prefix| rest.starts_with(prefix))
}

/// A basic-block label, the only kind of local label control flow reaches.
fn is_block_label(label: &str) -> bool {
    label.strip_prefix('.').unwrap_or(label).starts_with("LBB")
}

/// Whether a line is a directive that moves to another section.
fn changes_section(text: &str) -> bool {
    let Some(directive) = text.strip_prefix('.') else { return false };
    let name = directive.split(char::is_whitespace).next().unwrap_or(directive);
    matches!(
        name,
        "section"
            | "text"
            | "data"
            | "rodata"
            | "const_data"
            | "const"
            | "cstring"
            | "literal4"
            | "literal8"
            | "literal16"
            | "bss"
    )
}

/// Whether a `.section` directive names code: `__TEXT,__text` on Mach-O,
/// `.text*` on ELF and COFF.
fn is_text_section(arguments: &str) -> bool {
    let mut parts = arguments.split(',').map(str::trim);
    match parts.next() {
        Some("__TEXT") => parts.next() == Some("__text"),
        Some(name) => name.starts_with(".text"),
        None => false,
    }
}

/// Branches, calls, and returns: what ends a basic block.
fn transfers_control(arch: Arch, mnemonic: &str) -> bool {
    match arch {
        Arch::Aarch64 => {
            mnemonic.starts_with("b.")
                || matches!(
                    mnemonic,
                    "b" | "bl" | "blr" | "br" | "ret" | "cbz" | "cbnz" | "tbz" | "tbnz" | "brk"
                )
        }
        Arch::X86 => {
            mnemonic.starts_with('j')
                || matches!(mnemonic, "call" | "callq" | "calll" | "ret" | "retq" | "ud2")
        }
        Arch::Other => matches!(mnemonic, "bl" | "b" | "call" | "callq" | "jmp"),
    }
}

/// The symbol a direct call or tail call goes to.
fn call_target<'a>(mnemonic: &str, operands: &'a str) -> Option<&'a str> {
    if !matches!(mnemonic, "bl" | "b" | "call" | "callq" | "calll" | "jmp" | "jmpq") {
        return None;
    }

    // Registers, memory, and immediates are indirect; a symbol is one word.
    let target = operands.strip_prefix('*').unwrap_or(operands);
    if target.is_empty()
        || target.starts_with(['%', '(', '[', '$'])
        || target.starts_with(|c: char| c.is_ascii_digit())
        || target.contains(char::is_whitespace)
        || is_local(target)
    {
        return None;
    }

    // `memcpy@PLT`, `memcpy@GOTPCREL(%rip)`.
    target.split('@').next()
}

/// The symbol whose address an instruction takes: `adrp x0, _sym@PAGE` on
/// arm64, `leaq _sym(%rip), %rax` on x86. An address taken means the target
/// may be reached indirectly, which the call graph must know.
fn address_target<'a>(arch: Arch, mnemonic: &str, operands: &'a str) -> Option<&'a str> {
    let target = match arch {
        Arch::Aarch64 if mnemonic == "adrp" => operands.split(',').nth(1)?.trim(),
        Arch::X86 if mnemonic.starts_with("lea") && operands.contains("(%rip)") => {
            operands.split('(').next()?.trim()
        }
        _ => return None,
    };

    symbol_target(target.split('@').next()?)
}

/// `value` when it names a real symbol: not a local label, a number, or an
/// anonymous constant. A trailing `+offset` is dropped.
fn symbol_target(value: &str) -> Option<&str> {
    let end = value.find(|c| !is_identifier_char(c)).unwrap_or(value.len());
    let target = &value[..end];

    (!target.is_empty()
        && !target.starts_with(|c: char| c.is_ascii_digit())
        && !is_local(target)
        && !target.contains("anon.")
        && !target.contains("__unnamed"))
    .then_some(target)
}

/// Bytes one value of a data directive emits.
fn directive_width(arch: Arch, name: &str) -> u64 {
    match name {
        "quad" | "xword" | "8byte" => 8,
        "long" | "4byte" => 4,
        // `.word` is 4 bytes on ARM and 2 on x86.
        "word" if arch == Arch::Aarch64 => 4,
        "short" | "hword" | "2byte" | "word" => 2,
        "byte" => 1,
        _ => 0,
    }
}

/// A load or store, the kind of instruction copies are made of.
fn is_move(arch: Arch, mnemonic: &str, operands: &str) -> bool {
    match arch {
        Arch::Aarch64 => matches!(
            mnemonic,
            "ldp"
                | "stp"
                | "ldr"
                | "str"
                | "ldur"
                | "stur"
                | "ldnp"
                | "stnp"
                | "ldrb"
                | "strb"
                | "ldrh"
                | "strh"
                | "ldrsb"
                | "ldrsh"
                | "ldrsw"
                | "ldpsw"
        ),
        Arch::X86 => {
            (mnemonic.starts_with("mov") || mnemonic.starts_with("vmov"))
                && operands.contains(['(', '['])
        }
        Arch::Other => false,
    }
}

/// Which report a call belongs in, judged by the demangled callee.
fn sink(callee: &str) -> Option<Sink> {
    let bare = callee.trim_start_matches('_');
    if matches!(bare, "memcpy" | "memmove" | "memset" | "memcmp" | "bcmp")
        || ["::memcpy", "::memmove", "::memset"].iter().any(|suffix| callee.ends_with(suffix))
    {
        return Some(Sink::Copy);
    }

    if callee.contains("bounds_check") || callee.ends_with("_fail") {
        return Some(Sink::Panic(BOUNDS_CHECKS));
    }
    if callee.ends_with("unwrap_failed") || callee.ends_with("expect_failed") {
        return Some(Sink::Panic(UNWRAPS));
    }
    if callee.contains("capacity_overflow")
        || callee.contains("handle_alloc_error")
        || callee.ends_with("raw_vec::handle_error")
    {
        return Some(Sink::Panic(ALLOCATION));
    }
    if callee.contains("panic") {
        return Some(Sink::Panic(OTHER));
    }

    if callee.contains("core::fmt::")
        || callee.contains("alloc::fmt::")
        || callee.contains("write_fmt")
    {
        return Some(Sink::Formatting);
    }

    None
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    /// Parse `text` for a binary whose code symbols and sizes are `sizes`.
    fn report(arch: Arch, sizes: &[(&str, u64)], text: &str) -> AssemblyReport {
        parsed(arch, sizes, text).0
    }

    /// Like `report`, also returning the reference graph the pass collected.
    fn parsed(arch: Arch, sizes: &[(&str, u64)], text: &str) -> (AssemblyReport, Edges) {
        let (report, edges, _) = everything(arch, sizes, text);
        (report, edges)
    }

    /// The whole pass: report, reference graph, and collected constants.
    fn everything(
        arch: Arch,
        sizes: &[(&str, u64)],
        text: &str,
    ) -> (AssemblyReport, Edges, Collected) {
        let sizes: FxHashMap<String, u64> =
            sizes.iter().map(|&(name, size)| (name.to_owned(), size)).collect();
        let mut parser = Parser::new(arch, &sizes, Path::new("/work/space"));
        parser.parse(text.as_bytes()).expect("parse");
        parser.report(&[PathBuf::from("test.s")], 20)
    }

    /// Run the constants analysis over `text`.
    fn constants_of(
        arch: Arch,
        sizes: &[(&str, u64)],
        text: &str,
    ) -> crate::constants::ConstantsReport {
        let (_, _, collected) = everything(arch, sizes, text);
        let sizes: FxHashMap<String, u64> =
            sizes.iter().map(|&(name, size)| (name.to_owned(), size)).collect();
        crate::constants::analyze(collected, &sizes, Path::new("/work/space"), 20)
    }

    const MACHO: &str = r#"
	.section	__TEXT,__text,regular,pure_instructions
	.file	1 "/work/space/crates/a/src" "lib.rs"
	.file	2 "/Users/x/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/serde-1.0.1" "src/de.rs"
	.file	3 "/rustc/abc/library/core/src" "ptr/mod.rs"
	.globl	__ZN1a1fE
	.p2align	2
__ZN1a1fE:
Lfunc_begin0:
	.loc	1 10 0
	.cfi_startproc
	stp	x29, x30, [sp, #-16]!
Ltmp0:
	.loc	3 825 5 prologue_end
	ldr	x8, [x0]
	cmp	x8, x1
	b.hs	LBB0_2
	ldr	x0, [x0, #8]
	ldp	x29, x30, [sp], #16
	ret
LBB0_2:
Lloh0:
	adrp	x2, l_anon.4d3a.7@PAGE
Lloh1:
	add	x2, x2, l_anon.4d3a.7@PAGEOFF
	mov	x0, x1
	mov	x1, x8
	bl	__ZN4core9panicking18panic_bounds_check17h0000000000000000E
Lfunc_end0:
	.cfi_endproc

	.globl	__ZN1a1gE
	.p2align	2
__ZN1a1gE:
Lfunc_begin1:
	.loc	1 20 0
	.cfi_startproc
	stp	x29, x30, [sp, #-16]!
	ldr	x8, [x0]
Ltmp3:
	.loc	2 40 1
	cmp	x8, x1
	b.hs	LBB1_2
	ldr	x0, [x0, #8]
	ldp	x29, x30, [sp], #16
	ret
LBB1_2:
	adrp	x2, l_anon.4d3a.7@PAGE
	add	x2, x2, l_anon.4d3a.7@PAGEOFF
	mov	x0, x1
	mov	x1, x8
	bl	__ZN4core9panicking18panic_bounds_check17h0000000000000000E
Lfunc_end1:
	.cfi_endproc

	.globl	__ZN1a1hE
	.p2align	2
__ZN1a1hE:
	.cfi_startproc
	stp	x29, x30, [sp, #-16]!
	ldr	x8, [x0]
	cmp	x8, x1
	b.hs	LBB2_2
	ldr	x0, [x0, #8]
	ldp	x29, x30, [sp], #16
	ret
LBB2_2:
	adrp	x2, l_anon.4d3a.9@PAGE
	add	x2, x2, l_anon.4d3a.9@PAGEOFF
	mov	x0, x1
	mov	x1, x8
	bl	__ZN4core9panicking18panic_bounds_check17h0000000000000000E
	.cfi_endproc

	.section	__TEXT,__literal8,8byte_literals
lCPI3_0:
	.quad	0x3ff8000000000000
	.section	__TEXT,__text,regular,pure_instructions
	.globl	__ZN1a1kE
__ZN1a1kE:
	adrp	x8, lCPI3_0@PAGE
	ldr	d0, [x8, lCPI3_0@PAGEOFF]
	ret

	.section	__TEXT,__literal8,8byte_literals
lCPI4_0:
	.quad	0x4004000000000000
	.section	__TEXT,__text,regular,pure_instructions
	.globl	__ZN1a1lE
__ZN1a1lE:
	adrp	x8, lCPI4_0@PAGE
	ldr	d0, [x8, lCPI4_0@PAGEOFF]
	ret

	.globl	__ZN1a4copyE
__ZN1a4copyE:
	ldp	q0, q1, [x1]
	ldp	q2, q3, [x1, #32]
	stp	q0, q1, [x0]
	stp	q2, q3, [x0, #32]
	ldp	q0, q1, [x1, #64]
	ldp	q2, q3, [x1, #96]
	stp	q0, q1, [x0, #64]
	stp	q2, q3, [x0, #96]
	mov	x2, #4096
	bl	_memcpy
	adrp	x0, l_anon.4d3a.1@PAGE
	add	x0, x0, l_anon.4d3a.1@PAGEOFF
	bl	__ZN5alloc3fmt6format17h0000000000000000E
	ret

	.globl	__ZN1a7droppedE
__ZN1a7droppedE:
	ret

	.section	__TEXT,__const
l_anon.4d3a.7:
	.quad	1
"#;

    const GRAPH: &str = r"
	.section	__TEXT,__text,regular,pure_instructions
	.globl	_main
_main:
	bl	_RNvC1a3big
	bl	_RNvC1a6shared
	b.eq	LBB0_1
LBB0_1:
	adrp	x0, _RNvC1a5taken@PAGE
	add	x0, x0, _RNvC1a5taken@PAGEOFF
	adrp	x1, l_anon.9.9@PAGE
	ret

	.globl	_RNvC1a3big
_RNvC1a3big:
	b	_RNvC1a6shared
	ret

	.globl	_RNvC1a6shared
_RNvC1a6shared:
	bl	_RNvC1a5taken
	ret

	.globl	_RNvC1a5taken
_RNvC1a5taken:
	ret

	.section	__DATA,__const
	.p2align	3, 0x0
l_vtable.0:
	.quad	_RINvNtC4core3ptr9drop_glueNtC1a3FooE
	.quad	24
	.quad	8
	.quad	_RNvXC1bNtC1a3FooNtC1b7Service4call
";

    #[test]
    fn collects_the_reference_graph() {
        let sizes = [
            ("_main", 40),
            ("_RNvC1a3big", 500),
            ("_RNvC1a6shared", 300),
            ("_RNvC1a5taken", 300),
            ("_RINvNtC4core3ptr9drop_glueNtC1a3FooE", 16),
            ("_RNvXC1bNtC1a3FooNtC1b7Service4call", 16),
        ];
        let (_, edges) = parsed(Arch::Aarch64, &sizes, GRAPH);
        let sizes: FxHashMap<String, u64> =
            sizes.iter().map(|&(name, size)| (name.to_owned(), size)).collect();

        let report = crate::graph::analyze(edges, &sizes, 20);

        // Four calls (`b.eq` to a local label is not one), one taken address
        // (the `l_anon` adrp is not a symbol), and two vtable slots (the
        // numeric quads are not symbols).
        assert_eq!(report.edges, 7);

        // Only `big` has one caller and no taken address: `shared` is called
        // twice, `taken` is addressed.
        assert_eq!(report.single_callers.len(), 1);
        assert_eq!(report.single_callers[0].name, "a::big");
        assert_eq!(report.single_callers[0].caller, "_main");

        // The vtable is drop glue + a method naming the trait; its size is its
        // four pointer-width slots.
        assert_eq!(report.vtables.len(), 1);
        assert_eq!(report.vtables[0].bytes, 32);
        assert_eq!(report.vtables[0].count, 1);
        assert!(report.vtables[0].name.contains("Service"), "{}", report.vtables[0].name);
    }

    #[test]
    fn folds_identical_bodies_and_keeps_different_ones_apart() {
        let sizes = [
            ("__ZN1a1fE", 60),
            ("__ZN1a1gE", 60),
            ("__ZN1a1hE", 60),
            ("__ZN1a1kE", 12),
            ("__ZN1a1lE", 12),
            ("__ZN1a4copyE", 56),
        ];
        let report = report(Arch::Aarch64, &sizes, MACHO);

        // `f` and `g` differ only in names and debug labels; `h` loads another
        // constant, and `k`/`l` differ only in their constant pools.
        assert_eq!(report.identical.groups, 1);
        assert_eq!(report.identical.recoverable, 60);
        let group = &report.identical.largest[0];
        assert_eq!(group.names, ["a::f", "a::g"]);
        assert_eq!(group.instructions, 12);

        // `dropped` has no symbol, so it is seen but not counted.
        assert_eq!(report.functions, 7);
        assert_eq!(report.linked, 6);
        assert_eq!(report.bytes, 260);
    }

    #[test]
    fn counts_panic_blocks_from_the_branch_that_skips_them() {
        let report = report(Arch::Aarch64, &[("__ZN1a1fE", 60), ("__ZN1a1hE", 60)], MACHO);
        let panics = &report.panics;

        assert_eq!(panics.sites, 2);
        assert_eq!(panics.bounds_checks, 2);
        assert_eq!(panics.other, 0);
        // adrp, add, mov, mov, bl — not the compare and branch before the block.
        assert_eq!(panics.instructions, 10);
        assert_eq!(panics.constants, 2);
        assert_eq!(panics.functions[0].sites, 1);
        assert_eq!(panics.functions[0].instructions, 5);
    }

    #[test]
    fn finds_copies_and_formatting() {
        let report = report(Arch::Aarch64, &[("__ZN1a4copyE", 56)], MACHO);

        assert_eq!(report.copies.runs, 1);
        assert_eq!(report.copies.instructions, 8);
        assert_eq!(report.copies.calls, 1);
        assert_eq!(report.copies.functions[0].name, "a::copy");

        assert_eq!(report.formatting.sites, 1);
        // adrp, add, bl — the block after the memcpy call.
        assert_eq!(report.formatting.instructions, 3);
    }

    #[test]
    fn attributes_instructions_to_source_lines() {
        let report = report(Arch::Aarch64, &[("__ZN1a1fE", 60), ("__ZN1a1gE", 60)], MACHO);

        let line = |file: &str, number: u64| {
            report
                .lines
                .iter()
                .find(|line| line.file == file && line.line == number)
                .map(|line| line.instructions)
        };
        // `f`: one instruction on lib.rs:10, the rest on the core line.
        assert_eq!(line("crates/a/src/lib.rs", 10), Some(1));
        assert_eq!(line("library/core/src/ptr/mod.rs", 825), Some(11));
        assert_eq!(line("serde-1.0.1/src/de.rs", 40), Some(10));

        let workspace: Vec<&str> =
            report.workspace_lines.iter().map(|line| line.file.as_str()).collect();
        assert_eq!(workspace, ["crates/a/src/lib.rs", "crates/a/src/lib.rs"]);
    }

    const ELF: &str = r#"
	.file	"probe.cgu-0"
	.section	.text._ZN1a1fE,"ax",@progbits
	.globl	_ZN1a1fE
	.type	_ZN1a1fE,@function
_ZN1a1fE:
.Lfunc_begin0:
	.file	1 "/work/space" "src/main.rs"
	.loc	1 5 0
	.cfi_startproc
	pushq	%rbx
	cmpq	%rsi, (%rdi)
	jbe	.LBB0_2
	movq	8(%rdi), %rax
	popq	%rbx
	retq
.LBB0_2:
	leaq	.Lanon.1234.3(%rip), %rdx
	movq	%rsi, %rdi
	callq	*_ZN4core9panicking18panic_bounds_check17h0000000000000000E@GOTPCREL(%rip)
.Lfunc_end0:
	.size	_ZN1a1fE, .Lfunc_end0-_ZN1a1fE
	.cfi_endproc

	.section	.rodata.cst16,"aM",@progbits,16
.LCPI1_0:
	.long	1
	.section	.text._ZN1a1gE,"ax",@progbits
	.globl	_ZN1a1gE
_ZN1a1gE:
	movaps	.LCPI1_0(%rip), %xmm0
	movups	%xmm0, (%rdi)
	movups	%xmm0, 16(%rdi)
	movups	%xmm0, 32(%rdi)
	movups	%xmm0, 48(%rdi)
	movups	%xmm0, 64(%rdi)
	movups	%xmm0, 80(%rdi)
	movups	%xmm0, 96(%rdi)
	movups	%xmm0, 112(%rdi)
	movq	%rdi, %rax
	jmp	memcpy@PLT
"#;

    #[test]
    fn reads_elf_conventions() {
        let report = report(Arch::X86, &[("_ZN1a1fE", 40), ("_ZN1a1gE", 50)], ELF);

        assert_eq!(report.linked, 2);
        assert_eq!(report.instructions, 20);

        assert_eq!(report.panics.bounds_checks, 1);
        assert_eq!(report.panics.instructions, 3);
        assert_eq!(report.panics.constants, 1);

        // Nine stores in a row through the vector register, then a tail call.
        assert_eq!(report.copies.runs, 1);
        assert_eq!(report.copies.instructions, 9);
        assert_eq!(report.copies.calls, 1);

        assert_eq!(report.workspace_lines[0].file, "src/main.rs");
        assert_eq!(report.workspace_lines[0].instructions, 9);
    }

    /// An ELF basic-block label carries a leading dot, like a directive. If it
    /// is read as one, the block never resets and the two loads before the label
    /// are miscounted into the panic block that follows.
    const ELF_FALLTHROUGH: &str = r#"
	.section	.text._ZN1a1hE,"ax",@progbits
	.globl	_ZN1a1hE
_ZN1a1hE:
	movq	(%rdi), %rax
	movq	8(%rdi), %rcx
.LBB2_1:
	leaq	.Lanon.9999.0(%rip), %rdx
	callq	*_ZN4core9panicking9panic_fmt17h0000000000000000E@GOTPCREL(%rip)
"#;

    #[test]
    fn elf_block_label_resets_the_block() {
        let report = report(Arch::X86, &[("_ZN1a1hE", 32)], ELF_FALLTHROUGH);

        assert_eq!(report.panics.sites, 1);
        // leaq and callq, not the two movq before the `.LBB` label.
        assert_eq!(report.panics.instructions, 2);
    }

    /// A function whose panic block loads a location record; the constants
    /// that follow it: the record's path, a derived-Debug name, an `expect`
    /// message, a vtable with no drop glue, a lookup table, a jump table, and
    /// an exception table bound by `.cfi_lsda`.
    const MACHO_CONSTANTS: &str = r#"
	.section	__TEXT,__text,regular,pure_instructions
	.globl	__ZN1a1fE
	.p2align	2
__ZN1a1fE:
	.cfi_startproc
	.cfi_personality 155, ___rust_eh_personality
	.cfi_lsda 16, Lexception0
	stp	x29, x30, [sp, #-16]!
	cmp	x8, x1
	b.hs	LBB0_2
	adrp	x2, l_anon.h.20@PAGE
	add	x2, x2, l_anon.h.20@PAGEOFF
	adrp	x3, l_switch.table.__ZN1a1fE@PAGE
	adrp	x4, LJTI0_0@PAGE
	adrp	x5, l_anon.h.30@PAGE
	ret
LBB0_2:
	adrp	x2, l_anon.h.60@PAGE
	add	x2, x2, l_anon.h.60@PAGEOFF
	adrp	x3, l_anon.h.59@PAGE
	add	x3, x3, l_anon.h.59@PAGEOFF
	bl	__ZN4core9panicking18panic_bounds_check17h0000000000000000E
	.cfi_endproc
	.section	__TEXT,__const
LJTI0_0:
	.long	LBB0_2-LJTI0_0
	.long	LBB0_2-LJTI0_0
	.long	LBB0_2-LJTI0_0
	.section	__TEXT,__gcc_except_tab
	.p2align	2, 0x0
GCC_except_table0:
Lexception0:
	.byte	255
	.byte	255
	.byte	1
	.uleb128 Lcst_end0-Lcst_begin0
Lcst_begin0:
	.uleb128 Lfunc_begin0-Lfunc_begin0
	.uleb128 Ltmp0-Lfunc_begin0
	.byte	0
	.byte	0
	.uleb128 Ltmp0-Lfunc_begin0
	.uleb128 Ltmp1-Ltmp0
	.uleb128 Ltmp2-Lfunc_begin0
	.byte	0
Lcst_end0:
	.p2align	2, 0x0

	.section	__TEXT,__cstring,cstring_literals
l_anon.h.52:
	.asciz	"src/main.rs"

	.section	__TEXT,__const
l_anon.h.20:
	.ascii	"Circle"

l_anon.h.59:
	.ascii	"index must be a number"

	.section	__DATA,__const
	.p2align	3, 0x0
l_anon.h.60:
	.quad	l_anon.h.52
	.asciz	"\013\000\000\000\000\000\000\000(\000\000\000&\000\000"

	.p2align	3, 0x0
l_anon.h.30:
	.asciz	"\000\000\000\000\000\000\000\000\030\000\000\000\000\000\000\000\b\000\000\000\000\000\000"
	.quad	__ZN1a4drawE

	.section	__TEXT,__const
	.p2align	3, 0x0
l_switch.table.__ZN1a1fE:
	.quad	1
	.quad	2
	.quad	3
	.quad	4

	.section	__DWARF,__debug_info,regular,debug
Lsection_info:
Ldebug_info0:
	.long	Lset0
	.byte	4
	.quad	l_anon.h.59
	.asciz	"not a constant"
"#;

    #[test]
    fn reads_constants_by_shape() {
        let sizes = [("__ZN1a1fE", 80), ("__ZN1a4drawE", 40)];
        let report = constants_of(Arch::Aarch64, &sizes, MACHO_CONSTANTS);
        let class = |kind: crate::constants::Kind| {
            report
                .classes
                .iter()
                .find(|class| class.kind == kind)
                .map(|class| (class.bytes, class.count))
        };
        use crate::constants::Kind;

        // Everything the function loads counts; the debug section does not
        // (its `.quad` names a constant but is not one), the exception table
        // is kept apart, and the path is reached through the location record.
        assert_eq!(report.constants, 7, "{report:#?}");
        assert_eq!(report.linked, 7);
        assert_eq!(class(Kind::Location), Some((24, 1)));
        assert_eq!(class(Kind::Path), Some((12, 1)));
        assert_eq!(class(Kind::Name), Some((6, 1)));
        assert_eq!(class(Kind::Message), Some((22, 1)));
        assert_eq!(class(Kind::Vtable), Some((32, 1)));
        assert_eq!(class(Kind::SwitchTable), Some((32, 1)));
        assert_eq!(class(Kind::JumpTable), Some((12, 1)));
        assert_eq!(class(Kind::Lsda), None);
        assert_eq!(report.bytes, 24 + 12 + 6 + 22 + 32 + 32 + 12);

        // The location decodes to its file and line, charged to `f`.
        assert_eq!(report.locations.records, 1);
        assert_eq!(report.locations.bytes, 24 + 12);
        assert_eq!(report.locations.workspace_files.len(), 1);
        let file = &report.locations.workspace_files[0];
        assert_eq!(
            (file.file.as_str(), file.records, &file.lines[..]),
            ("src/main.rs", 1, &[40][..])
        );
        assert_eq!(report.locations.functions[0].name, "a::f");

        // The message was loaded in the panic block; the name was not.
        assert_eq!((report.panic_messages.bytes, report.panic_messages.count), (22, 1));

        // Strings by size, with who loads them; paths are not listed here.
        let strings: Vec<(&str, &str)> = report
            .strings
            .iter()
            .map(|string| (string.preview.as_str(), string.functions[0].as_str()))
            .collect();
        assert_eq!(strings, [("index must be a number", "a::f"), ("Circle", "a::f")]);

        // Both tables belong to `f`: the lookup table by name, the jump table
        // by who loads it.
        assert_eq!(report.tables.len(), 1);
        assert_eq!(
            (
                report.tables[0].name.as_str(),
                report.tables[0].bytes,
                report.tables[0].switch_tables,
                report.tables[0].jump_tables
            ),
            ("a::f", 44, 1, 1)
        );

        // `f` reaches everything, all of it exclusively.
        assert_eq!(report.functions.len(), 1);
        assert_eq!(
            (
                report.functions[0].bytes,
                report.functions[0].exclusive,
                report.functions[0].constants
            ),
            (140, 140, 7)
        );

        // The exception table, bound by `.cfi_lsda`: three header bytes and
        // nine fields, LEB128 ones counted at one byte; its inner labels do not
        // split it.
        assert_eq!(report.unwind.len(), 1);
        assert_eq!((report.unwind[0].name.as_str(), report.unwind[0].bytes), ("a::f", 12));
    }

    /// The same shapes as ELF spells them: `.L` labels, per-object sections,
    /// `.size`, and `.xword` (aarch64) or `.quad` pointers.
    const ELF_CONSTANTS: &str = r#"
	.section	.text._ZN1a1fE,"ax",@progbits
	.globl	_ZN1a1fE
	.p2align	4
	.type	_ZN1a1fE,@function
_ZN1a1fE:
	.cfi_startproc
	leaq	.L__unnamed_1(%rip), %rsi
	leaq	.Lanon.h.60(%rip), %rdi
	leaq	.Lswitch.table._ZN1a1fE(%rip), %rdx
	callq	*_ZN4core9panicking18panic_bounds_check17h0000000000000000E@GOTPCREL(%rip)
	.cfi_endproc

	.type	.L__unnamed_1,@object
	.section	.rodata.str1.1,"aMS",@progbits,1
.L__unnamed_1:
	.asciz	"src/lib.rs"
	.size	.L__unnamed_1, 11

	.type	.Lanon.h.60,@object
	.section	.data.rel.ro..Lanon.h.60,"aw",@progbits
	.p2align	3, 0x0
.Lanon.h.60:
	.quad	.L__unnamed_1
	.asciz	"\n\000\000\000\000\000\000\000\007\000\000\000\t\000\000"
	.size	.Lanon.h.60, 24

	.type	.Lswitch.table._ZN1a1fE,@object
	.section	.rodata..Lswitch.table._ZN1a1fE,"a",@progbits
	.p2align	3, 0x0
.Lswitch.table._ZN1a1fE:
	.quad	7
	.quad	8
	.size	.Lswitch.table._ZN1a1fE, 16
"#;

    #[test]
    fn reads_elf_constants_too() {
        let report = constants_of(Arch::X86, &[("_ZN1a1fE", 40)], ELF_CONSTANTS);
        use crate::constants::Kind;

        assert_eq!(report.linked, 3, "{report:#?}");
        let kinds: Vec<Kind> = report.classes.iter().map(|class| class.kind).collect();
        assert!(
            kinds.contains(&Kind::Location)
                && kinds.contains(&Kind::Path)
                && kinds.contains(&Kind::SwitchTable),
            "{kinds:?}"
        );
        assert_eq!(report.locations.workspace_files[0].file, "src/lib.rs");
        assert_eq!(report.locations.workspace_files[0].lines, [7]);
        assert_eq!(report.tables[0].bytes, 16);
    }
}
