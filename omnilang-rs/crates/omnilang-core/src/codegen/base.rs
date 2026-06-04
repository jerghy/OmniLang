use crate::ast::*;
use std::collections::HashMap;

/// Shared code generation context.
pub struct CodegenContext {
    pub output: Vec<String>,
    pub indent_level: usize,
    pub type_map: HashMap<NodeId, TypeRepr>,
    pub rename_map: Vec<HashMap<String, String>>,
    pub scope_level: usize,
}

impl CodegenContext {
    pub fn new(
        type_map: HashMap<NodeId, TypeRepr>,
        rename_map: Vec<HashMap<String, String>>,
    ) -> Self {
        Self {
            output: vec![],
            indent_level: 0,
            type_map,
            rename_map,
            scope_level: 0,
        }
    }

    pub fn emit(&mut self, line: &str) {
        let indent = "    ".repeat(self.indent_level);
        self.output.push(format!("{}{}", indent, line));
    }

    pub fn emit_raw(&mut self, s: &str) {
        self.output.push(s.to_string());
    }

    pub fn indent(&mut self) {
        self.indent_level += 1;
    }

    pub fn dedent(&mut self) {
        if self.indent_level > 0 {
            self.indent_level -= 1;
        }
    }

    pub fn enter_scope(&mut self) {
        self.scope_level += 1;
    }

    pub fn leave_scope(&mut self) {
        if self.scope_level > 0 {
            self.scope_level -= 1;
        }
    }

    /// Look up a renamed identifier (for Dart/Python shadow handling).
    pub fn rename(&self, name: &str) -> String {
        if self.scope_level < self.rename_map.len() {
            if let Some(new_name) = self.rename_map[self.scope_level].get(name) {
                return new_name.clone();
            }
        }
        name.to_string()
    }

    pub fn result(&self) -> String {
        self.output.join("\n") + "\n"
    }
}
