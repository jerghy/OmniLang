//! Regression tests for OmniView event-emit name resolution (fix 7.5b-K1).
//!
//! Root cause: `emit eventName(...)` statements were resolved through a
//! bare-event-name rename table that is last-wins — when two components
//! declare the same event name (e.g. `onSwitch` on TabBar AND on
//! HomePage/AboutPage), the later-declared component's prefixed emitter
//! overwrote the earlier one. Every TabBar button then dispatched to
//! `AboutPage_onSwitch` (a never-mounted emitter), `App_switchTab` never
//! fired, and all tab switching broke (33/45 browser tests, single root
//! cause).
//!
//! Fix: `emit` resolution is component-scoped — the currently-emitting
//! component's own event table wins (`{component}_{eventName}`), falling
//! back to the global rename table on miss (stmt.rs::resolve_event_emit).
//! These tests guard the fix and the sibling event families (onSwitch /
//! onNavigate / onSelect).

/// Compiles `src` and returns the generated JS, asserting compilation
/// succeeded.
fn compile_js(src: &str) -> String {
    let result = crate::compile_omniview(src, false, false);
    assert!(
        result.success,
        "compilation should succeed: {:?}",
        result.errors
    );
    result.files.unwrap().js
}

/// Count occurrences of a substring in the generated JS.
fn count_occurrences(js: &str, needle: &str) -> usize {
    js.matches(needle).count()
}

// ═════════════════════════════════════════════════════════════════════════════
// Shape 1 — same-named events resolve to each component's own emitter
// ═════════════════════════════════════════════════════════════════════════════

