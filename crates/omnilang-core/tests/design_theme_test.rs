//! Full-pipeline integration tests for themed design blocks.
//!
//! Tests that `@themes`, `@default-theme`, and per-theme token values
//! survive the full compiler pipeline: lexer -> parser -> semantic ->
//! IR lowering.  These tests verify that the theme fields are correctly
//! propagated from source text to DesignIR / TokenIR.
//!
//! See: docs/superpowers/plans/2026-07-28-design-theme-system.md (Task 13)

use omnilang_core::analyse::analyse_multi;
use omnilang_core::error::Diagnostic;
use omnilang_core::ir::lower;
use omnilang_core::ir::DesignIR;
use omnilang_core::ir::TokenValueIR;
use omnilang_core::lexer::tokenize;
use omnilang_core::parser::parse;
use omnilang_core::semantic::analyze;
use rustc_hash::FxHashMap;

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
    @themes { light, dark }
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
    @themes { light, dark }
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

#[test]
fn semantic_accepts_string_signal_for_color_token() {
    // A string signal is compatible with any token type (Color expects a string value)
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

    let type_errors: Vec<_> = semantics
        .diagnostics
        .iter()
        .filter(|d| d.message.contains("Type mismatch"))
        .collect();
    assert!(
        type_errors.is_empty(),
        "string signal should be compatible with color token, got: {:?}",
        type_errors
    );
}

#[test]
fn semantic_rejects_bool_expression_for_color_token() {
    // A bool expression is NOT compatible with a Color token (expects string)
    let source = r##"
import omniview;

store ThemeStore {
    signal flag: bool = true;
}

design AppDesign {
    token primary: color = ThemeStore.flag;
}
"##;
    let tokens = tokenize(source);
    let (program, _) = parse(tokens, source);
    let semantics = analyze(&program);

    let type_errors: Vec<_> = semantics
        .diagnostics
        .iter()
        .filter(|d| d.message.contains("Type mismatch"))
        .collect();
    assert!(
        !type_errors.is_empty(),
        "bool signal should be rejected for color token"
    );
    assert!(
        type_errors[0].message.contains("bool"),
        "error should mention bool type, got: {}",
        type_errors[0].message
    );
    assert!(
        type_errors[0].message.contains("color"),
        "error should mention color token type"
    );
}

#[test]
fn semantic_accepts_int_for_duration_token() {
    // An int signal is compatible with Duration (numeric-accepting token type)
    let source = r##"
import omniview;

store AnimStore {
    signal speed: int = 300;
}

design AppDesign {
    token duration: duration = AnimStore.speed;
}
"##;
    let tokens = tokenize(source);
    let (program, _) = parse(tokens, source);
    let semantics = analyze(&program);

    let type_errors: Vec<_> = semantics
        .diagnostics
        .iter()
        .filter(|d| d.message.contains("Type mismatch"))
        .collect();
    assert!(
        type_errors.is_empty(),
        "int signal should be compatible with duration token, got: {:?}",
        type_errors
    );
}

// ═══════════════════════════════════════════════════════════════
// v2 域 3/域 5 (D1 语法层): @themes { ... } 声明 + @theme 信号标记
// 程序级 validate_theme pass（@themes ↔ @theme 配对 + 语义验证）
// ═══════════════════════════════════════════════════════════════

/// Compile a set of source files through the real multi-file pipeline
/// (parse → semantic → lower → merge → validate_theme) and return the
/// diagnostics plus the merged IR.
fn compile_multi(sources: &[(&str, &str)]) -> (Vec<Diagnostic>, omnilang_core::ir::IRProgram) {
    let map: FxHashMap<String, String> = sources
        .iter()
        .map(|(p, s)| (p.to_string(), s.to_string()))
        .collect();
    let path_mapping = FxHashMap::default();
    let out = analyse_multi(&map, &path_mapping);
    (
        out.diagnostics,
        out.ir.expect("clean parse should produce IR"),
    )
}

/// Collect error diagnostics only.
fn errors(diags: &[Diagnostic]) -> Vec<&Diagnostic> {
    diags
        .iter()
        .filter(|d| d.severity == omnilang_core::error::Severity::Error)
        .collect()
}

const THEME_DESIGN: &str = r##"
import omniview;

design AppDesign {
    @themes { dark, light }
    token primary: color = "#6366f1" dark "#a5b4fc";
}
"##;

const THEME_STORE: &str = r##"
import omniview;

store ThemeStore {
    signal mode: String @theme = "light";
}
"##;

#[test]
fn theme_happy_path_same_file_wires_theme_signal() {
    let source = format!("{}\n{}", THEME_DESIGN.replace("\n\n", "\n"), THEME_STORE);
    let (diags, ir) = compile_multi(&[("app.omni", &source)]);
    assert!(
        errors(&diags).is_empty(),
        "happy path must be clean, got: {:?}",
        diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );

    let module = ir.modules.get("input.omni").unwrap();
    let design = module
        .designs
        .iter()
        .find(|d| d.name == "AppDesign")
        .expect("should have AppDesign");
    assert_eq!(design.themes, vec!["dark", "light"]);
    assert_eq!(
        design.theme_signal.as_deref(),
        Some("ThemeStore.mode"),
        "the @theme-marked store signal must be wired into DesignIR.theme_signal"
    );
}

