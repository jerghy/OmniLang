//! AST node types for `design` blocks in the OmniLang parser.
//!
//! A `design` block is a top-level construct that declares design tokens,
//! CSS-like classes, and variant dimensions.  These types are produced
//! by the parser (Task A2) and consumed during IR lowering (Phase G).

use serde::{Deserialize, Serialize};

use super::Expr;

/// Top-level design block: `design AppDesign { ... }`
///
/// Theme support (v2 域 3, design D1):
/// - `@themes { dark, light }` declares theme names -> `themes`
/// - `@default-theme light;` sets the fallback -> `default_theme`
/// - Tokens declare per-theme overrides via `TokenDef::theme_values`
/// - A top-level store signal marked `@theme` is the current theme mode;
///   the program-level `validate_theme` pass wires it into
///   `DesignIR.theme_signal` (Task 4.1 → 4.2)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DesignDef {
    pub name: String,
    pub tokens: Vec<TokenDef>,
    pub classes: Vec<DesignClassDef>,
    pub variants: Vec<VariantDef>,
    /// Theme names declared via `@themes { ... }` (e.g. `["light", "dark"]`).
    #[serde(default)]
    pub themes: Vec<String>,
    /// Optional default theme name set via `@default-theme ...;`.
    #[serde(default)]
    pub default_theme: Option<String>,
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

/// A design token value -- static literal, compile-time variable reference,
/// or runtime expression (Phase 2).
///
/// Static: `token primary: color = "#6366f1";`
/// Variable ref: `token primary: color = BRAND;` (resolved at compile time)
/// Expression: `token primary: color = ThemeStore.primaryColor;` (Phase 2)
///
/// Note: `PartialEq` and `Deserialize` are implemented manually to avoid
/// requiring those traits on the recursive `Expr` type (used only by the
/// `Expression` variant).
#[derive(Debug, Clone, Serialize)]
pub enum TokenValue {
    /// Static literal value: `"#6366f1"`, `"16px"`, `white`
    Literal(String),
    /// Variable reference: `BRAND`, `SPACING` (resolved at compile time, Phase 1)
    Variable(String),
    /// Runtime expression: `ThemeStore.primaryColor`, `App_dynamicAccent` (Phase 2)
    #[serde(skip)]
    Expression(Expr),
}

impl PartialEq for TokenValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (TokenValue::Literal(a), TokenValue::Literal(b)) => a == b,
            (TokenValue::Variable(a), TokenValue::Variable(b)) => a == b,
            // Expression variants are not compared structurally (Phase 2+).
            _ => false,
        }
    }
}

/// Manual Deserialize for `TokenValue` to avoid requiring `Expr: Deserialize`.
///
/// Format (externally tagged, same as the serde derive would produce):
/// - `{"Literal": "str"}` -> `TokenValue::Literal(String)`
/// - `{"Variable": "str"}` -> `TokenValue::Variable(String)`
/// - `{"Expression": ...}` -> error (not deserializable yet, Phase 2)
impl<'de> serde::Deserialize<'de> for TokenValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};

        struct TokenValueVisitor;

        impl<'de> Visitor<'de> for TokenValueVisitor {
            type Value = TokenValue;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a TokenValue with Literal or Variable variant")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let key: String = map
                    .next_key()?
                    .ok_or_else(|| de::Error::custom("expected single-entry map"))?;

                match key.as_str() {
                    "Literal" => {
                        let value: String = map.next_value()?;
                        Ok(TokenValue::Literal(value))
                    }
                    "Variable" => {
                        let value: String = map.next_value()?;
                        Ok(TokenValue::Variable(value))
                    }
                    "Expression" => Err(de::Error::custom(
                        "TokenValue::Expression cannot be deserialized yet",
                    )),
                    other => Err(de::Error::custom(format!(
                        "unknown variant '{other}', expected Literal, Variable, or Expression"
                    ))),
                }
            }
        }

        deserializer.deserialize_map(TokenValueVisitor)
    }
}

/// `token primary: color = "#6366f1";`
///
/// Per-theme overrides appear as `theme_values` keyed by theme name:
/// ```text
/// token primary: color = "#6366f1" dark "#a5b4fc";
/// ```
/// The outer `value` is the fallback (used when no theme context matches).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenDef {
    pub name: String,
    pub token_type: TokenType,
    pub value: TokenValue,
    /// Per-theme value overrides: `(theme_name, value)` pairs.
    #[serde(default)]
    pub theme_values: Vec<(String, TokenValue)>,
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
