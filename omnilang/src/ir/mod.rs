//! Intent IR types and AST -> IR lowering.
//! Mirrors Python omniview/ir_types.py. Serializes to/from JSON via serde.

pub mod logic;
pub mod lower_logic;
pub use logic::*;

use std::cell::Cell;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::parser::ast::*;
use crate::semantic::SemanticResult;
use crate::sourcemap::SourceMapEntry;

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

fn is_false(b: &bool) -> bool {
    !*b
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
            f("x", "float"), f("y", "float"), f("button", "int"),
            f("pressure", "float"), f("tiltX", "float"), f("tiltY", "float"),
            f("pointerType", "string"),
            f("alt", "bool"), f("ctrl", "bool"), f("shift", "bool"), f("meta", "bool"),
        ],
        EventDataKind::TouchEvent => vec![
            f("touches", "{x,y,id}[]"),
            f("alt", "bool"), f("ctrl", "bool"), f("shift", "bool"), f("meta", "bool"),
        ],
        EventDataKind::WheelEvent => vec![
            f("x", "float"), f("y", "float"),
            f("deltaX", "float"), f("deltaY", "float"), f("deltaZ", "float"),
            f("deltaMode", "int"),
            f("alt", "bool"), f("ctrl", "bool"), f("shift", "bool"), f("meta", "bool"),
        ],
        EventDataKind::KeyEvent => vec![
            f("key", "string"), f("code", "string"),
            f("alt", "bool"), f("ctrl", "bool"), f("shift", "bool"), f("meta", "bool"),
        ],
        EventDataKind::ScrollEvent => vec![
            f("x", "float"), f("y", "float"),
        ],
        EventDataKind::InputEvent => vec![
            f("value", "string"), f("checked", "bool"),
        ],
        EventDataKind::ToggleEvent => vec![
            f("checked", "bool"),
        ],
        EventDataKind::SubmitEvent => vec![
            f("values", "{string: string}"),
        ],
        EventDataKind::FocusEvent => vec![
            f("value", "string"),
        ],
        EventDataKind::ClipboardEvent => vec![
            f("text", "string"),
        ],
        EventDataKind::DragEvent => vec![
            f("x", "float"), f("y", "float"),
            f("alt", "bool"), f("ctrl", "bool"), f("shift", "bool"), f("meta", "bool"),
        ],
        EventDataKind::DropEvent => vec![
            f("x", "float"), f("y", "float"),
            f("files", "string[]"), f("text", "string"),
            f("alt", "bool"), f("ctrl", "bool"), f("shift", "bool"), f("meta", "bool"),
        ],
        EventDataKind::ContextMenuEvent => vec![
            f("x", "float"), f("y", "float"),
            f("alt", "bool"), f("ctrl", "bool"), f("shift", "bool"), f("meta", "bool"),
        ],
        EventDataKind::SelectEvent => vec![
            f("start", "int"), f("end", "int"), f("text", "string"),
        ],
        EventDataKind::ResizeEvent => vec![
            f("width", "float"), f("height", "float"),
        ],
        EventDataKind::MediaEvent => vec![
            f("currentTime", "float"), f("duration", "float"),
        ],
        EventDataKind::MediaLoadEvent => vec![
            f("naturalWidth", "float"), f("naturalHeight", "float"),
        ],
        EventDataKind::MediaErrorEvent => vec![
            f("message", "string"),
        ],
        EventDataKind::AnimationEvent => vec![
            f("animationName", "string"), f("elapsedTime", "float"),
        ],
        EventDataKind::TransitionEvent => vec![
            f("propertyName", "string"), f("elapsedTime", "float"),
        ],
    }
}

