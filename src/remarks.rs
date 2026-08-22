//! Code-growth decisions from LLVM's optimization remarks.
//!
//! Unrolling copies a loop body, peeling copies its first iterations,
//! vectorizing adds a wide body and a scalar remainder beside it: each is code
//! the source never wrote. Inlining multiplies a callee into its callers.
//! Machine-size remarks then say which backend passes added or removed
//! instructions, and prologue/epilogue remarks expose stack frames large
//! enough to correlate with copied values and large types.
//!
//! Inline and loop remarks come from each target crate's optimization pipeline;
//! machine growth and stack frames come from the final rustc invocation, which
//! includes fat LTO when enabled. The YAML (`-Zremark-dir`, through
//! `RUSTC_BOOTSTRAP=1`) is read with a small streaming parser of `Key: value`
//! lines and an `Args` list, nothing more.

use std::{
    fs,
    io::{self, BufRead, BufReader},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use rustc_hash::FxHashMap;

use crate::{
    assembly::{Origin, source},
    name::{defining_crate, demangle},
};

#[derive(Debug)]
pub struct RemarksReport {
    /// Remark files read, one per codegen unit and stage.
    pub files: usize,

    /// Loops the optimizer unrolled, peeled, or vectorized, in all.
    pub unrolled: usize,
    pub peeled: usize,
    pub vectorized: usize,

    /// Functions by the loops expanded in them, most first.
    pub functions: Vec<ExpandedFunction>,

    /// The expanded loops on lines in this workspace, most often first.
    pub workspace_sites: Vec<LoopSite>,

    pub inlining: InliningReport,

    /// Individual machine-instruction size changes LLVM reported.
    pub size_changes: usize,

    /// Linked functions that grew after instruction selection.
    pub machine_functions: Vec<MachineFunction>,

    /// Backend passes that added instructions, combined across functions.
    pub growth_passes: Vec<MachinePass>,

    /// Functions for which LLVM reported a nonzero stack frame.
    pub stack_functions: usize,

    /// The largest stack frames among functions surviving in the linked image.
    pub stack_frames: Vec<StackFrame>,
}

#[derive(Debug, Default)]
pub struct InliningReport {
    pub passed: usize,
    pub missed: usize,
    pub forced: usize,
    pub callers: Vec<InliningCaller>,
    pub callees: Vec<InlinedCallee>,
    pub workspace_sites: Vec<InliningSite>,
}

#[derive(Debug)]
pub struct InliningCaller {
    pub name: String,
    pub linked_bytes: u64,
    pub sites: usize,
    pub forced: usize,
}

#[derive(Debug)]
pub struct InlinedCallee {
    pub name: String,
    pub sites: usize,
    pub callers: usize,
    pub forced: usize,
}

#[derive(Debug)]
pub struct InliningSite {
    pub file: String,
    pub line: u64,
    pub caller: String,
    pub callee: String,
    pub detail: String,
    pub instances: usize,
    pub caller_bytes: u64,
    pub snippet: Option<String>,
}

#[derive(Debug)]
pub struct MachineFunction {
    pub name: String,
    pub linked_bytes: u64,
    pub added: u64,
    pub removed: u64,
}

#[derive(Debug)]
pub struct MachinePass {
    pub name: String,
    pub added: u64,
    pub removed: u64,
    pub functions: usize,
}

#[derive(Debug)]
pub struct StackFrame {
    pub name: String,
    pub linked_bytes: u64,
    pub stack_bytes: u64,
}

#[derive(Debug)]
pub struct ExpandedFunction {
    pub name: String,
    pub unrolled: usize,
    pub peeled: usize,
    pub vectorized: usize,

    /// The unroll factors summed: how many body copies unrolling made.
    pub copies: u64,
}

#[derive(Debug)]
pub struct LoopSite {
    pub file: String,
    pub line: u64,
    pub function: String,

    /// What was done: `unrolled ×12`, `peeled 1`, `vectorized 4×2`.
    pub detail: String,

    /// The line's source text, for lines in this workspace.
    pub snippet: Option<String>,
}

/// One remark document.
#[derive(Default)]
struct Remark {
    passed: bool,
    missed: bool,
    pass: String,
    name: String,
    function: String,
    file: String,
    line: u64,
    args: Vec<(String, String)>,
}

impl Remark {
    fn arg(&self, key: &str) -> Option<&str> {
        self.args.iter().find(|(name, _)| name == key).map(|(_, value)| value.as_str())
    }
}

#[derive(Default)]
struct InliningCallerTotal {
    sites: usize,
    forced: usize,
}

#[derive(Default)]
struct InlinedCalleeTotal {
    sites: usize,
    forced: usize,
    callers: rustc_hash::FxHashSet<String>,
}

#[derive(Default)]
struct MachineUnit {
    name: String,
    added: u64,
    removed: u64,
}

#[derive(Default)]
struct MachineTotal {
    added: u64,
    removed: u64,
}

#[derive(Default)]
struct PassTotal {
    added: u64,
    removed: u64,
    functions: rustc_hash::FxHashSet<String>,
}

type InlineSiteKey = (String, u64, String, String, String);
type LocatedCount = (usize, Origin);

/// Read every remark file in `dir`, keeping the `limit` largest lists.
/// `workspace` is the workspace root; `crate_dirs` maps each crate's name, as
/// rustc spells it, to its package directory, which a remark's relative source
/// path is relative to.
///
/// # Errors
///
/// Errors when the directory cannot be listed or a file read.
pub fn analyze(
    dir: &Path,
    workspace: &Path,
    crate_dirs: &FxHashMap<String, PathBuf>,
    linked_sizes: &FxHashMap<String, u64>,
    limit: usize,
) -> Result<RemarksReport> {
    let mut paths: Vec<_> = fs::read_dir(dir)
        .with_context(|| format!("failed to read {}", dir.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "yaml"))
        .collect();
    paths.sort();

    let mut linked: FxHashMap<String, u64> = FxHashMap::default();
    for (name, &bytes) in linked_sizes {
        *linked.entry(demangle(name)).or_default() += bytes;
    }

    let mut functions: FxHashMap<String, ExpandedFunction> = FxHashMap::default();
    let mut sites: FxHashMap<(String, u64, String, String), (usize, Origin)> = FxHashMap::default();
    let (mut unrolled, mut peeled, mut vectorized) = (0, 0, 0);
    let mut inlining = InliningReport::default();
    let mut inline_callers: FxHashMap<String, InliningCallerTotal> = FxHashMap::default();
    let mut inline_callees: FxHashMap<String, InlinedCalleeTotal> = FxHashMap::default();
    let mut inline_sites: FxHashMap<InlineSiteKey, LocatedCount> = FxHashMap::default();
    let mut machine_units: FxHashMap<(String, String), MachineUnit> = FxHashMap::default();
    let mut passes: FxHashMap<String, PassTotal> = FxHashMap::default();
    let mut size_changes = 0;
    let mut stacks: FxHashMap<String, u64> = FxHashMap::default();
    let mut files = 0;
    for path in paths {
        let file =
            fs::File::open(&path).with_context(|| format!("failed to open {}", path.display()))?;
        files += 1;
        let unit = path.display().to_string();
        // `<crate>.<hash>-cgu.N.<stage>.opt.yaml`: the crate whose files a
        // relative path is relative to.
        let krate = path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.split('.').next())
            .unwrap_or_default();
        let crate_dir = crate_dirs.get(krate);
        for remark in documents(BufReader::new(file)) {
            let remark = remark.with_context(|| format!("failed to read {}", path.display()))?;
            let name = demangle(&remark.function);
            let defining_dir = defining_crate(&remark.function, &name)
                .and_then(|name| crate_dirs.get(&name))
                .or(crate_dir);
            if let Some(detail) = expansion(&remark) {
                let entry = functions.entry(name.clone()).or_insert_with(|| ExpandedFunction {
                    name: name.clone(),
                    unrolled: 0,
                    peeled: 0,
                    vectorized: 0,
                    copies: 0,
                });
                match &detail {
                    Expansion::Unrolled(count) => {
                        unrolled += 1;
                        entry.unrolled += 1;
                        entry.copies += count;
                    }
                    Expansion::Peeled(_) => {
                        peeled += 1;
                        entry.peeled += 1;
                    }
                    Expansion::Vectorized(..) => {
                        vectorized += 1;
                        entry.vectorized += 1;
                    }
                }
                if let Some((file, origin)) = location(&remark, defining_dir, workspace) {
                    let key = (file, remark.line, name.clone(), detail.to_string());
                    let site = sites.entry(key).or_insert((0, origin));
                    site.0 += 1;
                }
            }

            match remark.pass.as_str() {
                "inline" if remark.passed => {
                    let Some(callee) = remark.arg("Callee") else { continue };
                    let caller = demangle(remark.arg("Caller").unwrap_or(&remark.function));
                    let callee = demangle(callee);
                    let forced = remark.name == "AlwaysInline"
                        || remark.arg("Reason").is_some_and(|reason| reason.contains("always"));
                    inlining.passed += 1;
                    inlining.forced += usize::from(forced);

                    let caller_total = inline_callers.entry(caller.clone()).or_default();
                    caller_total.sites += 1;
                    caller_total.forced += usize::from(forced);
                    let callee_total = inline_callees.entry(callee.clone()).or_default();
                    callee_total.sites += 1;
                    callee_total.forced += usize::from(forced);
                    callee_total.callers.insert(caller.clone());

                    if let Some((file, origin)) = location(&remark, defining_dir, workspace) {
                        let detail = inline_detail(&remark, forced);
                        let key = (file, remark.line, caller, callee, detail);
                        inline_sites.entry(key).or_insert((0, origin)).0 += 1;
                    }
                }
                "inline" if remark.missed => {
                    inlining.missed += 1;
                }
                "size-info" if remark.name == "FunctionMISizeChange" => {
                    let (Some(before), Some(after)) = (
                        remark.arg("MIInstrsBefore").and_then(|value| value.parse::<u64>().ok()),
                        remark.arg("MIInstrsAfter").and_then(|value| value.parse::<u64>().ok()),
                    ) else {
                        continue;
                    };
                    let machine = machine_units
                        .entry((unit.clone(), remark.function.clone()))
                        .or_insert_with(|| MachineUnit {
                            name: name.clone(),
                            ..MachineUnit::default()
                        });
                    if before == 0 {
                        continue;
                    }
                    let pass = remark.arg("Pass").unwrap_or_default();
                    // Debug pseudos are bookkeeping for DWARF, not emitted
                    // instructions and therefore not binary-size growth.
                    if pass.to_ascii_lowercase().contains("debug") {
                        continue;
                    }
                    size_changes += 1;
                    let (added, removed) =
                        if after >= before { (after - before, 0) } else { (0, before - after) };
                    machine.added += added;
                    machine.removed += removed;

                    if linked.contains_key(&name) {
                        let total = passes.entry(pass.to_owned()).or_default();
                        total.added += added;
                        total.removed += removed;
                        total.functions.insert(name);
                    }
                }
                "prologepilog" if remark.name == "StackSize" => {
                    if let Some(bytes) =
                        remark.arg("NumStackBytes").and_then(|value| value.parse::<u64>().ok())
                        && bytes > 0
                    {
                        let stack = stacks.entry(name).or_default();
                        *stack = (*stack).max(bytes);
                    }
                }
                _ => {}
            }
        }
    }

    let mut functions: Vec<ExpandedFunction> = functions.into_values().collect();
    functions.sort_by(|a, b| {
        (b.unrolled + b.peeled + b.vectorized)
            .cmp(&(a.unrolled + a.peeled + a.vectorized))
            .then_with(|| b.copies.cmp(&a.copies))
            .then_with(|| a.name.cmp(&b.name))
    });
    functions.truncate(limit);

    let mut all: Vec<(LoopSite, usize, Origin)> = sites
        .into_iter()
        .map(|((file, line, function, detail), (count, origin))| {
            (LoopSite { file, line, function, detail, snippet: None }, count, origin)
        })
        .collect();
    all.sort_by(|(a, x, _), (b, y, _)| {
        y.cmp(x).then_with(|| (&a.file, a.line, &a.function).cmp(&(&b.file, b.line, &b.function)))
    });
    let workspace_sites = all
        .into_iter()
        .filter(|(_, _, origin)| *origin == Origin::Workspace)
        .take(limit)
        .map(|(site, _, _)| site)
        .collect();

    inlining.callers = inline_callers
        .into_iter()
        .filter_map(|(name, total)| {
            Some(InliningCaller {
                linked_bytes: *linked.get(&name)?,
                name,
                sites: total.sites,
                forced: total.forced,
            })
        })
        .collect();
    inlining.callers.sort_by(|a, b| {
        b.linked_bytes
            .cmp(&a.linked_bytes)
            .then_with(|| b.sites.cmp(&a.sites))
            .then_with(|| a.name.cmp(&b.name))
    });
    inlining.callers.truncate(limit);

    inlining.callees = inline_callees
        .into_iter()
        .map(|(name, total)| InlinedCallee {
            name,
            sites: total.sites,
            callers: total.callers.len(),
            forced: total.forced,
        })
        .collect();
    inlining.callees.sort_by(|a, b| {
        b.sites
            .cmp(&a.sites)
            .then_with(|| b.callers.cmp(&a.callers))
            .then_with(|| a.name.cmp(&b.name))
    });
    inlining.callees.truncate(limit);

    let mut inline_sites: Vec<(InliningSite, Origin)> = inline_sites
        .into_iter()
        .map(|((file, line, caller, callee, detail), (instances, origin))| {
            let caller_bytes = linked.get(&caller).copied().unwrap_or_default();
            (
                InliningSite {
                    file,
                    line,
                    caller,
                    callee,
                    detail,
                    instances,
                    caller_bytes,
                    snippet: None,
                },
                origin,
            )
        })
        .collect();
    inline_sites.sort_by(|(a, _), (b, _)| {
        b.caller_bytes
            .cmp(&a.caller_bytes)
            .then_with(|| b.instances.cmp(&a.instances))
            .then_with(|| (&a.file, a.line, &a.callee).cmp(&(&b.file, b.line, &b.callee)))
    });
    inlining.workspace_sites = inline_sites
        .into_iter()
        .filter(|(_, origin)| *origin == Origin::Workspace)
        .take(limit)
        .map(|(site, _)| site)
        .collect();

    let mut machine_totals: FxHashMap<String, MachineTotal> = FxHashMap::default();
    for unit in machine_units.into_values() {
        let total = machine_totals.entry(unit.name).or_default();
        total.added += unit.added;
        total.removed += unit.removed;
    }
    let mut machine_functions: Vec<MachineFunction> = machine_totals
        .into_iter()
        .filter_map(|(name, total)| {
            let linked_bytes = *linked.get(&name)?;
            (total.added > 0).then_some(MachineFunction {
                name,
                linked_bytes,
                added: total.added,
                removed: total.removed,
            })
        })
        .collect();
    machine_functions.sort_by(|a, b| {
        b.added
            .cmp(&a.added)
            .then_with(|| b.linked_bytes.cmp(&a.linked_bytes))
            .then_with(|| a.name.cmp(&b.name))
    });
    machine_functions.truncate(limit);

    let mut growth_passes: Vec<MachinePass> = passes
        .into_iter()
        .filter(|(_, total)| total.added > 0)
        .map(|(name, total)| MachinePass {
            name,
            added: total.added,
            removed: total.removed,
            functions: total.functions.len(),
        })
        .collect();
    growth_passes.sort_by(|a, b| b.added.cmp(&a.added).then_with(|| a.name.cmp(&b.name)));
    growth_passes.truncate(limit);

    let mut stack_frames: Vec<StackFrame> = stacks
        .into_iter()
        .filter_map(|(name, stack_bytes)| {
            Some(StackFrame { linked_bytes: *linked.get(&name)?, name, stack_bytes })
        })
        .collect();
    stack_frames.sort_by(|a, b| {
        b.stack_bytes
            .cmp(&a.stack_bytes)
            .then_with(|| b.linked_bytes.cmp(&a.linked_bytes))
            .then_with(|| a.name.cmp(&b.name))
    });
    let stack_functions = stack_frames.len();
    stack_frames.truncate(limit);

    Ok(RemarksReport {
        files,
        unrolled,
        peeled,
        vectorized,
        functions,
        workspace_sites,
        inlining,
        size_changes,
        machine_functions,
        growth_passes,
        stack_functions,
        stack_frames,
    })
}

