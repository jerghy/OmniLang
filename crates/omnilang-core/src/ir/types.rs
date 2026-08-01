//! Intent IR type definitions — all types used in the IR layer.
//! Mirrors Python omniview/ir_types.py. Serializes to/from JSON via serde.

use crate::lexer::Span;
use crate::sourcemap::SourceMapEntry;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use strum::IntoStaticStr;

// Cross-reference: ComponentIR, StoreIR, Enums need logic types
use super::logic::*;

use super::types_motion::*;

// Re-export extracted types so `use crate::ir::types::*` still resolves them.
// Also brings them into scope for type definitions remaining in this file.
pub use super::events::*;
pub use super::signals::*;

fn is_false(b: &bool) -> bool {
    !*b
}

// ═══════════════════════════════════════════════════════
// BinaryOpIR — strongly-typed binary operator
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, IntoStaticStr)]
#[strum(serialize_all = "snake_case")]
pub enum BinaryOpIR {
    #[serde(rename = "add")]
    Add,
    #[serde(rename = "sub")]
    Sub,
    #[serde(rename = "mul")]
    Mul,
    #[serde(rename = "div")]
    Div,
    #[serde(rename = "mod")]
    Mod,
    #[serde(rename = "eq")]
    Eq,
    #[serde(rename = "neq")]
    Neq,
    #[serde(rename = "lt")]
    Lt,
    #[serde(rename = "gt")]
    Gt,
    #[serde(rename = "lteq")]
    #[strum(serialize = "lteq")]
    LtEq,
    #[serde(rename = "gteq")]
    #[strum(serialize = "gteq")]
    GtEq,
    #[serde(rename = "and")]
    And,
    #[serde(rename = "or")]
    Or,
    #[serde(rename = "??")]
    #[strum(serialize = "??")]
    NullishCoalesce,
    #[serde(rename = "assign")]
    Assign,
    #[serde(rename = "addassign")]
    #[strum(serialize = "addassign")]
    AddAssign,
    #[serde(rename = "subassign")]
    #[strum(serialize = "subassign")]
    SubAssign,
    #[serde(rename = "mulassign")]
    #[strum(serialize = "mulassign")]
    MulAssign,
    #[serde(rename = "divassign")]
    #[strum(serialize = "divassign")]
    DivAssign,
    #[serde(rename = "modassign")]
    #[strum(serialize = "modassign")]
    ModAssign,
}

impl BinaryOpIR {
    /// Return the string representation of this binary operator,
    /// matching `BinaryOpKind::as_ir_str()` from the AST.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        self.into()
    }
}

impl std::fmt::Display for BinaryOpIR {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ═══════════════════════════════════════════════════════
// UnaryOpIR — strongly-typed unary operator
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum UnaryOpIR {
    #[serde(rename = "!")]
    Not,
    #[serde(rename = "-")]
    Neg,
    #[serde(rename = "await")]
    Await,
}

impl UnaryOpIR {
    /// Return the string representation of this unary operator.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Not => "!",
            Self::Neg => "-",
            Self::Await => "await ",
        }
    }
}

