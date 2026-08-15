//! Detect crates that resolve to more than one version.
//!
//! Every extra version is compiled and linked separately, so a duplicate is
//! avoidable binary size. Unlike `cargo tree --duplicates`, the graph is walked
//! as the linker sees it, and two kinds of edge are dropped along the way:
//!
//! - dev- and build-dependencies, which are never linked into a shipped binary;
//! - anything behind a proc-macro, which runs inside the compiler rather than
//!   in the binary. Proc-macros are ordinary `Normal` dependencies, so the edge
//!   kind alone does not catch them — the *package* has to be inspected.
//!
//! Without the second rule almost every Rust project reports `syn` as a
//! duplicate, even though neither copy contributes a byte to the output.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

use anyhow::{Context, Result};
use cargo_metadata::{
    DependencyKind, Metadata, Node, NodeDep, Package, PackageId, Target, semver::Version,
};
use serde::Serialize;

/// A crate name that resolves to more than one version, with every version
/// found and who pulls it in.
#[derive(Debug, Serialize)]
pub struct Duplicate {
    /// Crate name shared by all the versions below.
    pub name: String,

    /// Every resolved version, ordered by semver.
    pub versions: Vec<DuplicateVersion>,
}

/// One resolved version of a duplicated crate.
#[derive(Debug, Serialize)]
pub struct DuplicateVersion {
    /// The resolved version.
    pub version: String,

    /// Packages that depend on this particular version — the crates to bump or
    /// unify to collapse the duplicate.
    pub dependents: Vec<Dependent>,
}

/// A package that depends on one specific version of a duplicated crate.
#[derive(Debug, Serialize)]
pub struct Dependent {
    /// Name of the depending package.
    pub name: String,

    /// Version of the depending package.
    pub version: String,
}

/// Find every crate that resolves to more than one version in `metadata`.
///
/// Results are sorted by crate name, then by version, so output is stable
/// across runs.
///
/// # Errors
///
/// Returns an error when `metadata` carries no dependency resolution, which
/// happens when `cargo metadata` was run with `--no-deps`.
pub fn find(metadata: &Metadata) -> Result<Vec<Duplicate>> {
    let resolve =
        metadata.resolve.as_ref().context("`cargo metadata` returned no dependency resolution")?;

    let nodes: HashMap<&PackageId, &Node> =
        resolve.nodes.iter().map(|node| (&node.id, node)).collect();
    let packages: HashMap<&PackageId, &Package> =
        metadata.packages.iter().map(|package| (&package.id, package)).collect();

    let reachable = reachable_from_workspace(metadata, &nodes, &packages);

    // name -> version -> package id, for everything that actually links.
    let mut by_name: BTreeMap<&str, BTreeMap<&Version, &PackageId>> = BTreeMap::new();
    for id in &reachable {
        if let Some(package) = packages.get(id) {
            by_name.entry(package.name.as_str()).or_default().insert(&package.version, id);
        }
    }
    by_name.retain(|_, versions| versions.len() > 1);

    let duplicated: HashSet<&PackageId> =
        by_name.values().flat_map(|versions| versions.values().copied()).collect();
    let dependents = collect_dependents(&reachable, &nodes, &packages, &duplicated);

    Ok(by_name
        .into_iter()
        .map(|(name, versions)| Duplicate {
            name: name.to_owned(),
            versions: versions
                .into_iter()
                .map(|(version, id)| DuplicateVersion {
                    version: version.to_string(),
                    dependents: dependents
                        .get(id)
                        .into_iter()
                        .flatten()
                        .map(|(name, version)| Dependent {
                            name: (*name).to_owned(),
                            version: version.clone(),
                        })
                        .collect(),
                })
                .collect(),
        })
        .collect())
}

/// Walk the resolve graph from the workspace members over linkable edges only,
/// returning every package that ends up in a workspace target's binary.
fn reachable_from_workspace<'a>(
    metadata: &'a Metadata,
    nodes: &HashMap<&'a PackageId, &'a Node>,
    packages: &HashMap<&'a PackageId, &'a Package>,
) -> HashSet<&'a PackageId> {
    let mut reachable = HashSet::new();
    let mut queue: VecDeque<&PackageId> = metadata.workspace_members.iter().collect();

    while let Some(id) = queue.pop_front() {
        // A proc-macro is loaded by the compiler and dropped after codegen, so
        // neither it nor anything it pulls in reaches the binary. Stop here
        // rather than descending.
        if packages.get(id).is_some_and(|package| is_proc_macro(package)) {
            continue;
        }

        if !reachable.insert(id) {
            continue;
        }

        let Some(node) = nodes.get(id) else { continue };
        queue.extend(node.deps.iter().filter(|dep| links_into_binary(dep)).map(|dep| &dep.pkg));
    }

    reachable
}

/// Whether this package builds a proc-macro rather than something linkable.
fn is_proc_macro(package: &Package) -> bool {
    package.targets.iter().any(Target::is_proc_macro)
}

/// Map each duplicated package to the packages depending on it.
fn collect_dependents<'a>(
    reachable: &HashSet<&'a PackageId>,
    nodes: &HashMap<&'a PackageId, &'a Node>,
    packages: &HashMap<&'a PackageId, &'a Package>,
    duplicated: &HashSet<&'a PackageId>,
) -> HashMap<&'a PackageId, BTreeSet<(&'a str, String)>> {
    let mut dependents: HashMap<&PackageId, BTreeSet<(&str, String)>> = HashMap::new();

    for id in reachable {
        let (Some(node), Some(package)) = (nodes.get(id), packages.get(id)) else { continue };

        for dep in &node.deps {
            if links_into_binary(dep) && duplicated.contains(&dep.pkg) {
                dependents
                    .entry(&dep.pkg)
                    .or_default()
                    .insert((package.name.as_str(), package.version.to_string()));
            }
        }
    }

    dependents
}

/// Whether an edge can carry code into the final binary.
///
/// `dep_kinds` is empty only for metadata produced by cargo versions predating
/// the field, where the kind is unknown; treat those edges as normal rather
/// than silently dropping them.
fn links_into_binary(dep: &NodeDep) -> bool {
    dep.dep_kinds.is_empty()
        || dep.dep_kinds.iter().any(|kind| matches!(kind.kind, DependencyKind::Normal))
}
