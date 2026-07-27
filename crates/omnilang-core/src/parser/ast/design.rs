//! AST node types for `design` blocks in the OmniLang parser.
//!
//! A `design` block is a top-level construct that declares design tokens,
//! CSS-like classes, and variant dimensions.  These types are produced
//! by the parser (Task A2) and consumed during IR lowering (Phase G).

use serde::{Deserialize, Serialize};

/// Top-level design block: `design AppDesign { ... }`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DesignDef {
    pub name: String,
    pub tokens: Vec<TokenDef>,
    pub classes: Vec<DesignClassDef>,
    pub variants: Vec<VariantDef>,
}

/// Token type annotation — 13 recognized types for design tokens.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TokenType {
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

/// `token primary: color = "#6366f1";`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenDef {
    pub name: String,
    pub token_type: TokenType,
    pub value: String,
}

/// `class Card { ... }`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DesignClassDef {
    pub name: String,
    pub properties: Vec<DesignProperty>,
    pub pseudo_states: Vec<PseudoState>,
}

/// A single CSS-like property: `background: token(surface);`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DesignProperty {
    pub name: String,
    pub value: DesignValue,
}

/// Value in a design property
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DesignValue {
    Literal(String),
    TokenRef(String),
    Mixed(Vec<DesignValuePart>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DesignValuePart {
    Text(String),
    Token(String),
}

/// `&:hover { ... }`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PseudoState {
    pub pseudo: String,
    pub properties: Vec<DesignProperty>,
}

/// `variant Button { @dim color: primary | danger; ... }`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VariantDef {
    pub name: String,
    pub dimensions: Vec<VariantDimension>,
    pub defaults: Vec<(String, String)>,
    pub base: Vec<DesignProperty>,
    pub branches: Vec<VariantBranch>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VariantDimension {
    pub name: String,
    pub values: Vec<String>,
}

/// `color.primary { background: token(primary); }`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VariantBranch {
    pub dimension: String,
    pub value: String,
    pub properties: Vec<DesignProperty>,
}