fn location(
    remark: &Remark,
    crate_dir: Option<&PathBuf>,
    workspace: &Path,
) -> Option<(String, Origin)> {
    if remark.file.is_empty() {
        return None;
    }
    let file = Path::new(&remark.file);
    let absolute = match crate_dir {
        Some(dir) if !file.is_absolute() => dir.join(file).display().to_string(),
        _ => remark.file.clone(),
    };
    Some(source(&absolute, workspace))
}

fn inline_detail(remark: &Remark, forced: bool) -> String {
    if forced {
        return "forced by #[inline(always)]".to_owned();
    }
    match (remark.arg("Cost"), remark.arg("Threshold")) {
        (Some(cost), Some(threshold)) => format!("cost {cost}, threshold {threshold}"),
        _ => remark.name.clone(),
    }
}

/// What a passed remark says the optimizer did to a loop.
enum Expansion {
    Unrolled(u64),
    Peeled(u64),
    Vectorized(u64, u64),
}

impl std::fmt::Display for Expansion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unrolled(count) => write!(f, "unrolled \u{d7}{count}"),
            Self::Peeled(count) => write!(f, "peeled {count}"),
            Self::Vectorized(width, interleave) => {
                write!(f, "vectorized {width}\u{d7}{interleave}")
            }
        }
    }
}

