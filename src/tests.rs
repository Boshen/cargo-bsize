use std::path::PathBuf;

use rustc_hash::FxHashMap;

use crate::{
    CargoBsize, CargoBsizeOptions, features, name,
    output::OutputFormat,
    sections::{self, Category},
    symbols,
};

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/duplicates")
}

fn run(format: OutputFormat) -> String {
    let mut written = Vec::new();
    let _code =
        CargoBsize::new(&mut written, CargoBsizeOptions::new(fixture()).with_format(format)).run();
    String::from_utf8(written).expect("invalid UTF-8")
}

/// The fixture resolves `dup` to four versions, but 3.0.0 is reachable only
/// through a dev-dependency and 4.0.0 only through a proc-macro, so neither
/// links. `cargo tree --duplicates` reports all four.
#[test]
fn reports_only_versions_that_link() {
    assert_eq!(
        run(OutputFormat::Text),
        "agent instructions\n  Only propose source-code changes. Do not propose configuration changes.\n\n1 duplicate dependency\n  (the same crate at several versions; each ships its own copy of the code)\ndup\n  1.0.0 — used by a v0.1.0\n  2.0.0 — used by b v0.1.0\n"
    );
}

/// The fixture's `dup` 1.0.0 turns on a default feature; `a` asks for it with
/// default features on. `dup` 2.0.0 has no features and is left out; the
/// dev-only and proc-macro-only versions do not link at all.
#[test]
fn reports_the_features_each_linked_dependency_was_built_with() {
    let metadata = cargo_metadata::MetadataCommand::new()
        .current_dir(fixture())
        .exec()
        .expect("cargo metadata");
    let report = features::analyze(&metadata, None, 20).expect("features");

    let crates: Vec<(&str, &str, &[String], bool)> = report
        .crates
        .iter()
        .map(|krate| {
            (krate.name.as_str(), krate.version.as_str(), &krate.features[..], krate.default)
        })
        .collect();
    assert_eq!(crates, [("dup", "1.0.0", &["default".to_owned(), "extra".to_owned()][..], true)]);
    let requester = &report.crates[0].requested_by[0];
    assert_eq!(
        (requester.name.as_str(), requester.default, requester.features.is_empty()),
        ("a", true, true)
    );
    assert_eq!(report.crates[0].bytes, None);
}

/// The test binary itself exercises real Mach-O/ELF parsing with no fixture.
#[test]
fn section_and_symbol_sizes_reconcile() {
    let path = std::env::current_exe().expect("no current exe");
    let data = std::fs::read(&path).expect("failed to read");
    let file = object::File::parse(&*data).expect("failed to parse");

    let sections = sections::analyze(&file, &path, data.len() as u64);
    assert_eq!(sections.total, data.len() as u64);
    assert_eq!(sections.accounted + sections.other, sections.total);

    // Symbols only cover the sections they live in, never more.
    let attributable: u64 = sections
        .categories
        .iter()
        .filter(|entry| matches!(entry.category, Category::Code | Category::ReadOnlyData))
        .map(|entry| entry.size)
        .sum();

    let symbols = symbols::analyze(&file, &FxHashMap::default(), 20);
    let attributed = symbols.code.bytes + symbols.data.bytes;
    assert!(attributed <= attributable, "{attributed} > {attributable}");
    assert!(symbols.crates.iter().any(|entry| entry.name == "cargo_bsize"));
}

#[test]
fn reads_the_instantiating_crate_from_v0_mangling() {
    // A generic defined in `tower_lsp_server` that `oxlint` instantiated.
    let cross_crate = "_RINvMNtNtCsfXhDYECNaj2_16tower_lsp_server7jsonrpc7requestNtB3_7Request12from_requestNtNtCs7amwG967rZS_8ls_types7request18ApplyWorkspaceEditECsk86NwXxcbD0_6oxlint";
    assert_eq!(name::instantiating_crate(cross_crate).as_deref(), Some("oxlint"));

    // No trailing crate: v0 omits it when a crate instantiates its own generic.
    assert_eq!(name::instantiating_crate("_RNvNtCs1234_4core3fmt5write"), None);
}

#[test]
fn renders_json() {
    let report: serde_json::Value =
        serde_json::from_str(&run(OutputFormat::Json)).expect("output was not valid JSON");

    assert_eq!(report["duplicates"][0]["name"], "dup");
    assert_eq!(report["duplicates"][0]["versions"][0]["version"], "1.0.0");
    assert_eq!(report["duplicates"][0]["versions"][0]["dependents"][0]["name"], "a");
    assert!(report["duplicates"][0]["versions"][2].is_null());
}
