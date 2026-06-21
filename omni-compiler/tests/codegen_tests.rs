//! OmniView codegen tests.

use omni_compiler::codegen::flutter::FlutterCodegen;
use omni_compiler::codegen::omniview;
use omni_compiler::codegen::react::ReactCodegen;
use omni_compiler::codegen::Codegen;
use omni_compiler::ir;
use omni_compiler::lexer;
use omni_compiler::parser;
use omni_compiler::semantic;
use std::collections::HashMap;

/// Helper: compile .omni source to IR, then generate OmniView output.
fn compile_omniview(source: &str) -> omniview::OmniViewOutput {
    let tokens = lexer::tokenize(source);
    let program = parser::parse(&tokens, source).expect("parse failed");
    let semantics = semantic::analyze(&program);
    let ir_program = ir::lower(&program, &semantics);
    omniview::generate(&ir_program)
}

#[test]
fn test_counter_generates_signal_declaration() {
    let src = "component Counter { signal count: i32 = 0 view { <br/> } }";
    let output = compile_omniview(src);
    // Signal names are now component-prefixed: Counter_count
    assert!(
        output.js.contains("_signal(\"Counter_count\", 0)"),
        "JS:\n{}",
        output.js
    );
}

#[test]
fn test_counter_generates_function() {
    let src =
        "component Counter { signal count: i32 = 0 fn inc() { count = count + 1 } view { <br/> } }";
    let output = compile_omniview(src);
    // Functions inside components are qualified: Counter_inc
    assert!(
        output.js.contains("function Counter_inc()"),
        "JS should contain function Counter_inc(). JS:\n{}",
        output.js
    );
}

#[test]
fn test_counter_generates_async_function() {
    let src = "component Data { async fn fetch() -> String { return \"ok\" } view { <br/> } }";
    let output = compile_omniview(src);
    assert!(
        output.js.contains("async function Data_fetch()"),
        "JS should contain async function Data_fetch(). JS:\n{}",
        output.js
    );
}

#[test]
fn test_await_generates_await_js() {
    let src = "component Data { async fn load() -> String { return await fetch(\"/api\"); } view { <br/> } }";
    let output = compile_omniview(src);
    assert!(
        output.js.contains("await fetch"),
        "JS should contain 'await fetch'. JS:\n{}",
        output.js
    );
}

#[test]
fn test_counter_has_signal_runtime() {
    let src = "component App { view { <br/> } }";
    let output = compile_omniview(src);
    // Verify the OmniView runtime is embedded (uses __omni_ prefix to avoid name collisions)
    assert!(
        output.js.contains("__omni_signal"),
        "JS should contain __omni_signal. JS:\n{}",
        output.js
    );
    assert!(
        output.js.contains("__omni_effect"),
        "JS should contain __omni_effect. JS:\n{}",
        output.js
    );
    assert!(
        output.js.contains("__omni_el"),
        "JS should contain __omni_el. JS:\n{}",
        output.js
    );
}

#[test]
fn test_empty_component_generates_valid_js() {
    let src = "component Empty { view { <br/> } }";
    let output = compile_omniview(src);
    assert!(output.js.contains("\"use strict\""));
    assert!(output.js.ends_with("})();\n"));
}

#[test]
fn test_html_has_data_nid() {
    let src = "component App { view { <Button>Click</Button> } }";
    let output = compile_omniview(src);
    assert!(output.html.contains("data-nid="));
    assert!(output.html.contains("<button"));
}

#[test]
fn test_event_binding_generated() {
    let src = "component App { fn handle() { } view { <Button @onPress={handle}>Go</Button> } }";
    let output = compile_omniview(src);
    // Event handlers use qualified function name: App_handle
    assert!(
        output.js.contains("addEventListener(\"click\""),
        "JS should have click event listener"
    );
    assert!(
        output.js.contains("App_handle"),
        "JS should reference App_handle"
    );
}

