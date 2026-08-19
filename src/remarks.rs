//! The loops the optimizer expanded, from LLVM's optimization remarks.
//!
//! Unrolling copies a loop body, peeling copies its first iterations,
//! vectorizing adds a wide body and a scalar remainder beside it: each is code
//! the source never wrote, and each is a decision `-Cremark=loop-unroll
//! -Cremark=loop-vectorize` reports with the function and the source line of
//! the loop. Ranked by function and listed for the workspace's own lines, they
//! name the loops worth simplifying, moving behind `#[inline(never)]`, or
//! marking `#[cold]` — and the functions where `opt-level="z"` would give the
//! most back.
//!
//! The remarks come per crate, from each crate's own optimization pipeline
//! (`-Zremark-dir`, through `RUSTC_BOOTSTRAP=1`); what a fat LTO pass decides
//! afterwards is not reported. The YAML is read with a small parser of its own:
//! documents of `Key: value` lines and an `Args` list, nothing more.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use rustc_hash::FxHashMap;

use crate::{
    assembly::{Origin, source},
    name::demangle,
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
    pass: String,
    name: String,
    function: String,
    file: String,
    line: u64,
    args: Vec<(String, String)>,
}

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
    limit: usize,
) -> Result<RemarksReport> {
    let mut paths: Vec<_> = fs::read_dir(dir)
        .with_context(|| format!("failed to read {}", dir.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "yaml"))
        .collect();
    paths.sort();

    let mut functions: FxHashMap<String, ExpandedFunction> = FxHashMap::default();
    let mut sites: FxHashMap<(String, u64, String, String), (usize, Origin)> = FxHashMap::default();
    let (mut unrolled, mut peeled, mut vectorized) = (0, 0, 0);
    let mut files = 0;
    for path in paths {
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        files += 1;
        // `<crate>.<hash>-cgu.N.<stage>.opt.yaml`: the crate whose files a
        // relative path is relative to.
        let krate = path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.split('.').next())
            .unwrap_or_default();
        let crate_dir = crate_dirs.get(krate);
        for remark in documents(&text) {
            let Some(detail) = expansion(&remark) else { continue };
            let name = demangle(&remark.function);
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
            if !remark.file.is_empty() {
                let absolute = match crate_dir {
                    Some(dir) if !remark.file.starts_with('/') => {
                        dir.join(&remark.file).display().to_string()
                    }
                    _ => remark.file.clone(),
                };
                let (file, origin) = source(&absolute, workspace);
                let key = (file, remark.line, name, detail.to_string());
                let site = sites.entry(key).or_insert((0, origin));
                site.0 += 1;
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

    Ok(RemarksReport { files, unrolled, peeled, vectorized, functions, workspace_sites })
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
    let arg = |key: &str| {
        remark.args.iter().find(|(name, _)| name == key).and_then(|(_, value)| value.parse().ok())
    };
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
/// one line or two, and an `Args:` list of `- Key: value` items.
fn documents(text: &str) -> Vec<Remark> {
    let mut remarks = Vec::new();
    let mut current: Option<Remark> = None;
    let mut in_args = false;
    let mut debug_loc = String::new();

    let flush = |current: &mut Option<Remark>, remarks: &mut Vec<Remark>| {
        if let Some(remark) = current.take() {
            remarks.push(remark);
        }
    };

    for line in text.lines() {
        if let Some(kind) = line.strip_prefix("--- ") {
            flush(&mut current, &mut remarks);
            current = Some(Remark { passed: kind.trim() == "!Passed", ..Remark::default() });
            in_args = false;
            debug_loc.clear();
            continue;
        }
        let Some(remark) = current.as_mut() else { continue };
        if line.starts_with("...") {
            flush(&mut current, &mut remarks);
            continue;
        }

        // A DebugLoc may wrap onto a second line; gather it to the brace.
        if !debug_loc.is_empty() {
            debug_loc.push(' ');
            debug_loc.push_str(line.trim());
            if line.contains('}') {
                (remark.file, remark.line) = debug_loc_fields(&debug_loc);
                debug_loc.clear();
            }
            continue;
        }

        if in_args {
            if let Some(item) = line.trim_start().strip_prefix("- ")
                && let Some((key, value)) = item.split_once(':')
            {
                remark.args.push((key.trim().to_owned(), unquote(value)));
                continue;
            }
            in_args = false;
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
                    debug_loc.push_str(value);
                }
            }
            "Args" => in_args = true,
            _ => {}
        }
    }
    flush(&mut current, &mut remarks);
    remarks
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
    use super::{Expansion, debug_loc_fields, documents, expansion};

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

    #[test]
    fn parses_remark_documents() {
        let remarks = documents(YAML);
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
}
