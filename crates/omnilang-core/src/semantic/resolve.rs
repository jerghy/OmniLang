//! Pass 2 (merged): Resolve references, validate control flow inline.
//!
//! Walks all expressions in every module, component, store, function, and view
//! tree to verify that every [`VarRef`](crate::parser::ast::Expr::VarRef) resolves
//! to a declared symbol.  Also handles inline handler scopes and the `resolve_*`
//! sub-methods for directives.
//!
//! Validation of `break`/`continue` scoping and `await`-in-non-async is performed
//! inline during the traversal using a [`ValidationContext`] — avoiding a separate
//! pass.  Remaining validations (bind targets, store action refs, prop assignment,
//! store field access, motions) are called after the resolve walk for each
//! component / module.

//!
//! The resolve walk itself is split by domain: module orchestration in
//! [`super::resolve_walk`], the statement/expression walk in
//! [`super::resolve_expr`], and the view-tree walk in [`super::resolve_view`].
//! This file holds the design-token type-inference helpers
//! ([`infer_token_expr_type`], [`check_token_type_compatibility`]).

use super::symbols::SymbolTable;
use super::types::SemanticType;
use super::validate_types::infer_expr_type;
use crate::error::{Diagnostic, DiagnosticKind};
use crate::lexer::Span;
use crate::parser::ast::{Expr, TokenType};

/// Check that an expression's inferred type is compatible with the design token type.
///
/// Compatibility rules (Phase 2):
/// - `String` / `Any` (unknown) → always compatible (all token types accept CSS string values)
/// - `Int` / `Float` → compatible with Duration, Opacity, ZIndex, Size, LineHeight, FontWeight
/// - Everything else → type mismatch diagnostic
pub(super) fn check_token_type_compatibility(
    expr: &Expr,
    token_type: &TokenType,
    token_name: &str,
    symbols: &SymbolTable,
    scope: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let inferred = infer_token_expr_type(expr, symbols, scope);

    // String and Any are always compatible — CSS custom properties accept any string
    if matches!(inferred, SemanticType::String | SemanticType::Any) {
        return;
    }

    // Numeric types are compatible with numeric-accepting token types
    if matches!(inferred, SemanticType::Int | SemanticType::Float) {
        let numeric_ok = matches!(
            token_type,
            TokenType::Duration
                | TokenType::Opacity
                | TokenType::ZIndex
                | TokenType::Size
                | TokenType::LineHeight
                | TokenType::FontWeight
        );
        if numeric_ok {
            return;
        }
    }

    // Type mismatch — emit diagnostic
    let type_name = match inferred {
        SemanticType::Int => "int",
        SemanticType::Float => "float",
        SemanticType::Bool => "bool",
        SemanticType::Void => "void",
        SemanticType::Null => "null",
        SemanticType::Named(ref n) => n.as_str(),
        SemanticType::Array(_) => "array",
        SemanticType::Map(..) => "map",
        SemanticType::Optional(_) => "optional",
        SemanticType::Union(_) => "union",
        SemanticType::Fn(..) => "function",
        SemanticType::Any | SemanticType::String => unreachable!(),
    };

    let token_type_str = token_type_name(token_type);

    diagnostics.push(Diagnostic::error(
        DiagnosticKind::E0010,
        &format!(
            "Type mismatch in design token '{}': expression has type '{}' \
             but token type '{}' expects a string value. \
             Wrap the value in a string or use a compatible signal type.",
            token_name, type_name, token_type_str
        ),
        &Span {
            start: 0,
            end: 0,
            line: 0,
            col: 0,
        },
    ));
}

/// Extended type inference for design token expressions.
///
/// Unlike `infer_expr_type()`, this resolves `FieldAccess` expressions
/// (e.g. `ThemeStore.primaryColor`) by looking up the store signal's type
/// rather than returning `Any`.
fn infer_token_expr_type(expr: &Expr, symbols: &SymbolTable, scope: usize) -> SemanticType {
    match expr {
        Expr::FieldAccess { object, field, .. } => {
            // Resolve object → store name → signal type
            if let Expr::VarRef(store_name, _) = object.as_ref() {
                // Look up the store signal: store scope is "store:<StoreName>"
                let store_scope_name = format!("store:{}", store_name);
                if let Some(store_scope_idx) = symbols.find_scope_by_name(&store_scope_name) {
                    if let Some(scope) = symbols.get_scope(store_scope_idx) {
                        if let Some(sig_symbol) = scope.symbols.get(field) {
                            if let Some(type_str) = &sig_symbol.type_ann {
                                return SemanticType::parse_type_string(type_str);
                            }
                        }
                    }
                }
            }
            SemanticType::Any
        }
        _ => infer_expr_type(expr, symbols, scope),
    }
}