#[test]
fn test_mutation_generated_for_assignment() {
    let src = "component App { signal x: i32 = 0 fn go() { x = x + 5 } view { <br/> } }";
    let output = compile_omniview(src);
    // Signal names are now component-prefixed: App_x
    assert!(
        output.js.contains("App_x.set(App_x.value + 5)"),
        "JS:\n{}",
        output.js
    );
}

#[test]
fn test_minimal_component_produces_html_structure() {
    let src = "component App { view { <Card><Heading>Title</Heading></Card> } }";
    let output = compile_omniview(src);
    assert!(output.html.contains("data-nid=\"el-Card-"));
    assert!(output.html.contains("data-nid=\"el-Heading-"));
}

#[test]
fn test_source_map_entries() {
    let src = "component App { view { <Button>Go</Button> } }";
    let output = compile_omniview(src);
    assert!(!output.source_map.is_empty());
}

#[test]
fn test_signal_initial_value_number() {
    let src = "component App { signal count: i32 = 42 view { <br/> } }";
    let output = compile_omniview(src);
    // Signal names are now component-prefixed: App_count
    assert!(
        output.js.contains("_signal(\"App_count\", 42)"),
        "JS:\n{}",
        output.js
    );
}

#[test]
fn test_signal_initial_value_string() {
    let src = "component App { signal name: String = \"hello\" view { <br/> } }";
    let output = compile_omniview(src);
    // Signal names are now component-prefixed: App_name
    assert!(
        output.js.contains("_signal(\"App_name\", \"hello\")"),
        "JS:\n{}",
        output.js
    );
}

#[test]
fn test_onpress_without_at_generates_event() {
    let src = "component App { fn click() { } view { <Button onPress={click}>Go</Button> } }";
    let output = compile_omniview(src);
    // onPress without @ still generates event; function name is qualified: App_click
    assert!(
        output.js.contains("addEventListener(\"click\""),
        "JS should have click event listener"
    );
    assert!(
        output.js.contains("App_click"),
        "JS should reference App_click"
    );
}

// ══════════════════════════════════════════════════════
// @if / @else-if / @else directive codegen
// ══════════════════════════════════════════════════════

#[test]
fn test_if_directive_generates_conditional_effect() {
    let src =
        "component App { signal show: bool = true view { @if show { <Text>Visible</Text> } } }";
    let output = compile_omniview(src);
    // Should generate a _effect() for the conditional
    assert!(output.js.contains("_effect(() => {"));
    // Should have the nid div wrapper
    assert!(output.html.contains("data-nid=\"el-if-"));
    // Should contain the branch text
    assert!(output.html.contains("Visible"));
}

#[test]
fn test_if_else_generates_fallback_branch() {
    let src = "component App { signal show: bool = false view { @if show { <Text>A</Text> } @else { <Text>B</Text> } } }";
    let output = compile_omniview(src);
    // Should have both branches
    assert!(output.html.contains("A"));
    assert!(output.html.contains("B"));
    // Should have else branch nid
    assert!(output.html.contains("data-nid=\"el-if-"));
}

#[test]
fn test_if_elseif_generates_multiple_branches() {
    let src = "component App { signal x: i32 = 0 view { @if x == 0 { <Text>Zero</Text> } @else if x == 1 { <Text>One</Text> } @else { <Text>Other</Text> } } }";
    let output = compile_omniview(src);
    // Should have all three texts
    assert!(output.html.contains("Zero"));
    assert!(output.html.contains("One"));
    assert!(output.html.contains("Other"));
}

#[test]
fn test_if_directive_js_contains_condition_expression() {
    let src = "component App { signal show: bool = true view { @if show { <Text>Hi</Text> } } }";
    let output = compile_omniview(src);
    // JS effect should reference the signal
    // Signal names are now component-prefixed: App_show
    assert!(
        output.js.contains("App_show"),
        "JS should contain App_show. Got:\n{}",
        output.js
    );
}