/// Map .omni event name string -> EventDataKind.
/// Returns None for store-only or non-data events.
pub fn map_event_data_kind(event_name: &str) -> Option<EventDataKind> {
    match event_name {
        "onClick" | "onPress" | "onTap" | "onDoubleClick" | "onLongPress"
        | "onMouseMove" | "onHover" | "onMouseEnter" | "onMouseLeave"
        | "onPointerDown" | "onPointerUp" | "onPointerMove" | "onPointerCancel"
            => Some(EventDataKind::PointerEvent),
        "onTouchStart" | "onTouchMove" | "onTouchEnd" | "onTouchCancel"
            => Some(EventDataKind::TouchEvent),
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
        "onAnimationStart" | "onAnimationEnd" | "onAnimationIteration"
            => Some(EventDataKind::AnimationEvent),
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

// ═══════════════════════════════════════════════════════
// Lowering: AST -> IR
// ═══════════════════════════════════════════════════════

/// Lower AST -> IR.
pub fn lower(program: &Program, _semantics: &SemanticResult) -> IRProgram {
    let mut modules = HashMap::new();

    // Build variant → union name lookup for tagged union construction
    let mut variant_map: HashMap<String, String> = HashMap::new();
    for module in program.modules.values() {
        for tu in &module.tagged_unions {
            for variant in &tu.variants {
                variant_map.insert(variant.name.clone(), tu.name.clone());
            }
        }
    }

    for (module_name, module) in &program.modules {
        let mut components = HashMap::new();
        let mut stores = HashMap::new();
        let mut functions = HashMap::new();
        let mut structs = HashMap::new();
        let mut enums = HashMap::new();
        let mut type_aliases = HashMap::new();
        let mut imports: Vec<ImportIR> = Vec::new();
        let mut exports: Vec<ExportIR> = Vec::new();

        for comp in &module.components {
            let ir = lower_component(comp, &variant_map);
            components.insert(comp.name.clone(), ir);
        }

        for store in &module.stores {
            let ir = lower_store(store, &variant_map);
            stores.insert(store.name.clone(), ir);
        }

        for func in &module.functions {
            let ir = lower_logic::lower_function(func, &variant_map);
            functions.insert(func.name.clone(), ir);
        }

        for s in &module.structs {
            let ir = lower_logic::lower_struct(s);
            structs.insert(s.name.clone(), ir);
        }

        for tu in &module.tagged_unions {
            let ir = lower_logic::lower_enum(tu);
            enums.insert(tu.name.clone(), ir);
        }

        for ta in &module.type_aliases {
            let ir = lower_logic::lower_type_alias(ta);
            type_aliases.insert(ta.name.clone(), ir);
        }

        for imp in &module.imports {
            imports.push(lower_logic::lower_import(imp));
        }

        for exp in &module.exports {
            exports.push(lower_logic::lower_export(exp));
        }

        let motions: Vec<MotionIntent> = module
            .motions
            .iter()
            .map(|m| lower_motion_def(m, &variant_map))
            .collect();

        let timelines: Vec<TimelineIntent> = module
            .timelines
            .iter()
            .map(|t| lower_timeline_def(t, &variant_map))
            .collect();

        let module_ir = ModuleIR {
            name: module_name.clone(),
            functions,
            structs,
            enums,
            type_aliases,
            components,
            stores,
            themes: Vec::new(),
            imports,
            exports,
            motions,
            timelines,
        };

        modules.insert(module_name.clone(), module_ir);
    }

    // Collect stdlib imports: any "import { ... } from \"std/<module>\""
    let mut stdlib_imports: Vec<String> = Vec::new();
    for module in program.modules.values() {
        for imp in &module.imports {
            if let ImportStmt::Named(ni) = imp {
                if ni.path.starts_with("std/") {
                    let module_name = ni.path.strip_prefix("std/").unwrap().to_string();
                    if !stdlib_imports.contains(&module_name) {
                        stdlib_imports.push(module_name);
                    }
                }
            }
        }
    }

    IRProgram {
        version: "1.0".to_string(),
        modules,
        source_map: HashMap::new(),
        stdlib_imports,
    }
}

fn lower_component(comp: &ComponentDef, variant_map: &HashMap<String, String>) -> ComponentIR {
    let states: Vec<StateIntent> = comp
        .signals
        .iter()
        .map(|s| StateIntent {
            name: s.name.clone(),
            r#type: lower_type(&s.type_ann),
            initial_value: lower_expr(&s.init, variant_map),
            mutability: "mutable".into(),
            update_semantics: "setter-call".into(),
            accessor_style: "direct".into(),
            is_copy: false,
        })
        .collect();

    let mut computeds: Vec<ComputedIntent> = comp
        .computeds
        .iter()
        .map(|c| ComputedIntent {
            name: c.name.clone(),
            r#type: lower_type(&c.type_ann),
            body: lower_expr(&c.body, variant_map),
            deps: vec![],
        })
        .collect();

    // Lower inline-store getters into computed properties
    for g in &comp.getters {
        computeds.push(ComputedIntent {
            name: g.name.clone(),
            r#type: lower_type(&g.type_ann),
            body: lower_expr(&g.body, variant_map),
            deps: vec![],
        });
    }

    let effects: Vec<EffectIntent> = comp
        .lifecycles
        .iter()
        .filter(|lc| matches!(lc.kind, LifecycleKind::Effect))
        .map(|lc| EffectIntent {
            deps: lc.deps.clone(),
            body: lower_block(&lc.body, variant_map),
        })
        .collect();

    let lifecycles: Vec<LifecycleIntent> = comp
        .lifecycles
        .iter()
        .filter(|lc| !matches!(lc.kind, LifecycleKind::Effect))
        .map(|lc| LifecycleIntent {
            kind: match lc.kind {
                LifecycleKind::Mount => "mount".into(),
                LifecycleKind::Unmount => "unmount".into(),
                LifecycleKind::Effect => "effect".into(),
            },
            body: lower_block(&lc.body, variant_map),
        })
        .collect();

    let mut fns: Vec<FunctionIR> = comp
        .fns
        .iter()
        .map(|f| FunctionIR {
            name: f.name.clone(),
            visibility: "private".to_string(),
            params: f
                .params
                .iter()
                .map(|p| ParamIR {
                    name: p.name.clone(),
                    r#type: p.type_ann.as_ref().map(lower_type),
                })
                .collect(),
            return_type: f.return_type.as_ref().map(lower_type),
            body: lower_block(&f.body, variant_map),
            is_async: f.is_async,
        })
        .collect();
    // Also include component actions as regular functions
    for a in &comp.actions {
        fns.push(FunctionIR {
            name: a.name.clone(),
            visibility: "private".to_string(),
            params: a
                .params
                .iter()
                .map(|p| ParamIR {
                    name: p.name.clone(),
                    r#type: p.type_ann.as_ref().map(lower_type),
                })
                .collect(),
            return_type: a.return_type.as_ref().map(lower_type),
            body: lower_block(&a.body, variant_map),
            is_async: a.is_async,
        });
    }

    let props: HashMap<String, TypeIR> = comp
        .props
        .iter()
        .map(|p| (p.name.clone(), lower_type(&p.type_ann)))
        .collect();

    let prop_defaults: HashMap<String, ExprIR> = comp
        .props
        .iter()
        .filter_map(|p| {
            p.default
                .as_ref()
                .map(|d| (p.name.clone(), lower_expr(d, variant_map)))
        })
        .collect();

    // Collect input-binding signal names for <input> element auto-binding.
    // Only auto-bind signals whose name suggests they're for input binding.
    // If none exist, fall back to all String signals for backward compatibility.
    let input_signals: Vec<String> = {
        let string_signals: Vec<_> = comp.signals.iter()
            .filter(|s| matches!(s.type_ann, TypeAnnotation::Named { ref name, .. } if name == "String"))
            .map(|s| s.name.clone())
            .collect();
        // Prefer signals whose name suggests they're for input binding
        let candidates: Vec<_> = string_signals
            .iter()
            .filter(|n| n.contains("input") || n.contains("Input"))
            .cloned()
            .collect();
        if candidates.is_empty() { string_signals } else { candidates }
    };

    let mut view = comp
        .view
        .as_ref()
        .map(|v| lower_view_block(v, &input_signals, variant_map));

    let styles: Vec<StyleRuleIR> = comp
        .styles
        .iter()
        .map(|s| {
            let properties: HashMap<String, ExprIR> = s
                .properties
                .iter()
                .map(|(k, v)| {
                    let ir_val = match v {
                        StyleValue::Number(n) => {
                            // JSON does not support NaN or Infinity; fall back to 0
                            // for these edge cases (should not occur in well-formed source).
                            let num = if n.is_finite() {
                                serde_json::Number::from_f64(*n)
                                    .unwrap_or_else(|| serde_json::Number::from(0))
                            } else {
                                serde_json::Number::from(0)
                            };
                            ExprIR::LiteralExpr {
                                value: serde_json::Value::Number(num),
                            }
                        }
                        StyleValue::String(str_val) => ExprIR::LiteralExpr {
                            value: serde_json::Value::String(str_val.clone()),
                        },
                        StyleValue::Ident(id) => ExprIR::LiteralExpr {
                            value: serde_json::Value::String(id.clone()),
                        },
                        StyleValue::HexColor(h) => ExprIR::LiteralExpr {
                            value: serde_json::Value::String(h.clone()),
                        },
                        StyleValue::Dimension(val, unit) => ExprIR::LiteralExpr {
                            value: serde_json::Value::String(format!("{}{}", val, unit)),
                        },
                        StyleValue::List(parts) => {
                            let s: String = parts
                                .iter()
                                .map(|p| match p {
                                    StyleValuePart::Ident(id) => id.clone(),
                                    StyleValuePart::Number(n) => n.to_string(),
                                    StyleValuePart::Dimension(val, unit) => {
                                        format!("{}{}", val, unit)
                                    }
                                    StyleValuePart::HexColor(h) => h.clone(),
                                })
                                .collect::<Vec<_>>()
                                .join(" ");
                            ExprIR::LiteralExpr {
                                value: serde_json::Value::String(s),
                            }
                        }
                    };
                    (k.clone(), ir_val)
                })
                .collect();
            StyleRuleIR {
                class_name: s.class_name.clone(),
                properties,
                output_mode: "css".into(),
                is_tag_selector: s.is_tag_selector,
            }
        })
        .collect();

    let motions: Vec<MotionIntent> = comp
        .motions
        .iter()
        .map(|m| lower_motion_def(m, variant_map))
        .collect();

    let timelines: Vec<TimelineIntent> = comp
        .timelines
        .iter()
        .map(|t| lower_timeline_def(t, variant_map))
        .collect();

    let hero_defs: Vec<HeroIntent> = comp
        .hero_defs
        .iter()
        .map(|h| HeroIntent {
            name: h.name.clone(),
            over: lower_expr(&h.over, variant_map),
            ease: h.ease.as_ref().map(|e| lower_easing(e)),
        })
        .collect();

    // ── Pass 2: Backfill used_fields for fn_ref event handlers ──
    if let Some(ref mut view_tree) = view {
        backfill_fn_ref_event_fields(view_tree, &fns);
    }

    ComponentIR {
        name: comp.name.clone(),
        generic_params: vec![],
        props,
        prop_defaults,
        states,
        computeds,
        effects,
        lifecycles,
        fns,
        store_refs: comp.store_refs.clone(),
        refs: vec![],
        event_names: comp.events.iter().map(|e| e.name.clone()).collect(),
        view,
        styles,
        motions,
        timelines,
        if_mode_default: comp.if_mode_default.clone(),
        hero_defs,
    }
}

// ═══════════════════════════════════════════════════════
// Pass 2: Backfill used_fields for fn_ref event handlers
// ═══════════════════════════════════════════════════════

/// Pass 2: backfill used_fields for fn_ref event handlers by walking the
/// referenced function's IR body for `FieldAccessExpr { object: IdentifierExpr("e"), field }`.
fn backfill_fn_ref_event_fields(view: &mut ViewTree, fns: &[FunctionIR]) {
    match view {
        ViewTree::Element(ref mut el) => {
            for (_event_name, handler) in el.events.iter_mut() {
                if handler.used_fields.is_some()
                    || handler.event_data.is_none()
                    || handler.kind != "fn_ref"
                {
                    continue;
                }
                let kind = handler.event_data.as_ref().unwrap();
                let schema = event_data_schema(kind);
                let valid_names: Vec<&str> = schema.iter().map(|f| f.name.as_str()).collect();

                if let Some(ref fn_name) = handler.r#ref {
                    let func_ir = fns.iter().find(|f| f.name == *fn_name)
                        .or_else(|| {
                            fns.iter().find(|f| {
                                f.name == *fn_name
                                    || f.name.ends_with(&format!("_{}", fn_name))
                            })
                        });

                    if let Some(func) = func_ir {
                        let mut used = Vec::new();
                        collect_ir_event_fields(&func.body, &mut used);
                        let used: Vec<String> = used
                            .into_iter()
                            .filter(|f| valid_names.contains(&f.as_str()))
                            .collect();
                        handler.used_fields = Some(used);
                    }
                }
            }
            for child in &mut el.children {
                backfill_fn_ref_event_fields(child, fns);
            }
        }
        ViewTree::Conditional(ref mut cond) => {
            for branch in &mut cond.branches {
                if let Some(ref mut body) = branch.body {
                    backfill_fn_ref_event_fields(body, fns);
                }
            }
            if let Some(ref mut fallback) = cond.fallback {
                backfill_fn_ref_event_fields(fallback, fns);
            }
        }
        ViewTree::ListRendering(ref mut list) => {
            if let Some(ref mut body) = list.body {
                backfill_fn_ref_event_fields(body, fns);
            }
            if let Some(ref mut empty) = list.empty_body {
                backfill_fn_ref_event_fields(empty, fns);
            }
        }
        _ => {} // Interpolation, Text, Slot — no events
    }
}

/// Walk a BlockIR statement list for FieldAccessExpr where object is IdentifierExpr("e").
fn collect_ir_event_fields(block: &BlockIR, fields: &mut Vec<String>) {
    for stmt in &block.statements {
        match stmt {
            StatementIR::ExprStmtIR { expr } => {
                walk_expr_event_fields(expr, fields);
            }
            StatementIR::IfStmtIR {
                condition,
                then_body,
                else_ifs,
                else_body,
            } => {
                walk_expr_event_fields(condition, fields);
                collect_ir_event_fields(then_body, fields);
                for (_, block) in else_ifs {
                    collect_ir_event_fields(block, fields);
                }
                if let Some(block) = else_body {
                    collect_ir_event_fields(block, fields);
                }
            }
            StatementIR::LetStmtIR { value: Some(expr), .. } => {
                walk_expr_event_fields(expr, fields);
            }
            StatementIR::ReturnStmtIR { value: Some(expr) } => {
                walk_expr_event_fields(expr, fields);
            }
            StatementIR::BlockStmtIR { body } => {
                collect_ir_event_fields(body, fields);
            }
            StatementIR::WhileStmtIR { condition, body } => {
                walk_expr_event_fields(condition, fields);
                collect_ir_event_fields(body, fields);
            }
            StatementIR::ForInStmtIR { iterable, body, .. } => {
                walk_expr_event_fields(iterable, fields);
                collect_ir_event_fields(body, fields);
            }
            StatementIR::ThrowStmtIR { expr } => {
                walk_expr_event_fields(expr, fields);
            }
            StatementIR::TryCatchStmtIR {
                try_body,
                catch_body,
                finally_body,
                ..
            } => {
                collect_ir_event_fields(try_body, fields);
                if let Some(body) = catch_body {
                    collect_ir_event_fields(body, fields);
                }
                if let Some(body) = finally_body {
                    collect_ir_event_fields(body, fields);
                }
            }
            StatementIR::EventEmitStmtIR { args, .. } => {
                for arg in args {
                    walk_expr_event_fields(arg, fields);
                }
            }
            StatementIR::MutationStmt { intent: Some(intent) } => {
                if let Some(amount) = &intent.amount {
                    walk_expr_event_fields(amount, fields);
                }
                if let Some(item) = &intent.item {
                    walk_expr_event_fields(item, fields);
                }
            }
            _ => {} // BreakStmtIR, ContinueStmtIR, ReturnStmtIR(None), MutationStmt(None), LetStmtIR(None), etc.
        }
    }
}

/// Walk an ExprIR tree for `FieldAccessExpr { object: Box(IdentifierExpr("e")), field }`.
fn walk_expr_event_fields(expr: &ExprIR, fields: &mut Vec<String>) {
    match expr {
        ExprIR::FieldAccessExpr { object, field } => {
            if let ExprIR::IdentifierExpr { name } = object.as_ref() {
                if name == "e" && !fields.contains(field) {
                    fields.push(field.clone());
                }
            }
            walk_expr_event_fields(object, fields);
        }
        ExprIR::OptionalChainExpr { object, .. } => {
            walk_expr_event_fields(object, fields);
        }
        ExprIR::MethodCallExpr { object, args, .. } => {
            walk_expr_event_fields(object, fields);
            for arg in args {
                walk_expr_event_fields(arg, fields);
            }
        }
        ExprIR::CallExpr { args, .. } => {
            for arg in args {
                walk_expr_event_fields(arg, fields);
            }
        }
        ExprIR::BinaryOpExpr { left, right, .. } => {
            walk_expr_event_fields(left, fields);
            walk_expr_event_fields(right, fields);
        }
        ExprIR::UnaryOpExpr { operand, .. } => walk_expr_event_fields(operand, fields),
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            walk_expr_event_fields(cond, fields);
            walk_expr_event_fields(then_expr, fields);
            walk_expr_event_fields(else_expr, fields);
        }
        ExprIR::StructExpr { fields: map_fields, .. } => {
            for (_, v) in map_fields {
                walk_expr_event_fields(v, fields);
            }
        }
        ExprIR::ArrayExpr { elements, .. } => {
            for e in elements {
                walk_expr_event_fields(e, fields);
            }
        }
        ExprIR::IndexExpr { object, index } => {
            walk_expr_event_fields(object, fields);
            walk_expr_event_fields(index, fields);
        }
        ExprIR::ClosureExpr { body, .. } => {
            walk_expr_event_fields(body, fields);
        }
        ExprIR::BlockExpr { statements, tail } => {
            // Walk expressions in block expression statements
            for stmt in statements {
                match stmt {
                    StatementIR::ExprStmtIR { expr } => {
                        walk_expr_event_fields(expr, fields);
                    }
                    StatementIR::LetStmtIR { value: Some(expr), .. } => {
                        walk_expr_event_fields(expr, fields);
                    }
                    _ => {}
                }
            }
            walk_expr_event_fields(tail, fields);
        }
        ExprIR::AwaitExpr { operand } => walk_expr_event_fields(operand, fields),
        ExprIR::MatchExpr { matched, arms } => {
            walk_expr_event_fields(matched, fields);
            for arm in arms {
                collect_ir_event_fields(&arm.body, fields);
            }
        }
        ExprIR::VariantLiteral { fields: variant_fields, .. } => {
            for (_, v) in variant_fields {
                walk_expr_event_fields(v, fields);
            }
        }
        ExprIR::StoreFieldAccessExpr { call_args, .. } => {
            for arg in call_args {
                walk_expr_event_fields(arg, fields);
            }
        }
        _ => {} // LiteralExpr, IdentifierExpr — no sub-expressions
    }
}

