//! Detect crates that resolve to more than one version.
//!
//! Every extra version is compiled and linked separately. Unlike
//! `cargo tree --duplicates`, the graph is walked as the linker sees it:
//! dev- and build-dependencies are skipped, and so is anything behind a
//! proc-macro, which runs in the compiler rather than in the binary.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

use anyhow::{Context, Result};
use cargo_metadata::{
    DependencyKind, Metadata, Node, NodeDep, Package, PackageId, Target, semver::Version,
};
use serde::Serialize;

/// A crate name that resolves to more than one version.
#[derive(Debug, Serialize)]
pub struct Duplicate {
    pub name: String,
    pub versions: Vec<DuplicateVersion>,
}

#[derive(Debug, Serialize)]
pub struct DuplicateVersion {
    pub version: String,
    pub dependents: Vec<Dependent>,
}

/// A package depending on one specific version of a duplicated crate — the
/// crate to bump or unify to collapse it.
#[derive(Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Dependent {
    pub name: String,
    pub version: String,
}

/// Find every crate that links at more than one version, sorted by name then
/// version.
///
/// # Errors
///
/// Errors when `metadata` carries no dependency resolution, as produced by
/// `cargo metadata --no-deps`.
pub fn find(metadata: &Metadata) -> Result<Vec<Duplicate>> {
    let resolve =
        metadata.resolve.as_ref().context("`cargo metadata` returned no dependency resolution")?;
    let nodes: HashMap<&PackageId, &Node> =
        resolve.nodes.iter().map(|node| (&node.id, node)).collect();
    let packages: HashMap<&PackageId, &Package> =
        metadata.packages.iter().map(|package| (&package.id, package)).collect();

    let linked = linked_packages(metadata, &nodes, &packages);

    let mut by_name: BTreeMap<&str, BTreeMap<&Version, &PackageId>> = BTreeMap::new();
    for id in &linked {
        if let Some(package) = packages.get(id) {
            by_name.entry(package.name.as_str()).or_default().insert(&package.version, id);
        }
    }
    by_name.retain(|_, versions| versions.len() > 1);

    let duplicated: HashSet<&PackageId> =
        by_name.values().flat_map(|versions| versions.values().copied()).collect();
    let mut dependents = dependents_of(&duplicated, &linked, &nodes, &packages);

    Ok(by_name
        .into_iter()
        .map(|(name, versions)| Duplicate {
            name: name.to_owned(),
            versions: versions
                .into_iter()
                .map(|(version, id)| DuplicateVersion {
                    version: version.to_string(),
                    dependents: dependents.remove(id).unwrap_or_default().into_iter().collect(),
                })
                .collect(),
        })
        .collect())
}

/// Crate names whose code can reach the binary, spelled as rustc spells them.
///
/// A proc-macro crate is compiled and monomorphized like any other, but it runs
/// inside the compiler, so none of its instantiations reach the output.
#[must_use]
pub fn linkable_crates(metadata: &Metadata) -> HashSet<String> {
    let Some(resolve) = metadata.resolve.as_ref() else { return HashSet::new() };

    let nodes: HashMap<&PackageId, &Node> =
        resolve.nodes.iter().map(|node| (&node.id, node)).collect();
    let packages: HashMap<&PackageId, &Package> =
        metadata.packages.iter().map(|package| (&package.id, package)).collect();

    linked_packages(metadata, &nodes, &packages)
        .iter()
        .filter_map(|id| packages.get(id))
        .map(|package| package.name.as_str().replace('-', "_"))
        .collect()
}

/// Packages reachable from the workspace over edges that carry code into a binary.
fn linked_packages<'a>(
    metadata: &'a Metadata,
    nodes: &HashMap<&'a PackageId, &'a Node>,
    packages: &HashMap<&'a PackageId, &'a Package>,
) -> HashSet<&'a PackageId> {
    let mut linked = HashSet::new();
    let mut queue: VecDeque<&PackageId> = metadata.workspace_members.iter().collect();

    while let Some(id) = queue.pop_front() {
        // A proc-macro is dropped after codegen, so nothing under it reaches the
        // binary. Its edges are `Normal`, so only the package reveals this.
        let is_proc_macro =
            packages.get(id).is_some_and(|pkg| pkg.targets.iter().any(Target::is_proc_macro));

        if is_proc_macro || !linked.insert(id) {
            continue;
        }

        let Some(node) = nodes.get(id) else { continue };
        queue.extend(node.deps.iter().filter(|dep| is_linkable(dep)).map(|dep| &dep.pkg));
    }

    linked
}

fn dependents_of<'a>(
    duplicated: &HashSet<&'a PackageId>,
    linked: &HashSet<&'a PackageId>,
    nodes: &HashMap<&'a PackageId, &'a Node>,
    packages: &HashMap<&'a PackageId, &'a Package>,
) -> HashMap<&'a PackageId, BTreeSet<Dependent>> {
    let mut dependents: HashMap<&PackageId, BTreeSet<Dependent>> = HashMap::new();

    for id in linked {
        let (Some(node), Some(package)) = (nodes.get(id), packages.get(id)) else { continue };

        for dep in node.deps.iter().filter(|dep| is_linkable(dep) && duplicated.contains(&dep.pkg))
        {
            dependents.entry(&dep.pkg).or_default().insert(Dependent {
                name: package.name.as_str().to_owned(),
                version: package.version.to_string(),
            });
        }
    }

    dependents
}

/// An empty `dep_kinds` means cargo predates the field, so assume it links.
fn is_linkable(dep: &NodeDep) -> bool {
    dep.dep_kinds.is_empty()
        || dep.dep_kinds.iter().any(|kind| matches!(kind.kind, DependencyKind::Normal))
}