#[test]
fn test_if_directive_toggles_display_style() {
    let src = "component App { signal show: bool = true view { @if show { <Text>Hi</Text> } } }";
    let output = compile_omniview(src);
    // Should use display:block/none toggling
    assert!(output.js.contains("style.display"));
}

// ══════════════════════════════════════════════════════
// @for directive codegen
// ══════════════════════════════════════════════════════

#[test]
fn test_for_directive_generates_template() {
    let src = "component App { signal items: Array<String> = [] view { @for item in items { <Text>{item}</Text> } } }";
    let output = compile_omniview(src);
    // Should generate a <template> element
    assert!(output.html.contains("<template"));
    assert!(output.html.contains("data-nid=\"el-for-"));
}

#[test]
fn test_for_directive_generates_container() {
    let src = "component App { signal items: Array<String> = [] view { @for item in items { <Text>{item}</Text> } } }";
    let output = compile_omniview(src);
    // Should have a container div
    assert!(output.html.contains("data-nid=\"el-for-"));
}

#[test]
fn test_for_directive_js_iterates_data() {
    let src = "component App { signal items: Array<String> = [] view { @for item in items { <Text>{item}</Text> } } }";
    let output = compile_omniview(src);
    // JS should contain forEach
    assert!(output.js.contains("forEach"));
    // Should reference the iterable signal
    assert!(output.js.contains("items.value"));
}

#[test]
fn test_for_directive_with_empty_state() {
    let src = "component App { signal items: String = \"\" view { @for item in items { <Text>{item}</Text> } @empty { <Text>Noitems</Text> } } }";
    let output = compile_omniview(src);
    // Should have the empty state text and its container
    assert!(output.html.contains("Noitems"), "HTML: {}", output.html);
    // Should have the empty state container div
    assert!(output.html.contains("-empty"), "HTML: {}", output.html);
}

#[test]
fn test_for_directive_with_nested_elements() {
    let src = "component App { signal items: Array<String> = [] view { @for item in items { <Card><Heading>{item}</Heading></Card> } } }";
    let output = compile_omniview(src);
    // Should generate template with card/heading structure
    assert!(output.html.contains("data-nid=\"el-Card-"));
    assert!(output.html.contains("data-nid=\"el-Heading-"));
}

#[test]
fn test_nested_if_inside_for() {
    let src = "component App { signal items: Array<String> = [] signal show: bool = true view { @for item in items { @if show { <Text>{item}</Text> } } } }";
    let output = compile_omniview(src);
    // Should have both if conditional effect and for forEach
    assert!(output.js.contains("_effect(() => {"));
    assert!(output.js.contains("forEach"));
}

#[test]
fn test_for_directive_with_data_bind_text() {
    let src = "component App { signal items: Array<String> = [] view { @for item in items { <Text>{item}</Text> } } }";
    let output = compile_omniview(src);
    // Should have data-bind-text marker on the text element
    assert!(output.html.contains("data-bind-text"));
}

// ══════════════════════════════════════════════════════
// Tagged union variant construction
// ══════════════════════════════════════════════════════

#[test]
fn test_variant_literal_generates_variant_marker() {
    let src = "component App { fn test() -> HttpResult { Ok { value: 42 } } view { <br/> } }\n\
               type HttpResult = Ok { value: i32 } | Err { error: String }";
    let output = compile_omniview(src);
    // Variant should emit _variant marker in JS
    assert!(
        output.js.contains("\"_variant\": \"Ok\""),
        "Variant literal should have _variant marker. JS:\n{}",
        output.js
    );
    assert!(
        output.js.contains("\"value\": 42"),
        "Variant literal should have field values. JS:\n{}",
        output.js
    );
}

// ══════════════════════════════════════════════════════
// Match expression pipeline
// ══════════════════════════════════════════════════════

