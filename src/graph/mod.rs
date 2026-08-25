//! The reference graph named by assembly and live input-object relocations:
//! who calls, addresses, and points at whom.
//!
//! A linked binary has no ordinary relocations left, so the graph combines the
//! assembly the compiler emitted with relocations recovered from the linker's
//! live input objects. Direct calls, taken addresses, and pointer slots then
//! form a whole-program graph even when one artifact omits a reference.
//!
//! Two things fall out that no symbol table can show. Vtables are anonymous
//! (`l_anon.…` private constants), which is why the dispatch view calls its
//! named total a floor — but a vtable's slots name its drop glue and methods,
//! so the trait object it serves can be recovered from what it points at. And
//! a function's callers are countable, so the functions kept alive by exactly
//! one call site — merge candidates — can be ranked.

mod roots;

use std::collections::VecDeque;

use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    dwarf::Site,
    name::{defining_crate, demangle, trait_of},
};

pub use roots::{RootCohort, RootRetention};

/// Functions below this size are not worth a "called from one place" row; at
/// shim size, the call is the body.
const MIN_SINGLE_CALLER: u64 = 256;

#[derive(Debug)]
pub struct GraphReport {
    /// Distinct references named by assembly or input-object relocations.
    pub edges: usize,

    /// Distinct edges by evidence source. `relocation_only_edges` is the
    /// portion input-object relocations added beyond what assembly named.
    pub assembly_edges: usize,
    pub relocation_edges: usize,
    pub relocation_only_edges: usize,

    /// Named code reached only after adding input-object relocations.
    pub relocation_covered_symbols: usize,
    pub relocation_covered_bytes: u64,

    /// Live linker inputs requested and successfully read for relocations.
    pub relocation_inputs: usize,
    pub relocation_objects: usize,

    /// Entry points, final-image exports, and initializer records used as the
    /// starting points for reachability and dominator analysis.
    pub roots: usize,

    /// All named linked code against which graph coverage is measured.
    pub code_symbols: usize,
    pub code_bytes: u64,

    /// Trait objects, recovered from the function pointers each anonymous
    /// vtable carries. `bytes` is the vtables themselves, not the methods they
    /// point at.
    pub vtables: Vec<VtableGroup>,

    /// Functions kept alive by exactly one call site, largest first.
    pub single_callers: Vec<SingleCaller>,

    /// Code and constants reached by each real entry, export, or initializer.
    /// `exclusive` is what removing that root alone can free.
    pub root_retention: Vec<RootRetention>,

    /// Exact small sets of roots that jointly keep code and constants alive.
    pub root_cohorts: Vec<RootCohort>,

    /// What removing a symbol frees — itself plus everything reachable only
    /// through it — by dominator analysis from every retention root.
    pub retained: Vec<Retained>,

    /// Linked code no reference the graph can see reaches from any root.
    pub unreachable: Unreachable,
}

#[derive(Debug)]
pub struct Retained {
    pub name: String,

    /// Bytes of the symbol itself.
    pub own: u64,

    /// Bytes freed if it went away: itself plus everything only reachable
    /// through it.
    pub retained: u64,

    /// How many other symbols would go with it.
    pub dominated: usize,

    /// Where it is defined, `file:line`, once the debug info has been read.
    pub defined_at: Option<String>,

    /// The shortest visible reference path from a retention root.
    pub why_live: WhyLive,
}

#[derive(Debug)]
pub struct Unreachable {
    pub symbols: usize,
    pub bytes: u64,

    /// Weakly connected groups of untraced code, largest first. These rank
    /// graph gaps for investigation; their bytes are not a savings estimate.
    pub islands: Vec<UntracedIsland>,
}

#[derive(Debug)]
pub struct UntracedIsland {
    pub symbols: usize,
    pub bytes: u64,
    pub references: usize,
    pub crates: Vec<IslandCrate>,

    /// The largest code symbol with no visible incoming reference inside the
    /// island, or simply the largest symbol when the island is cyclic.
    pub entry: String,
    pub entry_bytes: u64,

    /// Where the entry candidate is defined, once debug info has been read.
    pub defined_at: Option<String>,
}

#[derive(Debug)]
pub struct IslandCrate {
    pub name: String,
    pub bytes: u64,
}

#[derive(Debug)]
pub struct WhyLive {
    pub root: String,
    pub hops: Vec<WhyLiveHop>,
}

#[derive(Debug)]
pub struct WhyLiveHop {
    /// `call`, `address`, or `data`.
    pub reference: &'static str,

    /// `assembly`, `relocation`, or `assembly+relocation`.
    pub evidence: &'static str,
    pub target: String,
}

#[derive(Debug)]
pub struct VtableGroup {
    /// The trait, from the method slots' `<Type as Trait>` paths.
    pub name: String,
    pub bytes: u64,
    pub count: usize,
}

#[derive(Debug)]
pub struct SingleCaller {
    pub name: String,
    pub bytes: u64,

    /// The one function that calls it — where merging it would land.
    pub caller: String,

    /// Where it is defined, `file:line`, once the debug info has been read.
    pub defined_at: Option<String>,
}