impl std::fmt::Display for UnaryOpIR {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ═══════════════════════════════════════════════════════
// Expression IR
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "_kind")]
#[non_exhaustive]
pub enum ExprIR {
    LiteralExpr {
        value: serde_json::Value,
    },
    IdentifierExpr {
        name: String,
        #[serde(default)]
        type_ann: Option<String>,
    },
    FieldAccessExpr {
        object: Box<ExprIR>,
        field: String,
    },
    OptionalChainExpr {
        object: Box<ExprIR>,
        field: String,
    },
    BinaryOpExpr {
        op: BinaryOpIR,
        left: Box<ExprIR>,
        right: Box<ExprIR>,
    },
    UnaryOpExpr {
        op: UnaryOpIR,
        operand: Box<ExprIR>,
    },
    TernaryIR {
        cond: Box<ExprIR>,
        then_expr: Box<ExprIR>,
        else_expr: Box<ExprIR>,
    },
    CallExpr {
        name: String,
        args: Vec<ExprIR>,
    },
    ArrayExpr {
        elements: Vec<ExprIR>,
    },
    SpreadExpr {
        operand: Box<ExprIR>,
    },
    StructExpr {
        type_name: String,
        fields: Vec<(String, ExprIR)>,
    },
    StoreFieldAccessExpr {
        store_alias: String,
        field_name: String,
        call_args: Vec<ExprIR>,
    },
    MethodCallExpr {
        object: Box<ExprIR>,
        method: String,
        args: Vec<ExprIR>,
        /// Whether the call uses optional chaining: obj?.method(args)
        is_optional: bool,
    },
    /// Block expression in expression position — preserves all statements,
    /// with the last expression providing the value.
    BlockExpr {
        statements: Vec<StatementIR>,
        tail: Box<ExprIR>,
    },
    /// Array/map index access: arr[0] or map["key"]
    IndexExpr {
        object: Box<ExprIR>,
        index: Box<ExprIR>,
    },
    /// Closure / lambda expression: |params| body
    ClosureExpr {
        params: Vec<String>,
        body: Box<ExprIR>,
    },
    /// await expression: await <expr>
    AwaitExpr {
        operand: Box<ExprIR>,
    },
    /// Tagged union variant construction: Ok { value: 42 }
    VariantLiteral {
        union_type: String,
        variant: String,
        fields: Vec<(String, ExprIR)>,
    },
    /// match expression: match expr { Pattern => body, ... }
    MatchExpr {
        matched: Box<ExprIR>,
        arms: Vec<MatchArmIR>,
    },
    /// Map/Object literal expression: { key: expr, key2: expr2, ... }
    /// Unlike StructExpr, keys are runtime ExprIR (not compile-time strings),
    /// and there is no associated struct type name.
    MapExpr {
        entries: Vec<(ExprIR, ExprIR)>,
    },
    /// Semantic ref method call: `refName.focus()`, `refName.scrollTo(x, y)`, etc.
    /// Populated by the semantic pass from `@ref`-bound element method calls;
    /// consumed by codegen backends (Phase 4-6).
    RefOpExpr {
        ref_name: String,
        op: RefOpIR,
    },
}

impl ExprIR {
    /// Convenience constructor for IdentifierExpr with no type annotation.
    /// Reduces boilerplate in tests and internal code.
    #[must_use]
    pub fn ident(name: &str) -> Self {
        ExprIR::IdentifierExpr {
            name: name.to_string(),
            type_ann: None,
        }
    }
}

/// A single arm in a match expression IR.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchArmIR {
    pub pattern: MatchPatternIR,
    pub body: BlockIR,
    /// Tagged union type name (e.g. "HttpResult"), populated from variant_map during lowering.
    /// Used by codegen to emit the correct type prefix instead of hardcoded "UnionType".
    #[serde(default)]
    pub union_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchPatternIR {
    #[serde(rename = "_kind")]
    pub kind: MatchPatternKindIR,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MatchPatternKindIR {
    #[serde(rename = "variant")]
    Variant { name: String, bindings: Vec<String> },
    #[serde(rename = "wildcard")]
    Wildcard,
    #[serde(rename = "literal")]
    Literal { value: serde_json::Value },
}

// ═══════════════════════════════════════════════════════
// Type Kind Enum
// ═══════════════════════════════════════════════════════

/// Typed representation of a type kind, replacing the old `String` field.
///
/// Always serializes as a flat string (e.g. `"kind": "int"` or `"kind": "MyCustomType"`).
/// Generic type arguments are stored only in `TypeIR::type_args`.

#[derive(Debug, Clone, PartialEq)]
pub enum TypeKind {
    String,
    Int,
    Float,
    Bool,
    Void,
    Any,
    Named { name: String },
}

impl From<String> for TypeKind {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

impl From<&str> for TypeKind {
    fn from(s: &str) -> Self {
        match s {
            "string" | "String" => Self::String,
            "int" | "Int" | "i32" | "I32" | "i64" | "I64" => Self::Int,
            "float" | "Float" | "f32" | "F32" | "f64" | "F64" => Self::Float,
            "bool" | "Bool" | "boolean" | "Boolean" => Self::Bool,
            "void" | "Void" => Self::Void,
            "any" | "Any" => Self::Any,
            _ => Self::Named {
                name: s.to_string(),
            },
        }
    }
}

impl TypeKind {
    /// Return the string representation of this type kind.
    /// For `Named` variants, returns the inner name directly.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::String => "string",
            Self::Int => "int",
            Self::Float => "float",
            Self::Bool => "bool",
            Self::Void => "void",
            Self::Any => "any",
            Self::Named { name, .. } => name.as_str(),
        }
    }
}