/// Human-readable name for a design token type (for diagnostic messages).
fn token_type_name(tt: &TokenType) -> &'static str {
    match tt {
        TokenType::Color => "color",
        TokenType::Size => "size",
        TokenType::Radius => "radius",
        TokenType::FontFamily => "fontFamily",
        TokenType::FontSize => "fontSize",
        TokenType::FontWeight => "fontWeight",
        TokenType::LineHeight => "lineHeight",
        TokenType::LetterSpacing => "letterSpacing",
        TokenType::Shadow => "shadow",
        TokenType::Duration => "duration",
        TokenType::Easing => "easing",
        TokenType::Opacity => "opacity",
        TokenType::ZIndex => "zIndex",
    }
}

// ── Type inference for design token expressions ─────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::DiagnosticKind;
    use crate::parser::ast::Literal;
    use crate::semantic::symbols::{Symbol, SymbolKind, SymbolTable};
    use crate::semantic::types::SemanticType;

    fn span() -> Span {
        Span {
            start: 0,
            end: 0,
            line: 0,
            col: 0,
        }
    }

    fn lit(l: Literal) -> Expr {
        Expr::Literal(l, span())
    }

    fn var_ref(name: &str) -> Expr {
        Expr::VarRef(name.to_string(), span())
    }

    fn declare(symbols: &mut SymbolTable, name: &str, kind: SymbolKind, type_ann: Option<&str>) {
        symbols
            .declare(Symbol {
                name: name.to_string(),
                kind,
                type_ann: type_ann.map(str::to_string),
                mutable: false,
                declared_at: 0,
            })
            .expect("test symbol declaration should succeed");
    }

    // ── infer_token_expr_type: known branches ──────────────────

    #[test]
    fn token_expr_string_literal_is_string() {
        let symbols = SymbolTable::new();
        let ty = infer_token_expr_type(&lit(Literal::String("8px".to_string())), &symbols, 0);
        assert_eq!(ty, SemanticType::String);
    }

    #[test]
    fn token_expr_int_literal_is_int() {
        let symbols = SymbolTable::new();
        let ty = infer_token_expr_type(&lit(Literal::Int(12)), &symbols, 0);
        assert_eq!(ty, SemanticType::Int);
    }

    #[test]
    fn token_expr_float_literal_is_float() {
        let symbols = SymbolTable::new();
        let ty = infer_token_expr_type(&lit(Literal::Float(0.5)), &symbols, 0);
        assert_eq!(ty, SemanticType::Float);
    }

    #[test]
    fn token_expr_bool_literal_is_bool() {
        let symbols = SymbolTable::new();
        let ty = infer_token_expr_type(&lit(Literal::Bool(true)), &symbols, 0);
        assert_eq!(ty, SemanticType::Bool);
    }

    #[test]
    fn token_expr_array_literal_is_array_of_inner_type() {
        let symbols = SymbolTable::new();
        let ty = infer_token_expr_type(
            &Expr::ArrayLiteral {
                elements: vec![lit(Literal::Int(1)), lit(Literal::Int(2))],
                span: span(),
            },
            &symbols,
            0,
        );
        assert_eq!(ty, SemanticType::Array(Box::new(SemanticType::Int)));
    }

    #[test]
    fn token_expr_template_string_is_string() {
        let symbols = SymbolTable::new();
        let ty = infer_token_expr_type(
            &Expr::TemplateString {
                parts: vec![],
                span: span(),
            },
            &symbols,
            0,
        );
        assert_eq!(ty, SemanticType::String);
    }

    #[test]
    fn token_expr_call_returns_declared_return_type() {
        // "方法调用返回" — a call to a function whose return type is declared
        // infers to that type (String), not Any.
        let mut symbols = SymbolTable::new();
        declare(
            &mut symbols,
            "getAccentColor",
            SymbolKind::Function,
            Some("String"),
        );
        let ty = infer_token_expr_type(
            &Expr::FuncCall {
                func: Box::new(var_ref("getAccentColor")),
                args: vec![],
                span: span(),
            },
            &symbols,
            0,
        );
        assert_eq!(ty, SemanticType::String);
    }

    #[test]
    fn token_expr_call_without_return_type_is_any() {
        let symbols = SymbolTable::new();
        let ty = infer_token_expr_type(
            &Expr::FuncCall {
                func: Box::new(var_ref("unknownFn")),
                args: vec![],
                span: span(),
            },
            &symbols,
            0,
        );
        assert_eq!(ty, SemanticType::Any);
    }

    #[test]
    fn token_expr_method_call_is_any() {
        let symbols = SymbolTable::new();
        let ty = infer_token_expr_type(
            &Expr::MethodCall {
                object: Box::new(lit(Literal::String("x".to_string()))),
                method: "toUpper".to_string(),
                args: vec![],
                span: span(),
            },
            &symbols,
            0,
        );
        assert_eq!(ty, SemanticType::Any);
    }

    #[test]
    fn token_expr_var_ref_uses_declared_type() {
        let mut symbols = SymbolTable::new();
        declare(&mut symbols, "isDark", SymbolKind::Const, Some("bool"));
        let ty = infer_token_expr_type(&var_ref("isDark"), &symbols, 0);
        assert_eq!(ty, SemanticType::Bool);
    }

    #[test]
    fn token_expr_unknown_var_ref_is_any() {
        let symbols = SymbolTable::new();
        let ty = infer_token_expr_type(&var_ref("nope"), &symbols, 0);
        assert_eq!(ty, SemanticType::Any);
    }

    #[test]
    fn token_expr_field_access_resolves_store_signal_type() {
        // `ThemeStore.primaryColor` → the signal's declared type in the
        // `store:ThemeStore` scope (the store-typed path `infer_expr_type`
        // does not take).
        let mut symbols = SymbolTable::new();
        let store_scope = symbols.push_scope("store:ThemeStore");
        declare(
            &mut symbols,
            "primaryColor",
            SymbolKind::Signal,
            Some("Color"),
        );
        let ty = infer_token_expr_type(
            &Expr::FieldAccess {
                object: Box::new(var_ref("ThemeStore")),
                field: "primaryColor".to_string(),
                span: span(),
            },
            &symbols,
            store_scope,
        );
        assert_eq!(ty, SemanticType::Named("Color".to_string()));
    }

    #[test]
    fn token_expr_field_access_missing_store_is_any() {
        // No `store:ThemeStore` scope — the field access cannot be resolved
        // to a signal type and falls back to Any (no cascading errors).
        let symbols = SymbolTable::new();
        let ty = infer_token_expr_type(
            &Expr::FieldAccess {
                object: Box::new(var_ref("ThemeStore")),
                field: "primaryColor".to_string(),
                span: span(),
            },
            &symbols,
            0,
        );
        assert_eq!(ty, SemanticType::Any);
    }

    // ── check_token_type_compatibility ─────────────────────────

    #[test]
    fn token_compat_string_value_always_ok() {
        // String is compatible with every token type (CSS custom properties
        // accept any string).
        for token_type in [
            TokenType::Color,
            TokenType::Size,
            TokenType::Duration,
            TokenType::Shadow,
            TokenType::ZIndex,
        ] {
            let mut diags = Vec::new();
            let symbols = SymbolTable::new();
            check_token_type_compatibility(
                &lit(Literal::String("x".to_string())),
                &token_type,
                "primary",
                &symbols,
                0,
                &mut diags,
            );
            assert!(diags.is_empty(), "String must be compatible: {:?}", diags);
        }
    }

    #[test]
    fn token_compat_int_with_numeric_token_is_ok() {
        // Int is compatible with Duration / Opacity / ZIndex / Size /
        // LineHeight / FontWeight.
        for token_type in [
            TokenType::Duration,
            TokenType::Opacity,
            TokenType::ZIndex,
            TokenType::Size,
            TokenType::LineHeight,
            TokenType::FontWeight,
        ] {
            let mut diags = Vec::new();
            let symbols = SymbolTable::new();
            check_token_type_compatibility(
                &lit(Literal::Int(12)),
                &token_type,
                "radius",
                &symbols,
                0,
                &mut diags,
            );
            assert!(diags.is_empty(), "Int must be compatible: {:?}", diags);
        }
    }

    #[test]
    fn token_compat_int_with_color_token_errors() {
        // Int is NOT compatible with Color — E0010 type mismatch.
        let mut diags = Vec::new();
        let symbols = SymbolTable::new();
        check_token_type_compatibility(
            &lit(Literal::Int(12)),
            &TokenType::Color,
            "primary",
            &symbols,
            0,
            &mut diags,
        );
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].kind, DiagnosticKind::E0010);
        assert!(diags[0].message.contains("primary"), "{:?}", diags[0]);
    }

    #[test]
    fn token_compat_bool_with_size_token_errors() {
        let mut diags = Vec::new();
        let symbols = SymbolTable::new();
        check_token_type_compatibility(
            &lit(Literal::Bool(true)),
            &TokenType::Size,
            "radius",
            &symbols,
            0,
            &mut diags,
        );
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].kind, DiagnosticKind::E0010);
        assert!(
            diags[0].message.contains("'bool'"),
            "message must name the inferred type: {:?}",
            diags[0]
        );
    }

    #[test]
    fn token_compat_array_with_any_token_errors() {
        let mut diags = Vec::new();
        let symbols = SymbolTable::new();
        check_token_type_compatibility(
            &Expr::ArrayLiteral {
                elements: vec![lit(Literal::Int(1))],
                span: span(),
            },
            &TokenType::Color,
            "primary",
            &symbols,
            0,
            &mut diags,
        );
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("'array'"), "{:?}", diags[0]);
    }

    // ── token_type_name ─────────────────────────────────────────

    #[test]
    fn token_type_name_renders_human_readable_names() {
        assert_eq!(token_type_name(&TokenType::Color), "color");
        assert_eq!(token_type_name(&TokenType::Duration), "duration");
        assert_eq!(token_type_name(&TokenType::ZIndex), "zIndex");
    }
}
