//! Where the code was generated: bytes per crate and version, from the compile
//! units in DWARF.
//!
//! A mangled name says which crate defines a function; it does not say which
//! version of that crate, and a duplicated dependency ships one copy per
//! version. Each compile unit rustc emits names its crate and, through the
//! crate's root source path, the checkout it came from —
//! `…/registry/src/<index>/regex-1.10.2/src/lib.rs/@/regex.<hash>-cgu.0` — and
//! every out-of-line function's DIE sits in the unit that emitted it, with its
//! address range. Summing those ranges puts bytes on every version, and whatever
//! the units leave uncovered is code that came with no debug info: C and
//! assembly objects, linker stubs.
//!
//! The unit's own `DW_AT_low_pc`/`high_pc` cannot be used: under LTO the
//! functions of many units interleave in `.text`, and a unit's span then covers
//! everyone else's code too. The per-function ranges come from the type walk,
//! which visits every DIE anyway.

use std::path::Path;

use object::{Object, ObjectSection};
use rustc_hash::FxHashMap;

use crate::{
    duplicates::Duplicate,
    dwarf::{FunctionRange, Site, UnitInfo},
    name::generic_family,
    output::Report,
    sections::Category,
};

#[derive(Debug)]
pub struct ProvenanceReport {
    /// Code bytes no compile unit covers: objects built without debug info (C,
    /// assembly, std's backtrace crates), linker-generated stubs.
    pub uncovered: u64,
}

/// Attribute the code sections of `file` to the crates behind `units`, by the
/// function ranges the DIE walk found in each. `workspace` is the workspace
/// root.
#[must_use]
pub fn analyze(
    file: &object::File<'_>,
    units: &[UnitInfo],
    functions: &[FunctionRange],
    workspace: &Path,
) -> Provenance {
    let mut code: Vec<(u64, u64)> = file
        .sections()
        .filter(|section| {
            section.name().ok().is_some_and(|name| Category::of(name) == Category::Code)
        })
        .map(|section| (section.address(), section.address() + section.size()))
        .collect();
    code.sort_unstable();
    let code_bytes: u64 = code.iter().map(|&(begin, end)| end - begin).sum();

    let ranges = functions.iter().map(|range| (range.begin, range.end, range.unit)).collect();
    let per_unit = attribute(ranges, &code, units.len());
    let covered: u64 = per_unit.iter().sum();

    // Bytes per versioned crate, for the duplicate and feature views to join.
    let mut versions: FxHashMap<(String, String), u64> = FxHashMap::default();
    for (unit, bytes) in units.iter().zip(per_unit) {
        let unit = Unit::of(unit, workspace);
        if let Some(version) = unit.version {
            *versions.entry((unit.name, version)).or_default() += bytes;
        }
    }

    Provenance {
        report: ProvenanceReport { uncovered: code_bytes.saturating_sub(covered) },
        versions,
    }
}

/// What the read produced: the report, plus bytes for every versioned crate so
/// the duplicate-dependency and feature views can be costed.
pub struct Provenance {
    pub report: ProvenanceReport,
    versions: FxHashMap<(String, String), u64>,
}

impl Provenance {
    /// Put bytes on each duplicated version: the crate name matched with hyphens
    /// read as underscores, the version exactly.
    pub fn cost_duplicates(&self, duplicates: &mut [Duplicate]) {
        for duplicate in duplicates {
            let name = duplicate.name.replace('-', "_");
            for version in &mut duplicate.versions {
                version.bytes = self.bytes_of_version(&name, &version.version);
            }
        }
    }

    /// The code bytes of one crate version, by the crate name as rustc spells
    /// it (underscores) and the version.
    #[must_use]
    pub fn bytes_of_version(&self, name: &str, version: &str) -> Option<u64> {
        self.versions.get(&(name.to_owned(), version.to_owned())).copied()
    }
}

