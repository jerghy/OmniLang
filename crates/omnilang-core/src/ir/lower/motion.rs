//! Lowering for motion/animation definitions.
//!
//! Includes all motion-related lowering functions: motion definitions,
//! keyframes, playback properties, and easing.

use super::expr::lower_expr;
use crate::ir::types::*;
use crate::ir::types_motion::*;
use crate::ir::{DirectionValue, FillValue, LoopValue};
use crate::semantic::SemanticResult;
use rustc_hash::FxHashMap;

// ═══════════════════════════════════════════════════════
// Motion Lowering: AST -> IR
// ═══════════════════════════════════════════════════════

/// Lower a motion definition (MotionDef) to a MotionIntent.
pub(super) fn lower_motion_def(
    def: &crate::parser::ast::MotionDef,
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
) -> MotionIntent {
    let body = match &def.body {
        crate::parser::ast::MotionBody::Keyframe(kf) => {
            lower_keyframe_body(kf, variant_map, semantics)
        }
    };
    MotionIntent {
        name: def.name.clone(),
        body,
        source_span: None,
    }
}

/// Lower a keyframe body (KeyframeBody) to a KeyframeIR.
pub(super) fn lower_keyframe_body(
    kf: &crate::parser::ast::KeyframeBody,
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
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
            let properties: FxHashMap<String, ExprIR> = s
                .properties
                .iter()
                .map(|(k, v)| (k.clone(), lower_expr(v, variant_map, None, semantics)))
                .collect();
            let ease = s.ease.as_ref().map(lower_easing);
            KeyframeStopIR {
                position,
                properties,
                ease,
                source_span: None,
            }
        })
        .collect();
    let over = lower_expr(&kf.over, variant_map, None, semantics);
    let ease = kf.easing.as_ref().map(lower_easing);
    let playback = lower_playback_props(&kf.playback, variant_map, semantics);
    KeyframeIR {
        stops,
        over,
        ease,
        playback,
        source_span: None,
    }
}

/// Lower an easing expression to an EasingIR.
///
/// `steps(count, direction)` lowers to a named "steps(N,dir)" string so it
/// flows through the existing named-easing string path (WAAPI accepts
/// "steps(6,end)" natively). A dedicated EasingIR::Steps variant can be
/// added when the Flutter diagnostic for steps easing is implemented.
pub(super) fn lower_easing(e: &crate::parser::ast::EasingExpr) -> EasingIR {
    match e {
        crate::parser::ast::EasingExpr::Named(n) => EasingIR::Named(n.clone()),
        crate::parser::ast::EasingExpr::CubicBezier { x1, y1, x2, y2 } => EasingIR::CubicBezier {
            x1: *x1,
            y1: *y1,
            x2: *x2,
            y2: *y2,
        },
        crate::parser::ast::EasingExpr::Steps { count, direction } => {
            EasingIR::Named(format!("steps({},{})", count, direction))
        }
        crate::parser::ast::EasingExpr::Back { direction } => EasingIR::Back {
            direction: if direction == "out" {
                None
            } else {
                Some(direction.clone())
            },
        },
        crate::parser::ast::EasingExpr::Elastic { direction } => EasingIR::Elastic {
            direction: if direction == "out" {
                None
            } else {
                Some(direction.clone())
            },
        },
    }
}

/// Lower playback property vector to a PlaybackIR.
pub(super) fn lower_playback_props(
    props: &[crate::parser::ast::PlaybackProp],
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
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
                pb.delay = Some(lower_expr(e, variant_map, None, semantics));
            }
            crate::parser::ast::PlaybackProp::Paused(b) => {
                pb.paused = *b;
            }
        }
    }
    pb
}
