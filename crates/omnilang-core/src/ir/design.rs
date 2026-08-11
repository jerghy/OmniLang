//! Design IR types — lowered representation of `design` blocks.
//!
//! Pipeline: AST `DesignDef` → `DesignIR` (via lowering) → CSS codegen.
//!
//! All types derive Debug, Clone, Serialize, Deserialize for
//! JSON round-tripping in the IDE bridge and WASM bindings.

use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::ir::types::ExprIR;
use crate::lexer::Span;

/// A fully lowered design block.
///
/// Tokens are resolved into a `FxHashMap<String, TokenIR>` for O(1) lookup.
/// All `token(...)` references in class / variant properties have been
/// replaced with their resolved CSS values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignIR {
    pub name: String,
    /// Theme names declared via `@themes { ... }` (e.g. `["light", "dark"]`).
    #[serde(default)]
    pub themes: Vec<String>,
    /// The program's current-theme-mode signal, as the dotted path
    /// `"StoreName.signalName"` (e.g. `"ThemeStore.mode"`) — the store signal
    /// marked `@theme` (v2 域 5, design D1). Populated by the program-level
    /// `validate_theme` pass after IR merge; consumed by codegen (Task 4.2)
    /// to wire theme switching (data-theme subscription / ThemeModeNotifier).
    /// `None` when no signal is marked `@theme`.
    #[serde(default)]
    pub theme_signal: Option<String>,
    /// Optional default theme name set via `@default-theme ...;`.
    #[serde(default)]
    pub default_theme: Option<String>,
    pub tokens: FxHashMap<String, TokenIR>,
    pub classes: Vec<ClassIR>,
    pub variants: Vec<VariantIR>,
    /// Named breakpoint declarations: `breakpoint md: 768;` (batch 3, spec
    /// 2026-08-09 §2.1) — name → unitless px value, min-width semantics.
    /// Compile-time static table consumed by the `bp("name")` condition
    /// expression (validate_breakpoints pass + three-target codegen).
    #[serde(default)]
    pub breakpoints: FxHashMap<String, u32>,
    /// Source file path relative to project root (e.g. "src/design.omni").
    /// Populated during multi-file compilation; used by multi-file codegens
    /// (React) to place design outputs (design.css, hooks) in the same
    /// output directory as the design's source file.
    #[serde(default)]
    pub source_file: Option<String>,
    #[serde(default)]
    pub source_span: Option<Span>,
}

/// Resolved token value — either a static CSS string or a runtime expression.
///
/// `#[non_exhaustive]` sits at a crate boundary (`omnilang-core` → `omnilang`).
/// Adding a third variant would silently match existing wildcards without it;
/// this forces compile errors on new variants in downstream code.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TokenValueIR {
    /// Static CSS value resolved at compile time: `"#6366f1"`, `"16px"`.
    Static(String),
    /// Runtime expression that must be evaluated via `__omni_effect`:
    /// `ThemeStore.primaryColor`.
    Dynamic(ExprIR),
}

/// A single design token with its resolved value and type annotation.
///
/// Per-theme overrides are stored as `theme_values: FxHashMap<String, TokenValueIR>`;
/// the outer `value` is the fallback used when no theme context matches.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenIR {
    pub value: TokenValueIR,
    pub token_type: TokenTypeIR,
    /// Per-theme value overrides keyed by theme name.
    #[serde(default)]
    pub theme_values: FxHashMap<String, TokenValueIR>,
    #[serde(default)]
    pub source_span: Option<Span>,
}

/// Token type annotation mirroring the AST `TokenType` — 13 recognized types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TokenTypeIR {
    Color,
    Size,
    Radius,
    FontFamily,
    FontSize,
    FontWeight,
    LineHeight,
    LetterSpacing,
    Shadow,
    Duration,
    Easing,
    Opacity,
    ZIndex,
}

/// A CSS class with resolved property values and pseudo-state variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassIR {
    pub name: String,
    /// Resolved CSS property map: CSS property name → computed value string.
    pub properties: FxHashMap<String, String>,
    /// Pseudo-state overrides (e.g. `:hover`, `:focus`).
    pub pseudo_states: Vec<PseudoStateIR>,
    #[serde(default)]
    pub source_span: Option<Span>,
}

/// A pseudo-state variant of a class (e.g. `&:hover { ... }`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PseudoStateIR {
    pub pseudo: String,
    pub properties: FxHashMap<String, String>,
    #[serde(default)]
    pub source_span: Option<Span>,
}

/// A design system variant (e.g., `Button(color=primary, size=md)`).
/// NOT to be confused with [`logic::VariantIR`] for tagged union variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantIR {
    pub name: String,
    pub dimensions: Vec<VariantDimIR>,
    pub defaults: FxHashMap<String, String>,
    pub base: FxHashMap<String, String>,
    pub branches: Vec<VariantBranchIR>,
    #[serde(default)]
    pub source_span: Option<Span>,
}

/// A dimension axis of a variant (e.g. `@dim color: primary | danger`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantDimIR {
    pub name: String,
    pub values: Vec<String>,
    #[serde(default)]
    pub source_span: Option<Span>,
}

/// A single branch of a variant (e.g. `color.primary { ... }`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantBranchIR {
    pub dimension: String,
    pub value: String,
    pub properties: FxHashMap<String, String>,
    #[serde(default)]
    pub source_span: Option<Span>,
}