/// Put a definition site on every row that names a function: the largest
/// functions, generic families (any instantiation's site — they share the
/// generic's), inlined functions, and single-caller functions. `sites` is
/// keyed by demangled name.
pub fn attach(report: &mut Report, sites: &FxHashMap<String, Site>) {
    let site = |name: &str| sites.get(name).map(Site::display);

    if let Some(symbols) = &mut report.symbols {
        for symbol in &mut symbols.code.largest {
            symbol.defined_at = site(&symbol.name);
        }
        // A family's name has its turbofish stripped; find one instantiation.
        if !symbols.generics.is_empty() {
            let mut families: FxHashMap<String, String> = FxHashMap::default();
            for (name, site) in sites {
                if name.contains("::<") {
                    families.entry(generic_family(name)).or_insert_with(|| site.display());
                }
            }
            for family in &mut symbols.generics {
                family.defined_at =
                    families.get(&family.name).cloned().or_else(|| site(&family.name));
            }
        }
    }
    if let Some(inlined) = &mut report.inlined {
        for function in &mut inlined.functions {
            function.defined_at = site(&function.name);
        }
    }
    if let Some(graph) = &mut report.graph {
        for single in &mut graph.single_callers {
            single.defined_at = site(&single.name);
        }
    }
}

/// Bytes each unit's ranges cover inside the code sections, with an address
/// claimed by two units given to the first — the second is a folded copy.
fn attribute(mut ranges: Vec<(u64, u64, usize)>, code: &[(u64, u64)], units: usize) -> Vec<u64> {
    let mut bytes = vec![0; units];
    ranges.sort_unstable();

    let mut cursor = 0;
    for (begin, end, unit) in ranges {
        let begin = begin.max(cursor);
        if end <= begin {
            continue;
        }
        cursor = end;
        bytes[unit] += within(begin, end, code);
    }
    bytes
}

/// How much of `[begin, end)` falls inside the sorted `sections`.
fn within(begin: u64, end: u64, sections: &[(u64, u64)]) -> u64 {
    sections.iter().map(|&(start, stop)| end.min(stop).saturating_sub(begin.max(start))).sum()
}

/// What a compile unit's header says about its crate.
struct Unit {
    name: String,
    version: Option<String>,
}

impl Unit {
    fn of(unit: &UnitInfo, workspace: &Path) -> Self {
        if !unit.rust {
            return Self { name: unit.name.clone(), version: None };
        }
        crate_of(&unit.name, unit.comp_dir.as_deref(), workspace)
    }
}

/// Read a crate name and version out of a Rust unit's name — the crate root's
/// source path, then `/@/<crate>.<hash>-cgu.N` — and its compile directory. A
/// workspace crate has no version to read; a registry or vendored checkout
/// spells it in its directory.
fn crate_of(name: &str, comp_dir: Option<&str>, workspace: &Path) -> Unit {
    let (root, unit) = name.split_once("/@/").unwrap_or((name, ""));
    let krate = unit.split('.').next().filter(|krate| !krate.is_empty()).map_or_else(
        || root.rsplit('/').next().unwrap_or(root).trim_end_matches(".rs").to_owned(),
        str::to_owned,
    );

    // The unit's directory completes a relative root; std reports `/rustc/<hash>`.
    let absolute = match comp_dir {
        Some(dir) if !root.starts_with('/') => format!("{dir}/{root}"),
        _ => root.to_owned(),
    };
    let version = if Path::new(&absolute).strip_prefix(workspace).is_ok() {
        None
    } else {
        version_in(&absolute, &krate)
    };

    Unit { name: krate, version }
}

/// The version spelled by a `<crate>-<semver>` component of `path`, matched to
/// `krate` with hyphens read as underscores — the registry and vendored
/// checkout layout.
fn version_in(path: &str, krate: &str) -> Option<String> {
    path.split('/').find_map(|component| {
        let (name, version) = component.rsplit_once('-')?;
        // A pre-release or build tag may itself contain `-`, so widen leftward
        // until the name matches: `foo-bar-1.0.0-beta.1`.
        let mut name = name;
        let mut version = version.to_owned();
        loop {
            if is_semver(&version) && name.replace('-', "_") == krate {
                return Some(version);
            }
            let (shorter, tail) = name.rsplit_once('-')?;
            version = format!("{tail}-{version}");
            name = shorter;
        }
    })
}