// Custom Serialize: always write as a flat string.
// Generic type arguments are stored in TypeIR::type_args, not in TypeKind.
impl Serialize for TypeKind {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

// Custom Deserialize: read either a flat string (new/old format) or {name, type_args} object
// (old format, backward compat — type_args are ignored here since they're stored in TypeIR).
impl<'de> Deserialize<'de> for TypeKind {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::{MapAccess, Visitor};

        struct TypeKindVisitor;

        impl<'de> Visitor<'de> for TypeKindVisitor {
            type Value = TypeKind;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a string or a {name, ...} object")
            }

            fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<TypeKind, E> {
                Ok(TypeKind::from(s.to_string()))
            }

            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<TypeKind, M::Error> {
                let mut name = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "name" => name = Some(map.next_value()?),
                        "type_args" => {
                            // Ignore — type_args live only in TypeIR::type_args now
                            let _: serde::de::IgnoredAny = map.next_value()?;
                        }
                        _ => {
                            let _: serde::de::IgnoredAny = map.next_value()?;
                        }
                    }
                }
                let name = name.ok_or_else(|| serde::de::Error::missing_field("name"))?;
                Ok(TypeKind::Named { name })
            }
        }

        deserializer.deserialize_any(TypeKindVisitor)
    }
}

// ═══════════════════════════════════════════════════════
// Type IR
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeIR {
    pub kind: TypeKind,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub type_args: Vec<TypeIR>,
    #[serde(skip_serializing_if = "is_false", default)]
    pub is_nullable: bool,
}

// ═══════════════════════════════════════════════════════
// Signal & Binding References
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalRefIR {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindingIR {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<TypeIR>,
}

// ═══════════════════════════════════════════════════════
// Event Handler IR
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EventHandlerIR {
    FnRef {
        r#ref: String,
        #[serde(default)]
        params: Vec<String>,
        #[serde(default)]
        event_data: Option<EventDataKind>,
        /// Fields of `e` actually referenced in the handler body.
        /// None = all fields (conservative, for unresolved fn_ref).
        /// Some(vec) = exact set; empty vec = no fields used (pass `{}`).
        #[serde(default)]
        used_fields: Option<Vec<String>>,
        #[serde(default)]
        needs_prevent_default: bool,
        #[serde(default)]
        needs_stop_propagation: bool,
    },
    InlineFn {
        body: BlockIR,
        #[serde(default)]
        params: Vec<String>,
        #[serde(default)]
        event_data: Option<EventDataKind>,
        /// Fields of `e` actually referenced in the handler body.
        /// None = all fields (conservative, for unresolved fn_ref).
        /// Some(vec) = exact set; empty vec = no fields used (pass `{}`).
        #[serde(default)]
        used_fields: Option<Vec<String>>,
        #[serde(default)]
        needs_prevent_default: bool,
        #[serde(default)]
        needs_stop_propagation: bool,
    },
    StoreAction {
        r#ref: String,
    },
}

impl EventHandlerIR {
    /// Returns the event_data field across all variants that have it.
    /// StoreAction never has event data.
    pub fn event_data(&self) -> &Option<EventDataKind> {
        match self {
            Self::FnRef { event_data, .. } => event_data,
            Self::InlineFn { event_data, .. } => event_data,
            Self::StoreAction { .. } => &None,
        }
    }

    /// Returns the used_fields field for FnRef and InlineFn variants.
    pub fn used_fields(&self) -> &Option<Vec<String>> {
        match self {
            Self::FnRef { used_fields, .. } => used_fields,
            Self::InlineFn { used_fields, .. } => used_fields,
            Self::StoreAction { .. } => &None,
        }
    }

    /// Returns the needs_prevent_default flag for FnRef and InlineFn variants.
    pub fn needs_prevent_default(&self) -> bool {
        match self {
            Self::FnRef {
                needs_prevent_default,
                ..
            } => *needs_prevent_default,
            Self::InlineFn {
                needs_prevent_default,
                ..
            } => *needs_prevent_default,
            _ => false,
        }
    }

    /// Returns the needs_stop_propagation flag for FnRef and InlineFn variants.
    pub fn needs_stop_propagation(&self) -> bool {
        match self {
            Self::FnRef {
                needs_stop_propagation,
                ..
            } => *needs_stop_propagation,
            Self::InlineFn {
                needs_stop_propagation,
                ..
            } => *needs_stop_propagation,
            _ => false,
        }
    }
}

// (EventDataKind, EventFieldType, EventFieldDef, event_data_schema,
// map_event_data_kind, and event_data_kind_label moved to events.rs)

// ═══════════════════════════════════════════════════════
// Block IR & Statements
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockIR {
    pub statements: Vec<StatementIR>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "_kind")]
