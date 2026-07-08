//! Lowering for motion/animation definitions.
//!
//! Includes all motion-related lowering functions: motion definitions,
//! timeline definitions, keyframes, callbacks, playback properties,
//! easing, stagger, connectors, and position values.

use crate::ir::types::*;
use crate::ir::types_motion::*;
use crate::ir::{
    DirectionValue, FillValue, InterruptValue, LoopValue, StaggerDirection, WhenValue,
};
use crate::parser::ast::*;
use crate::semantic::SemanticResult;
use rustc_hash::FxHashMap;
use super::expr::lower_expr;

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
            MotionBodyIR::Keyframe(lower_keyframe_body(kf, variant_map, semantics))
        }
        crate::parser::ast::MotionBody::Timeline(tl) => {
            MotionBodyIR::Timeline(lower_timeline_body(tl, variant_map, semantics))
        }
        crate::parser::ast::MotionBody::CallbackOnly(cb) => {
            MotionBodyIR::CallbackOnly(lower_callback_body(cb, variant_map, semantics))
        }
    };
    MotionIntent {
        name: def.name.clone(),
        body,
    }
}

/// Lower a timeline definition (TimelineDef) to a TimelineIntent.
pub(super) fn lower_timeline_def(
    tl: &crate::parser::ast::TimelineDef,
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
) -> TimelineIntent {
    let items = lower_timeline_items(&tl.items, variant_map, semantics);
    TimelineIntent {
        name: tl.name.clone(),
        items,
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
            }
        })
        .collect();
    let over = lower_expr(&kf.over, variant_map, None, semantics);
    let ease = kf.ease.as_ref().map(lower_easing);
    let playback = lower_playback_props(&kf.playback, variant_map, semantics);
    let callbacks: Vec<(CallbackKindIR, ExprIR)> = kf
        .callbacks
        .iter()
        .map(|cs| match cs {
            crate::parser::ast::CallbackSpec::OnStart(e) => (
                CallbackKindIR::OnStart,
                lower_expr(e, variant_map, None, semantics),
            ),
            crate::parser::ast::CallbackSpec::OnUpdate(e) => (
                CallbackKindIR::OnUpdate,
                lower_expr(e, variant_map, None, semantics),
            ),
            crate::parser::ast::CallbackSpec::OnComplete(e) => (
                CallbackKindIR::OnComplete,
                lower_expr(e, variant_map, None, semantics),
            ),
            crate::parser::ast::CallbackSpec::OnRepeat(e) => (
                CallbackKindIR::OnRepeat,
                lower_expr(e, variant_map, None, semantics),
            ),
            crate::parser::ast::CallbackSpec::OnInterrupt(e) => (
                CallbackKindIR::OnInterrupt,
                lower_expr(e, variant_map, None, semantics),
            ),
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
pub(super) fn lower_timeline_body(
    tl: &crate::parser::ast::TimelineBody,
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
) -> TimelineIR {
    let items = lower_timeline_items(&tl.items, variant_map, semantics);
    let playback = lower_playback_props(&tl.playback, variant_map, semantics);
    TimelineIR { items, playback }
}

/// Lower timeline items (Vec<TimelineItem>) to Vec<TimelineItemIR>.
pub(super) fn lower_timeline_items(
    items: &[crate::parser::ast::TimelineItem],
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
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
                            .map(|(k, v)| (k.clone(), lower_expr(v, variant_map, None, semantics)))
                            .collect(),
                        over: lower_expr(&di.over, variant_map, None, semantics),
                        ease: di.ease.as_ref().map(lower_easing),
                        delay: di
                            .delay
                            .as_ref()
                            .map(|e| lower_expr(e, variant_map, None, semantics)),
                        color_space: di.color_space.clone(),
                        start: di.start.as_ref().map(format_position_value),
                        path: di.path.clone(),
                        path_rotate: di.path_rotate,
                    },
                    connector: lower_connector(&item.connector),
                });
            }
            _ => {
                let kind = lower_timeline_item_kind_simple(&item.kind, variant_map, semantics);
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
pub(super) fn lower_timeline_item_kind_simple(
    kind: &crate::parser::ast::TimelineItemKind,
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
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
                .map(|(k, v)| (k.clone(), lower_expr(v, variant_map, None, semantics)))
                .collect(),
            over: lower_expr(&di.over, variant_map, None, semantics),
            ease: di.ease.as_ref().map(lower_easing),
            delay: di
                .delay
                .as_ref()
                .map(|e| lower_expr(e, variant_map, None, semantics)),
            color_space: di.color_space.clone(),
            start: di.start.as_ref().map(format_position_value),
            path: di.path.clone(),
            path_rotate: di.path_rotate,
        },
        crate::parser::ast::TimelineItemKind::SetBlock(props) => TimelineItemKindIR::Set(
            props
                .iter()
                .map(|(k, v)| (k.clone(), lower_expr(v, variant_map, None, semantics)))
                .collect(),
        ),
        crate::parser::ast::TimelineItemKind::MotionRef(name) => {
            TimelineItemKindIR::MotionRef(name.clone())
        }
        crate::parser::ast::TimelineItemKind::Stagger(sc) => TimelineItemKindIR::Stagger(
            Box::new(lower_motion_expr(&sc.motion, variant_map)),
            StaggerTimingIR {
                each: match &sc.timing {
                    crate::parser::ast::StaggerTiming::Each(e) => {
                        lower_expr(e, variant_map, None, semantics)
                    }
                    crate::parser::ast::StaggerTiming::Amount(a) => {
                        lower_expr(a, variant_map, None, semantics)
                    }
                },
                from: Some(lower_stagger_direction(&sc.from)),
            },
            lower_stagger_direction(&sc.from),
        ),
        crate::parser::ast::TimelineItemKind::Label(name, cb) => TimelineItemKindIR::Label(
            name.clone(),
            cb.as_ref()
                .map(|e| lower_expr(e, variant_map, None, semantics)),
        ),
        crate::parser::ast::TimelineItemKind::StartPos(_pos) => {
            TimelineItemKindIR::Label("__start_pos__".into(), None)
        }
    }
}