/// How one symbol refers to another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Kind {
    /// A direct call or tail call.
    Call,

    /// An address taken in code (`adrp`, `lea`): the target may be called
    /// indirectly, so it is not single-caller material.
    Address,

    /// A pointer slot in a data symbol (`.quad`): vtables and function tables.
    Data,
}

impl Kind {
    const fn name(self) -> &'static str {
        match self {
            Self::Call => "call",
            Self::Address => "address",
            Self::Data => "data",
        }
    }
}

/// The references collected while the assembly streams by, as interned symbol
/// ids so the collection stays a pair of flat vectors.
#[derive(Default)]
pub(crate) struct Edges {
    names: Vec<String>,
    ids: FxHashMap<String, u32>,
    edges: Vec<(u32, u32, Kind, u8)>,

    relocation_inputs: usize,
    relocation_objects: usize,

    /// Bytes each data symbol emits, so an anonymous vtable — absent from the
    /// symbol table — still has a size.
    data_bytes: FxHashMap<u32, u64>,

    /// Data records the loader visits, such as constructor arrays. Entry
    /// points and exports are added after the final image has been read.
    roots: FxHashSet<u32>,
}

impl Edges {
    const ASSEMBLY: u8 = 1;
    const RELOCATION: u8 = 2;

    fn id(&mut self, name: &str) -> u32 {
        if let Some(&id) = self.ids.get(name) {
            return id;
        }
        let id = u32::try_from(self.names.len()).unwrap_or(u32::MAX);
        self.ids.insert(name.to_owned(), id);
        self.names.push(name.to_owned());
        id
    }

    pub(crate) fn call(&mut self, from: &str, to: &str) {
        let edge = (self.id(from), self.id(to), Kind::Call, Self::ASSEMBLY);
        self.edges.push(edge);
    }

    pub(crate) fn address(&mut self, from: &str, to: &str) {
        let edge = (self.id(from), self.id(to), Kind::Address, Self::ASSEMBLY);
        self.edges.push(edge);
    }

    pub(crate) fn slot(&mut self, from: &str, to: &str) {
        let edge = (self.id(from), self.id(to), Kind::Data, Self::ASSEMBLY);
        self.edges.push(edge);
    }

    pub(crate) fn relocation_call(&mut self, from: &str, to: &str) {
        let edge = (self.id(from), self.id(to), Kind::Call, Self::RELOCATION);
        self.edges.push(edge);
    }

    pub(crate) fn relocation_address(&mut self, from: &str, to: &str) {
        let edge = (self.id(from), self.id(to), Kind::Address, Self::RELOCATION);
        self.edges.push(edge);
    }

    pub(crate) fn relocation_slot(&mut self, from: &str, to: &str) {
        let edge = (self.id(from), self.id(to), Kind::Data, Self::RELOCATION);
        self.edges.push(edge);
    }

    pub(crate) fn relocation_stats(&mut self, inputs: usize, objects: usize) {
        self.relocation_inputs = inputs;
        self.relocation_objects = objects;
    }

    pub(crate) fn data_labels(&self) -> impl Iterator<Item = &str> {
        self.data_bytes.keys().map(|id| self.names[*id as usize].as_str())
    }

    pub(crate) fn data_bytes(&mut self, symbol: &str, bytes: u64) {
        let id = self.id(symbol);
        *self.data_bytes.entry(id).or_default() += bytes;
    }

    pub(crate) fn root(&mut self, symbol: &str) {
        let id = self.id(symbol);
        self.roots.insert(id);
    }
}

