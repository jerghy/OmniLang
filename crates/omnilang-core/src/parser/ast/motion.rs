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

/// Keyframe mode: percentage stops with shared duration/easing
#[derive(Debug, Clone, Serialize)]
pub struct KeyframeBody {
    pub stops: Vec<KeyframeStop>,
    pub over: Expr,
    pub easing: Option<EasingExpr>,
    pub playback: Vec<PlaybackProp>,
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
    Named(String), // ease-in, ease-out, linear (kebab-case canonical)
    CubicBezier { x1: f64, y1: f64, x2: f64, y2: f64 },
    Steps { count: u32, direction: String }, // steps(6, end)
    Back { direction: String },              // "in" | "out" | "inout"
    Elastic { direction: String },           // "in" | "out" | "inout"
}

/// Animation trigger specification (used in @prop attributes)
#[derive(Debug, Clone, Serialize)]
pub struct MotionTrigger {
    pub trigger: TriggerKind,
    pub motion_expr: Expr,
}

/// Trigger kind. `@scroll` / `@drag-*` carry their v1 control dimensions as
/// colon-form sub-parameters between the trigger keyword and the `={motion}`
/// binding (v2 BNF):
///
/// ```text
/// scroll_params ::= 'start:' expr 'end:' expr 'scrub:' bool
/// drag_params   ::= 'constraints:' '[' expr ',' expr ']' 'elastic:' expr
///                   'threshold:' expr ('velocity_max:' expr)?
/// ```
#[derive(Debug, Clone, Serialize)]
pub enum TriggerKind {
    Enter,
    Hover,
    Press,
    Focus,
    InView,
    Scroll {
        start: Option<Expr>,
        end: Option<Expr>,
        scrub: Option<Expr>,
    },
    DragHorizontal {
        constraints: Option<(Expr, Expr)>,
        elastic: Option<Expr>,
        threshold: Option<Expr>,
        velocity_max: Option<Expr>,
    },
    DragVertical {
        constraints: Option<(Expr, Expr)>,
        elastic: Option<Expr>,
        threshold: Option<Expr>,
        velocity_max: Option<Expr>,
    },
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
