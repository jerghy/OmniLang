//! Symbol table and scope types for semantic analysis.

use std::collections::HashMap;

/// A single symbol (variable, function, signal, etc.)
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub type_ann: Option<String>,
    pub mutable: bool,
    pub declared_at: usize, // position in source
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Variable,
    Signal,
    Computed,
    Prop,
    Event,
    Ref,
    Function,
    Action,
    Store,
    Component,
    Getter,
    Param,
    Unknown,
}

/// A scope in the symbol hierarchy.
#[derive(Debug, Clone)]
pub struct Scope {
    pub name: String,
    pub parent: Option<usize>, // index into SymbolTable.scopes
    pub symbols: HashMap<String, Symbol>,
    pub children: Vec<usize>,
}

/// Hierarchical symbol table with nested scopes.
#[derive(Debug)]
pub struct SymbolTable {
    pub scopes: Vec<Scope>,
    pub current_scope: usize, // index into scopes
    pub scope_by_name: HashMap<String, usize>,
}

impl SymbolTable {
    pub fn new() -> Self {
        let global = Scope {
            name: "global".into(),
            parent: None,
            symbols: HashMap::new(),
            children: vec![],
        };
        let mut scope_by_name = HashMap::new();
        scope_by_name.insert("global".to_string(), 0);
        SymbolTable {
            scopes: vec![global],
            current_scope: 0,
            scope_by_name,
        }
    }

    /// Enter a new child scope, returning its index.
    pub fn push_scope(&mut self, name: &str) -> usize {
        let parent = self.current_scope;
        let idx = self.scopes.len();
        self.scopes.push(Scope {
            name: name.to_string(),
            parent: Some(parent),
            symbols: HashMap::new(),
            children: vec![],
        });
        self.scopes[parent].children.push(idx);
        self.current_scope = idx;
        // Map scope name to index (use first occurrence; duplicate names use latest)
        self.scope_by_name.insert(name.to_string(), idx);
        idx
    }

    /// Exit current scope, returning to parent.
    pub fn pop_scope(&mut self) {
        if let Some(parent) = self.scopes[self.current_scope].parent {
            self.current_scope = parent;
        }
    }

    /// Declare a symbol in the current scope.
    pub fn declare(&mut self, symbol: Symbol) -> Result<(), String> {
        let scope = &mut self.scopes[self.current_scope];
        if scope.symbols.contains_key(&symbol.name) {
            return Err(format!(
                "Duplicate declaration: '{}' in scope '{}'",
                symbol.name, scope.name
            ));
        }
        scope.symbols.insert(symbol.name.clone(), symbol);
        Ok(())
    }

    /// Resolve a symbol by walking up the scope chain.
    pub fn resolve(&self, name: &str) -> Option<&Symbol> {
        let mut scope_idx = self.current_scope;
        loop {
            let scope = &self.scopes[scope_idx];
            if let Some(sym) = scope.symbols.get(name) {
                return Some(sym);
            }
            match scope.parent {
                Some(parent) => scope_idx = parent,
                None => return None,
            }
        }
    }

    /// Check if a symbol exists in any visible scope.
    pub fn contains(&self, name: &str) -> bool {
        self.resolve(name).is_some()
    }

    /// Find a scope by name using hash lookup (O(1) average).
    pub fn find_scope_by_name(&self, name: &str) -> Option<usize> {
        self.scope_by_name.get(name).copied()
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}