fn expansion(remark: &Remark) -> Option<Expansion> {
    if !remark.passed {
        return None;
    }
    let arg = |key: &str| remark.arg(key).and_then(|value| value.parse().ok());
    match (remark.pass.as_str(), remark.name.as_str()) {
        ("loop-unroll", "Peeled") => Some(Expansion::Peeled(arg("PeelCount").unwrap_or(1))),
        ("loop-unroll", _) => Some(Expansion::Unrolled(arg("UnrollCount").unwrap_or(1))),
        ("loop-vectorize", "Vectorized") => Some(Expansion::Vectorized(
            arg("VectorizationFactor").unwrap_or(1),
            arg("InterleaveCount").unwrap_or(1),
        )),
        _ => None,
    }
}

/// The remark documents in one file, as LLVM writes them: `--- !Passed`,
/// then `Key: value` lines, `DebugLoc: { File: '…', Line: N, Column: M }` over
/// one line or two, and an `Args:` list of `- Key: value` items. This is an
/// iterator rather than a `Vec`: machine-size output can be hundreds of MiB.
struct Documents<R: BufRead> {
    lines: io::Lines<R>,
    current: Option<Remark>,
    in_args: bool,
    debug_loc: String,
}

fn documents<R: BufRead>(reader: R) -> Documents<R> {
    Documents { lines: reader.lines(), current: None, in_args: false, debug_loc: String::new() }
}