/// Read the graph in `edges`. `sizes` maps a code symbol's raw label to its
/// bytes in the binary, as `symbols::code_sizes` produces. `sites` lets
/// generated initializer paths end at the first same-crate API code (or the
/// first workspace code below `main`).
pub(crate) fn analyze(
    mut edges: Edges,
    sizes: &FxHashMap<String, u64>,
    exported_roots: &[String],
    sites: &FxHashMap<String, Site>,
    limit: usize,
) -> GraphReport {
    edges.edges.sort_unstable();
    let mut merged: Vec<(u32, u32, Kind, u8)> = Vec::with_capacity(edges.edges.len());
    for (from, to, kind, source) in edges.edges.drain(..) {
        if let Some(last) = merged.last_mut()
            && (last.0, last.1, last.2) == (from, to, kind)
        {
            last.3 |= source;
        } else {
            merged.push((from, to, kind, source));
        }
    }
    edges.edges = merged;

    // A library has no `main`: its public definitions and loader-run
    // initializers are independent roots. Joining all of them under a hidden
    // node lets the ordinary single-root dominator algorithm answer what any
    // one function exclusively retains.
    for root in exported_roots {
        edges.root(root);
    }
    for entry in ["_main", "main"] {
        if sizes.contains_key(entry) {
            edges.root(entry);
        }
    }
    let reference_edges = edges.edges.len();
    let assembly_edges = edges.edges.iter().filter(|edge| edge.3 & Edges::ASSEMBLY != 0).count();
    let relocation_edges =
        edges.edges.iter().filter(|edge| edge.3 & Edges::RELOCATION != 0).count();
    let relocation_only_edges =
        edges.edges.iter().filter(|edge| edge.3 == Edges::RELOCATION).count();
    let mut roots: Vec<u32> = edges.roots.iter().copied().collect();
    roots.sort_by(|&left, &right| edges.names[left as usize].cmp(&edges.names[right as usize]));
    let synthetic = (!roots.is_empty()).then(|| {
        let synthetic = edges.id("\0cargo-bsize retention roots");
        edges
            .edges
            .extend(roots.iter().map(|&root| (synthetic, root, Kind::Address, Edges::ASSEMBLY)));
        synthetic
    });

    let (relocation_covered_symbols, relocation_covered_bytes) = synthetic.map_or((0, 0), |root| {
        let assembly: FxHashSet<u32> =
            reachable_postorder_from(&edges, root, Edges::ASSEMBLY).into_iter().collect();
        let combined: FxHashSet<u32> = reachable_postorder(&edges, root).into_iter().collect();
        sizes.iter().fold((0, 0), |(symbols, bytes), (label, size)| {
            let gained = edges
                .ids
                .get(label)
                .is_some_and(|id| combined.contains(id) && !assembly.contains(id));
            if gained { (symbols + 1, bytes + size) } else { (symbols, bytes) }
        })
    });

    let (root_retention, root_cohorts) = roots::analyze(&edges, sizes, &roots, sites, limit);
    let (retained, unreachable) = synthetic.map_or_else(
        || {
            let reached = FxHashSet::default();
            (Vec::new(), untraced(&edges, sizes, &reached, limit))
        },
        |root| retained(&edges, sizes, root, &roots, limit),
    );
    GraphReport {
        edges: reference_edges,
        assembly_edges,
        relocation_edges,
        relocation_only_edges,
        relocation_covered_symbols,
        relocation_covered_bytes,
        relocation_inputs: edges.relocation_inputs,
        relocation_objects: edges.relocation_objects,
        roots: roots.len(),
        code_symbols: sizes.len(),
        code_bytes: sizes.values().sum(),
        vtables: vtables(&edges, limit),
        single_callers: single_callers(&edges, sizes, limit),
        root_retention,
        root_cohorts,
        retained,
        unreachable,
    }
}

/// Group the data symbols whose slots look like a vtable — drop glue plus at
/// least one method pointer — by the trait the method slots implement.
fn vtables(edges: &Edges, limit: usize) -> Vec<VtableGroup> {
    let mut slots: FxHashMap<u32, Vec<u32>> = FxHashMap::default();
    for &(from, to, kind, _) in &edges.edges {
        if kind == Kind::Data {
            slots.entry(from).or_default().push(to);
        }
    }

    let mut groups: FxHashMap<String, (u64, usize)> = FxHashMap::default();
    for (symbol, members) in slots {
        let names: Vec<String> =
            members.iter().map(|&member| demangle(&edges.names[member as usize])).collect();
        let (drops, methods): (Vec<&String>, Vec<&String>) = names
            .iter()
            .partition(|name| name.contains("drop_in_place") || name.contains("drop_glue"));
        if drops.is_empty() || methods.is_empty() {
            continue;
        }

        // The trait comes from any method slot that names one; a `dyn Fn`
        // vtable's slot is `<F as Fn<…>>::call`, a service's is
        // `<S as Service<…>>::call`.
        let name = methods
            .iter()
            .find_map(|method| trait_of(method))
            .unwrap_or_else(|| "(trait not named by any slot)".to_owned());

        let bytes = edges.data_bytes.get(&symbol).copied().unwrap_or_default();
        let entry = groups.entry(name).or_default();
        entry.0 += bytes;
        entry.1 += 1;
    }

    let mut vtables: Vec<VtableGroup> = groups
        .into_iter()
        .map(|(name, (bytes, count))| VtableGroup { name, bytes, count })
        .collect();
    vtables.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
    vtables.truncate(limit);
    vtables
}

/// Functions with exactly one distinct caller and no taken address — nothing
/// else can reach them, so they live entirely for that one call site.
fn single_callers(
    edges: &Edges,
    sizes: &FxHashMap<String, u64>,
    limit: usize,
) -> Vec<SingleCaller> {
    let mut callers: FxHashMap<u32, FxHashSet<u32>> = FxHashMap::default();
    let mut addressed: FxHashSet<u32> = FxHashSet::default();
    for &(from, to, kind, _) in &edges.edges {
        match kind {
            Kind::Call if from != to => {
                callers.entry(to).or_default().insert(from);
            }
            Kind::Call => {}
            Kind::Address | Kind::Data => {
                addressed.insert(to);
            }
        }
    }

    let mut single: Vec<SingleCaller> = callers
        .into_iter()
        .filter(|(to, from)| from.len() == 1 && !addressed.contains(to))
        .filter_map(|(to, from)| {
            let label = &edges.names[to as usize];
            let bytes = *sizes.get(label)?;
            (bytes >= MIN_SINGLE_CALLER).then(|| SingleCaller {
                name: demangle(label),
                bytes,
                caller: demangle(&edges.names[*from.iter().next().expect("one caller") as usize]),
                defined_at: None,
            })
        })
        .collect();
    single.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
    single.truncate(limit);
    single
}

