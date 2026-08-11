// ═══════════════════════════════════════════════════════════
// View tree generation (JSX)
// ═══════════════════════════════════════════════════════════

use crate::codegen::escape_literal_str;
use crate::codegen::shared::inline_handler::{build_inline_handler, InlineHandlerConfig};
use crate::codegen::shared::style::{format_style_prop_key, format_style_value_expr};
use crate::codegen::try_format_stdlib_call;
use crate::codegen::CodeWriter;
use crate::codegen::Target;
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

// ── @modal title id (spec §2.1 rule 5) ────────────────────────────────
// With a Heading present the compiler generates `id="omni-modal-title"` on
// the first Heading element and points `aria-labelledby` at it; without a
// Heading the attribute is omitted entirely (semantic V13 already warned).
// The tag set mirrors validate_feedback_modal.rs's V13 collection.
const MODAL_HEADING_TAGS: &[&str] = &["Heading", "Title", "h1", "h2", "h3", "h4", "h5", "h6"];

/// Whether a subtree (slot fallbacks included) contains a heading element.
fn subtree_has_heading(node: &ViewTree) -> bool {
    match node {
        ViewTree::Element(el) => {
            MODAL_HEADING_TAGS.contains(&el.tag.as_str())
                || el.children.iter().any(subtree_has_heading)
        }
        ViewTree::Slot { fallback, .. } => fallback.iter().any(subtree_has_heading),
        ViewTree::Conditional(cond) => {
            cond.branches
                .iter()
                .any(|b| b.body.as_ref().is_some_and(subtree_has_heading))
                || cond.fallback.as_ref().is_some_and(subtree_has_heading)
        }
        ViewTree::ListRendering(list) => {
            list.body.as_ref().is_some_and(subtree_has_heading)
                || list.empty_body.as_ref().is_some_and(subtree_has_heading)
        }
        ViewTree::Fragment(children) => children.iter().any(subtree_has_heading),
        ViewTree::Text(_) | ViewTree::Interpolation(_) => false,
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
    }
}

