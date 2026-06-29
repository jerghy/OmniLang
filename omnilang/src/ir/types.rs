//! Intent IR type definitions — all types used in the IR layer.
//! Mirrors Python omniview/ir_types.py. Serializes to/from JSON via serde.

use crate::sourcemap::SourceMapEntry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Cross-reference: ComponentIR, StoreIR, Enums need logic types
use super::logic::*;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventHandlerIR {
    /// "fn_ref" | "inline_fn" | "store_action"
    pub kind: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleIntent {
    pub kind: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentIR {
    pub name: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreIR {
    pub name: String,
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
    pub fn all_components(&self) -> HashMap<&str, &ComponentIR> {
        self.modules
            .values()
            .flat_map(|m| m.components.iter())
            .map(|(k, v)| (k.as_str(), v))
            .collect()
    }

    /// Collect all public functions across all modules.
    pub fn all_functions(&self) -> Vec<&FunctionIR> {
        self.modules
            .values()
            .flat_map(|m| m.functions.values())
            .filter(|f| f.visibility == "public")
            .collect()
    }

    /// Collect all stores across all modules.
    pub fn all_stores(&self) -> HashMap<&str, &StoreIR> {
        self.modules
            .values()
            .flat_map(|m| m.stores.iter())
            .map(|(k, v)| (k.as_str(), v))
            .collect()
    }
}

// ═══════════════════════════════════════════════════════
// Motion / Animation IR
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionIntent {
    pub name: String,
    pub body: MotionBodyIR,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MotionBodyIR {
    Keyframe(KeyframeIR),
    Timeline(TimelineIR),
    CallbackOnly(CallbackBodyIR),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyframeIR {
    pub stops: Vec<KeyframeStopIR>,
    pub over: ExprIR,
    pub ease: Option<EasingIR>,
    pub color_space: Option<String>,
    pub playback: PlaybackIR,
    /// Path for motion path animation
    #[serde(default)]
    pub path: Option<String>,
    /// Whether to rotate element along the path
    #[serde(default)]
    pub path_rotate: Option<bool>,
    /// Callback blocks (onStart, onUpdate, onComplete, onRepeat, onInterrupt)
    #[serde(default)]
    pub callbacks: Vec<(CallbackKindIR, ExprIR)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyframeStopIR {
    pub position: f64,
    pub properties: HashMap<String, ExprIR>,
    pub ease: Option<EasingIR>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineIR {
    pub items: Vec<TimelineItemIR>,
    pub playback: PlaybackIR,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineItemIR {
    pub kind: TimelineItemKindIR,
    pub connector: ConnectorIR,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimelineItemKindIR {
    Direction {
        dir: DirectionIR,
        properties: HashMap<String, ExprIR>,
        over: ExprIR,
        ease: Option<EasingIR>,
        delay: Option<ExprIR>,
        color_space: Option<String>,
        start: Option<String>,
        /// Path for motion path animation
        path: Option<String>,
        /// Whether to rotate element along the path
        path_rotate: Option<bool>,
    },
    Set(Vec<(String, ExprIR)>),
    MotionRef(String),
    Stagger(Box<MotionExprIR>, StaggerTimingIR, StaggerDirection),
    Label(String, Option<ExprIR>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DirectionIR {
    To,
    FromTo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectorIR {
    None,
    Then,
    With,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallbackBodyIR {
    pub callbacks: Vec<(CallbackKindIR, ExprIR)>,
    pub over: ExprIR,
    pub playback: PlaybackIR,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CallbackKindIR {
    OnStart,
    OnUpdate,
    OnComplete,
    OnRepeat,
    OnInterrupt,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlaybackIR {
    pub loop_count: Option<LoopValue>,
    pub direction: Option<DirectionValue>,
    pub fill: Option<FillValue>,
    pub delay: Option<ExprIR>,
    pub paused: bool,
    pub repeat_delay: Option<ExprIR>,
    pub repeat_refresh: bool,
    pub time_scale: Option<ExprIR>,
    pub interrupt: Option<InterruptValue>,
    pub respect_reduced_motion: bool,
    pub when: Option<WhenValue>,
    pub delay_children: Option<ExprIR>,
    pub stagger_children: Option<ExprIR>,
    pub stagger_from: Option<StaggerDirection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EasingIR {
    Named(String),
    CubicBezier {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
    },
    Spring {
        tension: f64,
        friction: f64,
        velocity: Option<f64>,
    },
    Back {
        intensity: f64,
        direction: Option<String>,
    },
    Elastic {
        amplitude: f64,
        period: f64,
        direction: Option<String>,
    },
    Steps {
        count: u32,
        direction: String,
    },
}

/// Hero intent for cross-component shared element transitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeroIntent {
    pub name: String,
    pub over: ExprIR,
    pub ease: Option<EasingIR>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerIntent {
    pub kind: TriggerKind,
    pub motion_ref: Option<String>,
    pub inline_motion: Option<MotionBodyIR>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TriggerKind {
    Enter,
    Leave,
    Hover,
    Unhover,
    Press,
    Focus,
    Animate,
    InView,
    Scroll,
    DragHorizontal,
    DragVertical,
    Timeline,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatesIR {
    pub states: HashMap<String, StateTweenIR>,
    pub signal: Option<ExprIR>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTweenIR {
    pub properties: HashMap<String, ExprIR>,
    pub over: Option<ExprIR>,
    pub ease: Option<EasingIR>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MotionExprIR {
    Ref(String),
    Inline(MotionBodyIR),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineIntent {
    pub name: String,
    pub items: Vec<TimelineItemIR>,
}

/// Value type for loop_count.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoopValue {
    Infinite,
    Count(u32),
}

/// Value type for playback direction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DirectionValue {
    Normal,
    Reverse,
    Alternate,
    AlternateReverse,
}

/// Value type for fill mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FillValue {
    None,
    Forwards,
    Backwards,
    Both,
}

/// Value type for interrupt behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InterruptValue {
    Compose,
    Cancel,
    Finish,
    Replace,
}

/// Value type for when condition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WhenValue {
    Idle,
    Active,
    IdleOrActive,
}

/// Value type for stagger direction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StaggerDirection {
    First,
    Last,
    Center,
    Edges,
    Random,
}

/// Timing parameters for stagger effects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaggerTimingIR {
    pub each: ExprIR,
    pub from: Option<StaggerDirection>,
}

#[cfg(test)]
mod event_data_tests {
    use super::*;

    #[test]
    fn test_map_event_data_kind_pointer() {
        assert_eq!(
            map_event_data_kind("onClick"),
            Some(EventDataKind::PointerEvent)
        );
        assert_eq!(
            map_event_data_kind("onMouseMove"),
            Some(EventDataKind::PointerEvent)
        );
        assert_eq!(
            map_event_data_kind("onPointerDown"),
            Some(EventDataKind::PointerEvent)
        );
    }

    #[test]
    fn test_map_event_data_kind_keyboard() {
        assert_eq!(
            map_event_data_kind("onKeyDown"),
            Some(EventDataKind::KeyEvent)
        );
        assert_eq!(
            map_event_data_kind("onKeyUp"),
            Some(EventDataKind::KeyEvent)
        );
    }

    #[test]
    fn test_map_event_data_kind_scroll() {
        assert_eq!(
            map_event_data_kind("onScroll"),
            Some(EventDataKind::ScrollEvent)
        );
    }

    #[test]
    fn test_map_event_data_kind_none_for_unknown() {
        assert_eq!(map_event_data_kind("onUnknown"), None);
    }

    #[test]
    fn test_schema_pointer_event_has_button_and_modifiers() {
        let fields = event_data_schema(&EventDataKind::PointerEvent);
        let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"x"));
        assert!(names.contains(&"y"));
        assert!(names.contains(&"button"));
        assert!(names.contains(&"alt"));
        assert!(names.contains(&"ctrl"));
    }

    #[test]
    fn test_schema_scroll_event_only_position() {
        let fields = event_data_schema(&EventDataKind::ScrollEvent);
        let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["x", "y"]);
    }

    #[test]
    fn test_schema_twenty_kinds() {
        // All 20 variants produce non-empty field lists
        let kinds = [
            EventDataKind::PointerEvent,
            EventDataKind::TouchEvent,
            EventDataKind::WheelEvent,
            EventDataKind::KeyEvent,
            EventDataKind::ScrollEvent,
            EventDataKind::InputEvent,
            EventDataKind::ToggleEvent,
            EventDataKind::SubmitEvent,
            EventDataKind::FocusEvent,
            EventDataKind::ClipboardEvent,
            EventDataKind::DragEvent,
            EventDataKind::DropEvent,
            EventDataKind::ContextMenuEvent,
            EventDataKind::SelectEvent,
            EventDataKind::ResizeEvent,
            EventDataKind::MediaEvent,
            EventDataKind::MediaLoadEvent,
            EventDataKind::MediaErrorEvent,
            EventDataKind::AnimationEvent,
            EventDataKind::TransitionEvent,
        ];
        assert_eq!(kinds.len(), 20);
        for kind in &kinds {
            assert!(
                !event_data_schema(kind).is_empty(),
                "Schema empty for {:?}",
                kind
            );
        }
    }
}
