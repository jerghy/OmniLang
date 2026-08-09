use super::utils::{map_binary_op, serde_value_to_js};
use super::OmniViewGenerator;
use crate::codegen::{sanitize_css_value, try_format_stdlib_call, Target};
use crate::ir::*;

/// Toast runtime JS (spec §4.2 OmniView row, verbatim requirements):
/// global self-drawn `.omni-toast` container + `__omni_toastQueue` list +
/// timers (default 3000ms auto-dismiss, `if (ms !== 0)` guard for
/// `duration: 0` = no auto-close) + kind three-color classes
/// `.omni-toast--{kind}` + `aria-live="polite"` + `document.body.appendChild`.
const TOAST_RUNTIME_JS: &str = "\
(function () {
  if (window.__omni_toast) return;
  var _toastContainer = null;
  function ensureToastContainer() {
    if (!_toastContainer) {
      _toastContainer = document.querySelector(\".omni-toast\");
      if (!_toastContainer) {
        _toastContainer = document.createElement(\"div\");
        _toastContainer.className = \"omni-toast\";
        _toastContainer.setAttribute(\"aria-live\", \"polite\");
        document.body.appendChild(_toastContainer);
      }
    }
    return _toastContainer;
  }
  window.__omni_toastQueue = [];
  window.__omni_toast = function (message, kind, duration) {
    var ms = duration ?? 3000;
    var item = document.createElement(\"div\");
    item.className = \"omni-toast-item omni-toast--\" + (kind || \"info\");
    item.textContent = String(message);
    ensureToastContainer().appendChild(item);
    window.__omni_toastQueue.push(item);
    if (ms !== 0) {
      setTimeout(function () { item.remove(); }, ms);
    }
  };
})();
";

/// Minimal positioning + kind color classes for the self-drawn toast.
/// Emitted once per document together with `TOAST_RUNTIME_JS`.
const TOAST_CSS: &str = "\
.omni-toast { position: fixed; right: 16px; bottom: 16px; z-index: 2000; display: flex; \
flex-direction: column; gap: 8px; max-width: 320px; }\n\
.omni-toast-item { padding: 10px 14px; border-radius: 6px; background: #333; color: #fff; \
box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2); }\n\
.omni-toast--info { background: #2563eb; }\n\
.omni-toast--success { background: #16a34a; }\n\
.omni-toast--error { background: #dc2626; }\n";

/// True when the prop type is a function (`fn(String) -> String`), lowered
/// by `lower_type` to `TypeKind::Named { name: "function" }`.
pub(super) fn is_fn_type(ty: &TypeIR) -> bool {
    matches!(&ty.kind, TypeKind::Named { name } if name == "function")
}

impl OmniViewGenerator {
    // ══════════════════════════════════════════════════════
    // Expression → JavaScript
    // ══════════════════════════════════════════════════════

    pub(super) fn expr_to_js(&mut self, expr: &ExprIR) -> String {
        match expr {
            ExprIR::LiteralExpr { value } => serde_value_to_js(value),
            ExprIR::IdentifierExpr { name, .. } => {
                // Function locals (params + let) shadow store signal renames:
                // a param named `items` must NOT resolve to "TodoStore_items".
                // Accept the sanitized-ident rename (param `default` → `default_`)
                // but never a rename that is itself a signal name.
                if self.scope.fn_locals.contains(name.as_str()) {
                    return match self.name_resolver.name_renames.get(name.as_str()) {
                        Some(r)
                            if !self.name_resolver.signal_names.contains(r.as_str())
                                && !self.name_resolver.store_signal_names.contains(r.as_str()) =>
                        {
                            r.clone()
                        }
                        _ => name.clone(),
                    };
                }
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
                    && self
                        .name_resolver
                        .for_state_names
                        .contains_key(resolved.as_str())
                {
                    let (orig_name, default_js) =
                        &self.name_resolver.for_state_names[resolved.as_str()];
                    format!("__omni_forGetState(el, '{}', {})", orig_name, default_js)
                } else if self.scope.rewrite_for_item {
                    // Event handler / rebind fn emitted to top-level JS: the
                    // @for loop variable is out of scope there. The delegated
                    // listener sets __omni_forItemVar from the clicked clone
                    // before invoking the handler. Check the RESOLVED name:
                    // prop references inside an inlined child component map to
                    // "item" or "item.field" (e.g. id → "item.id"), not the
                    // bare loop variable.
                    if let Some(item) = self.scope.for_item_var.as_deref() {
                        if resolved == item {
                            "__omni_forItemVar".to_string()
                        } else if let Some(rest) = resolved.strip_prefix(item) {
                            if rest.starts_with('.') {
                                format!("__omni_forItemVar{}", rest)
                            } else {
                                resolved
                            }
                        } else {
                            resolved
                        }
                    } else {
                        resolved
                    }
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
                // Event-call form `onSwitch("x")` (semantically equivalent to
                // `emit onSwitch("x")`): resolve component-scoped FIRST (fix
                // 7.5b-K1). The module-wide rename tables below are last-wins
                // and cannot distinguish same-named events across components —
                // every TabBar button used to dispatch to AboutPage's emitter.
                // fn-prop/computed calls are unaffected: this branch only fires
                // when the current component actually declares the event.
                if let Some(prefixed) = self.component_event_prefixed(name) {
                    return format!("{}({})", prefixed, arg_strs.join(", "));
                }
                let resolved = self
                    .name_resolver
                    .name_renames
                    .get(name.as_str())
                    .or_else(|| self.name_resolver.global_renames.get(name.as_str()))
                    .cloned()
                    .unwrap_or_else(|| name.clone());
                if self.name_resolver.signal_names.contains(resolved.as_str()) {
                    if self.name_resolver.fn_prop_names.contains(name.as_str()) {
                        // fn-typed prop invoked with call syntax:
                        // `{transform("hello")}` → the stored value IS the
                        // function, so call it: X.value("hello"). Optional
                        // props default to null — degrade to an empty string
                        // instead of throwing until the parent wires the prop.
                        format!(
                            "({}.value || (function () {{ return ''; }}))({})",
                            resolved,
                            arg_strs.join(", ")
                        )
                    } else {
                        // Function-form computed invoked with call syntax:
                        // `{fullName()}` → the stored __omni_computed object's
                        // .value getter, not a function call.
                        format!("{}.value", resolved)
                    }
                } else {
                    format!("{}({})", resolved, arg_strs.join(", "))
                }
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
                // e.stopPropagation() / e.preventDefault() inside an event
                // handler are compile-time directives: the listener-level
                // flag (needs_stop_propagation / needs_prevent_default) is
                // what actually calls them on the real event. The handler
                // receives the extracted event-data object ({} when no fields
                // are used), which has no such methods — emit a no-op instead
                // of `{}.stopPropagation()` (TypeError).
                if (method == "stopPropagation" || method == "preventDefault")
                    && matches!(object.as_ref(), ExprIR::IdentifierExpr { name, .. } if name == "e")
                    && args.is_empty()
                {
                    return String::new();
                }
                let raw_store_name = if let ExprIR::IdentifierExpr { name, .. } = object.as_ref() {
                    name.clone()
                } else {
                    String::new()
                };
                let base = self.expr_to_js(object);
                if self.name_resolver.store_names.contains(&raw_store_name)
                    || self.name_resolver.store_names.contains(&base)
                {
                    // Phase 5.2: the store (top-level OR inline — both are
                    // module StoreIRs now) emits a store-level action
                    // function (`CartStore_addItem`). Prefer it over the
                    // legacy inline-store flattened-fn rename, so reads and
                    // writes stay on ONE state. Before this, an inline store
                    // inlined into a parent view split the state: reads
                    // resolved to the store-level signals while method calls
                    // resolved to the flattened component fn mutating
                    // component-level copies (verified on InlineStoreDemo).
                    let store_level_fn = if raw_store_name.is_empty() {
                        format!("{}_{}", base, method)
                    } else {
                        format!("{}_{}", raw_store_name, method)
                    };
                    if self
                        .name_resolver
                        .store_field_names
                        .contains(&store_level_fn)
                    {
                        let arg_strs: Vec<String> =
                            args.iter().map(|a| self.expr_to_js(a)).collect();
                        return format!("{}({})", store_level_fn, arg_strs.join(", "));
                    }
                    // Legacy fallback: resolve the method through name_renames
                    // (inline store where the store action is merged as a
                    // component function, e.g. CartStore.addItem →
                    // InlineStoreDemo_addItem).
                    if let Some(resolved_fn) = self
                        .name_resolver
                        .name_renames
                        .get(method.as_str())
                        .cloned()
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
                    "len" => {
                        // Map literals compile to plain JS objects (no native
                        // Map) — `({} || []).length` is undefined on objects.
                        // Arrays/strings keep `.length`; objects fall back to
                        // own-key count; null/undefined → 0 (mirrors the
                        // `|| []` null-safety of the previous shape).
                        format!(
                            "((_c) => Array.isArray(_c) || typeof _c === \"string\" ? _c.length : (_c ? Object.keys(_c).length : 0))({obj})"
                        )
                    }
                    "is_empty" => {
                        // Same dispatch as `len` — an object's size is its
                        // own-key count, not `undefined === 0`.
                        format!(
                            "((_c) => Array.isArray(_c) || typeof _c === \"string\" ? _c.length : (_c ? Object.keys(_c).length : 0))({obj}) === 0"
                        )
                    }
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
                    "contains" => {
                        // Map literals compile to plain JS objects (no native
                        // Map) — `.contains(key)` must not emit `obj.contains()`
                        // (TypeError). Arrays → includes; objects → hasOwnProperty.
                        let idx = args.first().map(|a| self.expr_to_js(a)).unwrap_or_default();
                        format!(
                            "((_c) => Array.isArray(_c) ? _c.includes({idx}) : Object.prototype.hasOwnProperty.call(_c, {idx}))({obj})"
                        )
                    }
                    _ => {
                        // Method-call form of std functions: `s.to_uppercase()`
                        // maps through the shared stdlib table with the receiver
                        // as the first argument ((s).toUpperCase() etc.).
                        let arg_strs: Vec<String> =
                            args.iter().map(|a| self.expr_to_js(a)).collect();
                        let mut std_args = vec![obj.clone()];
                        std_args.extend(arg_strs.iter().cloned());
                        if let Some(native) =
                            try_format_stdlib_call(method, &std_args, Target::OmniView)
                        {
                            native
                        } else {
                            format!("{}.{}({})", obj, method, arg_strs.join(", "))
                        }
                    }
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
                                // Literal patterns compare the subject value
                                // itself — primitives carry no `_variant` field,
                                // so `_v._variant === 0` never matched.
                                let v = self.js_value(value);
                                format!("if (_v === {}) {{ return {}; }}", v, body_js)
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
            ExprIR::ArrayMethod {
                method,
                subject,
                predicate,
            } => {
                // Array language methods (design doc 域 8): JS-family mapping
                // with the defensive `|| []` guard used by the other array
                // ops (len/get/push/contains) so a not-yet-initialized
                // signal value degrades to an empty array instead of
                // throwing. count → filter(pred).length (no JS count).
                let base = self.expr_to_js(subject);
                let obj = if self.name_resolver.signal_names.contains(&base) {
                    format!("{}.value", base)
                } else {
                    base
                };
                let pred = self.expr_to_js(predicate);
                match method {
                    ArrayMethodIR::Filter => format!("(({obj}) || []).filter({pred})"),
                    ArrayMethodIR::Map => format!("(({obj}) || []).map({pred})"),
                    ArrayMethodIR::Count => {
                        format!("(({obj}) || []).filter({pred}).length")
                    }
                }
            }
            ExprIR::RefOpExpr { ref_name, op } => self.format_ref_op(ref_name, op),
            // showToast 自绘全局容器调用（spec §4.2 OmniView row）：惰性创建
            // `.omni-toast` 容器（aria-live 播报）+ `__omni_toastQueue` 队列 +
            // 定时器（默认 3000ms 自动消失，duration 0 不自动关闭——`ms !== 0`
            // 守卫，F6 语义，与 react/component/mod.rs 一致）。运行时由
            // emit_toast_runtime_once 每文档注入一次。
            ExprIR::BuiltinToastCall {
                msg,
                kind,
                duration,
            } => {
                self.emit_toast_runtime_once();
                let msg_js = self.expr_to_js(msg);
                let kind_str = match kind {
                    Some(ToastKindIR::Info) => "info",
                    Some(ToastKindIR::Success) => "success",
                    Some(ToastKindIR::Error) => "error",
                    // V6 已硬错误拒绝 Unknown——到达 codegen 即语义违约；
                    // 原样透传保持诚实（与 core shared/expr.rs 一致）。
                    Some(ToastKindIR::Unknown(raw)) => raw.as_str(),
                    None => "info",
                };
                let mut call = format!("__omni_toast({msg_js}, \"{kind_str}\"");
                if let Some(d) = duration {
                    call = format!("{call}, {d}");
                }
                call.push(')');
                call
            }
            #[allow(unreachable_patterns)]
            _ => unreachable!("Unknown ExprIR variant — update codegen for new variant"),
        }
    }

    // ══════════════════════════════════════════════════════
    // showToast runtime (spec §4.2)
    // ══════════════════════════════════════════════════════

    /// Emit the global self-drawn toast runtime once per document: a
    /// lazily-created `.omni-toast` container with `aria-live="polite"`
    /// appended to `document.body`, the `__omni_toastQueue` list, and the
    /// `__omni_toast(message, kind, duration)` helper. Duration defaults to
    /// 3000ms; an explicit `duration: 0` disables auto-close (`if (ms !== 0)`
    /// guard). Kind maps onto the three-color modifier classes
    /// `.omni-toast--{info|success|error}`. The whole runtime is wrapped in
    /// an IIFE guarded by `window.__omni_toast` because top-level deferred JS
    /// accumulates across the whole document and must stay collision-free.
    fn emit_toast_runtime_once(&mut self) {
        if self.toast_runtime_emitted {
            return;
        }
        self.toast_runtime_emitted = true;
        self.css.push_str(TOAST_CSS);
        // Emit into the deferred buffer, NOT self.js: the call site formats
        // mid-function-body, and pushing here would land the runtime INSIDE
        // the enclosing function. all_deferred_js runs at top level after DOM
        // insertion and before mount lifecycle handlers — exactly where the
        // lazily-created body container belongs.
        self.all_deferred_js.push_str(TOAST_RUNTIME_JS);
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
            RefOpIR::CapturePointer => {
                // Parameterless DSL form (design D3) — the wrapper uses the
                // runtime's active-pointer tracking (__omni_activePointerId),
                // so no event object needs to be in scope at the call site.
                format!("{base}.capturePointer()")
            }
            RefOpIR::ReleasePointer => {
                // Native DOM API is `releasePointerCapture` — the wrapper method
                // is named accordingly (matches the React backend convention).
                format!("{base}.releasePointerCapture()")
            }
            // RefOpIR is #[non_exhaustive] — cross-crate matches need a wildcard.
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
        // ── @form 内置状态（Task 6）──
        // form.errors / form.valid / form.submitting 及两级 form.errors.<field>
        // 仅在当前组件存在 @form 意图时接管；否则按常规 store/属性解析。
        if self.scope.form_intent.is_some() {
            if let Some(js) = self.try_form_builtin_access(object, field) {
                return js;
            }
        }
        let raw_obj_name = if let ExprIR::IdentifierExpr { name, .. } = object {
            Some(name.clone())
        } else {
            None
        };
        let obj = self.expr_to_js(object);
        let check_name = raw_obj_name.as_deref().unwrap_or(&obj);
        if self.name_resolver.store_names.contains(check_name)
            || self.name_resolver.store_names.contains(&obj)
        {
            let renamed_obj = self
                .name_resolver
                .name_renames
                .get(check_name)
                .or_else(|| self.name_resolver.name_renames.get(obj.as_str()))
                .cloned()
                .unwrap_or_else(|| obj.clone());
            let var_name = format!("{}_{}", renamed_obj, field);
            // Store signal names persist in store_signal_names even after
            // generate_component clears signal_names (component-local) — a
            // store field referenced late in codegen (e.g. dynamic token
            // values) must still resolve to `.value`, not the signal object.
            if self.name_resolver.signal_names.contains(&var_name)
                || self.name_resolver.store_signal_names.contains(&var_name)
            {
                format!("{}{}", var_name, value_op)
            } else {
                var_name
            }
        } else {
            format!("{}{}{}", obj, dot_op, field)
        }
    }

    /// @form 内置状态访问解析（Task 6）：
    /// - 单级 `form.X`（X ∈ {errors, valid, submitting}）→ `{Comp}_formX` 信号的
    ///   `.value`（editor 模式经 `__omni_subs.get` 统一读取）。
    /// - 两级 `form.errors.<field>` → `{Comp}_formErrors.value["<field>"]`。
    ///
    /// 非 form 形态返回 None，由调用方走常规解析。
    fn try_form_builtin_access(&self, object: &ExprIR, field: &str) -> Option<String> {
        let builtin = |f: &str| match f {
            "errors" => Some("formErrors"),
            "valid" => Some("formValid"),
            "submitting" => Some("formSubmitting"),
            _ => None,
        };
        let comp = &self.scope.current_component;
        let signal_suffix = match object {
            ExprIR::IdentifierExpr { name, .. } if name == "form" => builtin(field)?,
            ExprIR::FieldAccessExpr {
                object: inner,
                field: mid,
            } => {
                let ExprIR::IdentifierExpr { name, .. } = inner.as_ref() else {
                    return None;
                };
                if name != "form" {
                    return None;
                }
                let suffix = builtin(mid)?;
                // 两级访问：form.errors.<field> → 信号 value 的字段索引
                return Some(if self.editor_mode {
                    format!(
                        "__omni_subs.get(\"{}_{}\").value[\"{}\"]",
                        comp, suffix, field
                    )
                } else {
                    format!("{}_{}.value[\"{}\"]", comp, suffix, field)
                });
            }
            _ => return None,
        };
        Some(if self.editor_mode {
            format!("__omni_subs.get(\"{}_{}\").value", comp, signal_suffix)
        } else {
            format!("{}_{}.value", comp, signal_suffix)
        })
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
