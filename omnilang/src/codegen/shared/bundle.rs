//! FormatBundle trait — unified formatter access for codegen backends.
//!
//! Provides a single point of access to expression, statement, and type
//! formatters for a given target family, eliminating the need for individual
//! formatter field lookups.
//!
//! # Architecture
//!
//! ```text
//! FormatBundle  —  expression + statement + type formatters + metadata
//!   ↓ supertrait
//! UnifiedCodegen —  per-module & per-component generation strategies
//! ```

// Imports used only by the gated FormatBundle trait.
// Gated on the same features as the formatter traits themselves.
#[cfg(any(
    feature = "codegen-javascript",
    feature = "codegen-react",
    feature = "codegen-vue",
    feature = "codegen-dart",
    feature = "codegen-flutter",
    feature = "codegen-rust",
    feature = "codegen-dioxus",
    feature = "codegen-python"
))]
use crate::codegen::json::JsonFormatConfig;
#[cfg(any(
    feature = "codegen-javascript",
    feature = "codegen-react",
    feature = "codegen-vue",
    feature = "codegen-dart",
    feature = "codegen-flutter",
    feature = "codegen-rust",
    feature = "codegen-dioxus",
    feature = "codegen-python"
))]
use crate::codegen::shared::expr::FormatExpr;
#[cfg(any(
    feature = "codegen-javascript",
    feature = "codegen-react",
    feature = "codegen-vue",
    feature = "codegen-dart",
    feature = "codegen-flutter",
    feature = "codegen-rust",
    feature = "codegen-dioxus",
    feature = "codegen-python"
))]
use crate::codegen::shared::stmt::FormatStmt;
#[cfg(any(
    feature = "codegen-javascript",
    feature = "codegen-react",
    feature = "codegen-vue",
    feature = "codegen-dart",
    feature = "codegen-flutter",
    feature = "codegen-rust",
    feature = "codegen-dioxus",
    feature = "codegen-python"
))]
use crate::codegen::shared::types::FormatType;
#[cfg(any(
    feature = "codegen-javascript",
    feature = "codegen-react",
    feature = "codegen-vue",
    feature = "codegen-dart",
    feature = "codegen-flutter",
    feature = "codegen-rust",
    feature = "codegen-dioxus",
    feature = "codegen-python"
))]
use crate::codegen::target::Target;

// ═══════════════════════════════════════════════════════════
// Family enum (no feature gate — always available)
// ═══════════════════════════════════════════════════════════

/// Code generation family identifier.
///
/// Each family shares common formatting conventions and structural patterns.
/// Multiple concrete targets may belong to the same family (e.g., JavaScript,
/// TypeScript, React, Vue all belong to the `Js` family).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Family {
    /// JavaScript family: JavaScript, TypeScript, React, Vue, OmniView.
    Js,
    /// Dart family: Dart, Flutter.
    Dart,
    /// Rust family: Rust, Dioxus.
    Rust,
    /// Python family: Python.
    Python,
    /// OmniView family (JS-based with unique formatting and runtime).
    OmniView,
}

impl Family {
    /// Return all known family variants in a static slice.
    #[must_use]
    pub const fn all() -> &'static [Family] {
        &[Family::Js, Family::Dart, Family::Rust, Family::Python, Family::OmniView]
    }
}

// ═══════════════════════════════════════════════════════════
// FormatBundle trait (gated — requires codegen features)
// ═══════════════════════════════════════════════════════════

/// Unified formatter bundle for codegen backends.
///
/// Implementors provide access to all three formatter types
/// (expression, statement, type) along with family and target metadata.
///
/// This trait is the foundation of the unified codegen architecture.
/// Backends implement it once and then only override per-item generation
/// methods in `UnifiedCodegen`.
#[cfg(any(
    feature = "codegen-javascript",
    feature = "codegen-react",
    feature = "codegen-vue",
    feature = "codegen-dart",
    feature = "codegen-flutter",
    feature = "codegen-rust",
    feature = "codegen-dioxus",
    feature = "codegen-python"
))]
pub(crate) trait FormatBundle: Send + Sync {
    /// The expression formatter type for this backend.
    type Expr: FormatExpr;

    /// The statement formatter type for this backend.
    type Stmt: FormatStmt<ExprFormatter = Self::Expr>;

    /// The type formatter type for this backend.
    type Type: FormatType;

    /// Return the family identifier for this backend.
    fn family(&self) -> Family;

    /// Return the concrete target for this backend (e.g., `Target::React`).
    fn target(&self) -> Target;

    /// Return a reference to the expression formatter.
    fn expr_fmt(&self) -> &Self::Expr;

    /// Return a reference to the statement formatter.
    fn stmt_fmt(&self) -> &Self::Stmt;

    /// Return a reference to the type formatter.
    fn type_fmt(&self) -> &Self::Type;

    /// Return the JSON formatting config for this backend.
    fn json_config(&self) -> &'static JsonFormatConfig;
}