#[test]
fn theme_happy_path_cross_file_wires_theme_signal() {
    // The real demo shape: @themes in design.omni, @theme signal in
    // stores/ThemeStore.omni — separate files. The program-level pass must
    // pair them across files.
    let (diags, ir) = compile_multi(&[
        ("design.omni", THEME_DESIGN),
        ("ThemeStore.omni", THEME_STORE),
    ]);
    assert!(
        errors(&diags).is_empty(),
        "cross-file pairing must be clean, got: {:?}",
        diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );

    let module = ir.modules.get("input.omni").unwrap();
    let design = module
        .designs
        .iter()
        .find(|d| d.name == "AppDesign")
        .expect("should have AppDesign");
    assert_eq!(
        design.theme_signal.as_deref(),
        Some("ThemeStore.mode"),
        "cross-file @themes ↔ @theme pairing must populate theme_signal"
    );
}

#[test]
fn theme_without_themes_declaration_is_error() {
    let source = r##"
import omniview;

design AppDesign {
    token primary: color = "#6366f1";
}

store ThemeStore {
    signal mode: String @theme = "light";
}
"##;
    let (diags, _) = compile_multi(&[("app.omni", source)]);
    let errs = errors(&diags);
    assert_eq!(errs.len(), 1, "got: {:?}", diags);
    assert!(
        errs[0].message.contains("未声明 @themes"),
        "should report missing @themes, got: {}",
        errs[0].message
    );
}

#[test]
fn theme_signal_must_be_string() {
    let source = r##"
import omniview;

design AppDesign {
    @themes { dark, light }
}

store ThemeStore {
    signal mode: Int @theme = 1;
}
"##;
    let (diags, _) = compile_multi(&[("app.omni", source)]);
    let errs = errors(&diags);
    assert_eq!(errs.len(), 1, "got: {:?}", diags);
    assert!(
        errs[0].message.contains("必须是 String"),
        "design doc verbatim message expected, got: {}",
        errs[0].message
    );
}

#[test]
fn duplicate_theme_signals_is_error() {
    let source = r##"
import omniview;

design AppDesign {
    @themes { dark, light }
}

store ThemeStore {
    signal mode: String @theme = "light";
    signal alt: String @theme = "dark";
}
"##;
    let (diags, _) = compile_multi(&[("app.omni", source)]);
    let errs = errors(&diags);
    assert_eq!(errs.len(), 1, "got: {:?}", diags);
    assert!(
        errs[0].message.contains("重复 @theme"),
        "should report duplicate @theme signals, got: {}",
        errs[0].message
    );
}

#[test]
fn duplicate_theme_names_is_error() {
    let source = r##"
import omniview;

design AppDesign {
    @themes { dark, dark }
}
"##;
    let (diags, _) = compile_multi(&[("app.omni", source)]);
    let errs = errors(&diags);
    assert_eq!(errs.len(), 1, "got: {:?}", diags);
    assert!(
        errs[0].message.contains("重复主题"),
        "should report duplicate theme names, got: {}",
        errs[0].message
    );
}

#[test]
fn empty_themes_is_error() {
    // (f) empty `@themes { }` hard-errors at parse time (the IR cannot
    // distinguish an explicit empty declaration from no declaration) — the
    // parse failure surfaces as an error diagnostic in the pipeline.
    let source = r##"
import omniview;

design AppDesign {
    @themes { }
}
"##;
    let map: FxHashMap<String, String> = [("app.omni".to_string(), source.to_string())]
        .into_iter()
        .collect();
    let path_mapping = FxHashMap::default();
    let out = analyse_multi(&map, &path_mapping);
    assert!(!out.success, "empty @themes must fail compilation");
    let errs: Vec<_> = out
        .diagnostics
        .iter()
        .filter(|d| d.severity == omnilang_core::error::Severity::Error)
        .collect();
    assert!(
        errs.iter().any(|d| d.message.contains("不能为空")),
        "should report empty @themes, got: {:?}",
        errs.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

#[test]
fn theme_signal_without_marker_is_unwired() {
    let source = r##"
import omniview;

design AppDesign {
    @themes { dark, light }
}

store ThemeStore {
    signal mode: String = "light";
}
"##;
    let (diags, ir) = compile_multi(&[("app.omni", source)]);
    assert!(errors(&diags).is_empty(), "got: {:?}", diags);
    let module = ir.modules.get("input.omni").unwrap();
    let design = module
        .designs
        .iter()
        .find(|d| d.name == "AppDesign")
        .unwrap();
    assert!(
        design.theme_signal.is_none(),
        "no @theme marker → theme_signal stays None"
    );
}

#[test]
fn theme_validation_runs_in_single_file_pipeline() {
    // run_pipeline (single-file entry) must surface the same theme errors
    // as the multi-file pipeline.
    let source = r##"
import omniview;

design AppDesign {
    token primary: color = "#6366f1";
}

store ThemeStore {
    signal mode: String @theme = "light";
}
"##;
    let output = omnilang_core::run_pipeline(source);
    let errs: Vec<_> = output
        .diagnostics
        .iter()
        .filter(|d| d.severity == omnilang_core::error::Severity::Error)
        .collect();
    assert!(
        errs.iter().any(|d| d.message.contains("未声明 @themes")),
        "single-file pipeline must surface theme errors, got: {:?}",
        errs.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

#[test]
fn semantic_accepts_literal_int_for_opacity_token() {
    // A literal int in an expression is compatible with Opacity
    let source = r##"
import omniview;

fn getOpacity(): int {
    return 1;
}

design AppDesign {
    token alpha: opacity = getOpacity();
}
"##;
    let tokens = tokenize(source);
    let (program, _) = parse(tokens, source);
    let semantics = analyze(&program);

    let type_errors: Vec<_> = semantics
        .diagnostics
        .iter()
        .filter(|d| d.message.contains("Type mismatch"))
        .collect();
    assert!(
        type_errors.is_empty(),
        "int return should be compatible with opacity token, got: {:?}",
        type_errors
    );
}
