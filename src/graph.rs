//! The reference graph the assembly names: who calls, addresses, and points at
//! whom.
//!
//! A linked binary has no relocations left, so the only whole-program record of
//! "X refers to Y" is the assembly the compiler emitted — every direct call
//! names its target, every `adrp`/`lea` names the symbol whose address it
//! takes, and every `.quad` in a constant names what it points at. The parser
//! collects those while it streams; this module reads the graph they form.
//!
//! Two things fall out that no symbol table can show. Vtables are anonymous
//! (`l_anon.…` private constants), which is why the dispatch view calls its
//! named total a floor — but a vtable's slots name its drop glue and methods,
//! so the trait object it serves can be recovered from what it points at. And
//! a function's callers are countable, so the functions kept alive by exactly
//! one call site — merge candidates — can be ranked.

use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::name::{demangle, trait_of};

/// Functions below this size are not worth a "called from one place" row; at
/// shim size, the call is the body.
const MIN_SINGLE_CALLER: u64 = 256;

#[derive(Debug, Serialize)]
pub struct GraphReport {
    /// Distinct references the assembly names, of every kind.
    pub edges: usize,

    /// Trait objects, recovered from the function pointers each anonymous
    /// vtable carries. `bytes` is the vtables themselves, not the methods they
    /// point at.
    pub vtables: Vec<VtableGroup>,

    /// Functions kept alive by exactly one call site, largest first.
    pub single_callers: Vec<SingleCaller>,

    /// What removing a symbol frees — itself plus everything reachable only
    /// through it — by dominator analysis from the entry point.
    pub retained: Vec<Retained>,

    /// Linked code no reference the graph can see reaches from the entry.
    pub unreachable: Unreachable,
}

#[derive(Debug, Serialize)]
pub struct Retained {
    pub name: String,

    /// Bytes of the symbol itself.
    pub own: u64,

    /// Bytes freed if it went away: itself plus everything only reachable
    /// through it.
    pub retained: u64,

    /// How many other symbols would go with it.
    pub dominated: usize,
}

#[derive(Debug, Serialize)]
pub struct Unreachable {
    pub symbols: usize,
    pub bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct VtableGroup {
    /// The trait, from the method slots' `<Type as Trait>` paths.
    pub name: String,
    pub bytes: u64,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct SingleCaller {
    pub name: String,
    pub bytes: u64,

    /// The one function that calls it — where merging it would land.
    pub caller: String,
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

/// The references collected while the assembly streams by, as interned symbol
/// ids so the collection stays a pair of flat vectors.
#[derive(Default)]
pub(crate) struct Edges {
    names: Vec<String>,
    ids: HashMap<String, u32>,
    edges: Vec<(u32, u32, Kind)>,

    /// Bytes each data symbol emits, so an anonymous vtable — absent from the
    /// symbol table — still has a size.
    data_bytes: HashMap<u32, u64>,
}

impl Edges {
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
        let edge = (self.id(from), self.id(to), Kind::Call);
        self.edges.push(edge);
    }

    pub(crate) fn address(&mut self, from: &str, to: &str) {
        let edge = (self.id(from), self.id(to), Kind::Address);
        self.edges.push(edge);
    }

    pub(crate) fn slot(&mut self, from: &str, to: &str) {
        let edge = (self.id(from), self.id(to), Kind::Data);
        self.edges.push(edge);
    }

    pub(crate) fn data_bytes(&mut self, symbol: &str, bytes: u64) {
        let id = self.id(symbol);
        *self.data_bytes.entry(id).or_default() += bytes;
    }
}

/// Read the graph in `edges`. `sizes` maps a code symbol's raw label to its
/// bytes in the binary, as `symbols::code_sizes` produces.
pub(crate) fn analyze(mut edges: Edges, sizes: &HashMap<String, u64>, limit: usize) -> GraphReport {
    edges.edges.sort_unstable();
    edges.edges.dedup();

    let (retained, unreachable) = retained(&edges, sizes, limit);
    GraphReport {
        edges: edges.edges.len(),
        vtables: vtables(&edges, limit),
        single_callers: single_callers(&edges, sizes, limit),
        retained,
        unreachable,
    }
}

/// Group the data symbols whose slots look like a vtable — drop glue plus at
/// least one method pointer — by the trait the method slots implement.
fn vtables(edges: &Edges, limit: usize) -> Vec<VtableGroup> {
    let mut slots: HashMap<u32, Vec<u32>> = HashMap::new();
    for &(from, to, kind) in &edges.edges {
        if kind == Kind::Data {
            slots.entry(from).or_default().push(to);
        }
    }

    let mut groups: HashMap<String, (u64, usize)> = HashMap::new();
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
fn single_callers(edges: &Edges, sizes: &HashMap<String, u64>, limit: usize) -> Vec<SingleCaller> {
    let mut callers: HashMap<u32, HashSet<u32>> = HashMap::new();
    let mut addressed: HashSet<u32> = HashSet::new();
    for &(from, to, kind) in &edges.edges {
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
            })
        })
        .collect();
    single.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
    single.truncate(limit);
    single
}

