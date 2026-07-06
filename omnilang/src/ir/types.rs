//! Intent IR type definitions — all types used in the IR layer.
//! Mirrors Python omniview/ir_types.py. Serializes to/from JSON via serde.

use crate::sourcemap::SourceMapEntry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Cross-reference: ComponentIR, StoreIR, Enums need logic types
use super::logic::*;

use super::types_motion::*;

fn is_false(b: &bool) -> bool {
    !*b
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
        op: String,
        left: Box<ExprIR>,
        right: Box<ExprIR>,
    },
    UnaryOpExpr {
        op: String,
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
}

/// A single arm in a match expression IR.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchArmIR {
    pub pattern: MatchPatternIR,
    pub body: BlockIR,
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
// Type IR
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeIR {
    pub kind: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub type_args: Vec<TypeIR>,
    #[serde(skip_serializing_if = "is_false", default)]
    pub is_nullable: bool,
    #[serde(skip_serializing_if = "is_false", default)]
    pub needs_clone: bool,
    #[serde(skip_serializing_if = "is_false", default)]
    pub needs_partial_eq: bool,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventHandlerKind {
    FnRef,
    InlineFn,
    StoreAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventHandlerIR {
    pub kind: EventHandlerKind,
    /// Inline handler body (None for fn_ref, store_action)
    pub body: Option<BlockIR>,
    /// Function name (fn_ref) or action path (store_action)
    pub r#ref: Option<String>,
    /// Inline handler declared parameter names
    #[serde(default)]
    pub params: Vec<String>,
    // ── Event data fields (new) ──
    /// Event payload category. None for store_action handlers.
    #[serde(default)]
    pub event_data: Option<EventDataKind>,
    /// Fields of `e` actually referenced in the handler body.
    /// None = all fields (conservative, for unresolved fn_ref).
    /// Some(vec) = exact set; empty vec = no fields used (pass `{}`).
    #[serde(default)]
    pub used_fields: Option<Vec<String>>,
    /// Handler explicitly calls e.preventDefault()
    #[serde(default)]
    pub needs_prevent_default: bool,
    /// Handler explicitly calls e.stopPropagation()
    #[serde(default)]
    pub needs_stop_propagation: bool,
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

/// A single field within an event data payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFieldDef {
    pub name: String,
    /// "float" | "string" | "bool" | "int" | "{x,y,id}[]" | "string[]"
    pub field_type: String,
}

/// Full schema: kind -> ordered field definitions.
/// Used by both lowering (field validation) and codegen (extraction).
#[must_use]
pub fn event_data_schema(kind: &EventDataKind) -> Vec<EventFieldDef> {
    let f = |name: &str, field_type: &str| EventFieldDef {
        name: name.to_string(),
        field_type: field_type.to_string(),
    };
    match kind {
        EventDataKind::PointerEvent => vec![
            f("x", "float"),
            f("y", "float"),
            f("button", "int"),
            f("pressure", "float"),
            f("tiltX", "float"),
            f("tiltY", "float"),
            f("pointerType", "string"),
            f("alt", "bool"),
            f("ctrl", "bool"),
            f("shift", "bool"),
            f("meta", "bool"),
        ],
        EventDataKind::TouchEvent => vec![
            f("touches", "{x,y,id}[]"),
            f("alt", "bool"),
            f("ctrl", "bool"),
            f("shift", "bool"),
            f("meta", "bool"),
        ],
        EventDataKind::WheelEvent => vec![
            f("x", "float"),
            f("y", "float"),
            f("deltaX", "float"),
            f("deltaY", "float"),
            f("deltaZ", "float"),
            f("deltaMode", "int"),
            f("alt", "bool"),
            f("ctrl", "bool"),
            f("shift", "bool"),
            f("meta", "bool"),
        ],
        EventDataKind::KeyEvent => vec![
            f("key", "string"),
            f("code", "string"),
            f("alt", "bool"),
            f("ctrl", "bool"),
            f("shift", "bool"),
            f("meta", "bool"),
        ],
        EventDataKind::ScrollEvent => vec![f("x", "float"), f("y", "float")],
        EventDataKind::InputEvent => vec![f("value", "string"), f("checked", "bool")],
        EventDataKind::ToggleEvent => vec![f("checked", "bool")],
        EventDataKind::SubmitEvent => vec![f("values", "{string: string}")],
        EventDataKind::FocusEvent => vec![f("value", "string")],
        EventDataKind::ClipboardEvent => vec![f("text", "string")],
        EventDataKind::DragEvent => vec![
            f("x", "float"),
            f("y", "float"),
            f("alt", "bool"),
            f("ctrl", "bool"),
            f("shift", "bool"),
            f("meta", "bool"),
        ],
        EventDataKind::DropEvent => vec![
            f("x", "float"),
            f("y", "float"),
            f("files", "string[]"),
            f("text", "string"),
            f("alt", "bool"),
            f("ctrl", "bool"),
            f("shift", "bool"),
            f("meta", "bool"),
        ],
        EventDataKind::ContextMenuEvent => vec![
            f("x", "float"),
            f("y", "float"),
            f("alt", "bool"),
            f("ctrl", "bool"),
            f("shift", "bool"),
            f("meta", "bool"),
        ],
        EventDataKind::SelectEvent => vec![f("start", "int"), f("end", "int"), f("text", "string")],
        EventDataKind::ResizeEvent => vec![f("width", "float"), f("height", "float")],
        EventDataKind::MediaEvent => vec![f("currentTime", "float"), f("duration", "float")],
        EventDataKind::MediaLoadEvent => {
            vec![f("naturalWidth", "float"), f("naturalHeight", "float")]
        }
        EventDataKind::MediaErrorEvent => vec![f("message", "string")],
        EventDataKind::AnimationEvent => {
            vec![f("animationName", "string"), f("elapsedTime", "float")]
        }
        EventDataKind::TransitionEvent => {
            vec![f("propertyName", "string"), f("elapsedTime", "float")]
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
    pub execution_mode: String,
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
    pub mutability: String,
    pub update_semantics: String,
    pub accessor_style: String,
    pub is_copy: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationIntent {
    pub target: SignalRefIR,
    pub operation: String,
    pub amount: Option<ExprIR>,
    pub item: Option<ExprIR>,
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
// Element IR
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementIR {
    pub tag: String,
    pub props: HashMap<String, ExprIR>,
    pub bindings: HashMap<String, SignalRefIR>,
    pub events: HashMap<String, EventHandlerIR>,
    pub style_classes: Option<serde_json::Value>,
    pub inline_style: Option<serde_json::Value>,
    pub children: Vec<ViewTree>,
    pub test_id: Option<String>,
    pub ref_name: Option<String>,
    pub construction_kind: String,
    /// Dynamic class map bindings: class_name → signal expression
    /// e.g. "visible" → IR for `isOpen` signal. Codegen emits
    /// __omni_effect(() => el.classList.toggle(name, signal.value))
    #[serde(default)]
    pub class_map_bindings: HashMap<String, ExprIR>,
    /// Source location in .omni file
    #[serde(default)]
    pub source_line: u32,
    #[serde(default)]
    pub source_col: u32,
    /// Motion animation triggers: on: hover, on: click, etc.
    #[serde(default)]
    pub motion_triggers: Vec<TriggerIntent>,
    /// States map: states: { visible: { ... }, hidden: { ... } }
    #[serde(default)]
    pub states: Option<StatesIR>,
    /// Paused override for animations (runtime expression)
    #[serde(default)]
    pub paused_override: Option<ExprIR>,
    /// Driven-by expression for animation
    #[serde(default)]
    pub driven_by: Option<ExprIR>,
    /// Hover reverse animation name
    #[serde(default)]
    pub hover_reverse: Option<String>,
    /// InView trigger: once expression
    #[serde(default)]
    pub in_view_once: Option<ExprIR>,
    /// InView trigger: repeat expression
    #[serde(default)]
    pub in_view_repeat: Option<ExprIR>,
    /// InView trigger: threshold expression
    #[serde(default)]
    pub in_view_threshold: Option<ExprIR>,
    /// Scroll sub-attributes
    #[serde(default)]
    pub scroll_start: Option<String>,
    #[serde(default)]
    pub scroll_end: Option<String>,
    #[serde(default)]
    pub scroll_scrub: Option<ExprIR>,
    /// Drag sub-attributes
    #[serde(default)]
    pub drag_constraints: Option<String>,
    #[serde(default)]
    pub drag_elastic: Option<ExprIR>,
    #[serde(default)]
    pub drag_scrub: Option<ExprIR>,
    #[serde(default)]
    pub drag_momentum: Option<ExprIR>,
    #[serde(default)]
    pub drag_threshold: Option<ExprIR>,
    #[serde(default)]
    pub drag_velocity_max: Option<ExprIR>,
    /// Hero tag and transition
    #[serde(default)]
    pub hero_tag: Option<String>,
    #[serde(default)]
    pub hero_transition: Option<ExprIR>,
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
    pub properties: HashMap<String, ExprIR>,
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
    pub props: HashMap<String, TypeIR>,
    /// Default values for props (keyed by prop name).
    /// Props without defaults must be provided by the parent.
    #[serde(default)]
    pub prop_defaults: HashMap<String, ExprIR>,
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
    pub signals: HashMap<String, TypeIR>,
    pub signal_defaults: HashMap<String, serde_json::Value>,
    pub getters: HashMap<String, ExprIR>,
    pub actions: Vec<FunctionIR>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IRProgram {
    pub version: String,
    pub modules: HashMap<String, ModuleIR>,
    pub source_map: HashMap<String, SourceMapEntry>,
    /// Stdlib modules imported via `import { ... } from "std/<module>"`
    #[serde(default)]
    pub stdlib_imports: Vec<String>,
}

impl Default for IRProgram {
    fn default() -> Self {
        IRProgram {
            version: "0.1".to_string(),
            modules: HashMap::new(),
            source_map: HashMap::new(),
            stdlib_imports: Vec::new(),
        }
    }
}

impl IRProgram {
    /// Collect all components across all modules.
    #[must_use]
    pub fn all_components(&self) -> HashMap<&str, &ComponentIR> {
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
    pub fn all_stores(&self) -> HashMap<&str, &StoreIR> {
        self.modules
            .values()
            .flat_map(|m| m.stores.iter())
            .map(|(k, v)| (k.as_str(), v))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_roundtrip_lifecycle_kind_mount() {
        let v = LifecycleKind::Mount;
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "\"mount\"");
        let back: LifecycleKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, LifecycleKind::Mount);
    }

    #[test]
    fn serde_roundtrip_lifecycle_kind_unmount() {
        let v = LifecycleKind::Unmount;
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "\"unmount\"");
        let back: LifecycleKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, LifecycleKind::Unmount);
    }

    #[test]
    fn serde_roundtrip_lifecycle_kind_effect() {
        let v = LifecycleKind::Effect;
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "\"effect\"");
        let back: LifecycleKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, LifecycleKind::Effect);
    }

    #[test]
    fn serde_roundtrip_event_handler_kind_fn_ref() {
        let v = EventHandlerKind::FnRef;
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "\"fn_ref\"");
        let back: EventHandlerKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, EventHandlerKind::FnRef);
    }

    #[test]
    fn serde_roundtrip_event_handler_kind_inline_fn() {
        let v = EventHandlerKind::InlineFn;
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "\"inline_fn\"");
        let back: EventHandlerKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, EventHandlerKind::InlineFn);
    }

    #[test]
    fn serde_roundtrip_event_handler_kind_store_action() {
        let v = EventHandlerKind::StoreAction;
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, "\"store_action\"");
        let back: EventHandlerKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, EventHandlerKind::StoreAction);
    }
}
