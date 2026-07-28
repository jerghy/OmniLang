//! Design IR types — lowered representation of `design` blocks.
//!
//! Pipeline: AST `DesignDef` → `DesignIR` (via lowering) → CSS codegen.
//!
//! All types derive Debug, Clone, Serialize, Deserialize for
//! JSON round-tripping in the IDE bridge and WASM bindings.

use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::lexer::Span;

/// A fully lowered design block.
///
/// Tokens are resolved into a `FxHashMap<String, TokenIR>` for O(1) lookup.
/// All `token(...)` references in class / variant properties have been
/// replaced with their resolved CSS values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignIR {
    pub name: String,
    /// Theme names declared via `@themes ...;` (e.g. `["light", "dark"]`).
    #[serde(default)]
    pub themes: Vec<String>,
    /// Optional default theme name set via `@default-theme ...;`.
    #[serde(default)]
    pub default_theme: Option<String>,
    pub tokens: FxHashMap<String, TokenIR>,
    pub classes: Vec<ClassIR>,
    pub variants: Vec<VariantIR>,
    #[serde(default)]
    pub source_span: Option<Span>,
}

/// A single design token with its resolved value and type annotation.
///
/// Per-theme overrides are stored as `theme_values: FxHashMap<String, String>`;
/// the outer `value` is the fallback used when no theme context matches.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenIR {
    pub value: String,
    pub token_type: TokenTypeIR,
    /// Per-theme value overrides keyed by theme name.
    #[serde(default)]
    pub theme_values: FxHashMap<String, String>,
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

/// A design variant — a parameterised group of CSS customisations
/// used by omni components for theming (e.g. `variant Button`).
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
