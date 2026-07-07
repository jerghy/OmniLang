//! Design IR types — lowered representation of `design` blocks.
//!
//! Pipeline: AST `DesignDef` → `DesignIR` (via lowering) → CSS codegen.
//!
//! All types derive Debug, Clone, Serialize, Deserialize for
//! JSON round-tripping in the IDE bridge and WASM bindings.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A fully lowered design block.
///
/// Tokens are resolved into a `HashMap<String, TokenIR>` for O(1) lookup.
/// All `token(...)` references in class / variant properties have been
/// replaced with their resolved CSS values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignIR {
    pub name: String,
    pub tokens: HashMap<String, TokenIR>,
    pub classes: Vec<ClassIR>,
    pub variants: Vec<VariantIR>,
}

/// A single design token with its resolved value and type annotation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenIR {
    pub value: String,
    pub token_type: TokenTypeIR,
}

/// Token type annotation mirroring the AST `TokenType`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TokenTypeIR {
    Color,
    Size,
    Font,
    Shadow,
}

/// A CSS class with resolved property values and pseudo-state variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassIR {
    pub name: String,
    /// Resolved CSS property map: CSS property name → computed value string.
    pub properties: HashMap<String, String>,
    /// Pseudo-state overrides (e.g. `:hover`, `:focus`).
    pub pseudo_states: Vec<PseudoStateIR>,
}

/// A pseudo-state variant of a class (e.g. `&:hover { ... }`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PseudoStateIR {
    pub pseudo: String,
    pub properties: HashMap<String, String>,
}

/// A design variant — a parameterised group of CSS customisations
/// used by omni components for theming (e.g. `variant Button`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantIR {
    pub name: String,
    pub dimensions: Vec<VariantDimIR>,
    pub defaults: HashMap<String, String>,
    pub base: HashMap<String, String>,
    pub branches: Vec<VariantBranchIR>,
}

/// A dimension axis of a variant (e.g. `@dim color: primary | danger`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantDimIR {
    pub name: String,
    pub values: Vec<String>,
}

/// A single branch of a variant (e.g. `color.primary { ... }`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantBranchIR {
    pub dimension: String,
    pub value: String,
    pub properties: HashMap<String, String>,
}