/// What each symbol retains — itself plus everything only reachable through it
/// — by building the dominator tree from the synthetic root, and the code
/// nothing reaches at all.
fn retained(
    edges: &Edges,
    sizes: &FxHashMap<String, u64>,
    root: u32,
    roots: &[u32],
    limit: usize,
) -> (Vec<Retained>, Unreachable) {
    let postorder = reachable_postorder(edges, root);
    let order: FxHashMap<u32, usize> =
        postorder.iter().enumerate().map(|(index, &node)| (node, index)).collect();
    let reached: FxHashSet<u32> = postorder.iter().copied().collect();
    let root_index = order[&root];
    let idom = immediate_dominators(edges, &postorder, &order, root_index);
    let parents = shortest_paths(edges, roots);

    // A symbol's own bytes: code from the symbol table, data from its emitted
    // slots.
    let own = |node: u32| -> u64 {
        let label = &edges.names[node as usize];
        sizes.get(label).copied().or_else(|| edges.data_bytes.get(&node).copied()).unwrap_or(0)
    };

    // Accumulate up the dominator tree. An immediate dominator always sits
    // later in postorder than what it dominates, so walking postorder forward
    // sums children before their parents see them.
    // Anonymous constants are nodes too, so their bytes flow into `retained`,
    // but `dominated` counts only named code symbols and only they get rows.
    let is_code = |node: u32| sizes.contains_key(&edges.names[node as usize]);
    let mut totals: Vec<u64> = postorder.iter().map(|&node| own(node)).collect();
    let mut counts: Vec<usize> = vec![0; postorder.len()];
    for index in 0..postorder.len() {
        if index == root_index {
            continue;
        }
        let Some(parent) = idom[index] else { continue };
        totals[parent] += totals[index];
        counts[parent] += counts[index] + usize::from(is_code(postorder[index]));
    }

    let mut rows: Vec<Retained> = postorder
        .iter()
        .enumerate()
        .filter(|&(index, &node)| {
            let label = &edges.names[node as usize];
            index != root_index
                && !matches!(label.as_str(), "_main" | "main")
                && is_code(node)
                && totals[index] > own(node)
        })
        .map(|(index, &node)| Retained {
            name: demangle(&edges.names[node as usize]),
            own: own(node),
            retained: totals[index],
            dominated: counts[index],
            defined_at: None,
            why_live: why_live(edges, &parents, node),
        })
        .collect();
    rows.sort_by(|a, b| b.retained.cmp(&a.retained).then_with(|| a.name.cmp(&b.name)));
    rows.truncate(limit);

    (rows, untraced(edges, sizes, &reached, limit))
}

/// A breadth-first forest from all real roots. Besides making the path the
/// shortest visible explanation, starting with sorted roots and sorted edges
/// keeps ties stable across runs.
fn shortest_paths(edges: &Edges, roots: &[u32]) -> FxHashMap<u32, (u32, Kind, u8)> {
    let mut successors: FxHashMap<u32, Vec<(u32, Kind, u8)>> = FxHashMap::default();
    for &(from, to, kind, source) in &edges.edges {
        successors.entry(from).or_default().push((to, kind, source));
    }

    let mut parents = FxHashMap::default();
    let mut visited: FxHashSet<u32> = roots.iter().copied().collect();
    let mut queue: VecDeque<u32> = roots.iter().copied().collect();
    while let Some(parent) = queue.pop_front() {
        for &(child, kind, source) in successors.get(&parent).into_iter().flatten() {
            if visited.insert(child) {
                parents.insert(child, (parent, kind, source));
                queue.push_back(child);
            }
        }
    }
    parents
}

fn why_live(edges: &Edges, parents: &FxHashMap<u32, (u32, Kind, u8)>, node: u32) -> WhyLive {
    let mut cursor = node;
    let mut reversed = Vec::new();
    while let Some(&(parent, kind, source)) = parents.get(&cursor) {
        reversed.push(WhyLiveHop {
            reference: kind.name(),
            evidence: match source {
                Edges::ASSEMBLY => "assembly",
                Edges::RELOCATION => "relocation",
                _ => "assembly+relocation",
            },
            target: demangle(&edges.names[cursor as usize]),
        });
        cursor = parent;
    }
    reversed.reverse();
    WhyLive { root: demangle(&edges.names[cursor as usize]), hops: reversed }
}

