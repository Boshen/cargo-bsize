use std::path::PathBuf;

use crate::{CargoBsize, CargoBsizeOptions, output::OutputFormat};

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
        "dup\n  1.0.0 — used by a v0.1.0\n  2.0.0 — used by b v0.1.0\n\n1 duplicate dependency\n"
    );
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
