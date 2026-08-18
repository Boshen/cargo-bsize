//! The source text behind the workspace rows.
//!
//! A `file:line` names where to look; the line itself says what is there — an
//! `unwrap()`, a `format!`, a `.collect::<Vec<_>>()` — and a reader deciding
//! which change to make wants that without opening the file. Only workspace
//! files are read: those are the lines a reader can edit, and the only ones the
//! report spells with a path that resolves here.

use std::{
    fs,
    path::{Path, PathBuf},
};

use rustc_hash::FxHashMap;

use crate::output::Report;

/// The longest snippet kept; the rest is elided.
const MAX_CHARS: usize = 100;

/// Reads source lines, each file once.
pub(crate) struct Snippets {
    workspace: PathBuf,
    cache: FxHashMap<PathBuf, Option<Vec<String>>>,
}

impl Snippets {
    pub(crate) fn new(workspace: &Path) -> Self {
        Self { workspace: workspace.to_owned(), cache: FxHashMap::default() }
    }

    /// The text of `line` (1-based) in `path`, resolved against the workspace
    /// root when relative; `None` when the file or line is not there.
    pub(crate) fn line(&mut self, path: &str, line: u64) -> Option<String> {
        let path = self.workspace.join(path);
        let lines = self
            .cache
            .entry(path.clone())
            .or_insert_with(|| {
                fs::read(&path)
                    .ok()
                    .map(|bytes| String::from_utf8_lossy(&bytes).lines().map(clean).collect())
            })
            .as_ref()?;
        let index = usize::try_from(line.checked_sub(1)?).ok()?;
        lines.get(index).filter(|text| !text.is_empty()).cloned()
    }
}

/// One line as the report shows it: surrounding whitespace dropped, inner runs
/// collapsed, and cut to `MAX_CHARS` with an ellipsis.
fn clean(text: &str) -> String {
    let mut out = String::new();
    for word in text.split_whitespace() {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    if out.chars().count() > MAX_CHARS {
        let cut = out.char_indices().nth(MAX_CHARS - 1).map_or(out.len(), |(index, _)| index);
        out.truncate(cut);
        out.push('\u{2026}');
    }
    out
}

/// Fill in the source text for every workspace row that names a line: the
/// assembly's source lines and the inlined view's call sites.
pub(crate) fn attach(report: &mut Report, workspace: &Path) {
    let mut snippets = Snippets::new(workspace);

    if let Some(assembly) = &mut report.assembly {
        for line in &mut assembly.workspace_lines {
            line.snippet = snippets.line(&line.file, line.line);
        }
    }
    if let Some(inlined) = &mut report.inlined {
        for site in &mut inlined.workspace_call_sites {
            site.snippet = snippets.line(&site.file, site.line);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{Snippets, clean};

    #[test]
    fn reads_a_workspace_line_once_and_cleans_it() {
        let mut snippets = Snippets::new(Path::new(env!("CARGO_MANIFEST_DIR")));
        // This file's first line, and a line that is not there.
        assert_eq!(
            snippets.line("src/snippets.rs", 1).as_deref(),
            Some("//! The source text behind the workspace rows.")
        );
        assert_eq!(snippets.line("src/snippets.rs", 0), None);
        assert_eq!(snippets.line("src/snippets.rs", 1_000_000), None);
        assert_eq!(snippets.line("src/does-not-exist.rs", 1), None);
        assert_eq!(snippets.cache.len(), 2);

        assert_eq!(clean("   let x =   foo( bar );  "), "let x = foo( bar );");
        let long = clean(&"a".repeat(150));
        assert_eq!(long.chars().count(), 100);
        assert!(long.ends_with('\u{2026}'));
    }
}
