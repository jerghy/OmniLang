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
use omnilang_core::ir::TokenValueIR;
use omnilang_core::lexer::tokenize;
use omnilang_core::parser::parse;
use omnilang_core::semantic::analyze;

/// Parse an OmniLang source string and return the lowered DesignIR
/// for the first design block found.
fn lower_first_design(source: &str) -> DesignIR {
    let tokens = tokenize(source);
    let (program, _) = parse(tokens, source);
    let mut semantics = analyze(&program);
    let ir = lower(&program, &mut semantics, None);

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
    let ir = lower(&program, &mut semantics, None);

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
    assert!(matches!(primary.value, TokenValueIR::Static(ref s) if s == "#6366f1"));
    assert!(
        matches!(primary.theme_values.get("dark"), Some(TokenValueIR::Static(ref s)) if s == "#a5b4fc"),
        "expected dark theme override to be \"#a5b4fc\""
    );

    let surface = design
        .tokens
        .get("surface")
        .expect("should have surface token");
    assert!(matches!(surface.value, TokenValueIR::Static(ref s) if s == "#ffffff"));
    assert!(
        matches!(surface.theme_values.get("dark"), Some(TokenValueIR::Static(ref s)) if s == "#111827"),
        "expected dark theme override to be \"#111827\""
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
    assert!(matches!(primary.value, TokenValueIR::Static(ref s) if s == "#6366f1"));
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

#[test]
fn semantic_rejects_undefined_variable_in_token() {
    let source = r##"
import omniview;

design AppDesign {
    token primary: color = UNDEFINED_VAR;
}
"##;
    let tokens = tokenize(source);
    let (program, _) = parse(tokens, source);
    let semantics = analyze(&program);

    // Should have an E0001 diagnostic for UNDEFINED_VAR
    let has_error = semantics
        .diagnostics
        .iter()
        .any(|d| d.message.contains("UNDEFINED_VAR"));
    assert!(has_error, "should report undefined variable in token value");
}

#[test]
fn semantic_accepts_defined_const_in_token() {
    let source = r##"
import omniview;

const BRAND = "#6366f1";

design AppDesign {
    token primary: color = BRAND;
}
"##;
    let tokens = tokenize(source);
    let (program, _) = parse(tokens, source);
    let semantics = analyze(&program);

    // No E0001 for BRAND since it's defined
    let brand_errors: Vec<_> = semantics
        .diagnostics
        .iter()
        .filter(|d| d.message.contains("BRAND"))
        .collect();
    assert!(brand_errors.is_empty(), "BRAND should be resolved");
}

#[test]
fn semantic_accepts_store_field_access_in_token_expression() {
    // ThemeStore.primaryColor is a field access on a defined store,
    // NOT a bare variable reference.  The resolver should not produce
    // E0001 for the store name (it's a module-level symbol).
    let source = r##"
import omniview;

store ThemeStore {
    signal primaryColor: string = "#6366f1";
}

design AppDesign {
    token primary: color = ThemeStore.primaryColor;
}
"##;
    let tokens = tokenize(source);
    let (program, _) = parse(tokens, source);
    let semantics = analyze(&program);

    let errors: Vec<_> = semantics
        .diagnostics
        .iter()
        .filter(|d| matches!(d.kind, omnilang_core::error::DiagnosticKind::E0001))
        .collect();
    assert!(
        errors.is_empty(),
        "should have no E0001 for ThemeStore.primaryColor, got: {:?}",
        errors
    );
}

#[test]
fn rejects_dynamic_token_with_theme_override() {
    // Phase 2 restriction: dynamic tokens (TokenValue::Expression) cannot
    // have theme overrides. The semantic analyzer should produce E0001.
    let source = r##"
import omniview;

store ThemeStore {
    signal primaryColor: string = "#6366f1";
}

design AppDesign {
    @themes light, dark;
    token primary: color = ThemeStore.primaryColor dark "#a5b4fc";
}
"##;
    let tokens = tokenize(source);
    let (program, _) = parse(tokens, source);
    let semantics = analyze(&program);

    let has_error = semantics
        .diagnostics
        .iter()
        .any(|d| d.message.contains("cannot have theme overrides"));
    assert!(
        has_error,
        "should produce diagnostic for dynamic token with theme override, got: {:?}",
        semantics.diagnostics
    );
}

#[test]
fn semantic_rejects_undefined_variable_in_token_expression() {
    // A function call to an undefined function inside a token expression
    // should produce E0001.
    let source = r##"
import omniview;

design AppDesign {
    token primary: color = undefinedFunc();
}
"##;
    let tokens = tokenize(source);
    let (program, _) = parse(tokens, source);
    let semantics = analyze(&program);

    let has_error = semantics
        .diagnostics
        .iter()
        .any(|d| d.message.contains("undefinedFunc"));
    assert!(
        has_error,
        "should report undefined variable in token expression"
    );
}
