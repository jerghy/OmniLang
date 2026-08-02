//! Integration tests for element ref system in OmniView codegen.
//! Tests ref declaration → wrapper object generation → element binding pipeline.

/// Test: basic ref declaration and binding generates wrapper object
#[test]
fn test_ref_basic_wrapper() {
    let source = r#"
component TestRef {
  ref box: Element
  view {
    <div ref:box>content</div>
  }
}
"#;
    let result = crate::compile_omniview(source, false, false);
    assert!(result.success, "compilation failed: {:?}", result.errors);
    let js = result
        .files
        .as_ref()
        .expect("Expected JS output")
        .js
        .clone();
    let html = result
        .files
        .as_ref()
        .expect("Expected HTML output")
        .html
        .clone();

    // Wrapper object exists
    assert!(
        js.contains("const box = (function()"),
        "JS should contain ref wrapper. Got:\n{}",
        js
    );
    // Uses __omni_el for element lookup
    assert!(
        js.contains("__omni_el("),
        "Wrapper should use __omni_el. Got:\n{}",
        js
    );
    // No stale data-ref attribute in HTML
    assert!(
        !html.contains("data-ref=\"box\""),
        "HTML should not have data-ref. Got:\n{}",
        html
    );
    // HTML still has the div with content
    assert!(
        html.contains("content"),
        "HTML should contain the element content. Got:\n{}",
        html
    );
}

/// Test: ref in component with @for (ref outside template)
#[test]
fn test_ref_not_in_for_template() {
    // For now, ref inside @for is not supported. Verify it at least
    // doesn't crash the compiler.
    let source = r#"
component TestRefFor {
  signal items: String[] = ["a", "b"]
  ref first: Element
  view {
    <div>
      <div ref:first>static</div>
      @for item in items {
        <span>{item}</span>
      }
    </div>
  }
}
"#;
    let result = crate::compile_omniview(source, false, false);
    assert!(
        result.success,
        "compilation should succeed without panic: {:?}",
        result.errors
    );
    let js = result
        .files
        .as_ref()
        .expect("Expected JS output")
        .js
        .clone();

    // Ref outside @for should still work
    assert!(
        js.contains("const first = (function()"),
        "Ref outside @for should still work. Got:\n{}",
        js
    );
}

