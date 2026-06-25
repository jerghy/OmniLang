//! Tests: CSS custom property (--name) parsing in style blocks.
//! and dynamic inline style codegen for CSS custom properties.

use omni_compiler::codegen::omniview;
use omni_compiler::ir;
use omni_compiler::lexer::tokenize;
use omni_compiler::parser::parse;
use omni_compiler::semantic;

/// Helper: compile .omni source to OmniView output (normal mode).
fn compile_omniview(source: &str) -> omniview::OmniViewOutput {
    let tokens = tokenize(source);
    let program = parse(&tokens, source).expect("parse failed");
    let semantics = semantic::analyze(&program);
    let ir_program = ir::lower(&program, &semantics);
    omniview::generate(&ir_program, false)
}

#[test]
fn test_parse_css_custom_property_in_style_block() {
    let src = r##"
component Test {
    style {
        .foo {
            --primary-color: #333;
            --spacing-md: 16px;
            color: var(--primary-color);
            padding: var(--spacing-md);
        }
    }
    view { <div class: foo/> }
}
"##;
    let tokens = tokenize(src);
    let result = parse(&tokens, src);
    assert!(result.is_ok(), "Should parse CSS custom properties: {:?}", result.err());
}

#[test]
fn test_inline_style_custom_property_static() {
    let src = r##"
component InlineStatic {
    view {
        <div style: { --bg: "#eee" }>Hello</div>
    }
}
"##;
    let result = compile_omniview(src);
    assert!(
        result.html.contains("--bg:#eee") || result.html.contains("--bg: #eee"),
        "Static -- prop must appear in style attribute. HTML: {}",
        result.html
    );
}

#[test]
fn test_inline_style_custom_property_dynamic() {
    let src = r##"
component InlineTest {
    signal color: String = "#ff0000";
    view {
        <div style: { --custom-color: color, padding: "16px" }>Hello</div>
    }
}
"##;
    let result = compile_omniview(src);
    assert!(
        result.js.contains("setProperty(\"--custom-color\"")
            || result.js.contains("setProperty('--custom-color'"),
        "Dynamic -- prop must use setProperty. JS: {}",
        result.js
    );
}
