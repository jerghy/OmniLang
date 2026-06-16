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
    let semantics = semantic::analyze(&program).expect("semantic analysis failed");
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
