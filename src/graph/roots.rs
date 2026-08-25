//! Retained code and constants attributed to real graph roots.

use std::collections::VecDeque;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    dwarf::Site,
    name::{defining_crate, demangle},
};

use super::{Edges, Kind, WhyLive, why_live};

/// A larger cohort is rarely an actionable "remove these APIs together"
/// opportunity, and spelling hundreds of generated roots makes the report
/// less useful to both people and agents.
const MAX_COHORT: usize = 8;

#[derive(Debug)]
pub struct RootRetention {
    /// All code and constants this root can reach. These totals overlap across
    /// rows whenever roots share a dependency.
    pub reachable: u64,
    pub reachable_symbols: usize,

    /// Bytes and named code reached by this root and no other real root.
    pub exclusive: u64,
    pub exclusive_symbols: usize,

    /// Where the path's API entry is defined.
    pub defined_at: Option<String>,

    /// The shortest visible path from the generated/exported root to its API
    /// entry: the first source location in the same crate, or workspace code
    /// below `main`.
    pub path: WhyLive,
}

#[derive(Debug)]
pub struct RootCohort {
    /// Code and constants whose exact owner set is `roots`.
    pub joint: u64,
    pub joint_symbols: usize,

    /// Everything freed if every listed root disappears: `joint`, the roots'
    /// exclusive bytes, and bytes owned by any smaller subset of this cohort.
    pub recoverable: u64,
    pub recoverable_symbols: usize,

    /// Source-level API entries, rather than generated initializer records.
    pub roots: Vec<String>,
}

#[derive(Default)]
struct Ownership {
    bytes: u64,
    symbols: usize,
}

struct Root {
    entry: String,
    report: RootRetention,
}

struct Trace<'a, 's> {
    edges: &'a Edges,
    sizes: &'a FxHashMap<String, u64>,
    sized: &'a FxHashMap<u32, (u64, bool)>,
    site_by_id: &'a FxHashMap<u32, &'s Site>,
    successors: &'a FxHashMap<u32, Vec<(u32, Kind, u8)>>,
    owners: &'a mut FxHashMap<u32, Vec<u32>>,
}

/// Invert reachability from every real root onto the graph's size-bearing
/// nodes. A node with one owner contributes to that root's removal saving; an
/// exact owner set with several roots is code those APIs must remove together.
pub(super) fn analyze(
    edges: &Edges,
    sizes: &FxHashMap<String, u64>,
    roots: &[u32],
    sites: &FxHashMap<String, Site>,
    limit: usize,
) -> (Vec<RootRetention>, Vec<RootCohort>) {
    if roots.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let mut sized: FxHashMap<u32, (u64, bool)> = FxHashMap::default();
    for (label, &bytes) in sizes {
        if let Some(&id) = edges.ids.get(label) {
            sized.insert(id, (bytes, true));
        }
    }
    for (&id, &bytes) in &edges.data_bytes {
        sized.entry(id).or_insert((bytes, false));
    }

    let mut site_by_id: FxHashMap<u32, &Site> = FxHashMap::default();
    for label in sizes.keys() {
        if let Some(&id) = edges.ids.get(label)
            && let Some(site) = sites.get(&demangle(label))
        {
            site_by_id.insert(id, site);
        }
    }

    let mut successors: FxHashMap<u32, Vec<(u32, Kind, u8)>> = FxHashMap::default();
    for &(from, to, kind, source) in &edges.edges {
        successors.entry(from).or_default().push((to, kind, source));
    }

    let mut owners: FxHashMap<u32, Vec<u32>> = FxHashMap::default();
    let mut rows = Vec::with_capacity(roots.len());
    {
        let mut trace = Trace {
            edges,
            sizes,
            sized: &sized,
            site_by_id: &site_by_id,
            successors: &successors,
            owners: &mut owners,
        };
        for (owner, &root) in roots.iter().enumerate() {
            rows.push(trace.root(u32::try_from(owner).unwrap_or(u32::MAX), root));
        }
    }

    let ownership = ownership(&sized, owners, &mut rows);
    let cohorts = cohorts(&ownership, &rows, roots.len(), limit);

    rows.sort_by(|left, right| {
        right
            .report
            .exclusive
            .cmp(&left.report.exclusive)
            .then_with(|| right.report.reachable.cmp(&left.report.reachable))
            .then_with(|| left.entry.cmp(&right.entry))
    });
    rows.truncate(limit);
    (rows.into_iter().map(|row| row.report).collect(), cohorts)
}

