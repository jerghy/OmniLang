//! Intent IR type definitions — all types used in the IR layer.
//! Mirrors Python omniview/ir_types.py. Serializes to/from JSON via serde.

use crate::sourcemap::SourceMapEntry;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use rustc_hash::FxHashMap;
use strum::IntoStaticStr;

// Cross-reference: ComponentIR, StoreIR, Enums need logic types
use super::logic::*;

use super::types_motion::*;

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
/// Serde round-trips as a flat string for backward compatibility
/// (e.g. `"kind": "int"` or `"kind": "MyCustomType"`).
#[derive(Debug, Clone)]
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
            "string" => Self::String,
            "int" => Self::Int,
            "float" => Self::Float,
            "bool" => Self::Bool,
            "void" => Self::Void,
            "any" => Self::Any,
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
            Self::Named { name } => name.as_str(),
        }
    }
}

// Custom Serialize: write as a flat string  (e.g. `"int"`, `"MyType"`)
impl Serialize for TypeKind {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

// Custom Deserialize: read a flat string and convert via From<String>
impl<'de> Deserialize<'de> for TypeKind {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(TypeKind::from(s))
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
    pub owner: Option<String>,
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
        #[serde(default)]
        event_data: Option<EventDataKind>,
    },
}

impl EventHandlerIR {
    /// Returns the event_data field across all variants that have it.
    pub fn event_data(&self) -> &Option<EventDataKind> {
        match self {
            Self::FnRef { event_data, .. } => event_data,
            Self::InlineFn { event_data, .. } => event_data,
            Self::StoreAction { event_data, .. } => event_data,
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
            Self::StoreAction { .. } => false,
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
            Self::StoreAction { .. } => false,
        }
    }
}

// ═══════════════════════════════════════════════════════
// Event Data Schema — cross-target event payload definitions
// ═══════════════════════════════════════════════════════

/// Event payload categories. Each variant has a fixed set of fields
/// defined in `event_data_schema()`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventDataKind {
    PointerEvent,
    TouchEvent,
    WheelEvent,
    KeyEvent,
    ScrollEvent,
    InputEvent,
    ToggleEvent,
    SubmitEvent,
    FocusEvent,
    ClipboardEvent,
    DragEvent,
    DropEvent,
    ContextMenuEvent,
    SelectEvent,
    ResizeEvent,
    MediaEvent,
    MediaLoadEvent,
    MediaErrorEvent,
    AnimationEvent,
    TransitionEvent,
}

/// Typed event field type — replaces the old freeform string.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EventFieldType {
    Float,
    Int,
    Bool,
    String,
    TouchArray,
    StringArray,
    StringMap,
}

impl EventFieldType {
    /// Return the string representation of this field type,
    /// matching the old freeform string format.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Float => "float",
            Self::Int => "int",
            Self::Bool => "bool",
            Self::String => "string",
            Self::TouchArray => "{x,y,id}[]",
            Self::StringArray => "string[]",
            Self::StringMap => "{string: string}",
        }
    }
}

impl std::fmt::Display for EventFieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A single field within an event data payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFieldDef {
    pub name: String,
    pub field_type: EventFieldType,
}

