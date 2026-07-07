use omnilang::lexer::tokenize;
use omnilang::parser;

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
    let result = parser::parse(&tokens, src);
    assert!(result.is_ok(), "Parse failed: {:?}", result.err());

    let program = result.unwrap();
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
    let result = parser::parse(&tokens, src);
    assert!(
        result.is_ok(),
        "Minimal design parse failed: {:?}",
        result.err()
    );

    let program = result.unwrap();
    let module = program.modules.get("input.omni").unwrap();
    assert_eq!(module.designs.len(), 1);
    assert_eq!(module.designs[0].name, "Minimal");
    assert_eq!(module.designs[0].tokens.len(), 0);
    assert_eq!(module.designs[0].classes.len(), 0);
    assert_eq!(module.designs[0].variants.len(), 0);
}
