//! Which features each dependency was built with, and who asked for them.
//!
//! A crate's size is partly a choice made in `Cargo.toml`: `regex` with or
//! without `unicode`, `tokio` with `full` or with three features. The resolved
//! feature set of every linked crate, next to the bytes it contributes, is what
//! a reader needs to propose `default-features = false` and a shorter feature
//! list — the same kind of manifest edit as unifying a duplicated version.
//! Bytes are the crate's whole code, from the compile units, not the features'
//! own share, which nothing measures.

use cargo_metadata::{DependencyKind, Metadata, Package};
use rustc_hash::FxHashMap;
use serde::Serialize;

use crate::{
    duplicates::{Graph, is_linkable},
    provenance::Provenance,
};

#[derive(Debug, Serialize)]
pub struct FeatureReport {
    /// Linked dependencies with at least one feature enabled, largest first.
    pub crates: Vec<CrateFeatures>,
}

#[derive(Debug, Serialize)]
pub struct CrateFeatures {
    pub name: String,
    pub version: String,

    /// Code bytes its compile units generated, when the debug info said.
    pub bytes: Option<u64>,

    /// The features resolved on, in the order cargo lists them.
    pub features: Vec<String>,

    /// Whether `default` is among them.
    pub default: bool,

    /// The linked crates that depend on it, and what each asked for.
    pub requested_by: Vec<Requester>,
}

#[derive(Debug, Serialize)]
pub struct Requester {
    pub name: String,
    pub version: String,

    /// The features it named in its dependency line.
    pub features: Vec<String>,

    /// Whether it left default features on.
    pub default: bool,
}

/// The features of every linked dependency in `metadata`, with bytes from
/// `provenance` when there is one, keeping the `limit` largest.
///
/// # Errors
///
/// Errors when `metadata` carries no dependency resolution.
pub fn analyze(
    metadata: &Metadata,
    provenance: Option<&Provenance>,
    limit: usize,
) -> anyhow::Result<FeatureReport> {
    let graph = Graph::new(metadata)
        .ok_or_else(|| anyhow::anyhow!("`cargo metadata` returned no dependency resolution"))?;

    // Who depends on each linked package, and with what.
    let mut requesters: FxHashMap<&cargo_metadata::PackageId, Vec<Requester>> =
        FxHashMap::default();
    for (id, package) in graph.linked_packages() {
        let Some(node) = graph.node(id) else { continue };
        for dep in node.deps.iter().filter(|dep| is_linkable(dep)) {
            let Some(target) = graph.package(&dep.pkg) else { continue };
            // The dependency line, by the crate's name or the name it was
            // renamed to.
            let line = package.dependencies.iter().find(|line| {
                matches!(line.kind, DependencyKind::Normal)
                    && (line.rename.as_deref() == Some(dep.name.as_str())
                        || (line.rename.is_none() && line.name == target.name.as_str()))
            });
            let Some(line) = line else { continue };
            requesters.entry(&dep.pkg).or_default().push(Requester {
                name: package.name.to_string(),
                version: package.version.to_string(),
                features: line.features.clone(),
                default: line.uses_default_features,
            });
        }
    }

    let workspace: Vec<&cargo_metadata::PackageId> = metadata.workspace_members.iter().collect();
    let mut crates: Vec<CrateFeatures> = graph
        .linked_packages()
        .filter(|(id, _)| !workspace.contains(id))
        .filter_map(|(id, package)| {
            let node = graph.node(id)?;
            let features: Vec<String> = node.features.iter().map(ToString::to_string).collect();
            if features.is_empty() {
                return None;
            }
            let mut requested_by = requesters.remove(id).unwrap_or_default();
            requested_by
                .sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.version.cmp(&b.version)));
            Some(CrateFeatures {
                bytes: provenance.and_then(|provenance| provenance.bytes_of(package)),
                default: features.iter().any(|feature| feature == "default"),
                name: package.name.to_string(),
                version: package.version.to_string(),
                features,
                requested_by,
            })
        })
        .collect();

    crates.sort_by(|a, b| {
        b.bytes
            .cmp(&a.bytes)
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.version.cmp(&b.version))
    });
    crates.truncate(limit);
    Ok(FeatureReport { crates })
}

impl Provenance {
    /// The code bytes of `package`'s compile units, by crate name (hyphens as
    /// underscores) and version.
    #[must_use]
    pub fn bytes_of(&self, package: &Package) -> Option<u64> {
        self.bytes_of_version(&package.name.replace('-', "_"), &package.version.to_string())
    }
}
