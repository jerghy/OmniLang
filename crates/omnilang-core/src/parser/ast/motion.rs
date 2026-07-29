//! Motion/Animation AST types plus Store and Theme types.
//!
//! Contains: MotionDef, EasingExpr, PlaybackProp, StoreDef.

use serde::Serialize;

use crate::lexer::Span;

use super::view::{ActionDef, SignalDecl};
use super::{Expr, TypeAnnotation};

// ── Motion / Animation AST ────────────────────────────

/// Top-level or component-scoped motion definition
#[derive(Debug, Clone, Serialize)]
pub struct MotionDef {
    pub name: String,
    pub body: MotionBody,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub enum MotionBody {
    Keyframe(KeyframeBody),
}

/// Keyframe mode: percentage stops with shared over/ease
#[derive(Debug, Clone, Serialize)]
pub struct KeyframeBody {
    pub stops: Vec<KeyframeStop>,
    pub over: Expr,
    pub ease: Option<EasingExpr>,
    pub playback: Vec<PlaybackProp>,
    /// Path for motion path animation: path: "M10 10 ..."
    pub path: Option<String>,
    /// Whether to rotate element along the path
    pub path_rotate: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct KeyframeStop {
    pub position: StopPosition,
    pub properties: Vec<(String, Expr)>,
    pub ease: Option<EasingExpr>,
}

#[derive(Debug, Clone, Serialize)]
pub enum StopPosition {
    From,
    To,
    Percent(f64),
}

#[derive(Debug, Clone, Serialize)]
pub enum PlaybackProp {
    Loop(LoopValue),
    Direction(DirectionValue),
    Fill(FillValue),
    Delay(Expr),
    Paused(bool),
}

#[derive(Debug, Clone, Serialize)]
pub enum LoopValue {
    Count(u32),
    Infinite,
}

#[derive(Debug, Clone, Serialize)]
pub enum DirectionValue {
    Normal,
    Reverse,
    Alternate,
    AlternateReverse,
}

#[derive(Debug, Clone, Serialize)]
pub enum FillValue {
    None,
    Forwards,
    Backwards,
    Both,
}

/// Easing expression
#[derive(Debug, Clone, Serialize)]
pub enum EasingExpr {
    Named(String),
    CubicBezier {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
    },
    Back {
        direction: Option<String>,
    },
    Elastic {
        direction: Option<String>,
    },
}

/// Hero definition (legacy — kept only for CompDef compatibility; not parsed)
#[derive(Debug, Clone, Serialize)]
pub struct HeroDef {
    pub name: String,
    pub over: Expr,
    pub ease: Option<EasingExpr>,
    pub span: Span,
}

/// Animation trigger specification (used in @prop attributes)
#[derive(Debug, Clone, Serialize)]
pub struct MotionTrigger {
    pub trigger: TriggerKind,
    pub motion_expr: Expr,
}

#[derive(Debug, Clone, Serialize)]
pub enum TriggerKind {
    Enter,
    Hover,
    Press,
    InView,
    Scroll,
    DragHorizontal,
    DragVertical,
}

// ── Store & Theme ─────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct StoreDef {
    pub name: String,
    pub signals: Vec<SignalDecl>,
    pub getters: Vec<GetterDecl>,
    pub actions: Vec<ActionDef>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize)]
pub struct GetterDecl {
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub body: Expr,
}
