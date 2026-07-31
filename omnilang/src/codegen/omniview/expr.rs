use super::utils::{map_binary_op, serde_value_to_js};
use super::OmniViewGenerator;
use crate::codegen::{sanitize_css_value, try_format_stdlib_call, Target};
use crate::ir::*;

impl OmniViewGenerator {
    // ══════════════════════════════════════════════════════
    // Expression → JavaScript
    // ══════════════════════════════════════════════════════

    pub(super) fn expr_to_js(&mut self, expr: &ExprIR) -> String {
        match expr {
            ExprIR::LiteralExpr { value } => serde_value_to_js(value),
            ExprIR::IdentifierExpr { name, .. } => {
                let resolved = self
                    .name_resolver
                    .name_renames
                    .get(name.as_str())
                    .or_else(|| self.name_resolver.global_renames.get(name.as_str()))
                    .cloned()
                    .unwrap_or_else(|| name.clone());
                if self.name_resolver.signal_names.contains(resolved.as_str()) {
                    if self.editor_mode {
                        format!("__omni_subs.get(\"{}\").value", resolved)
                    } else {
                        format!("{}.value", resolved)
                    }
                } else if self.name_resolver.signal_names.contains(name.as_str()) {
                    if self.editor_mode {
                        format!("__omni_subs.get(\"{}\").value", name)
                    } else {
                        format!("{}.value", name)
                    }
                } else if self.scope.in_for_template
                    && self.name_resolver.for_state_names.contains_key(resolved.as_str())
                {
                    let (orig_name, default_js) = &self.name_resolver.for_state_names[resolved.as_str()];
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
                self.resolve_store_field_access(object, field, ".", ".value")
            }
            ExprIR::OptionalChainExpr { object, field, .. } => {
                self.resolve_store_field_access(object, field, "?.", "?.value")
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
                    .name_resolver
                    .name_renames
                    .get(name.as_str())
                    .or_else(|| self.name_resolver.global_renames.get(name.as_str()))
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
                // Resolve store alias through renames (supports inline stores
                // where "CartStore" → "InlineStoreDemo")
                let resolved_alias = self
                    .name_resolver
                    .name_renames
                    .get(store_alias.as_str())
                    .cloned()
                    .unwrap_or_else(|| store_alias.clone());
                let var_name = format!("{}_{}", resolved_alias, field_name);
                if call_args.is_empty() {
                    if self.name_resolver.signal_names.contains(&var_name) {
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
                if self.name_resolver.store_names.contains(&raw_store_name) || self.name_resolver.store_names.contains(&base) {
                    // First try: resolve the method through name_renames (handles inline stores
                    // where the store action is merged as a component function, e.g.
                    // CartStore.addItem → InlineStoreDemo_addItem)
                    if let Some(resolved_fn) = self.name_resolver.name_renames.get(method.as_str()).cloned()
                    {
                        let arg_strs: Vec<String> =
                            args.iter().map(|a| self.expr_to_js(a)).collect();
                        return format!("{}({})", resolved_fn, arg_strs.join(", "));
                    }
                    // Fallback: resolve the store name through name_renames
                    let renamed_base = self
                        .name_resolver
                        .name_renames
                        .get(raw_store_name.as_str())
                        .or_else(|| self.name_resolver.name_renames.get(base.as_str()))
                        .cloned()
                        .unwrap_or_else(|| base.clone());
                    let var_name = format!("{}_{}", renamed_base, method);
                    let arg_strs: Vec<String> = args.iter().map(|a| self.expr_to_js(a)).collect();
                    return format!("{}({})", var_name, arg_strs.join(", "));
                }
                let obj = if self.name_resolver.signal_names.contains(&base) {
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
            ExprIR::SpreadExpr { operand } => {
                format!("...{}", self.expr_to_js(operand))
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
                let obj_access = if self.name_resolver.signal_names.contains(&obj) {
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
            ExprIR::MapExpr { entries } => {
                let items: Vec<String> = entries
                    .iter()
                    .map(|(k, v)| format!("{}: {}", self.expr_to_js(k), self.expr_to_js(v)))
                    .collect();
                format!("{{{}}}", items.join(", "))
            }
            ExprIR::RefOpExpr { ref_name, op } => self.format_ref_op(ref_name, op),
            #[allow(unreachable_patterns)]
            _ => unreachable!("Unknown ExprIR variant — update codegen for new variant"),
        }
    }

    /// Format a semantic ref operation (`ref.focus()`, `ref.scrollTo(x, y)`, …)
    /// to a native DOM API call on the ref wrapper object emitted by
    /// `emit_ref_wrappers` (store.rs). Every wrapper method guards internally
    /// via `__omni_el(nid)` — when the element is not yet mounted the op
    /// degrades to a no-op instead of throwing. `Measure` chains
    /// `getBoundingClientRect()` (the wrapper returns a zero-rect when
    /// unmounted, so `.width` / `.height` never hit null).
    fn format_ref_op(&mut self, ref_name: &str, op: &RefOpIR) -> String {
        let base = ref_name.to_string();
        match op {
            RefOpIR::Focus => format!("{base}.focus()"),
            RefOpIR::Blur => format!("{base}.blur()"),
            RefOpIR::ScrollTo { x, y } => self.format_ref_scroll(&base, x.as_deref(), y.as_deref()),
            RefOpIR::ScrollBy { dx, dy } => {
                self.format_ref_scroll_by(&base, dx.as_deref(), dy.as_deref())
            }
            RefOpIR::Measure { what } => match what {
                MeasureWhat::Position => format!("{base}.getBoundingClientRect()"),
                MeasureWhat::Width => format!("{base}.getBoundingClientRect().width"),
                MeasureWhat::Height => format!("{base}.getBoundingClientRect().height"),
            },
            RefOpIR::Select => format!("{base}.select()"),
            RefOpIR::SetSelection { start, end } => format!(
                "{base}.setSelectionRange({}, {})",
                self.expr_to_js(start),
                self.expr_to_js(end)
            ),
            RefOpIR::Value => format!("{base}.value"),
            // Assignment needs an explicit guard — `?.` is illegal on an
            // assignment LHS, and a setValue from @mount runs before the
            // ref attaches (would otherwise throw TypeError).
            RefOpIR::SetValue { value } => {
                format!(
                    "if ({base}) {{ {base}.value = {}; }}",
                    self.expr_to_js(value)
                )
            }
            RefOpIR::CapturePointer { id } => {
                format!("{base}.capturePointer({})", self.expr_to_js(id))
            }
            RefOpIR::ReleasePointer { id } => {
                format!("{base}.releasePointer({})", self.expr_to_js(id))
            }
            #[allow(unreachable_patterns)]
            _ => format!("/* RefOp::{op:?} on {ref_name} */"),
        }
    }

    /// Format `ref.scrollTo({ left: x, top: y })` — only axes provided in the
    /// source call are included in the options object.
    fn format_ref_scroll(&mut self, base: &str, x: Option<&ExprIR>, y: Option<&ExprIR>) -> String {
        let mut parts: Vec<String> = Vec::new();
        if let Some(x) = x {
            parts.push(format!("left: {}", self.expr_to_js(x)));
        }
        if let Some(y) = y {
            parts.push(format!("top: {}", self.expr_to_js(y)));
        }
        format!("{base}.scrollTo({{ {} }})", parts.join(", "))
    }

    /// Format `ref.scrollBy({ left: dx, top: dy })` — relative scroll delta.
    fn format_ref_scroll_by(
        &mut self,
        base: &str,
        dx: Option<&ExprIR>,
        dy: Option<&ExprIR>,
    ) -> String {
        let mut parts: Vec<String> = Vec::new();
        if let Some(dx) = dx {
            parts.push(format!("left: {}", self.expr_to_js(dx)));
        }
        if let Some(dy) = dy {
            parts.push(format!("top: {}", self.expr_to_js(dy)));
        }
        format!("{base}.scrollBy({{ {} }})", parts.join(", "))
    }

    pub(super) fn block_to_expr(&mut self, block: &BlockIR) -> String {
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

    /// Shared store-name resolution logic for FieldAccessExpr and OptionalChainExpr.
    /// `dot_op` is the access operator (`.` or `?.`) used when the object is NOT a store.
    /// `value_op` is the value accessor (`.value` or `?.value`) used when the resolved
    /// variable is a signal (store field accessed via signal).
    fn resolve_store_field_access(
        &mut self,
        object: &ExprIR,
        field: &str,
        dot_op: &str,
        value_op: &str,
    ) -> String {
        let raw_obj_name = if let ExprIR::IdentifierExpr { name, .. } = object {
            Some(name.clone())
        } else {
            None
        };
        let obj = self.expr_to_js(object);
        let check_name = raw_obj_name.as_deref().unwrap_or(&obj);
        if self.name_resolver.store_names.contains(check_name) || self.name_resolver.store_names.contains(&obj) {
            let renamed_obj = self
                .name_resolver
                .name_renames
                .get(check_name)
                .or_else(|| self.name_resolver.name_renames.get(obj.as_str()))
                .cloned()
                .unwrap_or_else(|| obj.clone());
            let var_name = format!("{}_{}", renamed_obj, field);
            if self.name_resolver.signal_names.contains(&var_name) {
                format!("{}{}", var_name, value_op)
            } else {
                var_name
            }
        } else {
            format!("{}{}{}", obj, dot_op, field)
        }
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
    // Expression → Label (human-readable short name)
    // ══════════════════════════════════════════════════════

    /// Extract a short human-readable label from an expression.
    /// Returns the identifier name for identifiers, "obj.field" for field access,
    /// and the trimmed JS expression for complex expressions.
    pub(super) fn expr_label(&mut self, expr: &ExprIR) -> String {
        match expr {
            ExprIR::IdentifierExpr { name, .. } => name.clone(),
            ExprIR::FieldAccessExpr { object, field } => {
                let obj_label = self.expr_label(object);
                format!("{}.{}", obj_label, field)
            }
            ExprIR::MethodCallExpr { object, method, .. } => {
                let obj_label = self.expr_label(object);
                format!("{}.{}()", obj_label, method)
            }
            _ => {
                // For complex expressions, use the JS representation trimmed
                let js = self.expr_to_js(expr);
                let trimmed = js.trim_end_matches("()");
                // Truncate long expressions — use char boundary to avoid UTF-8 panics
                if trimmed.chars().count() > 40 {
                    let end = trimmed
                        .char_indices()
                        .nth(39)
                        .map(|(i, _)| i)
                        .unwrap_or(trimmed.len());
                    format!("{}…", &trimmed[..end])
                } else {
                    trimmed.to_string()
                }
            }
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