impl<R: BufRead> Iterator for Documents<R> {
    type Item = io::Result<Remark>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let Some(line) = self.lines.next() else {
                return self.current.take().map(Ok);
            };
            let line = match line {
                Ok(line) => line,
                Err(error) => return Some(Err(error)),
            };

            if let Some(kind) = line.strip_prefix("--- ") {
                let finished = self.current.replace(Remark {
                    passed: kind.trim() == "!Passed",
                    missed: kind.trim() == "!Missed",
                    ..Remark::default()
                });
                self.in_args = false;
                self.debug_loc.clear();
                if let Some(finished) = finished {
                    return Some(Ok(finished));
                }
                continue;
            }
            let Some(remark) = self.current.as_mut() else { continue };
            if line.starts_with("...") {
                return self.current.take().map(Ok);
            }

            // A DebugLoc may wrap onto a second line; gather it to the brace.
            if !self.debug_loc.is_empty() {
                self.debug_loc.push(' ');
                self.debug_loc.push_str(line.trim());
                if line.contains('}') {
                    (remark.file, remark.line) = debug_loc_fields(&self.debug_loc);
                    self.debug_loc.clear();
                }
                continue;
            }

            if self.in_args {
                if let Some(item) = line.trim_start().strip_prefix("- ")
                    && let Some((key, value)) = item.split_once(':')
                {
                    remark.args.push((key.trim().to_owned(), unquote(value)));
                    continue;
                }
                // Callee and caller arguments carry a nested DebugLoc. It
                // belongs to that argument, not the document, and must not end
                // Args before Cost, Threshold, and Reason are read.
                if line.starts_with(char::is_whitespace) {
                    continue;
                }
                self.in_args = false;
            }