/// Full schema: kind -> ordered field definitions.
/// Used by both lowering (field validation) and codegen (extraction).
#[must_use]
pub fn event_data_schema(kind: &EventDataKind) -> Vec<EventFieldDef> {
    let f = |name: &str, field_type: EventFieldType| EventFieldDef {
        name: name.to_string(),
        field_type,
    };
    match kind {
        EventDataKind::PointerEvent => vec![
            f("x", EventFieldType::Float),
            f("y", EventFieldType::Float),
            f("button", EventFieldType::Int),
            f("pressure", EventFieldType::Float),
            f("tiltX", EventFieldType::Float),
            f("tiltY", EventFieldType::Float),
            f("pointerType", EventFieldType::String),
            f("alt", EventFieldType::Bool),
            f("ctrl", EventFieldType::Bool),
            f("shift", EventFieldType::Bool),
            f("meta", EventFieldType::Bool),
        ],
        EventDataKind::TouchEvent => vec![
            f("touches", EventFieldType::TouchArray),
            f("alt", EventFieldType::Bool),
            f("ctrl", EventFieldType::Bool),
            f("shift", EventFieldType::Bool),
            f("meta", EventFieldType::Bool),
        ],
        EventDataKind::WheelEvent => vec![
            f("x", EventFieldType::Float),
            f("y", EventFieldType::Float),
            f("deltaX", EventFieldType::Float),
            f("deltaY", EventFieldType::Float),
            f("deltaZ", EventFieldType::Float),
            f("deltaMode", EventFieldType::Int),
            f("alt", EventFieldType::Bool),
            f("ctrl", EventFieldType::Bool),
            f("shift", EventFieldType::Bool),
            f("meta", EventFieldType::Bool),
        ],
        EventDataKind::KeyEvent => vec![
            f("key", EventFieldType::String),
            f("code", EventFieldType::String),
            f("alt", EventFieldType::Bool),
            f("ctrl", EventFieldType::Bool),
            f("shift", EventFieldType::Bool),
            f("meta", EventFieldType::Bool),
        ],
        EventDataKind::ScrollEvent => vec![f("x", EventFieldType::Float), f("y", EventFieldType::Float)],
        EventDataKind::InputEvent => vec![f("value", EventFieldType::String), f("checked", EventFieldType::Bool)],
        EventDataKind::ToggleEvent => vec![f("checked", EventFieldType::Bool)],
        EventDataKind::SubmitEvent => vec![f("values", EventFieldType::StringMap)],
        EventDataKind::FocusEvent => vec![f("value", EventFieldType::String)],
        EventDataKind::ClipboardEvent => vec![f("text", EventFieldType::String)],
        EventDataKind::DragEvent => vec![
            f("x", EventFieldType::Float),
            f("y", EventFieldType::Float),
            f("alt", EventFieldType::Bool),
            f("ctrl", EventFieldType::Bool),
            f("shift", EventFieldType::Bool),
            f("meta", EventFieldType::Bool),
        ],
        EventDataKind::DropEvent => vec![
            f("x", EventFieldType::Float),
            f("y", EventFieldType::Float),
            f("files", EventFieldType::StringArray),
            f("text", EventFieldType::String),
            f("alt", EventFieldType::Bool),
            f("ctrl", EventFieldType::Bool),
            f("shift", EventFieldType::Bool),
            f("meta", EventFieldType::Bool),
        ],
        EventDataKind::ContextMenuEvent => vec![
            f("x", EventFieldType::Float),
            f("y", EventFieldType::Float),
            f("alt", EventFieldType::Bool),
            f("ctrl", EventFieldType::Bool),
            f("shift", EventFieldType::Bool),
            f("meta", EventFieldType::Bool),
        ],
        EventDataKind::SelectEvent => vec![f("start", EventFieldType::Int), f("end", EventFieldType::Int), f("text", EventFieldType::String)],
        EventDataKind::ResizeEvent => vec![f("width", EventFieldType::Float), f("height", EventFieldType::Float)],
        EventDataKind::MediaEvent => vec![f("currentTime", EventFieldType::Float), f("duration", EventFieldType::Float)],
        EventDataKind::MediaLoadEvent => {
            vec![f("naturalWidth", EventFieldType::Float), f("naturalHeight", EventFieldType::Float)]
        }
        EventDataKind::MediaErrorEvent => vec![f("message", EventFieldType::String)],
        EventDataKind::AnimationEvent => {
            vec![f("animationName", EventFieldType::String), f("elapsedTime", EventFieldType::Float)]
        }
        EventDataKind::TransitionEvent => {
            vec![f("propertyName", EventFieldType::String), f("elapsedTime", EventFieldType::Float)]
        }
    }
}

/// Map .omni event name string -> EventDataKind.
/// Returns None for store-only or non-data events.
#[must_use]
pub fn map_event_data_kind(event_name: &str) -> Option<EventDataKind> {
    match event_name {
        "onClick" | "onPress" | "onTap" | "onDoubleClick" | "onLongPress" | "onMouseMove"
        | "onHover" | "onMouseEnter" | "onMouseLeave" | "onPointerDown" | "onPointerUp"
        | "onPointerMove" | "onPointerCancel" => Some(EventDataKind::PointerEvent),
        "onTouchStart" | "onTouchMove" | "onTouchEnd" | "onTouchCancel" => {
            Some(EventDataKind::TouchEvent)
        }
        "onWheel" => Some(EventDataKind::WheelEvent),
        "onKeyDown" | "onKeyUp" => Some(EventDataKind::KeyEvent),
        "onScroll" => Some(EventDataKind::ScrollEvent),
        "onChange" | "onInput" => Some(EventDataKind::InputEvent),
        "onToggle" => Some(EventDataKind::ToggleEvent),
        "onSubmit" => Some(EventDataKind::SubmitEvent),
        "onFocus" | "onBlur" => Some(EventDataKind::FocusEvent),
        "onCopy" | "onCut" | "onPaste" => Some(EventDataKind::ClipboardEvent),
        "onDragStart" | "onDrag" | "onDragEnd" => Some(EventDataKind::DragEvent),
        "onDrop" => Some(EventDataKind::DropEvent),
        "onContextMenu" => Some(EventDataKind::ContextMenuEvent),
        "onSelect" => Some(EventDataKind::SelectEvent),
        "onResize" => Some(EventDataKind::ResizeEvent),
        "onPlay" | "onPause" | "onEnded" => Some(EventDataKind::MediaEvent),
        "onLoad" => Some(EventDataKind::MediaLoadEvent),
        "onError" => Some(EventDataKind::MediaErrorEvent),
        "onAnimationStart" | "onAnimationEnd" | "onAnimationIteration" => {
            Some(EventDataKind::AnimationEvent)
        }
        "onTransitionEnd" => Some(EventDataKind::TransitionEvent),
        _ => None,
    }
}