#[non_exhaustive]
pub enum StatementIR {
    LetStmtIR {
        name: String,
        value: Option<ExprIR>,
        r#type: Option<TypeIR>,
    },
    IfStmtIR {
        condition: ExprIR,
        then_body: BlockIR,
        else_ifs: Vec<(ExprIR, BlockIR)>,
        else_body: Option<BlockIR>,
    },
    ReturnStmtIR {
        value: Option<ExprIR>,
    },
    MutationStmt {
        intent: Option<MutationIntent>,
    },
    EventEmitStmtIR {
        event_name: String,
        args: Vec<ExprIR>,
    },
    ExprStmtIR {
        expr: ExprIR,
    },
    BlockStmtIR {
        body: BlockIR,
    },
    WhileStmtIR {
        condition: ExprIR,
        body: BlockIR,
    },
    ForInStmtIR {
        var_name: String,
        iterable: ExprIR,
        body: BlockIR,
    },
    BreakStmtIR,
    ContinueStmtIR,
    ThrowStmtIR {
        expr: ExprIR,
    },
    TryCatchStmtIR {
        try_body: BlockIR,
        catch_var: Option<String>,
        catch_body: Option<BlockIR>,
        finally_body: Option<BlockIR>,
    },
}

// (SignalIntent, MutationOp, MutationIntent, ComputedIntent,
// EffectIntent, LifecycleKind, LifecycleIntent moved to signals.rs)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionalBranch {
    pub condition: ExprIR,
    pub body: Option<ViewTree>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionalIntent {
    pub branches: Vec<ConditionalBranch>,
    pub fallback: Option<ViewTree>,
    /// Transition mode for conditional rendering: "sync" | "wait"
    #[serde(default)]
    pub transition_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListRenderingIntent {
    pub iterable: ExprIR,
    pub item_binding: BindingIR,
    pub body: Option<ViewTree>,
    pub key_expr: Option<ExprIR>,
    pub empty_body: Option<ViewTree>,
    /// `@for item in list [scroll]` — hint for Flutter ListView.builder codegen.
    #[serde(default)]
    pub scroll_hint: bool,
}

// ═══════════════════════════════════════════════════════
// ConstructionKind
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstructionKind {
    Component,
    Conditional,
    List,
    Normal,
    Static,
}

// ═══════════════════════════════════════════════════════
// Element IR
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementIR {
    pub tag: String,
    pub props: FxHashMap<String, ExprIR>,
    pub bindings: FxHashMap<String, SignalRefIR>,
    pub events: FxHashMap<String, EventHandlerIR>,
    pub style_classes: Option<serde_json::Value>,
    pub inline_style: Option<serde_json::Value>,
    pub children: Vec<ViewTree>,
    pub construction_kind: ConstructionKind,
    /// Dynamic class map bindings: class_name → signal expression
    /// e.g. "visible" → IR for `isOpen` signal. Codegen emits
    /// __omni_effect(() => el.classList.toggle(name, signal.value))
    #[serde(default)]
    pub class_map_bindings: FxHashMap<String, ExprIR>,
    /// Source location in .omni file
    #[serde(default)]
    pub source_span: Option<Span>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motion: Option<Box<MotionBehavior>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scroll: Option<Box<ScrollBehavior>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drag: Option<Box<DragBehavior>>,
    /// Element ref binding name — connects this element to a component-level
    /// ref declaration. Maps to useRef (React), __omni_el wrapper (OmniView),
    /// or GlobalKey (Flutter).
    #[serde(default)]
    pub ref_binding: Option<String>,
    /// Semantic ref bindings extracted from `@ref`-bound elements:
    /// binding kind (e.g. "focus", "scroll-x", "scroll-y", "value") → expression.
    /// Populated during view lowering; validated by the semantic pass; consumed
    /// by codegen backends (Phase 4-6).
    #[serde(default)]
    pub semantic_bindings: FxHashMap<String, ExprIR>,
}

impl ElementIR {
    pub fn motion_behavior(&self) -> Option<&MotionBehavior> {
        self.motion.as_deref()
    }
    pub fn scroll_behavior(&self) -> Option<&ScrollBehavior> {
        self.scroll.as_deref()
    }
    pub fn drag_behavior(&self) -> Option<&DragBehavior> {
        self.drag.as_deref()
    }
}

// ═══════════════════════════════════════════════════════
// ViewTree recursive type
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(untagged)]
pub enum ViewTree {
    Element(Box<ElementIR>),
    Conditional(Box<ConditionalIntent>),
    ListRendering(Box<ListRenderingIntent>),
    /// A template interpolation `{ expr }` — the inner ExprIR is lowered from the AST,
    /// so codegen can emit proper JS for any expression (VarRef, if-else, binary, call, …).
    Interpolation(Box<ExprIR>),
    Text(String),
    /// A `<slot>` placeholder directive.
    Slot {
        /// Slot name — `None` for the default/unnamed slot.
        name: Option<String>,
        /// Fallback content rendered when the parent provides no content
        /// for this slot. Preserved from the AST `default_content` during
        /// lowering (was previously discarded).
        fallback: Vec<ViewTree>,
        /// Optional type constraint declared in the source (`@slot name: Type`).
        /// Lowered from the AST `type_constraint`.
        type_constraint: Option<TypeIR>,
        /// Data variable name derived from the slot name when a type
        /// constraint is present — used as the data variable name in codegen.
        data_param: Option<String>,
    },
    /// Inline list of child nodes (no wrapper element) — used for slot projection.
    Fragment(Vec<ViewTree>),
}