fn is_semver(text: &str) -> bool {
    let mut parts = text.split(['-', '+']).next().unwrap_or(text).split('.');
    let numeric = |part: Option<&str>| part.is_some_and(|part| part.parse::<u64>().is_ok());
    numeric(parts.next())
        && numeric(parts.next())
        && numeric(parts.next())
        && parts.next().is_none()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{Provenance, ProvenanceReport, attribute, crate_of, version_in};
    use crate::duplicates::{Duplicate, DuplicateVersion};

    #[test]
    fn reads_crate_and_version_from_the_unit_name() {
        let workspace = Path::new("/work/space");

        let unit = crate_of(
            "/home/u/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/regex-syntax-0.8.11/src/lib.rs/@/regex_syntax.972c148c546c0ea6-cgu.0",
            Some(
                "/home/u/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/regex-syntax-0.8.11",
            ),
            workspace,
        );
        assert_eq!((unit.name.as_str(), unit.version.as_deref()), ("regex_syntax", Some("0.8.11")));

        // A workspace crate: relative root, workspace compile directory, and no
        // version even if a directory happens to spell one.
        let unit =
            crate_of("src/main.rs/@/probe.8833b9212e0f81bc-cgu.0", Some("/work/space"), workspace);
        assert_eq!((unit.name.as_str(), unit.version.as_deref()), ("probe", None));
        let unit =
            crate_of("src/lib.rs/@/probe.abc-cgu.0", Some("/work/space/probe-1.0.0"), workspace);
        assert_eq!(unit.version, None);

        // std, spelled relative to `/rustc/<hash>`.
        let unit =
            crate_of("library/std/src/lib.rs/@/std.abc-cgu.0", Some("/rustc/deadbeef"), workspace);
        assert_eq!((unit.name.as_str(), unit.version.as_deref()), ("std", None));

        // A crate vendored into std carries its version.
        let unit = crate_of(
            "/rust/deps/addr2line-0.25.1/src/lib.rs/@/addr2line.abc-cgu.0",
            Some("/rust/deps/addr2line-0.25.1"),
            workspace,
        );
        assert_eq!(unit.version.as_deref(), Some("0.25.1"));

        // A git checkout has no version in its path.
        let unit = crate_of(
            "/home/u/.cargo/git/checkouts/foo-1a2b3c/9f8e7d/src/lib.rs/@/foo.abc-cgu.0",
            None,
            workspace,
        );
        assert_eq!(unit.version, None);
    }

    #[test]
    fn matches_a_version_component_to_the_crate() {
        assert_eq!(version_in("/v/regex-1.10.2/src/lib.rs", "regex"), Some("1.10.2".to_owned()));
        assert_eq!(
            version_in("/v/foo-bar-1.0.0-beta.1/src/lib.rs", "foo_bar"),
            Some("1.0.0-beta.1".to_owned())
        );
        // The name must match: `regex-1.10.2` is not `regex_syntax`.
        assert_eq!(version_in("/v/regex-1.10.2/src/lib.rs", "regex_syntax"), None);
        assert_eq!(version_in("/rustc/abc/library/core/src/lib.rs", "core"), None);
    }

    #[test]
    fn costs_each_duplicated_version_by_crate_and_version() {
        let provenance = Provenance {
            report: ProvenanceReport { uncovered: 0 },
            versions: [
                (("regex_syntax".to_owned(), "0.8.11".to_owned()), 168_164),
                (("regex_syntax".to_owned(), "0.7.5".to_owned()), 90_000),
            ]
            .into_iter()
            .collect(),
        };
        let version = |version: &str| DuplicateVersion {
            version: version.to_owned(),
            dependents: Vec::new(),
            bytes: None,
        };
        let mut duplicates = vec![Duplicate {
            name: "regex-syntax".to_owned(),
            versions: vec![version("0.7.5"), version("0.8.11"), version("0.6.0")],
        }];

        provenance.cost_duplicates(&mut duplicates);
        let bytes: Vec<Option<u64>> =
            duplicates[0].versions.iter().map(|version| version.bytes).collect();
        // The hyphenated package name matches the underscored crate; a version
        // whose code left no unit stays unknown rather than reading as zero.
        assert_eq!(bytes, vec![Some(90_000), Some(168_164), None]);
    }

    #[test]
    fn overlapping_ranges_go_to_the_first_unit_and_stay_inside_code() {
        let code = [(0x1000, 0x2000)];
        // Unit 1 claims [0x1000, 0x1800), unit 0 overlaps it and runs past the
        // section; a tombstoned range at 0 counts for nothing.
        let ranges = vec![(0x1400, 0x2400, 0), (0x1000, 0x1800, 1), (0, 0x100, 1)];
        assert_eq!(attribute(ranges, &code, 2), vec![0x800, 0x800]);
    }
}
