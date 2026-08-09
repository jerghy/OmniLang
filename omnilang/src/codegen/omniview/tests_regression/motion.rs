//! Regression tests for OmniView motion/animation codegen.
//!
//! These are the first regression tests for the OmniView backend. They test motion
//! codegen output at three levels:
//!   Phase 1: Pure functions (no generator state needed)
//!   Phase 2: Read-only conversion (need OmniViewGenerator::default())
//!   Phase 3: State-mutating functions (need &mut OmniViewGenerator)

use super::super::*;
use rustc_hash::FxHashMap;

// ═════════════════════════════════════════════════════════════════════════════
// Phase 1: Pure Function Tests
// ═════════════════════════════════════════════════════════════════════════════

// [Removed] test_format_transform_values — format_transform_value fn no longer exists
// [Removed] test_is_transform_prop — is_transform_prop fn no longer exists

// ═════════════════════════════════════════════════════════════════════════════
// Phase 2: Read-Only Conversion Tests (need OmniViewGenerator::default())
// ═════════════════════════════════════════════════════════════════════════════

/// Tests all 6 EasingIR variants, including optional field defaults.
#[test]
fn test_easing_all_variants() {
    let gen = OmniViewGenerator::default();

    // Named easing → JSON string
    assert_eq!(
        gen.easing_to_js(&EasingIR::Named("ease-out".into())),
        "\"ease-out\""
    );
    assert_eq!(
        gen.easing_to_js(&EasingIR::Named("linear".into())),
        "\"linear\""
    );

    // CubicBezier
    let cb = gen.easing_to_js(&EasingIR::CubicBezier {
        x1: 0.42,
        y1: 0.0,
        x2: 0.58,
        y2: 1.0,
    });
    assert!(
        cb.contains("cubicBezier"),
        "CubicBezier should contain type marker"
    );
    assert!(cb.contains("x1:0.42"));
    assert!(cb.contains("y2:1"));

    // Back with default direction (None → "out")
    let bk = gen.easing_to_js(&EasingIR::Back { direction: None });
    assert!(bk.contains("back"));
    assert!(
        bk.contains("direction:\"out\""),
        "default Back direction should be \"out\""
    );

    // Back with explicit "in" direction
    let bk2 = gen.easing_to_js(&EasingIR::Back {
        direction: Some("in".into()),
    });
    assert!(bk2.contains("direction:\"in\""));

    // Elastic with default direction (None → "out")
    let el = gen.easing_to_js(&EasingIR::Elastic { direction: None });
    assert!(el.contains("elastic"));
    assert!(el.contains("direction:\"out\""));
    // [Removed] Spring, Steps — EasingIR variants no longer exist
}

// [Removed] test_playback_config — playback_to_config fn no longer exists; playback options
// are handled inline in keyframe_to_animate_call

/// Tests keyframe_to_animate_call with both from/to stops.
#[test]
fn test_keyframe_animate_call_basic() {
    let mut gen = OmniViewGenerator::default();
    let kf = KeyframeIR {
        stops: vec![
            KeyframeStopIR {
                position: 0.0,
                properties: FxHashMap::from_iter([(
                    "opacity".into(),
                    ExprIR::LiteralExpr {
                        value: serde_json::json!(0),
                    },
                )]),
                ease: None,

                source_span: None,
            },
            KeyframeStopIR {
                position: 1.0,
                properties: FxHashMap::from_iter([(
                    "opacity".into(),
                    ExprIR::LiteralExpr {
                        value: serde_json::json!(1),
                    },
                )]),
                ease: None,

                source_span: None,
            },
        ],
        over: ExprIR::LiteralExpr {
            value: serde_json::json!(300),
        },
        ease: None,
        playback: PlaybackIR::default(),
        source_span: None,
    };

    let (kf_js, opts_js) = gen.keyframe_to_animate_call(&kf);

    // Keyframe array format
    assert!(kf_js.starts_with('['), "keyframes should be a JS array");
    assert!(kf_js.ends_with(']'), "keyframes should end with ']'");
    assert!(
        kf_js.contains("offset:0"),
        "first stop should have offset:0"
    );
    assert!(
        kf_js.contains("offset:1"),
        "second stop should have offset:1"
    );
    assert!(
        kf_js.contains("\"opacity\""),
        "should contain opacity property"
    );

    // Options contain duration and default fill
    assert!(
        opts_js.contains("duration:300"),
        "options should contain duration:300"
    );
    assert!(
        opts_js.contains("fill:\"both\""),
        "default fill should be \"both\""
    );
}

