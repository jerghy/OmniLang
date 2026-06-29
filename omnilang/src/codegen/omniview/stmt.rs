//! Statement and mutation emission.
use super::OmniViewGenerator;
use crate::ir::*;

impl OmniViewGenerator {
    pub(super) fn emit_statement(&mut self, stmt: &StatementIR) {
        match stmt {
            StatementIR::LetStmtIR { name, value, .. } => {
                let safe_name = super::super::sanitize_js_ident(name);
                // Register rename for body references
                if safe_name != *name {
                    self.name_renames.insert(name.clone(), safe_name.clone());
                }
                let val = value
                    .as_ref()
                    .map(|e| self.expr_to_js(e))
                    .unwrap_or_else(|| "undefined".into());
                self.js
                    .push_str(&format!("  let {} = {};\n", safe_name, val));
            }
            StatementIR::ReturnStmtIR { value } => {
                let val = value
                    .as_ref()
                    .map(|e| self.expr_to_js(e))
                    .unwrap_or_else(|| "undefined".into());
                self.js.push_str(&format!("  return {};\n", val));
            }
            StatementIR::IfStmtIR {
                condition,
                then_body,
                else_ifs,
                else_body,
            } => {
                let cond = self.expr_to_js(condition);
                self.js.push_str(&format!("  if ({}) {{\n", cond));
                for s in &then_body.statements {
                    self.emit_statement(s);
                }
                for (elif_cond, elif_body) in else_ifs {
                    let elif_js = self.expr_to_js(elif_cond);
                    self.js
                        .push_str(&format!("  }} else if ({}) {{\n", elif_js));
                    for s in &elif_body.statements {
                        self.emit_statement(s);
                    }
                }
                if let Some(ref else_b) = else_body {
                    self.js.push_str("  } else {\n");
                    for s in &else_b.statements {
                        self.emit_statement(s);
                    }
                }
                self.js.push_str("  }\n");
            }
            StatementIR::ExprStmtIR { expr } => {
                let code = self.expr_to_js(expr);
                self.js.push_str(&format!("  {};\n", code));
            }
            StatementIR::MutationStmt { intent } => {
                if let Some(ref intent) = intent {
                    self.emit_mutation(intent);
                }
            }
            StatementIR::WhileStmtIR { condition, body } => {
                let cond = self.expr_to_js(condition);
                self.js.push_str(&format!("  while ({}) {{\n", cond));
                for s in &body.statements {
                    self.emit_statement(s);
                }
                self.js.push_str("  }\n");
            }
            StatementIR::ForInStmtIR {
                var_name,
                iterable,
                body,
            } => {
                let iter = self.expr_to_js(iterable);
                self.js
                    .push_str(&format!("  for (const {} of {}) {{\n", var_name, iter));
                for s in &body.statements {
                    self.emit_statement(s);
                }
                self.js.push_str("  }\n");
            }
            StatementIR::BreakStmtIR => {
                self.js.push_str("  break;\n");
            }
            StatementIR::ContinueStmtIR => {
                self.js.push_str("  continue;\n");
            }
            StatementIR::ThrowStmtIR { expr } => {
                let e = self.expr_to_js(expr);
                self.js.push_str(&format!("  throw {};\n", e));
            }
            StatementIR::TryCatchStmtIR {
                try_body,
                catch_var,
                catch_body,
                finally_body,
            } => {
                self.js.push_str("  try {\n");
                for s in &try_body.statements {
                    self.emit_statement(s);
                }
                if let Some(ref cv) = catch_var {
                    self.js.push_str(&format!("  }} catch ({}) {{\n", cv));
                } else if catch_body.is_some() {
                    self.js.push_str("  } catch (_) {\n");
                }
                if let Some(ref cb) = catch_body {
                    for s in &cb.statements {
                        self.emit_statement(s);
                    }
                }
                if let Some(ref fb) = finally_body {
                    self.js.push_str("  } finally {\n");
                    for s in &fb.statements {
                        self.emit_statement(s);
                    }
                }
                self.js.push_str("  }\n");
            }
            _ => {
                // Placeholder for unhandled statements
                self.js.push_str("  /* unhandled stmt */\n");
            }
        }
    }