fn lower_store(store: &StoreDef, variant_map: &HashMap<String, String>) -> StoreIR {
    let signals: HashMap<String, TypeIR> = store
        .signals
        .iter()
        .map(|s| (s.name.clone(), lower_type(&s.type_ann)))
        .collect();

    let signal_defaults: HashMap<String, serde_json::Value> = store
        .signals
        .iter()
        .map(|s| {
            let val = match &s.init {
                Expr::Literal(Literal::Int(n), _) => serde_json::Value::Number((*n).into()),
                Expr::Literal(Literal::Float(f), _) => {
                    serde_json::json!(f)
                }
                Expr::Literal(Literal::String(s), _) => serde_json::Value::String(s.clone()),
                Expr::Literal(Literal::Bool(b), _) => serde_json::Value::Bool(*b),
                Expr::Literal(Literal::Null, _) => serde_json::Value::Null,
                Expr::ArrayLiteral { elements, .. } => {
                    serde_json::Value::Array(elements.iter().map(expr_to_json_value).collect())
                }
                _ => {
                    // Non-literal initializer: serialize as a placeholder
                    // indicating the expression cannot be evaluated at compile time.
                    serde_json::Value::String("<runtime expr>".into())
                }
            };
            (s.name.clone(), val)
        })
        .collect();

    let getters: HashMap<String, ExprIR> = store
        .getters
        .iter()
        .map(|g| (g.name.clone(), lower_expr(&g.body, variant_map)))
        .collect();

    let actions: Vec<FunctionIR> = store
        .actions
        .iter()
        .map(|action| FunctionIR {
            name: action.name.clone(),
            visibility: "public".into(),
            params: action
                .params
                .iter()
                .map(|p| ParamIR {
                    name: p.name.clone(),
                    r#type: p.type_ann.as_ref().map(lower_type),
                })
                .collect(),
            return_type: action.return_type.as_ref().map(lower_type),
            body: lower_block(&action.body, variant_map),
            is_async: action.is_async,
        })
        .collect();

    StoreIR {
        name: store.name.clone(),
        signals,
        signal_defaults,
        getters,
        actions,
    }
}

/// Convert a simple AST expression to a JSON value (for store signal defaults).
fn expr_to_json_value(expr: &Expr) -> serde_json::Value {
    match expr {
        Expr::Literal(Literal::Int(n), _) => serde_json::Value::Number((*n).into()),
        Expr::Literal(Literal::Float(f), _) => serde_json::json!(f),
        Expr::Literal(Literal::String(s), _) => serde_json::Value::String(s.clone()),
        Expr::Literal(Literal::Bool(b), _) => serde_json::Value::Bool(*b),
        Expr::Literal(Literal::Null, _) => serde_json::Value::Null,
        Expr::ArrayLiteral { elements, .. } => {
            serde_json::Value::Array(elements.iter().map(expr_to_json_value).collect())
        }
        _ => serde_json::Value::String(format!("{:?}", expr)),
    }
}

fn lower_view_block(
    view: &ViewBlock,
    string_signals: &[String],
    variant_map: &HashMap<String, String>,
) -> ViewTree {
    if let Some(first) = view.children.first() {
        let sig_idx = Cell::new(0usize);
        lower_view_node(first, string_signals, Some(&sig_idx), variant_map)
    } else {
        ViewTree::Text("".into())
    }
}

/// Collects `e.field` references and `e.preventDefault()`/`e.stopPropagation()`
/// calls from an inline event handler AST body.  Used during IR lowering
/// to populate `EventHandlerIR.used_fields`.
struct EventFieldCollector {
    fields: Vec<String>,
    needs_prevent_default: bool,
    needs_stop_propagation: bool,
}

impl EventFieldCollector {
    fn collect(block: &crate::parser::ast::Block) -> Self {
        let mut c = Self {
            fields: vec![],
            needs_prevent_default: false,
            needs_stop_propagation: false,
        };
        for stmt in &block.stmts {
            c.visit_stmt(stmt);
        }
        c
    }

    fn visit_stmt(&mut self, stmt: &crate::parser::ast::Stmt) {
        match stmt {
            crate::parser::ast::Stmt::ExprStmt { expr, .. } => {
                self.visit_expr(expr);
            }
            crate::parser::ast::Stmt::VarDecl { init: Some(init), .. } => {
                self.visit_expr(init);
            }
            crate::parser::ast::Stmt::Assign { value, .. } => {
                self.visit_expr(value);
            }
            crate::parser::ast::Stmt::If {
                condition,
                then_body,
                else_ifs,
                else_body,
                ..
            } => {
                self.visit_expr(condition);
                self.visit_block(then_body);
                for (_, elsif_block) in else_ifs {
                    self.visit_block(elsif_block);
                }
                if let Some(else_b) = else_body {
                    self.visit_block(else_b);
                }
            }
            crate::parser::ast::Stmt::While { condition, body, .. } => {
                self.visit_expr(condition);
                self.visit_block(body);
            }
            crate::parser::ast::Stmt::ForIn { iterable, body, .. } => {
                self.visit_expr(iterable);
                self.visit_block(body);
            }
            crate::parser::ast::Stmt::Return { value: Some(value), .. } => {
                self.visit_expr(value);
            }
            crate::parser::ast::Stmt::Print { arg, .. } => {
                self.visit_expr(arg);
            }
            crate::parser::ast::Stmt::EventEmit { args, .. } => {
                for arg in args {
                    self.visit_expr(arg);
                }
            }
            crate::parser::ast::Stmt::Throw { expr, .. } => {
                self.visit_expr(expr);
            }
            crate::parser::ast::Stmt::TryCatch {
                try_body,
                catch_body,
                finally_body,
                ..
            } => {
                self.visit_block(try_body);
                if let Some(catch_b) = catch_body {
                    self.visit_block(catch_b);
                }
                if let Some(finally_b) = finally_body {
                    self.visit_block(finally_b);
                }
            }
            _ => {}
        }
    }

    fn visit_block(&mut self, block: &crate::parser::ast::Block) {
        for stmt in &block.stmts {
            self.visit_stmt(stmt);
        }
    }

    fn visit_expr(&mut self, expr: &crate::parser::ast::Expr) {
        use crate::parser::ast::Expr;
        match expr {
            // e.field
            Expr::FieldAccess { object, field, .. } => {
                if let Expr::VarRef(name, _) = object.as_ref() {
                    if name == "e" && !self.fields.contains(field) {
                        self.fields.push(field.clone());
                    }
                } else {
                    self.visit_expr(object);
                }
            }
            // e.method(args)
            Expr::MethodCall {
                object, method, args, ..
            } => {
                if let Expr::VarRef(name, _) = object.as_ref() {
                    if name == "e" {
                        match method.as_str() {
                            "preventDefault" => self.needs_prevent_default = true,
                            "stopPropagation" => self.needs_stop_propagation = true,
                            _ => {}
                        }
                    }
                } else {
                    self.visit_expr(object);
                }
                for arg in args {
                    self.visit_expr(arg);
                }
            }
            Expr::BinaryOp { left, right, .. } => {
                self.visit_expr(left);
                self.visit_expr(right);
            }
            Expr::UnaryOp { operand, .. } => self.visit_expr(operand),
            Expr::FuncCall { func, args, .. } => {
                self.visit_expr(func);
                for arg in args {
                    self.visit_expr(arg);
                }
            }
            Expr::IfExpr {
                condition,
                then_expr,
                else_expr,
                ..
            }
            | Expr::TernaryExpr {
                condition,
                then_expr,
                else_expr,
                ..
            } => {
                self.visit_expr(condition);
                self.visit_expr(then_expr);
                self.visit_expr(else_expr);
            }
            Expr::ArrayLiteral { elements, .. } => {
                for e in elements {
                    self.visit_expr(e);
                }
            }
            Expr::MapLiteral { entries, .. } => {
                for (k, v) in entries {
                    self.visit_expr(k);
                    self.visit_expr(v);
                }
            }
            _ => {}
        }
    }
}