/// Lower a callback body (CallbackBody) to a CallbackBodyIR.
pub(super) fn lower_callback_body(
    cb: &crate::parser::ast::CallbackBody,
    variant_map: &FxHashMap<String, String>,
    semantics: &mut SemanticResult,
) -> CallbackBodyIR {
    let callbacks: Vec<(CallbackKindIR, ExprIR)> = cb
        .callbacks
        .iter()
        .map(|cs| match cs {
            crate::parser::ast::CallbackSpec::OnStart(e) => (
                CallbackKindIR::OnStart,
                lower_expr(e, variant_map, None, semantics),
            ),
            crate::parser::ast::CallbackSpec::OnUpdate(e) => (
                CallbackKindIR::OnUpdate,
                lower_expr(e, variant_map, None, semantics),
            ),
            crate::parser::ast::CallbackSpec::OnComplete(e) => (
                CallbackKindIR::OnComplete,
                lower_expr(e, variant_map, None, semantics),
            ),
            crate::parser::ast::CallbackSpec::OnRepeat(e) => (
                CallbackKindIR::OnRepeat,
                lower_expr(e, variant_map, None, semantics),
            ),
            crate::parser::ast::CallbackSpec::OnInterrupt(e) => (
                CallbackKindIR::OnInterrupt,
                lower_expr(e, variant_map, None, semantics),
            ),
        })
        .collect();
    let over = lower_expr(&cb.over, variant_map, None, semantics);
    let playback = lower_playback_props(&cb.playback, variant_map, semantics);
    CallbackBodyIR {
        callbacks,
        over,
        playback,
    }
}

/// Lower an easing expression to an EasingIR.
pub(super) fn lower_easing(e: &crate::parser::ast::EasingExpr) -> EasingIR {
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
            crate::parser::ast::PlaybackProp::RepeatDelay(e) => {
                pb.repeat_delay = Some(lower_expr(e, variant_map, None, semantics));
            }
            crate::parser::ast::PlaybackProp::RepeatRefresh(b) => {
                pb.repeat_refresh = *b;
            }
            crate::parser::ast::PlaybackProp::TimeScale(e) => {
                pb.time_scale = Some(lower_expr(e, variant_map, None, semantics));
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
                pb.delay_children = Some(lower_expr(e, variant_map, None, semantics));
            }
            crate::parser::ast::PlaybackProp::StaggerChildren(e) => {
                pb.stagger_children = Some(lower_expr(e, variant_map, None, semantics));
            }
            crate::parser::ast::PlaybackProp::StaggerFrom(sd) => {
                pb.stagger_from = Some(lower_stagger_direction(sd));
            }
        }
    }
    pb
}

/// Lower a motion expression (Expr reference) to a MotionExprIR.
pub(super) fn lower_motion_expr(
    expr: &Expr,
    _variant_map: &FxHashMap<String, String>,
) -> MotionExprIR {
    match expr {
        Expr::VarRef(name, _) => MotionExprIR::Ref(name.clone()),
        // For non-identifier expressions (literal numbers, null placeholders, etc.),
        // return an empty ref which the codegen handles by skipping motion ref emission.
        _ => MotionExprIR::Ref(String::new()),
    }
}

/// Lower a Connector to ConnectorIR.
pub(super) fn lower_connector(c: &crate::parser::ast::Connector) -> ConnectorIR {
    match c {
        crate::parser::ast::Connector::None => ConnectorIR::None,
        crate::parser::ast::Connector::Then => ConnectorIR::Then,
        crate::parser::ast::Connector::With => ConnectorIR::With,
    }
}

/// Lower a StaggerDirection (AST) to StaggerDirection (IR).
pub(super) fn lower_stagger_direction(
    sd: &crate::parser::ast::StaggerDirection,
) -> StaggerDirection {
    match sd {
        crate::parser::ast::StaggerDirection::Start => StaggerDirection::First,
        crate::parser::ast::StaggerDirection::Center => StaggerDirection::Center,
        crate::parser::ast::StaggerDirection::End => StaggerDirection::Last,
        crate::parser::ast::StaggerDirection::Edges => StaggerDirection::Edges,
    }
}

/// Format a PositionValue to a string representation.
pub(super) fn format_position_value(pv: &crate::parser::ast::PositionValue) -> String {
    match pv {
        crate::parser::ast::PositionValue::After => "after".into(),
        crate::parser::ast::PositionValue::WithPrev => "with-prev".into(),
        crate::parser::ast::PositionValue::Overlap(v) => format!("overlap({})", v),
        crate::parser::ast::PositionValue::AfterPrev(v) => format!("after-prev({})", v),
        crate::parser::ast::PositionValue::WithOffset(v) => format!("with-offset({})", v),
        crate::parser::ast::PositionValue::Absolute(v) => format!("absolute({})", v),
    }
}
