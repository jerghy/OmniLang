//! OmniView codegen tests.

use omni_compiler::codegen::omniview;
use omni_compiler::ir;
use omni_compiler::lexer;
use omni_compiler::parser;
use omni_compiler::semantic;

/// Helper: compile .omni source to IR, then generate OmniView output.
fn compile_omniview(source: &str) -> omniview::OmniViewOutput {
    let tokens = lexer::tokenize(source);
    let program = parser::parse(&tokens).expect("parse failed");
    let semantics = semantic::analyze(&program);
    let ir_program = ir::lower(&program, &semantics);
    omniview::generate(&ir_program)
}

#[test]
fn test_counter_generates_signal_declaration() {
    let src = "component Counter { signal count: i32 = 0 view { <br/> } }";
    let output = compile_omniview(src);
    assert!(output.js.contains("_signal(\"count\", 0)"));
}

#[test]
fn test_counter_generates_function() {
    let src = "component Counter { signal count: i32 = 0 fn inc() { count = count + 1 } view { <br/> } }";
    let output = compile_omniview(src);
    assert!(output.js.contains("function inc()"), "JS should contain function inc()");
}

#[test]
fn test_counter_has_signal_runtime() {
    let src = "component App { view { <br/> } }";
    let output = compile_omniview(src);
    assert!(output.js.contains("function _signal("));
    assert!(output.js.contains("function _effect("));
    assert!(output.js.contains("function _el("));
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
    assert!(output.js.contains("addEventListener(\"click\", handle)"));
}

#[test]
fn test_mutation_generated_for_assignment() {
    let src = "component App { signal x: i32 = 0 fn go() { x = x + 5 } view { <br/> } }";
    let output = compile_omniview(src);
    assert!(output.js.contains("x.set(x.value + 5)"));
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
    assert!(output.js.contains("_signal(\"count\", 42)"));
}

#[test]
fn test_signal_initial_value_string() {
    let src = "component App { signal name: String = \"hello\" view { <br/> } }";
    let output = compile_omniview(src);
    assert!(output.js.contains("_signal(\"name\", \"hello\")"));
}

#[test]
fn test_onpress_without_at_generates_event() {
    let src = "component App { fn click() { } view { <Button onPress={click}>Go</Button> } }";
    let output = compile_omniview(src);
    assert!(output.js.contains("addEventListener(\"click\", click)"));
}

// ══════════════════════════════════════════════════════
// @if / @else-if / @else directive codegen
// ══════════════════════════════════════════════════════

#[test]
fn test_if_directive_generates_conditional_effect() {
    let src = "component App { signal show: bool = true view { @if show { <Text>Visible</Text> } } }";
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
    assert!(output.js.contains("show.value"));
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