/// Map EventDataKind to a string label for the @for JS kind-lookup map.
#[must_use]
pub fn event_data_kind_label(kind: &EventDataKind) -> &'static str {
    match kind {
        EventDataKind::PointerEvent => "PointerEvent",
        EventDataKind::TouchEvent => "TouchEvent",
        EventDataKind::WheelEvent => "WheelEvent",
        EventDataKind::KeyEvent => "KeyEvent",
        EventDataKind::ScrollEvent => "ScrollEvent",
        EventDataKind::InputEvent => "InputEvent",
        EventDataKind::ToggleEvent => "ToggleEvent",
        EventDataKind::SubmitEvent => "SubmitEvent",
        EventDataKind::FocusEvent => "FocusEvent",
        EventDataKind::ClipboardEvent => "ClipboardEvent",
        EventDataKind::DragEvent => "DragEvent",
        EventDataKind::DropEvent => "DropEvent",
        EventDataKind::ContextMenuEvent => "ContextMenuEvent",
        EventDataKind::SelectEvent => "SelectEvent",
        EventDataKind::ResizeEvent => "ResizeEvent",
        EventDataKind::MediaEvent => "MediaEvent",
        EventDataKind::MediaLoadEvent => "MediaLoadEvent",
        EventDataKind::MediaErrorEvent => "MediaErrorEvent",
        EventDataKind::AnimationEvent => "AnimationEvent",
        EventDataKind::TransitionEvent => "TransitionEvent",
    }
}

// ═══════════════════════════════════════════════════════
// Block IR & Statements
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockIR {
    pub statements: Vec<StatementIR>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "_kind")]
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

