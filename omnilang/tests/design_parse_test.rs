use omnilang::lexer::tokenize;
use omnilang::parser;
use omnilang::parser::ast::design::*;

#[test]
fn parse_full_design_file() {
    let src = r##"
import omniview;

design AppDesign {
    token primary: color = "#6366f1";
    token spacingMd: size = "16px";

    class Card {
        background: token(surface);
        &:hover {
            box-shadow: "0 4px 12px rgba(0,0,0,0.08)";
        }
    }

    variant Button {
        @dim color: primary | danger;
        @dim size: sm | md;
        @default { color: primary; size: md; }
        base {
            display: inline-flex;
            border-radius: token(radiusMd);
        }
        color.primary { background: token(primary); }
        size.sm { padding: "4px 8px"; }
    }
}
"##;
    let tokens = tokenize(src);
    let (program, errors) = parser::parse(tokens, src);
    assert!(errors.is_empty(), "Parse failed: {:?}", errors);

    let module = program.modules.get("input.omni").unwrap();
    assert_eq!(module.designs.len(), 1, "should have one design block");

    let design = &module.designs[0];
    assert_eq!(design.name, "AppDesign");
    assert_eq!(design.tokens.len(), 2, "should have 2 tokens");
    assert_eq!(design.classes.len(), 1, "should have 1 class");
    assert_eq!(design.variants.len(), 1, "should have 1 variant");
}

#[test]
fn parse_minimal_design_block() {
    let src = "design Minimal { }";
    let tokens = tokenize(src);
    let (program, errors) = parser::parse(tokens, src);
    assert!(
        errors.is_empty(),
        "Minimal design parse failed: {:?}",
        errors
    );
    let module = program.modules.get("input.omni").unwrap();
    assert_eq!(module.designs.len(), 1);
    assert_eq!(module.designs[0].name, "Minimal");
    assert_eq!(module.designs[0].tokens.len(), 0);
    assert_eq!(module.designs[0].classes.len(), 0);
    assert_eq!(module.designs[0].variants.len(), 0);
}

#[test]
fn parse_design_mixed_value() {
    let src = r##"
        design Test {
            class Card {
                border: "1px solid " token(primary);
            }
        }
    "##;
    let tokens = tokenize(src);
    let (program, _) = parser::parse(tokens, src);
    let module = program.modules.get("input.omni").unwrap();
    assert_eq!(module.designs.len(), 1);
    let design = &module.designs[0];
    assert_eq!(design.classes.len(), 1);
    let cls = &design.classes[0];
    assert_eq!(cls.properties.len(), 1);
    // "1px solid " token(primary) should parse as Mixed
    match &cls.properties[0].value {
        DesignValue::Mixed(parts) => {
            assert_eq!(parts.len(), 2, "Mixed should have 2 parts");
            assert_eq!(parts[0], DesignValuePart::Text("1px solid ".to_string()));
            assert_eq!(parts[1], DesignValuePart::Token("primary".to_string()));
        }
        other => panic!("expected DesignValue::Mixed, got {:?}", other),
    }
}

#[test]
fn parse_design_pseudo_element() {
    let src = r##"
        design Test {
            class Card {
                &::before {
                    content: "x";
                }
            }
        }
    "##;
    let tokens = tokenize(src);
    let (program, _) = parser::parse(tokens, src);
    let module = program.modules.get("input.omni").unwrap();
    assert_eq!(module.designs.len(), 1);
    let design = &module.designs[0];
    assert_eq!(design.classes.len(), 1);
    let cls = &design.classes[0];
    assert_eq!(cls.pseudo_states.len(), 1);
    assert_eq!(
        cls.pseudo_states[0].pseudo, "::before",
        "pseudo-element should include '::' prefix"
    );
    assert_eq!(cls.pseudo_states[0].properties.len(), 1);
    assert_eq!(cls.pseudo_states[0].properties[0].name, "content");
}

#[test]
fn parse_all_token_types() {
    let src = r##"
        design Tokens {
            token c: color = "#000";
            token s: size = "16px";
            token r: radius = "8px";
            token ff: fontFamily = "sans-serif";
            token fs: fontSize = "16px";
            token fw: fontWeight = "400";
            token lh: lineHeight = "1.5";
            token ls: letterSpacing = "0.01em";
            token sh: shadow = "0 1px 3px";
            token d: duration = "200ms";
            token e: easing = "ease-in";
            token o: opacity = "0.5";
            token z: zIndex = "100";
        }
    "##;
    let tokens = tokenize(src);
    let (program, _) = parser::parse(tokens, src);
    let module = program.modules.get("input.omni").unwrap();
    assert_eq!(module.designs.len(), 1);
    let design = &module.designs[0];
    assert_eq!(design.tokens.len(), 13, "should parse all 13 token types");
    // Verify a few specific types
    assert_eq!(design.tokens[0].token_type, TokenType::Color);
    assert_eq!(design.tokens[4].token_type, TokenType::FontSize);
    assert_eq!(design.tokens[9].token_type, TokenType::Duration);
    assert_eq!(design.tokens[12].token_type, TokenType::ZIndex);
}

#[test]
fn parse_variant_defaults_wrapper() {
    let src = r##"
        design Test {
            variant Button {
                @default { color: primary; }
                base {
                    display: inline-flex;
                }
            }
        }
    "##;
    let tokens = tokenize(src);
    let (program, _) = parser::parse(tokens, src);
    let module = program.modules.get("input.omni").unwrap();
    assert_eq!(module.designs.len(), 1);
    let design = &module.designs[0];
    assert_eq!(design.variants.len(), 1);
    let v = &design.variants[0];
    assert_eq!(v.defaults.len(), 1, "should have 1 default");
    assert_eq!(v.defaults[0], ("color".into(), "primary".into()));
}

#[test]
fn parse_design_error_on_unknown() {
    let src = r##"
        design Test {
            class Card {
                @garbage { }
            }
        }
    "##;
    let tokens = tokenize(src);
    let (_, errors) = parser::parse(tokens, src);
    assert!(
        !errors.is_empty(),
        "should produce error for unknown directive"
    );
}

#[test]
fn parse_design_block_comment() {
    let src = r##"
        design Test {
            /* block comment */
            token primary: color = "#000";
        }
    "##;
    let tokens = tokenize(src);
    let (program, errors) = parser::parse(tokens, src);
    assert!(
        errors.is_empty(),
        "comment should not cause parse failure: {:?}",
        errors
    );
    let module = program.modules.get("input.omni").unwrap();
    assert_eq!(module.designs.len(), 1);
    let design = &module.designs[0];
    assert_eq!(design.tokens.len(), 1, "should still parse the token");
}
