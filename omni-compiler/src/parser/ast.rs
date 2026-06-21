//! AST node types for the OmniLang parser.
//!
//! All AST nodes defined per spec §3.3.1. The parser produces a Program from token stream.
//! Types follow the Python AST structure with Rust idioms (Box for recursion, enum dispatch).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::lexer::Span;

// ── Base types ────────────────────────────────────────

/// Type annotation — used in variable declarations, function signatures, etc.
#[derive(Debug, Clone)]
pub enum TypeAnnotation {
    Named {
        name: String,
        type_args: Vec<TypeAnnotation>,
        span: Span,
    },
    Optional {
        base: Box<TypeAnnotation>,
        span: Span,
    },
    Union {
        types: Vec<TypeAnnotation>,
        span: Span,
    },
    Fn {
        params: Vec<TypeAnnotation>,
        return_type: Box<TypeAnnotation>,
        span: Span,
    },
}

// ── Expressions (omnilang + omniview) ─────────────────

#[derive(Debug, Clone)]
pub enum Expr {
    Literal(Literal, Span),
    VarRef(String, Span),
    BinaryOp {
        left: Box<Expr>,
        op: BinaryOpKind,
        right: Box<Expr>,
        span: Span,
    },
    UnaryOp {
        op: UnaryOpKind,
        operand: Box<Expr>,
        span: Span,
    },
    FuncCall {
        func: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    MethodCall {
        object: Box<Expr>,
        method: String,
        args: Vec<Expr>,
        span: Span,
    },
    IndexGet {
        object: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    FieldAccess {
        object: Box<Expr>,
        field: String,
        span: Span,
    },
    OptionalChain {
        object: Box<Expr>,
        field: String,
        span: Span,
    },
    ArrayLiteral {
        elements: Vec<Expr>,
        span: Span,
    },
    MapLiteral {
        entries: Vec<(Expr, Expr)>,
        span: Span,
    },
    IfExpr {
        condition: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
        span: Span,
    },
    TernaryExpr {
        condition: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
        span: Span,
    },
    TemplateString {
        parts: Vec<TemplatePart>,
        span: Span,
    },
    StructLiteral {
        type_name: String,
        fields: Vec<(String, Expr)>,
        span: Span,
    },
    StoreFieldAccess {
        store_alias: String,
        field_name: String,
        span: Span,
    },

    /// Block expression: { stmts; trailing_expr }
    Block {
        body: Block,
        span: Span,
    },

    /// Closure / lambda expression: |params| body
    ClosureExpr {
        params: Vec<String>,
        body: Box<Expr>,
        span: Span,
    },

    /// await expression: await <expr>
    AwaitExpr {
        operand: Box<Expr>,
        span: Span,
    },

    /// match expression: match expr { Pattern => body, ... }
    MatchExpr {
        matched: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },

    /// Reserved for P2 features; parser emits this instead of crashing
    NotYetImplemented {
        feature: String,
        span: Span,
    },
}

#[derive(Debug, Clone)]
pub enum Literal {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Null,
}

#[derive(Debug, Clone)]
pub enum BinaryOpKind {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Neq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
    NullishCoalesce,
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    ModAssign,
}

#[derive(Debug, Clone)]
pub enum UnaryOpKind {
    Not,
    Neg,
    Await,
}

#[derive(Debug, Clone)]
pub enum TemplatePart {
    Literal(String),
    Expr(Expr),
}

// ── Statements ────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Stmt {
    VarDecl {
        name: String,
        type_ann: Option<TypeAnnotation>,
        mutable: bool,
        init: Option<Expr>,
        span: Span,
    },
    Assign {
        target: AssignTarget,
        op: BinaryOpKind,
        value: Expr,
        span: Span,
    },
    If {
        condition: Expr,
        then_body: Block,
        else_ifs: Vec<(Expr, Block)>,
        else_body: Option<Block>,
        span: Span,
    },
    While {
        condition: Expr,
        body: Block,
        span: Span,
    },
    ForIn {
        var_name: String,
        iterable: Expr,
        body: Block,
        span: Span,
    },
    Return {
        value: Option<Expr>,
        span: Span,
    },
    Break(Span),
    Continue(Span),
    Print {
        newline: bool,
        arg: Expr,
        span: Span,
    },
    ExprStmt {
        expr: Expr,
        span: Span,
    },
    EventEmit {
        name: String,
        args: Vec<Expr>,
        span: Span,
    },
    Throw {
        expr: Expr,
        span: Span,
    },
    TryCatch {
        try_body: Block,
        catch_var: Option<String>,
        catch_body: Option<Block>,
        finally_body: Option<Block>,
        span: Span,
    },
}

#[derive(Debug, Clone)]
pub enum AssignTarget {
    Name(String),
    Index { object: String, index: Box<Expr> },
    Field { object: String, field: String },
}

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub trailing_expr: Option<Box<Expr>>,
    pub span: Span,
}