/// Inject `id="omni-modal-title"` into the first heading element found in
/// document order (spec §2.1 rule 5). Purely additive clone-transform: the
/// caller passes `injected` across calls so only the FIRST heading gets the
/// id. A pre-existing user `id` prop on that heading is left untouched —
/// the user's explicit id wins over the synthesized one (idiomatic attr
/// precedence; the semantic layer does not restrict modal-title ids).
fn inject_modal_title_id(node: &ViewTree, injected: &mut bool) -> ViewTree {
    if *injected {
        return node.clone();
    }
    match node {
        ViewTree::Element(el) => {
            if MODAL_HEADING_TAGS.contains(&el.tag.as_str()) {
                *injected = true;
                let mut new_el = (**el).clone();
                // F1 (spec §2.1 rule 5): a user-supplied `id` wins over the
                // synthesized one — `insert` would silently overwrite it.
                new_el
                    .props
                    .entry("id".to_string())
                    .or_insert(ExprIR::LiteralExpr {
                        value: serde_json::Value::String("omni-modal-title".to_string()),
                    });
                return ViewTree::Element(Box::new(new_el));
            }
            let children = el
                .children
                .iter()
                .map(|c| inject_modal_title_id(c, injected))
                .collect();
            ViewTree::Element(Box::new(ElementIR {
                children,
                ..(**el).clone()
            }))
        }
        ViewTree::Slot { name, fallback, .. } => ViewTree::Slot {
            name: name.clone(),
            fallback: fallback
                .iter()
                .map(|c| inject_modal_title_id(c, injected))
                .collect(),
        },
        ViewTree::Conditional(cond) => ViewTree::Conditional(Box::new(ConditionalIntent {
            branches: cond
                .branches
                .iter()
                .map(|b| ConditionalBranch {
                    body: b.body.as_ref().map(|v| inject_modal_title_id(v, injected)),
                    ..b.clone()
                })
                .collect(),
            fallback: cond
                .fallback
                .as_ref()
                .map(|v| inject_modal_title_id(v, injected)),
            transition_mode: cond.transition_mode.clone(),
        })),
        ViewTree::ListRendering(list) => ViewTree::ListRendering(Box::new(ListRenderingIntent {
            body: list
                .body
                .as_ref()
                .map(|v| inject_modal_title_id(v, injected)),
            empty_body: list
                .empty_body
                .as_ref()
                .map(|v| inject_modal_title_id(v, injected)),
            ..(**list).clone()
        })),
        ViewTree::Fragment(children) => ViewTree::Fragment(
            children
                .iter()
                .map(|c| inject_modal_title_id(c, injected))
                .collect(),
        ),
        ViewTree::Text(_) | ViewTree::Interpolation(_) => node.clone(),
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ViewTree variant — update codegen for new variant"),
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

            // ── Task 4 (批次 1 反馈模态): @modal / @errorBoundary /
            // @skeleton (spec §4 React 三目标映射) — each directive replaces
            // the element's normal rendering entirely. All three can only
            // appear on HTML elements (never `fragment` wrappers / component
            // instances), so these branches run before the component-
            // composition check.
            if let Some(modal) = &el.modal {
                // @modal → controlled `<dialog>` (spec §4.1 React verbatim):
                // the useLayoutEffect emitted in component/mod.rs (showModal()
                // /close()) is the single source of truth for the native
                // dialog state — the JSX deliberately carries NO `open`
                // attribute. Emitting `open={sig}` would open the dialog
                // NON-modally first, making showModal() throw ("already open
                // as a non-modal dialog"), and React 19 unmounts the whole
                // root on that error; showModal() itself sets the native
                // open attribute, so `dialog[open]` and `:modal` both match,
                // and the layout effect runs before paint (no flash).
                // onCancel intercepts the native cancel event (preventDefault
                // + controlled set) so it cannot desync the controlled state;
                // the backdrop click is detected via `e.target === ref.current`;
                // role="dialog" / aria-modal="true" / aria-labelledby carry
                // the dialog semantics.
                //
                // The ref name must mirror motion::walk_motion_elements: a
                // motion-carrying modal element is classified by
                // classify_motion_el (which consumes the shared counter); a
                // plain modal element is not a motion element and consumes
                // the counter via the modal-only branch. Mirrored below so
                // the synthesized `__el_{idx}` ref stays aligned with the
                // useLayoutEffect emitter.
                let modal_ref =
                    match super::motion::classify_motion_el(el, component_map, motion_counter) {
                        Some((ref_name, _)) => ref_name,
                        None => el.ref_binding.clone().unwrap_or_else(|| {
                            let idx = *motion_counter;
                            *motion_counter += 1;
                            format!("__el_{}", idx)
                        }),
                    };
                let setter = format!("set{}", capitalize_first(&modal.open_signal));
                // F1 (spec §2.1 rule 5): only with a Heading in the dialog
                // content is `aria-labelledby` emitted — and it points at the
                // synthesized `id="omni-modal-title"` the compiler places on
                // the first Heading (see inject_modal_title_id below).
                // Without a Heading the attribute is omitted entirely (the
                // semantic V13 warning already fired; a dangling reference
                // would be worse than none).
                let has_heading = modal
                    .slots
                    .iter()
                    .any(|slot| slot.fallback.iter().any(subtree_has_heading))
                    || el.children.iter().any(subtree_has_heading);
                let mut dlg_attrs = vec![
                    format!("ref={{{}Ref}}", modal_ref),
                    format!(
                        "onCancel={{(e) => {{ e.preventDefault(); {setter}(false); }}}}"
                    ),
                    format!(
                        "onClick={{(e) => {{ if (e.target === {modal_ref}Ref.current) {setter}(false); }}}}"
                    ),
                    "role=\"dialog\"".to_string(),
                    "aria-modal=\"true\"".to_string(),
                ];
                if has_heading {
                    dlg_attrs.push("aria-labelledby=\"omni-modal-title\"".to_string());
                }
                w.emit("<dialog");
                w.indent();
                for attr in &dlg_attrs {
                    w.emit(attr);
                }
                w.dedent();
                w.emit(">");
                // Named-slot content projects in document order; each slot's
                // fallback content lands directly in the dialog body
                // (spec §4.1: 内容为命名槽投影，匿名子内容兜底进 body 槽).
                w.indent();
                // F1: the first heading found in render order (named slots
                // first, then the anonymous body content) carries the
                // synthesized title id the aria-labelledby references.
                let mut injected = false;
                for slot in &modal.slots {
                    for child in &slot.fallback {
                        let node = if has_heading && !injected {
                            inject_modal_title_id(child, &mut injected)
                        } else {
                            child.clone()
                        };
                        gen_view_node_tsx(
                            w,
                            &node,
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
                            // dialog children are JSX children — expression
                            // nodes need `{...}` containers
                            true,
                        );
                    }
                }
                // F4 (spec §2.1 rule 3): anonymous children fall back into
                // the body slot — render everything that is NOT a slot
                // directive after the named-slot projections (the lowering
                // keeps `@slot` directives in `el.children` as
                // `ViewTree::Slot` nodes; those were just projected above
                // and must not render twice).
                for child in &el.children {
                    if matches!(child, ViewTree::Slot { .. }) {
                        continue;
                    }
                    let node = if has_heading && !injected {
                        inject_modal_title_id(child, &mut injected)
                    } else {
                        child.clone()
                    };
                    gen_view_node_tsx(
                        w,
                        &node,
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
                        // dialog children are JSX children
                        true,
                    );
                }
                w.dedent();
                w.emit("</dialog>");
                return;
            }

            if let Some(eb) = &el.error_boundary {
                // @errorBoundary → wrap the element's children in the
                // generated `ErrorBoundary` class (spec §4.4 React). The
                // body-level `@slot fallback` (lowered into
                // `error_boundary.fallback`) becomes the `fallback` prop;
                // with no fallback the class renders `null` (semantic V12
                // already warned).
                w.emit("<ErrorBoundary");
                if let Some(ref fb) = eb.fallback {
                    w.emit(" fallback={");
                    gen_branch_body_tsx(
                        w,
                        &ViewTree::Fragment(fb.fallback.clone()),
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
                        // the fallback sits inside the `fallback={...}` JSX
                        // expression — never a JSX-children position
                        false,
                    );
                    w.emit("}");
                }
                w.emit(">");
                w.indent();
                for child in &el.children {
                    // F7: the `@slot fallback` directive renders ONLY as the
                    // `fallback` prop above — filter it out of the children
                    // loop or the fallback UI is emitted a second time as
                    // `{fallback ?? ...}` (a runtime double-render; it even
                    // compiles clean when the slot name surfaces as a
                    // component prop). Named-slot filters mirror the semantic
                    // V14 duplicate-fallback detection pattern — only the
                    // `fallback` slot is special here, other slots keep
                    // rendering as ordinary children projections.
                    if matches!(child, ViewTree::Slot { name: Some(n), .. } if n == "fallback") {
                        continue;
                    }
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
                        // ErrorBoundary children are JSX children
                        true,
                    );
                }
                w.dedent();
                w.emit("</ErrorBoundary>");
                return;
            }

            if let Some(skeleton) = &el.skeleton {
                // @skeleton → placeholder element with the compiler-
                // generated shimmer classes (spec §4.3 React): the base
                // class plus the shape class (text/line/circle/card —
                // default `text`, legality checked by semantic V10). The
                // `omni-shimmer` keyframes and the class rules live in the
                // component's .module.css (component::gen_component_css).
                // The placeholder is self-closing — it carries no content.
                let shape = skeleton.shape.as_deref().unwrap_or("text");
                let skeleton_tag = to_react_tag(&el.tag);
                w.emit_fmt(format_args!(
                    "<{} className={{styles[\"omni-skeleton\"] + \" \" + styles[\"omni-skeleton--{shape}\"]}} />",
                    skeleton_tag
                ));
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
                            // Task 4 评审（retry 可达性）: @load 回调别名映射
                            // 为 load 标识符（load 已是 useCallback 稳定
                            // 引用，不能再包 __omni_cb_ 包装）。
                            let ref_emitted =
                                if r#ref.as_str() == "retry" && !comp.load_intent.is_empty() {
                                    "load".to_string()
                                } else if prop_fn_ctx.wrapped_fns.contains(r#ref) {
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
                                Some(warnings),
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
                    warnings.lock().unwrap().push(
                        "事件 'toggle' 在 React 目标仅 <details> 元素有合成事件（onToggle）——其他元素上处理器已跳过".to_string(),
                    );
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
                        // Task 4 评审（retry 可达性）: retry 是 @load 回调别
                        // 名（语义 pass 播种内建符号）——FnRef 直发路径映射
                        // 为 load 标识符（与表达式上下文的
                        // store_field_renames 映射一致）；别名不在生成输出
                        // 中发射，避免未使用局部（eslint no-unused-vars）。
                        let ref_name = if r#ref.as_str() == "retry" && !comp.load_intent.is_empty()
                        {
                            "load"
                        } else {
                            r#ref.as_str()
                        };
                        // Task 4 评审（TS2554 防护）: retry→load 映射的引用在
                        // lowering 侧按用户 fn 反查 used_fields 失败（"retry"
                        // 非 comp.fns 成员）→ event_data 整 schema 展开。load
                        // 是无参 useCallback，事件对象必须整体跳过（否则
                        // `load({ x: ... })` 报 Expected 0 arguments）。
                        let is_retry_mapped =
                            r#ref.as_str() == "retry" && !comp.load_intent.is_empty();
                        let call = if possibly_undefined.contains(ref_name) {
                            format!("{ref_name}?.")
                        } else {
                            ref_name.to_string()
                        };
                        // Task 4 (@form, spec 2026-08-08 §4.1): 提交拦截——
                        // form 元素的 @onSubmit 包装为 preventDefault + 校验
                        // 执行器：校验失败不发事件（return 跳过原 handler），
                        // DOM 默认提交被阻止。handler 实参形状与既有 setter
                        // 分支对齐（0 参 fn 不传 e，TS2554 避免）。
                        if html_tag == "form"
                            && react_event == "onSubmit"
                            && comp.form_intent.is_some()
                        {
                            // 实参形状按 fn 声明参数数保真（0 参不传 e 避免
                            // TS2554；1 参传 e 与 setter 分支对齐）——覆盖
                            // event_data 为空对象（空体 `fn submit(e) {}` 是
                            // brief 的规范形态，无消费字段）与无事件数据两种
                            // 路径；retry→load 映射（load 恒 0 参）保持零参。
                            let handler_call = if is_retry_mapped {
                                format!("{call}();")
                            } else if event_data.is_some() {
                                let obj = emit_event_object_react(event_data, used_fields, "e");
                                if obj == "{}" {
                                    let n_params = comp
                                        .fns
                                        .iter()
                                        .find(|f| f.name == ref_name)
                                        .map(|f| f.params.len())
                                        .unwrap_or(0);
                                    if n_params == 1 {
                                        format!("{call}(e);")
                                    } else {
                                        format!("{call}();")
                                    }
                                } else {
                                    format!("{call}({obj});")
                                }
                            } else {
                                let n_params = comp
                                    .fns
                                    .iter()
                                    .find(|f| f.name == ref_name)
                                    .map(|f| f.params.len())
                                    .unwrap_or(0);
                                if n_params == 1 {
                                    format!("{call}(e);")
                                } else {
                                    format!("{call}();")
                                }
                            };
                            // M1: 校验通过时清空残留错误（此前失败提交留下的
                            // formErrors），随后才调用业务 handler。
                            attrs.push(format!(
                                "{}={{(e) => {{ e.preventDefault(); const errs = validateForm(formValues); if (Object.keys(errs).length > 0) {{ setFormErrors(errs); return; }} setFormErrors({{}}); {} }}}}",
                                react_event, handler_call
                            ));
                            continue;
                        }
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
                        } else if event_data.is_some() && !is_retry_mapped {
                            let event_obj = emit_event_object_react(event_data, used_fields, "e");
                            // When no event fields are actually needed (empty object),
                            // skip the (e) wrapper to avoid unused-param warnings.
                            if event_obj == "{}" {
                                attrs.push(format!("{}={{{}}}", react_event, ref_name));
                            } else {
                                attrs.push(format!(
                                    "{}={{(e) => {{ {}({}) }}}}",
                                    react_event, call, event_obj
                                ));
                            }
                        } else {
                            attrs.push(format!("{}={{{}}}", react_event, ref_name));
                        }
                    }
                    EventHandlerIR::InlineFn {
                        body,
                        params,
                        event_data,
                        used_fields,
                        ..
                    } => {
                        // Task 7 (批次 4 数据能力): debounce/throttle 值形态
                        // （spec §2.5 + §4.1）——单 ExprStmt 体且调用名 ∈
                        // TIMING_FUNCS 时，处理器即期望值：`@onInput={debounce(
                        // doSearch, 300)}` → `onInput={useMemo(() => debounce(
                        // doSearch, 300), [doSearch])}` 稳定引用（trailing-only
                        // 默认，P6）。deps = [fn]（首参标识符）或 []。
                        match body.statements.as_slice() {
                            [StatementIR::ExprStmtIR {
                                expr: ExprIR::CallExpr { name, args },
                            }] if crate::semantic::load_contract::TIMING_FUNCS
                                .contains(&name.as_str()) =>
                            {
                                let formatted: Vec<String> = args
                                    .iter()
                                    .map(|a| {
                                        format_expr_ir_tsx_renamed(
                                            a,
                                            ref_names,
                                            store_field_renames,
                                        )
                                    })
                                    .collect();
                                let native = try_format_stdlib_call(
                                    name.as_str(),
                                    &formatted,
                                    Target::React,
                                )
                                .unwrap_or_default();
                                let dep = match args.first() {
                                    Some(ExprIR::IdentifierExpr { .. }) => {
                                        formatted.first().cloned().unwrap_or_default()
                                    }
                                    _ => String::new(),
                                };
                                let deps = if dep.is_empty() {
                                    "[]".to_string()
                                } else {
                                    format!("[{dep}]")
                                };
                                attrs.push(format!(
                                    "{}={{useMemo(() => {}, {})}}",
                                    react_event, native, deps
                                ));
                                continue;
                            }
                            _ => {}
                        }
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
                                    Some(warnings),
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
                                        Some(warnings),
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
                                    Some(warnings),
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
            emit_drag_react_attrs(el, comp, &mut attrs, possibly_undefined, warnings);

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

            // Task 4 (@form, spec 2026-08-08 §4.1): 校验字段 aria 接线——
            // bind:value/bind:checked 的字段若在 @form 校验清单
            // （FormIntentIR.fields）中，发射 aria-invalid/aria-describedby。
            // 错误文本容器由用户 @if={form.errors.<field>} 显式渲染，
            // codegen 不自动生成错误 DOM（错误容器 id 约定 err-<field>）。
            if let Some(form) = &comp.form_intent {
                // Task 6 评审（plan-mandated，与 OmniView 目标 4b25619f 对齐）:
                // @form 表单发射 noValidate——浏览器原生约束校验在 submit
                // 事件之前拦截失败提交（必填为空时 submit 不 fire、
                // validateForm 永不运行、formErrors 永不填充），与「自绘
                // 错误」受控校验三态不兼容。required/pattern 等 HTML5 属性
                // 保留（a11y 语义），校验职责由 validateForm 独占。JSX 布尔
                // 属性缩写字面量即 noValidate={true}；用户显式声明
                // noValidate 时防重复（JSX 不允许同名重复属性）。
                if html_tag == "form" && !attrs.iter().any(|a| a.starts_with("noValidate")) {
                    attrs.push("noValidate".to_string());
                }
                for (bind_name, signal_ref) in &el.bindings {
                    if matches!(bind_name.as_str(), "value" | "checked") {
                        let field = signal_ref
                            .name
                            .rsplit('.')
                            .next()
                            .unwrap_or(&signal_ref.name);
                        if form.fields.iter().any(|f| f.name == field) {
                            attrs.push(format!(
                                "aria-invalid={{formErrors[\"{field}\"] ? true : undefined}}"
                            ));
                            attrs.push(format!(
                                "aria-describedby={{formErrors[\"{field}\"] ? \"err-{field}\" : undefined}}"
                            ));
                        }
                    }
                }
            }

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
            // Task 7 (批次 4 数据能力): 无限滚动哨兵（spec §2.3）——
            // scroll 模式 @load 的 data 列表在末项后发射哨兵 div
            // （IntersectionObserver 观察）。哨兵判定
            // `_i === __omni_iterable.length - 1` 需要 __omni_iterable
            // 作用域，故无 empty_body 的列表也强制 IIFE 形态
            // （空态分支 (null) 兜底——裸 `()` 是语法错误）。
            // V7/V8（批次 4 验证网）保证 scroll 必 named + paginated。
            let scroll_iterable = match &list.iterable {
                ExprIR::IdentifierExpr { name, .. } => comp.load_intent.iter().find_map(|li| {
                    if li.mode.scroll {
                        let data_name = match li.name.as_deref() {
                            Some(p) => format!("{p}Data"),
                            None => "data".to_string(),
                        };
                        (data_name == *name).then(|| {
                            format!(
                                "{}ScrollSentinelRef",
                                li.name.as_deref().unwrap_or_default()
                            )
                        })
                    } else {
                        None
                    }
                }),
                _ => None,
            };
            let force_scroll_iife = scroll_iterable.is_some();
            if list.empty_body.is_some() || force_scroll_iife {
                if in_jsx_children {
                    w.emit("{");
                    w.indent();
                }
                // Single-eval the iterable (Task 5.5): the previous shape
                // `({} && {}.length === 0) ? ... : ({}.map(...))` evaluated
                // the iterable TWICE per render (side effects / allocations
                // ran twice). Hoist to a const inside an IIFE so the list
                // expression runs exactly once. Task 6 (batch 4): data 可空化
                // 后迭代点统一 `?? []` 兜底（spec §2.1）——null 迭代不崩溃。
                w.emit_fmt(format_args!(
                    "(() => {{ const __omni_iterable = {} ?? []; return (__omni_iterable && __omni_iterable.length === 0) ? (",
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
                } else if force_scroll_iife {
                    w.emit("null");
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
                // Task 6（批次 4）: 无 empty_body 路径同样 `?? []` 兜底
                // （data 可空化后 null 直接崩溃——spec §2.1 渲染兜底）
                w.emit_fmt(format_args!(
                    "({} ?? []).map(({}, _i) => (",
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
            // Task 7 (批次 4 数据能力): 无限滚动哨兵——列表末项后发射
            // 哨兵 div（spec §2.3）。`__omni_iterable` 在强制 IIFE 形态
            // 下作用域内可见；双守卫（LoadingMore/HasMore）在
            // LoadMore 回调内，哨兵本身仅判定"末项"。
            if let Some(sentinel_ref) = &scroll_iterable {
                w.emit_fmt(format_args!(
                    "{{_i === __omni_iterable.length - 1 ? <div ref={{{}}} /> : null}}",
                    sentinel_ref
                ));
            }
            w.dedent();
            w.emit("</React.Fragment>");
            w.dedent();
            w.emit("))");
            w.dedent();
            if list.empty_body.is_some() || force_scroll_iife {
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
            // Layout components (@layout) render `<Outlet />` at the
            // UNNAMED slot position (spec §4.2) — nested routes mount into
            // the layout's body via the router's children tree, not via a
            // `children` prop.
            if comp.layout_intent.is_some() && name.is_none() {
                w.emit("<Outlet />");
                return;
            }
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
    warnings: Option<&Mutex<Vec<String>>>,
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
            warnings,
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
            warnings,
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
    comp: &ComponentIR,
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
    // Task 4 评审（retry 可达性）: @load 回调别名在 @drag 处理器上同样映
    // 射为 load 标识符（load 是无参 useCallback，`load()` 直调合法）。
    let move_fn = if move_fn == "retry" && !comp.load_intent.is_empty() {
        "load"
    } else {
        move_fn
    };
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

    // Handler arity — same resolution as semantic/validate_drag (component
    // fns; the FnRef IR's own `params` is empty for drag handlers). An
    // empty DragIntent (`{}`) is only callable bare when the handler
    // declares zero params: `fn move(e) {}` still requires its argument
    // (`move({})` — a bare `move()` would be TS2554 "Expected 1 arguments,
    // but got 0"), while `fn move()` must NOT receive one (TS2554 the
    // other way).
    let fn_arity = |name: &str| -> usize {
        comp.fns
            .iter()
            .find(|f| f.name == name)
            .map_or(0, |f| f.params.len())
    };
    // Call arguments: the object literal as-is, or `{}` for an empty
    // DragIntent — unless the handler takes no params, then no arguments.
    let call_args = |obj: &str, name: &str| -> String {
        // Task 4 评审（TS2554 防护）: retry→load 映射（load 为无参
        // useCallback）恒零参直调——"load" 不在 comp.fns，fn_arity 查无
        // 返回 0，但非空 DragIntent 仍会走到 else 分支传对象；对 load
        // 整体跳过实参（含 @dragEnd 侧，共用本闭包）。
        if name == "load" && !comp.load_intent.is_empty() {
            return String::new();
        }
        if obj == "{}" {
            if fn_arity(name) == 0 {
                String::new()
            } else {
                "{}".to_string()
            }
        } else {
            obj.to_string()
        }
    };

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
    let move_args = call_args(&move_obj, move_fn);
    let move_body = if move_args.is_empty() {
        format!("if ({state_expr}) {move_call}();")
    } else {
        format!("if ({state_expr}) {move_call}({move_args});")
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
            // Task 4 评审: @dragEnd={retry} 同 move 侧映射为 load。
            let end_fn = if end_fn == "retry" && !comp.load_intent.is_empty() {
                "load"
            } else {
                end_fn
            };
            let end_call = call(end_fn);
            let end_obj = drag_event_object_react(end_fields, "s");
            let end_args = call_args(&end_obj, end_fn);
            if end_args.is_empty() {
                format!(" if (s) {end_call}();")
            } else {
                format!(" if (s) {end_call}({end_args});")
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
        // Bind the event arg only when the body reads it. An arity-0 drag
        // handler (`fn move()`) or an intent object without raw-event
        // fields (dx/dy/startX/startY reference the snapshot, not `e`)
        // leaves `e` unused — which trips ESLint no-unused-vars in
        // gate'd builds. The pointermove guard and `s` snapshot are
        // identifiers (never "e."), so the probe is exact.
        let param = if body.contains("e.") { "(e)" } else { "()" };
        attrs.push(format!("{prop}={{{param} => {{ {body} }}}}"));
    }
}

/// Build the DragIntent event object literal for a drag handler call.
///
/// DragIntent fields are synthesized (start snapshot + pointer delta), not
/// raw event passthrough — the shared `emit_event_object_react` cannot
/// express them (its DragIntent mapping falls through to `undefined`), so
/// the object is built here, honoring `used_fields` with the same
/// filter-in-schema-order semantics. `state` names the drag-start
/// snapshot: `{ref}_dragStartRef.current` for pointermove, `s` for the
/// pointerup snapshot.
///
/// No `?? 0` fallback: every extractor is statically non-null inside the
/// caller's guard — `e.clientX/clientY/pointerId` are typed `number`, and
/// the snapshot is narrowed by `if ({state})` / `if (s)`. A fallback
/// would be dead code and trips TS2869 / ESLint
/// no-constant-binary-expression on the synthesized expression.
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
            .map(|(name, extract)| format!("{name}: {extract}"))
            .collect(),
        Some(_) => return "{}".to_string(),
        None => all
            .into_iter()
            .map(|(name, extract)| format!("{name}: {extract}"))
            .collect(),
    };
    format!("{{ {} }}", fields.join(", "))
}