// ═══════════════════════════════════════════════════════
// RefOpIR — semantic element ref operations
// ═══════════════════════════════════════════════════════

/// Which geometric quantity a `ref.measure()` read (`ref.width`, `ref.height`,
/// `ref.position`) requests. Codegen appends the corresponding property to the
/// measure result (or returns the position object for `position`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasureWhat {
    Width,
    Height,
    Position,
}

/// A semantic operation invoked on an element bound via `@ref={name}`.
///
/// Lowered from AST ref method calls (e.g. `myRef.focus()`) by the semantic
/// pass into `ExprIR::RefOpExpr`. Each variant carries the expressions needed
/// by the target backend; `Option` fields are `None` when the source call
/// omitted that argument. `ExprIR` children are boxed to break the recursive
/// size cycle with `ExprIR::RefOpExpr`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum RefOpIR {
    /// `ref.focus()` — move focus to the element.
    Focus,
    /// `ref.blur()` — remove focus from the element.
    Blur,
    /// `ref.scrollTo(x?, y?)` — absolute scroll position.
    ScrollTo {
        x: Option<Box<ExprIR>>,
        y: Option<Box<ExprIR>>,
    },
    /// `ref.scrollBy(dx?, dy?)` — relative scroll delta.
    ScrollBy {
        dx: Option<Box<ExprIR>>,
        dy: Option<Box<ExprIR>>,
    },
    /// `ref.measure()` — read the element's bounding box.
    /// `what` records which dimension the source requested (`ref.width`,
    /// `ref.height`, or `ref.position`), so codegen can append the right
    /// property to the measure result.
    Measure { what: MeasureWhat },
    /// `ref.select()` — select the element's text content.
    Select,
    /// `ref.setSelection(start, end)` — select a range of text.
    SetSelection {
        start: Box<ExprIR>,
        end: Box<ExprIR>,
    },
    /// `ref.value` — read the element's current value (input/textarea).
    Value,
    /// `ref.setValue(value)` — set the element's value.
    SetValue { value: Box<ExprIR> },
    /// `ref.capturePointer(id)` — capture a pointer on the element.
    CapturePointer { id: Box<ExprIR> },
    /// `ref.releasePointer(id)` — release a captured pointer.
    ReleasePointer { id: Box<ExprIR> },
}

