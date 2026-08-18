//! The read-only-data and section cost of panic, formatting, and unwinding
//! infrastructure — and the build levers that remove it.
//!
//! The code views show where panic and format calls are; the unwind sections
//! and the read-only data those calls lean on are counted nowhere else. A panic
//! site loads a `core::panic::Location`; a `tracing` log site emits a static
//! `__CALLSITE` metadata record; both sit in the constant sections, sized
//! exactly once DWARF is read. Consolidating them names a lever a code view
//! cannot: `panic="abort"` drops the unwind tables, `panic_immediate_abort`
//! strips the locations, disabling `tracing` removes the callsite metadata.

use std::{collections::HashMap, hash::BuildHasher};

use object::{Object, ObjectSection};
use serde::Serialize;

use crate::{
    sections::Category,
    symbols::{Total, sized_symbols},
};

#[derive(Debug, Serialize)]
pub struct OverheadReport {
    /// Unwind and exception-table sections (`eh_frame`, `gcc_except_tab`, …).
    pub unwind: u64,

    /// Read-only data that is panic / format / tracing infrastructure, by kind.
    /// The individual symbols are already in the data-symbol view; here only the
    /// rollup matters, since this data is typically many small records.
    pub data: Vec<InfraGroup>,
}

#[derive(Debug, Serialize)]
pub struct InfraGroup {
    pub kind: String,
    pub bytes: u64,
    pub symbols: usize,
}

/// Total the panic/format/unwind infrastructure in `file`. `static_sizes`
/// supplies exact data sizes from DWARF.
pub fn analyze<S: BuildHasher>(
    file: &object::File<'_>,
    static_sizes: &HashMap<String, u64, S>,
) -> OverheadReport {
    let unwind = file
        .sections()
        .filter_map(|section| {
            let (_, size) = section.file_range()?;
            (Category::of(section.name().ok()?) == Category::Unwind).then_some(size)
        })
        .sum();

    let (_, data) = sized_symbols(file, static_sizes);
    let mut groups: HashMap<&'static str, Total> = HashMap::new();
    for symbol in &data {
        if let Some(kind) = classify(&symbol.name) {
            groups.entry(kind).or_default().add(symbol.size);
        }
    }

    let mut data: Vec<InfraGroup> = groups
        .into_iter()
        .map(|(kind, total)| InfraGroup {
            kind: kind.to_owned(),
            bytes: total.bytes,
            symbols: total.count,
        })
        .collect();
    data.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.kind.cmp(&b.kind)));

    OverheadReport { unwind, data }
}

/// The infrastructure a read-only-data symbol belongs to, if any, judged by its
/// demangled name.
fn classify(name: &str) -> Option<&'static str> {
    if name.contains("__CALLSITE") || name.contains("tracing") || name.contains("_METADATA") {
        Some("tracing metadata")
    } else if name.contains("panic") {
        Some("panic")
    } else if name.contains("::fmt::") {
        Some("formatting")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::classify;

    #[test]
    fn classifies_infrastructure_data() {
        assert_eq!(
            classify("<oxlint::lsp::ServerLinter>::actions::__CALLSITE::META"),
            Some("tracing metadata")
        );
        assert_eq!(
            classify("<tracing_subscriber::registry::DataInner>::default::NULL_METADATA"),
            Some("tracing metadata")
        );
        assert_eq!(classify("core::panicking::panic_fmt::MESSAGE"), Some("panic"));
        assert_eq!(classify("core::fmt::num::DEC_DIGITS_LUT"), Some("formatting"));
        assert_eq!(classify("oxc_linter::generated::rules_enum::RULE_NAMES"), None);
    }
}
