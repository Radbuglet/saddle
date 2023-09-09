use petgraph::{algo::toposort, stable_graph::NodeIndex, visit::EdgeRef, Direction};
use rustc_hash::{FxHashMap, FxHashSet};
use std::fmt::{self, Write};

// === Helpers === //

const INDENT_SIZE: u32 = 4;

#[derive(Copy, Clone)]
struct Indent(u32);

impl fmt::Display for Indent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for _ in 0..self.0 {
            f.write_char(' ')?;
        }
        Ok(())
    }
}

fn borrow_two<T>(list: &mut [T], a: usize, b: usize) -> (&mut T, &mut T) {
    assert_ne!(a, b);

    if a < b {
        let (left, right) = list.split_at_mut(a + 1);
        (&mut left[a], &mut right[b - a - 1])
    } else {
        let (b, a) = borrow_two(list, b, a);
        (a, b)
    }
}

// === Definitions === //

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ScopeId<'a>(pub String, pub [&'a (); 0]);

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ComponentId<'a>(pub String, pub [&'a (); 0]);

#[derive(Debug, Clone)]
pub struct ScopeMeta<'a> {
    pub _dummy: [&'a (); 0],
    pub name: String,
    pub defined_at: &'a str,
}

#[derive(Debug, Clone)]
pub struct ComponentMeta<'a> {
    pub _dummy: [&'a (); 0],
    pub name: String,
}

#[derive(Debug, Copy, Clone)]
pub struct CallMeta<'a> {
    pub def_path: &'a str,
}

#[derive(Debug, Copy, Clone)]
pub struct BorrowMeta<'a> {
    pub def_path: &'a str,
    pub mutability: Mutability,
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Default)]
pub enum Mutability {
    #[default]
    Immutable,
    Mutable,
}

impl Mutability {
    pub fn adjective(self) -> &'static str {
        match self {
            Mutability::Mutable => "mutably",
            Mutability::Immutable => "immutably",
        }
    }

    pub fn is_compatible_with(self, other: Mutability) -> bool {
        use Mutability::*;
        matches!((self, other), (Immutable, Immutable))
    }

    pub fn strictest(self, other: Mutability) -> Self {
        use Mutability::*;
        if matches!((self, other), (Immutable, Immutable)) {
            Immutable
        } else {
            Mutable
        }
    }
}

// === Validator === //

#[derive(Debug, Default)]
pub struct Validator<'a> {
    call_graph: petgraph::Graph<Scope<'a>, CallMeta<'a>>,
    scope_id_to_node: FxHashMap<ScopeId<'a>, NodeIndex>,
    component_meta: FxHashMap<ComponentId<'a>, ComponentMeta<'a>>,
}

#[derive(Debug, Default)]
struct Scope<'a> {
    borrows: FxHashMap<ComponentId<'a>, (Mutability, Vec<BorrowMeta<'a>>)>,
    grants: FxHashMap<ComponentId<'a>, (Mutability, Vec<BorrowMeta<'a>>)>,
    meta: Option<ScopeMeta<'a>>,
}

impl<'a> Validator<'a> {
    fn get_scope_node(&mut self, scope: ScopeId<'a>) -> NodeIndex {
        *self
            .scope_id_to_node
            .entry(scope)
            .or_insert_with(|| self.call_graph.add_node(Scope::default()))
    }

    pub fn push_call_edge(&mut self, from: ScopeId<'a>, to: ScopeId<'a>, meta: CallMeta<'a>) {
        let from_idx = self.get_scope_node(from);
        let to_idx = self.get_scope_node(to);

        self.call_graph.add_edge(from_idx, to_idx, meta);
    }

    pub fn push_access(
        &mut self,
        scope: ScopeId<'a>,
        component: ComponentId<'a>,
        req_access: Mutability,
        meta: BorrowMeta<'a>,
    ) {
        let scope_idx = self.get_scope_node(scope);
        let (curr_access, metas) = self.call_graph[scope_idx]
            .borrows
            .entry(component)
            .or_insert_with(Default::default);

        *curr_access = curr_access.strictest(req_access);
        metas.push(meta);
    }

    pub fn push_grant(
        &mut self,
        scope: ScopeId<'a>,
        component: ComponentId<'a>,
        req_access: Mutability,
        meta: BorrowMeta<'a>,
    ) {
        let scope_idx = self.get_scope_node(scope);
        let (curr_access, metas) = self.call_graph[scope_idx]
            .grants
            .entry(component)
            .or_insert_with(Default::default);

        *curr_access = curr_access.strictest(req_access);
        metas.push(meta);
    }

    pub fn annotate_scope(&mut self, scope: ScopeId<'a>, meta: ScopeMeta<'a>) {
        let scope_idx = self.get_scope_node(scope);
        self.call_graph[scope_idx].meta = Some(meta);
    }

