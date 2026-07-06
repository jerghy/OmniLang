//! Motion/Animation AST types plus Store and Theme types.
//!
//! Contains: MotionDef, TimelineDef, EasingExpr, StaggerCall, PlaybackProp, StoreDef, ThemeDef, etc.

use crate::lexer::Span;

use super::view::{ActionDef, SignalDecl};
use super::{Expr, TypeAnnotation};

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
    /// Path for motion path animation: path: "M10 10 ..."
    pub path: Option<String>,
    /// Whether to rotate element along the path
    pub path_rotate: Option<bool>,
    /// Optional callback blocks after keyframe stops
    pub callbacks: Vec<CallbackSpec>,
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
#[allow(clippy::large_enum_variant)]
pub enum TimelineItemKind {
    Direction(DirectionItem),
    SetBlock(Vec<(String, Expr)>),
    MotionRef(String),
    Stagger(StaggerCall),
    Label(String, Option<Expr>),
    /// Timeline item position modifier: start: "withPrev" | "overlap(N)" | etc.
    StartPos(PositionValue),
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
    /// Path for motion path animation: path: "M10 10 ..."
    pub path: Option<String>,
    /// Whether to rotate element along the path
    pub path_rotate: Option<bool>,
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

/// Hero definition for cross-component shared element transitions
#[derive(Debug, Clone)]
pub struct HeroDef {
    pub name: String,
    pub over: Expr,
    pub ease: Option<EasingExpr>,
    pub span: Span,
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