/// Group code with no root path into weakly connected islands. Direction is
/// retained separately to identify a likely missing entry: a code symbol with
/// no incoming reference from the rest of its island.
fn untraced(
    edges: &Edges,
    sizes: &FxHashMap<String, u64>,
    reached: &FxHashSet<u32>,
    limit: usize,
) -> Unreachable {
    let is_untraced = |label: &String| edges.ids.get(label).is_none_or(|id| !reached.contains(id));
    let symbols = sizes.keys().filter(|label| is_untraced(label)).count();
    let bytes = sizes.iter().filter(|(label, _)| is_untraced(label)).map(|(_, bytes)| bytes).sum();

    let mut code: FxHashMap<u32, (&str, u64)> = FxHashMap::default();
    for (label, &bytes) in sizes {
        if let Some(&id) = edges.ids.get(label)
            && !reached.contains(&id)
        {
            code.insert(id, (label, bytes));
        }
    }
    let mut linked: FxHashSet<u32> = code.keys().copied().collect();
    linked.extend(edges.data_bytes.keys().filter(|id| !reached.contains(id)).copied());

    let mut neighbors: FxHashMap<u32, Vec<u32>> = FxHashMap::default();
    let mut incoming: FxHashMap<u32, usize> = FxHashMap::default();
    let mut outgoing: FxHashMap<u32, usize> = FxHashMap::default();
    for &(from, to, _, _) in &edges.edges {
        // Do not join otherwise independent islands through a shared external
        // symbol. Only linked code and emitted data carry size in this graph.
        if !linked.contains(&from) || !linked.contains(&to) {
            continue;
        }
        neighbors.entry(from).or_default().push(to);
        if from != to {
            neighbors.entry(to).or_default().push(from);
            *incoming.entry(to).or_default() += 1;
        }
        *outgoing.entry(from).or_default() += 1;
    }

    let mut islands = Vec::new();
    let mut visited = FxHashSet::default();
    let mut starts: Vec<u32> = code.keys().copied().collect();
    starts.sort_unstable();
    for start in starts {
        if !visited.insert(start) {
            continue;
        }
        let mut stack = vec![start];
        let mut nodes = Vec::new();
        while let Some(node) = stack.pop() {
            nodes.push(node);
            for &neighbor in neighbors.get(&node).into_iter().flatten() {
                if visited.insert(neighbor) {
                    stack.push(neighbor);
                }
            }
        }

        let members: Vec<(u32, &str, u64)> = nodes
            .iter()
            .filter_map(|id| code.get(id).map(|&(label, bytes)| (*id, label, bytes)))
            .collect();
        if !members.is_empty() {
            let references =
                nodes.iter().map(|id| outgoing.get(id).copied().unwrap_or_default()).sum();
            islands.push(island(&members, &incoming, references));
        }
    }

    // A linked symbol the assembly never names has no graph id, so it is a
    // useful singleton island rather than disappearing into only the total.
    for (label, &bytes) in sizes {
        if !edges.ids.contains_key(label) {
            islands.push(island(&[(u32::MAX, label, bytes)], &incoming, 0));
        }
    }

    islands.sort_by(|left, right| {
        right.bytes.cmp(&left.bytes).then_with(|| left.entry.cmp(&right.entry))
    });
    islands.truncate(limit);
    Unreachable { symbols, bytes, islands }
}

fn island(
    members: &[(u32, &str, u64)],
    incoming: &FxHashMap<u32, usize>,
    references: usize,
) -> UntracedIsland {
    let largest = members
        .iter()
        .max_by(|left, right| left.2.cmp(&right.2).then_with(|| right.1.cmp(left.1)))
        .copied()
        .expect("nonempty island");
    let entry = members
        .iter()
        .filter(|(id, _, _)| incoming.get(id).copied().unwrap_or_default() == 0)
        .max_by(|left, right| left.2.cmp(&right.2).then_with(|| right.1.cmp(left.1)))
        .copied()
        .unwrap_or(largest);

    let mut crates: FxHashMap<String, u64> = FxHashMap::default();
    for &(_, label, bytes) in members {
        let name = demangle(label);
        let krate = defining_crate(label, &name).unwrap_or_else(|| "(unknown)".to_owned());
        *crates.entry(krate).or_default() += bytes;
    }
    let mut crates: Vec<IslandCrate> =
        crates.into_iter().map(|(name, bytes)| IslandCrate { name, bytes }).collect();
    crates.sort_by(|left, right| {
        right.bytes.cmp(&left.bytes).then_with(|| left.name.cmp(&right.name))
    });

    UntracedIsland {
        symbols: members.len(),
        bytes: members.iter().map(|member| member.2).sum(),
        references,
        crates,
        entry: demangle(entry.1),
        entry_bytes: entry.2,
        defined_at: None,
    }
}

/// Reachable symbols in depth-first postorder, built iteratively to avoid
/// recursing on a potentially deep reference graph.
fn reachable_postorder(edges: &Edges, root: u32) -> Vec<u32> {
    reachable_postorder_from(edges, root, Edges::ASSEMBLY | Edges::RELOCATION)
}

fn reachable_postorder_from(edges: &Edges, root: u32, sources: u8) -> Vec<u32> {
    let mut successors: FxHashMap<u32, Vec<u32>> = FxHashMap::default();
    for &(from, to, _, source) in &edges.edges {
        if source & sources != 0 {
            successors.entry(from).or_default().push(to);
        }
    }

    let mut postorder = Vec::new();
    let mut visited: FxHashSet<u32> = [root].into_iter().collect();
    let mut stack = vec![(root, 0)];
    while let Some(top) = stack.last_mut() {
        let (node, next) = (top.0, top.1);
        top.1 += 1;
        if let Some(&child) = successors.get(&node).and_then(|children| children.get(next)) {
            if visited.insert(child) {
                stack.push((child, 0));
            }
        } else {
            postorder.push(node);
            stack.pop();
        }
    }

    postorder
}

