//! Code by the workspace file it is defined in.
//!
//! Crates are the coarsest view and functions the finest; between them sit the
//! files and directories the reader actually edits. The join is the definition
//! site the DWARF walk already collected per function, against the symbol
//! table's code sizes — so a generated file shows the full cost of what it
//! generates, and a directory of lint rules shows what the set costs together.
//!
//! Code inlined away is charged to its caller's file, like every symbol view;
//! the inlined section's line and origin tables carry that remainder.

use rustc_hash::FxHashMap;

use crate::{dwarf::Site, name::demangle};

#[derive(Debug)]
pub struct FileReport {
    /// Bytes in functions defined in this workspace, and how many of the
    /// binary's functions that is.
    pub bytes: u64,
    pub functions: usize,

    /// Workspace files by the code defined in them, largest first.
    pub files: Vec<Location>,

    /// Their parent directories, likewise.
    pub directories: Vec<Location>,
}

#[derive(Debug)]
pub struct Location {
    pub path: String,
    pub bytes: u64,
    pub functions: usize,
}

/// Sum each symbol's bytes into its definition file and that file's directory,
/// for definitions in this workspace.
pub fn analyze(
    sites: &FxHashMap<String, Site>,
    code_sizes: &FxHashMap<String, u64>,
    limit: usize,
) -> FileReport {
    let mut files: FxHashMap<&str, Location> = FxHashMap::default();
    let mut bytes = 0;
    let mut functions = 0;

    for (mangled, &size) in code_sizes {
        let name = demangle(mangled);
        // The assembler suffixes duplicated bodies (`fmt.155`); the site is
        // recorded under the plain name.
        let site = sites.get(&name).or_else(|| {
            let (base, copy) = name.rsplit_once('.')?;
            copy.bytes().all(|byte| byte.is_ascii_digit()).then(|| sites.get(base))?
        });
        let Some(site) = site.filter(|site| site.workspace) else { continue };

        bytes += size;
        functions += 1;
        let entry = files.entry(site.file.as_str()).or_insert_with(|| Location {
            path: site.file.clone(),
            bytes: 0,
            functions: 0,
        });
        entry.bytes += size;
        entry.functions += 1;
    }

    let mut directories: FxHashMap<&str, Location> = FxHashMap::default();
    for file in files.values() {
        let parent = file.path.rsplit_once('/').map_or("", |(parent, _)| parent);
        let entry = directories.entry(parent).or_insert_with(|| Location {
            path: parent.to_owned(),
            bytes: 0,
            functions: 0,
        });
        entry.bytes += file.bytes;
        entry.functions += file.functions;
    }

    // The directory keys borrow the file paths, so they rank first.
    let directories = rank(directories, limit);
    let files = rank(files, limit);

    FileReport { bytes, functions, files, directories }
}

fn rank(map: FxHashMap<&str, Location>, limit: usize) -> Vec<Location> {
    let mut locations: Vec<Location> = map.into_values().collect();
    locations.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.path.cmp(&b.path)));
    locations.truncate(limit);
    locations
}

#[cfg(test)]
mod tests {
    use rustc_hash::FxHashMap;

    use super::analyze;
    use crate::dwarf::Site;

    #[test]
    fn sums_code_into_files_and_directories() {
        let site = |file: &str, workspace| Site { file: file.to_owned(), line: 1, workspace };
        let sites: FxHashMap<String, Site> = [
            ("a::one".to_owned(), site("crates/a/src/x.rs", true)),
            ("a::two".to_owned(), site("crates/a/src/x.rs", true)),
            ("a::three".to_owned(), site("crates/a/src/sub/y.rs", true)),
            ("dep::f".to_owned(), site("dep-1.0.0/src/lib.rs", false)),
        ]
        .into_iter()
        .collect();
        // `a::two` only links as a suffixed copy; `dep::f` is not workspace
        // code; `a::unknown` has no site.
        let sizes: FxHashMap<String, u64> = [
            ("_RNvC1a3one", 100),
            ("_RNvC1a3two.7", 40),
            ("_RNvC1a5three", 10),
            ("_RNvC3dep1f", 1000),
            ("_RNvC1a7unknown", 1),
        ]
        .into_iter()
        .map(|(name, size)| (name.to_owned(), size))
        .collect();

        let report = analyze(&sites, &sizes, 10);

        assert_eq!((report.bytes, report.functions), (150, 3));
        let files: Vec<(&str, u64, usize)> = report
            .files
            .iter()
            .map(|location| (location.path.as_str(), location.bytes, location.functions))
            .collect();
        assert_eq!(files, [("crates/a/src/x.rs", 140, 2), ("crates/a/src/sub/y.rs", 10, 1)]);
        let directories: Vec<(&str, u64)> = report
            .directories
            .iter()
            .map(|location| (location.path.as_str(), location.bytes))
            .collect();
        assert_eq!(directories, [("crates/a/src", 140), ("crates/a/src/sub", 10)]);
    }
}
