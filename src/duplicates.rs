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
    let graph =
        Graph::new(metadata).context("`cargo metadata` returned no dependency resolution")?;

    let mut by_name: BTreeMap<&str, BTreeMap<&Version, &PackageId>> = BTreeMap::new();
    for (id, package) in graph.linked_packages() {
        by_name.entry(package.name.as_str()).or_default().insert(&package.version, id);
    }
    by_name.retain(|_, versions| versions.len() > 1);

    let duplicated: HashSet<&PackageId> =
        by_name.values().flat_map(|versions| versions.values().copied()).collect();
    let mut dependents = graph.dependents_of(&duplicated);

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

/// The dependency graph as the linker sees it.
struct Graph<'a> {
    nodes: HashMap<&'a PackageId, &'a Node>,
    packages: HashMap<&'a PackageId, &'a Package>,

    /// Packages reachable from the workspace over edges that carry code into a
    /// binary.
    linked: HashSet<&'a PackageId>,
}

impl<'a> Graph<'a> {
    /// `None` when `metadata` carries no dependency resolution.
    fn new(metadata: &'a Metadata) -> Option<Self> {
        let resolve = metadata.resolve.as_ref()?;
        let nodes: HashMap<&PackageId, &Node> =
            resolve.nodes.iter().map(|node| (&node.id, node)).collect();
        let packages: HashMap<&PackageId, &Package> =
            metadata.packages.iter().map(|package| (&package.id, package)).collect();

        let mut linked = HashSet::new();
        let mut queue: VecDeque<&PackageId> = metadata.workspace_members.iter().collect();
        while let Some(id) = queue.pop_front() {
            // A proc-macro is dropped after codegen, so nothing under it reaches
            // the binary. Its edges are `Normal`, so only the package reveals this.
            let is_proc_macro = packages
                .get(id)
                .is_some_and(|package| package.targets.iter().any(Target::is_proc_macro));

            if is_proc_macro || !linked.insert(id) {
                continue;
            }

            let Some(node) = nodes.get(id) else { continue };
            queue.extend(node.deps.iter().filter(|dep| is_linkable(dep)).map(|dep| &dep.pkg));
        }

        Some(Self { nodes, packages, linked })
    }

    /// Every linked package that `cargo metadata` described.
    fn linked_packages(&self) -> impl Iterator<Item = (&'a PackageId, &'a Package)> {
        self.linked.iter().filter_map(|&id| Some((id, *self.packages.get(id)?)))
    }

    /// The linked packages depending directly on each of `duplicated`.
    fn dependents_of(
        &self,
        duplicated: &HashSet<&'a PackageId>,
    ) -> HashMap<&'a PackageId, BTreeSet<Dependent>> {
        let mut dependents: HashMap<&PackageId, BTreeSet<Dependent>> = HashMap::new();

        for (id, package) in self.linked_packages() {
            let Some(node) = self.nodes.get(id) else { continue };

            for dep in
                node.deps.iter().filter(|dep| is_linkable(dep) && duplicated.contains(&dep.pkg))
            {
                dependents.entry(&dep.pkg).or_default().insert(Dependent {
                    name: package.name.as_str().to_owned(),
                    version: package.version.to_string(),
                });
            }
        }

        dependents
    }
}

/// An empty `dep_kinds` means cargo predates the field, so assume it links.
fn is_linkable(dep: &NodeDep) -> bool {
    dep.dep_kinds.is_empty()
        || dep.dep_kinds.iter().any(|kind| matches!(kind.kind, DependencyKind::Normal))
}
