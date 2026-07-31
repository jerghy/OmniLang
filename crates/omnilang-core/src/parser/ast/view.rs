//! OmniView-specific AST types: ComponentDef, ViewNode, Element, StyleValue, etc.

use serde::Serialize;

use crate::lexer::Span;

use super::motion::{GetterDecl, MotionDef, MotionTrigger};
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
    pub lifecycles: Vec<LifecycleHook>,
    pub fns: Vec<FunctionDef>,
    pub actions: Vec<ActionDef>,
    pub getters: Vec<GetterDecl>,
    pub motions: Vec<MotionDef>,
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