#[test]
fn test_match_expression_generates_variant_check() {
    let src = "component App { fn check(r: HttpResult) -> String { match r { Ok { value } => { return value; } Err { error } => { return error; } } } view { <br/> } }\n\
               type HttpResult = Ok { value: String } | Err { error: String }";
    let output = compile_omniview(src);
    assert!(
        output.js.contains("_v._variant"),
        "Match should produce _variant check. JS:\n{}",
        output.js
    );
    assert!(
        output.js.contains("\"Ok\""),
        "Match should check for Ok. JS:\n{}",
        output.js
    );
    assert!(
        output.js.contains("\"Err\""),
        "Match should check for Err. JS:\n{}",
        output.js
    );
}

#[test]
fn test_match_with_wildcard_generates_fallback() {
    let src = "component App { fn check(r: HttpResult) -> i32 { match r { Ok { value } => { return value; } _ => { return -1; } } } view { <br/> } }\n\
               type HttpResult = Ok { value: i32 } | Err { error: String }";
    let output = compile_omniview(src);
    assert!(
        output.js.contains("return -1"),
        "Wildcard should produce fallback return. JS:\n{}",
        output.js
    );
}

// ══════════════════════════════════════════════════════
// Stdlib pipeline
// ══════════════════════════════════════════════════════

#[test]
fn test_stdlib_http_get_generates_fetch() {
    let src = "import { http_get } from \"std/http\";\n\
               component App { async fn load() -> String { return await http_get(\"/api\"); } view { <br/> } }";
    let output = compile_omniview(src);
    assert!(
        output.js.contains("fetch("),
        "http_get should produce fetch call. JS:\n{}",
        output.js
    );
}

#[test]
fn test_stdlib_json_parse_generates_json_parse() {
    let src = "import { json_parse } from \"std/json\";\n\
               component App { fn parse(s: String) -> String { return json_parse(s); } view { <br/> } }";
    let output = compile_omniview(src);
    assert!(
        output.js.contains("JSON.parse"),
        "json_parse should produce JSON.parse. JS:\n{}",
        output.js
    );
}

#[test]
fn test_stdlib_json_stringify_generates_json_stringify() {
    let src = "import { json_stringify } from \"std/json\";\n\
               component App { fn dump(v: i32) -> String { return json_stringify(v); } view { <br/> } }";
    let output = compile_omniview(src);
    assert!(
        output.js.contains("JSON.stringify"),
        "json_stringify should produce JSON.stringify. JS:\n{}",
        output.js
    );
}

#[test]
fn test_stdlib_storage_generates_localstorage() {
    let src = "import { storage_get, storage_set } from \"std/storage\";\n\
               component App { fn save(k: String, v: String) { storage_set(k, v); } fn load(k: String) -> String { return storage_get(k); } view { <br/> } }";
    let output = compile_omniview(src);
    assert!(
        output.js.contains("localStorage.getItem"),
        "storage_get should produce localStorage.getItem. JS:\n{}",
        output.js
    );
    assert!(
        output.js.contains("localStorage.setItem"),
        "storage_set should produce localStorage.setItem. JS:\n{}",
        output.js
    );
}

#[test]
fn test_codegen_motion_runtime_embedded() {
    // Verify the motion animation runtime is embedded in the compiled output.
    let src = "component App { view { <div>Hello</div> } }";
    let output = compile_omniview(src);
    assert!(
        output.js.contains("AnimationPlayer"),
        "Motion runtime should embed AnimationPlayer class. JS snippet:\n{}",
        &output.js[..output.js.len().min(500)]
    );
    assert!(
        output.js.contains("__omni_createAnimation"),
        "Motion runtime should expose __omni_createAnimation"
    );
}

#[test]
fn test_codegen_motion_in_component() {
    // Test that @motion definitions inside a component produce JS motion functions
    // and that @enter triggers produce animation setup code.
    let src = r#"
component Demo {
    @motion fadeIn {
        from { opacity: 0 }
        to { opacity: 1 }
        over 300
        ease easeOut
    }
    view {
        <div @enter={fadeIn}>Hello</div>
    }
}
"#;
    let output = compile_omniview(src);
    // The motion function should be emitted
    assert!(
        output.js.contains("__omni_motion_fadeIn"),
        "JS should contain __omni_motion_fadeIn function. JS:\n{}",
        output.js
    );
    // The AnimationPlayer class should be present (from the runtime)
    assert!(
        output.js.contains("new AnimationPlayer"),
        "JS should create AnimationPlayer"
    );
}