impl Trace<'_, '_> {
    fn root(&mut self, owner: u32, root: u32) -> Root {
        let root_label = &self.edges.names[root as usize];
        let root_name = demangle(root_label);
        let root_crate = defining_crate(root_label, &root_name);
        let root_is_main = matches!(root_label.as_str(), "_main" | "main");
        let mut visited: FxHashSet<u32> = [root].into_iter().collect();
        let mut parents = FxHashMap::default();
        let mut queue = VecDeque::from([root]);
        let mut first_code = None;
        let mut source_entry = None;
        let mut reachable = 0;
        let mut reachable_symbols = 0;

        while let Some(node) = queue.pop_front() {
            if self.sizes.contains_key(&self.edges.names[node as usize]) {
                first_code.get_or_insert(node);
                if source_entry.is_none()
                    && let Some(site) = self.site_by_id.get(&node)
                {
                    let label = &self.edges.names[node as usize];
                    let name = demangle(label);
                    let same_crate = root_crate.as_ref().is_some_and(|krate| {
                        defining_crate(label, &name).as_ref() == Some(krate)
                            && site_belongs_to_crate(site, krate)
                    });
                    if (root_is_main && site.workspace) || same_crate {
                        source_entry = Some(node);
                    }
                }
            }
            if let Some(&(bytes, is_code)) = self.sized.get(&node) {
                reachable += bytes;
                reachable_symbols += usize::from(is_code);
                self.owners.entry(node).or_default().push(owner);
            }
            for &(child, kind, source) in self.successors.get(&node).into_iter().flatten() {
                if visited.insert(child) {
                    parents.insert(child, (node, kind, source));
                    queue.push_back(child);
                }
            }
        }

        let entry = source_entry.or(first_code).unwrap_or(root);
        Root {
            entry: demangle(&self.edges.names[entry as usize]),
            report: RootRetention {
                reachable,
                reachable_symbols,
                exclusive: 0,
                exclusive_symbols: 0,
                defined_at: self.site_by_id.get(&entry).map(|site| site.display()),
                path: why_live(self.edges, &parents, entry),
            },
        }
    }
}

fn ownership(
    sized: &FxHashMap<u32, (u64, bool)>,
    owners: FxHashMap<u32, Vec<u32>>,
    rows: &mut [Root],
) -> FxHashMap<Vec<u32>, Ownership> {
    let mut ownership: FxHashMap<Vec<u32>, Ownership> = FxHashMap::default();
    for (node, owners) in owners {
        let Some(&(bytes, is_code)) = sized.get(&node) else { continue };
        if let [owner] = owners.as_slice()
            && let Some(row) = rows.get_mut(*owner as usize)
        {
            row.report.exclusive += bytes;
            row.report.exclusive_symbols += usize::from(is_code);
        }
        let group = ownership.entry(owners).or_default();
        group.bytes += bytes;
        group.symbols += usize::from(is_code);
    }
    ownership
}

fn cohorts(
    ownership: &FxHashMap<Vec<u32>, Ownership>,
    rows: &[Root],
    root_count: usize,
    limit: usize,
) -> Vec<RootCohort> {
    let mut candidates: Vec<(&Vec<u32>, &Ownership)> = ownership
        .iter()
        .filter(|(owners, _)| owners.len() >= 2 && owners.len() <= MAX_COHORT)
        // Removing every root is just removing the whole program/library, not
        // a useful cohort opportunity.
        .filter(|(owners, _)| owners.len() < root_count)
        .collect();
    candidates.sort_by(|(left_roots, left), (right_roots, right)| {
        right
            .bytes
            .cmp(&left.bytes)
            .then_with(|| left_roots.len().cmp(&right_roots.len()))
            .then_with(|| left_roots.cmp(right_roots))
    });
    candidates.truncate(limit);

    candidates
        .into_iter()
        .map(|(candidate, joint)| {
            let mut recoverable = Ownership::default();
            for (owners, owned) in ownership {
                if is_subset(owners, candidate) {
                    recoverable.bytes += owned.bytes;
                    recoverable.symbols += owned.symbols;
                }
            }
            RootCohort {
                joint: joint.bytes,
                joint_symbols: joint.symbols,
                recoverable: recoverable.bytes,
                recoverable_symbols: recoverable.symbols,
                roots: candidate.iter().map(|&owner| rows[owner as usize].entry.clone()).collect(),
            }
        })
        .collect()
}

/// A proc macro can attribute a generated function to its own crate even when
/// the symbol belongs to the caller. Require the source path to name the same
/// crate before presenting it as the API implementation.
fn site_belongs_to_crate(site: &Site, krate: &str) -> bool {
    let normalized = site.file.replace('-', "_");
    normalized.split('/').any(|component| {
        component == krate
            || component.strip_prefix(krate).is_some_and(|suffix| {
                suffix
                    .strip_prefix('_')
                    .and_then(|suffix| suffix.chars().next())
                    .is_some_and(|first| first.is_ascii_digit())
            })
    })
}

/// Both lists are sorted root indices.
fn is_subset(subset: &[u32], superset: &[u32]) -> bool {
    let mut right = 0;
    for &left in subset {
        while right < superset.len() && superset[right] < left {
            right += 1;
        }
        if superset.get(right) != Some(&left) {
            return false;
        }
        right += 1;
    }
    true
}

#[cfg(test)]
mod tests {
    use crate::dwarf::Site;

    use super::site_belongs_to_crate;

    #[test]
    fn distinguishes_api_sources_from_proc_macro_sources() {
        let site = |file: &str| Site { file: file.to_owned(), line: 1, workspace: false };

        assert!(site_belongs_to_crate(
            &site("crates/rolldown_binding/src/lib.rs"),
            "rolldown_binding"
        ));
        assert!(site_belongs_to_crate(
            &site("oxc-resolver-napi-11.24.3/src/lib.rs"),
            "oxc_resolver_napi"
        ));
        assert!(!site_belongs_to_crate(&site("ctor-1.0.9/src/parse.rs"), "rolldown_binding"));
    }
}
