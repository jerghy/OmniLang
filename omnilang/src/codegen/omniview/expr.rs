use super::utils::{map_binary_op, serde_value_to_js};
use super::OmniViewGenerator;
use crate::codegen::{sanitize_css_value, try_format_stdlib_call, Target};
use crate::ir::*;

impl OmniViewGenerator {
    // ══════════════════════════════════════════════════════
    // Expression → JavaScript
    // ══════════════════════════════════════════════════════

    pub(super) fn expr_to_js(&self, expr: &ExprIR) -> String {
        match expr {
            ExprIR::LiteralExpr { value } => serde_value_to_js(value),
            ExprIR::IdentifierExpr { name, .. } => {
                let resolved = self
                    .name_renames
                    .get(name.as_str())
                    .or_else(|| self.global_renames.get(name.as_str()))
                    .cloned()
                    .unwrap_or_else(|| name.clone());
                if self.signal_names.contains(resolved.as_str()) {
                    format!("{}.value", resolved)
                } else if self.signal_names.contains(name.as_str()) {
                    format!("{}.value", name)
                } else if self.in_for_template
                    && self.for_state_names.contains_key(resolved.as_str())
                {
                    let (orig_name, default_js) = &self.for_state_names[resolved.as_str()];
                    format!("__omni_forGetState(el, '{}', {})", orig_name, default_js)
                } else {
                    resolved
                }
            }
            ExprIR::BinaryOpExpr { op, left, right } => {
                let l = self.expr_to_js(left);
                let r = self.expr_to_js(right);
                let js_op = map_binary_op(op);
                format!("{} {} {}", l, js_op, r)
            }
            ExprIR::UnaryOpExpr { op, operand } => {
                let o = self.expr_to_js(operand);
                format!("{}{}", op, o)
            }
            ExprIR::FieldAccessExpr { object, field } => {
                let raw_obj_name = if let ExprIR::IdentifierExpr { name, .. } = object.as_ref() {
                    Some(name.clone())
                } else {
                    None
                };
                let obj = self.expr_to_js(object);
                let check_name = raw_obj_name.as_deref().unwrap_or(&obj);
                if self.store_names.contains(check_name) || self.store_names.contains(&obj) {
                    let renamed_obj = self
                        .name_renames
                        .get(check_name)
                        .or_else(|| self.name_renames.get(obj.as_str()))
                        .cloned()
                        .unwrap_or_else(|| obj.clone());
                    let var_name = format!("{}_{}", renamed_obj, field);
                    if self.signal_names.contains(&var_name) {
                        format!("{}.value", var_name)
                    } else {
                        var_name
                    }
                } else {
                    format!("{}.{}", obj, field)
                }
            }
            ExprIR::OptionalChainExpr { object, field, .. } => {
                let raw_obj_name = if let ExprIR::IdentifierExpr { name, .. } = object.as_ref() {
                    Some(name.clone())
                } else {
                    None
                };
                let obj = self.expr_to_js(object);
                let check_name = raw_obj_name.as_deref().unwrap_or(&obj);
                if self.store_names.contains(check_name) || self.store_names.contains(&obj) {
                    let renamed_obj = self
                        .name_renames
                        .get(check_name)
                        .or_else(|| self.name_renames.get(obj.as_str()))
                        .cloned()
                        .unwrap_or_else(|| obj.clone());
                    let var_name = format!("{}_{}", renamed_obj, field);
                    if self.signal_names.contains(&var_name) {
                        format!("{}?.value", var_name)
                    } else {
                        var_name
                    }
                } else {
                    format!("{}?.{}", obj, field)
                }
            }
            ExprIR::CallExpr { name, args } => {
                if name == "println" || name == "print" {
                    let arg_strs: Vec<String> = args.iter().map(|a| self.expr_to_js(a)).collect();
                    return format!("console.log({})", arg_strs.join(", "));
                }
                let arg_strs: Vec<String> = args.iter().map(|a| self.expr_to_js(a)).collect();
                if let Some(native) = try_format_stdlib_call(name, &arg_strs, Target::OmniView) {
                    return native;
                }
                let resolved = self
                    .name_renames
                    .get(name.as_str())
                    .or_else(|| self.global_renames.get(name.as_str()))
                    .cloned()
                    .unwrap_or_else(|| name.clone());
                format!("{}({})", resolved, arg_strs.join(", "))
            }
            ExprIR::StoreFieldAccessExpr {
                store_alias,
                field_name,
                call_args,
                ..
            } => {
                let var_name = format!("{}_{}", store_alias, field_name);
                if call_args.is_empty() {
                    if self.signal_names.contains(&var_name) {
                        format!("{}.value", var_name)
                    } else {
                        var_name.clone()
                    }
                } else {
                    let arg_strs: Vec<String> =
                        call_args.iter().map(|a| self.expr_to_js(a)).collect();
                    format!("{}({})", var_name, arg_strs.join(", "))
                }
            }
            ExprIR::TernaryIR {
                cond,
                then_expr,
                else_expr,
            } => {
                let c = self.expr_to_js(cond);
                let t = self.expr_to_js(then_expr);
                let e = self.expr_to_js(else_expr);
                format!("{} ? {} : {}", c, t, e)
            }
            ExprIR::MethodCallExpr {
                is_optional: _,
                method,
                args,
                object,
                ..
            } => {
                let raw_store_name = if let ExprIR::IdentifierExpr { name, .. } = object.as_ref() {
                    name.clone()
                } else {
                    String::new()
                };
                let base = self.expr_to_js(object);
                if self.store_names.contains(&raw_store_name) || self.store_names.contains(&base) {
                    let renamed_base = self
                        .name_renames
                        .get(raw_store_name.as_str())
                        .or_else(|| self.name_renames.get(base.as_str()))
                        .cloned()
                        .unwrap_or_else(|| base.clone());
                    let var_name = format!("{}_{}", renamed_base, method);
                    let arg_strs: Vec<String> = args.iter().map(|a| self.expr_to_js(a)).collect();
                    return format!("{}({})", var_name, arg_strs.join(", "));
                }
                let obj = if self.signal_names.contains(&base) {
                    format!("{}.value", base)
                } else {
                    base
                };
                match method.as_str() {
                    "to_string" => format!("String({})", obj),
                    "len" => format!("({} || []).length", obj),
                    "is_empty" => format!("({} || []).length === 0", obj),
                    "get" => {
                        let idx = args.first().map(|a| self.expr_to_js(a)).unwrap_or_default();
                        format!("({} || [])[{}]", obj, idx)
                    }
                    "push" => {
                        let vals: Vec<String> = args.iter().map(|a| self.expr_to_js(a)).collect();
                        format!(
                            "((_a) => {{ _a.push({}); return _a.slice(); }})({} || [])",
                            vals.join(", "),
                            obj
                        )
                    }
                    "pop" => format!(
                        "((_a) => {{ _a.pop(); return _a.slice(); }})({} || [])",
                        obj
                    ),
                    _ => format!(
                        "{}.{}({})",
                        obj,
                        method,
                        args.iter()
                            .map(|a| self.expr_to_js(a))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                }
            }
            ExprIR::ArrayExpr { elements } => {
                let items: Vec<String> = elements.iter().map(|e| self.expr_to_js(e)).collect();
                format!("[{}]", items.join(", "))
            }
            ExprIR::StructExpr { fields, .. } => {
                let entries: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("\"{}\": {}", k, self.expr_to_js(v)))
                    .collect();
                format!("{{{}}}", entries.join(", "))
            }
            ExprIR::VariantLiteral {
                variant, fields, ..
            } => {
                let mut entries: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("\"{}\": {}", k, self.expr_to_js(v)))
                    .collect();
                entries.insert(0, format!("\"_variant\": \"{}\"", variant));
                format!("{{{}}}", entries.join(", "))
            }
            ExprIR::BlockExpr { statements, tail } => {
                if statements.is_empty() {
                    self.expr_to_js(tail)
                } else {
                    let mut js = String::from("(function(){ ");
                    for stmt in statements {
                        js.push_str(&self.stmt_to_js_inline(stmt));
                        js.push(';');
                    }
                    js.push_str("return ");
                    js.push_str(&self.expr_to_js(tail));
                    js.push_str("; })()");
                    js
                }
            }
            ExprIR::IndexExpr { object, index } => {
                let obj = self.expr_to_js(object);
                let idx = self.expr_to_js(index);
                let obj_access = if self.signal_names.contains(&obj) {
                    format!("{}.value", obj)
                } else {
                    obj
                };
                format!("({} || [])[{}]", obj_access, idx)
            }
            ExprIR::AwaitExpr { operand } => {
                format!("await {}", self.expr_to_js(operand))
            }
            ExprIR::MatchExpr { matched, arms } => {
                let matched_js = self.expr_to_js(matched);
                let cases: Vec<String> = arms
                    .iter()
                    .map(|arm| {
                        let body_js = self.block_to_expr(&arm.body);
                        match &arm.pattern.kind {
                            MatchPatternKindIR::Variant { name, bindings } => {
                                let let_bindings: Vec<String> = bindings
                                    .iter()
                                    .map(|b| format!(" const {} = _v.{}; ", b, b))
                                    .collect();
                                format!(
                                    "if (_v._variant === \"{}\") {{ {} return {}; }}",
                                    name,
                                    let_bindings.join(""),
                                    body_js
                                )
                            }
                            MatchPatternKindIR::Wildcard => {
                                format!("return {};", body_js)
                            }
                            MatchPatternKindIR::Literal { value } => {
                                let v = self.js_value(value);
                                format!("if (_v._variant === {}) {{ return {}; }}", v, body_js)
                            }
                        }
                    })
                    .collect();
                format!(
                    "/*match*/ (() => {{ const _v = {}; {}; throw new Error('match: no arm matched'); }})()",
                    matched_js,
                    cases.join(" ")
                )
            }
            ExprIR::ClosureExpr { params, body } => {
                let body_js = self.expr_to_js(body);
                let params_str = params.join(", ");
                format!("(({}) => {})", params_str, body_js)
            }
        }
    }

    pub(super) fn block_to_expr(&self, block: &BlockIR) -> String {
        if let Some(last) = block.statements.last() {
            match last {
                StatementIR::ExprStmtIR { expr } => self.expr_to_js(expr),
                StatementIR::ReturnStmtIR { value } => value
                    .as_ref()
                    .map(|e| self.expr_to_js(e))
                    .unwrap_or_else(|| "undefined".into()),
                _ => "undefined".into(),
            }
        } else {
            "undefined".into()
        }
    }

    pub(super) fn js_value(&self, value: &serde_json::Value) -> String {
        crate::codegen::format_json_value(value, &crate::codegen::JSON_OMNIVIEW)
    }

    pub(super) fn expr_to_static_str(&self, expr: &ExprIR) -> String {
        match expr {
            ExprIR::LiteralExpr { value } => match value {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                serde_json::Value::Null => String::new(),
                _ => String::new(),
            },
            ExprIR::IdentifierExpr { name, .. } => format!("{{{}}}", name),
            _ => String::new(),
        }
    }

    // ══════════════════════════════════════════════════════
    // Style → CSS
    // ══════════════════════════════════════════════════════

    pub(super) fn expr_to_css(&self, expr: &ExprIR) -> String {
        match expr {
            ExprIR::LiteralExpr { value } => match value {
                serde_json::Value::String(s) => sanitize_css_value(s),
                // Emit number as-is; dimensions carry their unit as strings from IR.
                // Strip trailing ".0" from integer-valued floats (serde_json stores as f64).
                serde_json::Value::Number(n) => {
                    let s = n.to_string();
                    if s.ends_with(".0") {
                        s[..s.len() - 2].to_string()
                    } else {
                        s
                    }
                }
                _ => String::new(),
            },
            _ => String::new(),
        }
    }
}