/// Tests keyframe_to_animate_call edge cases: empty stops, from-only, per-stop easing.
#[test]
fn test_keyframe_animate_call_edge_cases() {
    let mut gen = OmniViewGenerator::default();

    // Edge case 1: No stops → empty keyframe array
    let empty = KeyframeIR {
        stops: vec![],
        over: ExprIR::LiteralExpr {
            value: serde_json::json!(300),
        },
        ease: None,
        playback: PlaybackIR::default(),
        source_span: None,
    };
    let (kf_js, _) = gen.keyframe_to_animate_call(&empty);
    assert_eq!(kf_js, "[]", "empty stops should produce empty JS array");

    // Edge case 2: From-only (no position 1.0 stop) — still produces valid output
    let from_only = KeyframeIR {
        stops: vec![KeyframeStopIR {
            position: 0.0,
            properties: FxHashMap::from_iter([(
                "opacity".into(),
                ExprIR::LiteralExpr {
                    value: serde_json::json!(0),
                },
            )]),
            ease: None,

            source_span: None,
        }],
        over: ExprIR::LiteralExpr {
            value: serde_json::json!(500),
        },
        ease: None,
        playback: PlaybackIR::default(),
        source_span: None,
    };
    let (kf_js_fo, opts_fo) = gen.keyframe_to_animate_call(&from_only);
    assert!(
        kf_js_fo.contains("offset:0"),
        "from-only should still contain offset:0"
    );
    assert!(opts_fo.contains("duration:500"), "duration should be 500ms");

    // Edge case 3: Keyframe with per-stop easing
    let with_ease = KeyframeIR {
        stops: vec![
            KeyframeStopIR {
                position: 0.0,
                properties: FxHashMap::from_iter([(
                    "opacity".into(),
                    ExprIR::LiteralExpr {
                        value: serde_json::json!(0),
                    },
                )]),
                ease: Some(EasingIR::Named("ease-in".into())),

                source_span: None,
            },
            KeyframeStopIR {
                position: 1.0,
                properties: FxHashMap::from_iter([(
                    "opacity".into(),
                    ExprIR::LiteralExpr {
                        value: serde_json::json!(1),
                    },
                )]),
                ease: Some(EasingIR::Named("ease-out".into())),

                source_span: None,
            },
        ],
        over: ExprIR::LiteralExpr {
            value: serde_json::json!(300),
        },
        ease: Some(EasingIR::Named("ease-in-out".into())),
        playback: PlaybackIR::default(),
        source_span: None,
    };
    let (kf_js_we, _) = gen.keyframe_to_animate_call(&with_ease);
    assert!(
        kf_js_we.contains("easing:"),
        "per-stop easing should appear in keyframe array"
    );
    assert!(
        kf_js_we.contains("ease-in"),
        "first stop easing should be preserved"
    );
    assert!(
        kf_js_we.contains("ease-out"),
        "second stop easing should be preserved"
    );
}

// [Removed] test_keyframe_animate_call_with_path — path/path_rotate fields removed from KeyframeIR

