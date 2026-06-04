use crate::ast::*;
use crate::errors::ErrorCollector;
use std::collections::{HashMap, HashSet};

/// Detect variable shadowing and generate rename maps for Dart/Python targets.
/// Returns a Vec of scope-level renames: rename_map[scope_level] = { old_name -> new_name }
pub fn check(
    program: &Program,
    _scope_info: &super::resolver::ScopeInfo,
    errors: &mut ErrorCollector,
) -> Vec<HashMap<String, String>> {
    let mut checker = ShadowChecker {
        errors,
        renames: vec![HashMap::new()], // scope level 0 (function scope)
        scope_stack: vec![HashSet::new()], // variables declared at each scope level
        counter: 0,
    };

    for func in &program.functions {
        if let Some(ref body) = func.body {
            checker.check_block(body);
        }
    }

    checker.renames
}

struct ShadowChecker<'a> {
    errors: &'a mut ErrorCollector,
    renames: Vec<HashMap<String, String>>, // per-scope-level renames
    scope_stack: Vec<HashSet<String>>,      // variables declared at each level
    counter: u32,
}

impl<'a> ShadowChecker<'a> {
    fn current_level(&self) -> usize {
        self.scope_stack.len() - 1
    }

    fn push_scope(&mut self) {
        self.scope_stack.push(HashSet::new());
        self.renames.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scope_stack.pop();
        // Keep the renames for this level (they're still referenced)
    }

    fn check_block(&mut self, block: &BlockNode) {
        for stmt in &block.stmts {
            self.check_stmt(stmt);
        }
        // trailing expr doesn't introduce new variables
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VarDecl(vd) => {
                // Check if this variable shadows any variable in outer scopes
                let name = &vd.name;
                for level in (0..self.current_level()).rev() {
                    if self.scope_stack[level].contains(name) {
                        // Found shadowing — generate rename
                        let new_name = format!("{}_shadow_{}", name, self.counter);
                        self.counter += 1;
                        let level = self.current_level(); self.renames[level].insert(name.clone(), new_name.clone());
                        // Register the NEW name in the current scope
                        let level = self.current_level(); self.scope_stack[level].insert(new_name);
                        // Skip registering the old name
                        return;
                    }
                }
                // Not shadowing — register normally
                let level = self.current_level(); self.scope_stack[level].insert(name.clone());
            }
            Stmt::Destructure(d) => {
                // Register destructured names
                match &d.pattern {
                    DestructurePattern::Array(elems, rest) => {
                        for elem in elems {
                            if let DestructureElement::Bind(name) = elem {
                                let level = self.current_level(); self.scope_stack[level].insert(name.clone());
                            }
                        }
                        if let Some(name) = rest {
                            let level = self.current_level(); self.scope_stack[level].insert(name.clone());
                        }
                    }
                    DestructurePattern::Object(fields) => {
                        for (_, bind_name) in fields {
                            let level = self.current_level(); self.scope_stack[level].insert(bind_name.clone());
                        }
                    }
                }
            }
            Stmt::IfStmt(s) => {
                self.push_scope();
                self.check_block(&s.then_body);
                self.pop_scope();
                for (_, body) in &s.else_ifs {
                    self.push_scope();
                    self.check_block(body);
                    self.pop_scope();
                }
                if let Some(ref else_body) = s.else_body {
                    self.push_scope();
                    self.check_block(else_body);
                    self.pop_scope();
                }
            }
            Stmt::WhileStmt(s) => {
                self.push_scope();
                self.check_block(&s.body);
                self.pop_scope();
            }
            Stmt::ForInStmt(s) => {
                self.push_scope();
                // Register loop variable
                let name = &s.var_name;
                // Check shadowing for loop variable
                let mut renamed = false;
                for level in (0..self.current_level() - 1).rev() {
                    if self.scope_stack[level].contains(name) {
                        let new_name = format!("{}_shadow_{}", name, self.counter);
                        self.counter += 1;
                        let level = self.current_level(); self.renames[level].insert(name.clone(), new_name);
                        renamed = true;
                        break;
                    }
                }
                if !renamed {
                    let level = self.current_level(); self.scope_stack[level].insert(name.clone());
                }
                self.check_block(&s.body);
                self.pop_scope();
            }
            _ => {}
        }
    }
}