    pub fn annotate_component(&mut self, component: ComponentId<'a>, meta: ComponentMeta<'a>) {
        self.component_meta.insert(component, meta);
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        // Assuming our graph is a DAG, toposort the scopes.
        let Ok(topos) = toposort(&self.call_graph, None) else {
            // If the graph is not a DAG, we know that it is invalid since a dependency issue could
            // be induced by taking the same borrowing edge several times.
            //
            // We generate a list of offending scopes using "Tarjan's strongly connected components
            // algorithm." A strongly connected component (or SCC) is a set of nodes in a graph
            // where each node in the set has a path to another node in that set. We know that
            // finding the SCCs in a graph is an effective way of finding portions of the graph
            // containing cycles because:
            //
            // 1. If the graph contains a cycle, that cycle will be part of an SCC (although the SCC may
            //    contain more nodes than just it).
            // 2. If the graph contains an SCC, within that SCC, we can construct many simple cycles
            //    by taking any of the paths from any of the nodes to itself.
            //
            // Hence, determining SCCs is an effective way of printing out portions of the graph with
            // offending cycles.
            //
            // We decided to list out SCCs rather than simple cycles because, in the worst case scenario,
            // the number of simple cycles in a graph grows factorially w.r.t the number of vertices.
            // This is because, in a K^n graph, our cycles would be at least all possible permutations of
            // those `n` nodes.
            let sccs = petgraph::algo::tarjan_scc(&self.call_graph);
            let mut f = String::new();
            write!(
                f,
                "Failed to validate saddle scope graph: scopes may be called in a cycle, which could \
                 cause borrow violations.\n\
                 \n\
                 The following scopes form cycles:\n\n",
            ).unwrap();

            let mut i = 1;

            for scc in sccs.into_iter() {
                // If the SCC is just an individual node without any self-edges, ignore it.
                if scc.len() == 1
                    && !self
                        .call_graph
                        .edges_directed(scc[0], Direction::Outgoing)
                        .any(|edge| edge.source() == edge.target())
                {
                    continue;
                }

                // Otherwise, print out the SCCs nodes and the way they connect to one another.
                writeln!(f, "Cycle {}:", i).unwrap();
                i += 1;

                let scc = scc.into_iter().collect::<FxHashSet<_>>();

                for &scope in &scc {
                    writeln!(
                        f,
                        " - Scope {}, which could call into...",
                        self.call_graph[scope].meta.as_ref().unwrap().name,
                    )
                    .unwrap();

                    for caller in self.call_graph.edges_directed(scope, Direction::Outgoing) {
                        if !scc.contains(&caller.target()) {
                            continue;
                        }

                        writeln!(
                            f,
                            "    - Scope {} through the invocation declared at {}",
                            self.call_graph[caller.target()].meta.as_ref().unwrap().name,
                            caller.weight().def_path,
                        )
                        .unwrap();
                    }
                }

                writeln!(f).unwrap();
            }

            anyhow::bail!("{f}");
        };

        // Working in topological order, we populate the set of all components which could possibly
        // be borrowed when a scope is entered.
        struct ValidationCx<'a> {
            validator: &'a Validator<'a>,
            potentially_borrowed: Vec<FxHashMap<ComponentId<'a>, Mutability>>,
            err_msg_or_empty: String,
        }

        impl<'a> ValidationCx<'a> {
            pub fn new(validator: &'a Validator, node_count: usize) -> Self {
                Self {
                    validator,
                    potentially_borrowed: (0..node_count).map(|_| FxHashMap::default()).collect(),
                    err_msg_or_empty: String::new(),
                }
            }