/// Cooper–Harvey–Kennedy immediate dominators. Indices refer to positions in
/// `postorder`; walking them in reverse gives reverse postorder.
fn immediate_dominators(
    edges: &Edges,
    postorder: &[u32],
    order: &FxHashMap<u32, usize>,
    root: usize,
) -> Vec<Option<usize>> {
    let mut predecessors: FxHashMap<u32, Vec<u32>> = FxHashMap::default();
    for &(from, to, _, _) in &edges.edges {
        if order.contains_key(&from) && order.contains_key(&to) {
            predecessors.entry(to).or_default().push(from);
        }
    }

    let mut dominators = vec![None; postorder.len()];
    dominators[root] = Some(root);
    let mut changed = true;
    while changed {
        changed = false;
        for index in (0..postorder.len()).rev().filter(|&index| index != root) {
            let mut new = None;
            for &predecessor in predecessors.get(&postorder[index]).into_iter().flatten() {
                let predecessor = order[&predecessor];
                if dominators[predecessor].is_none() {
                    continue;
                }
                new = Some(new.map_or(predecessor, |current| {
                    intersect_dominators(&dominators, current, predecessor)
                }));
            }
            if new.is_some() && dominators[index] != new {
                dominators[index] = new;
                changed = true;
            }
        }
    }

    dominators
}

fn intersect_dominators(dominators: &[Option<usize>], mut left: usize, mut right: usize) -> usize {
    while left != right {
        while left < right {
            left = dominators[left].expect("processed dominator");
        }
        while right < left {
            right = dominators[right].expect("processed dominator");
        }
    }
    left
}

#[cfg(test)]
mod tests {
    use rustc_hash::FxHashMap;

    use super::{Edges, analyze};
    use crate::dwarf::Site;

    #[test]
    fn recovers_vtables_and_single_callers() {
        let mut edges = Edges::default();

        // `main` calls `big` from one place; `helper` also calls `shared`.
        edges.call("_main", "_RNvC1a3big");
        edges.call("_main", "_RNvC1a6shared");
        edges.call("_RNvC1a6helper", "_RNvC1a6shared");
        // A duplicate of the same call must not create a second caller.
        edges.call("_main", "_RNvC1a3big");
        // `addressed` is called once but also address-taken: not single-caller.
        edges.call("_main", "_RNvC1a9addressed");
        edges.address("_main", "_RNvC1a9addressed");

        // A vtable: drop glue, then a method naming the trait. Unmangled names
        // pass through `demangle` unchanged, so the shapes are what matter.
        edges.slot("l_anon.1.0", "core::ptr::drop_glue::<a::Foo>");
        edges.slot("l_anon.1.0", "<a::Foo as tower_service::Service<Request>>::call");
        edges.data_bytes("l_anon.1.0", 40);
        // A data symbol with pointers but no drop glue is not a vtable.
        edges.slot("l_anon.1.1", "_RNvC1a6shared");
        edges.data_bytes("l_anon.1.1", 16);

        let sizes: FxHashMap<String, u64> = [
            ("_RNvC1a3big".to_owned(), 5000),
            ("_RNvC1a6shared".to_owned(), 4000),
            ("_RNvC1a9addressed".to_owned(), 3000),
            ("_main".to_owned(), 100),
        ]
        .into_iter()
        .collect();

        let report = analyze(edges, &sizes, &[], &FxHashMap::default(), 10);

        // Only `big` qualifies: `shared` has two callers, `addressed` is
        // address-taken, and the slot reference keeps `shared` out too.
        assert_eq!(report.single_callers.len(), 1);
        assert_eq!(report.single_callers[0].name, "a::big");
        assert_eq!(report.single_callers[0].bytes, 5000);
        assert_eq!(report.single_callers[0].caller, "_main");

        assert_eq!(report.vtables.len(), 1);
        assert_eq!(report.vtables[0].bytes, 40);
        assert_eq!(report.vtables[0].count, 1);
        assert!(report.vtables[0].name.contains("Service"), "{}", report.vtables[0].name);
    }

