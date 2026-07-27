//! Regression tests for OmniView @for list rendering codegen.
//!
//! Covers:
//! - Basic state init with no body (template HTML + effect header)
//! - Keyed iteration with empty-state fallback
//! - Event delegation setup

use super::super::*;
use rustc_hash::FxHashMap;

// ═════════════════════════════════════════════════════════════════════════════
// @for List Rendering Codegen Tests
// ═════════════════════════════════════════════════════════════════════════════

/// Tests basic @for state initialization with no body, no key, no empty state.
/// Verifies template HTML structure and JS effect header.
#[test]
fn test_list_state_init_basic() {
    let mut gen = OmniViewGenerator::test_new();
    let list = ListRenderingIntent {
        iterable: ExprIR::LiteralExpr {
            value: serde_json::json!([1, 2, 3]),
        },
        item_binding: BindingIR {
            name: "item".into(),
            r#type: None,
        },
        body: None,
        key_expr: None,
        empty_body: None,
    };
    let html = gen.emit_list_rendering_html(&list, 0);

    // Template HTML: item template
    assert!(
        html.contains("<template data-nid=\"el-for-001-tpl\">"),
        "should emit item template with nid"
    );
    assert!(
        html.contains("<!-- empty item -->"),
        "should emit empty item placeholder when no body"
    );

    // Template HTML: container marker
    assert!(
        html.contains("<template data-nid=\"el-for-001-ctr\" data-for-container>"),
        "should emit container marker"
    );

    // No empty-state template (only item template and container marker)
    assert!(
        !html.contains("-empty\""),
        "should not emit empty-state template when no empty_body"
    );

    // JS effect header
    assert!(
        gen.js.contains("__omni_effect(() => {"),
        "should wrap in __omni_effect"
    );
    assert!(
        gen.js.contains("const data = [1, 2, 3]"),
        "should set data from iterable expression"
    );

    // Template/container lookup
    assert!(
        gen.js.contains("const tpl = __omni_el(\"el-for-001-tpl\")"),
        "should look up template by nid"
    );
    assert!(
        gen.js.contains("const ctr = __omni_el(\"el-for-001-ctr\")"),
        "should look up container by nid"
    );

    // Guard clauses for missing elements
    assert!(
        gen.js.contains("if (!tpl || !ctr) return;"),
        "should guard missing template/container"
    );
    assert!(
        gen.js.contains("const parent = tpl.parentNode"),
        "should get parent from template"
    );
    assert!(
        gen.js.contains("if (!parent) return;"),
        "should guard missing parent"
    );

    // Clear loop: remove rendered items between tpl and ctr
    assert!(
        gen.js
            .contains("while (tpl.nextElementSibling && tpl.nextElementSibling !== ctr)"),
        "should clear between tpl and ctr"
    );

    // Empty data guard (no empty_body → early return)
    assert!(
        gen.js.contains("if (!data || data.length === 0) return;"),
        "should early-return for empty data"
    );

    // Iteration
    assert!(
        gen.js.contains("data.forEach((item, _idx) => {"),
        "should iterate data"
    );

    // Clone mechanism
    assert!(
        gen.js
            .contains("var __root = tpl.content.firstElementChild"),
        "should get template root"
    );
    assert!(
        gen.js.contains("var __cloneItem = function()"),
        "should define clone function"
    );

    // Insert before container marker
    assert!(
        gen.js.contains("parent.insertBefore(clone, ctr)"),
        "should insert before container marker"
    );

    // No key-specific code
    assert!(
        !gen.js.contains("_forKeys"),
        "should not have key map without key_expr"
    );
    assert!(
        !gen.js.contains("Omni.flipBefore"),
        "should not have FLIP without key"
    );

    // Close effect
    assert!(gen.js.ends_with("});\n"), "should close __omni_effect");
}