            pub fn validate_scope(&mut self, scope: NodeIndex, scope_info: &Scope) {
                let f = &mut self.err_msg_or_empty;
                let pbs = &self.potentially_borrowed[scope.index()];

                for (req_ty, (req_mut, _req_meta)) in &scope_info.borrows {
                    // If the request is compatible with the PBS, ignore it.
                    let Some(pre_mut) = pbs.get(&req_ty) else {
                        continue;
                    };

                    if pre_mut.is_compatible_with(*req_mut) {
                        return;
                    }

                    // Otherwise, log out the error chain.
                    let comp_meta = &self.validator.component_meta[&req_ty];
                    writeln!(
                        f,
                        "The scope {} defined at {} borrows the component {} {} even though it may have already been borrowed {}.",
                        scope_info.meta.as_ref().unwrap().name,
                        scope_info.meta.as_ref().unwrap().defined_at,
                        comp_meta.name,
                        req_mut.adjective(),
                        pre_mut.adjective(),
                    )
                    .unwrap();

                    // TODO: Improve diagnostics for sub grants
                    fn print_tree<'a>(
                        f: &mut String,
                        validator: &Validator,
                        potentially_borrowed: &[FxHashMap<ComponentId<'a>, Mutability>],
                        desired_comp: &ComponentId<'a>,
                        desired_mut: Mutability,
                        target: NodeIndex,
                        indent: u32,
                    ) {
                        // There are two ways our target node may have been called with a specific
                        // offending borrow type: inherited and direct.

                        // We begin by logging out the direct calls.
                        for borrow_meta in validator.call_graph[target]
                            .borrows
                            .get(&desired_comp)
                            .map_or(&Vec::new(), |(_, borrow_meta)| borrow_meta)
                        {
                            writeln!(
                                f,
                                "{}- This scope could have borrowed the component {} at location {}.",
                                Indent(indent),
                                borrow_meta.mutability.adjective(),
                                borrow_meta.def_path,
                            )
                            .unwrap();
                        }

                        // Now, we log out indirect calls.
                        let mut printed_callers = FxHashSet::default();

                        for caller in validator
                            .call_graph
                            .neighbors_directed(target, Direction::Incoming)
                        {
                            if !printed_callers.insert(caller) {
                                continue;
                            }

                            let Some(caller_mut) = potentially_borrowed[caller.index()]
                                .get(&desired_comp)
                                .filter(|v| !v.is_compatible_with(desired_mut))
                            else {
                                continue;
                            };

                            writeln!(
                                f,
                                "{}- The scope {} defined at {} may have called it while the component was held {}.\n\
                                 {}  Hint: the following scopes may have been responsible for the aforementioned call...",
                                Indent(indent),
                                validator.call_graph[caller].meta.as_ref().unwrap().name,
                                validator.call_graph[caller].meta.as_ref().unwrap().defined_at,
                                caller_mut.adjective(),
                                Indent(indent),
                            )
                            .unwrap();

                            for edge in validator.call_graph.edges_connecting(caller, target) {
                                writeln!(
                                    f,
                                    "{}- {}",
                                    Indent(indent + INDENT_SIZE),
                                    edge.weight().def_path,
                                )
                                .unwrap();
                            }

                            writeln!(f, "{}  Tracing back responsibility...", Indent(indent))
                                .unwrap();

                            print_tree(
                                f,
                                validator,
                                potentially_borrowed,
                                desired_comp,
                                desired_mut,
                                caller,
                                indent + INDENT_SIZE,
                            );
                        }
                    }

                    print_tree(
                        f,
                        self.validator,
                        &self.potentially_borrowed,
                        req_ty,
                        *req_mut,
                        scope,
                        INDENT_SIZE,
                    );

                    f.push_str("\n\n");
                }
            }

            pub fn propagate_borrows_to_self(&mut self, src_idx: NodeIndex, src: &'a Scope) {
                // Propagate scope borrows to self
                // TODO: This is fine to run several times but really shouldn't be.
                for (req_ty, (req_mut, _req_meta)) in &src.borrows {
                    let pbs_mut = self.potentially_borrowed[src_idx.index()]
                        .entry(req_ty.clone())
                        .or_insert(Mutability::Immutable);

                    *pbs_mut = pbs_mut.strictest(*req_mut);
                }
            }

            pub fn propagate_borrows_to_others(&mut self, caller: NodeIndex, callee: NodeIndex) {
                let (caller_pbs, callee_pbs) = borrow_two(
                    &mut self.potentially_borrowed,
                    caller.index(),
                    callee.index(),
                );

                for (req_ty, req_mut) in &*caller_pbs {
                    let mut req_mut = *req_mut;

                    // Downgrade callee's PBS if they have a grant for the specific component.
                    match self.validator.call_graph[callee]
                        .grants
                        .get(&req_ty)
                        .map(|(m, _)| *m)
                    {
                        Some(Mutability::Immutable) => req_mut = Mutability::Immutable,
                        Some(Mutability::Mutable) => continue,
                        None => {}
                    }

                    // Extend the callee's PBS.
                    let pbs_mut = callee_pbs
                        .entry(req_ty.clone())
                        .or_insert(Mutability::Immutable);

                    *pbs_mut = pbs_mut.strictest(req_mut);
                }
            }
        }

        let mut cx = ValidationCx::new(self, self.call_graph.node_count());

        for src_idx in topos {
            let src = &self.call_graph[src_idx];

            // Validate ourselves given our PBS
            cx.validate_scope(src_idx, src);

            // Extend our own PBS with our borrows
            cx.propagate_borrows_to_self(src_idx, src);

            // Propagate it to others
            for callee in self.call_graph.edges_directed(src_idx, Direction::Outgoing) {
                cx.propagate_borrows_to_others(src_idx, callee.target());
            }
        }

        // If we had any errors while validating this graph
        if !cx.err_msg_or_empty.is_empty() {
            anyhow::bail!(
                "Failed to validate the scope graph:\n\n{}",
                cx.err_msg_or_empty,
            );
        }

        // Otherwise, the graph is fully valid.
        Ok(())
    }
}