    #[test]
    fn dominators_measure_what_each_symbol_retains() {
        let mut edges = Edges::default();
        // A diamond into a cycle, and a chain only `a` reaches.
        edges.call("_main", "_RNvC1a1a");
        edges.call("_main", "_RNvC1a1b");
        edges.call("_RNvC1a1a", "_RNvC1a1c");
        edges.call("_RNvC1a1b", "_RNvC1a1c");
        edges.call("_RNvC1a1c", "_RNvC1a1d");
        edges.call("_RNvC1a1d", "_RNvC1a1c");
        edges.call("_RNvC1a1a", "_RNvC1a1e");
        edges.call("_RNvC1a1e", "_RNvC1a1f");

        let sizes: FxHashMap<String, u64> = [
            ("_main", 100),
            ("_RNvC1a1a", 1000),
            ("_RNvC1a1b", 1000),
            ("_RNvC1a1c", 1000),
            ("_RNvC1a1d", 1000),
            ("_RNvC1a1e", 1000),
            ("_RNvC1a1f", 1000),
            ("_RNvC1a1g", 500),
            ("_RNvC1a1z", 700),
        ]
        .map(|(name, size)| (name.to_owned(), size))
        .into_iter()
        .collect();

        // A method reached only through a vtable `a` loads: the constant is a
        // node, so the method is reachable and its bytes flow into `a`.
        edges.address("_RNvC1a1a", "l_anon.9.0");
        edges.slot("l_anon.9.0", "core::ptr::drop_glue::<a::G>");
        edges.slot("l_anon.9.0", "_RNvC1a1g");
        edges.data_bytes("l_anon.9.0", 40);

        let report = analyze(edges, &sizes, &[], &FxHashMap::default(), 10);

        // `c` is reached two ways, so only `main` dominates it; the cycle back
        // from `d` changes nothing; `e` and `f` exist only through `a`.
        let rows: Vec<(&str, u64, u64, usize)> = report
            .retained
            .iter()
            .map(|row| (row.name.as_str(), row.own, row.retained, row.dominated))
            .collect();
        // `a` retains `e`, `f`, and — through the vtable's 40 B and its
        // method `g` — 540 B more; the constant itself is not a row and not
        // in the counts.
        assert_eq!(
            rows,
            [("a::a", 1000, 3540, 3), ("a::c", 1000, 2000, 1), ("a::e", 1000, 2000, 1)]
        );
        let why_live = &report.retained[0].why_live;
        assert_eq!(why_live.root, "_main");
        assert_eq!(why_live.hops.len(), 1);
        assert_eq!(why_live.hops[0].reference, "call");
        assert_eq!(why_live.hops[0].target, "a::a");

        // `z` is linked but nothing references it.
        assert_eq!(report.unreachable.symbols, 1);
        assert_eq!(report.unreachable.bytes, 700);
        assert_eq!(report.unreachable.islands.len(), 1);
        assert_eq!(report.unreachable.islands[0].entry, "a::z");
    }

    #[test]
    fn dominators_start_from_every_export() {
        let mut edges = Edges::default();
        edges.call("_export_a", "a_only");
        edges.call("_export_a", "shared");
        edges.call("_export_b", "b_only");
        edges.call("_export_b", "shared");
        edges.call("a_only", "a_child");

        let sizes: FxHashMap<String, u64> = [
            ("_export_a", 100),
            ("_export_b", 100),
            ("a_only", 1000),
            ("a_child", 500),
            ("b_only", 700),
            ("shared", 900),
            ("unseen", 300),
        ]
        .map(|(name, size)| (name.to_owned(), size))
        .into_iter()
        .collect();

        let roots = ["_export_a".to_owned(), "_export_b".to_owned()];
        let report = analyze(edges, &sizes, &roots, &FxHashMap::default(), 10);

        assert_eq!(report.roots, 2);
        assert_eq!(report.code_symbols, 7);
        assert_eq!(report.code_bytes, 3600);
        assert_eq!(report.retained[0].name, "_export_a");
        assert_eq!(report.retained[0].retained, 1600);
        assert_eq!(report.retained[0].dominated, 2);
        assert_eq!(report.retained[0].why_live.root, "_export_a");
        assert!(report.retained[0].why_live.hops.is_empty());
        assert_eq!(report.root_retention.len(), 2);
        let export_a = &report.root_retention[0];
        assert_eq!(export_a.path.root, "_export_a");
        assert_eq!(export_a.reachable, 2500);
        assert_eq!(export_a.exclusive, 1600);
        assert_eq!(export_a.reachable_symbols, 4);
        assert_eq!(export_a.exclusive_symbols, 3);
        assert!(export_a.path.hops.is_empty());
        let export_b = &report.root_retention[1];
        assert_eq!(export_b.path.root, "_export_b");
        assert_eq!(export_b.reachable, 1700);
        assert_eq!(export_b.exclusive, 800);
        // The only possible cohort is every root, which is deliberately not
        // reported as a useful removal opportunity.
        assert!(report.root_cohorts.is_empty());
        assert_eq!(report.unreachable.symbols, 1);
        assert_eq!(report.unreachable.bytes, 300);
    }

