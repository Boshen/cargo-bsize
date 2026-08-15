use std::path::PathBuf;

use cargo_metadata::Metadata;

use crate::{CargoBsize, CargoBsizeOptions, duplicates, output, output::OutputFormat};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures").join(name)
}

fn metadata_for(name: &str) -> Metadata {
    CargoBsize::new(Vec::new(), CargoBsizeOptions::new(fixture(name)))
        .metadata()
        .expect("failed to resolve fixture metadata")
}

/// The fixture resolves `dup` to four versions, but 3.0.0 is reachable only
/// through a dev-dependency and 4.0.0 only through a proc-macro, so neither
/// links into a shipped binary. `cargo tree --duplicates` reports all four;
/// we report two.
#[test]
fn skips_versions_that_never_link() {
    let metadata = metadata_for("duplicates");
    let duplicates = duplicates::find(&metadata).expect("analysis failed");

    assert_eq!(duplicates.len(), 1, "expected `dup` as the only duplicated crate");
    assert_eq!(duplicates[0].name, "dup");

    let versions: Vec<&str> =
        duplicates[0].versions.iter().map(|version| version.version.as_str()).collect();
    assert_eq!(
        versions,
        ["1.0.0", "2.0.0"],
        "dev-only 3.0.0 and proc-macro-only 4.0.0 must not be reported"
    );
}

#[test]
fn reports_who_pulls_each_version() {
    let metadata = metadata_for("duplicates");
    let duplicates = duplicates::find(&metadata).expect("analysis failed");

    let dependents: Vec<Vec<String>> = duplicates[0]
        .versions
        .iter()
        .map(|version| {
            version
                .dependents
                .iter()
                .map(|dependent| format!("{} v{}", dependent.name, dependent.version))
                .collect()
        })
        .collect();

    assert_eq!(dependents, [["a v0.1.0"], ["b v0.1.0"]]);
}

#[test]
fn text_output_says_so_when_clean() {
    let mut written = Vec::new();
    output::render(&mut written, &[], OutputFormat::Text).expect("render failed");

    assert_eq!(String::from_utf8(written).expect("invalid UTF-8"), "no duplicate dependencies\n");
}

#[test]
fn text_output_names_versions_and_dependents() {
    let metadata = metadata_for("duplicates");
    let duplicates = duplicates::find(&metadata).expect("analysis failed");

    let mut written = Vec::new();
    output::render(&mut written, &duplicates, OutputFormat::Text).expect("render failed");
    let written = String::from_utf8(written).expect("invalid UTF-8");

    assert!(written.contains("1.0.0 — used by a v0.1.0"), "unexpected output:\n{written}");
    assert!(written.contains("2.0.0 — used by b v0.1.0"), "unexpected output:\n{written}");
    assert!(written.contains("1 duplicate dependency"), "unexpected output:\n{written}");
}

/// End-to-end through the runner, asserting the JSON shape machine consumers see.
#[test]
fn json_report_has_a_stable_shape() {
    let options = CargoBsizeOptions::new(fixture("duplicates")).with_format(OutputFormat::Json);

    let mut written = Vec::new();
    let _code = CargoBsize::new(&mut written, options).run();
    let report: serde_json::Value =
        serde_json::from_slice(&written).expect("output was not valid JSON");

    assert_eq!(report["duplicates"][0]["name"], "dup");
    assert_eq!(report["duplicates"][0]["versions"][0]["version"], "1.0.0");
    assert_eq!(report["duplicates"][0]["versions"][0]["dependents"][0]["name"], "a");
    assert_eq!(report["duplicates"][0]["versions"][0]["dependents"][0]["version"], "0.1.0");
    assert!(report["duplicates"][0]["versions"][2].is_null(), "dev-only version leaked into JSON");
}