// ═══════════════════════════════════════════════════════
// Style & Function IR
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleRuleIR {
    pub class_name: String,
    pub properties: FxHashMap<String, ExprIR>,
    pub output_mode: String,
    #[serde(default)]
    pub is_tag_selector: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamIR {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<TypeIR>,
}

// ═══════════════════════════════════════════════════════
// Top-Level IR
// ═══════════════════════════════════════════════════════

/// A DOM element ref declaration (e.g., `ref myButton: HtmlElement`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefDeclIR {
    pub name: String,
    /// The HTML element tag this ref is bound to (e.g., "input", "video", "canvas").
    /// Used by React codegen to emit the correct TypeScript type for useRef.
    /// None means default to HTMLElement (generic fallback).
    #[serde(default)]
    pub element_tag: Option<String>,
}

/// A declared component event with its TypeScript parameter type.
/// Replaces the parallel arrays event_names / event_param_types
/// to make the name->type relationship structural rather than positional.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventDeclIR {
    pub name: String,
    /// TypeScript parameter type string (e.g., "string", "number", "unknown").
    /// Empty string means untyped (emits "unknown" in generated .d.ts).
    #[serde(default)]
    pub param_type: String,
}

/// A component body slot declaration (`@slot name { fallback }`), ADR-053 D5.
/// Mirrors the fields of `ViewTree::Slot` so a declaration can be turned into
/// a `ViewTree::Slot` (for the component's own view) via `into_view_tree()`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SlotDeclIR {
    /// Slot name — `None` for the default/unnamed slot.
    pub name: Option<String>,
    /// Fallback content rendered when the parent provides no content
    /// for this slot.
    pub fallback: Vec<ViewTree>,
    /// Optional type constraint declared in the source (`@slot name: Type`).
    pub type_constraint: Option<TypeIR>,
    /// Data variable name derived from the slot name when a type
    /// constraint is present.
    pub data_param: Option<String>,
}