    #[test]
    fn attributes_shared_bytes_to_exact_root_cohorts() {
        let mut edges = Edges::default();
        edges.call("root_a", "a_only");
        edges.call("root_b", "b_only");
        edges.call("root_c", "c_only");
        edges.call("root_a", "shared_ab");
        edges.call("root_b", "shared_ab");
        edges.call("root_a", "shared_all");
        edges.call("root_b", "shared_all");
        edges.call("root_c", "shared_all");

        let sizes: FxHashMap<String, u64> = [
            ("root_a", 10),
            ("root_b", 20),
            ("root_c", 30),
            ("a_only", 100),
            ("b_only", 200),
            ("c_only", 300),
            ("shared_ab", 400),
            ("shared_all", 500),
        ]
        .map(|(name, size)| (name.to_owned(), size))
        .into_iter()
        .collect();
        let roots = ["root_a".to_owned(), "root_b".to_owned(), "root_c".to_owned()];

        let report = analyze(edges, &sizes, &roots, &FxHashMap::default(), 10);

        let cohort = report.root_cohorts.first().expect("a/b cohort");
        assert_eq!(cohort.roots, ["root_a", "root_b"]);
        assert_eq!(cohort.joint, 400);
        assert_eq!(cohort.joint_symbols, 1);
        // Removing A and B frees their own roots and branches plus `shared_ab`,
        // but not `shared_all`, which C still reaches.
        assert_eq!(cohort.recoverable, 730);
        assert_eq!(cohort.recoverable_symbols, 5);
    }

    #[test]
    fn names_initializer_roots_by_their_first_code_entry() {
        let mut edges = Edges::default();
        edges.slot("initializer_record", "register_api");
        edges.call("register_api", "implementation");
        edges.data_bytes("initializer_record", 8);
        let sizes: FxHashMap<String, u64> = [("register_api", 100), ("implementation", 500)]
            .map(|(name, size)| (name.to_owned(), size))
            .into_iter()
            .collect();
        let sites: FxHashMap<String, Site> = [
            (
                "register_api".to_owned(),
                Site { file: "ctor/src/lib.rs".to_owned(), line: 1, workspace: false },
            ),
            (
                "implementation".to_owned(),
                Site { file: "src/lib.rs".to_owned(), line: 42, workspace: true },
            ),
        ]
        .into_iter()
        .collect();

        // Unmangled fixture labels do not name a defining crate, so model a
        // binary entry: it is allowed to resolve to the first workspace site.
        edges.root("_main");
        edges.slot("_main", "initializer_record");
        let report = analyze(edges, &sizes, &[], &sites, 10);

        let root = &report.root_retention[0];
        assert_eq!(root.reachable, 608);
        assert_eq!(root.exclusive, 608);
        assert_eq!(root.defined_at.as_deref(), Some("src/lib.rs:42"));
        assert_eq!(root.path.root, "_main");
        assert_eq!(root.path.hops.len(), 3);
        assert_eq!(root.path.hops[0].reference, "data");
        assert_eq!(root.path.hops[2].target, "implementation");
    }

    #[test]
    fn ranks_untraced_islands_and_finds_their_entries() {
        let mut edges = Edges::default();
        edges.call("_main", "reachable");
        edges.call("_RNvC1a6orphan", "_RNvC1a5child");

        let sizes: FxHashMap<String, u64> = [
            ("_main", 100),
            ("reachable", 100),
            ("_RNvC1a6orphan", 500),
            ("_RNvC1a5child", 700),
            ("isolated", 1000),
        ]
        .map(|(name, size)| (name.to_owned(), size))
        .into_iter()
        .collect();

        let report = analyze(edges, &sizes, &[], &FxHashMap::default(), 10);

        assert_eq!(report.unreachable.symbols, 3);
        assert_eq!(report.unreachable.bytes, 2200);
        assert_eq!(report.unreachable.islands.len(), 2);
        let connected = &report.unreachable.islands[0];
        assert_eq!(connected.bytes, 1200);
        assert_eq!(connected.symbols, 2);
        assert_eq!(connected.references, 1);
        assert_eq!(connected.entry, "a::orphan");
        assert_eq!(connected.entry_bytes, 500);
        assert_eq!(connected.crates[0].name, "a");
        assert_eq!(connected.crates[0].bytes, 1200);
        assert_eq!(report.unreachable.islands[1].entry, "isolated");
    }

    #[test]
    fn merges_assembly_and_relocation_evidence() {
        let mut edges = Edges::default();
        edges.call("_main", "a");
        edges.relocation_call("_main", "a");
        edges.relocation_call("a", "b");
        edges.relocation_call("b", "c");
        edges.relocation_stats(3, 2);
        let sizes: FxHashMap<String, u64> = [("_main", 100), ("a", 500), ("b", 300), ("c", 200)]
            .map(|(name, size)| (name.to_owned(), size))
            .into_iter()
            .collect();

        let report = analyze(edges, &sizes, &[], &FxHashMap::default(), 10);

        assert_eq!(report.edges, 3);
        assert_eq!(report.assembly_edges, 1);
        assert_eq!(report.relocation_edges, 3);
        assert_eq!(report.relocation_only_edges, 2);
        assert_eq!(report.relocation_covered_symbols, 2);
        assert_eq!(report.relocation_covered_bytes, 500);
        assert_eq!(report.relocation_inputs, 3);
        assert_eq!(report.relocation_objects, 2);
        let path =
            &report.retained.iter().find(|entry| entry.name == "b").expect("retained b").why_live;
        assert_eq!(path.hops[0].evidence, "assembly+relocation");
        assert_eq!(path.hops[1].evidence, "relocation");
    }
}