            let Some((key, value)) = line.split_once(':') else { continue };
            let value = value.trim();
            match key.trim() {
                "Pass" => remark.pass = unquote(value),
                "Name" => remark.name = unquote(value),
                "Function" => remark.function = unquote(value),
                "DebugLoc" => {
                    if value.contains('}') {
                        (remark.file, remark.line) = debug_loc_fields(value);
                    } else {
                        self.debug_loc.push_str(value);
                    }
                }
                "Args" => self.in_args = true,
                _ => {}
            }
        }
    }
}

/// `File` and `Line` out of `{ File: '…', Line: N, Column: M }`.
fn debug_loc_fields(text: &str) -> (String, u64) {
    let field = |key: &str| {
        text.split(',')
            .chain(text.split('{'))
            .find_map(|part| part.trim().trim_start_matches('{').trim().strip_prefix(key))
            .map(|rest| unquote(rest.trim_start_matches(':').trim_end_matches('}')))
    };
    let file = field("File").unwrap_or_default();
    let line = field("Line").and_then(|line| line.trim().parse().ok()).unwrap_or(0);
    (file, line)
}

/// A YAML scalar without its quotes.
fn unquote(value: &str) -> String {
    let value = value.trim();
    let value = value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')).unwrap_or(value);
    let value = value.strip_prefix('"').and_then(|v| v.strip_suffix('"')).unwrap_or(value);
    value.replace("''", "'")
}

