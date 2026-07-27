//! OmniView-specific AST types: ComponentDef, ViewNode, Element, StyleValue, etc.

use serde::Serialize;

use crate::lexer::Span;

use super::motion::{GetterDecl, HeroDef, MotionDef, MotionTrigger, StateDef, TimelineDef};
use super::{Block, Expr, FunctionDef, Param, TypeAnnotation};

// ── OmniView-Specific AST ─────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ComponentDef {
    pub name: String,
    pub props: Vec<PropDecl>,
    pub signals: Vec<SignalDecl>,
    pub computeds: Vec<ComputedDecl>,
    pub events: Vec<EventDecl>,
    pub refs: Vec<RefDecl>,
    pub store_refs: Vec<String>,
    pub if_mode_default: Option<String>,
    pub hero_defs: Vec<HeroDef>,
    pub lifecycles: Vec<LifecycleHook>,
    pub fns: Vec<FunctionDef>,
    pub actions: Vec<ActionDef>,
    pub getters: Vec<GetterDecl>,
    pub motions: Vec<MotionDef>,
    pub timelines: Vec<TimelineDef>,
    pub view: Option<ViewBlock>,
    pub styles: Vec<StyleRule>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct PropDecl {
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub default: Option<Expr>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SignalDecl {
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub init: Expr,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComputedDecl {
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub body: Expr,
}

#[derive(Debug, Clone, Serialize)]
pub struct EventDecl {
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub is_optional: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RefDecl {
    pub name: String,
    pub type_ann: TypeAnnotation,
}

#[derive(Debug, Clone, Serialize)]
pub struct LifecycleHook {
    pub kind: LifecycleKind,
    pub body: Block,
    pub deps: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub enum LifecycleKind {
    Mount,
    Unmount,
    Effect,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActionDef {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeAnnotation>,
    pub body: Block,
    pub is_async: bool,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct ViewBlock {
    pub children: Vec<ViewNode>,
}

#[derive(Debug, Clone, Serialize)]
pub enum ViewNode {
    Element(Element),
    IfDirective(IfDirective),
    ForDirective(ForDirective),
    SlotDirective {
        name: Option<String>,
        default_content: Option<Vec<ViewNode>>,
    },
    Text(String),
    Interpolation(Expr),
}

#[derive(Debug, Clone, Serialize)]
pub struct Element {
    pub tag: String,
    pub props: Vec<ElementProp>,
    pub children: Vec<ViewNode>,
    pub self_closing: bool,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub enum ElementProp {
    Static {
        name: String,
        value: Expr,
    },
    Bind {
        name: String,
        signal_path: String,
    },
    Event {
        name: String,
        handler: EventHandler,
    },
    Class {
        value: ClassValue,
    },
    /// `variant: AppDesign.Button(color=primary, size=md)` in view templates
    Variant {
        design_name: String,
        variant_name: String,
        dimension_values: Vec<(String, String)>,
    },
    Style {
        entries: Vec<(String, Expr)>,
    },
    TestId(String),
    Ref(String),
    MotionTrigger(MotionTrigger),
    StatesMap(Vec<(String, StateDef)>),
    AnimateSignal(Expr),
    PausedOverride(Expr),
    HoverReverse(String),
    /// @driven-by={signal} — external progress control
    DrivenBy(Expr),
    /// @in-view-once={bool}
    InViewOnce(Expr),
    /// @in-view-repeat={bool}
    InViewRepeat(Expr),
    /// @in-view-threshold={number}
    InViewThreshold(Expr),
    /// when: — play order on @enter/@leave
    When(String),
    /// delayChildren: — extra delay before children
    DelayChildren(Expr),
    /// staggerChildren: — auto-stagger shorthand
    StaggerChildren(Expr),
    /// @scroll-start="top 80%"
    ScrollStart(String),
    /// @scroll-end="bottom 20%"
    ScrollEnd(String),
    /// @scroll-scrub={0.5}
    ScrollScrub(Expr),
    /// @drag-constraints="left:0 right:300"
    DragConstraints(String),
    /// @drag-elastic={0.2}
    DragElastic(Expr),
    /// @drag-scrub={true}
    DragScrub(Expr),
    /// @drag-momentum={friction:0.95}
    DragMomentum(Expr),
    /// @drag-threshold={30%}
    DragThreshold(Expr),
    /// @drag-velocity-max={2000px}
    DragVelocityMax(Expr),
    /// @hero="tag-name"
    HeroTag(String),
    /// @hero-transition={motionName}
    HeroTransition(Expr),
}

#[derive(Debug, Clone, Serialize)]
pub enum EventHandler {
    InlineFn { params: Vec<Param>, body: Block },
    FnRef(String),
    StoreAction { store: String, action: String },
}

#[derive(Debug, Clone, Serialize)]
pub enum ClassValue {
    StyleRef(String),
    StringLiteral(String),
    ClassMap(Vec<(String, Expr)>),
    /// `class: AppDesign.Card` — reference to a design block class
    DesignClass {
        design_name: String,
        class_name: String,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct IfDirective {
    pub condition: Expr,
    pub body: Vec<ViewNode>,
    pub else_ifs: Vec<(Expr, Vec<ViewNode>)>,
    pub else_body: Option<Vec<ViewNode>>,
    pub transition_mode: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForDirective {
    pub item_name: String,
    pub iterable: Expr,
    pub body: Vec<ViewNode>,
    pub empty_body: Option<Vec<ViewNode>>,
    pub key_expr: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct StyleRule {
    pub class_name: String,
    pub properties: Vec<(String, StyleValue)>,
    pub is_tag_selector: bool,
}

#[derive(Debug, Clone, Serialize)]
pub enum StyleValue {
    Dimension(f64, String),
    HexColor(String),
    String(String),
    Number(f64),
    Ident(String),
    List(Vec<StyleValuePart>),
}

#[derive(Debug, Clone, Serialize)]
pub enum StyleValuePart {
    Dimension(f64, String),
    Ident(String),
    Number(f64),
    HexColor(String),
}