/// What each symbol retains — itself plus everything only reachable through it
/// — by building the dominator tree from the entry point, and the code nothing
/// reaches at all.
fn retained(
    edges: &Edges,
    sizes: &HashMap<String, u64>,
    limit: usize,
) -> (Vec<Retained>, Unreachable) {
    let Some(root) = ["_main", "main"].iter().find_map(|name| edges.ids.get(*name).copied()) else {
        // Without an entry point there is no "reachable from"; every linked
        // symbol is untraced rather than unreachable.
        return (Vec::new(), Unreachable { symbols: 0, bytes: 0 });
    };

    let mut successors: HashMap<u32, Vec<u32>> = HashMap::new();
    for &(from, to, _) in &edges.edges {
        successors.entry(from).or_default().push(to);
    }

    // Reverse postorder from the root, iteratively: the order the dominator
    // fixpoint needs, and the reachable set as a side effect.
    let mut postorder: Vec<u32> = Vec::new();
    let mut visited: HashSet<u32> = HashSet::from([root]);
    let mut stack: Vec<(u32, usize)> = vec![(root, 0)];
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
    let order: HashMap<u32, usize> =
        postorder.iter().enumerate().map(|(index, &node)| (node, index)).collect();

    let mut predecessors: HashMap<u32, Vec<u32>> = HashMap::new();
    for &(from, to, _) in &edges.edges {
        if order.contains_key(&from) && order.contains_key(&to) {
            predecessors.entry(to).or_default().push(from);
        }
    }

    // Cooper–Harvey–Kennedy: intersect predecessors' dominators to a fixpoint,
    // walking in reverse postorder. Indices are postorder positions.
    let intersect = |idom: &[Option<usize>], mut a: usize, mut b: usize| {
        while a != b {
            while a < b {
                a = idom[a].expect("processed");
            }
            while b < a {
                b = idom[b].expect("processed");
            }
        }
        a
    };
    let mut idom: Vec<Option<usize>> = vec![None; postorder.len()];
    let root_index = order[&root];
    idom[root_index] = Some(root_index);
    let mut changed = true;
    while changed {
        changed = false;
        for index in (0..postorder.len()).rev() {
            if index == root_index {
                continue;
            }
            let mut new = None;
            for &pred in predecessors.get(&postorder[index]).into_iter().flatten() {
                let pred = order[&pred];
                if idom[pred].is_none() {
                    continue;
                }
                new = Some(match new {
                    None => pred,
                    Some(current) => intersect(&idom, current, pred),
                });
            }
            if new.is_some() && idom[index] != new {
                idom[index] = new;
                changed = true;
            }
        }
    }

    // A symbol's own bytes: code from the symbol table, data from its emitted
    // slots.
    let own = |node: u32| -> u64 {
        let label = &edges.names[node as usize];
        sizes.get(label).copied().or_else(|| edges.data_bytes.get(&node).copied()).unwrap_or(0)
    };

    // Accumulate up the dominator tree. An immediate dominator always sits
    // later in postorder than what it dominates, so walking postorder forward
    // sums children before their parents see them.
    let mut totals: Vec<u64> = postorder.iter().map(|&node| own(node)).collect();
    let mut counts: Vec<usize> = vec![0; postorder.len()];
    for index in 0..postorder.len() {
        if index == root_index {
            continue;
        }
        let Some(parent) = idom[index] else { continue };
        totals[parent] += totals[index];
        counts[parent] += counts[index] + 1;
    }

    let mut rows: Vec<Retained> = postorder
        .iter()
        .enumerate()
        .filter(|&(index, _)| index != root_index && counts[index] > 0)
        .map(|(index, &node)| Retained {
            name: demangle(&edges.names[node as usize]),
            own: own(node),
            retained: totals[index],
            dominated: counts[index],
        })
        .collect();
    rows.sort_by(|a, b| b.retained.cmp(&a.retained).then_with(|| a.name.cmp(&b.name)));
    rows.truncate(limit);

    // Linked code the graph never reached: no path of calls, addresses, and
    // slots from the entry names it.
    let mut unreachable = Unreachable { symbols: 0, bytes: 0 };
    for (label, &bytes) in sizes {
        let reached = edges.ids.get(label).is_some_and(|id| order.contains_key(id));
        if !reached {
            unreachable.symbols += 1;
            unreachable.bytes += bytes;
        }
    }

    (rows, unreachable)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{Edges, analyze};

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

        let sizes: HashMap<String, u64> = [
            ("_RNvC1a3big".to_owned(), 5000),
            ("_RNvC1a6shared".to_owned(), 4000),
            ("_RNvC1a9addressed".to_owned(), 3000),
            ("_main".to_owned(), 100),
        ]
        .into();

        let report = analyze(edges, &sizes, 10);

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

        let sizes: HashMap<String, u64> = [
            ("_main", 100),
            ("_RNvC1a1a", 1000),
            ("_RNvC1a1b", 1000),
            ("_RNvC1a1c", 1000),
            ("_RNvC1a1d", 1000),
            ("_RNvC1a1e", 1000),
            ("_RNvC1a1f", 1000),
            ("_RNvC1a1z", 700),
        ]
        .map(|(name, size)| (name.to_owned(), size))
        .into();

        let report = analyze(edges, &sizes, 10);

        // `c` is reached two ways, so only `main` dominates it; the cycle back
        // from `d` changes nothing; `e` and `f` exist only through `a`.
        let rows: Vec<(&str, u64, u64, usize)> = report
            .retained
            .iter()
            .map(|row| (row.name.as_str(), row.own, row.retained, row.dominated))
            .collect();
        assert_eq!(
            rows,
            [("a::a", 1000, 3000, 2), ("a::c", 1000, 2000, 1), ("a::e", 1000, 2000, 1)]
        );

        // `z` is linked but nothing references it.
        assert_eq!(report.unreachable.symbols, 1);
        assert_eq!(report.unreachable.bytes, 700);
    }
}