    /// Convert a single statement to an inline JS string (no buffer push).
    pub(super) fn stmt_to_js_inline(&self, stmt: &StatementIR) -> String {
        match stmt {
            StatementIR::LetStmtIR { name, value, .. } => {
                let safe_name = super::super::sanitize_js_ident(name);
                let val = value
                    .as_ref()
                    .map(|e| self.expr_to_js(e))
                    .unwrap_or_else(|| "undefined".into());
                format!("let {} = {}", safe_name, val)
            }
            StatementIR::ExprStmtIR { expr } => self.expr_to_js(expr),
            StatementIR::ReturnStmtIR { value } => {
                let val = value
                    .as_ref()
                    .map(|e| self.expr_to_js(e))
                    .unwrap_or_else(|| "undefined".into());
                format!("return {}", val)
            }
            StatementIR::MutationStmt { intent } => {
                if let Some(intent) = intent {
                    let target = self
                        .name_renames
                        .get(intent.target.name.as_str())
                        .cloned()
                        .unwrap_or_else(|| intent.target.name.clone());
                    let op = &intent.operation;
                    match &intent.amount {
                        Some(amt) => {
                            format!("{} {} {}", target, op, self.expr_to_js(amt))
                        }
                        None => format!("{} {}", target, op),
                    }
                } else {
                    String::new()
                }
            }
            _ => String::from("/* unhandled stmt in block expr */"),
        }
    }

    pub(super) fn emit_mutation(&mut self, intent: &MutationIntent) {
        let target = self
            .name_renames
            .get(intent.target.name.as_str())
            .cloned()
            .unwrap_or_else(|| intent.target.name.clone());
        // Check if target is a known signal (component-local or store).
        // If not, it's a local variable — emit regular JS assignment.
        let is_signal =
            self.signal_names.contains(&target) || self.store_signal_names.contains(&target);
        let op = &intent.operation;
        if !is_signal {
            // Local variable mutation — plain JS assignment
            let amt = intent
                .amount
                .as_ref()
                .map(|e| self.expr_to_js(e))
                .unwrap_or_else(|| "undefined".into());
            match op.as_str() {
                "direct_assign" => {
                    self.js.push_str(&format!("  {} = {};\n", target, amt));
                }
                "increment" => {
                    self.js.push_str(&format!("  {} += {};\n", target, amt));
                }
                "decrement" => {
                    self.js.push_str(&format!("  {} -= {};\n", target, amt));
                }
                "multiply" => {
                    self.js.push_str(&format!("  {} *= {};\n", target, amt));
                }
                "divide" => {
                    self.js.push_str(&format!("  {} /= {};\n", target, amt));
                }
                "modulo" => {
                    self.js.push_str(&format!("  {} %= {};\n", target, amt));
                }
                _ => {
                    self.js.push_str(&format!("  {} = {};\n", target, amt));
                }
            }
            return;
        }
        match op.as_str() {
            "direct_assign" => {
                let val = intent
                    .amount
                    .as_ref()
                    .map(|e| self.expr_to_js(e))
                    .unwrap_or_else(|| "undefined".into());
                self.js.push_str(&format!("  {}.set({});\n", target, val));
            }
            "increment" => {
                let amt = intent
                    .amount
                    .as_ref()
                    .map(|e| self.expr_to_js(e))
                    .unwrap_or_else(|| "1".into());
                self.js
                    .push_str(&format!("  {}.set({}.value + {});\n", target, target, amt));
            }
            "decrement" => {
                let amt = intent
                    .amount
                    .as_ref()
                    .map(|e| self.expr_to_js(e))
                    .unwrap_or_else(|| "1".into());
                self.js
                    .push_str(&format!("  {}.set({}.value - {});\n", target, target, amt));
            }
            "multiply" => {
                let amt = intent
                    .amount
                    .as_ref()
                    .map(|e| self.expr_to_js(e))
                    .unwrap_or_else(|| "1".into());
                self.js
                    .push_str(&format!("  {}.set({}.value * {});\n", target, target, amt));
            }
            "divide" => {
                let amt = intent
                    .amount
                    .as_ref()
                    .map(|e| self.expr_to_js(e))
                    .unwrap_or_else(|| "1".into());
                self.js
                    .push_str(&format!("  {}.set({}.value / {});\n", target, target, amt));
            }
            "modulo" => {
                let amt = intent
                    .amount
                    .as_ref()
                    .map(|e| self.expr_to_js(e))
                    .unwrap_or_else(|| "1".into());
                self.js
                    .push_str(&format!("  {}.set({}.value % {});\n", target, target, amt));
            }
            "push" => {
                let item = intent
                    .amount
                    .as_ref()
                    .map(|e| self.expr_to_js(e))
                    .unwrap_or_else(|| "undefined".into());
                self.js.push_str(&format!(
                    "  {}.value.push({}); {}.set({}.value.slice());\n",
                    target, item, target, target
                ));
            }
            "pop" => {
                self.js.push_str(&format!(
                    "  {}.value.pop(); {}.set({}.value.slice());\n",
                    target, target, target
                ));
            }
            _ => {
                self.js
                    .push_str(&format!("  /* mutation: {} {} */\n", target, op));
            }
        }
    }
}