#[cfg(test)]
mod tests {
    use std::{io, path::PathBuf};

    use rustc_hash::FxHashMap;

    use super::{Expansion, analyze, debug_loc_fields, documents, expansion};

    const YAML: &str = "--- !Passed
Pass:            loop-unroll
Name:            FullyUnrolled
DebugLoc:        { File: '/work/space/src/lib.rs',
                   Line: 12, Column: 5 }
Function:        _ZN1a1fE
Args:
  - String:          'completely unrolled loop with '
  - UnrollCount:     '12'
  - String:          ' iterations'
...
--- !Passed
Pass:            loop-vectorize
Name:            Vectorized
DebugLoc:        { File: '/work/space/src/lib.rs', Line: 40, Column: 9 }
Function:        _ZN1a1fE
Args:
  - String:          'vectorized '
  - VectorizationFactor: '4'
  - InterleaveCount: '2'
...
--- !Missed
Pass:            loop-vectorize
Name:            MissedDetails
Function:        _ZN1a1gE
Args:
  - String:          loop not vectorized
...
";

    const PIPELINE_YAML: &str = "--- !Passed
Pass:            inline
Name:            Inlined
DebugLoc:        { File: 'src/lib.rs', Line: 10, Column: 5 }
Function:        caller
Args:
  - Callee:          tiny
    DebugLoc:        { File: 'src/lib.rs', Line: 1, Column: 1 }
  - String:          ' inlined into '
  - Caller:          caller
    DebugLoc:        { File: 'src/lib.rs', Line: 8, Column: 1 }
  - Cost:            '12'
  - Threshold:       '50'
...
--- !Passed
Pass:            inline
Name:            AlwaysInline
DebugLoc:        { File: 'src/lib.rs', Line: 10, Column: 5 }
Function:        caller
Args:
  - Callee:          tiny
    DebugLoc:        { File: 'src/lib.rs', Line: 1, Column: 1 }
  - Caller:          caller
  - Reason:          always inline attribute
...
--- !Missed
Pass:            inline
Name:            TooCostly
Function:        caller
Args:
  - Callee:          large
  - Caller:          caller
  - Cost:            '90'
  - Threshold:       '50'
...
";