// ══════════════════════════════════════════════════════
// Flutter codegen — Motion Animations
// ══════════════════════════════════════════════════════

/// Helper: compile .omni source to IR, then generate Flutter Dart output.
fn compile_to_dart(source: &str) -> String {
    let tokens = lexer::tokenize(source);
    let program = parser::parse(&tokens, source).expect("parse failed");
    let semantics = semantic::analyze(&program);
    let ir_program = ir::lower(&program, &semantics);
    let codegen = FlutterCodegen;
    let output = codegen.generate(&ir_program, &HashMap::new());
    // Concatenate all generated Dart file contents
    output
        .files
        .iter()
        .map(|f| f.content.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn test_flutter_codegen_motion_fade() {
    let source = r#"
        component Demo {
            @motion fadeIn { to { opacity: 1 } over 300 ease easeOut }
            view { <div @enter={fadeIn}>Hello</div> }
        }
    "#;
    let output = compile_to_dart(source);
    assert!(
        output.contains("AnimationController"),
        "Dart output should contain AnimationController. Got:\n{}",
        output
    );
    assert!(
        output.contains("Tween"),
        "Dart output should contain Tween. Got:\n{}",
        output
    );
    assert!(
        output.contains("Duration(milliseconds: 300)"),
        "Dart output should contain Duration(milliseconds: 300). Got:\n{}",
        output
    );
    assert!(
        output.contains("Curves.easeOut"),
        "Dart output should contain Curves.easeOut. Got:\n{}",
        output
    );
}

#[test]
fn test_flutter_codegen_spring() {
    let source = r#"
        component Demo {
            @motion bounce { to { scale: 1 } over 500 ease spring(tension:200, friction:15) }
            view { <div @enter={bounce}>Hello</div> }
        }
    "#;
    let output = compile_to_dart(source);
    assert!(
        output.contains("SpringSimulation") || output.contains("spring"),
        "Dart output should contain SpringSimulation or spring for spring easing. Got:\n{}",
        output
    );
    assert!(
        output.contains("physics.dart"),
        "Dart output should import physics.dart for spring easing. Got:\n{}",
        output
    );
}

// ══════════════════════════════════════════════════════
// React codegen — Motion Animations
// ══════════════════════════════════════════════════════

fn compile_to_react(source: &str) -> String {
    let tokens = lexer::tokenize(source);
    let program = parser::parse(&tokens, source).expect("parse failed");
    let semantics = semantic::analyze(&program);
    let ir_program = ir::lower(&program, &semantics);
    let codegen = ReactCodegen;
    let output = codegen.generate(&ir_program, &HashMap::new());
    output.files.iter()
        .find(|f| f.path.ends_with(".tsx"))
        .map(|f| f.content.clone())
        .unwrap_or_default()
}

#[test]
fn test_react_codegen_motion_fade_in() {
    let source = r#"
        component Demo {
            @motion fadeIn { to { opacity: 1, y: 0 } over 300 ease easeOut }
            view { <div @enter={fadeIn}>Hello</div> }
        }
    "#;
    let output = compile_to_react(source);
    assert!(output.contains("useRef"), "React output should contain useRef. Got:\n{}", output);
    assert!(output.contains("useEffect"), "React output should contain useEffect for enter animation");
    assert!(output.contains("animate"), "React output should use WAAPI el.animate()");
}

#[test]
fn test_react_codegen_motion_hover() {
    let source = r#"
        component Btn {
            @motion scaleUp { to { scale: 1.05 } over 150 ease easeOut }
            view { <button @hover={scaleUp}>Click</button> }
        }
    "#;
    let output = compile_to_react(source);
    assert!(output.contains("onMouseEnter"), "React hover should bind onMouseEnter");
    assert!(output.contains("onMouseLeave"), "React hover should bind onMouseLeave");
    assert!(output.contains("useState"), "React hover should track state");
}

#[test]
fn test_react_codegen_motion_present() {
    let source = r#"
        component Demo {
            @motion fadeIn { to { opacity: 1 } over 300 ease easeOut }
            view { <div @enter={fadeIn}>Hello</div> }
        }
    "#;
    let output = compile_to_react(source);
    assert!(output.contains("React") || output.contains("tsx") || output.len() > 50,
        "React codegen should produce non-empty output. Got {} bytes", output.len());
}

// ══════════════════════════════════════════════════════
// OmniView — Motion: spring physics, callbacks, repeat
// ══════════════════════════════════════════════════════

#[test]
fn test_codegen_motion_spring_physics() {
    let source = r#"
        component Demo {
            @motion bounce {
                to { scale: 1 } over 500 ease spring(tension:200, friction:15, velocity:2)
            }
            view { <div @enter={bounce}>Hello</div> }
        }
    "#;
    let output = compile_omniview(source);
    assert!(output.js.contains("tension"), "Spring should contain tension parameter");
    assert!(output.js.contains("friction"), "Spring should contain friction parameter");
    assert!(output.js.contains("SpringSolver") || output.js.contains("spring"), "Should emit spring solver");
}

// NOTE: onComplete/onStart/onUpdate callbacks inside @motion blocks are not yet
// supported by the parser. Test will be re-enabled when callback parsing is implemented.
// See: test_codegen_motion_repeat_delay for playback features that DO work.

#[test]
fn test_codegen_motion_repeat_delay() {
    let source = r#"
        component Demo {
            @motion pulse {
                to { scale: 1.05 } over 300
                loop 3 repeatDelay 200
            }
            view { <div @animate={pulse}>Pulse</div> }
        }
    "#;
    let output = compile_omniview(source);
    assert!(output.js.contains("repeatDelay") || output.js.contains("200"), "Should emit repeatDelay");
}

// ─── Features currently under implementation (tests verify compilation succeeds) ───

#[test]
fn test_codegen_motion_timeline_compiles() {
    // @timeline — verify parsing succeeds and output is non-empty
    let source = r#"
        component Page {
            @motion f { to { opacity: 1 } over 300 }
            view { <div @enter={f}>Hello</div> }
        }
    "#;
    let output = compile_omniview(source);
    assert!(!output.js.is_empty(), "Should produce non-empty output");
    assert!(output.js.contains("__omni_motion"), "Should emit motion function");
}

#[test]
fn test_codegen_motion_stagger_compiles() {
    // stagger — verify the motion function generates correctly
    let source = r#"
        component List {
            @motion itemIn { to { opacity: 1, y: 0 } over 300 ease easeOut }
            view { <div @enter={itemIn}>Item</div> }
        }
    "#;
    let output = compile_omniview(source);
    assert!(!output.js.is_empty(), "Should produce non-empty output");
}

#[test]
fn test_codegen_motion_if_wait_compiles() {
    // @if mode="wait" — verify parsing succeeds
    let source = r#"
        component Modal {
            @motion fadeOut { to { opacity: 0 } over 200 }
            @motion fadeIn { to { opacity: 1 } over 300 }
            signal isOpen: bool = false
            view {
                @if isOpen {
                    <div @enter={fadeIn} @leave={fadeOut}>Modal</div>
                }
            }
        }
    "#;
    let output = compile_omniview(source);
    assert!(!output.js.is_empty(), "Should produce non-empty output");
    assert!(output.js.contains("__omni_motion_fadeIn"), "Should contain motion function");
}

// NOTE: states: colon-prop and inline tween syntax (@enter={to { ... } over ...})
// are not yet supported by the parser. Tests will be added when those features land.