/// Two components declare the same event name (`onSwitch`). Each `emit`
/// must resolve to `{own component}_{event}`, NOT to the other component's
/// prefixed emitter (bare-name table is last-wins and cannot distinguish
/// the two declarations). Covers BOTH emission syntaxes: the `emit`
/// keyword in fn bodies (EventEmitStmtIR) and the equivalent function-call
/// form `onSwitch(...)` in view handlers (CallExpr) — the pre-fix last-wins
/// table made one component's emit call the other's emitter.
#[test]
fn same_named_event_emits_resolve_to_own_component() {
    let js = compile_js(
        r#"
component TabA {
    event onSwitch: String;
    fn pressA() { emit onSwitch("a"); }
    fn pressAView() { onSwitch("a-view"); }
    view {
        <div>
            <button @onPress={pressA}>A</button>
            <button @onPress={pressAView}>A View</button>
        </div>
    }
}
component TabB {
    event onSwitch: String;
    fn pressB() { emit onSwitch("b"); }
    fn pressBView() { onSwitch("b-view"); }
    view {
        <div>
            <button @onPress={pressB}>B</button>
            <button @onPress={pressBView}>B View</button>
        </div>
    }
}
"#,
    );

    // Each emit statement resolves to its own component's prefixed emitter
    // (emit-keyword form, fn bodies).
    assert!(
        js.contains("TabA_onSwitch(\"a\");"),
        "TabA's emit should resolve to TabA_onSwitch. JS:\n{}",
        js
    );
    assert!(
        js.contains("TabB_onSwitch(\"b\");"),
        "TabB's emit should resolve to TabB_onSwitch. JS:\n{}",
        js
    );
    // Call-form emits in fn bodies resolve per-component too (the equivalent
    // `onSwitch(...)` syntax lowers to CallExpr, a different resolution path).
    assert!(
        js.contains("TabA_onSwitch(\"a-view\");"),
        "TabA's call-form emit should resolve to TabA_onSwitch. JS:\n{}",
        js
    );
    assert!(
        js.contains("TabB_onSwitch(\"b-view\");"),
        "TabB's call-form emit should resolve to TabB_onSwitch. JS:\n{}",
        js
    );
    // No cross-component leakage: neither component's emit may call the
    // other's emitter (the pre-fix last-wins table made one of them wrong).
    assert!(
        !js.contains("TabB_onSwitch(\"a\")"),
        "TabA's emit must NOT resolve to TabB_onSwitch. JS:\n{}",
        js
    );
    assert!(
        !js.contains("TabA_onSwitch(\"b\")"),
        "TabB's emit must NOT resolve to TabA_onSwitch. JS:\n{}",
        js
    );
    assert!(
        !js.contains("TabB_onSwitch(\"a-view\")"),
        "TabA's view emit must NOT resolve to TabB_onSwitch. JS:\n{}",
        js
    );
    assert!(
        !js.contains("TabA_onSwitch(\"b-view\")"),
        "TabB's view emit must NOT resolve to TabA_onSwitch. JS:\n{}",
        js
    );

    // Both prefixed emitter stubs are defined.
    assert!(
        js.contains("function TabA_onSwitch("),
        "TabA_onSwitch emitter stub should be defined. JS:\n{}",
        js
    );
    assert!(
        js.contains("function TabB_onSwitch("),
        "TabB_onSwitch emitter stub should be defined. JS:\n{}",
        js
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Shape 2 — project-level TabBar shape: handler calls + >=3 occurrences
// ═════════════════════════════════════════════════════════════════════════════

/// Mirrors the learn-omniview TabBar/App scenario (function-call form
/// `onSwitch("...")` inside `@onPress` inline handlers, child inlined into
/// the parent view). A sibling component (AboutPage) declares the SAME
/// event name so the module-wide last-wins table would point at AboutPage's
/// emitter — the exact collision that broke all tab switching. The
/// generated handler bodies must call `TabBar_onSwitch`, never
/// `AboutPage_onSwitch`, and `TabBar_onSwitch` must appear >= 3 times
/// (definition + self-reference + calls). The parent's `switchTab` binding
/// must resolve to `App_switchTab`.
#[test]
fn tabbar_switch_emit_resolves_and_occurs_three_plus() {
    let js = compile_js(
        r#"
component TabBar {
    prop activeTab: String = "overview";
    event onSwitch: String;
    fn isActive(tab: String) -> bool { return activeTab == tab; }
    view {
        <div>
            <button @onPress={() => { onSwitch("overview"); }}>Overview</button>
            <button @onPress={() => { onSwitch("ui"); }}>UI</button>
            <button @onPress={() => { emit onSwitch("emit-overview"); }}>Emit Overview</button>
        </div>
    }
}
component AboutPage {
    event onSwitch: String;
    view {
        <div>
            <button @onPress={() => { onSwitch("about"); }}>Go to About</button>
            <button @onPress={() => { emit onSwitch("emit-about"); }}>Emit About</button>
        </div>
    }
}
component App {
    signal activeTab: String = "overview";
    fn switchTab(tab: String) { activeTab = tab; }
    view {
        <div>
            <TabBar activeTab={activeTab} @onSwitch={switchTab} />
            <AboutPage />
        </div>
    }
}
"#,
    );

    // Handler bodies call the component's own prefixed emitter.
    assert!(
        js.contains("TabBar_onSwitch(\"overview\");"),
        "TabBar overview handler should call TabBar_onSwitch. JS:\n{}",
        js
    );
    assert!(
        js.contains("TabBar_onSwitch(\"ui\");"),
        "TabBar ui handler should call TabBar_onSwitch. JS:\n{}",
        js
    );
    // emit-keyword form in the inlined view handler (EventEmitStmtIR path)
    // resolves per-component too.
    assert!(
        js.contains("TabBar_onSwitch(\"emit-overview\");"),
        "TabBar emit-keyword handler should call TabBar_onSwitch. JS:\n{}",
        js
    );
    // AboutPage's own handler resolves to AboutPage's emitter — and TabBar's
    // handlers must NEVER call it (the pre-fix bug: all 9 TabBar buttons
    // dispatched to AboutPage_onSwitch, so App_switchTab never fired).
    assert!(
        js.contains("AboutPage_onSwitch(\"about\");"),
        "AboutPage handler should call AboutPage_onSwitch. JS:\n{}",
        js
    );
    assert!(
        js.contains("AboutPage_onSwitch(\"emit-about\");"),
        "AboutPage emit-keyword handler should call AboutPage_onSwitch. JS:\n{}",
        js
    );
    assert!(
        !js.contains("AboutPage_onSwitch(\"overview\")"),
        "TabBar's overview handler must NOT resolve to AboutPage_onSwitch. JS:\n{}",
        js
    );
    assert!(
        !js.contains("AboutPage_onSwitch(\"ui\")"),
        "TabBar's ui handler must NOT resolve to AboutPage_onSwitch. JS:\n{}",
        js
    );
    assert!(
        !js.contains("AboutPage_onSwitch(\"emit-overview\")"),
        "TabBar's emit-keyword handler must NOT resolve to AboutPage_onSwitch. JS:\n{}",
        js
    );
    assert!(
        !js.contains("TabBar_onSwitch(\"about\")"),
        "AboutPage's handler must NOT resolve to TabBar_onSwitch. JS:\n{}",
        js
    );

    // >= 3 occurrences: function definition + self-reference + handler calls.
    let count = count_occurrences(&js, "TabBar_onSwitch");
    assert!(
        count >= 3,
        "TabBar_onSwitch should appear >= 3 times (definition + self-reference + calls), got {}: JS:\n{}",
        count,
        js
    );

    // The parent's event binding resolves to the parent's own fn.
    assert!(
        js.contains("App_switchTab"),
        "App's switchTab fn should be referenced by the @onSwitch binding. JS:\n{}",
        js
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Shape 3 — sibling event families: onSelect must never resolve to the
// user-zone CodeFlowMidChain emitter; onNavigate must stay per-component
// ═════════════════════════════════════════════════════════════════════════════

/// AdvancedTypesDemo (FnFormatter) and CodeFlowMidChain both declare
/// `onSelect`. AdvancedTypesDemo's emit must resolve to
/// `AdvancedTypesDemo_onSelect` (its own emitter) and NEVER to
/// `CodeFlowMidChain_onSelect` — user-zone pollution guard. Both
/// components are inlined into a parent view so their emit statements are
/// resolved after ALL components' event tables are populated (the
/// last-wins collision window — fn-body emits during a component's own
/// generation pass resolve by generation-order locality and would not
/// discriminate the fix).
#[test]
fn onselect_emit_never_resolves_to_codeflowmidchain() {
    let js = compile_js(
        r#"
component FnFormatter {
    event onSelect: String;
    signal clicks: i32 = 0;
    fn pickItem() {
        clicks = clicks + 1;
        onSelect("item #" + clicks.to_string());
    }
    view {
        <div>
            <button @onPress={pickItem}>Pick</button>
            <button @onPress={() => { onSelect("item #view"); }}>Pick View</button>
        </div>
    }
}
component CodeFlowMidChain {
    event onSelect: Void;
    view {
        <button @onPress={() => { emit onSelect(); }}>Fire</button>
    }
}
component App {
    view {
        <div>
            <FnFormatter />
            <CodeFlowMidChain />
        </div>
    }
}
"#,
    );

    assert!(
        js.contains("FnFormatter_onSelect(\"item #\" + String(FnFormatter_clicks.value))"),
        "FnFormatter's fn-body emit should resolve to FnFormatter_onSelect. JS:\n{}",
        js
    );
    assert!(
        js.contains("FnFormatter_onSelect(\"item #view\");"),
        "FnFormatter's view emit should resolve to FnFormatter_onSelect. JS:\n{}",
        js
    );
    assert!(
        js.contains("CodeFlowMidChain_onSelect();"),
        "CodeFlowMidChain's emit should resolve to CodeFlowMidChain_onSelect. JS:\n{}",
        js
    );
    // User-zone pollution guard: FnFormatter's emits must NEVER resolve to
    // CodeFlowMidChain's emitter.
    assert!(
        !js.contains("CodeFlowMidChain_onSelect(\"item #\")"),
        "FnFormatter's emit must NOT resolve to CodeFlowMidChain_onSelect. JS:\n{}",
        js
    );
}

/// onNavigate is the event family behind browser test 02 ("CapabilityCard
/// Explore navigates to tab"). Same-named onNavigate declarations on two
/// components must each resolve to their own prefixed emitter — in both the
/// emit-keyword form and the call form, and both inlined into a parent view
/// (the last-wins collision window).
#[test]
fn onavigate_same_named_events_resolve_to_own_component() {
    let js = compile_js(
        r#"
component NavCard {
    event onNavigate: String;
    fn go() { emit onNavigate("tab"); }
    view {
        <div>
            <button @onPress={go}>Go</button>
            <button @onPress={() => { onNavigate("tab-view"); }}>Go View</button>
        </div>
    }
}
component OtherCard {
    event onNavigate: String;
    fn goOther() { emit onNavigate("elsewhere"); }
    view {
        <button @onPress={() => { onNavigate("other-view"); }}>Go Other View</button>
    }
}
component App {
    view {
        <div>
            <NavCard />
            <OtherCard />
        </div>
    }
}
"#,
    );

    assert!(
        js.contains("NavCard_onNavigate(\"tab\");"),
        "NavCard's emit should resolve to NavCard_onNavigate. JS:\n{}",
        js
    );
    assert!(
        js.contains("OtherCard_onNavigate(\"elsewhere\");"),
        "OtherCard's emit should resolve to OtherCard_onNavigate. JS:\n{}",
        js
    );
    assert!(
        js.contains("NavCard_onNavigate(\"tab-view\");"),
        "NavCard's view emit should resolve to NavCard_onNavigate. JS:\n{}",
        js
    );
    assert!(
        js.contains("OtherCard_onNavigate(\"other-view\");"),
        "OtherCard's view emit should resolve to OtherCard_onNavigate. JS:\n{}",
        js
    );
    assert!(
        !js.contains("OtherCard_onNavigate(\"tab\")"),
        "NavCard's emit must NOT resolve to OtherCard_onNavigate. JS:\n{}",
        js
    );
    assert!(
        !js.contains("OtherCard_onNavigate(\"tab-view\")"),
        "NavCard's view emit must NOT resolve to OtherCard_onNavigate. JS:\n{}",
        js
    );
    assert!(
        !js.contains("NavCard_onNavigate(\"other-view\")"),
        "OtherCard's view emit must NOT resolve to NavCard_onNavigate. JS:\n{}",
        js
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Shape 4 — emit inside a BlockExpr (deferral #78)
// ═════════════════════════════════════════════════════════════════════════════

/// Deferral #78: `stmt_to_js_inline` had no `EventEmitStmtIR` arm, so an
/// `emit` statement inside a BlockExpr degraded to a `/* unhandled stmt */`
/// comment + warnings entry instead of emitting the event. `emit` is legal
/// in block expressions at every layer — the parser accepts
/// `TokenKind::Emit` inside `parse_block` (stmt.rs::parse_stmt), the
/// semantic layer walks `Stmt::EventEmit` without rejection, and a
/// function-form computed's body is preserved as `Expr::Block` →
/// `ExprIR::BlockExpr` with ALL statements (component.rs::parse_computed_decl)
/// — so the codegen catch-all was the only silent gap. The emit must resolve
/// component-scoped like the fn-body path (resolve_event_emit).
#[test]
fn emit_in_computed_body_blockexpr_emits_event() {
    let js = compile_js(
        r#"
component Badge {
    event onReady: String;
    computed label() -> String {
        emit onReady("computed-ready");
        return "ready";
    }
    view {
        <div>{label()}</div>
    }
}
"#,
    );

    // The emit statement inside the computed's block body must become a real
    // component-scoped event call (the deferred `/* unhandled stmt */` gap).
    assert!(
        js.contains("Badge_onReady(\"computed-ready\");"),
        "emit inside a BlockExpr should emit the real event call. JS:\n{}",
        js
    );
    assert!(
        js.contains("function Badge_onReady("),
        "the Badge_onReady emitter stub should be defined. JS:\n{}",
        js
    );
    assert!(
        !js.contains("unhandled stmt"),
        "emit inside a BlockExpr must not degrade to a placeholder comment. JS:\n{}",
        js
    );
}

/// Plain block expression in expression position (`let v = { emit onPing(); 42 }`)
/// — the `{ ... }` initializer is `Expr::Block` → `ExprIR::BlockExpr` with the
/// emit statement in `statements` and `42` as the tail. (A bare `{ ... };`
/// STATEMENT lowers to `BlockStmtIR` and takes the working statement path —
/// lower/stmt.rs:56 — so only an expression-position block exercises
/// `stmt_to_js_inline`.)
#[test]
fn emit_in_plain_blockexpr_in_fn_body_emits_event() {
    let js = compile_js(
        r#"
component Badge {
    event onPing: Void;
    fn fire() {
        let v = { emit onPing(); 42 };
    }
    view {
        <button @onPress={fire}>Fire</button>
    }
}
"#,
    );

    assert!(
        js.contains("Badge_onPing();"),
        "emit inside a plain block expression should emit the event call. JS:\n{}",
        js
    );
    assert!(
        !js.contains("unhandled stmt"),
        "emit inside a plain block expression must not degrade to a placeholder comment. JS:\n{}",
        js
    );
}