fn lower_view_node(
    node: &ViewNode,
    string_signals: &[String],
    sig_idx: Option<&Cell<usize>>,
    variant_map: &HashMap<String, String>,
) -> ViewTree {
    match node {
        ViewNode::Element(el) => {
            let mut props = HashMap::new();
            let mut bindings = HashMap::new();
            let mut events = HashMap::new();
            let mut inline_style: Option<serde_json::Value> = None;
            let mut has_value_binding = false;
            let mut class_map_bindings_from_prop: HashMap<String, ExprIR> = HashMap::new();
            let mut motion_triggers: Vec<TriggerIntent> = Vec::new();
            let mut states_map: Option<HashMap<String, StateTweenIR>> = None;
            let mut animate_signal: Option<ExprIR> = None;
            let mut paused_override: Option<ExprIR> = None;
            let mut hover_reverse: Option<String> = None;
            let mut driven_by: Option<ExprIR> = None;
            let mut in_view_once: Option<ExprIR> = None;
            let mut in_view_repeat: Option<ExprIR> = None;
            let mut in_view_threshold: Option<ExprIR> = None;
            let mut scroll_start: Option<String> = None;
            let mut scroll_end: Option<String> = None;
            let mut scroll_scrub: Option<ExprIR> = None;
            let mut drag_constraints: Option<String> = None;
            let mut drag_elastic: Option<ExprIR> = None;
            let mut drag_scrub: Option<ExprIR> = None;
            let mut drag_momentum: Option<ExprIR> = None;
            let mut drag_threshold: Option<ExprIR> = None;
            let mut drag_velocity_max: Option<ExprIR> = None;
            let mut hero_tag: Option<String> = None;
            let mut hero_transition: Option<ExprIR> = None;

            for prop in &el.props {
                match prop {
                    ElementProp::Static { name, value } => {
                        props.insert(name.clone(), lower_expr(value, variant_map));
                    }
                    ElementProp::Bind { name, signal_path } => {
                        if name == "value" {
                            has_value_binding = true;
                        }
                        bindings.insert(
                            name.clone(),
                            SignalRefIR {
                                owner: None,
                                name: signal_path.clone(),
                            },
                        );
                    }
                    ElementProp::Event { name, handler } => {
                        // Collect event data kind
                        let event_data = map_event_data_kind(&name);
                        let mut needs_pd = name == "onSubmit"; // backward compat
                        let mut needs_sp = false;
                        let mut used_fields: Option<Vec<String>> = None;

                        let ir_handler = match handler {
                            EventHandler::InlineFn { params, body, .. } => {
                                if event_data.is_some() {
                                    let collector = EventFieldCollector::collect(body);
                                    used_fields = Some(collector.fields);
                                    needs_pd = needs_pd || collector.needs_prevent_default;
                                    needs_sp = collector.needs_stop_propagation;
                                }
                                EventHandlerIR {
                                    kind: "inline_fn".into(),
                                    body: Some(lower_block(body, variant_map)),
                                    r#ref: None,
                                    params: params.iter().map(|p| p.name.clone()).collect(),
                                    event_data: event_data.clone(),
                                    used_fields: used_fields.clone(),
                                    needs_prevent_default: needs_pd,
                                    needs_stop_propagation: needs_sp,
                                }
                            }
                            EventHandler::FnRef(fn_name) => {
                                // Defer used_fields to Pass 2 (fn_ref backfill)
                                EventHandlerIR {
                                    kind: "fn_ref".into(),
                                    body: None,
                                    r#ref: Some(fn_name.clone()),
                                    params: vec![],
                                    event_data: event_data.clone(),
                                    used_fields: None, // backfilled in Pass 2
                                    needs_prevent_default: needs_pd,
                                    needs_stop_propagation: needs_sp,
                                }
                            }
                            EventHandler::StoreAction { store, action } => {
                                // Store actions receive no event data
                                EventHandlerIR {
                                    kind: "store_action".into(),
                                    body: None,
                                    r#ref: Some(format!("{}.{}", store, action)),
                                    params: vec![],
                                    event_data: None,
                                    used_fields: None,
                                    needs_prevent_default: false,
                                    needs_stop_propagation: false,
                                }
                            }
                        };
                        events.insert(name.clone(), ir_handler);
                    }
                    ElementProp::Class { value } => {
                        // class: styleRef or class: "literal-class"
                        let mut class_map_bindings: HashMap<String, ExprIR> = HashMap::new();
                        let class_str = match value {
                            ClassValue::StyleRef(s) => s.clone(),
                            ClassValue::StringLiteral(s) => s.clone(),
                            ClassValue::ClassMap(entries) => {
                                // Include classes based on their literal value:
                                //   Bool(true)  → static truth  → include the class
                                //   Bool(false) → static falsy → skip the class
                                //   dynamic     → unknown at compile time → JS handles
                                let mut static_classes: Vec<String> = Vec::new();
                                for (key, val_expr) in entries {
                                    let mut skip = false;
                                    if let Expr::Literal(Literal::Bool(false), _) = val_expr {
                                        // Explicitly false: exclude the class entirely
                                        skip = true;
                                    }
                                    // For true or dynamic values, include the class.
                                    // Dynamic toggling will be handled by JS later.
                                    if !skip {
                                        static_classes.push(key.clone());
                                    }
                                    // Collect dynamic bindings (non-literal values) for reactive JS toggling
                                    if !matches!(val_expr, Expr::Literal(Literal::Bool(_), _)) {
                                        class_map_bindings
                                            .insert(key.clone(), lower_expr(val_expr, variant_map));
                                    }
                                }
                                static_classes.join(" ")
                            }
                        };
                        props.insert(
                            "class".into(),
                            ExprIR::LiteralExpr {
                                value: serde_json::Value::String(class_str),
                            },
                        );
                        // Pass class_map_bindings through the element
                        class_map_bindings_from_prop = class_map_bindings;
                    }
                    ElementProp::Style { entries } => {
                        // style: { key: value, ... } — passed as inline_style
                        // Lower each entry to a JSON representation for codegen
                        let style_map: serde_json::Map<String, serde_json::Value> = entries
                            .iter()
                            .map(|(key, expr)| {
                                let ir_expr = lower_expr(expr, variant_map);
                                let json_val = lower_expr_to_json_from_ir(&ir_expr);
                                (key.clone(), json_val)
                            })
                            .collect();
                        inline_style = Some(serde_json::Value::Object(style_map));
                    }
                    ElementProp::TestId(id) => {
                        // test-id="..." attribute
                        props.insert(
                            "data-testid".into(),
                            ExprIR::LiteralExpr {
                                value: serde_json::Value::String(id.clone()),
                            },
                        );
                    }
                    ElementProp::Ref(r) => {
                        // ref="..." attribute
                        props.insert(
                            "data-ref".into(),
                            ExprIR::LiteralExpr {
                                value: serde_json::Value::String(r.clone()),
                            },
                        );
                    }
                    // Motion animation triggers: on:hover={fadeIn}, etc.
                    ElementProp::MotionTrigger(mt) => {
                        let motion_ref = match &mt.motion_expr {
                            Expr::VarRef(name, _) => Some(name.clone()),
                            _ => None,
                        };
                        motion_triggers.push(TriggerIntent {
                            kind: match &mt.trigger {
                                crate::parser::ast::TriggerKind::Enter => TriggerKind::Enter,
                                crate::parser::ast::TriggerKind::Leave => TriggerKind::Leave,
                                crate::parser::ast::TriggerKind::Hover => TriggerKind::Hover,
                                crate::parser::ast::TriggerKind::Unhover => TriggerKind::Unhover,
                                crate::parser::ast::TriggerKind::Press => TriggerKind::Press,
                                crate::parser::ast::TriggerKind::Focus => TriggerKind::Focus,
                                crate::parser::ast::TriggerKind::Animate => TriggerKind::Animate,
                                crate::parser::ast::TriggerKind::InView => TriggerKind::InView,
                                crate::parser::ast::TriggerKind::Scroll => TriggerKind::Scroll,
                                crate::parser::ast::TriggerKind::DragHorizontal => {
                                    TriggerKind::DragHorizontal
                                }
                                crate::parser::ast::TriggerKind::DragVertical => {
                                    TriggerKind::DragVertical
                                }
                                crate::parser::ast::TriggerKind::Timeline => TriggerKind::Timeline,
                            },
                            motion_ref,
                            inline_motion: None,
                        });
                    }
                    // States map: states: { visible: { ... }, hidden: { ... } }
                    ElementProp::StatesMap(states) => {
                        let mut states_hm: HashMap<String, StateTweenIR> = HashMap::new();
                        for (name, state_def) in states {
                            let properties: HashMap<String, ExprIR> = state_def
                                .properties
                                .iter()
                                .map(|(k, v)| (k.clone(), lower_expr(v, variant_map)))
                                .collect();
                            let over = state_def.over.as_ref().map(|e| lower_expr(e, variant_map));
                            let ease = state_def.ease.as_ref().map(|e| lower_easing(e));
                            states_hm.insert(
                                name.clone(),
                                StateTweenIR {
                                    properties,
                                    over,
                                    ease,
                                },
                            );
                        }
                        states_map = Some(states_hm);
                    }
                    // Animate signal: @state={signal}
                    ElementProp::AnimateSignal(expr) => {
                        animate_signal = Some(lower_expr(expr, variant_map));
                    }
                    // Paused override: @paused={expr}
                    ElementProp::PausedOverride(expr) => {
                        paused_override = Some(lower_expr(expr, variant_map));
                    }
                    // Hover reverse animation name
                    ElementProp::HoverReverse(s) => {
                        hover_reverse = Some(s.clone());
                    }
                    // Driven-by expression for external progress control
                    ElementProp::DrivenBy(expr) => {
                        driven_by = Some(lower_expr(expr, variant_map));
                    }
                    // InView trigger: once
                    ElementProp::InViewOnce(expr) => {
                        in_view_once = Some(lower_expr(expr, variant_map));
                    }
                    // InView trigger: repeat
                    ElementProp::InViewRepeat(expr) => {
                        in_view_repeat = Some(lower_expr(expr, variant_map));
                    }
                    // InView trigger: threshold
                    ElementProp::InViewThreshold(expr) => {
                        in_view_threshold = Some(lower_expr(expr, variant_map));
                    }
                    // Parent-child orchestration
                    ElementProp::When(_)
                    | ElementProp::DelayChildren(_)
                    | ElementProp::StaggerChildren(_) => {
                        // Lowered at trigger level; no direct IR field on ElementIR
                    }
                    // Scroll sub-attributes
                    ElementProp::ScrollStart(s) => {
                        scroll_start = Some(s.clone());
                    }
                    ElementProp::ScrollEnd(s) => {
                        scroll_end = Some(s.clone());
                    }
                    ElementProp::ScrollScrub(expr) => {
                        scroll_scrub = Some(lower_expr(expr, variant_map));
                    }
                    // Drag sub-attributes
                    ElementProp::DragConstraints(s) => {
                        drag_constraints = Some(s.clone());
                    }
                    ElementProp::DragElastic(expr) => {
                        drag_elastic = Some(lower_expr(expr, variant_map));
                    }
                    ElementProp::DragScrub(expr) => {
                        drag_scrub = Some(lower_expr(expr, variant_map));
                    }
                    ElementProp::DragMomentum(expr) => {
                        drag_momentum = Some(lower_expr(expr, variant_map));
                    }
                    ElementProp::DragThreshold(expr) => {
                        drag_threshold = Some(lower_expr(expr, variant_map));
                    }
                    ElementProp::DragVelocityMax(expr) => {
                        drag_velocity_max = Some(lower_expr(expr, variant_map));
                    }
                    // Hero attributes
                    ElementProp::HeroTag(s) => {
                        hero_tag = Some(s.clone());
                    }
                    ElementProp::HeroTransition(expr) => {
                        hero_transition = Some(lower_expr(expr, variant_map));
                    }
                }
            }

            // Auto-bind <input> elements to String signals if no explicit bind:value
            if let Some(sig_counter) = sig_idx {
                let idx = sig_counter.get();
                if el.tag == "input" && !has_value_binding && idx < string_signals.len() {
                    let sig_name = string_signals[idx].clone();
                    sig_counter.set(idx + 1);
                    bindings.insert(
                        "value".into(),
                        SignalRefIR {
                            owner: None,
                            name: sig_name,
                        },
                    );
                }
            }

            let children: Vec<ViewTree> = el
                .children
                .iter()
                .map(|c| lower_view_node(c, string_signals, sig_idx, variant_map))
                .collect();

            ViewTree::Element(ElementIR {
                tag: el.tag.clone(),
                props,
                bindings,
                events,
                style_classes: None,
                inline_style,
                children,
                test_id: None,
                ref_name: None,
                construction_kind: "tag-mapping".into(),
                class_map_bindings: class_map_bindings_from_prop,
                source_line: el.span.line,
                source_col: el.span.col,
                motion_triggers,
                states: states_map.map(|s| StatesIR {
                    states: s,
                    signal: animate_signal,
                }),
                paused_override,
                driven_by,
                hover_reverse,
                in_view_once,
                in_view_repeat,
                in_view_threshold,
                scroll_start,
                scroll_end,
                scroll_scrub,
                drag_constraints,
                drag_elastic,
                drag_scrub,
                drag_momentum,
                drag_threshold,
                drag_velocity_max,
                hero_tag,
                hero_transition,
            })
        }
        ViewNode::Text(s) => ViewTree::Text(s.clone()),
        ViewNode::Interpolation(expr) => {
            // Lower the inner expression to ExprIR so codegen can emit proper JS
            ViewTree::Interpolation(Box::new(lower_expr(expr, variant_map)))
        }
        ViewNode::IfDirective(if_dir) => {
            let branches: Vec<ConditionalBranch> = std::iter::once(ConditionalBranch {
                condition: lower_expr(&if_dir.condition, variant_map),
                body: Some(ViewTree::Element(ElementIR {
                    tag: "fragment".into(),
                    props: HashMap::new(),
                    bindings: HashMap::new(),
                    events: HashMap::new(),
                    style_classes: None,
                    inline_style: None,
                    children: if_dir
                        .body
                        .iter()
                        .map(|n| lower_view_node(n, string_signals, None, variant_map))
                        .collect(),
                    class_map_bindings: HashMap::new(),
                    test_id: None,
                    ref_name: None,
                    construction_kind: "conditional".into(),
                    source_line: 0,
                    source_col: 0,
                    motion_triggers: vec![],
                    states: None,
                    paused_override: None,
                    driven_by: None,
                    hover_reverse: None,
                    in_view_once: None,
                    in_view_repeat: None,
                    in_view_threshold: None,
                    scroll_start: None,
                    scroll_end: None,
                    scroll_scrub: None,
                    drag_constraints: None,
                    drag_elastic: None,
                    drag_scrub: None,
                    drag_momentum: None,
                    drag_threshold: None,
                    drag_velocity_max: None,
                    hero_tag: None,
                    hero_transition: None,
                })),
            })
            .chain(if_dir.else_ifs.iter().map(|(cond, body)| {
                ConditionalBranch {
                    condition: lower_expr(cond, variant_map),
                    body: Some(ViewTree::Element(ElementIR {
                        tag: "fragment".into(),
                        props: HashMap::new(),
                        bindings: HashMap::new(),
                        events: HashMap::new(),
                        style_classes: None,
                        inline_style: None,
                        class_map_bindings: HashMap::new(),
                        children: body
                            .iter()
                            .map(|n| lower_view_node(n, string_signals, None, variant_map))
                            .collect(),
                        test_id: None,
                        ref_name: None,
                        construction_kind: "conditional".into(),
                        source_line: 0,
                        source_col: 0,
                        motion_triggers: vec![],
                        states: None,
                        paused_override: None,
                        driven_by: None,
                        hover_reverse: None,
                        in_view_once: None,
                        in_view_repeat: None,
                        in_view_threshold: None,
                        scroll_start: None,
                        scroll_end: None,
                        scroll_scrub: None,
                        drag_constraints: None,
                        drag_elastic: None,
                        drag_scrub: None,
                        drag_momentum: None,
                        drag_threshold: None,
                        drag_velocity_max: None,
                        hero_tag: None,
                        hero_transition: None,
                    })),
                }
            }))
            .collect();

            let fallback = if_dir.else_body.as_ref().map(|body| {
                ViewTree::Element(ElementIR {
                    tag: "fragment".into(),
                    props: HashMap::new(),
                    bindings: HashMap::new(),
                    events: HashMap::new(),
                    style_classes: None,
                    inline_style: None,
                    children: body
                        .iter()
                        .map(|n| lower_view_node(n, string_signals, None, variant_map))
                        .collect(),
                    test_id: None,
                    ref_name: None,
                    class_map_bindings: HashMap::new(),
                    construction_kind: "conditional".into(),
                    source_line: 0,
                    source_col: 0,
                    motion_triggers: vec![],
                    states: None,
                    paused_override: None,
                    driven_by: None,
                    hover_reverse: None,
                    in_view_once: None,
                    in_view_repeat: None,
                    in_view_threshold: None,
                    scroll_start: None,
                    scroll_end: None,
                    scroll_scrub: None,
                    drag_constraints: None,
                    drag_elastic: None,
                    drag_scrub: None,
                    drag_momentum: None,
                    drag_threshold: None,
                    drag_velocity_max: None,
                    hero_tag: None,
                    hero_transition: None,
                })
            });

            ViewTree::Conditional(Box::new(ConditionalIntent {
                branches,
                fallback,
                transition_mode: if_dir.transition_mode.clone(),
            }))
        }
        ViewNode::ForDirective(for_dir) => {
            let lower_body = |nodes: &[ViewNode]| -> Option<ViewTree> {
                if nodes.is_empty() {
                    None
                } else {
                    Some(ViewTree::Element(ElementIR {
                        tag: "fragment".into(),
                        props: HashMap::new(),
                        bindings: HashMap::new(),
                        events: HashMap::new(),
                        style_classes: None,
                        inline_style: None,
                        class_map_bindings: HashMap::new(),
                        children: nodes
                            .iter()
                            .map(|n| lower_view_node(n, string_signals, None, variant_map))
                            .collect(),
                        test_id: None,
                        ref_name: None,
                        construction_kind: "list".into(),
                        source_line: 0,
                        source_col: 0,
                        motion_triggers: vec![],
                        states: None,
                        paused_override: None,
                        driven_by: None,
                        hover_reverse: None,
                        in_view_once: None,
                        in_view_repeat: None,
                        in_view_threshold: None,
                        scroll_start: None,
                        scroll_end: None,
                        scroll_scrub: None,
                        drag_constraints: None,
                        drag_elastic: None,
                        drag_scrub: None,
                        drag_momentum: None,
                        drag_threshold: None,
                        drag_velocity_max: None,
                        hero_tag: None,
                        hero_transition: None,
                    }))
                }
            };

            ViewTree::ListRendering(Box::new(ListRenderingIntent {
                iterable: lower_expr(&for_dir.iterable, variant_map),
                item_binding: BindingIR {
                    name: for_dir.item_name.clone(),
                    r#type: None,
                },
                body: lower_body(&for_dir.body),
                key_expr: for_dir
                    .key_expr
                    .as_ref()
                    .map(|e| lower_expr(e, variant_map)),
                empty_body: for_dir
                    .empty_body
                    .as_ref()
                    .and_then(|nodes| lower_body(nodes.as_slice())),
            }))
        }
        ViewNode::SlotDirective { name, default_content: _ } => ViewTree::Slot(name.clone()),
    }
}

