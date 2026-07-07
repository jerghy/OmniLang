//! Cycle detection — Tarjan SCC algorithm for import graph.
//!
//! Detects circular imports across files. Cycles are a compile error.

use crate::parser::ast::{ImportStmt, Program};
use std::collections::HashMap;
use std::sync::Arc;

/// Detect circular imports using Tarjan's strongly connected components algorithm.
///
/// `path_mapping` resolves each import's original path to the canonical file key
/// used in `programs`.
///
/// Returns list of cycles found (each cycle is a list of file paths).
pub fn detect_cycles(
    programs: &HashMap<String, Arc<Program>>,
    path_mapping: &HashMap<String, String>,
) -> Vec<Vec<String>> {
    let file_paths: Vec<&String> = programs.keys().collect();
    let path_to_idx: HashMap<&String, usize> = file_paths
        .iter()
        .enumerate()
        .map(|(i, p)| (*p, i))
        .collect();

    let n = file_paths.len();
    if n == 0 {
        return Vec::new();
    }

    let mut graph: Vec<Vec<usize>> = vec![vec![]; n];

    // Build adjacency list from imports, resolving through path_mapping
    for (path, program) in programs {
        let from = path_to_idx[path];
        for module in program.modules.values() {
            for import in &module.imports {
                let import_path = match import {
                    ImportStmt::Named(n) => &n.path,
                    ImportStmt::Namespace(ns) => &ns.path,
                    ImportStmt::TypeOnly(t) => &t.path,
                    ImportStmt::Lib(_) => continue,
                };
                // Resolve import path to canonical key
                if let Some(resolved) = path_mapping.get(import_path) {
                    if let Some(&to) = path_to_idx.get(resolved) {
                        graph[from].push(to);
                    }
                }
            }
            // Also add edges for re-exports
            for exp in &module.exports {
                if let Some(resolved) = path_mapping.get(&exp.path) {
                    if let Some(&to) = path_to_idx.get(resolved) {
                        graph[from].push(to);
                    }
                }
            }
        }
    }

    // Tarjan's SCC
    struct TarjanState {
        index: usize,
        stack: Vec<usize>,
        on_stack: Vec<bool>,
        indices: Vec<Option<usize>>,
        lowlinks: Vec<usize>,
        cycles: Vec<Vec<String>>,
    }

    let mut state = TarjanState {
        index: 0,
        stack: Vec::new(),
        on_stack: vec![false; n],
        indices: vec![None; n],
        lowlinks: vec![0; n],
        cycles: Vec::new(),
    };

    fn strongconnect(
        v: usize,
        graph: &[Vec<usize>],
        file_paths: &[&String],
        state: &mut TarjanState,
    ) {
        state.indices[v] = Some(state.index);
        state.lowlinks[v] = state.index;
        state.index += 1;
        state.stack.push(v);
        state.on_stack[v] = true;

        for &w in &graph[v] {
            if state.indices[w].is_none() {
                strongconnect(w, graph, file_paths, state);
                state.lowlinks[v] = state.lowlinks[v].min(state.lowlinks[w]);
            } else if state.on_stack[w] {
                let w_idx = match state.indices[w] {
                    Some(idx) => idx,
                    None => continue,
                };
                state.lowlinks[v] = state.lowlinks[v].min(w_idx);
            }
        }

        let v_idx = match state.indices[v] {
            Some(idx) => idx,
            None => return,
        };
        if state.lowlinks[v] == v_idx {
            let mut scc = Vec::new();
            while let Some(w) = state.stack.pop() {
                state.on_stack[w] = false;
                scc.push(file_paths[w].to_string());
                if w == v {
                    break;
                }
            }
            // A length-1 SCC is a cycle only if the vertex has a self-loop
            if scc.len() > 1 {
                state.cycles.push(scc);
            } else {
                // Check for self-import (vertex has an edge to itself)
                if graph[v].contains(&v) {
                    state.cycles.push(scc);
                }
            }
        }
    }

    for v in 0..n {
        if state.indices[v].is_none() {
            strongconnect(v, &graph, &file_paths, &mut state);
        }
    }

    state.cycles
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Span;
    use crate::parser::ast::*;

    fn make_program(imports: Vec<ImportStmt>) -> Program {
        let modules = HashMap::from([("input.omni".to_string(), {
            Module {
                imports,
                ..Default::default()
            }
        })]);
        Program { modules }
    }

    fn span() -> Span {
        Span {
            start: 0,
            end: 0,
            line: 0,
            col: 0,
        }
    }

    #[test]
    fn test_no_cycles() {
        let a = make_program(vec![ImportStmt::Named(NamedImport {
            names: vec!["Button".into()],
            path: "./b.omni".into(),
            span: span(),
        })]);
        let b = make_program(vec![]);

        let mut programs: HashMap<String, Arc<Program>> = HashMap::new();
        programs.insert("/proj/a.omni".into(), Arc::new(a));
        programs.insert("/proj/b.omni".into(), Arc::new(b));

        let mut path_mapping = HashMap::new();
        path_mapping.insert("./b.omni".into(), "/proj/b.omni".into());

        let cycles = detect_cycles(&programs, &path_mapping);
        assert!(cycles.is_empty());
    }

    #[test]
    fn test_simple_cycle() {
        let a = make_program(vec![ImportStmt::Named(NamedImport {
            names: vec!["Button".into()],
            path: "./b.omni".into(),
            span: span(),
        })]);
        let b = make_program(vec![ImportStmt::Named(NamedImport {
            names: vec!["add".into()],
            path: "./a.omni".into(),
            span: span(),
        })]);

        let mut programs: HashMap<String, Arc<Program>> = HashMap::new();
        programs.insert("/proj/a.omni".into(), Arc::new(a));
        programs.insert("/proj/b.omni".into(), Arc::new(b));

        let mut path_mapping = HashMap::new();
        path_mapping.insert("./b.omni".into(), "/proj/b.omni".into());
        path_mapping.insert("./a.omni".into(), "/proj/a.omni".into());

        let cycles = detect_cycles(&programs, &path_mapping);
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 2);
    }

    #[test]
    fn test_three_node_cycle() {
        let a = make_program(vec![ImportStmt::Named(NamedImport {
            names: vec!["X".into()],
            path: "./b.omni".into(),
            span: span(),
        })]);
        let b = make_program(vec![ImportStmt::Named(NamedImport {
            names: vec!["Y".into()],
            path: "./c.omni".into(),
            span: span(),
        })]);
        let c = make_program(vec![ImportStmt::Named(NamedImport {
            names: vec!["Z".into()],
            path: "./a.omni".into(),
            span: span(),
        })]);

        let mut programs: HashMap<String, Arc<Program>> = HashMap::new();
        programs.insert("/proj/a.omni".into(), Arc::new(a));
        programs.insert("/proj/b.omni".into(), Arc::new(b));
        programs.insert("/proj/c.omni".into(), Arc::new(c));

        let mut path_mapping = HashMap::new();
        path_mapping.insert("./b.omni".into(), "/proj/b.omni".into());
        path_mapping.insert("./c.omni".into(), "/proj/c.omni".into());
        path_mapping.insert("./a.omni".into(), "/proj/a.omni".into());

        let cycles = detect_cycles(&programs, &path_mapping);
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].len(), 3);
    }
}