/// P0 regression: durations must emit as numeric milliseconds, never strings —
/// Chrome's `el.animate()` throws a TypeError on `duration: "500ms"` and the
/// whole app fails at bootstrap. Covers unit strings ("300ms", "1.5s"),
/// bare numbers (>= 1.0 are ms, < 1.0 are seconds per CSS convention), and
/// delay (WAAPI delay is also a number).
#[test]
fn test_duration_and_delay_emit_numeric_ms() {
    let mut gen = OmniViewGenerator::default();

    for (over, expected) in [
        (serde_json::json!("300ms"), "300"),
        (serde_json::json!("1.5s"), "1500"),
        (serde_json::json!("0.5s"), "500"),
        (serde_json::json!("0.5"), "500"),
        (serde_json::json!(400), "400"),
        (serde_json::json!(1), "1"),
        (serde_json::json!(0.5), "500"),
    ] {
        let kf = KeyframeIR {
            stops: vec![],
            over: ExprIR::LiteralExpr {
                value: over.clone(),
            },
            ease: None,
            playback: PlaybackIR::default(),
            source_span: None,
        };
        let (_, opts_js) = gen.keyframe_to_animate_call(&kf);
        assert!(
            opts_js.contains(&format!("duration:{},", expected)),
            "over {:?} should emit duration:{} — got: {}",
            over,
            expected,
            opts_js
        );
        assert!(
            !opts_js.contains("duration:\""),
            "duration must never emit as a string — got: {}",
            opts_js
        );
    }

    // delay: "500ms" string IR (defensive — the parser only accepts plain
    // numbers for delay, but constructed IR may carry a unit string).
    let kf = KeyframeIR {
        stops: vec![],
        over: ExprIR::LiteralExpr {
            value: serde_json::json!(300),
        },
        ease: None,
        playback: PlaybackIR {
            delay: Some(ExprIR::LiteralExpr {
                value: serde_json::json!("500ms"),
            }),
            ..PlaybackIR::default()
        },
        source_span: None,
    };
    let (_, opts_js) = gen.keyframe_to_animate_call(&kf);
    assert!(
        opts_js.contains("delay:500"),
        "delay should emit delay:500 — got: {}",
        opts_js
    );
    assert!(
        !opts_js.contains("delay:\""),
        "delay must never emit as a string — got: {}",
        opts_js
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Phase 3: State-Mutating Function Tests (need &mut OmniViewGenerator)
// ═════════════════════════════════════════════════════════════════════════════

/// Tests that emit_motion_js produces a valid `__omni_motion_{name}` factory
/// function for a simple keyframe animation (fade-in: opacity 0 → 1).
#[test]
fn test_emit_motion_js_keyframe() {
    let mut gen = OmniViewGenerator::default();
    let motion = MotionIntent {
        name: "fadeIn".into(),
        source_file: None,
        body: KeyframeIR {
            stops: vec![
                KeyframeStopIR {
                    position: 0.0,
                    properties: FxHashMap::from_iter([(
                        "opacity".into(),
                        ExprIR::LiteralExpr {
                            value: serde_json::json!(0),
                        },
                    )]),
                    ease: None,

                    source_span: None,
                },
                KeyframeStopIR {
                    position: 1.0,
                    properties: FxHashMap::from_iter([(
                        "opacity".into(),
                        ExprIR::LiteralExpr {
                            value: serde_json::json!(1),
                        },
                    )]),
                    ease: None,

                    source_span: None,
                },
            ],
            over: ExprIR::LiteralExpr {
                value: serde_json::json!(300),
            },
            ease: Some(EasingIR::Named("ease-out".into())),
            playback: PlaybackIR::default(),
            source_span: None,
        },

        source_span: None,
    };
    gen.emit_motion_js(&motion);
    insta::assert_snapshot!("emit_motion_js_keyframe", &gen.js);
}

// [Removed] test_emit_motion_js_callback_only — CallbackBodyIR no longer exists
// [Removed] test_emit_motion_js_timeline — TimelineIR no longer exists

/// Tests emit_motion_triggers_for_element with a named @enter motion reference.
/// Verifies the mount_js output and the enter_trigger_nids registration.
#[test]
fn test_motion_trigger_enter_named() {
    let mut gen = OmniViewGenerator::default();
    let el = ElementIR {
        tag: "div".into(),
        props: FxHashMap::default(),
        bindings: FxHashMap::default(),
        events: FxHashMap::default(),
        style_classes: None,
        inline_style: None,
        children: vec![],
        construction_kind: ConstructionKind::Static,
        class_map_bindings: FxHashMap::default(),
        source_span: None,
        motion: Some(Box::new(MotionBehavior {
            triggers: vec![TriggerIntent {
                kind: TriggerKind::Enter,
                motion_ref: Some("fadeIn".into()),
                inline_motion: None,

                source_span: None,
            }],
            in_view_once: None,
            in_view_repeat: None,
            in_view_threshold: None,
            source_span: None,
        })),
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
    };
    gen.emit_motion_triggers_for_element("el-div-001", &el);

    let output = format!(
        "// === mount_js ===\n{}\n// === js ===\n{}\n// === enter_trigger_nids ===\n{:?}",
        gen.mount_js, gen.js, gen.enter_trigger_nids
    );
    insta::assert_snapshot!("motion_trigger_enter_named", &output);

    // Verify the enter trigger was registered for signal-change replay
    assert_eq!(
        gen.enter_trigger_nids.len(),
        1,
        "should register 1 enter trigger nid"
    );
    assert_eq!(
        gen.enter_trigger_nids[0].0, "el-div-001",
        "nid should match"
    );
    assert_eq!(
        gen.enter_trigger_nids[0].1, "fadeIn",
        "motion name should match"
    );
    assert_eq!(
        gen.enter_trigger_nids[0].2, "enter",
        "trigger kind should be 'enter'"
    );
}

/// Tests emit_motion_triggers_for_element with an @hover named motion reference.
/// @hover auto-binds mouseenter/mouseleave events in the JS output.
#[test]
fn test_motion_trigger_hover_named() {
    let mut gen = OmniViewGenerator::default();
    let el = ElementIR {
        tag: "button".into(),
        props: FxHashMap::default(),
        bindings: FxHashMap::default(),
        events: FxHashMap::default(),
        style_classes: None,
        inline_style: None,
        children: vec![],
        construction_kind: ConstructionKind::Static,
        class_map_bindings: FxHashMap::default(),
        source_span: None,
        motion: Some(Box::new(MotionBehavior {
            triggers: vec![TriggerIntent {
                kind: TriggerKind::Hover,
                motion_ref: Some("scaleUp".into()),
                inline_motion: None,

                source_span: None,
            }],
            in_view_once: None,
            in_view_repeat: None,
            in_view_threshold: None,
            source_span: None,
        })),
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
    };
    gen.emit_motion_triggers_for_element("el-btn-002", &el);

    insta::assert_snapshot!("motion_trigger_hover_named", &gen.js);
}

/// @focus auto-binds focusin/focusout events in the JS output.
#[test]
fn test_motion_trigger_focus_named() {
    let mut gen = OmniViewGenerator::default();
    let el = ElementIR {
        tag: "input".into(),
        props: FxHashMap::default(),
        bindings: FxHashMap::default(),
        events: FxHashMap::default(),
        style_classes: None,
        inline_style: None,
        children: vec![],
        construction_kind: ConstructionKind::Static,
        class_map_bindings: FxHashMap::default(),
        source_span: None,
        motion: Some(Box::new(MotionBehavior {
            triggers: vec![TriggerIntent {
                kind: TriggerKind::Focus,
                motion_ref: Some("pulse".into()),
                inline_motion: None,

                source_span: None,
            }],
            in_view_once: None,
            in_view_repeat: None,
            in_view_threshold: None,
            source_span: None,
        })),
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
    };
    gen.emit_motion_triggers_for_element("el-input-003", &el);

    insta::assert_snapshot!("motion_trigger_focus_named", &gen.js);
}

// ═════════════════════════════════════════════════════════════════════════════
// Phase 3b: Motion Trigger + Behavior Variations
// ═════════════════════════════════════════════════════════════════════════════

/// Tests emit_motion_triggers_for_element with an @scroll trigger.
/// Tests the Scroll trigger path which reads scroll behavior fields (start, end, scrub).
#[test]
fn test_scroll_trigger() {
    let mut gen = OmniViewGenerator::default();
    let el = ElementIR {
        tag: "div".into(),
        props: FxHashMap::default(),
        bindings: FxHashMap::default(),
        events: FxHashMap::default(),
        style_classes: None,
        inline_style: None,
        children: vec![],
        construction_kind: ConstructionKind::Static,
        class_map_bindings: FxHashMap::default(),
        source_span: None,
        motion: Some(Box::new(MotionBehavior {
            triggers: vec![TriggerIntent {
                kind: TriggerKind::Scroll {
                    start: Some(ExprIR::LiteralExpr {
                        value: serde_json::json!(0),
                    }),
                    end: Some(ExprIR::LiteralExpr {
                        value: serde_json::json!(500),
                    }),
                    scrub: Some(ExprIR::LiteralExpr {
                        value: serde_json::json!(true),
                    }),
                },
                motion_ref: Some("fadeIn".into()),
                inline_motion: None,
                source_span: None,
            }],
            in_view_once: None,
            in_view_repeat: None,
            in_view_threshold: None,
            source_span: None,
        })),
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
    };
    gen.emit_motion_triggers_for_element("el-scroll-001", &el);
    let output = format!(
        "// === mount_js ===\n{}\n// === js ===\n{}",
        gen.mount_js, gen.js
    );
    assert!(
        output.contains("scroll"),
        "scroll trigger output should contain 'scroll'"
    );
    insta::assert_snapshot!("scroll_trigger", &output);
}

/// Tests emit_motion_triggers_for_element with an @drag trigger (DragHorizontal).
/// Tests the Drag trigger path which reads drag behavior fields (constraints, elastic, etc.).
#[test]
fn test_drag_trigger() {
    let mut gen = OmniViewGenerator::default();
    let el = ElementIR {
        tag: "div".into(),
        props: FxHashMap::default(),
        bindings: FxHashMap::default(),
        events: FxHashMap::default(),
        style_classes: None,
        inline_style: None,
        children: vec![],
        construction_kind: ConstructionKind::Static,
        class_map_bindings: FxHashMap::default(),
        source_span: None,
        motion: Some(Box::new(MotionBehavior {
            triggers: vec![TriggerIntent {
                kind: TriggerKind::DragHorizontal {
                    constraints: Some((
                        ExprIR::LiteralExpr {
                            value: serde_json::json!(0),
                        },
                        ExprIR::LiteralExpr {
                            value: serde_json::json!(300),
                        },
                    )),
                    elastic: Some(ExprIR::LiteralExpr {
                        value: serde_json::json!(0.1),
                    }),
                    threshold: None,
                },
                motion_ref: Some("fadeIn".into()),
                inline_motion: None,
                source_span: None,
            }],
            in_view_once: None,
            in_view_repeat: None,
            in_view_threshold: None,
            source_span: None,
        })),
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
    };
    gen.emit_motion_triggers_for_element("el-drag-001", &el);
    let output = format!(
        "// === mount_js ===\n{}\n// === js ===\n{}",
        gen.mount_js, gen.js
    );
    assert!(
        output.contains("drag"),
        "drag trigger output should contain 'drag'"
    );
    insta::assert_snapshot!("drag_trigger", &output);
}

/// Tests the no-constraints drag path (deferral #19): maxExtent must be
/// derived from the motion's own keyframe range (translateX 0→300 ⇒ 300)
/// instead of Infinity, so the element follows its animation while dragging.
#[test]
fn test_drag_trigger_no_constraints_derived_extent() {
    let mut gen = OmniViewGenerator::default();
    gen.motion_bodies.insert(
        "slideX".to_string(),
        KeyframeIR {
            stops: vec![
                KeyframeStopIR {
                    position: 0.0,
                    properties: FxHashMap::from_iter([(
                        "translateX".into(),
                        ExprIR::LiteralExpr {
                            value: serde_json::json!(0),
                        },
                    )]),
                    ease: None,
                    source_span: None,
                },
                KeyframeStopIR {
                    position: 1.0,
                    properties: FxHashMap::from_iter([(
                        "translateX".into(),
                        ExprIR::LiteralExpr {
                            value: serde_json::json!(300),
                        },
                    )]),
                    ease: None,
                    source_span: None,
                },
            ],
            over: ExprIR::LiteralExpr {
                value: serde_json::json!(300),
            },
            ease: None,
            playback: PlaybackIR::default(),
            source_span: None,
        },
    );
    let el = ElementIR {
        tag: "div".into(),
        props: FxHashMap::default(),
        bindings: FxHashMap::default(),
        events: FxHashMap::default(),
        style_classes: None,
        inline_style: None,
        children: vec![],
        construction_kind: ConstructionKind::Static,
        class_map_bindings: FxHashMap::default(),
        source_span: None,
        motion: Some(Box::new(MotionBehavior {
            triggers: vec![TriggerIntent {
                kind: TriggerKind::DragHorizontal {
                    constraints: None,
                    elastic: None,
                    threshold: None,
                },
                motion_ref: Some("slideX".into()),
                inline_motion: None,
                source_span: None,
            }],
            in_view_once: None,
            in_view_repeat: None,
            in_view_threshold: None,
            source_span: None,
        })),
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
    };
    gen.emit_motion_triggers_for_element("el-drag-002", &el);
    let output = format!(
        "// === mount_js ===\n{}\n// === js ===\n{}",
        gen.mount_js, gen.js
    );
    assert!(
        output.contains("const maxExtent = 300;"),
        "no-constraints drag should derive maxExtent from keyframes:\n{}",
        output
    );
    assert!(
        !output.contains("Number.POSITIVE_INFINITY"),
        "derived extent must not fall back to the unbounded constant:\n{}",
        output
    );
}

/// Tests the no-constraints drag path when the motion has NO translate
/// keyframes: must fall back to the previous unbounded (inert) behavior and
/// surface a compiler warning — never NaN or a bogus extent.
#[test]
fn test_drag_trigger_no_translate_falls_back_unbounded() {
    let mut gen = OmniViewGenerator::default();
    gen.motion_bodies.insert(
        "fadeOnly".to_string(),
        KeyframeIR {
            stops: vec![
                KeyframeStopIR {
                    position: 0.0,
                    properties: FxHashMap::from_iter([(
                        "opacity".into(),
                        ExprIR::LiteralExpr {
                            value: serde_json::json!(0),
                        },
                    )]),
                    ease: None,
                    source_span: None,
                },
                KeyframeStopIR {
                    position: 1.0,
                    properties: FxHashMap::from_iter([(
                        "opacity".into(),
                        ExprIR::LiteralExpr {
                            value: serde_json::json!(1),
                        },
                    )]),
                    ease: None,
                    source_span: None,
                },
            ],
            over: ExprIR::LiteralExpr {
                value: serde_json::json!(300),
            },
            ease: None,
            playback: PlaybackIR::default(),
            source_span: None,
        },
    );
    let el = ElementIR {
        tag: "div".into(),
        props: FxHashMap::default(),
        bindings: FxHashMap::default(),
        events: FxHashMap::default(),
        style_classes: None,
        inline_style: None,
        children: vec![],
        construction_kind: ConstructionKind::Static,
        class_map_bindings: FxHashMap::default(),
        source_span: None,
        motion: Some(Box::new(MotionBehavior {
            triggers: vec![TriggerIntent {
                kind: TriggerKind::DragVertical {
                    constraints: None,
                    elastic: None,
                    threshold: None,
                },
                motion_ref: Some("fadeOnly".into()),
                inline_motion: None,
                source_span: None,
            }],
            in_view_once: None,
            in_view_repeat: None,
            in_view_threshold: None,
            source_span: None,
        })),
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
    };
    gen.emit_motion_triggers_for_element("el-drag-003", &el);
    let output = format!(
        "// === mount_js ===\n{}\n// === js ===\n{}",
        gen.mount_js, gen.js
    );
    assert!(
        output.contains("const maxExtent = Number.POSITIVE_INFINITY;"),
        "no-translate motion should fall back to the unbounded extent:\n{}",
        output
    );
    assert!(
        output.contains("WARNING"),
        "fallback must carry a compiler warning:\n{}",
        output
    );
    assert!(
        gen.warnings.iter().any(|w| w.contains("el-drag-003")),
        "fallback must push a generator warning, got: {:?}",
        gen.warnings
    );
}

/// Tests the INLINE motion branch of @drag (no motion_ref — the body lives on
/// the trigger itself): the derived extent must flow through the same
/// emit_trigger_drag_js path as the named-ref branch.
#[test]
fn test_drag_trigger_inline_motion_derived_extent() {
    let mut gen = OmniViewGenerator::default();
    let el = ElementIR {
        tag: "div".into(),
        props: FxHashMap::default(),
        bindings: FxHashMap::default(),
        events: FxHashMap::default(),
        style_classes: None,
        inline_style: None,
        children: vec![],
        construction_kind: ConstructionKind::Static,
        class_map_bindings: FxHashMap::default(),
        source_span: None,
        motion: Some(Box::new(MotionBehavior {
            triggers: vec![TriggerIntent {
                kind: TriggerKind::DragHorizontal {
                    constraints: None,
                    elastic: None,
                    threshold: None,
                },
                motion_ref: None,
                inline_motion: Some(KeyframeIR {
                    stops: vec![
                        KeyframeStopIR {
                            position: 0.0,
                            properties: FxHashMap::from_iter([(
                                "translateX".into(),
                                ExprIR::LiteralExpr {
                                    value: serde_json::json!(0),
                                },
                            )]),
                            ease: None,
                            source_span: None,
                        },
                        KeyframeStopIR {
                            position: 1.0,
                            properties: FxHashMap::from_iter([(
                                "translateX".into(),
                                ExprIR::LiteralExpr {
                                    value: serde_json::json!(250),
                                },
                            )]),
                            ease: None,
                            source_span: None,
                        },
                    ],
                    over: ExprIR::LiteralExpr {
                        value: serde_json::json!(300),
                    },
                    ease: None,
                    playback: PlaybackIR::default(),
                    source_span: None,
                }),
                source_span: None,
            }],
            in_view_once: None,
            in_view_repeat: None,
            in_view_threshold: None,
            source_span: None,
        })),

        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
    };
    gen.emit_motion_triggers_for_element("el-drag-004", &el);
    let output = format!(
        "// === mount_js ===\n{}\n// === js ===\n{}",
        gen.mount_js, gen.js
    );
    assert!(
        output.contains("(inline)"),
        "inline drag trigger should be marked inline:\n{}",
        output
    );
    assert!(
        output.contains("const maxExtent = 250;"),
        "inline drag should derive maxExtent from the inline motion keyframes:\n{}",
        output
    );
    assert!(
        gen.warnings.is_empty(),
        "inline derived extent must not warn, got: {:?}",
        gen.warnings
    );
}

/// Tests emit_motion_triggers_for_element with an @in-view trigger and
/// in_view_once / in_view_threshold configurations.
#[test]
fn test_inview_trigger() {
    let mut gen = OmniViewGenerator::default();
    let el = ElementIR {
        tag: "div".into(),
        props: FxHashMap::default(),
        bindings: FxHashMap::default(),
        events: FxHashMap::default(),
        style_classes: None,
        inline_style: None,
        children: vec![],
        construction_kind: ConstructionKind::Static,
        class_map_bindings: FxHashMap::default(),
        source_span: None,
        motion: Some(Box::new(MotionBehavior {
            triggers: vec![TriggerIntent {
                kind: TriggerKind::InView,
                motion_ref: Some("reveal".into()),
                inline_motion: None,
                source_span: None,
            }],
            in_view_once: Some(ExprIR::LiteralExpr {
                value: serde_json::json!(true),
            }),
            in_view_repeat: None,
            in_view_threshold: Some(ExprIR::LiteralExpr {
                value: serde_json::json!(0.5),
            }),
            source_span: None,
        })),
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
    };
    gen.emit_motion_triggers_for_element("el-inview-001", &el);
    let output = format!(
        "// === mount_js ===\n{}\n// === js ===\n{}",
        gen.mount_js, gen.js
    );
    assert!(
        output.contains("inView") || output.contains("IntersectionObserver"),
        "inview trigger output should contain 'inView' or 'IntersectionObserver'"
    );
    insta::assert_snapshot!("inview_trigger", &output);
}

/// Tests emit_motion_triggers_for_element with an @enter trigger on an element
/// that has a driven_by signal binding (driven_by is processed separately by
/// emit_element_motion_bindings, but this verifies the trigger path is correct).
#[test]
fn test_driven_by() {
    let mut gen = OmniViewGenerator::default();
    let el = ElementIR {
        tag: "div".into(),
        props: FxHashMap::default(),
        bindings: FxHashMap::default(),
        events: FxHashMap::default(),
        style_classes: None,
        inline_style: None,
        children: vec![],
        construction_kind: ConstructionKind::Static,
        class_map_bindings: FxHashMap::default(),
        source_span: None,
        motion: Some(Box::new(MotionBehavior {
            triggers: vec![TriggerIntent {
                kind: TriggerKind::Enter,
                motion_ref: Some("fadeIn".into()),
                inline_motion: None,
                source_span: None,
            }],
            in_view_once: None,
            in_view_repeat: None,
            in_view_threshold: None,
            source_span: None,
        })),
        ref_binding: None,
        semantic_bindings: FxHashMap::default(),
        a11y: None,
        drag_binding: None,
        modal: None,
        skeleton: None,
        error_boundary: None,
    };
    gen.emit_motion_triggers_for_element("el-driven-001", &el);
    let output = format!(
        "// === mount_js ===\n{}\n// === js ===\n{}\n// === enter_trigger_nids ===\n{:?}",
        gen.mount_js, gen.js, gen.enter_trigger_nids
    );
    assert!(
        output.contains("enter trigger"),
        "should contain enter trigger output"
    );
    insta::assert_snapshot!("driven_by", &output);
}

// ═════════════════════════════════════════════════════════════════════════════
// Phase 3c: Motion JS Variations (stagger, timeline with items)
// ═════════════════════════════════════════════════════════════════════════════

/// Tests emit_motion_js with stagger_children and stagger_from in the playback config.
/// Verifies the factory function includes stagger configuration in the options.
#[test]
fn test_stagger_children() {
    let mut gen = OmniViewGenerator::default();
    let motion = MotionIntent {
        name: "staggerAnim".into(),
        source_file: None,
        body: KeyframeIR {
            stops: vec![
                KeyframeStopIR {
                    position: 0.0,
                    properties: FxHashMap::from_iter([(
                        "opacity".into(),
                        ExprIR::LiteralExpr {
                            value: serde_json::json!(0),
                        },
                    )]),
                    ease: None,
                    source_span: None,
                },
                KeyframeStopIR {
                    position: 1.0,
                    properties: FxHashMap::from_iter([(
                        "opacity".into(),
                        ExprIR::LiteralExpr {
                            value: serde_json::json!(1),
                        },
                    )]),
                    ease: None,
                    source_span: None,
                },
            ],
            over: ExprIR::LiteralExpr {
                value: serde_json::json!(300),
            },
            ease: Some(EasingIR::Named("ease-out".into())),
            playback: PlaybackIR::default(),
            source_span: None,
        },
        source_span: None,
    };
    gen.emit_motion_js(&motion);
    // Note: stagger_children in PlaybackIR is currently used by
    // playback_to_config (for timeline items), but keyframe_to_animate_call
    // only handles fill/direction/loop_count/delay from playback inline.
    // This test verifies the factory function is emitted with the correct
    // keyframe data and serves as a baseline when stagger is added.
    assert!(
        gen.js.contains("motion_staggerAnim"),
        "should emit motion factory function"
    );
    insta::assert_snapshot!("stagger_children", &gen.js);
}

// [Removed] test_timeline_with_items — TimelineIR no longer exists
