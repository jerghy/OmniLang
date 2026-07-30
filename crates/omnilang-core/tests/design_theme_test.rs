//! Full-pipeline integration tests for themed design blocks.
//!
//! Tests that `@themes`, `@default-theme`, and per-theme token values
//! survive the full compiler pipeline: lexer -> parser -> semantic ->
//! IR lowering.  These tests verify that the theme fields are correctly
//! propagated from source text to DesignIR / TokenIR.
//!
//! See: docs/superpowers/plans/2026-07-28-design-theme-system.md (Task 13)

use omnilang_core::ir::lower;
use omnilang_core::ir::DesignIR;
use omnilang_core::lexer::tokenize;
use omnilang_core::parser::parse;
use omnilang_core::semantic::analyze;

/// Parse an OmniLang source string and return the lowered DesignIR
/// for the first design block found.
fn lower_first_design(source: &str) -> DesignIR {
    let tokens = tokenize(source);
    let (program, _) = parse(tokens, source);
    let mut semantics = analyze(&program);
    let ir = lower(&program, &mut semantics);

    let module = ir
        .modules
        .get("input.omni")
        .expect("should have input.omni module");
    module
        .designs
        .first()
        .cloned()
        .expect("should have at least one design")
}

#[test]
fn full_pipeline_design_with_themes() {
    // Use r##"..."## raw strings because the OmniLang source contains `"#` sequences
    // (e.g. "#a5b4fc") which would terminate a single-# raw string early.
    let source = r##"
import omniview;

design AppDesign {
    @themes light, dark;
    @default-theme light;

    token primary: color = "#6366f1" dark "#a5b4fc";
    token surface: color = "#ffffff" dark "#111827";

    class Card {
        background: token(surface);
    }
}
"##;
    let tokens = tokenize(source);
    let (program, _) = parse(tokens, source);
    let mut semantics = analyze(&program);
    let ir = lower(&program, &mut semantics);

    // Find the design in the IR
    let module = ir
        .modules
        .get("input.omni")
        .expect("should have input.omni module");
    let design = module
        .designs
        .iter()
        .find(|d| d.name == "AppDesign")
        .expect("should have AppDesign design");

    // Verify theme fields survived the pipeline
    assert_eq!(design.themes, vec!["light", "dark"]);
    assert_eq!(design.default_theme, Some("light".to_string()));

    // Verify token theme values
    let primary = design
        .tokens
        .get("primary")
        .expect("should have primary token");
    assert_eq!(primary.value, "#6366f1");
    assert_eq!(
        primary.theme_values.get("dark"),
        Some(&"#a5b4fc".to_string())
    );

    let surface = design
        .tokens
        .get("surface")
        .expect("should have surface token");
    assert_eq!(surface.value, "#ffffff");
    assert_eq!(
        surface.theme_values.get("dark"),
        Some(&"#111827".to_string())
    );

    // Verify class still resolves token references
    let card = design
        .classes
        .iter()
        .find(|c| c.name == "Card")
        .expect("should have Card class");
    assert!(card.properties.contains_key("background"));

    // No diagnostics should be emitted by the parser or semantic analyzer
    assert!(
        semantics.diagnostics.is_empty(),
        "should have no diagnostics, got: {:?}",
        semantics.diagnostics
    );
}

#[test]
fn design_without_themes_still_works() {
    let source = r##"
import omniview;

design SimpleDesign {
    token primary: color = "#6366f1";
    class Card {
        background: token(primary);
    }
}
"##;
    let design = lower_first_design(source);

    // Backward compatibility: themes should default to empty
    assert!(
        design.themes.is_empty(),
        "design without @themes should have empty themes"
    );
    assert_eq!(design.default_theme, None);

    // Tokens should still work without theme_values
    let primary = design
        .tokens
        .get("primary")
        .expect("should have primary token");
    assert_eq!(primary.value, "#6366f1");
    assert!(
        primary.theme_values.is_empty(),
        "token without theme values should have empty theme_values"
    );

    // Classes should still resolve token references
    let card = design
        .classes
        .iter()
        .find(|c| c.name == "Card")
        .expect("should have Card class");
    assert!(card.properties.contains_key("background"));
}