// ═══════════════════════════════════════════════════════
// Core Intent Types
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateIntent {
    pub name: String,
    pub r#type: TypeIR,
    pub initial_value: ExprIR,
    pub is_copy: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationOp {
    DirectAssign,
    Increment,
    Decrement,
    Multiply,
    Divide,
    Modulo,
    Push,
    Pop,
}

impl std::fmt::Display for MutationOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MutationOp::DirectAssign => write!(f, "direct_assign"),
            MutationOp::Increment => write!(f, "increment"),
            MutationOp::Decrement => write!(f, "decrement"),
            MutationOp::Multiply => write!(f, "multiply"),
            MutationOp::Divide => write!(f, "divide"),
            MutationOp::Modulo => write!(f, "modulo"),
            MutationOp::Push => write!(f, "push"),
            MutationOp::Pop => write!(f, "pop"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation")]
pub enum MutationIntent {
    #[serde(rename = "direct_assign")]
    DirectAssign {
        target: SignalRefIR,
        #[serde(default)]
        value: Option<Box<ExprIR>>,
    },
    #[serde(rename = "increment")]
    Increment {
        target: SignalRefIR,
        amount: Option<Box<ExprIR>>,
    },
    #[serde(rename = "decrement")]
    Decrement {
        target: SignalRefIR,
        amount: Option<Box<ExprIR>>,
    },
    #[serde(rename = "multiply")]
    Multiply {
        target: SignalRefIR,
        amount: Option<Box<ExprIR>>,
    },
    #[serde(rename = "divide")]
    Divide {
        target: SignalRefIR,
        amount: Option<Box<ExprIR>>,
    },
    #[serde(rename = "modulo")]
    Modulo {
        target: SignalRefIR,
        amount: Option<Box<ExprIR>>,
    },
    #[serde(rename = "push")]
    Push {
        target: SignalRefIR,
        item: Option<Box<ExprIR>>,
    },
    #[serde(rename = "pop")]
    Pop {
        target: SignalRefIR,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputedIntent {
    pub name: String,
    pub r#type: TypeIR,
    pub body: ExprIR,
    pub deps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectIntent {
    pub deps: Vec<String>,
    pub body: BlockIR,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleKind {
    Mount,
    Unmount,
    Effect,
}

impl std::fmt::Display for LifecycleKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LifecycleKind::Mount => write!(f, "mount"),
            LifecycleKind::Unmount => write!(f, "unmount"),
            LifecycleKind::Effect => write!(f, "effect"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleIntent {
    pub kind: LifecycleKind,
    pub body: BlockIR,
}

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
}

// ═══════════════════════════════════════════════════════
// ConstructionKind
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstructionKind {
    TagMapping,
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
    pub test_id: Option<String>,
    pub ref_name: Option<String>,
    pub construction_kind: ConstructionKind,
    /// Dynamic class map bindings: class_name → signal expression
    /// e.g. "visible" → IR for `isOpen` signal. Codegen emits
    /// __omni_effect(() => el.classList.toggle(name, signal.value))
    #[serde(default)]
    pub class_map_bindings: FxHashMap<String, ExprIR>,
    /// Source location in .omni file
    #[serde(default)]
    pub source_line: u32,
    #[serde(default)]
    pub source_col: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motion: Option<Box<MotionBehavior>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scroll: Option<Box<ScrollBehavior>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drag: Option<Box<DragBehavior>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hero: Option<Box<HeroBehavior>>,
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
    pub fn hero_behavior(&self) -> Option<&HeroBehavior> {
        self.hero.as_deref()
    }
}

// ═══════════════════════════════════════════════════════
// ViewTree recursive type
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum ViewTree {
    Element(ElementIR),
    Conditional(Box<ConditionalIntent>),
    ListRendering(Box<ListRenderingIntent>),
    /// A template interpolation `{ expr }` — the inner ExprIR is lowered from the AST,
    /// so codegen can emit proper JS for any expression (VarRef, if-else, binary, call, …).
    Interpolation(Box<ExprIR>),
    Text(String),
    /// A `<slot>` placeholder directive. `Some("name")` for named slots,
    /// `None` for the default/unnamed slot.
    Slot(Option<String>),
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComponentIR {
    pub name: String,
    /// Source file path relative to project root (e.g. "src/components/Counter.omni")
    /// Populated during multi-file compilation; used by codegen for output directory structure.
    #[serde(default)]
    pub source_file: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub generic_params: Vec<String>,
    pub props: FxHashMap<String, TypeIR>,
    /// Default values for props (keyed by prop name).
    /// Props without defaults must be provided by the parent.
    #[serde(default)]
    pub prop_defaults: FxHashMap<String, ExprIR>,
    pub states: Vec<StateIntent>,
    pub computeds: Vec<ComputedIntent>,
    pub effects: Vec<EffectIntent>,
    pub lifecycles: Vec<LifecycleIntent>,
    pub fns: Vec<FunctionIR>,
    pub store_refs: Vec<String>,
    pub refs: Vec<serde_json::Value>,
    /// Event declarations (e.g., `event onDelete: i32`)
    #[serde(default)]
    pub event_names: Vec<String>,
    pub view: Option<ViewTree>,
    pub styles: Vec<StyleRuleIR>,
    /// Component-level @if mode default
    #[serde(default)]
    pub if_mode_default: Option<String>,
    /// Hero definitions for cross-component shared element transitions
    #[serde(default)]
    pub hero_defs: Vec<HeroIntent>,
    /// Motion animation definitions (@motion blocks)
    #[serde(default)]
    pub motions: Vec<MotionIntent>,
    /// Timeline animation definitions (@timeline blocks)
    #[serde(default)]
    pub timelines: Vec<TimelineIntent>,
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
    pub actions: Vec<FunctionIR>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IRProgram {
    pub version: String,
    pub modules: FxHashMap<String, ModuleIR>,
    pub source_map: FxHashMap<String, SourceMapEntry>,
    /// Stdlib modules imported via `import { ... } from "std/<module>"`
    #[serde(default)]
    pub stdlib_imports: Vec<String>,
}

impl Default for IRProgram {
    fn default() -> Self {
        IRProgram {
            version: "0.1".to_string(),
            modules: FxHashMap::default(),
            source_map: FxHashMap::default(),
            stdlib_imports: Vec::new(),
        }
    }
}

impl IRProgram {
    /// Collect all components across all modules.
    #[must_use]
    pub fn all_components(&self) -> FxHashMap<&str, &ComponentIR> {
        self.modules
            .values()
            .flat_map(|m| m.components.iter())
            .map(|(k, v)| (k.as_str(), v))
            .collect()
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
