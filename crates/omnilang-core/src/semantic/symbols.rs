//! Symbol table and scope types for semantic analysis.

use rustc_hash::FxHashMap;
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
    pub symbols: FxHashMap<String, Symbol>,
    pub children: Vec<usize>,
}

/// Hierarchical symbol table with nested scopes.
#[derive(Debug)]
pub struct SymbolTable {
    scopes: Vec<Scope>,
    current_scope: usize, // index into scopes
    scope_by_name: FxHashMap<String, usize>,
}

impl SymbolTable {
    pub fn new() -> Self {
        let global = Scope {
            name: "global".into(),
            parent: None,
            symbols: FxHashMap::default(),
            children: vec![],
        };
        let mut scope_by_name = FxHashMap::default();
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
            symbols: FxHashMap::default(),
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

    /// Check if a symbol exists only in the current scope (not parent scopes).
    pub fn is_in_current_scope(&self, name: &str) -> bool {
        self.scopes[self.current_scope].symbols.contains_key(name)
    }

    /// Find a scope by name using hash lookup (O(1) average).
    pub fn find_scope_by_name(&self, name: &str) -> Option<usize> {
        self.scope_by_name.get(name).copied()
    }

    // ── Read-only accessors ────────────────────────────

    /// Return the current scope index.
    #[must_use]
    pub fn current_scope_index(&self) -> usize {
        self.current_scope
    }

    /// Return a reference to the current scope.
    #[must_use]
    pub fn current_scope(&self) -> &Scope {
        &self.scopes[self.current_scope]
    }

    /// Set the current scope index.
    pub fn set_current_scope(&mut self, index: usize) {
        self.current_scope = index;
    }

    /// Get a scope by index, if it exists.
    #[must_use]
    pub fn get_scope(&self, index: usize) -> Option<&Scope> {
        self.scopes.get(index)
    }

    /// Return a slice of all scopes.
    #[must_use]
    pub fn scopes(&self) -> &[Scope] {
        &self.scopes
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}