// ── Match Expression ──────────────────────────────────

/// A single arm in a match expression: Pattern => body
#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: MatchPattern,
    pub body: Block,
}

/// A pattern in a match arm: Variant { bindings } or Wildcard
#[derive(Debug, Clone)]
pub struct MatchPattern {
    pub kind: MatchPatternKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum MatchPatternKind {
    /// Variant pattern: Ok { value, response }
    Variant { name: String, bindings: Vec<String> },
    /// Wildcard pattern: _
    Wildcard,
    /// Literal pattern: 42, "hello", true (basic support)
    Literal(Literal),
}

// ── Top-Level Declarations ────────────────────────────

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub type_ann: Option<TypeAnnotation>,
}

#[derive(Debug, Clone)]
pub struct FunctionDef {
    pub visibility: Visibility,
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeAnnotation>,
    pub is_async: bool,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub type_ann: TypeAnnotation,
}

#[derive(Debug, Clone)]
pub struct StructDef {
    pub visibility: Visibility,
    pub name: String,
    pub is_nominal: bool,
    pub fields: Vec<StructField>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TaggedUnionDef {
    pub name: String,
    pub variants: Vec<Variant>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Variant {
    pub name: String,
    pub fields: Vec<StructField>,
}

#[derive(Debug, Clone)]
pub struct TypeAlias {
    pub visibility: Visibility,
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub span: Span,
}

// ── OmniView-Specific AST ─────────────────────────────

#[derive(Debug, Clone)]
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
    pub timelines: Vec<TimelineDef>,
    pub view: Option<ViewBlock>,
    pub styles: Vec<StyleRule>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct PropDecl {
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub default: Option<Expr>,
}

#[derive(Debug, Clone)]
pub struct SignalDecl {
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub init: Expr,
}

#[derive(Debug, Clone)]
pub struct ComputedDecl {
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub body: Expr,
}

#[derive(Debug, Clone)]
pub struct EventDecl {
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub is_optional: bool,
}

#[derive(Debug, Clone)]
pub struct RefDecl {
    pub name: String,
    pub type_ann: TypeAnnotation,
}

#[derive(Debug, Clone)]
pub struct LifecycleHook {
    pub kind: LifecycleKind,
    pub body: Block,
    pub deps: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum LifecycleKind {
    Mount,
    Unmount,
    Effect,
}

#[derive(Debug, Clone)]
pub struct ActionDef {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeAnnotation>,
    pub body: Block,
    pub is_async: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ViewBlock {
    pub children: Vec<ViewNode>,
}

#[derive(Debug, Clone)]
pub enum ViewNode {
    Element(Element),
    IfDirective(IfDirective),
    ForDirective(ForDirective),
    SlotDirective(Option<String>),
    Text(String),
    Interpolation(Expr),
}

#[derive(Debug, Clone)]
pub struct Element {
    pub tag: String,
    pub props: Vec<ElementProp>,
    pub children: Vec<ViewNode>,
    pub self_closing: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
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
    Style {
        entries: Vec<(String, Expr)>,
    },
    TestId(String),
    Ref(String),
    MotionTrigger(MotionTrigger),
    StatesMap(Vec<(String, StateDef)>),
    AnimateSignal(Expr),
    PausedOverride(bool),
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
}

#[derive(Debug, Clone)]
pub enum EventHandler {
    InlineFn { params: Vec<Param>, body: Block },
    FnRef(String),
    StoreAction { store: String, action: String },
}

#[derive(Debug, Clone)]
pub enum ClassValue {
    StyleRef(String),
    StringLiteral(String),
    ClassMap(Vec<(String, Expr)>),
}

#[derive(Debug, Clone)]
pub struct IfDirective {
    pub condition: Expr,
    pub body: Vec<ViewNode>,
    pub else_ifs: Vec<(Expr, Vec<ViewNode>)>,
    pub else_body: Option<Vec<ViewNode>>,
    pub transition_mode: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ForDirective {
    pub item_name: String,
    pub iterable: Expr,
    pub body: Vec<ViewNode>,
    pub empty_body: Option<Vec<ViewNode>>,
    pub key_expr: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct StyleRule {
    pub class_name: String,
    pub properties: Vec<(String, StyleValue)>,
}

#[derive(Debug, Clone)]
pub enum StyleValue {
    Dimension(f64, String),
    HexColor(String),
    String(String),
    Number(f64),
    Ident(String),
    List(Vec<StyleValuePart>),
}

#[derive(Debug, Clone)]
pub enum StyleValuePart {
    Dimension(f64, String),
    Ident(String),
    Number(f64),
    HexColor(String),
}

// ── Motion / Animation AST ────────────────────────────

/// Top-level or component-scoped motion definition
#[derive(Debug, Clone)]
pub struct MotionDef {
    pub name: String,
    pub body: MotionBody,
    pub span: Span,
}

/// Cross-element timeline orchestration
#[derive(Debug, Clone)]
pub struct TimelineDef {
    pub name: String,
    pub items: Vec<TimelineItem>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum MotionBody {
    Keyframe(KeyframeBody),
    Timeline(TimelineBody),
    CallbackOnly(CallbackBody),
}

/// Keyframe mode: percentage stops with shared over/ease
#[derive(Debug, Clone)]
pub struct KeyframeBody {
    pub stops: Vec<KeyframeStop>,
    pub over: Expr,
    pub ease: Option<EasingExpr>,
    pub color_space: Option<String>,
    pub playback: Vec<PlaybackProp>,
}

#[derive(Debug, Clone)]
pub struct KeyframeStop {
    pub position: StopPosition,
    pub properties: Vec<(String, Expr)>,
    pub ease: Option<EasingExpr>,
}

#[derive(Debug, Clone)]
pub enum StopPosition {
    From,
    To,
    Percent(f64),
}

/// Timeline mode: sequential/parallel items with independent timing
#[derive(Debug, Clone)]
pub struct TimelineBody {
    pub items: Vec<TimelineItem>,
    pub playback: Vec<PlaybackProp>,
}

#[derive(Debug, Clone)]
pub struct TimelineItem {
    pub kind: TimelineItemKind,
    pub connector: Connector,
}

#[derive(Debug, Clone)]
pub enum TimelineItemKind {
    Direction(DirectionItem),
    SetBlock(Vec<(String, Expr)>),
    MotionRef(String),
    Stagger(StaggerCall),
    Label(String, Option<Expr>),
}

#[derive(Debug, Clone)]
pub struct DirectionItem {
    pub direction: Direction,
    pub properties: Vec<(String, Expr)>,
    pub over: Expr,
    pub ease: Option<EasingExpr>,
    pub delay: Option<Expr>,
    pub color_space: Option<String>,
    pub start: Option<PositionValue>,
}

#[derive(Debug, Clone)]
pub enum Direction {
    To,
    FromTo,
}

#[derive(Debug, Clone)]
pub enum Connector {
    None,
    Then,
    With,
}

#[derive(Debug, Clone)]
pub enum PositionValue {
    After,
    WithPrev,
    Overlap(String),
    AfterPrev(String),
    WithOffset(String),
    Absolute(String),
}

/// Callback-only mode
#[derive(Debug, Clone)]
pub struct CallbackBody {
    pub callbacks: Vec<CallbackSpec>,
    pub over: Expr,
    pub playback: Vec<PlaybackProp>,
}

#[derive(Debug, Clone)]
pub enum CallbackSpec {
    OnStart(Expr),
    OnUpdate(Expr),
    OnComplete(Expr),
    OnRepeat(Expr),
    OnInterrupt(Expr),
}

#[derive(Debug, Clone)]
pub enum PlaybackProp {
    Loop(LoopValue),
    Direction(DirectionValue),
    Fill(FillValue),
    Delay(Expr),
    Paused(bool),
    RepeatDelay(Expr),
    RepeatRefresh(bool),
    TimeScale(Expr),
    Interrupt(InterruptValue),
    RespectReducedMotion(bool),
    When(WhenValue),
    DelayChildren(Expr),
    StaggerChildren(Expr),
    StaggerFrom(StaggerDirection),
}

#[derive(Debug, Clone)]
pub enum LoopValue {
    Count(u32),
    Infinite,
}

#[derive(Debug, Clone)]
pub enum DirectionValue {
    Normal,
    Reverse,
    Alternate,
    AlternateReverse,
}

#[derive(Debug, Clone)]
pub enum FillValue {
    None,
    Forwards,
    Backwards,
    Both,
}

#[derive(Debug, Clone)]
pub enum InterruptValue {
    Stop,
    Complete,
    Queue,
    Replace,
}

#[derive(Debug, Clone)]
pub enum WhenValue {
    BeforeChildren,
    AfterChildren,
    WithChildren,
}

/// Easing expression
#[derive(Debug, Clone)]
pub enum EasingExpr {
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

/// Stagger call
#[derive(Debug, Clone)]
pub struct StaggerCall {
    pub motion: Box<Expr>,
    pub timing: StaggerTiming,
    pub from: StaggerDirection,
}

#[derive(Debug, Clone)]
pub enum StaggerTiming {
    Each(Expr),
    Amount(Expr),
}

#[derive(Debug, Clone)]
pub enum StaggerDirection {
    Start,
    Center,
    End,
    Edges,
}

/// Animation trigger specification (used in @prop attributes)
#[derive(Debug, Clone)]
pub struct MotionTrigger {
    pub trigger: TriggerKind,
    pub motion_expr: Expr,
}

#[derive(Debug, Clone)]
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

/// State definition for state-based animation
#[derive(Debug, Clone)]
pub struct StateDef {
    pub name: String,
    pub properties: Vec<(String, Expr)>,
    pub over: Option<Expr>,
    pub ease: Option<EasingExpr>,
}

// ── Store & Theme ─────────────────────────────────────

#[derive(Debug, Clone)]
pub struct StoreDef {
    pub name: String,
    pub signals: Vec<SignalDecl>,
    pub getters: Vec<GetterDecl>,
    pub actions: Vec<ActionDef>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct GetterDecl {
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub body: Expr,
}

#[derive(Debug, Clone)]
pub struct ThemeDef {
    pub categories: Vec<ThemeCategory>,
}

#[derive(Debug, Clone)]
pub struct ThemeCategory {
    pub name: String,
    pub values: Vec<(String, String)>,
}

// ── Visibility ────────────────────────────────────────

/// Visibility modifier for declarations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum Visibility {
    Public,
    #[default]
    Private,
}

// ── Import / Export Types ─────────────────────────────

/// Named import: `import { A, B } from "./path.omni";`
#[derive(Debug, Clone)]
pub struct NamedImport {
    pub names: Vec<String>,
    pub path: String,
    pub span: Span,
}

/// Namespace import: `import "./path.omni" as name;`
#[derive(Debug, Clone)]
pub struct NamespaceImport {
    pub path: String,
    pub alias: String,
    pub span: Span,
}

/// Type-only import: `import type { A, B } from "./path.omni";`
#[derive(Debug, Clone)]
pub struct TypeImport {
    pub names: Vec<String>,
    pub path: String,
    pub span: Span,
}

/// Re-export: `export { A, B } from "./path.omni";`
#[derive(Debug, Clone)]
pub struct ExportFrom {
    pub names: Vec<String>,
    pub path: String,
    pub span: Span,
}

/// All import variants in one enum
#[derive(Debug, Clone)]
pub enum ImportStmt {
    Named(NamedImport),
    Namespace(NamespaceImport),
    TypeOnly(TypeImport),
    Lib(String), // "omniview" — bare import marker
}

// ── Module & Program ──────────────────────────────────

/// A single source module — corresponds to one .omni file.
#[derive(Debug, Clone, Default)]
pub struct Module {
    pub name: String,
    pub functions: Vec<FunctionDef>,
    pub structs: Vec<StructDef>,
    pub tagged_unions: Vec<TaggedUnionDef>,
    pub type_aliases: Vec<TypeAlias>,
    pub components: Vec<ComponentDef>,
    pub stores: Vec<StoreDef>,
    pub themes: Vec<ThemeDef>,
    pub imports: Vec<ImportStmt>,
    pub exports: Vec<ExportFrom>,
    pub motions: Vec<MotionDef>,
    pub timelines: Vec<TimelineDef>,
}

#[derive(Debug, Clone, Default)]
pub struct Program {
    pub modules: HashMap<String, Module>,
}