    const CODEGEN_YAML: &str = "--- !Analysis
Pass:            size-info
Name:            FunctionMISizeChange
Function:        caller
Args:
  - Pass:            AArch64 Instruction Selection
  - MIInstrsBefore:  '0'
  - MIInstrsAfter:   '100'
  - Delta:           '100'
...
--- !Analysis
Pass:            size-info
Name:            FunctionMISizeChange
Function:        caller
Args:
  - Pass:            Tail Duplication
  - MIInstrsBefore:  '100'
  - MIInstrsAfter:   '120'
  - Delta:           '20'
...
--- !Analysis
Pass:            size-info
Name:            FunctionMISizeChange
Function:        caller
Args:
  - Pass:            Live DEBUG_VALUE analysis
  - MIInstrsBefore:  '120'
  - MIInstrsAfter:   '1120'
  - Delta:           '1000'
...
--- !Analysis
Pass:            size-info
Name:            FunctionMISizeChange
Function:        caller
Args:
  - Pass:            Machine code sinking
  - MIInstrsBefore:  '120'
  - MIInstrsAfter:   '110'
  - Delta:           '-10'
...
--- !Analysis
Pass:            prologepilog
Name:            StackSize
Function:        caller
Args:
  - NumStackBytes:   '4096'
...
";

    const LTO_LOCATION_YAML: &str = "--- !Passed
Pass:            loop-unroll
Name:            FullyUnrolled
DebugLoc:        { File: 'src/de.rs', Line: 7, Column: 1 }
Function:        _ZN10serde_json2deE
Args:
  - UnrollCount:     '4'
...
";

    #[test]
    fn parses_remark_documents() {
        let remarks: Vec<_> = documents(YAML.as_bytes()).collect::<io::Result<_>>().unwrap();
        assert_eq!(remarks.len(), 3);
        assert_eq!((remarks[0].file.as_str(), remarks[0].line), ("/work/space/src/lib.rs", 12));
        assert_eq!(remarks[0].function, "_ZN1a1fE");
        assert!(matches!(expansion(&remarks[0]), Some(Expansion::Unrolled(12))));
        assert_eq!((remarks[1].file.as_str(), remarks[1].line), ("/work/space/src/lib.rs", 40));
        assert!(matches!(expansion(&remarks[1]), Some(Expansion::Vectorized(4, 2))));
        // A missed remark is not an expansion.
        assert!(!remarks[2].passed);
        assert!(expansion(&remarks[2]).is_none());

        assert_eq!(
            debug_loc_fields("{ File: 'a b.rs', Line: 3, Column: 1 }"),
            ("a b.rs".to_owned(), 3)
        );
    }

