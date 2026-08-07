// ═══════════════════════════════════════════════════════════
// View tree generation (JSX)
// ═══════════════════════════════════════════════════════════

use crate::codegen::escape_literal_str;
use crate::codegen::shared::inline_handler::{build_inline_handler, InlineHandlerConfig};
use crate::codegen::shared::style::{format_style_prop_key, format_style_value_expr};
use crate::codegen::CodeWriter;
use crate::ir::*;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

use super::super::capitalize_first;
use super::component::jsx::component_declared_slots;
use super::design::DesignCtx;
use super::expr::{format_expr_ir_tsx_renamed, generate_statement_tsx};
use super::utils::{
    emit_event_object_react, to_jsx_attr, to_react_event, to_react_event_or_legacy, to_react_tag,
};
use crate::codegen::sanitize::is_dangerous_url;
use std::fmt::Write;
use std::sync::Mutex;

/// Split a qualified binding target (`Store.field`) and verify it against
/// the store-owner map — the same `bare-field → store-ref` map the mutation
/// machinery uses. Returns `(owner, field)` only when the field genuinely
/// belongs to the named store; anything else falls back to the component
/// setter path (never `setStore.field(...)`).
fn split_qualified_signal<'a>(
    name: &'a str,
    store_signal_owners: &FxHashMap<String, String>,
) -> Option<(&'a str, &'a str)> {
    let (owner, field) = name.split_once('.')?;
    if store_signal_owners.get(field).map(String::as_str) == Some(owner) {
        Some((owner, field))
    } else {
        None
    }
}