impl SlotDeclIR {
    /// Convert this declaration into a `ViewTree::Slot` placeholder. Used
    /// by view-level `@slot` directives, which lower to a Slot node in
    /// view-tree position from the shared lowering result. Component body
    /// declarations stay in `ComponentIR.slots` (consumed by codegen
    /// directly); inline-motion hoisting walks `ComponentIR.slots` itself.
    #[must_use]
    pub fn into_view_tree(self) -> ViewTree {
        ViewTree::Slot {
            name: self.name,
            fallback: self.fallback,
            type_constraint: self.type_constraint,
            data_param: self.data_param,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComponentIR {
    pub name: String,
    /// Source file path relative to project root (e.g. "src/components/Counter.omni")
    /// Populated during multi-file compilation; used by codegen for output directory structure.
    #[serde(default)]
    pub source_file: Option<String>,
    pub props: FxHashMap<String, TypeIR>,
    /// Default values for props (keyed by prop name).
    /// Props without defaults must be provided by the parent.
    #[serde(default)]
    pub prop_defaults: FxHashMap<String, ExprIR>,
    pub signals: Vec<SignalIntent>,
    pub computeds: Vec<ComputedIntent>,
    pub effects: Vec<EffectIntent>,
    pub lifecycles: Vec<LifecycleIntent>,
    pub fns: Vec<FunctionIR>,
    pub store_refs: Vec<String>,
    pub refs: Vec<RefDeclIR>,
    /// Component event declarations with their TypeScript parameter types.
    #[serde(default)]
    pub events: Vec<EventDeclIR>,
    /// Component body slot declarations (`@slot name { fallback }`),
    /// ADR-053 D5.
    #[serde(default)]
    pub slots: Vec<SlotDeclIR>,
    pub view: Option<ViewTree>,
    pub styles: Vec<StyleRuleIR>,
    /// Source location in .omni file
    #[serde(default)]
    pub source_span: Option<Span>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StoreIR {
    pub name: String,
    /// Source file path relative to project root (e.g. "src/stores/TodoStore.omni")
    /// Populated during multi-file compilation; used by codegen for output directory structure.
    #[serde(default)]
    pub source_file: Option<String>,
    pub signals: FxHashMap<String, TypeIR>,
    pub signal_defaults: FxHashMap<String, serde_json::Value>,
    pub getters: FxHashMap<String, ExprIR>,
    /// Return types for each getter, populated during lowering from GetterDecl.type_ann.
    #[serde(default)]
    pub getter_types: FxHashMap<String, TypeIR>,
    pub actions: Vec<FunctionIR>,
    /// Source location in .omni file
    #[serde(default)]
    pub source_span: Option<Span>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IRProgram {
    pub version: String,
    pub modules: FxHashMap<String, ModuleIR>,
    pub source_map: FxHashMap<String, SourceMapEntry>,
    /// Stdlib modules imported via `import { ... } from "std/<module>"`
    #[serde(default)]
    pub stdlib_imports: FxHashSet<String>,
    /// Source location in .omni file
    #[serde(default)]
    pub source_span: Option<Span>,
    /// Motion name → MotionIntent index for O(1) lookups.
    /// Built during lowering (lower/mod.rs) after all modules are processed.
    #[serde(skip)]
    pub motion_index: FxHashMap<String, MotionIntent>,
}

impl IRProgram {
    /// Find a motion definition by name (O(1) via `motion_index`).
    pub fn find_motion(&self, name: &str) -> Option<&MotionIntent> {
        self.motion_index.get(name)
    }
}

impl Default for IRProgram {
    fn default() -> Self {
        IRProgram {
            version: "0.1".to_string(),
            modules: FxHashMap::default(),
            source_map: FxHashMap::default(),
            stdlib_imports: FxHashSet::default(),
            source_span: None,
            motion_index: FxHashMap::default(),
        }
    }
}

impl IRProgram {
    /// Check whether a store exists by name across all modules (short-circuit).
    #[must_use]
    pub fn has_store(&self, name: &str) -> bool {
        self.modules.values().any(|m| m.stores.contains_key(name))
    }

    /// Iterate over all components across all modules without building a HashMap.
    pub fn iter_all_components(&self) -> impl Iterator<Item = (&str, &ComponentIR)> {
        self.modules
            .values()
            .flat_map(|m| m.components.iter())
            .map(|(k, v)| (k.as_str(), v))
    }

    /// Collect all components across all modules.
    #[must_use]
    pub fn all_components(&self) -> FxHashMap<&str, &ComponentIR> {
        self.iter_all_components().collect()
    }

    /// Collect all public functions across all modules.
    #[must_use]
    pub fn all_functions(&self) -> Vec<&FunctionIR> {
        self.modules
            .values()
            .flat_map(|m| m.functions.values())
            .filter(|f| f.visibility == Visibility::Public)
            .collect()
    }

    /// Collect all stores across all modules.
    #[must_use]
    pub fn all_stores(&self) -> FxHashMap<&str, &StoreIR> {
        self.modules
            .values()
            .flat_map(|m| m.stores.iter())
            .map(|(k, v)| (k.as_str(), v))
            .collect()
    }
}

#[cfg(test)]
#[path = "types_tests.rs"]
mod tests;
