use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Resolve an import path relative to a source file.
pub fn resolve_import(import_path: &str, from_file: &Path) -> Option<PathBuf> {
    let base = from_file.parent()?;
    let resolved = base.join(import_path);
    // Try .omni extension
    if resolved.exists() {
        return Some(resolved);
    }
    let with_ext = resolved.with_extension("omni");
    if with_ext.exists() {
        return Some(with_ext);
    }
    // Try directory with index.omni
    let index = resolved.join("index.omni");
    if index.exists() {
        return Some(index);
    }
    None
}

/// Build a dependency graph from a set of modules.
pub struct ModuleGraph {
    pub nodes: HashMap<String, ModuleNode>,
}

pub struct ModuleNode {
    pub path: String,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
}

impl ModuleGraph {
    pub fn new() -> Self {
        Self { nodes: HashMap::new() }
    }

    pub fn add_module(&mut self, path: &str, imports: Vec<String>, exports: Vec<String>) {
        self.nodes.insert(path.to_string(), ModuleNode {
            path: path.to_string(), imports, exports,
        });
    }

    /// Detect cycles using Tarjan's SCC algorithm.
    pub fn detect_cycles(&self) -> Vec<Vec<String>> {
        let mut cycles = vec![];
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();
        let mut path = vec![];

        for node in self.nodes.keys() {
            if !visited.contains(node) {
                self.dfs(node, &mut visited, &mut stack, &mut path, &mut cycles);
            }
        }
        cycles
    }

    fn dfs(&self, node: &str, visited: &mut HashSet<String>, stack: &mut HashSet<String>,
           path: &mut Vec<String>, cycles: &mut Vec<Vec<String>>) {
        visited.insert(node.to_string());
        stack.insert(node.to_string());
        path.push(node.to_string());

        if let Some(n) = self.nodes.get(node) {
            for imp in &n.imports {
                if stack.contains(imp) {
                    // Found a cycle
                    let start = path.iter().position(|x| x == imp).unwrap_or(0);
                    cycles.push(path[start..].to_vec());
                } else if !visited.contains(imp) {
                    self.dfs(imp, visited, stack, path, cycles);
                }
            }
        }

        stack.remove(node);
        path.pop();
    }

    /// Topological sort of modules (for compilation order).
    pub fn compilation_order(&self) -> Vec<String> {
        let mut order = vec![];
        let mut visited = HashSet::new();
        for node in self.nodes.keys() {
            if !visited.contains(node) {
                self.topo(node, &mut visited, &mut order);
            }
        }
        order.reverse();
        order
    }

    fn topo(&self, node: &str, visited: &mut HashSet<String>, order: &mut Vec<String>) {
        visited.insert(node.to_string());
        if let Some(n) = self.nodes.get(node) {
            for imp in &n.imports {
                if !visited.contains(imp) {
                    self.topo(imp, visited, order);
                }
            }
        }
        order.push(node.to_string());
    }
}

impl Default for ModuleGraph {
    fn default() -> Self { Self::new() }
}

/// Module loader: recursively load .omni files starting from an entry point.
pub fn load_modules(entry: &Path) -> Result<Vec<(String, String)>, String> {
    let mut loaded = vec![];
    let mut seen = HashSet::new();
    let mut queue = vec![entry.to_path_buf()];
    while let Some(path) = queue.pop() {
        let canon = path.canonicalize().map_err(|e| format!("{}: {}", path.display(), e))?;
        let key = canon.to_string_lossy().to_string();
        if seen.contains(&key) { continue; }
        seen.insert(key.clone());
        let source = std::fs::read_to_string(&canon)
            .map_err(|e| format!("Cannot read {}: {}", canon.display(), e))?;
        // Extract imports
        for line in source.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("import ") {
                if let Some(path_str) = extract_import_path(trimmed) {
                    if let Some(resolved) = resolve_import(&path_str, &canon) {
                        queue.push(resolved);
                    }
                }
            }
        }
        loaded.push((key, source));
    }
    Ok(loaded)
}

fn extract_import_path(line: &str) -> Option<String> {
    // Simple extraction: "import { ... } from \"path\"" -> path
    let from_pos = line.find("from ")?;
    let after_from = &line[from_pos + 5..].trim();
    if after_from.starts_with('"') {
        let end = after_from[1..].find('"')?;
        Some(after_from[1..=end].to_string())
    } else {
        None
    }
}