/// Test: multiple refs in one component
#[test]
fn test_multiple_refs() {
    let source = r#"
component MultiRef {
  ref header: Element
  ref footer: Element
  view {
    <div>
      <div ref:header>Header</div>
      <div ref:footer>Footer</div>
    </div>
  }
}
"#;
    let result = crate::compile_omniview(source, false, false);
    assert!(result.success, "compilation failed: {:?}", result.errors);
    let js = result
        .files
        .as_ref()
        .expect("Expected JS output")
        .js
        .clone();

    assert!(
        js.contains("const header = (function()"),
        "Expected header ref wrapper"
    );
    assert!(
        js.contains("const footer = (function()"),
        "Expected footer ref wrapper"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Semantic ref bindings → __omni_effect wrappers
// ═════════════════════════════════════════════════════════════════════════════

/// Test: focus/scroll-x/scroll-y/value semantic bindings emit __omni_effect
/// wrappers driving the bound signal → native DOM side effects.
#[test]
fn test_semantic_bindings_emit_effects() {
    let source = r#"
component SearchBox {
  signal active: Bool = false
  signal pos: Float = 0
  signal hpos: Float = 0
  signal text: String = ""
  ref field: Element
  view {
    <input ref:field focus={active} scroll-y={pos} scroll-x={hpos} value={text} />
  }
}
"#;
    let result = crate::compile_omniview(source, false, false);
    assert!(
        result.success,
        "compilation should succeed: {:?}",
        result.errors
    );
    let js = result
        .files
        .as_ref()
        .expect("Expected JS output")
        .js
        .clone();

    // focus binding → focus()/blur() on the boolean signal
    assert!(
        js.contains("if (SearchBox_active.value) { el.focus(); } else { el.blur(); }"),
        "focus binding should emit focus/blur effect. Got:\n{}",
        js
    );
    // scroll-y binding → el.scrollTop
    assert!(
        js.contains("el.scrollTop = SearchBox_pos.value;"),
        "scroll-y binding should emit scrollTop assignment. Got:\n{}",
        js
    );
    // scroll-x binding → el.scrollLeft
    assert!(
        js.contains("el.scrollLeft = SearchBox_hpos.value;"),
        "scroll-x binding should emit scrollLeft assignment. Got:\n{}",
        js
    );
    // value binding → guarded one-way value sync
    assert!(
        js.contains("if (el.value !== SearchBox_text.value) { el.value = SearchBox_text.value; }"),
        "value binding should emit guarded value sync. Got:\n{}",
        js
    );
}

/// Test: the semantic binding effects key on the element nid via __omni_el.
#[test]
fn test_semantic_bindings_use_omniel() {
    let source = r#"
component FocusBox {
  signal on: Bool = false
  ref box: Element
  view {
    <div ref:box focus={on}>content</div>
  }
}
"#;
    let result = crate::compile_omniview(source, false, false);
    assert!(
        result.success,
        "compilation should succeed: {:?}",
        result.errors
    );
    let js = result
        .files
        .as_ref()
        .expect("Expected JS output")
        .js
        .clone();

    assert!(
        js.contains("const el = __omni_el(\"el-div-"),
        "binding effect should look up the element by nid. Got:\n{}",
        js
    );
    assert!(
        js.contains("if (el) { if (FocusBox_on.value) { el.focus(); }"),
        "binding effect should guard on element presence. Got:\n{}",
        js
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// RefOpExpr → native DOM APIs
// ═════════════════════════════════════════════════════════════════════════════

/// Test: every RefOpIR variant formats to a native DOM call on the ref wrapper.
#[test]
fn test_ref_ops_format_native_dom_calls() {
    let source = r#"
component RefOps {
  signal x: Float = 0
  signal y: Float = 0
  signal text: String = ""
  ref box: Element
  fn runAll() {
    box.focus()
    box.blur()
    box.scrollTo(x, y)
    box.scrollBy(1, 2)
    box.measure()
    box.width
    box.height
    box.position
    box.select()
    box.setSelection(0, 3)
    box.value
    box.capturePointer()
    box.releasePointer()
  }
  view {
    <div ref:box @onClick={runAll}>click</div>
  }
}
"#;
    let result = crate::compile_omniview(source, false, false);
    assert!(
        result.success,
        "compilation should succeed: {:?}",
        result.errors
    );
    let js = result
        .files
        .as_ref()
        .expect("Expected JS output")
        .js
        .clone();
    let expectations = [
        "box.focus();",
        "box.blur();",
        "box.scrollTo({ left: RefOps_x.value, top: RefOps_y.value });",
        "box.scrollBy({ left: 1, top: 2 });",
        "box.getBoundingClientRect();",
        "box.getBoundingClientRect().width;",
        "box.getBoundingClientRect().height;",
        "box.select();",
        "box.setSelectionRange(0, 3);",
        "box.value;",
        // Design D3: parameterless — the wrapper resolves the ACTIVE pointer
        // from the runtime tracker (no id is passed at the call site).
        "box.capturePointer();",
        "box.releasePointerCapture();",
    ];
    for expected in expectations {
        assert!(
            js.contains(expected),
            "expected RefOp output to contain {:?}. Got:\n{}",
            expected,
            js
        );
    }
    // Ref wrapper shape: capturePointer/releasePointerCapture are
    // parameterless and read the runtime's active-pointer tracker; the DSL
    // releasePointer maps to the native releasePointerCapture API
    // (releasePointer does not exist on DOM). scrollTo/scrollBy forward the
    // options object as ONE argument (the 2-arg (x, y) forwarding breaks the
    // options-object call → NaN → no scroll).
    let wrapper_assertions = [
        "capturePointer() { const e = _el(); if (e && __omni_activePointerId != null) e.setPointerCapture(__omni_activePointerId); },",
        "releasePointerCapture() { const e = _el(); if (e && __omni_activePointerId != null) e.releasePointerCapture(__omni_activePointerId); },",
        "scrollTo(opts) { const e = _el(); if (e) e.scrollTo(opts); },",
        "scrollBy(opts) { const e = _el(); if (e) e.scrollBy(opts); },",
    ];
    for expected in wrapper_assertions {
        assert!(
            js.contains(expected),
            "expected ref wrapper to contain {:?}. Got:\n{}",
            expected,
            js
        );
    }
    // The runtime's active-pointer tracker (D3) records the most recent
    // pointerdown's id in the capture phase.
    assert!(
        js.contains("__omni_activePointerId = e.pointerId")
            && js.contains("document.addEventListener('pointerdown'"),
        "runtime must track the active pointer id. Got:\n{}",
        js
    );
    for broken in [
        "e.releasePointer(id)",
        "capturePointer(id)",
        "e.scrollTo(x, y)",
        "e.scrollBy(dx, dy)",
    ] {
        assert!(
            !js.contains(broken),
            "ref wrapper must not contain broken form {:?}. Got:\n{}",
            broken,
            js
        );
    }
}

/// Test: imperative ref writes (`ref.value = x`, `ref.setValue(x)`, and the
/// bare `ref = x` form) are hard-diagnosed with the bind:value migration hint
/// (SetValue removed, 11 → 10 ops — two-way `bind:value` covers the write
/// intent, D2).
#[test]
fn test_ref_value_write_forms_are_diagnosed() {
    for write in ["box.value = text", "box.setValue(text)", "box = text"] {
        let source = format!(
            r#"
component RefOps {{
  signal text: String = ""
  ref box: Element
  fn runAll() {{
    {write}
  }}
  view {{
    <div ref:box @onClick={{runAll}}>click</div>
  }}
}}
"#
        );
        let result = crate::compile_omniview(&source, false, false);
        assert!(
            !result.success,
            "write form `{}` must fail to compile",
            write
        );
        let errors: String = result
            .errors
            .iter()
            .map(|d| d.message.as_str())
            .collect::<Vec<_>>()
            .join(" | ");
        assert!(
            errors.contains("命令式写已删除") && errors.contains("bind:value"),
            "write form `{}` must diagnose with the bind:value hint. Got: {}",
            write,
            errors
        );
    }
}

/// Test: the two-way `bind:value` binding (the replacement for the removed
/// imperative write) still compiles on a ref-bound element and emits the
/// two-way sync machinery.
#[test]
fn test_bind_value_two_way_binding_unaffected() {
    let source = r#"
component SearchBox {
  signal text: String = ""
  ref field: Element
  view {
    <input ref:field bind:value={text} />
  }
}
"#;
    let result = crate::compile_omniview(source, false, false);
    assert!(
        result.success,
        "bind:value must still compile: {:?}",
        result.errors
    );
    let js = result
        .files
        .as_ref()
        .expect("Expected JS output")
        .js
        .clone();
    // The two-way sync emits the guarded value write-back (signal → element)
    // and the "input" event listener (element → signal).
    assert!(
        js.contains("__omni_subs.get(\"SearchBox_text\")"),
        "bind:value must wire the bound signal. Got:\n{}",
        js
    );
    assert!(
        js.contains("addEventListener(\"input\"") || js.contains("\"input\""),
        "bind:value must wire the input event listener. Got:\n{}",
        js
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Slot fallback rendering
// ═════════════════════════════════════════════════════════════════════════════

/// Test: slot fallback content renders inside the <slot> element.
#[test]
fn test_slot_fallback_renders_inside_slot() {
    let source = r#"
component Panel {
  view {
    <div>
      @slot header {
        <h4>Default header</h4>
      }
      @slot {
        Default body
      }
    </div>
  }
}
"#;
    let result = crate::compile_omniview(source, false, false);
    assert!(
        result.success,
        "compilation should succeed: {:?}",
        result.errors
    );
    let html = result
        .files
        .as_ref()
        .expect("Expected HTML output")
        .html
        .clone();

    // Fallback content must appear between the slot's open and close tags.
    let named_open = "<slot name=\"header\">";
    let named_pos = html.find(named_open).expect("named slot opening tag");
    let named_close = html[named_pos..]
        .find("</slot>")
        .expect("named slot closing tag");
    assert!(
        html[named_pos..named_pos + named_close].contains("Default header"),
        "named slot fallback should render inside the slot element. Got:\n{}",
        html
    );
    let default_open = "<slot>";
    let default_pos = html.find(default_open).expect("default slot opening tag");
    let default_close = html[default_pos..]
        .find("</slot>")
        .expect("default slot closing tag");
    assert!(
        html[default_pos..default_pos + default_close].contains("Default body"),
        "default slot fallback should render inside the slot element. Got:\n{}",
        html
    );
}

/// Test: slot without fallback stays empty (childrenMap injection fills it).
#[test]
fn test_slot_without_fallback_stays_empty() {
    let source = r#"
component EmptySlots {
  view {
    <div>
      @slot header
      @slot
    </div>
  }
}
"#;
    let result = crate::compile_omniview(source, false, false);
    assert!(
        result.success,
        "compilation should succeed: {:?}",
        result.errors
    );
    let html = result
        .files
        .as_ref()
        .expect("Expected HTML output")
        .html
        .clone();

    assert!(
        html.contains("<slot name=\"header\"></slot>"),
        "named slot without fallback should stay empty. Got:\n{}",
        html
    );
    assert!(
        html.contains("<slot></slot>"),
        "default slot without fallback should stay empty. Got:\n{}",
        html
    );
}