/// Tests @for with key expression and empty-state fallback.
/// Verifies key map setup, empty-state template, and leave animation integration.
#[test]
fn test_list_with_key_and_empty_state() {
    let mut gen = OmniViewGenerator::test_new();
    let list = ListRenderingIntent {
        iterable: ExprIR::LiteralExpr {
            value: serde_json::json!([1, 2, 3]),
        },
        item_binding: BindingIR {
            name: "item".into(),
            r#type: None,
        },
        body: None,
        key_expr: Some(ExprIR::ident("item")),
        empty_body: Some(ViewTree::Text("Nothing to show".into())),
    };
    let html = gen.emit_list_rendering_html(&list, 0);

    // Empty-state template in HTML
    assert!(
        html.contains("<template data-nid=\"el-for-001-empty\">"),
        "should emit empty-state template"
    );

    // Key map initialization on container
    assert!(
        gen.js
            .contains("const _forKeys = ctr._forKeys || (ctr._forKeys = new Map())"),
        "should set up key map on container"
    );

    // Key function definition
    assert!(
        gen.js.contains("const keyFn = (item) =>"),
        "should define key function"
    );

    // Old/new key set tracking
    assert!(
        gen.js.contains("const oldKeys = new Set(_forKeys.keys())"),
        "should capture old keys"
    );
    assert!(
        gen.js.contains("const newKeys = new Set()"),
        "should create new keys set"
    );

    // Empty state handling
    assert!(
        gen.js.contains("if (!data || data.length === 0)"),
        "should check for empty data"
    );
    assert!(
        gen.js
            .contains("const emptyEl = __omni_el(\"el-for-001-empty\")"),
        "should look up empty template"
    );
    assert!(
        gen.js.contains("firstElementChild.cloneNode(true)"),
        "should clone empty template content"
    );

    // FLIP animation recording
    assert!(
        gen.js.contains("Omni.flipBefore(parent)"),
        "should record FLIP positions before"
    );
    assert!(
        gen.js.contains("Omni.flipAfter(parent)"),
        "should animate FLIP positions after"
    );

    // Key-based identity tracking inside forEach
    assert!(
        gen.js.contains("const _key = keyFn(item)"),
        "should compute key per item"
    );
    assert!(
        gen.js.contains("newKeys.add(_key)"),
        "should track new keys"
    );

    // Leave animation integration in key-based cleanup
    assert!(
        gen.js
            .contains("__omni_animations.get(leavingNid + '_leave')"),
        "should check for leave animation"
    );
    assert!(
        gen.js.contains("player.play().then(() =>"),
        "should play leave animation then remove"
    );

    // No warnings
    assert!(gen.warnings.is_empty(), "no warnings expected");
}

/// Tests @for with events on the body element, verifying delegated event
/// listener setup with _eventKinds map, _extractEvent function, and
/// _forHandle event handler.
#[test]
fn test_list_with_event_delegation() {
    let mut gen = OmniViewGenerator::test_new();

    let mut events = FxHashMap::default();
    events.insert(
        "onClick".into(),
        EventHandlerIR::FnRef {
            r#ref: "handleClick".into(),
            params: vec!["e".into()],
            event_data: Some(EventDataKind::PointerEvent),
            used_fields: None,
            needs_prevent_default: false,
            needs_stop_propagation: false,
        },
    );

    let body = ViewTree::Element(ElementIR {
        tag: "button".into(),
        props: FxHashMap::default(),
        bindings: FxHashMap::default(),
        events,
        style_classes: None,
        inline_style: None,
        children: vec![],
        test_id: None,
        ref_name: None,
        construction_kind: ConstructionKind::Static,
        class_map_bindings: FxHashMap::default(),
        source_line: 0,
        source_col: 0,
        motion: None,
        scroll: None,
        drag: None,
        hero: None,
    });

    let list = ListRenderingIntent {
        iterable: ExprIR::LiteralExpr {
            value: serde_json::json!(["a", "b", "c"]),
        },
        item_binding: BindingIR {
            name: "item".into(),
            r#type: None,
        },
        body: Some(body),
        key_expr: None,
        empty_body: None,
    };
    let _html = gen.emit_list_rendering_html(&list, 0);

    // Event kind lookup map (maps DOM event names to EventDataKind labels)
    assert!(
        gen.js.contains("const _eventKinds = {"),
        "should emit event kind lookup map"
    );
    assert!(
        gen.js.contains("'click': 'PointerEvent'"),
        "should map onClick -> click PointerEvent"
    );

    // Event extraction function (switch on EventDataKind)
    assert!(
        gen.js.contains("function _extractEvent(kind, e)"),
        "should emit event extraction function"
    );
    assert!(
        gen.js.contains("case 'PointerEvent'"),
        "should have PointerEvent case"
    );

    // Handler function with closest-based event delegation
    assert!(
        gen.js.contains("const _forHandle = (e) =>"),
        "should emit event handler"
    );
    assert!(
        gen.js
            .contains("e.target.closest('[data-event-' + e.type + ']')"),
        "should use event delegation via closest"
    );

    // One-time attachment guard on container
    assert!(
        gen.js.contains("ctr.dataset.forEventsAttached"),
        "should guard against duplicate attachment"
    );

    // Dynamic event subscription
    assert!(
        gen.js.contains("['click'].forEach(function(evt)"),
        "should subscribe to click event"
    );
    assert!(
        gen.js.contains("parent.addEventListener(evt, _forHandle)"),
        "should add event listener to parent"
    );

    // The rebind function is referenced in the event handler (for state sync)
    // even when there are no text bindings to rebind.
    assert!(
        gen.js.contains("typeof __omni_rebind__item === 'function'"),
        "should reference rebind function in event handler"
    );
    // But the rebind function DEFINITION should NOT be emitted without text bindings
    assert!(
        !gen.js.contains("function __omni_rebind__item("),
        "should not define rebind function without text bindings"
    );

    // No warnings
    assert!(gen.warnings.is_empty(), "no warnings expected");
}
