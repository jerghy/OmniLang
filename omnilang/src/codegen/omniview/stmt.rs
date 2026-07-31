//! Statement and mutation emission.
use super::OmniViewGenerator;
use crate::ir::*;
use std::fmt::Write;

impl OmniViewGenerator {
    pub(super) fn emit_statement(&mut self, stmt: &StatementIR) {
        match stmt {
            StatementIR::LetStmtIR { name, value, .. } => {
                let safe_name = super::super::sanitize_js_ident(name);
                // Register rename for body references
                if safe_name != *name {
                    self.name_resolver
                        .name_renames
                        .insert(name.clone(), safe_name.clone());
                }
                let val = value
                    .as_ref()
                    .map(|e| self.expr_to_js(e))
                    .unwrap_or_else(|| "undefined".into());
                let _ = writeln!(self.js, "  let {} = {};", safe_name, val);
            }
            StatementIR::ReturnStmtIR { value } => {
                let val = value
                    .as_ref()
                    .map(|e| self.expr_to_js(e))
                    .unwrap_or_else(|| "undefined".into());
                let _ = writeln!(self.js, "  return {};", val);
            }
            StatementIR::IfStmtIR {
                condition,
                then_body,
                else_ifs,
                else_body,
            } => {
                let cond = self.expr_to_js(condition);
                let _ = writeln!(self.js, "  if ({}) {{", cond);
                for s in &then_body.statements {
                    self.emit_statement(s);
                }
                for (elif_cond, elif_body) in else_ifs {
                    let elif_js = self.expr_to_js(elif_cond);
                    let _ = writeln!(self.js, "  }} else if ({}) {{", elif_js);
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
                let _ = writeln!(self.js, "  {};", code);
            }
            StatementIR::MutationStmt { intent } => {
                if let Some(ref intent) = intent {
                    self.emit_mutation(intent);
                }
            }
            StatementIR::WhileStmtIR { condition, body } => {
                let cond = self.expr_to_js(condition);
                let _ = writeln!(self.js, "  while ({}) {{", cond);
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
                let _ = writeln!(self.js, "  for (const {} of {}) {{", var_name, iter);
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
                let _ = writeln!(self.js, "  throw {};", e);
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
                    let _ = writeln!(self.js, "  }} catch ({}) {{", cv);
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
            StatementIR::EventEmitStmtIR { event_name, args } => {
                let arg_str: Vec<String> = args.iter().map(|a| self.expr_to_js(a)).collect();
                let resolved_name = self
                    .name_resolver
                    .name_renames
                    .get(event_name.as_str())
                    .cloned()
                    .unwrap_or_else(|| event_name.clone());
                let _ = writeln!(self.js, "  {}({});", resolved_name, arg_str.join(", "));
            }
            StatementIR::BlockStmtIR { body } => {
                for stmt in &body.statements {
                    self.emit_statement(stmt);
                }
            }
            #[allow(unreachable_patterns)]
            _ => {
                self.warnings.push(
                    "OmniView: unhandled StatementIR variant — generated output may be incomplete. \
                     This is a compiler bug; please report it.".into()
                );
            }
        }
    }

    /// Convert a single statement to an inline JS string (no buffer push).
    pub(super) fn stmt_to_js_inline(&mut self, stmt: &StatementIR) -> String {
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
                    self.format_mutation_stmt_inline(intent)
                } else {
                    String::new()
                }
            }
            StatementIR::BlockStmtIR { body } => {
                let parts: Vec<String> = body
                    .statements
                    .iter()
                    .map(|s| self.stmt_to_js_inline(s))
                    .filter(|s| !s.is_empty())
                    .collect();
                parts.join("; ")
            }
            _ => {
                self.warnings
                    .push("OmniView: unhandled StatementIR variant in inline expression".into());
                String::from("/* unhandled stmt */")
            }
        }
    }

    pub(super) fn format_mutation_stmt_inline(&mut self, intent: &MutationIntent) -> String {
        // Capture emit_mutation output by temporarily swapping the JS buffer
        let old_js = std::mem::take(&mut self.js);
        self.emit_mutation(intent);
        let result = std::mem::replace(&mut self.js, old_js);
        // Clean up leading whitespace/newlines for inline use
        result.trim().to_string()
    }

    /// Emit compound assignment operators (Increment, Decrement, Multiply, Divide, Modulo).
    /// Extracted to eliminate 5-fold repetition in emit_mutation.
    fn emit_compound_assign(
        &mut self,
        t: &str,
        amount: &Option<Box<ExprIR>>,
        js_op: &str,
        default: &str,
    ) {
        let is_signal = self.name_resolver.signal_names.contains(t)
            || self.name_resolver.store_signal_names.contains(t);
        let amt = amount
            .as_ref()
            .map(|v| self.expr_to_js(v))
            .unwrap_or_else(|| default.into());
        if is_signal {
            let _ = writeln!(
                self.js,
                "  __omni_subs.get(\"{0}\").set(__omni_subs.get(\"{0}\").value {op} {1});",
                t,
                amt,
                op = js_op
            );
        } else {
            let _ = writeln!(self.js, "  {} {}= {};", t, js_op, amt);
        }
    }

    pub(super) fn emit_mutation(&mut self, intent: &MutationIntent) {
        let resolve_target = |sig_ref: &SignalRefIR| -> String {
            self.name_resolver
                .name_renames
                .get(sig_ref.name.as_str())
                .cloned()
                .unwrap_or_else(|| sig_ref.name.clone())
        };

        match intent {
            MutationIntent::DirectAssign { target, value, .. } => {
                let t = resolve_target(target);
                let is_signal = self.name_resolver.signal_names.contains(&t)
                    || self.name_resolver.store_signal_names.contains(&t);
                let val = value
                    .as_ref()
                    .map(|v| self.expr_to_js(v))
                    .unwrap_or_else(|| "undefined".into());
                if is_signal {
                    let _ = writeln!(self.js, "  __omni_subs.get(\"{}\").set({});", t, val);
                } else {
                    let _ = writeln!(self.js, "  {} = {};", t, val);
                }
            }
            MutationIntent::Increment { target, amount } => {
                let t = resolve_target(target);
                self.emit_compound_assign(&t, amount, "+", "1");
            }
            MutationIntent::Decrement { target, amount } => {
                let t = resolve_target(target);
                self.emit_compound_assign(&t, amount, "-", "1");
            }
            MutationIntent::Multiply { target, amount } => {
                let t = resolve_target(target);
                self.emit_compound_assign(&t, amount, "*", "1");
            }
            MutationIntent::Divide { target, amount } => {
                let t = resolve_target(target);
                self.emit_compound_assign(&t, amount, "/", "1");
            }
            MutationIntent::Modulo { target, amount } => {
                let t = resolve_target(target);
                self.emit_compound_assign(&t, amount, "%", "1");
            }
            MutationIntent::Push { target, item } => {
                let t = resolve_target(target);
                let it = item
                    .as_ref()
                    .map(|v| self.expr_to_js(v))
                    .unwrap_or_else(|| "undefined".into());
                let _ = writeln!(self.js, "  __omni_subs.get(\"{0}\").value.push({1}); __omni_subs.get(\"{0}\").set(__omni_subs.get(\"{0}\").value.slice());", t, it);
            }
            MutationIntent::Pop { target } => {
                let t = resolve_target(target);
                let _ = writeln!(self.js, "  __omni_subs.get(\"{0}\").value.pop(); __omni_subs.get(\"{0}\").set(__omni_subs.get(\"{0}\").value.slice());", t);
            }
            #[allow(unreachable_patterns)]
            _ => unreachable!("Unknown MutationIntent variant — update codegen for new variant"),
        }
    }
}