    #[test]
    fn keeps_reading_arguments_after_nested_debug_locations() {
        let remarks: Vec<_> =
            documents(PIPELINE_YAML.as_bytes()).collect::<io::Result<_>>().unwrap();
        assert_eq!(remarks.len(), 3);
        assert_eq!(remarks[0].arg("Callee"), Some("tiny"));
        assert_eq!(remarks[0].arg("Caller"), Some("caller"));
        assert_eq!(remarks[0].arg("Cost"), Some("12"));
        assert_eq!(remarks[0].arg("Threshold"), Some("50"));
        assert!(remarks[2].missed);
    }

    #[test]
    fn ranks_linked_inlining_machine_growth_and_stack_frames() {
        let dir = std::env::temp_dir().join(format!(
            "cargo-bsize-remarks-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("probe.hash-cgu.0.opt.opt.yaml"), PIPELINE_YAML).unwrap();
        std::fs::write(dir.join("probe.hash-cgu.0.codegen.opt.yaml"), CODEGEN_YAML).unwrap();

        let crate_dirs: FxHashMap<String, PathBuf> =
            [("probe".to_owned(), dir.clone())].into_iter().collect();
        let linked: FxHashMap<String, u64> = [("caller".to_owned(), 1000)].into_iter().collect();
        let report = analyze(&dir, &dir, &crate_dirs, &linked, 20).unwrap();

        assert_eq!(
            (report.inlining.passed, report.inlining.forced, report.inlining.missed),
            (2, 1, 1)
        );
        assert_eq!(report.inlining.callers[0].linked_bytes, 1000);
        assert_eq!(report.inlining.callers[0].sites, 2);
        assert_eq!(report.inlining.callees[0].sites, 2);
        assert_eq!(report.inlining.workspace_sites.len(), 2);

        assert_eq!(report.size_changes, 2);
        let function = &report.machine_functions[0];
        assert_eq!((function.added, function.removed), (20, 10));
        assert_eq!(report.growth_passes[0].name, "Tail Duplication");
        assert_eq!(report.growth_passes[0].added, 20);
        assert!(!report.growth_passes.iter().any(|pass| pass.name.contains("DEBUG")));

        assert_eq!(report.stack_functions, 1);
        assert_eq!(report.stack_frames[0].stack_bytes, 4096);

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn resolves_lto_relative_paths_from_the_functions_crate() {
        let root = std::env::temp_dir().join(format!(
            "cargo-bsize-lto-location-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let remarks = root.join("remarks");
        let workspace = root.join("workspace");
        let dependency = root.join("registry/src/index/serde-json");
        std::fs::create_dir_all(&remarks).unwrap();
        std::fs::create_dir_all(&workspace).unwrap();
        std::fs::create_dir_all(&dependency).unwrap();
        std::fs::write(remarks.join("cargo_bsize.hash-cgu.0.opt.opt.yaml"), LTO_LOCATION_YAML)
            .unwrap();

        let crate_dirs: FxHashMap<String, PathBuf> =
            [("cargo_bsize".to_owned(), workspace.clone()), ("serde_json".to_owned(), dependency)]
                .into_iter()
                .collect();
        let report = analyze(&remarks, &workspace, &crate_dirs, &FxHashMap::default(), 20).unwrap();

        assert_eq!(report.unrolled, 1);
        assert!(report.workspace_sites.is_empty());
        std::fs::remove_dir_all(&root).unwrap();
    }
}