/// v4 H3: 组件信号是否为数字类型（i8-u64/f32/f64/isize/usize）。
/// Optional 已在 lowering 折叠为 `TypeIR::is_nullable`（kind 判定天然覆盖
/// `i32?`）；Union 折叠为 `a | b` 型 Named 名（`lower_type` 按 kind 串拼接）——
/// 任一成员数字即视为数字（`i32 | String` 的写回仍需强转：对数字分支是类型
/// 诚实，对字符串分支仅损失类型精确度，value 仍可读）。
fn signal_is_numeric(comp: &ComponentIR, name: &str) -> bool {
    comp.signals
        .iter()
        .any(|s| s.name == name && type_ir_is_numeric(&s.r#type))
}

/// Whether a TypeIR denotes a numeric type: the canonical Int/Float kinds
/// (i32/i64/f32/f64 — `TypeKind::from` folds them) or a numeric name from
/// the narrow-int set (i8/u8/u16/u32/u64/isize/usize stay `Named`; unions
/// join member kind strings with " | ", so each member is checked).
fn type_ir_is_numeric(ty: &TypeIR) -> bool {
    match &ty.kind {
        TypeKind::Int | TypeKind::Float => true,
        TypeKind::Named { name } => name.split(" | ").any(|member| {
            matches!(
                member,
                "int"
                    | "float"
                    | "i8"
                    | "i16"
                    | "i32"
                    | "i64"
                    | "u8"
                    | "u16"
                    | "u32"
                    | "u64"
                    | "f32"
                    | "f64"
                    | "isize"
                    | "usize"
            )
        }),
        _ => false,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn gen_view_node_tsx(
    w: &mut CodeWriter,
    node: &ViewTree,
    comp: &ComponentIR,
    component_map: &FxHashMap<&str, &ComponentIR>,
    design_ctx: &DesignCtx,
    signal_names: &FxHashSet<String>,
    store_signal_owners: &FxHashMap<String, String>,
    motion_names: &FxHashSet<String>,
    ref_names: &FxHashSet<String>,
    store_field_renames: &FxHashMap<String, String>,
    pruned_signal_names: &FxHashSet<String>,
    possibly_undefined: &FxHashSet<String>,
    direct_write_signals: &FxHashSet<String>,
    direct_write_applies: &FxHashMap<String, Vec<String>>,
    motion_counter: &mut usize,
    prop_fn_ctx: &PropFnMemoCtx,
    warnings: &Mutex<Vec<String>>,
    // Whether this node sits directly in JSX-children position (between
    // element/fragment tags). Expression nodes (conditional, list
    // rendering) must then be wrapped in a JSX expression container
    // `{...}`; in a parenthesized expression position (component view
    // root, conditional branch bodies, slot fallbacks, map callbacks)
    // the container braces are a syntax error — `return ( { x } )` is
    // not valid TS (view-root block expressions, I2).
    in_jsx_children: bool,
) {
    match node {
        ViewTree::Element(el) => {
            // Fragment tag -> React fragment shorthand (transparent — the
            // children take the fragment's own position, so the flag is
            // inherited, not forced)
            if el.tag == "fragment" {
                if el.children.is_empty() {
                    return;
                }
                for child in &el.children {
                    gen_view_node_tsx(
                        w,
                        child,
                        comp,
                        component_map,
                        design_ctx,
                        signal_names,
                        store_signal_owners,
                        motion_names,
                        ref_names,
                        store_field_renames,
                        pruned_signal_names,
                        possibly_undefined,
                        direct_write_signals,
                        direct_write_applies,
                        motion_counter,
                        prop_fn_ctx,
                        warnings,
                        in_jsx_children,
                    );
                }
                return;
            }

            // Component composition: check if tag is a known component
            if let Some(child_comp) = component_map.get(el.tag.as_str()) {
                // Ref forwarding for custom components is Phase 2 — skip for now
                if el.ref_binding.is_some() {
                    // Only native HTML elements support @ref in the current phase
                }
                // @drag (ADR-073) needs the element handle (@ref) that
                // component instances don't get in the current phase — the
                // four synthesized pointer props would degrade to plain
                // callback props on the child and capture would never work.
                // Warn + skip (same family as the ref-forwarding note above).
                if el.drag_binding.is_some() {
                    warnings.lock().unwrap().push(
                        "@drag 在组件实例上不支持（组件实例无 @ref 元素句柄）——@drag 已跳过，请在原生 HTML 元素上使用"
                            .into(),
                    );
                }
                // Partition children: elements carrying a literal
                // `slot="name"` attribute, or consumer-position
                // `@slot name { ... }` projections, matching a declared slot
                // project into the named prop; everything else goes to the
                // default `children` prop. Unknown slot names fall back into
                // `children` (content stays visible and the generated TS
                // never breaks).
                let (default_children, named_children) =
                    partition_component_children(el, child_comp, component_map);
                // Task 2.8 (perf M4): a children-free instance whose every
                // prop/event value is render-stable (literals / signals /
                // computeds / props / store refs / useCallback-wrapped fns)
                // instantiates the `Memoized<Child>` alias — the memo
                // shallow-compare skips the child on prop-equal re-renders
                // (list-cascade elimination). Fresh arrows and dynamic
                // expressions disqualify the instance (memo would never hit).
                let instance_tag = if prop_fn_ctx.memo_children.contains(el.tag.as_str())
                    && component_instance_memoizable(
                        el,
                        !default_children.is_empty() || !named_children.is_empty(),
                        prop_fn_ctx,
                    ) {
                    format!("Memoized{}", el.tag)
                } else {
                    el.tag.clone()
                };
                let mut comp_attrs: Vec<String> = Vec::new();
                for (prop_name, prop_value) in &el.props {
                    // A local fn passed BY VALUE is emitted as its stable
                    // `__omni_cb_<fn>` useCallback wrapper (task 2.8).
                    let js_val = match prop_value {
                        ExprIR::IdentifierExpr { name, .. }
                            if prop_fn_ctx.wrapped_fns.contains(name) =>
                        {
                            format!("__omni_cb_{}", name)
                        }
                        _ => format_expr_ir_tsx_renamed(prop_value, ref_names, store_field_renames),
                    };
                    comp_attrs.push(format!("{}={{{}}}", prop_name, js_val));
                }
                // a11y block → aria-* props on the component element (React
                // components accept and forward aria-* props — never a
                // silent drop).
                comp_attrs.extend(a11y_react_attrs(el, ref_names, store_field_renames));
                for (event_name, handler) in &el.events {
                    // Component-instance events are callback PROPS of the
                    // child (declared `event onX` become optional props,
                    // component/mod.rs) — NOT DOM listeners. An unmapped
                    // canonical (longpress/hover) keeps the legacy `onX`
                    // spelling so a child-declared event prop of the same
                    // name still wires (v4 3.3; DOM elements warn + skip
                    // instead — see gen_view_node_tsx; toggle maps to
                    // onToggle, task 6.1b — same spelling either way).
                    let jsx_event = to_react_event_or_legacy(event_name);
                    match handler {
                        EventHandlerIR::FnRef { r#ref, .. } => {
                            // Bare fn-ref events on a component instance use
                            // the stable useCallback wrapper when the ref is
                            // a local fn (task 2.8).
                            let ref_emitted = if prop_fn_ctx.wrapped_fns.contains(r#ref) {
                                format!("__omni_cb_{}", r#ref)
                            } else {
                                r#ref.clone()
                            };
                            comp_attrs.push(format!("{}={{{}}}", jsx_event, ref_emitted));
                        }
                        EventHandlerIR::InlineFn { body, params, .. } => {
                            let handler_str = format_inline_handler(
                                body,
                                signal_names,
                                store_signal_owners,
                                params,
                                ref_names,
                                store_field_renames,
                                pruned_signal_names,
                                possibly_undefined,
                                direct_write_signals,
                                direct_write_applies,
                            );
                            comp_attrs.push(format!("{}={{{}}}", jsx_event, handler_str));
                        }
                        EventHandlerIR::StoreAction { r#ref, .. } => {
                            // Call through the action slice when the store is
                            // subscribed per field (T3.3), otherwise through
                            // the store hook object.
                            let action_ref = store_field_renames
                                .get(r#ref)
                                .cloned()
                                .unwrap_or_else(|| r#ref.clone());
                            comp_attrs
                                .push(format!("{}={{() => {{ {}(); }}}}", jsx_event, action_ref));
                        }
                    }
                }
                for (slot_name, child) in &named_children {
                    // Render the projected content into the named prop and
                    // strip the `slot` attribute so it never leaks into the
                    // DOM (it is only meaningful for web components).
                    let mut child_w = crate::codegen::CodeWriter::new();
                    gen_consumer_child_tsx(
                        &mut child_w,
                        child,
                        comp,
                        component_map,
                        design_ctx,
                        signal_names,
                        store_signal_owners,
                        motion_names,
                        ref_names,
                        store_field_renames,
                        pruned_signal_names,
                        possibly_undefined,
                        direct_write_signals,
                        direct_write_applies,
                        motion_counter,
                        prop_fn_ctx,
                        warnings,
                        // the content lands inside `slotName={...}` — an
                        // attribute expression, not JSX children
                        false,
                    );
                    let child_jsx = child_w.into_string();
                    let trimmed = child_jsx.trim();
                    if !trimmed.is_empty() {
                        comp_attrs.push(format!("{}={{{}}}", slot_name, trimmed));
                    }
                }
                let attrs_str = comp_attrs.join(" ");
                if default_children.is_empty() {
                    w.emit_fmt(format_args!("<{} {}/>", instance_tag, attrs_str));
                } else {
                    w.emit_fmt(format_args!("<{} {}>", instance_tag, attrs_str));
                    w.indent();
                    for child in default_children {
                        gen_consumer_child_tsx(
                            w,
                            child,
                            comp,
                            component_map,
                            design_ctx,
                            signal_names,
                            store_signal_owners,
                            motion_names,
                            ref_names,
                            store_field_renames,
                            pruned_signal_names,
                            possibly_undefined,
                            direct_write_signals,
                            direct_write_applies,
                            motion_counter,
                            prop_fn_ctx,
                            warnings,
                            // default children render between the tags —
                            // JSX-children position
                            true,
                        );
                    }
                    w.dedent();
                    w.emit_fmt(format_args!("</{}>", instance_tag));
                }
                return; // IMPORTANT: skip HTML element handling
            }
            // Otherwise, fall through to existing HTML element handling
            // Normalize OmniView custom tags (Button→button, Card→div, etc.)
            let html_tag = to_react_tag(&el.tag);
            // Build attribute list from props, bindings, events
            let mut attrs: Vec<String> = Vec::new();

            // Collect binding attr names for dedup with static props
            let bound_attr_names: FxHashSet<String> = el
                .bindings
                .keys()
                .map(|bind_name| to_jsx_attr(bind_name))
                .collect();

            // Collect class_map_binding names so we can exclude them from static className
            let binding_names: FxHashSet<&str> = el
                .class_map_bindings
                .keys()
                .map(|cls| cls.as_str())
                .collect();

            // Static props — skip names that will be populated by bindings
            // `_design_variant` (variant: DesignName.Variant(dim=val)) is
            // expanded into literal global class names after the other class
            // attributes are merged (see below).
            let mut variant_classes: Option<Vec<String>> = None;
            for (prop_name, prop_value) in &el.props {
                // Skip internal IR props (not valid HTML/React attributes)
                if prop_name == "_design_variant" {
                    variant_classes = design_ctx.expand_variant_classes(prop_value);
                    continue;
                }
                let jsx_attr = to_jsx_attr(prop_name);
                // Skip if this attr name will be generated by a binding (e.g., value= vs bind:value)
                if bound_attr_names.contains(&jsx_attr) {
                    continue;
                }
                match prop_value {
                    ExprIR::LiteralExpr { value } if value.is_string() => {
                        let s = value.as_str().unwrap_or_default(); // defense-in-depth fallback
                        if jsx_attr == "className" {
                            // Split multi-class names: "browserTab tabActive" → styles['browserTab'] + ' ' + styles['tabActive']
                            // Exclude class names that are handled by class_map_bindings.
                            // Design classes are global CSS in design.css and
                            // are emitted as literal class names (HIGH #1).
                            let classes: Vec<&str> = s
                                .split_whitespace()
                                .filter(|c| !binding_names.contains(c))
                                .collect();
                            if classes.is_empty() {
                                // All classes are handled by bindings — skip static className
                            } else if classes.len() > 1 {
                                let parts: Vec<String> = classes
                                    .iter()
                                    .map(|c| design_ctx.format_class_ref(c))
                                    .collect();
                                attrs.push(format!("className={{{}}}", parts.join(" + ' ' + ")));
                            } else {
                                attrs.push(format!(
                                    "className={{{}}}",
                                    design_ctx.format_class_ref(classes[0])
                                ));
                            }
                        } else {
                            // Check URL attributes for dangerous schemes (javascript:, data:, vbscript:)
                            // Mirrors the shared sanitizer's compile-time is_dangerous_url() defense-in-depth
                            // against supply-chain compromises where a third-party .omni component
                            // contains literal javascript:/data: URLs.
                            let url_attrs: &[&str] =
                                &["href", "src", "srcset", "action", "formaction", "poster"];
                            if url_attrs.contains(&jsx_attr.as_str()) && is_dangerous_url(s) {
                                attrs.push(format!("{}=\"#blocked\"", jsx_attr));
                            } else {
                                let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
                                attrs.push(format!("{}=\"{}\"", jsx_attr, escaped));
                            }
                        }
                    }
                    _ => {
                        let jsx_val =
                            format_expr_ir_tsx_renamed(prop_value, ref_names, store_field_renames);
                        attrs.push(format!("{}={{{}}}", jsx_attr, jsx_val));
                    }
                }
            }

            // Two-way bindings: value={x}, collect binding event setters for dedup with user events
            let mut binding_events_map: FxHashMap<String, String> = FxHashMap::default();
            for (bind_name, signal_ref) in &el.bindings {
                let jsx_attr = to_jsx_attr(bind_name);
                let read_name = store_field_renames
                    .get(&signal_ref.name)
                    .cloned()
                    .unwrap_or_else(|| signal_ref.name.clone());
                attrs.push(format!("{}={{{}}}", jsx_attr, read_name));
                match bind_name.as_str() {
                    "value" | "checked" => {
                        // The write-back goes through the SAME mutation
                        // machinery as action statements (C7): a store signal
                        // updates via `useStore.setState({ field: value })`,
                        // a component signal via its `setX` setter. The
                        // element value shape differs (input vs checkbox).
                        // v4 H3: 数字类型 signal 的写回必须强制转换——裸
                        // e.target.value 是字符串，会污染 number 状态
                        // （类型谎言有运行时后果：age+1 → "301"）。
                        let event_value = if bind_name == "checked" {
                            "e.target.checked".to_string()
                        } else if signal_is_numeric(comp, &signal_ref.name) {
                            "Number(e.target.value)".to_string()
                        } else {
                            "e.target.value".to_string()
                        };
                        let setter = if let Some((owner, field)) =
                            split_qualified_signal(&signal_ref.name, store_signal_owners)
                        {
                            format!("use{}.setState({{ {}: {} }})", owner, field, event_value)
                        } else {
                            format!("set{}({})", capitalize_first(&signal_ref.name), event_value)
                        };
                        binding_events_map.insert("onChange".to_string(), setter);
                    }
                    _ => {}
                }
            }

            // Motion triggers — per-element ref + JSX handler wiring. The
            // effects themselves live in motion_effects.rs; this site only:
            // 1. assigns the element's motion ref (user @ref, or synthesized
            //    `__el_N` — see motion::classify_motion_el), and
            // 2. wires @hover/@press to useCallback handlers emitted there.
            let motion_info = super::motion::classify_motion_el(el, component_map, motion_counter);
            if let Some((ref motion_ref_name, motion_idx)) = &motion_info {
                if el.ref_binding.is_none() {
                    // Synthesized refs are declared as `const __el_NRef = useRef(...)`
                    attrs.push(format!("ref={{{}Ref}}", motion_ref_name));
                }
                for trigger in el
                    .motion_behavior()
                    .map_or(&[] as &[TriggerIntent], |m| &m.triggers)
                {
                    match trigger.kind {
                        TriggerKind::Hover => {
                            attrs.push(format!("onMouseEnter={{handleEnter_{}}}", motion_idx));
                            attrs.push(format!("onMouseLeave={{handleLeave_{}}}", motion_idx));
                        }
                        TriggerKind::Press => {
                            attrs.push(format!("onPointerDown={{handleDown_{}}}", motion_idx));
                            attrs.push(format!("onPointerUp={{handleUp_{}}}", motion_idx));
                        }
                        _ => {}
                    }
                }
            }

            // Events — with dedup for binding conflicts and motion name filtering
            for (event_name, handler) in &el.events {
                let Some(react_event) = to_react_event(event_name) else {
                    // v4 3.3 (compiler HIGH-3): canonical 事件无 React 合成
                    // 事件对应（longpress/hover——Flutter 便捷形状混入
                    // canonical 表）——发射 `onLongpress` 是 React 丢弃的静默
                    // 死事件 + tsc 不识别 prop。warning + 跳过死处理器
                    // （Flutter 侧正确映射 onLongPress/onHover/onChanged）。
                    let canonical = crate::codegen::shared::event::canonical_event_name(event_name)
                        .unwrap_or(event_name);
                    warnings.lock().unwrap().push(format!(
                        "事件 '{canonical}' 在 React 目标无对应合成事件——处理器已跳过（Flutter 提供 onLongPress/onHover/onChanged 映射）"
                    ));
                    continue;
                };
                // Task 6.1b (gate FAIL-2): toggle 是 <details> 的真实 DOM 事件
                // （React 合成 onToggle 仅 details 元素存在）。details 上正常
                // 发射；其他元素与 hover/longpress 同族 warn + 跳过（html_tag
                // 为归一化后的 JSX 标签，见 to_react_tag）。
                if crate::codegen::shared::event::canonical_event_name(event_name) == Some("toggle")
                    && html_tag != "details"
                {
                    warnings.lock().unwrap().push(format!(
                        "事件 'toggle' 在 React 目标仅 <details> 元素有合成事件（onToggle）——其他元素上处理器已跳过"
                    ));
                    continue;
                }
                match handler {
                    EventHandlerIR::FnRef {
                        r#ref,
                        event_data,
                        used_fields,
                        ..
                    } => {
                        // Skip motion names — they're animation references, not functions
                        if motion_names.contains(r#ref.as_str()) {
                            continue;
                        }
                        // Task 5.6 (deferral #61): an event-data wrapper
                        // INVOKES the handler — a possibly-undefined target
                        // (signal initialized null, e.g. the `const f = null`
                        // hoisted shape) must use the optional call form
                        // `f?.(eventObj)` (TS2722). Local fns are always
                        // defined and stay bare `f(...)`.
                        let call = if possibly_undefined.contains(r#ref.as_str()) {
                            format!("{}?.", r#ref)
                        } else {
                            r#ref.clone()
                        };
                        if let Some(setter_call) = binding_events_map.remove(&react_event) {
                            // v4 M3 (task 2.4): 与无 setter 分支的空事件特判
                            // 对齐——事件对象为空（0 参 fn 无消费字段）时不
                            // 构造 `{}` 实参（TS2554）；`call` 已含 `?.`
                            // 可选调用（Task 5.6/#61），零参直调 `f?.();`
                            // 保持合法；无事件数据时按事件参数传递
                            // （setter 与 handler 共用 e）。
                            let handler_call = if event_data.is_some() {
                                let obj = emit_event_object_react(event_data, used_fields, "e");
                                if obj == "{}" {
                                    format!("{call}();")
                                } else {
                                    format!("{call}({obj});")
                                }
                            } else {
                                format!("{call}(e);")
                            };
                            attrs.push(format!(
                                "{}={{(e) => {{ {}; {} }}}}",
                                react_event, setter_call, handler_call
                            ));
                        } else if event_data.is_some() {
                            let event_obj = emit_event_object_react(event_data, used_fields, "e");
                            // When no event fields are actually needed (empty object),
                            // skip the (e) wrapper to avoid unused-param warnings.
                            if event_obj == "{}" {
                                attrs.push(format!("{}={{{}}}", react_event, r#ref));
                            } else {
                                attrs.push(format!(
                                    "{}={{(e) => {{ {}({}) }}}}",
                                    react_event, call, event_obj
                                ));
                            }
                        } else {
                            attrs.push(format!("{}={{{}}}", react_event, r#ref));
                        }
                    }
                    EventHandlerIR::InlineFn {
                        body,
                        params,
                        event_data,
                        used_fields,
                        ..
                    } => {
                        if let Some(setter_call) = binding_events_map.remove(&react_event) {
                            // Generate body with binding setter prepended
                            let mut merged_hw = CodeWriter::new();
                            merged_hw.emit_fmt(format_args!("{};", setter_call));
                            // Emit event data extraction for InlineFn with event_data
                            // (M3: skip the Object.assign when no event fields
                            // are actually needed — `Object.assign(e, {})` is
                            // pure noise; the setter below still uses `e`).
                            if event_data.is_some() {
                                let event_obj =
                                    emit_event_object_react(event_data, used_fields, "e");
                                if event_obj != "{}" {
                                    merged_hw
                                        .emit_fmt(format_args!("Object.assign(e, {});", event_obj));
                                }
                            }
                            // v4 H1 (task 2.1): 写后读线程化——改写语句列表
                            // （`x = v; use(x)` → `const nextX = v; setX(nextX);
                            // use(nextX);`），无写后读时原样返回。
                            let statements = super::expr::rewrite_write_through(&body.statements);
                            for stmt in &statements {
                                generate_statement_tsx(
                                    &mut merged_hw,
                                    stmt,
                                    signal_names,
                                    store_signal_owners,
                                    ref_names,
                                    store_field_renames,
                                    pruned_signal_names,
                                    possibly_undefined,
                                    direct_write_signals,
                                    direct_write_applies,
                                );
                            }
                            let content = merged_hw.into_string();
                            let params_str = if params.is_empty() {
                                "e".to_string()
                            } else {
                                params.join(", ")
                            };
                            let mut result = format!("({}) => {{\n", params_str);
                            for line in content.lines() {
                                let _ = writeln!(result, "        {}", line);
                            }
                            result.push_str("      }");
                            attrs.push(format!("{}={{{}}}", react_event, result));
                        } else {
                            // Non-binding InlineFn: add event extraction before body
                            if event_data.is_some() {
                                let event_obj =
                                    emit_event_object_react(event_data, used_fields, "e");
                                // M3: with no event fields needed, skip the
                                // `Object.assign(e, {})` noise AND drop the
                                // now-unused implicit event param entirely —
                                // a paramless `() => { ... }` carries no
                                // unused-param lint. A body that calls
                                // e.preventDefault() / e.stopPropagation()
                                // keeps `e` (the scan below covers that case
                                // conservatively).
                                let mut merged_hw = CodeWriter::new();
                                if event_obj != "{}" {
                                    merged_hw
                                        .emit_fmt(format_args!("Object.assign(e, {});", event_obj));
                                }
                                // v4 H1 (task 2.1): 写后读线程化。
                                let statements =
                                    super::expr::rewrite_write_through(&body.statements);
                                for stmt in &statements {
                                    generate_statement_tsx(
                                        &mut merged_hw,
                                        stmt,
                                        signal_names,
                                        store_signal_owners,
                                        ref_names,
                                        store_field_renames,
                                        pruned_signal_names,
                                        possibly_undefined,
                                        direct_write_signals,
                                        direct_write_applies,
                                    );
                                }
                                let content = merged_hw.into_string();
                                let mut body_ids = FxHashSet::default();
                                for stmt in &body.statements {
                                    super::component::collect_stmt_identifiers(stmt, &mut body_ids);
                                }
                                // With the extraction skipped, no event data
                                // is needed — emit a paramless arrow (`() =>
                                // { ... }`) instead of a dangling `(e)`, so
                                // the generated handler carries no unused
                                // param at all (the eslint config flags even
                                // `_e`). A body that calls e.preventDefault()
                                // / e.stopPropagation() still keeps `e`.
                                let params_str = if params.is_empty() {
                                    if event_obj == "{}" && !body_ids.contains("e") {
                                        String::new()
                                    } else {
                                        "e".to_string()
                                    }
                                } else {
                                    params.join(", ")
                                };
                                let mut result = format!("({}) => {{\n", params_str);
                                for line in content.lines() {
                                    let _ = writeln!(result, "        {}", line);
                                }
                                result.push_str("      }");
                                attrs.push(format!("{}={{{}}}", react_event, result));
                            } else {
                                let handler_str = format_inline_handler(
                                    body,
                                    signal_names,
                                    store_signal_owners,
                                    params,
                                    ref_names,
                                    store_field_renames,
                                    pruned_signal_names,
                                    possibly_undefined,
                                    direct_write_signals,
                                    direct_write_applies,
                                );
                                attrs.push(format!("{}={{{}}}", react_event, handler_str));
                            }
                        }
                    }
                    EventHandlerIR::StoreAction { r#ref, .. } => {
                        // Call through the action slice when the store is
                        // subscribed per field (T3.3), otherwise through the
                        // store hook object.
                        let action_ref = store_field_renames
                            .get(r#ref)
                            .cloned()
                            .unwrap_or_else(|| r#ref.clone());
                        if let Some(setter_call) = binding_events_map.remove(&react_event) {
                            attrs.push(format!(
                                "{}={{(e) => {{ {}; {}(); }}}}",
                                react_event, setter_call, action_ref
                            ));
                        } else {
                            attrs.push(format!(
                                "{react_event}={{() => {{ {action_ref}(); }}}}",
                                react_event = react_event,
                                action_ref = action_ref
                            ));
                        }
                    }
                }
            }

            // Add remaining binding events that weren't merged with user events
            for (event_name, setter_call) in &binding_events_map {
                attrs.push(format!("{}={{(e) => {{ {}; }}}}", event_name, setter_call));
            }

            // @drag draggable-intent wiring (ADR-073) — four synthesized
            // pointer props (setPointerCapture + start snapshot / DragIntent
            // move / release + optional end / release + clear). Capture is a
            // compiler-internal detail, never user-facing syntax (ADR-071
            // decision 3). A user's explicit pointer event on the same
            // element wins the colliding prop (warn + skip the drag side).
            emit_drag_react_attrs(el, &mut attrs, possibly_undefined, warnings);

            // Style classes from view — design classes emit literal names
            // (global rules in design.css), component classes stay CSS-module
            // lookups (HIGH #1).
            if let Some(ref classes) = el.style_classes {
                if let Some(classes_str) = classes.as_str() {
                    let class_parts: Vec<&str> = classes_str.split_whitespace().collect();
                    if class_parts.len() > 1 {
                        let parts: Vec<String> = class_parts
                            .iter()
                            .map(|c| design_ctx.format_class_ref(c))
                            .collect();
                        attrs.push(format!("className={{{}}}", parts.join(" + ' ' + ")));
                    } else {
                        attrs.push(format!(
                            "className={{{}}}",
                            design_ctx.format_class_ref(classes_str)
                        ));
                    }
                }
            }

            // Inline style
            if let Some(ref style_json) = el.inline_style {
                if let Some(obj) = style_json.as_object() {
                    let mut pairs: Vec<String> = Vec::new();
                    for (prop, val) in obj {
                        // Phase 1 (Task 4): 激活直写条目从 style prop 剔除
                        // ——apply 在 mutation 点、B2 在 useEffect 直写，保留
                        // render 路径会双写覆盖（无关重渲染把 style 重置回
                        // 初值）。静态条目（字面量）不在 key 集，保留。
                        if let Some(ref el_ref) = el.ref_binding {
                            if prop_fn_ctx
                                .direct_write_keys
                                .contains(&(el_ref.clone(), prop.clone()))
                            {
                                continue;
                            }
                        }
                        let js_prop = format_style_prop_key(prop);
                        let js_val = format_style_value_expr(val, ref_names);
                        pairs.push(format!("{}: {}", js_prop, js_val));
                    }
                    if !pairs.is_empty() {
                        attrs.push(format!("style={{{{ {} }}}}", pairs.join(", ")));
                    }
                }
            }

            // Dynamic class bindings: class:active={isOpen} → className using
            // CSS module syntax for component classes, literal names for
            // design classes (global rules in design.css, HIGH #1).
            if !el.class_map_bindings.is_empty() {
                let mut static_classes: Vec<String> = Vec::new();
                let mut dynamic_parts: Vec<String> = Vec::new();
                for (cls, expr) in &el.class_map_bindings {
                    match expr {
                        ExprIR::LiteralExpr { value } if value.as_bool() == Some(true) => {
                            static_classes.push(design_ctx.format_class_ref(cls));
                        }
                        ExprIR::LiteralExpr { value } if value.as_bool() == Some(false) => {
                            // skip — class not applied
                        }
                        _ => {
                            let cond =
                                format_expr_ir_tsx_renamed(expr, ref_names, store_field_renames);
                            let class_ref = design_ctx.format_class_ref(cls);
                            dynamic_parts.push(format!("${{{cond} ? {class_ref} : ''}}"));
                        }
                    }
                }
                // Merge into existing className attribute, or create a new one
                let mut new_class = String::new();
                if !static_classes.is_empty() {
                    new_class = static_classes.join(" + ' ' + ");
                }
                if !dynamic_parts.is_empty() {
                    if !new_class.is_empty() {
                        new_class =
                            format!("({}) + ' ' + `{}`", new_class, dynamic_parts.join(" "));
                    } else {
                        new_class = format!("`{}`", dynamic_parts.join(" "));
                    }
                }
                if !new_class.is_empty() {
                    if let Some(pos) = attrs.iter().position(|a| a.starts_with("className=")) {
                        // Merge: prepend the pre-existing static class name if it exists
                        let old = &attrs[pos];
                        // Extract the value from existing className={...}
                        if let Some(inner) = old.strip_prefix("className={") {
                            if let Some(existing) = inner.strip_suffix("}") {
                                // Prepend existing static class to the new_class
                                let merged = if !static_classes.is_empty() {
                                    // class_map_bindings already provided static classes
                                    format!("{existing} + ' ' + ({new_class})")
                                } else {
                                    // Static class from props + dynamic from bindings
                                    format!("{existing} + ' ' + {new_class}")
                                };
                                attrs[pos] = format!("className={{{merged}}}");
                            } else {
                                attrs[pos] = format!("className={{{new_class}}}");
                            }
                        } else {
                            attrs[pos] = format!("className={{{new_class}}}");
                        }
                    } else {
                        attrs.push(format!("className={{{}}}", new_class));
                    }
                }
            }

            // Design variant binding: `variant: LearnDesign.Button(color=primary)`
            // → literal global class names ("Button primary") matching the
            // combination rules in design.css (HIGH #1).  Merged last so a
            // `class:` on the same element keeps both.
            if let Some(variant_classes) = variant_classes {
                let joined = variant_classes.join(" ");
                append_class_attr(&mut attrs, format!("\"{}\"", joined));
            }

            // a11y block (design doc 域 10): aria-* attributes. Static
            // string entries emit as quoted attributes; expression entries
            // (hidden/alt) emit as {expr} — same static-vs-dynamic split as
            // the regular props loop above. Emitted BEFORE the placeholder
            // auto-fill so an explicit a11y label suppresses the placeholder
            // fallback (a duplicated aria-label would be invalid JSX).
            attrs.extend(a11y_react_attrs(el, ref_names, store_field_renames));

            // Ensure <button> has explicit type to prevent accidental form submission
            if html_tag == "button" && !attrs.iter().any(|a| a.starts_with("type=")) {
                attrs.push("type=\"button\"".into());
            }

            // Ensure form fields with placeholder have an accessible label
            let is_form_field = matches!(html_tag.as_str(), "input" | "textarea" | "select");
            if is_form_field
                && attrs.iter().any(|a| a.starts_with("placeholder="))
                && !attrs
                    .iter()
                    .any(|a| a.starts_with("aria-label=") || a.starts_with("aria-labelledby="))
            {
                // Extract placeholder text to use as aria-label
                if let Some(ph_attr) = attrs.iter().find(|a| a.starts_with("placeholder=")) {
                    // Static placeholder: placeholder="Search..."
                    if let Some(start) = ph_attr.find("=\"") {
                        let val_start = start + 2;
                        if let Some(end) = ph_attr[val_start..].find('"') {
                            let ph_val = &ph_attr[val_start..val_start + end];
                            attrs.push(format!("aria-label=\"{}\"", ph_val));
                        }
                    }
                }
            }

            // Ref binding — emit ref={nameRef} JSX attribute
            if let Some(ref ref_name) = el.ref_binding {
                attrs.push(format!("ref={{{}Ref}}", ref_name));
            }

            // Emit opening tag
            if attrs.is_empty() && el.children.is_empty() {
                // Self-closing tag
                w.emit_fmt(format_args!("<{} />", html_tag));
                return;
            }

            if attrs.is_empty() {
                w.emit_fmt(format_args!("<{}>", html_tag));
            } else {
                w.emit_fmt(format_args!("<{}", html_tag));
                w.indent();
                for attr in &attrs {
                    w.emit(&attr.to_string());
                }
                if el.children.is_empty() {
                    w.dedent();
                    w.emit("/>");
                    return;
                }
                w.dedent();
                w.emit(">");
            }

            // Children
            w.indent();
            for child in &el.children {
                gen_view_node_tsx(
                    w,
                    child,
                    comp,
                    component_map,
                    design_ctx,
                    signal_names,
                    store_signal_owners,
                    motion_names,
                    ref_names,
                    store_field_renames,
                    pruned_signal_names,
                    possibly_undefined,
                    direct_write_signals,
                    direct_write_applies,
                    motion_counter,
                    prop_fn_ctx,
                    warnings,
                    // element children are JSX children — expression nodes
                    // need `{...}` containers
                    true,
                );
            }
            w.dedent();

            // Closing tag
            w.emit_fmt(format_args!("</{}>", html_tag));
        }
        ViewTree::Conditional(cond) => {
            let branches = &cond.branches;
            let fallback = &cond.fallback;

            // mode: "wait" — OmniLang has no leave-animation mechanism to
            // wait for, so the transition degrades to an instant swap (same
            // timing as mode: "none"). The degradation is stated in the
            // output so it is never a silent no-op. In JSX-children position
            // the note is a `{/* */}` container comment; in a parenthesized
            // expression position (view root / branch bodies) a container
            // comment would be a syntax error, so it degrades to a `//` line
            // comment.
            if cond.transition_mode.as_deref() == Some("wait") {
                if in_jsx_children {
                    w.emit("{/* @if mode: \"wait\" — OmniLang has no leave animation; degrades to an instant swap (same as mode: \"none\"). */}");
                } else {
                    w.emit("// @if mode: \"wait\" — OmniLang has no leave animation; degrades to an instant swap (same as mode: \"none\").");
                }
            }

            // mode: "fade" — each branch renders inside a wrapper div
            // carrying the CSS-module fade class (opacity 0 → 1, defined in
            // the component's .module.css).
            let is_fade = cond.transition_mode.as_deref() == Some("fade");

            if branches.is_empty() {
                if let Some(ref fb) = fallback {
                    emit_fade_branch_open(w, is_fade, "fb");
                    gen_branch_body_tsx(
                        w,
                        fb,
                        comp,
                        component_map,
                        design_ctx,
                        signal_names,
                        store_signal_owners,
                        motion_names,
                        ref_names,
                        store_field_renames,
                        pruned_signal_names,
                        possibly_undefined,
                        direct_write_signals,
                        direct_write_applies,
                        motion_counter,
                        prop_fn_ctx,
                        warnings,
                        // fallback sits directly in the conditional's own
                        // position — a fade wrapper div puts it in JSX
                        // children, otherwise the position is inherited
                        is_fade || in_jsx_children,
                    );
                    emit_fade_branch_close(w, is_fade);
                }
                return;
            }

            // Single branch without fallback → {condition && (...)}
            if branches.len() == 1 && fallback.is_none() {
                if in_jsx_children {
                    w.emit("{");
                    w.indent();
                }
                let cond_str = format_expr_ir_tsx_renamed(
                    &branches[0].condition,
                    ref_names,
                    store_field_renames,
                );
                w.emit_fmt(format_args!("{} && (", cond_str));
                w.indent();
                emit_fade_branch_open(w, is_fade, "0");
                if let Some(ref body) = branches[0].body {
                    gen_branch_body_tsx(
                        w,
                        body,
                        comp,
                        component_map,
                        design_ctx,
                        signal_names,
                        store_signal_owners,
                        motion_names,
                        ref_names,
                        store_field_renames,
                        pruned_signal_names,
                        possibly_undefined,
                        direct_write_signals,
                        direct_write_applies,
                        motion_counter,
                        prop_fn_ctx,
                        warnings,
                        // the branch body sits inside `( ... )` — or inside
                        // the fade wrapper div, which is JSX children
                        is_fade,
                    );
                }
                emit_fade_branch_close(w, is_fade);
                w.dedent();
                w.emit(")");
                if in_jsx_children {
                    w.dedent();
                    w.emit("}");
                }
                return;
            }

            // Multiple branches or fallback → nested ternary
            // Clean structure: each branch closes `? (` before the next `: (`
            // Remaining `: (` from branches 1..N get closed after fallback/null.
            if in_jsx_children {
                w.emit("{");
                w.indent();
            }

            // branches.is_empty() returned above, so split_first() cannot
            // fail here — the nested-ternary path always has a first branch.
            let (first_branch, rest) = branches.split_first().unwrap_or_else(|| {
                unreachable!("branches is non-empty — the empty case returned above")
            });

            // First branch: `cond ? ( body )`
            let cond_str =
                format_expr_ir_tsx_renamed(&first_branch.condition, ref_names, store_field_renames);
            w.emit_fmt(format_args!("{} ? (", cond_str));
            w.indent();
            emit_fade_branch_open(w, is_fade, "0");
            if let Some(ref body) = first_branch.body {
                gen_branch_body_tsx(
                    w,
                    body,
                    comp,
                    component_map,
                    design_ctx,
                    signal_names,
                    store_signal_owners,
                    motion_names,
                    ref_names,
                    store_field_renames,
                    pruned_signal_names,
                    possibly_undefined,
                    direct_write_signals,
                    direct_write_applies,
                    motion_counter,
                    prop_fn_ctx,
                    warnings,
                    is_fade,
                );
            }
            emit_fade_branch_close(w, is_fade);
            w.dedent();

            // Remaining branches: `) : ( cond ? ( body )`
            for (i, branch) in rest.iter().enumerate() {
                w.emit(") : (");
                w.indent();
                let cond_str =
                    format_expr_ir_tsx_renamed(&branch.condition, ref_names, store_field_renames);
                w.emit_fmt(format_args!("{} ? (", cond_str));
                w.indent();
                emit_fade_branch_open(w, is_fade, &(i + 1).to_string());
                if let Some(ref body) = branch.body {
                    gen_branch_body_tsx(
                        w,
                        body,
                        comp,
                        component_map,
                        design_ctx,
                        signal_names,
                        store_signal_owners,
                        motion_names,
                        ref_names,
                        store_field_renames,
                        pruned_signal_names,
                        possibly_undefined,
                        direct_write_signals,
                        direct_write_applies,
                        motion_counter,
                        prop_fn_ctx,
                        warnings,
                        is_fade,
                    );
                }
                emit_fade_branch_close(w, is_fade);
                w.dedent();
            }

            // Fallback or null
            if let Some(ref fb) = fallback {
                w.emit(") : (");
                w.indent();
                emit_fade_branch_open(w, is_fade, "fb");
                gen_branch_body_tsx(
                    w,
                    fb,
                    comp,
                    component_map,
                    design_ctx,
                    signal_names,
                    store_signal_owners,
                    motion_names,
                    ref_names,
                    store_field_renames,
                    pruned_signal_names,
                    possibly_undefined,
                    direct_write_signals,
                    direct_write_applies,
                    motion_counter,
                    prop_fn_ctx,
                    warnings,
                    is_fade,
                );
                emit_fade_branch_close(w, is_fade);
                w.dedent();
                w.emit(")");
            } else {
                w.emit(") : null");
            }
            // Close each `: (` from rest branches
            for _ in 0..rest.len() {
                w.emit(")");
            }

            if in_jsx_children {
                w.dedent();
                w.emit("}");
            }
        }
        ViewTree::ListRendering(list) => {
            let iterable_str =
                format_expr_ir_tsx_renamed(&list.iterable, ref_names, store_field_renames);
            let item_name = &list.item_binding.name;
            let key_attr = if let Some(ref ke) = list.key_expr {
                format!(
                    " key={{{}}}",
                    format_expr_ir_tsx_renamed(ke, ref_names, store_field_renames)
                )
            } else {
                // Index-key fallback (Task 5.5): without key={expr} React
                // reconciles by index anyway — make it explicit and leave a
                // generated warning in the output (the semantic layer warns
                // for OBJECT items — design doc 域 6; the index fallback is
                // the codegen safety net). NEVER String(item) — that renders
                // "[object Object]" for object items and silently collapses
                // keyed reconciliation.
                " key={_i} /* omni: no key={expr} on @for — index-key fallback; reorder/filter loses item state — add key={...} for object items */".to_string()
            };
            if list.empty_body.is_some() {
                if in_jsx_children {
                    w.emit("{");
                    w.indent();
                }
                // Single-eval the iterable (Task 5.5): the previous shape
                // `({} && {}.length === 0) ? ... : ({}.map(...))` evaluated
                // the iterable TWICE per render (side effects / allocations
                // ran twice). Hoist to a const inside an IIFE so the list
                // expression runs exactly once.
                w.emit_fmt(format_args!(
                    "(() => {{ const __omni_iterable = {}; return (__omni_iterable && __omni_iterable.length === 0) ? (",
                    iterable_str
                ));
                w.indent();
                if let Some(ref empty_body) = list.empty_body {
                    gen_branch_body_tsx(
                        w,
                        empty_body,
                        comp,
                        component_map,
                        design_ctx,
                        signal_names,
                        store_signal_owners,
                        motion_names,
                        ref_names,
                        store_field_renames,
                        pruned_signal_names,
                        possibly_undefined,
                        direct_write_signals,
                        direct_write_applies,
                        motion_counter,
                        prop_fn_ctx,
                        warnings,
                        // the empty body sits inside the IIFE's return
                        // parens — never a JSX-children position
                        false,
                    );
                }
                w.dedent();
                w.emit_fmt(format_args!(
                    ") : (__omni_iterable.map(({}, _i) => (",
                    item_name
                ));
            } else {
                if in_jsx_children {
                    w.emit("{");
                    w.indent();
                }
                w.emit_fmt(format_args!(
                    "{}.map(({}, _i) => (",
                    iterable_str, item_name
                ));
            }
            w.indent();
            w.emit_fmt(format_args!("<React.Fragment{}>", key_attr));
            w.indent();
            if let Some(ref body) = list.body {
                gen_view_node_tsx(
                    w,
                    body,
                    comp,
                    component_map,
                    design_ctx,
                    signal_names,
                    store_signal_owners,
                    motion_names,
                    ref_names,
                    store_field_renames,
                    pruned_signal_names,
                    possibly_undefined,
                    direct_write_signals,
                    direct_write_applies,
                    motion_counter,
                    prop_fn_ctx,
                    warnings,
                    // the map callback parens wrap the whole
                    // `<React.Fragment>...</React.Fragment>` element — the
                    // body renders BETWEEN the fragment tags, which is
                    // JSX-children position: a nested conditional must keep
                    // its `{...}` container (a bare `cond && (<div/>)` there
                    // renders as literal text)
                    true,
                );
            }
            w.dedent();
            w.emit("</React.Fragment>");
            w.dedent();
            w.emit("))");
            w.dedent();
            if list.empty_body.is_some() {
                // The preceding `))` closed the lambda-body and map-call
                // parens; here the ternary's else paren closes (`)`), the
                // return statement ends (`;`), the arrow body and the IIFE
                // call paren close (`})`), and the IIFE is invoked (`()`).
                // In JSX-children position a final `}` closes the container
                // (no stray `;` — JSX expression containers hold exactly
                // one expression); at the view root / in parens it is a
                // syntax error and must not be emitted.
                if in_jsx_children {
                    w.emit("); })()}");
                } else {
                    w.emit("); })()");
                }
            } else if in_jsx_children {
                w.emit("}");
            }
        }
        ViewTree::Text(text) => {
            // Skip empty/whitespace-only text nodes. Punctuation-only text
            // (e.g. `#` in `<li>#{item.id} — {item.name}</li>`) is LEGITIMATE
            // source content and must render — the OmniView target keeps it,
            // and the punctuation clause here dropped it on React only
            // (task 2.1 verification: test 26 list rendering). The `{`-leading
            // injection guard below is unaffected.
            let text_trimmed = text.trim();
            if text_trimmed.is_empty() {
                return;
            }
            // NOTE: `{`-leading text is NOT special-cased — legit
            // interpolations arrive as ViewTree::Interpolation nodes; a
            // `{`-shaped TEXT node only survives parsing via the parser's
            // malformed-interpolation fallback (parser/view/mod.rs), and
            // treating it as code was a live injection vector
            // (`{"", alert(1), ""}` → `{..., alert(1), ...}` comma
            // operator, Task 3.1 review CRITICAL round 2). Such text is
            // wrapped in a JS string literal and rendered literally.
            //
            // Escape separation (Task 2.4): the `{'...'}` wrapper is a JS
            // STRING LITERAL in an expression container — React renders its
            // content verbatim (no HTML entity decoding), so `<`, `>`, `&`
            // must stay RAW: entity-escaping them produced visible `&amp;amp;`
            // text (ts/react M1) and double-escaped the demos' titles. Only
            // JS-level escaping (backslash / quote / control chars) applies
            // here; HTML entity escaping belongs on the JSXText/HTML paths
            // (OmniView escape_html), which React's runtime text escaping
            // makes redundant in expression containers anyway.
            let escaped = escape_literal_str(text, '\'');
            w.emit_fmt(format_args!("{{'{}'}}", escaped));
        }
        ViewTree::Interpolation(expr) => {
            let js_expr = format_expr_ir_tsx_renamed(expr, ref_names, store_field_renames);
            w.emit_fmt(format_args!("{{{}}}", js_expr));
        }
        ViewTree::Fragment(children) => {
            // Transparent node — the children take the fragment's own
            // position (a `<>...</>` boundary is only added by
            // gen_branch_body_tsx, which passes the flag itself).
            for child in children {
                gen_view_node_tsx(
                    w,
                    child,
                    comp,
                    component_map,
                    design_ctx,
                    signal_names,
                    store_signal_owners,
                    motion_names,
                    ref_names,
                    store_field_renames,
                    pruned_signal_names,
                    possibly_undefined,
                    direct_write_signals,
                    direct_write_applies,
                    motion_counter,
                    prop_fn_ctx,
                    warnings,
                    in_jsx_children,
                );
            }
        }
        ViewTree::Slot { name, fallback, .. } => {
            // Named slots map to same-named props; the unnamed slot maps to
            // `children`. When the parent provides no content for the slot,
            // the preserved fallback (Task 1.2) is rendered instead:
            //   {header ?? (fallback JSX)}
            // Multi-node fallbacks are wrapped in a fragment by
            // gen_branch_body_tsx so the emitted TSX stays valid.
            let prop_name = name.as_deref().unwrap_or("children");
            if fallback.is_empty() {
                w.emit_fmt(format_args!("{{{}}}", prop_name));
            } else {
                w.emit_fmt(format_args!("{{{} ?? (", prop_name));
                w.indent();
                gen_branch_body_tsx(
                    w,
                    &ViewTree::Fragment(fallback.clone()),
                    comp,
                    component_map,
                    design_ctx,
                    signal_names,
                    store_signal_owners,
                    motion_names,
                    ref_names,
                    store_field_renames,
                    pruned_signal_names,
                    possibly_undefined,
                    direct_write_signals,
                    direct_write_applies,
                    motion_counter,
                    prop_fn_ctx,
                    warnings,
                    // the fallback sits inside the `( ... )` of
                    // `{name ?? ( ... )}` — never a JSX-children position
                    false,
                );
                w.dedent();
                w.emit(")}");
            }
        }
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}

/// Emit a conditional/ternary branch body (also used for `@for` empty
/// states). A body with multiple top-level nodes is wrapped in a React
/// fragment (`<>...</>`) so the adjacent JSX children stay valid inside the
/// parenthesized `(body)` expression — e.g. an `@if` body containing a slot
/// plus a sibling element.
///
/// Also used for the component root view (`gen_jsx_return`) so that
/// multi-element roots (e.g. a view with an input + buttons as siblings)
/// emit valid JSX instead of adjacent root expressions.
#[allow(clippy::too_many_arguments)]
pub(super) fn gen_branch_body_tsx(
    w: &mut CodeWriter,
    body: &ViewTree,
    comp: &ComponentIR,
    component_map: &FxHashMap<&str, &ComponentIR>,
    design_ctx: &DesignCtx,
    signal_names: &FxHashSet<String>,
    store_signal_owners: &FxHashMap<String, String>,
    motion_names: &FxHashSet<String>,
    ref_names: &FxHashSet<String>,
    store_field_renames: &FxHashMap<String, String>,
    pruned_signal_names: &FxHashSet<String>,
    possibly_undefined: &FxHashSet<String>,
    direct_write_signals: &FxHashSet<String>,
    direct_write_applies: &FxHashMap<String, Vec<String>>,
    motion_counter: &mut usize,
    prop_fn_ctx: &PropFnMemoCtx,
    warnings: &Mutex<Vec<String>>,
    // Whether this body sits in JSX-children position (see
    // `gen_view_node_tsx`). A multi-node body is wrapped in a `<>...</>`
    // fragment, which puts its children in JSX-children position
    // regardless of the incoming flag.
    in_jsx_children: bool,
) {
    let is_multi = match body {
        // Lowering wraps multi-node bodies in an element with tag "fragment";
        // ViewTree::Fragment is the inline form.
        ViewTree::Fragment(children) => children.len() > 1,
        ViewTree::Element(el) if el.tag == "fragment" => el.children.len() > 1,
        _ => false,
    };
    if is_multi {
        w.emit("<>");
        w.indent();
    }
    gen_view_node_tsx(
        w,
        body,
        comp,
        component_map,
        design_ctx,
        signal_names,
        store_signal_owners,
        motion_names,
        ref_names,
        store_field_renames,
        pruned_signal_names,
        possibly_undefined,
        direct_write_signals,
        direct_write_applies,
        motion_counter,
        prop_fn_ctx,
        warnings,
        is_multi || in_jsx_children,
    );
    if is_multi {
        w.dedent();
        w.emit("</>");
    }
}

/// Open/close the `mode: "fade"` wrapper div around a conditional branch.
///
/// `key` identifies the branch inside the conditional (`"0"`, `"1"`, ...;
/// `"fb"` for the fallback). The key is REQUIRED for replay: React
/// reconciliation reuses same-position DOM nodes, so a ternary swap
/// (`@if/@else`) would otherwise keep the old wrapper node — animation
/// already finished, opacity stuck at 1 — and the fade would never replay.
/// The key forces a remount, so the CSS animation replays exactly when the
/// shown branch changes. The class and its keyframes are defined in the
/// component's `.module.css` (see `component::gen_component_css`) —
/// same-module definition + reference so the bundler scopes both
/// consistently.
fn emit_fade_branch_open(w: &mut CodeWriter, is_fade: bool, key: &str) {
    if is_fade {
        w.emit_fmt(format_args!(
            "<div key=\"{}\" className={{styles['omni-if-fade']}}>",
            key
        ));
        w.indent();
    }
}

fn emit_fade_branch_close(w: &mut CodeWriter, is_fade: bool) {
    if is_fade {
        w.dedent();
        w.emit("</div>");
    }
}

/// If the view tree is a consumer-side slot projection, return the slot
/// name. Two forms are recognized:
/// - an element with a literal `slot="name"` prop (mirrors the OmniView
///   backend's `has_slot_attr`);
/// - a consumer-position `@slot name { ... }` directive, whose fallback
///   content is the projected content.
fn child_slot_name(node: &ViewTree) -> Option<String> {
    match node {
        ViewTree::Element(el) => el.props.get("slot").and_then(|v| match v {
            ExprIR::LiteralExpr { value } => value.as_str().map(str::to_string),
            _ => None,
        }),
        ViewTree::Slot { name, .. } => name.clone(),
        _ => None,
    }
}

/// Task 2.8 (perf M4) codegen context — built by
/// `component::collect_prop_fn_memo_ctx` (per component file) and threaded
/// through the JSX emission so component instantiations can use the
/// useCallback wrappers (`__omni_cb_<fn>`) and the `Memoized<Child>`
/// aliases.
pub(super) struct PropFnMemoCtx {
    /// Component-local fns passed BY VALUE to child component instances
    /// (bare identifier props / bare FnRef events). Each emits
    /// `const __omni_cb_<name> = useCallback(<name>, [reads]);` — the fn
    /// const is recreated every render, and a memoized child would otherwise
    /// see a fresh fn identity on every render (memo never hits).
    pub wrapped_fns: FxHashSet<String>,
    /// Child component names with a module-scope `const Memoized<Name> =
    /// React.memo(<Name>);` alias — the child received at least one
    /// memoizable (children-free, stable-props) instance.
    pub memo_children: FxHashSet<String>,
    /// Identifier names whose identity is render-stable: signals, computeds,
    /// props, store aliases/fields, slice locals, wrapped-fn wrapper names,
    /// and lib fns. The memo instance check accepts only these plus scalar
    /// literals.
    pub stable_idents: FxHashSet<String>,
    /// Library fn names — module-scope consts, stable by construction (the
    /// event side of the memo check accepts these without a wrapper).
    pub lib_fns: FxHashSet<String>,
    /// Phase 1 (Task 4): activated direct-write `(ref_name, style_prop)`
    /// pairs — every activated style entry is driven by the direct-write
    /// channel (D1 apply at mutation sites / B2 useEffect), never by the
    /// render path; the JSX style prop must skip these pairs (a stale
    /// re-apply on an unrelated re-render would override the live DOM
    /// write — the double-path race). Static entries (literals) are not in
    /// the set and stay in the style prop. Built by the component emitter
    /// and threaded here through the shared JSX context (mod.rs → jsx.rs →
    /// view.rs).
    pub direct_write_keys: FxHashSet<(String, String)>,
}

/// Task 2.8: whether a child component instance is a React.memo candidate —
/// it receives NO children/slot projections, and every prop/event value is
/// a render-stable form (scalar literal / stable identifier /
/// useCallback-wrapped fn). Fresh arrow handlers (event_data extraction,
/// inline fns, store-action wrappers) and dynamic a11y expressions
/// disqualify the instance — memo's shallow prop compare would never hit
/// and the wrap would be pure overhead.
///
/// Conservative by design: a false negative only costs a missed memo hit —
/// there is no correctness consequence (memo never skips a render whose
/// props are unequal; a memoized child with internal store subscriptions is
/// unaffected — zustand's useSyncExternalStore forceUpdate bypasses memo).
pub(super) fn component_instance_memoizable(
    el: &ElementIR,
    has_projected_children: bool,
    ctx: &PropFnMemoCtx,
) -> bool {
    if has_projected_children {
        // children / named-slot props are a fresh element tree every render.
        return false;
    }
    for prop_value in el.props.values() {
        match prop_value {
            ExprIR::LiteralExpr { value } => {
                // Scalar literals (string/number/bool/null) are stable;
                // object/array literals are fresh per render.
                if value.is_object() || value.is_array() {
                    return false;
                }
            }
            ExprIR::IdentifierExpr { name, .. } => {
                if !ctx.stable_idents.contains(name) {
                    return false;
                }
            }
            _ => return false,
        }
    }
    for handler in el.events.values() {
        match handler {
            EventHandlerIR::FnRef {
                r#ref, event_data, ..
            } => {
                if event_data.is_some() {
                    return false; // fresh `(e) => { fn(e) }` arrow
                }
                if !(ctx.wrapped_fns.contains(r#ref) || ctx.lib_fns.contains(r#ref)) {
                    return false;
                }
            }
            // InlineFn / StoreAction — a fresh arrow each render.
            _ => return false,
        }
    }
    if let Some(a11y) = &el.a11y {
        // Static-string a11y entries are stable; dynamic entries (hidden,
        // non-literal alt) are fresh expressions per render.
        if a11y.hidden.is_some() {
            return false;
        }
        if let Some(alt) = &a11y.alt {
            if !matches!(alt, ExprIR::LiteralExpr { .. }) {
                return false;
            }
        }
    }
    true
}

/// Partition a component instantiation's children into named slot
/// projections (elements carrying a literal `slot="name"` matching a
/// declared slot, or consumer-position `@slot` projections) and default
/// children. Shared by the JSX emitter and the task 2.8 memo analysis so
/// the "receives children?" decision can never diverge between the two.
pub(super) fn partition_component_children<'a>(
    el: &'a ElementIR,
    child_comp: &ComponentIR,
    component_map: &FxHashMap<&str, &ComponentIR>,
) -> (Vec<&'a ViewTree>, Vec<(String, &'a ViewTree)>) {
    let mut declared_slot_names: FxHashSet<String> = FxHashSet::default();
    for n in component_declared_slots(child_comp, component_map)
        .into_iter()
        .flatten()
    {
        declared_slot_names.insert(n);
    }
    let mut default_children: Vec<&ViewTree> = Vec::new();
    let mut named_children: Vec<(String, &ViewTree)> = Vec::new();
    for child in &el.children {
        match child_slot_name(child) {
            Some(slot_name) if declared_slot_names.contains(&slot_name) => {
                named_children.push((slot_name, child));
            }
            _ => default_children.push(child),
        }
    }
    (default_children, named_children)
}

/// Render a child of a component instantiation (consumer-side projection
/// content). Consumer-position `@slot name { ... }` directives render their
/// fallback — that IS the projected content — instead of the slot placeholder.
/// Elements get the `slot` attribute stripped so it never reaches the DOM.
#[allow(clippy::too_many_arguments)]
fn gen_consumer_child_tsx(
    w: &mut CodeWriter,
    child: &ViewTree,
    comp: &ComponentIR,
    component_map: &FxHashMap<&str, &ComponentIR>,
    design_ctx: &DesignCtx,
    signal_names: &FxHashSet<String>,
    store_signal_owners: &FxHashMap<String, String>,
    motion_names: &FxHashSet<String>,
    ref_names: &FxHashSet<String>,
    store_field_renames: &FxHashMap<String, String>,
    pruned_signal_names: &FxHashSet<String>,
    possibly_undefined: &FxHashSet<String>,
    direct_write_signals: &FxHashSet<String>,
    direct_write_applies: &FxHashMap<String, Vec<String>>,
    motion_counter: &mut usize,
    prop_fn_ctx: &PropFnMemoCtx,
    warnings: &Mutex<Vec<String>>,
    // Whether this projected content sits in JSX-children position:
    // default children render between `<Comp>`/`</Comp>` (true), named
    // slot projections render inside an attribute expression
    // `slotName={...}` (false — see the `gen_consumer_child_tsx` call
    // sites in the component arm).
    in_jsx_children: bool,
) {
    match child {
        ViewTree::Slot { fallback, .. } => {
            gen_branch_body_tsx(
                w,
                &ViewTree::Fragment(fallback.clone()),
                comp,
                component_map,
                design_ctx,
                signal_names,
                store_signal_owners,
                motion_names,
                ref_names,
                store_field_renames,
                pruned_signal_names,
                possibly_undefined,
                direct_write_signals,
                direct_write_applies,
                motion_counter,
                prop_fn_ctx,
                warnings,
                in_jsx_children,
            );
        }
        _ => {
            gen_view_node_tsx(
                w,
                &strip_slot_prop(child),
                comp,
                component_map,
                design_ctx,
                signal_names,
                store_signal_owners,
                motion_names,
                ref_names,
                store_field_renames,
                pruned_signal_names,
                possibly_undefined,
                direct_write_signals,
                direct_write_applies,
                motion_counter,
                prop_fn_ctx,
                warnings,
                in_jsx_children,
            );
        }
    }
}

/// Remove the `slot` prop from an element so it never reaches the DOM as a
/// web-component attribute after projection into a React named prop.
fn strip_slot_prop(node: &ViewTree) -> ViewTree {
    match node {
        ViewTree::Element(el) if el.props.contains_key("slot") => {
            let mut props = el.props.clone();
            props.remove("slot");
            ViewTree::Element(Box::new(ElementIR {
                props,
                ..*el.clone()
            }))
        }
        other => other.clone(),
    }
}

/// Build an inline arrow function string from a BlockIR.
/// Uses the declared params from the Omni source (e.g., `(text)` or `(e)`) instead of hardcoding `e`,
/// so that the handler body which references the parameter by name works correctly.
/// Unused params are prefixed with `_` to suppress ESLint no-unused-vars warnings.
// 8 cohesive context params (all namespaces consulted while emitting a
// handler); bundling them into a context struct is a larger refactor than
// this lint warrants for an internal helper.
#[allow(clippy::too_many_arguments)]
fn format_inline_handler(
    body: &BlockIR,
    signal_names: &FxHashSet<String>,
    store_signal_owners: &FxHashMap<String, String>,
    params: &[String],
    ref_names: &FxHashSet<String>,
    store_field_renames: &FxHashMap<String, String>,
    pruned_signal_names: &FxHashSet<String>,
    possibly_undefined: &FxHashSet<String>,
    direct_write_signals: &FxHashSet<String>,
    direct_write_applies: &FxHashMap<String, Vec<String>>,
) -> String {
    // Generate the body content first to check which params are actually used
    // (v4 H1, task 2.1: 写后读线程化——改写语句列表，无写后读时原样返回).
    let mut body_w = CodeWriter::new();
    let statements = super::expr::rewrite_write_through(&body.statements);
    for stmt in &statements {
        generate_statement_tsx(
            &mut body_w,
            stmt,
            signal_names,
            store_signal_owners,
            ref_names,
            store_field_renames,
            pruned_signal_names,
            possibly_undefined,
            direct_write_signals,
            direct_write_applies,
        );
    }
    let body_content = body_w.into_string();

    // Check if params are used in the body; prefix unused ones with _
    let effective_params: Vec<String> = params
        .iter()
        .map(|p| {
            if body_content.contains(p.as_str()) {
                p.clone()
            } else {
                format!("_{}", p)
            }
        })
        .collect();

    let param_str = if effective_params.is_empty() {
        String::new()
    } else {
        effective_params.join(", ")
    };
    let config = InlineHandlerConfig {
        opener: if param_str.is_empty() {
            "() => {\n".to_string()
        } else {
            format!("({}) => {{\n", param_str)
        },
        closer: "\n      }".to_string(),
        empty_handler: "() => {}".to_string(),
        indent: "        ",
        use_writer: false,
    };
    build_inline_handler(body, &config, |w, s| {
        generate_statement_tsx(
            w,
            s,
            signal_names,
            store_signal_owners,
            ref_names,
            store_field_renames,
            pruned_signal_names,
            possibly_undefined,
            direct_write_signals,
            direct_write_applies,
        )
    })
}

/// Emit the `a11y: { ... }` block (design doc 域 10) as JSX attributes.
///
/// Mapping (design doc mapping table):
/// - `label` → `aria-label="..."` (static — label is a string literal)
/// - `role`  → `role="..."` (static — role is a string literal)
/// - `hidden`→ `aria-hidden={expr}` (dynamic expression)
/// - `live`  → `aria-live="polite"` (static — polite/assertive literal)
/// - `alt`   → `alt="..."` when literal, `alt={expr}` when dynamic
///
/// String literals are escaped exactly like the static props loop so a
/// quote/backslash payload can never break out of the attribute.
fn a11y_react_attrs(
    el: &ElementIR,
    ref_names: &FxHashSet<String>,
    store_field_renames: &FxHashMap<String, String>,
) -> Vec<String> {
    let Some(ref a11y) = el.a11y else {
        return Vec::new();
    };
    let mut attrs: Vec<String> = Vec::new();
    if let Some(ref label) = a11y.label {
        let escaped = label.replace('\\', "\\\\").replace('"', "\\\"");
        attrs.push(format!("aria-label=\"{}\"", escaped));
    }
    if let Some(ref role) = a11y.role {
        let escaped = role.replace('\\', "\\\\").replace('"', "\\\"");
        attrs.push(format!("role=\"{}\"", escaped));
    }
    if let Some(ref hidden) = a11y.hidden {
        let js_val = format_expr_ir_tsx_renamed(hidden, ref_names, store_field_renames);
        attrs.push(format!("aria-hidden={{{}}}", js_val));
    }
    if let Some(ref live) = a11y.live {
        let escaped = live.replace('\\', "\\\\").replace('"', "\\\"");
        attrs.push(format!("aria-live=\"{}\"", escaped));
    }
    if let Some(ref alt) = a11y.alt {
        match alt {
            ExprIR::LiteralExpr { value } if value.is_string() => {
                let s = value.as_str().unwrap_or_default();
                let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
                attrs.push(format!("alt=\"{}\"", escaped));
            }
            _ => {
                let js_val = format_expr_ir_tsx_renamed(alt, ref_names, store_field_renames);
                attrs.push(format!("alt={{{}}}", js_val));
            }
        }
    }
    attrs
}

/// Append a static class expression to the element's `className` attribute,
/// merging into the existing attribute when present and creating a new one
/// otherwise (used for design variant combination classes).
fn append_class_attr(attrs: &mut Vec<String>, class_expr: String) {
    if let Some(pos) = attrs.iter().position(|a| a.starts_with("className=")) {
        let old = attrs[pos].clone();
        if let Some(inner) = old.strip_prefix("className={") {
            if let Some(existing) = inner.strip_suffix('}') {
                attrs[pos] = format!("className={{{} + ' ' + {}}}", existing, class_expr);
                return;
            }
        }
        attrs[pos] = format!("className={{{}}}", class_expr);
    } else {
        attrs.push(format!("className={{{}}}", class_expr));
    }
}

// ══════════════════════════════════════════════════════
// @drag draggable-intent wiring (ADR-073)
// ══════════════════════════════════════════════════════

/// Emit the four JSX pointer props that implement `@drag` (ADR-073): drag
/// is a compiler-synthesized gesture over native Pointer Events, not a DOM
/// event, so the capture lifecycle is emitted per-ref-element (ADR-071
/// decision 3 — capture is a compiler-internal detail, never user-facing
/// syntax):
///   onPointerDown    → setPointerCapture + snapshot the start onto the
///                      `{ref}_dragStartRef` useRef cell (React state
///                      space — component functions have no DOM access)
///   onPointerMove    → while dragging, call `on_drag` with a synthesized
///                      DragIntent object
///                      `{ x, y, dx, dy, pointerId, startX, startY }`
///   onPointerUp      → release + clear + (optional) `on_drag_end` with the
///                      same DragIntent shape read from a start snapshot
///   onPointerCancel  → release + clear
///
/// A user's explicit pointer event on the same element wins the colliding
/// prop — the drag side is skipped with a warning (a duplicated JSX prop
/// would be a tsc error TS17001 and a silent React drop); the other three
/// drag props stay wired.
fn emit_drag_react_attrs(
    el: &ElementIR,
    attrs: &mut Vec<String>,
    possibly_undefined: &FxHashSet<String>,
    warnings: &Mutex<Vec<String>>,
) {
    let Some(drag) = &el.drag_binding else {
        return;
    };
    let Some(ref ref_name) = &el.ref_binding else {
        // Defensive: semantic/validate_drag hard-errors @drag without @ref
        // (the start-snapshot cell keys off the ref name).
        warnings
            .lock()
            .unwrap()
            .push("[react] @drag 需要同元素 @ref——已跳过 @drag 接线".into());
        return;
    };
    let EventHandlerIR::FnRef {
        r#ref: move_fn,
        used_fields,
        ..
    } = &drag.on_drag
    else {
        // Unreachable: lowering always produces FnRef for drag handlers
        // (validate_drag enforces bare function names).
        warnings
            .lock()
            .unwrap()
            .push("[react] @drag 处理器必须是函数名（FnRef），已跳过".into());
        return;
    };
    // Task 5.6 (deferral #61) parity: a possibly-undefined target (event
    // prop / null-initialized signal) uses the optional call form; local
    // fns are always defined and stay bare `f(...)`.
    let call = |name: &str| {
        if possibly_undefined.contains(name) {
            format!("{}?.", name)
        } else {
            name.to_string()
        }
    };
    let move_call = call(move_fn);
    let state_ref = format!("{}_dragStartRef", ref_name);
    let state_expr = format!("{}.current", state_ref);

    let mut props: Vec<(String, String)> = Vec::new();
    // pointerdown: capture the pointer + snapshot the start point
    props.push((
        "onPointerDown".into(),
        format!(
            "e.currentTarget.setPointerCapture(e.pointerId); {state_expr} = {{ x: e.clientX, y: e.clientY }};"
        ),
    ));
    // pointermove: drag-guarded synthesized DragIntent call (dx/dy computed
    // against the start snapshot)
    let move_obj = drag_event_object_react(used_fields, &state_expr);
    let move_body = if move_obj == "{}" {
        format!("if ({state_expr}) {move_call}();")
    } else {
        format!("if ({state_expr}) {move_call}({move_obj});")
    };
    props.push(("onPointerMove".into(), move_body));
    // pointerup: snapshot the start BEFORE clearing, so the end handler
    // reads the same DragIntent shape from the snapshot var `s`
    let end_js = match &drag.on_drag_end {
        Some(EventHandlerIR::FnRef {
            r#ref: end_fn,
            used_fields: end_fields,
            ..
        }) => {
            let end_call = call(end_fn);
            let end_obj = drag_event_object_react(end_fields, "s");
            if end_obj == "{}" {
                format!(" if (s) {end_call}();")
            } else {
                format!(" if (s) {end_call}({end_obj});")
            }
        }
        Some(_) => {
            warnings
                .lock()
                .unwrap()
                .push("[react] @dragEnd 处理器必须是函数名（FnRef），已跳过".into());
            String::new()
        }
        None => String::new(),
    };
    props.push((
        "onPointerUp".into(),
        format!(
            "e.currentTarget.releasePointerCapture?.(e.pointerId); const s = {state_expr}; {state_expr} = null;{end_js}"
        ),
    ));
    // pointercancel: release + clear only
    props.push((
        "onPointerCancel".into(),
        format!("e.currentTarget.releasePointerCapture?.(e.pointerId); {state_expr} = null;"),
    ));

    for (prop, body) in props {
        if attrs.iter().any(|a| a.starts_with(&format!("{prop}="))) {
            warnings.lock().unwrap().push(format!(
                "元素 @drag 与显式 {prop} 事件共存——{prop} 以显式事件为准，@drag 的 {prop} 接线已跳过"
            ));
            continue;
        }
        attrs.push(format!("{prop}={{(e) => {{ {body} }}}}"));
    }
}

/// Build the DragIntent event object literal for a drag handler call.
///
/// DragIntent fields are synthesized (start snapshot + pointer delta), not
/// raw event passthrough — the shared `emit_event_object_react` cannot
/// express them (its DragIntent mapping falls through to `undefined`), so
/// the object is built here, honoring `used_fields` with the same
/// filter-in-schema-order + `?? 0` fallback semantics. `state` names the
/// drag-start snapshot: `{ref}_dragStartRef.current` for pointermove, `s`
/// for the pointerup snapshot.
fn drag_event_object_react(used_fields: &Option<Vec<String>>, state: &str) -> String {
    let all: Vec<(String, String)> = vec![
        ("x".into(), "e.clientX".into()),
        ("y".into(), "e.clientY".into()),
        ("dx".into(), format!("e.clientX - {state}.x")),
        ("dy".into(), format!("e.clientY - {state}.y")),
        ("pointerId".into(), "e.pointerId".into()),
        ("startX".into(), format!("{state}.x")),
        ("startY".into(), format!("{state}.y")),
    ];
    let fields: Vec<String> = match used_fields {
        Some(used) if !used.is_empty() => all
            .into_iter()
            .filter(|(name, _)| used.contains(name))
            .map(|(name, extract)| format!("{name}: {extract} ?? 0"))
            .collect(),
        Some(_) => return "{}".to_string(),
        None => all
            .into_iter()
            .map(|(name, extract)| format!("{name}: {extract} ?? 0"))
            .collect(),
    };
    format!("{{ {} }}", fields.join(", "))
}