fn lower_stmt(stmt: &Stmt, variant_map: &HashMap<String, String>) -> StatementIR {
    match stmt {
        Stmt::VarDecl { name, init, .. } => StatementIR::LetStmtIR {
            name: name.clone(),
            value: init.as_ref().map(|e| lower_expr(e, variant_map)),
            r#type: None,
        },
        Stmt::Return { value, .. } => StatementIR::ReturnStmtIR {
            value: value.as_ref().map(|e| lower_expr(e, variant_map)),
        },
        Stmt::ExprStmt { expr, .. } => {
            // Bare block in statement position: emit a scoped block
            if let Expr::Block { body, .. } = expr {
                return StatementIR::BlockStmtIR {
                    body: lower_block(body, variant_map),
                };
            }
            if let Some(mutation) = try_lower_mutation(expr, variant_map) {
                StatementIR::MutationStmt {
                    intent: Some(mutation),
                }
            } else {
                StatementIR::ExprStmtIR {
                    expr: lower_expr(expr, variant_map),
                }
            }
        }
        Stmt::If {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => StatementIR::IfStmtIR {
            condition: lower_expr(condition, variant_map),
            then_body: lower_block(then_body, variant_map),
            else_ifs: else_ifs
                .iter()
                .map(|(cond, body)| {
                    (
                        lower_expr(cond, variant_map),
                        lower_block(body, variant_map),
                    )
                })
                .collect(),
            else_body: else_body.as_ref().map(|b| lower_block(b, variant_map)),
        },
        Stmt::Print { newline, arg, .. } => StatementIR::ExprStmtIR {
            expr: ExprIR::CallExpr {
                name: if *newline { "println" } else { "print" }.into(),
                args: vec![lower_expr(arg, variant_map)],
            },
        },
        Stmt::While {
            condition, body, ..
        } => StatementIR::WhileStmtIR {
            condition: lower_expr(condition, variant_map),
            body: lower_block(body, variant_map),
        },
        Stmt::ForIn {
            var_name,
            iterable,
            body,
            ..
        } => StatementIR::ForInStmtIR {
            var_name: var_name.clone(),
            iterable: lower_expr(iterable, variant_map),
            body: lower_block(body, variant_map),
        },
        Stmt::Assign { target, value, .. } => {
            let (name, owner) = match target {
                AssignTarget::Name(n) => (n.clone(), None),
                AssignTarget::Field { object, field } => (field.clone(), Some(object.clone())),
                AssignTarget::Index { object, .. } => (object.clone(), None),
            };
            StatementIR::MutationStmt {
                intent: Some(MutationIntent {
                    target: SignalRefIR { owner, name },
                    operation: "direct_assign".into(),
                    amount: Some(lower_expr(value, variant_map)),
                    item: None,
                }),
            }
        }
        Stmt::Break(_) => StatementIR::BreakStmtIR,
        Stmt::Continue(_) => StatementIR::ContinueStmtIR,
        Stmt::Throw { expr, .. } => StatementIR::ThrowStmtIR {
            expr: lower_expr(expr, variant_map),
        },
        Stmt::TryCatch {
            try_body,
            catch_var,
            catch_body,
            finally_body,
            ..
        } => StatementIR::TryCatchStmtIR {
            try_body: lower_block(try_body, variant_map),
            catch_var: catch_var.clone(),
            catch_body: catch_body.as_ref().map(|b| lower_block(b, variant_map)),
            finally_body: finally_body.as_ref().map(|b| lower_block(b, variant_map)),
        },
        _ => StatementIR::ExprStmtIR {
            expr: ExprIR::LiteralExpr {
                value: serde_json::Value::Null,
            },
        },
    }
}

/// Try to lower an expression statement as a mutation intent.
/// Handles: MethodCall, FuncCall(FieldAccess), and Assignment to signals.
fn try_lower_mutation(
    expr: &Expr,
    variant_map: &HashMap<String, String>,
) -> Option<MutationIntent> {
    // Pattern 1: Expr::MethodCall { object: VarRef("count"), method: "set", args: [value] }
    if let Expr::MethodCall {
        object,
        method,
        args,
        ..
    } = expr
    {
        if let Expr::VarRef(target_name, _) = object.as_ref() {
            return mutation_for_method(target_name, method, args.first(), variant_map);
        }
    }
    // Pattern 2: Expr::FuncCall { func: FieldAccess { VarRef("count"), "set" }, args: [value] }
    if let Expr::FuncCall { func, args, .. } = expr {
        if let Expr::FieldAccess { object, field, .. } = func.as_ref() {
            if let Expr::VarRef(target_name, _) = object.as_ref() {
                return mutation_for_method(target_name, field, args.first(), variant_map);
            }
        }
    }
    // Pattern 3: Assignment to signal: count = value or count += 1
    if let Expr::BinaryOp {
        left, op, right, ..
    } = expr
    {
        if let Expr::VarRef(target_name, _) = left.as_ref() {
            match op {
                BinaryOpKind::Assign => {
                    return Some(MutationIntent {
                        target: SignalRefIR {
                            owner: None,
                            name: target_name.clone(),
                        },
                        operation: "direct_assign".into(),
                        amount: Some(lower_expr(right, variant_map)),
                        item: None,
                    });
                }
                BinaryOpKind::AddAssign => {
                    return Some(MutationIntent {
                        target: SignalRefIR {
                            owner: None,
                            name: target_name.clone(),
                        },
                        operation: "increment".into(),
                        amount: Some(lower_expr(right, variant_map)),
                        item: None,
                    });
                }
                BinaryOpKind::SubAssign => {
                    return Some(MutationIntent {
                        target: SignalRefIR {
                            owner: None,
                            name: target_name.clone(),
                        },
                        operation: "decrement".into(),
                        amount: Some(lower_expr(right, variant_map)),
                        item: None,
                    });
                }
                BinaryOpKind::MulAssign => {
                    return Some(MutationIntent {
                        target: SignalRefIR {
                            owner: None,
                            name: target_name.clone(),
                        },
                        operation: "multiply".into(),
                        amount: Some(lower_expr(right, variant_map)),
                        item: None,
                    });
                }
                BinaryOpKind::DivAssign => {
                    return Some(MutationIntent {
                        target: SignalRefIR {
                            owner: None,
                            name: target_name.clone(),
                        },
                        operation: "divide".into(),
                        amount: Some(lower_expr(right, variant_map)),
                        item: None,
                    });
                }
                BinaryOpKind::ModAssign => {
                    return Some(MutationIntent {
                        target: SignalRefIR {
                            owner: None,
                            name: target_name.clone(),
                        },
                        operation: "modulo".into(),
                        amount: Some(lower_expr(right, variant_map)),
                        item: None,
                    });
                }
                _ => {}
            }
        }
        // Pattern 4: Assignment to store field: MyStore.count = value or MyStore.count += 1
        if let Expr::FieldAccess { object, field, .. } = left.as_ref() {
            if let Expr::VarRef(owner_name, _) = object.as_ref() {
                match op {
                    BinaryOpKind::Assign => {
                        return Some(MutationIntent {
                            target: SignalRefIR {
                                owner: Some(owner_name.clone()),
                                name: field.clone(),
                            },
                            operation: "direct_assign".into(),
                            amount: Some(lower_expr(right, variant_map)),
                            item: None,
                        });
                    }
                    BinaryOpKind::AddAssign => {
                        return Some(MutationIntent {
                            target: SignalRefIR {
                                owner: Some(owner_name.clone()),
                                name: field.clone(),
                            },
                            operation: "increment".into(),
                            amount: Some(lower_expr(right, variant_map)),
                            item: None,
                        });
                    }
                    BinaryOpKind::SubAssign => {
                        return Some(MutationIntent {
                            target: SignalRefIR {
                                owner: Some(owner_name.clone()),
                                name: field.clone(),
                            },
                            operation: "decrement".into(),
                            amount: Some(lower_expr(right, variant_map)),
                            item: None,
                        });
                    }
                    _ => {}
                }
            }
        }
    }
    None
}

fn mutation_for_method(
    target_name: &str,
    method: &str,
    amount: Option<&Expr>,
    variant_map: &HashMap<String, String>,
) -> Option<MutationIntent> {
    let target = SignalRefIR {
        owner: None,
        name: target_name.into(),
    };
    let amount = amount.map(|e| lower_expr(e, variant_map));
    match method {
        "set" | "setValue" => Some(MutationIntent {
            target,
            operation: "direct_assign".into(),
            amount,
            item: None,
        }),
        "inc" | "increment" => Some(MutationIntent {
            target,
            operation: "increment".into(),
            amount,
            item: None,
        }),
        "dec" | "decrement" => Some(MutationIntent {
            target,
            operation: "decrement".into(),
            amount,
            item: None,
        }),
        "push" => Some(MutationIntent {
            target,
            operation: "push".into(),
            amount,
            item: None,
        }),
        "pop" => Some(MutationIntent {
            target,
            operation: "pop".into(),
            amount,
            item: None,
        }),
        _ => None,
    }
}

pub(crate) fn lower_block(block: &Block, variant_map: &HashMap<String, String>) -> BlockIR {
    let mut statements: Vec<StatementIR> = block
        .stmts
        .iter()
        .map(|s| lower_stmt(s, variant_map))
        .collect();

    // Include trailing expression as an expression statement
    if let Some(ref expr) = block.trailing_expr {
        if let Some(mutation) = try_lower_mutation(expr, variant_map) {
            statements.push(StatementIR::MutationStmt {
                intent: Some(mutation),
            });
        } else {
            statements.push(StatementIR::ExprStmtIR {
                expr: lower_expr(expr, variant_map),
            });
        }
    }

    BlockIR {
        statements,
        execution_mode: "client-only".into(),
    }
}

pub(crate) fn lower_expr(expr: &Expr, variant_map: &HashMap<String, String>) -> ExprIR {
    match expr {
        Expr::Literal(lit, _) => {
            let value = match lit {
                Literal::Int(n) => serde_json::Value::Number((*n).into()),
                Literal::Float(f) => serde_json::json!(f),
                Literal::String(s) => serde_json::Value::String(s.clone()),
                Literal::Bool(b) => serde_json::Value::Bool(*b),
                Literal::Null => serde_json::Value::Null,
            };
            ExprIR::LiteralExpr { value }
        }
        Expr::VarRef(name, _) => {
            // Check if this bare identifier is a tagged union variant (no fields)
            if let Some(union_name) = variant_map.get(name) {
                ExprIR::VariantLiteral {
                    union_type: union_name.clone(),
                    variant: name.clone(),
                    fields: vec![],
                }
            } else {
                ExprIR::IdentifierExpr { name: name.clone() }
            }
        }
        Expr::BinaryOp {
            left, op, right, ..
        } => ExprIR::BinaryOpExpr {
            op: match op {
                BinaryOpKind::NullishCoalesce => "??".into(),
                _ => format!("{:?}", op).to_lowercase(),
            },
            left: Box::new(lower_expr(left, variant_map)),
            right: Box::new(lower_expr(right, variant_map)),
        },
        Expr::UnaryOp { op, operand, .. } => ExprIR::UnaryOpExpr {
            op: match op {
                UnaryOpKind::Not => "!".into(),
                UnaryOpKind::Neg => "-".into(),
                UnaryOpKind::Await => "await ".into(),
            },
            operand: Box::new(lower_expr(operand, variant_map)),
        },
        Expr::FuncCall { func, args, .. } => {
            // Pattern: signal.get() → signal.value
            if let Expr::FieldAccess { object, field, .. } = func.as_ref() {
                if let Expr::VarRef(obj_name, _) = object.as_ref() {
                    // Signal method calls
                    if field == "get" {
                        return ExprIR::FieldAccessExpr {
                            object: Box::new(ExprIR::IdentifierExpr {
                                name: obj_name.clone(),
                            }),
                            field: "value".into(),
                        };
                    }
                    if field == "set" || field == "inc" || field == "dec" {
                        // Mutation — handled at statement level by try_lower_mutation
                        return ExprIR::IdentifierExpr {
                            name: obj_name.clone(),
                        };
                    }
                    // Store field access
                    return ExprIR::StoreFieldAccessExpr {
                        store_alias: obj_name.clone(),
                        field_name: field.clone(),
                        call_args: args.iter().map(|e| lower_expr(e, variant_map)).collect(),
                    };
                }
            }
            // Regular function call
            if let Expr::VarRef(name, _) = func.as_ref() {
                ExprIR::CallExpr {
                    name: name.clone(),
                    args: args.iter().map(|e| lower_expr(e, variant_map)).collect(),
                }
            } else {
                ExprIR::LiteralExpr {
                    value: serde_json::Value::Null,
                }
            }
        }
        Expr::FieldAccess { object, field, .. } => ExprIR::FieldAccessExpr {
            object: Box::new(lower_expr(object, variant_map)),
            field: field.clone(),
        },
        Expr::OptionalChain { object, field, .. } => ExprIR::OptionalChainExpr {
            object: Box::new(lower_expr(object, variant_map)),
            field: field.clone(),
        },
        Expr::MethodCall {
            object,
            method,
            args,
            ..
        } => {
            // get() with no args → signal access (.value)
            // get(i) with args → array/map index access
            if method == "get" && args.is_empty() {
                ExprIR::FieldAccessExpr {
                    object: Box::new(lower_expr(object, variant_map)),
                    field: "value".into(),
                }
            } else if method == "set" {
                // signal.set(value) → handled at statement level by try_lower_mutation
                ExprIR::IdentifierExpr {
                    name: method.clone(),
                }
            } else {
                // General method call: to_string, len, get(i), push, pop, contains, insert, etc.
                ExprIR::MethodCallExpr {
                    object: Box::new(lower_expr(object, variant_map)),
                    method: method.clone(),
                    args: args.iter().map(|e| lower_expr(e, variant_map)).collect(),
                }
            }
        }
        Expr::StoreFieldAccess {
            store_alias,
            field_name,
            ..
        } => ExprIR::StoreFieldAccessExpr {
            store_alias: store_alias.clone(),
            field_name: field_name.clone(),
            call_args: vec![],
        },
        Expr::ArrayLiteral { elements, .. } => ExprIR::ArrayExpr {
            elements: elements
                .iter()
                .map(|e| lower_expr(e, variant_map))
                .collect(),
        },
        Expr::MapLiteral { entries, .. } => {
            let map: serde_json::Map<String, serde_json::Value> = entries
                .iter()
                .map(|(k, v)| {
                    let key = match lower_expr(k, variant_map) {
                        ExprIR::LiteralExpr { value } => match value {
                            serde_json::Value::String(s) => s,
                            other => format!("{}", other),
                        },
                        other => {
                            // For non-string literal keys, use the identifiers' name or Debug as fallback
                            match other {
                                ExprIR::IdentifierExpr { name } => name,
                                _ => format!("{:?}", other),
                            }
                        }
                    };
                    (key, lower_expr_to_json(v))
                })
                .collect();
            ExprIR::LiteralExpr {
                value: serde_json::Value::Object(map),
            }
        }
        Expr::StructLiteral {
            type_name, fields, ..
        } => {
            let ir_fields: Vec<(String, ExprIR)> = fields
                .iter()
                .map(|(name, val)| (name.clone(), lower_expr(val, variant_map)))
                .collect();
            // Check if this is a tagged union variant construction
            if let Some(union_name) = variant_map.get(type_name) {
                ExprIR::VariantLiteral {
                    union_type: union_name.clone(),
                    variant: type_name.clone(),
                    fields: ir_fields,
                }
            } else {
                ExprIR::StructExpr {
                    type_name: type_name.clone(),
                    fields: ir_fields,
                }
            }
        }
        Expr::IfExpr {
            condition,
            then_expr,
            else_expr,
            ..
        } => ExprIR::TernaryIR {
            cond: Box::new(lower_expr(condition, variant_map)),
            then_expr: Box::new(lower_expr(then_expr, variant_map)),
            else_expr: Box::new(lower_expr(else_expr, variant_map)),
        },
        Expr::TernaryExpr {
            condition,
            then_expr,
            else_expr,
            ..
        } => ExprIR::TernaryIR {
            cond: Box::new(lower_expr(condition, variant_map)),
            then_expr: Box::new(lower_expr(then_expr, variant_map)),
            else_expr: Box::new(lower_expr(else_expr, variant_map)),
        },
        Expr::Block { body, .. } => {
            // Block in expression position: preserve ALL statements.
            // The last expression statement provides the value; preceding
            // statements (e.g. print, let, mutations) are kept as-is.
            let block_ir = lower_block(body, variant_map);
            if let Some(last) = block_ir.statements.last() {
                match last {
                    StatementIR::ExprStmtIR { expr } => {
                        if block_ir.statements.len() == 1 {
                            // Simple block { expr } — just the expression
                            expr.clone()
                        } else {
                            // Multi-statement block: wrap in BlockExpr
                            ExprIR::BlockExpr {
                                statements: block_ir.statements[..block_ir.statements.len() - 1]
                                    .to_vec(),
                                tail: Box::new(expr.clone()),
                            }
                        }
                    }
                    _ => {
                        // Last statement is not an expression (e.g. return, if)
                        // Preserve all statements; value is null
                        ExprIR::BlockExpr {
                            statements: block_ir.statements,
                            tail: Box::new(ExprIR::LiteralExpr {
                                value: serde_json::Value::Null,
                            }),
                        }
                    }
                }
            } else {
                ExprIR::LiteralExpr {
                    value: serde_json::Value::Null,
                }
            }
        }
        Expr::IndexGet { object, index, .. } => ExprIR::IndexExpr {
            object: Box::new(lower_expr(object, variant_map)),
            index: Box::new(lower_expr(index, variant_map)),
        },
        Expr::ClosureExpr { params, body, .. } => ExprIR::ClosureExpr {
            params: params.clone(),
            body: Box::new(lower_expr(body, variant_map)),
        },
        Expr::AwaitExpr { operand, .. } => ExprIR::AwaitExpr {
            operand: Box::new(lower_expr(operand, variant_map)),
        },
        Expr::MatchExpr { matched, arms, .. } => {
            let ir_arms: Vec<MatchArmIR> = arms
                .iter()
                .map(|arm| {
                    let pattern = match &arm.pattern.kind {
                        MatchPatternKind::Variant { name, bindings } => MatchPatternIR {
                            kind: MatchPatternKindIR::Variant {
                                name: name.clone(),
                                bindings: bindings.clone(),
                            },
                        },
                        MatchPatternKind::Wildcard => MatchPatternIR {
                            kind: MatchPatternKindIR::Wildcard,
                        },
                        MatchPatternKind::Literal(lit) => {
                            let value = match lit {
                                Literal::Int(n) => serde_json::Value::Number((*n).into()),
                                Literal::String(s) => serde_json::Value::String(s.clone()),
                                Literal::Bool(b) => serde_json::Value::Bool(*b),
                                _ => serde_json::Value::Null,
                            };
                            MatchPatternIR {
                                kind: MatchPatternKindIR::Literal { value },
                            }
                        }
                    };
                    MatchArmIR {
                        pattern,
                        body: lower_block(&arm.body, variant_map),
                    }
                })
                .collect();
            ExprIR::MatchExpr {
                matched: Box::new(lower_expr(matched, variant_map)),
                arms: ir_arms,
            }
        }
        _ => ExprIR::LiteralExpr {
            value: serde_json::Value::Null,
        },
    }
}

/// Lower an expression to a serde_json::Value (for literal extraction).
fn lower_expr_to_json(expr: &Expr) -> serde_json::Value {
    match lower_expr(expr, &HashMap::new()) {
        ExprIR::LiteralExpr { value } => value,
        ExprIR::IdentifierExpr { name } => serde_json::Value::String(name),
        other => serde_json::Value::String(format!("{:?}", other)),
    }
}

/// Convert an ExprIR back to a JSON value for inline_style serialization.
fn lower_expr_to_json_from_ir(expr: &ExprIR) -> serde_json::Value {
    match expr {
        ExprIR::LiteralExpr { value } => value.clone(),
        ExprIR::IdentifierExpr { name } => serde_json::json!({"$signal": name}),
        ExprIR::FieldAccessExpr { object, field } => {
            // Resolve ThemeStore.currentBg → flat JS name ThemeStore_currentBg
            if let ExprIR::IdentifierExpr { name: obj_name } = object.as_ref() {
                serde_json::json!({"$signal": format!("{}_{}", obj_name, field)})
            } else {
                let obj = lower_expr_to_json_from_ir(object);
                serde_json::json!({"$signal_concat": [obj, field.as_str()]})
            }
        }
        ExprIR::BinaryOpExpr { op: _, left, right } => {
            serde_json::json!({
                "$concat": [
                    lower_expr_to_json_from_ir(left),
                    lower_expr_to_json_from_ir(right)
                ]
            })
        }
        ExprIR::CallExpr { name, args } => {
            serde_json::json!({
                "$call": {
                    "name": name,
                    "args": args.iter().map(lower_expr_to_json_from_ir).collect::<Vec<_>>()
                }
            })
        }
        ExprIR::StoreFieldAccessExpr {
            store_alias,
            field_name,
            ..
        } => {
            serde_json::json!({"$signal": format!("{}_{}", store_alias, field_name)})
        }
        other => serde_json::Value::String(format!("{:?}", other)),
    }
}

pub(crate) fn lower_type(ty: &TypeAnnotation) -> TypeIR {
    match ty {
        TypeAnnotation::Named {
            name, type_args, ..
        } => {
            let kind = name.clone();
            let args: Vec<TypeIR> = type_args.iter().map(lower_type).collect();
            TypeIR {
                kind,
                type_args: args,
                is_nullable: false,
                needs_clone: false,
                needs_partial_eq: false,
            }
        }
        TypeAnnotation::Optional { base, .. } => {
            let mut inner = lower_type(base);
            inner.is_nullable = true;
            inner
        }
        TypeAnnotation::Union { types, .. } => {
            // Preserve all union variants by joining kind strings
            let kinds: Vec<String> = types
                .iter()
                .map(|t| {
                    let lowered = lower_type(t);
                    lowered.kind
                })
                .collect();
            TypeIR {
                kind: kinds.join(" | "),
                type_args: vec![],
                is_nullable: false,
                needs_clone: false,
                needs_partial_eq: false,
            }
        }
        TypeAnnotation::Fn { .. } => TypeIR {
            kind: "function".into(),
            type_args: vec![],
            is_nullable: false,
            needs_clone: false,
            needs_partial_eq: false,
        },
    }
}

// ═══════════════════════════════════════════════════════
// Motion Lowering: AST -> IR
// ═══════════════════════════════════════════════════════

/// Lower a motion definition (MotionDef) to a MotionIntent.
fn lower_motion_def(
    def: &crate::parser::ast::MotionDef,
    variant_map: &HashMap<String, String>,
) -> MotionIntent {
    let body = match &def.body {
        crate::parser::ast::MotionBody::Keyframe(kf) => {
            MotionBodyIR::Keyframe(lower_keyframe_body(kf, variant_map))
        }
        crate::parser::ast::MotionBody::Timeline(tl) => {
            MotionBodyIR::Timeline(lower_timeline_body(tl, variant_map))
        }
        crate::parser::ast::MotionBody::CallbackOnly(cb) => {
            MotionBodyIR::CallbackOnly(lower_callback_body(cb, variant_map))
        }
    };
    MotionIntent {
        name: def.name.clone(),
        body,
    }
}

/// Lower a timeline definition (TimelineDef) to a TimelineIntent.
fn lower_timeline_def(
    tl: &crate::parser::ast::TimelineDef,
    variant_map: &HashMap<String, String>,
) -> TimelineIntent {
    let items = lower_timeline_items(&tl.items, variant_map);
    TimelineIntent {
        name: tl.name.clone(),
        items,
    }
}

/// Lower a keyframe body (KeyframeBody) to a KeyframeIR.
fn lower_keyframe_body(
    kf: &crate::parser::ast::KeyframeBody,
    variant_map: &HashMap<String, String>,
) -> KeyframeIR {
    let stops: Vec<KeyframeStopIR> = kf
        .stops
        .iter()
        .map(|s| {
            let position = match s.position {
                crate::parser::ast::StopPosition::From => 0.0,
                crate::parser::ast::StopPosition::To => 1.0,
                crate::parser::ast::StopPosition::Percent(p) => p / 100.0,
            };
            let properties: HashMap<String, ExprIR> = s
                .properties
                .iter()
                .map(|(k, v)| (k.clone(), lower_expr(v, variant_map)))
                .collect();
            let ease = s.ease.as_ref().map(|e| lower_easing(e));
            KeyframeStopIR {
                position,
                properties,
                ease,
            }
        })
        .collect();
    let over = lower_expr(&kf.over, variant_map);
    let ease = kf.ease.as_ref().map(|e| lower_easing(e));
    let playback = lower_playback_props(&kf.playback, variant_map);
    let callbacks: Vec<(CallbackKindIR, ExprIR)> = kf
        .callbacks
        .iter()
        .map(|cs| match cs {
            crate::parser::ast::CallbackSpec::OnStart(e) => {
                (CallbackKindIR::OnStart, lower_expr(e, variant_map))
            }
            crate::parser::ast::CallbackSpec::OnUpdate(e) => {
                (CallbackKindIR::OnUpdate, lower_expr(e, variant_map))
            }
            crate::parser::ast::CallbackSpec::OnComplete(e) => {
                (CallbackKindIR::OnComplete, lower_expr(e, variant_map))
            }
            crate::parser::ast::CallbackSpec::OnRepeat(e) => {
                (CallbackKindIR::OnRepeat, lower_expr(e, variant_map))
            }
            crate::parser::ast::CallbackSpec::OnInterrupt(e) => {
                (CallbackKindIR::OnInterrupt, lower_expr(e, variant_map))
            }
        })
        .collect();
    KeyframeIR {
        stops,
        over,
        ease,
        color_space: kf.color_space.clone(),
        playback,
        path: kf.path.clone(),
        path_rotate: kf.path_rotate,
        callbacks,
    }
}

/// Lower a timeline body (TimelineBody) to a TimelineIR.
fn lower_timeline_body(
    tl: &crate::parser::ast::TimelineBody,
    variant_map: &HashMap<String, String>,
) -> TimelineIR {
    let items = lower_timeline_items(&tl.items, variant_map);
    let playback = lower_playback_props(&tl.playback, variant_map);
    TimelineIR { items, playback }
}

/// Lower timeline items (Vec<TimelineItem>) to Vec<TimelineItemIR>.
fn lower_timeline_items(
    items: &[crate::parser::ast::TimelineItem],
    variant_map: &HashMap<String, String>,
) -> Vec<TimelineItemIR> {
    let mut result: Vec<TimelineItemIR> = Vec::new();

    // StartPos modifiers come AFTER the items they modify in the AST
    // (e.g., from { ... } to { ... } over 300 ... start: "withPrev").
    // We need to apply them to the PREVIOUS To-direction item.
    // Strategy: collect pending StartPos values and retroactively apply
    // them to the last Direction item in the result.

    for item in items {
        match &item.kind {
            crate::parser::ast::TimelineItemKind::StartPos(pos) => {
                let formatted = format_position_value(pos);
                // Apply start mode to the most recent From+To Direction pair.
                // AST order: From-Dir, To-Dir, [Label...], StartPos.
                // We scan backward past labels/staggers and update up to 2
                // consecutive Direction items that don't already have a start.
                let mut applied = 0;
                for ir_item in result.iter_mut().rev() {
                    match &mut ir_item.kind {
                        TimelineItemKindIR::Direction { ref mut start, .. } if start.is_none() => {
                            *start = Some(formatted.clone());
                            applied += 1;
                            if applied >= 2 {
                                break; // Updated the pair
                            }
                        }
                        TimelineItemKindIR::Label(..) => {
                            continue; // Skip labels
                        }
                        _ => break, // Stop at non-Direction, non-Label items
                    }
                }
                if applied == 0 {
                    // No preceding Direction — emit as label
                    result.push(TimelineItemIR {
                        kind: TimelineItemKindIR::Label("__start_pos__".into(), None),
                        connector: lower_connector(&item.connector),
                    });
                }
            }
            crate::parser::ast::TimelineItemKind::Direction(di) => {
                result.push(TimelineItemIR {
                    kind: TimelineItemKindIR::Direction {
                        dir: match di.direction {
                            crate::parser::ast::Direction::To => DirectionIR::To,
                            crate::parser::ast::Direction::FromTo => DirectionIR::FromTo,
                        },
                        properties: di
                            .properties
                            .iter()
                            .map(|(k, v)| (k.clone(), lower_expr(v, variant_map)))
                            .collect(),
                        over: lower_expr(&di.over, variant_map),
                        ease: di.ease.as_ref().map(|e| lower_easing(e)),
                        delay: di.delay.as_ref().map(|e| lower_expr(e, variant_map)),
                        color_space: di.color_space.clone(),
                        start: di.start.as_ref().map(|pv| format_position_value(pv)),
                        path: di.path.clone(),
                        path_rotate: di.path_rotate,
                    },
                    connector: lower_connector(&item.connector),
                });
            }
            _ => {
                let kind = lower_timeline_item_kind_simple(&item.kind, variant_map);
                result.push(TimelineItemIR {
                    kind,
                    connector: lower_connector(&item.connector),
                });
            }
        }
    }

    result
}

/// Lower a single timeline item kind (without StartPos handling).
fn lower_timeline_item_kind_simple(
    kind: &crate::parser::ast::TimelineItemKind,
    variant_map: &HashMap<String, String>,
) -> TimelineItemKindIR {
    match kind {
        crate::parser::ast::TimelineItemKind::Direction(di) => TimelineItemKindIR::Direction {
            dir: match di.direction {
                crate::parser::ast::Direction::To => DirectionIR::To,
                crate::parser::ast::Direction::FromTo => DirectionIR::FromTo,
            },
            properties: di
                .properties
                .iter()
                .map(|(k, v)| (k.clone(), lower_expr(v, variant_map)))
                .collect(),
            over: lower_expr(&di.over, variant_map),
            ease: di.ease.as_ref().map(|e| lower_easing(e)),
            delay: di.delay.as_ref().map(|e| lower_expr(e, variant_map)),
            color_space: di.color_space.clone(),
            start: di.start.as_ref().map(format_position_value),
            path: di.path.clone(),
            path_rotate: di.path_rotate,
        },
        crate::parser::ast::TimelineItemKind::SetBlock(props) => TimelineItemKindIR::Set(
            props
                .iter()
                .map(|(k, v)| (k.clone(), lower_expr(v, variant_map)))
                .collect(),
        ),
        crate::parser::ast::TimelineItemKind::MotionRef(name) => {
            TimelineItemKindIR::MotionRef(name.clone())
        }
        crate::parser::ast::TimelineItemKind::Stagger(sc) => TimelineItemKindIR::Stagger(
            Box::new(lower_motion_expr(&sc.motion, variant_map)),
            StaggerTimingIR {
                each: match &sc.timing {
                    crate::parser::ast::StaggerTiming::Each(e) => lower_expr(e, variant_map),
                    crate::parser::ast::StaggerTiming::Amount(a) => lower_expr(a, variant_map),
                },
                from: Some(lower_stagger_direction(&sc.from)),
            },
            lower_stagger_direction(&sc.from),
        ),
        crate::parser::ast::TimelineItemKind::Label(name, cb) => TimelineItemKindIR::Label(
            name.clone(),
            cb.as_ref().map(|e| lower_expr(e, variant_map)),
        ),
        crate::parser::ast::TimelineItemKind::StartPos(_pos) => {
            TimelineItemKindIR::Label("__start_pos__".into(), None)
        }
    }
}

/// Lower a callback body (CallbackBody) to a CallbackBodyIR.
fn lower_callback_body(
    cb: &crate::parser::ast::CallbackBody,
    variant_map: &HashMap<String, String>,
) -> CallbackBodyIR {
    let callbacks: Vec<(CallbackKindIR, ExprIR)> = cb
        .callbacks
        .iter()
        .map(|cs| match cs {
            crate::parser::ast::CallbackSpec::OnStart(e) => {
                (CallbackKindIR::OnStart, lower_expr(e, variant_map))
            }
            crate::parser::ast::CallbackSpec::OnUpdate(e) => {
                (CallbackKindIR::OnUpdate, lower_expr(e, variant_map))
            }
            crate::parser::ast::CallbackSpec::OnComplete(e) => {
                (CallbackKindIR::OnComplete, lower_expr(e, variant_map))
            }
            crate::parser::ast::CallbackSpec::OnRepeat(e) => {
                (CallbackKindIR::OnRepeat, lower_expr(e, variant_map))
            }
            crate::parser::ast::CallbackSpec::OnInterrupt(e) => {
                (CallbackKindIR::OnInterrupt, lower_expr(e, variant_map))
            }
        })
        .collect();
    let over = lower_expr(&cb.over, variant_map);
    let playback = lower_playback_props(&cb.playback, variant_map);
    CallbackBodyIR {
        callbacks,
        over,
        playback,
    }
}

/// Lower an easing expression to an EasingIR.
fn lower_easing(e: &crate::parser::ast::EasingExpr) -> EasingIR {
    match e {
        crate::parser::ast::EasingExpr::Named(n) => EasingIR::Named(n.clone()),
        crate::parser::ast::EasingExpr::CubicBezier { x1, y1, x2, y2 } => EasingIR::CubicBezier {
            x1: *x1,
            y1: *y1,
            x2: *x2,
            y2: *y2,
        },
        crate::parser::ast::EasingExpr::Spring {
            tension,
            friction,
            velocity,
        } => EasingIR::Spring {
            tension: *tension,
            friction: *friction,
            velocity: *velocity,
        },
        crate::parser::ast::EasingExpr::Back {
            intensity,
            direction,
        } => EasingIR::Back {
            intensity: *intensity,
            direction: direction.clone(),
        },
        crate::parser::ast::EasingExpr::Elastic {
            amplitude,
            period,
            direction,
        } => EasingIR::Elastic {
            amplitude: *amplitude,
            period: *period,
            direction: direction.clone(),
        },
        crate::parser::ast::EasingExpr::Steps { count, direction } => EasingIR::Steps {
            count: *count,
            direction: direction.clone(),
        },
    }
}

/// Lower playback property vector to a PlaybackIR.
fn lower_playback_props(
    props: &[crate::parser::ast::PlaybackProp],
    variant_map: &HashMap<String, String>,
) -> PlaybackIR {
    let mut pb = PlaybackIR::default();
    for prop in props {
        match prop {
            crate::parser::ast::PlaybackProp::Loop(lv) => {
                pb.loop_count = Some(match lv {
                    crate::parser::ast::LoopValue::Count(n) => LoopValue::Count(*n),
                    crate::parser::ast::LoopValue::Infinite => LoopValue::Infinite,
                });
            }
            crate::parser::ast::PlaybackProp::Direction(dv) => {
                pb.direction = Some(match dv {
                    crate::parser::ast::DirectionValue::Normal => DirectionValue::Normal,
                    crate::parser::ast::DirectionValue::Reverse => DirectionValue::Reverse,
                    crate::parser::ast::DirectionValue::Alternate => DirectionValue::Alternate,
                    crate::parser::ast::DirectionValue::AlternateReverse => {
                        DirectionValue::AlternateReverse
                    }
                });
            }
            crate::parser::ast::PlaybackProp::Fill(fv) => {
                pb.fill = Some(match fv {
                    crate::parser::ast::FillValue::None => FillValue::None,
                    crate::parser::ast::FillValue::Forwards => FillValue::Forwards,
                    crate::parser::ast::FillValue::Backwards => FillValue::Backwards,
                    crate::parser::ast::FillValue::Both => FillValue::Both,
                });
            }
            crate::parser::ast::PlaybackProp::Delay(e) => {
                pb.delay = Some(lower_expr(e, variant_map));
            }
            crate::parser::ast::PlaybackProp::Paused(b) => {
                pb.paused = *b;
            }
            crate::parser::ast::PlaybackProp::RepeatDelay(e) => {
                pb.repeat_delay = Some(lower_expr(e, variant_map));
            }
            crate::parser::ast::PlaybackProp::RepeatRefresh(b) => {
                pb.repeat_refresh = *b;
            }
            crate::parser::ast::PlaybackProp::TimeScale(e) => {
                pb.time_scale = Some(lower_expr(e, variant_map));
            }
            crate::parser::ast::PlaybackProp::Interrupt(iv) => {
                pb.interrupt = Some(match iv {
                    crate::parser::ast::InterruptValue::Stop => InterruptValue::Cancel,
                    crate::parser::ast::InterruptValue::Complete => InterruptValue::Finish,
                    crate::parser::ast::InterruptValue::Queue => InterruptValue::Compose,
                    crate::parser::ast::InterruptValue::Replace => InterruptValue::Replace,
                });
            }
            crate::parser::ast::PlaybackProp::RespectReducedMotion(b) => {
                pb.respect_reduced_motion = *b;
            }
            crate::parser::ast::PlaybackProp::When(wv) => {
                pb.when = Some(match wv {
                    crate::parser::ast::WhenValue::BeforeChildren => WhenValue::Idle,
                    crate::parser::ast::WhenValue::AfterChildren => WhenValue::Active,
                    crate::parser::ast::WhenValue::WithChildren => WhenValue::IdleOrActive,
                });
            }
            crate::parser::ast::PlaybackProp::DelayChildren(e) => {
                pb.delay_children = Some(lower_expr(e, variant_map));
            }
            crate::parser::ast::PlaybackProp::StaggerChildren(e) => {
                pb.stagger_children = Some(lower_expr(e, variant_map));
            }
            crate::parser::ast::PlaybackProp::StaggerFrom(sd) => {
                pb.stagger_from = Some(lower_stagger_direction(sd));
            }
        }
    }
    pb
}

/// Lower a motion expression (Expr reference) to a MotionExprIR.
fn lower_motion_expr(expr: &Expr, _variant_map: &HashMap<String, String>) -> MotionExprIR {
    match expr {
        Expr::VarRef(name, _) => MotionExprIR::Ref(name.clone()),
        // For non-identifier expressions (literal numbers, null placeholders, etc.),
        // return an empty ref which the codegen handles by skipping motion ref emission.
        _ => MotionExprIR::Ref(String::new()),
    }
}

/// Lower a Connector to ConnectorIR.
fn lower_connector(c: &crate::parser::ast::Connector) -> ConnectorIR {
    match c {
        crate::parser::ast::Connector::None => ConnectorIR::None,
        crate::parser::ast::Connector::Then => ConnectorIR::Then,
        crate::parser::ast::Connector::With => ConnectorIR::With,
    }
}

/// Lower a StaggerDirection (AST) to StaggerDirection (IR).
fn lower_stagger_direction(sd: &crate::parser::ast::StaggerDirection) -> StaggerDirection {
    match sd {
        crate::parser::ast::StaggerDirection::Start => StaggerDirection::First,
        crate::parser::ast::StaggerDirection::Center => StaggerDirection::Center,
        crate::parser::ast::StaggerDirection::End => StaggerDirection::Last,
        crate::parser::ast::StaggerDirection::Edges => StaggerDirection::Edges,
    }
}

/// Format a PositionValue to a string representation.
fn format_position_value(pv: &crate::parser::ast::PositionValue) -> String {
    match pv {
        crate::parser::ast::PositionValue::After => "after".into(),
        crate::parser::ast::PositionValue::WithPrev => "with-prev".into(),
        crate::parser::ast::PositionValue::Overlap(v) => format!("overlap({})", v),
        crate::parser::ast::PositionValue::AfterPrev(v) => format!("after-prev({})", v),
        crate::parser::ast::PositionValue::WithOffset(v) => format!("with-offset({})", v),
        crate::parser::ast::PositionValue::Absolute(v) => format!("absolute({})", v),
    }
}

#[cfg(test)]
mod event_data_tests {
    use super::*;

    #[test]
    fn test_map_event_data_kind_pointer() {
        assert_eq!(map_event_data_kind("onClick"), Some(EventDataKind::PointerEvent));
        assert_eq!(map_event_data_kind("onMouseMove"), Some(EventDataKind::PointerEvent));
        assert_eq!(map_event_data_kind("onPointerDown"), Some(EventDataKind::PointerEvent));
    }

    #[test]
    fn test_map_event_data_kind_keyboard() {
        assert_eq!(map_event_data_kind("onKeyDown"), Some(EventDataKind::KeyEvent));
        assert_eq!(map_event_data_kind("onKeyUp"), Some(EventDataKind::KeyEvent));
    }

    #[test]
    fn test_map_event_data_kind_scroll() {
        assert_eq!(map_event_data_kind("onScroll"), Some(EventDataKind::ScrollEvent));
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
            EventDataKind::PointerEvent, EventDataKind::TouchEvent,
            EventDataKind::WheelEvent, EventDataKind::KeyEvent,
            EventDataKind::ScrollEvent, EventDataKind::InputEvent,
            EventDataKind::ToggleEvent, EventDataKind::SubmitEvent,
            EventDataKind::FocusEvent, EventDataKind::ClipboardEvent,
            EventDataKind::DragEvent, EventDataKind::DropEvent,
            EventDataKind::ContextMenuEvent, EventDataKind::SelectEvent,
            EventDataKind::ResizeEvent, EventDataKind::MediaEvent,
            EventDataKind::MediaLoadEvent, EventDataKind::MediaErrorEvent,
            EventDataKind::AnimationEvent, EventDataKind::TransitionEvent,
        ];
        assert_eq!(kinds.len(), 20);
        for kind in &kinds {
            assert!(!event_data_schema(kind).is_empty(),
                "Schema empty for {:?}", kind);
        }
    }
}
