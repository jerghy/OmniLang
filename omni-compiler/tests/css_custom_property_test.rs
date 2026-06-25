//! Tests: CSS custom property (--name) parsing in style blocks.

use omni_compiler::lexer::tokenize;
use omni_compiler::parser::parse;

#[test]
fn test_parse_css_custom_property_in_style_block() {
    let src = r#"
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
"#;
    let tokens = tokenize(src);
    let result = parse(&tokens, src);
    assert!(result.is_ok(), "Should parse CSS custom properties: {:?}", result.err());
}
