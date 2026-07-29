//! Regression tests for OmniView motion/animation codegen.
//!
//! These are the first regression tests for the OmniView backend. They test motion
//! codegen output at three levels:
//!   Phase 1: Pure functions (no generator state needed)
//!   Phase 2: Read-only conversion (need OmniViewGenerator::test_new())
//!   Phase 3: State-mutating functions (need &mut OmniViewGenerator)

use super::super::*;
use rustc_hash::FxHashMap;

// ═════════════════════════════════════════════════════════════════════════════
// Phase 1: Pure Function Tests
// ═════════════════════════════════════════════════════════════════════════════

/// Verifies all 6 CSS transform shorthand mappings plus the unknown-key fallback.
#[test]
fn test_format_transform_values() {
    // x → translateX(...px)
    assert_eq!(
        OmniViewGenerator::format_transform_value("x", "100"),
        "translateX(100px)"
    );
    // y → translateY(...px)
    assert_eq!(
        OmniViewGenerator::format_transform_value("y", "-50"),
        "translateY(-50px)"
    );
    // scale, scaleX, scaleY → key(value)
    assert_eq!(
        OmniViewGenerator::format_transform_value("scale", "1.5"),
        "scale(1.5)"
    );
    assert_eq!(
        OmniViewGenerator::format_transform_value("scaleX", "2"),
        "scaleX(2)"
    );
    assert_eq!(
        OmniViewGenerator::format_transform_value("scaleY", "0.5"),
        "scaleY(0.5)"
    );
    // rotate, rotateX, rotateY, rotateZ → key(...deg)
    assert_eq!(
        OmniViewGenerator::format_transform_value("rotate", "45"),
        "rotate(45deg)"
    );
    assert_eq!(
        OmniViewGenerator::format_transform_value("rotateX", "90"),
        "rotateX(90deg)"
    );
    assert_eq!(
        OmniViewGenerator::format_transform_value("rotateY", "180"),
        "rotateY(180deg)"
    );
    assert_eq!(
        OmniViewGenerator::format_transform_value("rotateZ", "360"),
        "rotateZ(360deg)"
    );
    // Unknown key: passes through as key(value) — no px or deg suffix
    assert_eq!(
        OmniViewGenerator::format_transform_value("skewX", "10"),
        "skewX(10)"
    );
    assert_eq!(
        OmniViewGenerator::format_transform_value("translateX", "50"),
        "translateX(50)"
    );
    // Edge: zero and decimal values
    assert_eq!(
        OmniViewGenerator::format_transform_value("x", "0"),
        "translateX(0px)"
    );
    assert_eq!(
        OmniViewGenerator::format_transform_value("scale", "0.001"),
        "scale(0.001)"
    );
}

/// Verifies `is_transform_prop` correctly identifies all 9 recognized shorthand
/// transform properties and rejects standard CSS properties and full CSS names.
#[test]
fn test_is_transform_prop() {
    // All 9 recognized transform shorthand properties
    assert!(OmniViewGenerator::is_transform_prop("x"));
    assert!(OmniViewGenerator::is_transform_prop("y"));
    assert!(OmniViewGenerator::is_transform_prop("scale"));
    assert!(OmniViewGenerator::is_transform_prop("scaleX"));
    assert!(OmniViewGenerator::is_transform_prop("scaleY"));
    assert!(OmniViewGenerator::is_transform_prop("rotate"));
    assert!(OmniViewGenerator::is_transform_prop("rotateX"));
    assert!(OmniViewGenerator::is_transform_prop("rotateY"));
    assert!(OmniViewGenerator::is_transform_prop("rotateZ"));

    // Standard CSS properties — NOT transform shorthands
    assert!(!OmniViewGenerator::is_transform_prop("opacity"));
    assert!(!OmniViewGenerator::is_transform_prop("color"));
    assert!(!OmniViewGenerator::is_transform_prop("width"));
    assert!(!OmniViewGenerator::is_transform_prop("height"));
    assert!(!OmniViewGenerator::is_transform_prop("background"));
    assert!(!OmniViewGenerator::is_transform_prop("border-radius"));

    // Full CSS transform function names — NOT shorthand
    assert!(!OmniViewGenerator::is_transform_prop("translateX"));
    assert!(!OmniViewGenerator::is_transform_prop("translateY"));
    assert!(!OmniViewGenerator::is_transform_prop("translateZ"));

    // Edge: empty string
    assert!(!OmniViewGenerator::is_transform_prop(""));
}

// ═════════════════════════════════════════════════════════════════════════════
// Phase 2: Read-Only Conversion Tests (need OmniViewGenerator::test_new())
// ═════════════════════════════════════════════════════════════════════════════

/// Tests all 6 EasingIR variants, including optional field defaults.
#[test]
fn test_easing_all_variants() {
    let mut gen = OmniViewGenerator::test_new();

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

    // Spring with default velocity (None → 0)
    let sp = gen.easing_to_js(&EasingIR::Spring {
        tension: 180.0,
        friction: 12.0,
        velocity: None,
    });
    assert!(sp.contains("spring"));
    assert!(sp.contains("velocity:0"), "default velocity should be 0");

    // Spring with explicit velocity
    let sp2 = gen.easing_to_js(&EasingIR::Spring {
        tension: 100.0,
        friction: 10.0,
        velocity: Some(0.5),
    });
    assert!(sp2.contains("velocity:0.5"));

    // Back with default direction (None → "out")
    let bk = gen.easing_to_js(&EasingIR::Back {
        intensity: 1.5,
        direction: None,
    });
    assert!(bk.contains("back"));
    assert!(
        bk.contains("direction:\"out\""),
        "default Back direction should be \"out\""
    );

    // Back with explicit "in" direction
    let bk2 = gen.easing_to_js(&EasingIR::Back {
        intensity: 2.0,
        direction: Some("in".into()),
    });
    assert!(bk2.contains("direction:\"in\""));

    // Elastic with default direction (None → "out")
    let el = gen.easing_to_js(&EasingIR::Elastic {
        amplitude: 1.0,
        period: 0.5,
        direction: None,
    });
    assert!(el.contains("elastic"));
    assert!(el.contains("direction:\"out\""));

    // Steps
    let st = gen.easing_to_js(&EasingIR::Steps {
        count: 4,
        direction: "start".into(),
    });
    assert!(st.contains("steps"), "Steps should contain type marker");
    assert!(st.contains("count:4"));
}

/// Tests playback_to_config for each configurable field in PlaybackIR.
#[test]
fn test_playback_config() {
    let mut gen = OmniViewGenerator::test_new();

    // Empty playback → no config properties
    let empty = gen.playback_to_config(&PlaybackIR::default());
    assert!(empty.is_empty(), "empty playback should produce no config");

    // Loop infinite → "iterations": Infinity
    let inf = gen.playback_to_config(&PlaybackIR {
        loop_count: Some(LoopValue::Infinite),
        ..Default::default()
    });
    assert!(inf.iter().any(|s| s.contains("Infinity")));

    // Loop count → "iterations": N
    let cnt = gen.playback_to_config(&PlaybackIR {
        loop_count: Some(LoopValue::Count(3)),
        ..Default::default()
    });
    assert!(cnt.iter().any(|s| s.contains("\"iterations\": 3")));

    // Direction → "direction": "..."
    let dir = gen.playback_to_config(&PlaybackIR {
        direction: Some(DirectionValue::Alternate),
        ..Default::default()
    });
    assert!(dir.iter().any(|s| s.contains("alternate")));
    let rev = gen.playback_to_config(&PlaybackIR {
        direction: Some(DirectionValue::Reverse),
        ..Default::default()
    });
    assert!(rev.iter().any(|s| s.contains("reverse")));

    // Fill → "fill": "..."
    let fill = gen.playback_to_config(&PlaybackIR {
        fill: Some(FillValue::Both),
        ..Default::default()
    });
    assert!(fill.iter().any(|s| s.contains("both")));
    let fwd = gen.playback_to_config(&PlaybackIR {
        fill: Some(FillValue::Forwards),
        ..Default::default()
    });
    assert!(fwd.iter().any(|s| s.contains("forwards")));

    // Delay → "delay": value
    let delay = gen.playback_to_config(&PlaybackIR {
        delay: Some(ExprIR::LiteralExpr {
            value: serde_json::json!(500),
        }),
        ..Default::default()
    });
    assert!(delay.iter().any(|s| s.contains("\"delay\": 500")));

    // Paused → "paused": true
    let paused = gen.playback_to_config(&PlaybackIR {
        paused: true,
        ..Default::default()
    });
    assert!(paused.iter().any(|s| s.contains("paused")));

    // respectReducedMotion
    let rrm = gen.playback_to_config(&PlaybackIR {
        respect_reduced_motion: true,
        ..Default::default()
    });
    assert!(rrm.iter().any(|s| s.contains("respectReducedMotion")));

    // Stagger children
    let stagger = gen.playback_to_config(&PlaybackIR {
        stagger_children: Some(ExprIR::LiteralExpr {
            value: serde_json::json!(0.05),
        }),
        ..Default::default()
    });
    assert!(stagger.iter().any(|s| s.contains("staggerChildren")));

    // Interrupt behavior
    let int = gen.playback_to_config(&PlaybackIR {
        interrupt: Some(InterruptValue::Cancel),
        ..Default::default()
    });
    assert!(int.iter().any(|s| s.contains("cancel")));

    // When (timeline child ordering)
    let when = gen.playback_to_config(&PlaybackIR {
        when: Some(WhenValue::Idle),
        ..Default::default()
    });
    assert!(when.iter().any(|s| s.contains("beforeChildren")));

    // StaggerFrom direction
    let sf = gen.playback_to_config(&PlaybackIR {
        stagger_from: Some(StaggerDirection::First),
        ..Default::default()
    });
    assert!(sf.iter().any(|s| s.contains("staggerFrom")));

    // StaggerFrom: Random → mapped to "edges" (as per current implementation)
    let sf_rand = gen.playback_to_config(&PlaybackIR {
        stagger_from: Some(StaggerDirection::Random),
        ..Default::default()
    });
    assert!(
        sf_rand.iter().any(|s| s.contains("edges")),
        "StaggerDirection::Random should map to 'edges'"
    );

    // Repeat delay
    let rd = gen.playback_to_config(&PlaybackIR {
        repeat_delay: Some(ExprIR::LiteralExpr {
            value: serde_json::json!(200),
        }),
        ..Default::default()
    });
    assert!(rd.iter().any(|s| s.contains("repeatDelay")));
}

/// Tests keyframe_to_animate_call with both from/to stops.
#[test]
fn test_keyframe_animate_call_basic() {
    let mut gen = OmniViewGenerator::test_new();
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
        color_space: None,
        playback: PlaybackIR::default(),
        path: None,
        path_rotate: None,
        callbacks: vec![],

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
    let mut gen = OmniViewGenerator::test_new();

    // Edge case 1: No stops → empty keyframe array
    let empty = KeyframeIR {
        stops: vec![],
        over: ExprIR::LiteralExpr {
            value: serde_json::json!(300),
        },
        ease: None,
        color_space: None,
        playback: PlaybackIR::default(),
        path: None,
        path_rotate: None,
        callbacks: vec![],

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
        color_space: None,
        playback: PlaybackIR::default(),
        path: None,
        path_rotate: None,
        callbacks: vec![],

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
        color_space: None,
        playback: PlaybackIR::default(),
        path: None,
        path_rotate: None,
        callbacks: vec![],

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

/// Tests keyframe_to_animate_call with path animation and path rotation (path fields
/// are not emitted by keyframe_to_animate_call itself; timeline.rs handles them).
#[test]
fn test_keyframe_animate_call_with_path() {
    let mut gen = OmniViewGenerator::test_new();
    let kf = KeyframeIR {
        stops: vec![
            KeyframeStopIR {
                position: 0.0,
                properties: FxHashMap::from_iter([(
                    "x".into(),
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
                    "x".into(),
                    ExprIR::LiteralExpr {
                        value: serde_json::json!(200),
                    },
                )]),
                ease: None,

                source_span: None,
            },
        ],
        over: ExprIR::LiteralExpr {
            value: serde_json::json!(1000),
        },
        ease: Some(EasingIR::Named("linear".into())),
        color_space: None,
        playback: PlaybackIR::default(),
        path: Some("M0,0 C100,200 200,200 300,0".into()),
        path_rotate: Some(true),
        callbacks: vec![],

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
        kf_js.contains("\"x\""),
        "keyframes should contain 'x' property"
    );

    // Options: duration, easing
    assert!(
        opts_js.contains("duration:1000"),
        "options should contain duration:1000"
    );
    assert!(
        opts_js.contains("linear"),
        "options should contain easing name"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Phase 3: State-Mutating Function Tests (need &mut OmniViewGenerator)
// ═════════════════════════════════════════════════════════════════════════════

/// Tests that emit_motion_js produces a valid `__omni_motion_{name}` factory
/// function for a simple keyframe animation (fade-in: opacity 0 → 1).
#[test]
fn test_emit_motion_js_keyframe() {
    let mut gen = OmniViewGenerator::test_new();
    let motion = MotionIntent {
        name: "fadeIn".into(),
        body: MotionBodyIR::Keyframe(KeyframeIR {
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
            color_space: None,
            playback: PlaybackIR::default(),
            path: None,
            path_rotate: None,
            callbacks: vec![],

            source_span: None,
        }),

        source_span: None,
    };
    gen.emit_motion_js(&motion);
    insta::assert_snapshot!("emit_motion_js_keyframe", &gen.js);
}

/// Tests that emit_motion_js produces a correct factory for callback-only
/// motion definitions (no visual properties, just callbacks).
#[test]
fn test_emit_motion_js_callback_only() {
    let mut gen = OmniViewGenerator::test_new();
    let motion = MotionIntent {
        name: "logComplete".into(),
        body: MotionBodyIR::CallbackOnly(CallbackBodyIR {
            callbacks: vec![(
                CallbackKindIR::OnComplete,
                ExprIR::LiteralExpr {
                    value: serde_json::json!("animation done"),
                },
            )],
            over: ExprIR::LiteralExpr {
                value: serde_json::json!(500),
            },
            playback: PlaybackIR::default(),

            source_span: None,
        }),

        source_span: None,
    };
    gen.emit_motion_js(&motion);
    insta::assert_snapshot!("emit_motion_js_callback_only", &gen.js);
}

/// Tests that emit_motion_js delegates to timeline stub for Timeline body.
#[test]
fn test_emit_motion_js_timeline() {
    let mut gen = OmniViewGenerator::test_new();
    let motion = MotionIntent {
        name: "sequenceAnim".into(),
        body: MotionBodyIR::Timeline(TimelineIR {
            items: vec![],
            playback: PlaybackIR::default(),

            source_span: None,
        }),

        source_span: None,
    };
    gen.emit_motion_js(&motion);
    insta::assert_snapshot!("emit_motion_js_timeline", &gen.js);
}

/// Tests emit_states_js with reserved state names (idle/hover/pressed),
/// which auto-binds mouse events to transition between states.
#[test]
fn test_emit_states_js_reserved() {
    let mut gen = OmniViewGenerator::test_new();
    let states = StatesIR {
        states: FxHashMap::from_iter([
            (
                "idle".into(),
                StateTweenIR {
                    properties: FxHashMap::from_iter([(
                        "opacity".into(),
                        ExprIR::LiteralExpr {
                            value: serde_json::json!(1),
                        },
                    )]),
                    over: None,
                    ease: None,

                    source_span: None,
                },
            ),
            (
                "hover".into(),
                StateTweenIR {
                    properties: FxHashMap::from_iter([
                        (
                            "opacity".into(),
                            ExprIR::LiteralExpr {
                                value: serde_json::json!(0.8),
                            },
                        ),
                        (
                            "scale".into(),
                            ExprIR::LiteralExpr {
                                value: serde_json::json!(1.05),
                            },
                        ),
                    ]),
                    over: Some(ExprIR::LiteralExpr {
                        value: serde_json::json!(200),
                    }),
                    ease: Some(EasingIR::Named("ease-out".into())),

                    source_span: None,
                },
            ),
            (
                "pressed".into(),
                StateTweenIR {
                    properties: FxHashMap::from_iter([
                        (
                            "opacity".into(),
                            ExprIR::LiteralExpr {
                                value: serde_json::json!(0.6),
                            },
                        ),
                        (
                            "scale".into(),
                            ExprIR::LiteralExpr {
                                value: serde_json::json!(0.95),
                            },
                        ),
                    ]),
                    over: Some(ExprIR::LiteralExpr {
                        value: serde_json::json!(100),
                    }),
                    ease: Some(EasingIR::Named("ease-in".into())),

                    source_span: None,
                },
            ),
        ]),
        signal: None,

        source_span: None,
    };
    gen.emit_states_js("el-card-001", &states);

    // The function emits to both mount_js (event bindings) and js (lookup table).
    // Snapshot combines both for verification.
    let output = format!(
        "// === mount_js ===\n{}\n// === js ===\n{}",
        gen.mount_js, gen.js
    );
    insta::assert_snapshot!("emit_states_js_reserved", &output);
}

/// Tests emit_motion_triggers_for_element with a named @enter motion reference.
/// Verifies the mount_js output and the enter_trigger_nids registration.
#[test]
fn test_motion_trigger_enter_named() {
    let mut gen = OmniViewGenerator::test_new();
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
            states: None,
            paused_override: None,
            driven_by: None,
            hover_reverse: None,
            in_view_once: None,
            in_view_repeat: None,
            in_view_threshold: None,
            source_span: None,
        })),
        scroll: None,
        drag: None,
        hero: None,
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
    let mut gen = OmniViewGenerator::test_new();
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
            states: None,
            paused_override: None,
            driven_by: None,
            hover_reverse: None,
            in_view_once: None,
            in_view_repeat: None,
            in_view_threshold: None,
            source_span: None,
        })),
        scroll: None,
        drag: None,
        hero: None,
    };
    gen.emit_motion_triggers_for_element("el-btn-002", &el);

    insta::assert_snapshot!("motion_trigger_hover_named", &gen.js);
}

// ═════════════════════════════════════════════════════════════════════════════
// Phase 3b: Motion Trigger + Behavior Variations
// ═════════════════════════════════════════════════════════════════════════════

/// Tests emit_motion_triggers_for_element with an @scroll trigger.
/// Tests the Scroll trigger path which reads scroll behavior fields (start, end, scrub).
#[test]
fn test_scroll_trigger() {
    let mut gen = OmniViewGenerator::test_new();
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
                kind: TriggerKind::Scroll,
                motion_ref: Some("fadeIn".into()),
                inline_motion: None,
                source_span: None,
            }],
            states: None,
            paused_override: None,
            driven_by: None,
            hover_reverse: None,
            in_view_once: None,
            in_view_repeat: None,
            in_view_threshold: None,
            source_span: None,
        })),
        scroll: Some(Box::new(ScrollBehavior {
            start: Some("0".into()),
            end: Some("500".into()),
            scrub: Some(ExprIR::LiteralExpr {
                value: serde_json::json!(true),
            }),
            source_span: None,
        })),
        drag: None,
        hero: None,
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
    let mut gen = OmniViewGenerator::test_new();
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
                kind: TriggerKind::DragHorizontal,
                motion_ref: Some("fadeIn".into()),
                inline_motion: None,
                source_span: None,
            }],
            states: None,
            paused_override: None,
            driven_by: None,
            hover_reverse: None,
            in_view_once: None,
            in_view_repeat: None,
            in_view_threshold: None,
            source_span: None,
        })),
        scroll: None,
        drag: Some(Box::new(DragBehavior {
            constraints: Some("left: 0; right: 300; top: 0; bottom: 0".into()),
            elastic: Some(ExprIR::LiteralExpr {
                value: serde_json::json!(0.1),
            }),
            scrub: None,
            momentum: Some(ExprIR::LiteralExpr {
                value: serde_json::json!(true),
            }),
            threshold: None,
            velocity_max: None,
            source_span: None,
        })),
        hero: None,
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

/// Tests emit_motion_triggers_for_element with an @in-view trigger and
/// in_view_once / in_view_threshold configurations.
#[test]
fn test_inview_trigger() {
    let mut gen = OmniViewGenerator::test_new();
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
            states: None,
            paused_override: None,
            driven_by: None,
            hover_reverse: None,
            in_view_once: Some(ExprIR::LiteralExpr {
                value: serde_json::json!(true),
            }),
            in_view_repeat: None,
            in_view_threshold: Some(ExprIR::LiteralExpr {
                value: serde_json::json!(0.5),
            }),
            source_span: None,
        })),
        scroll: None,
        drag: None,
        hero: None,
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
    let mut gen = OmniViewGenerator::test_new();
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
            states: None,
            paused_override: None,
            driven_by: Some(ExprIR::IdentifierExpr {
                name: "scrollProgress".into(),
                type_ann: None,
            }),
            hover_reverse: None,
            in_view_once: None,
            in_view_repeat: None,
            in_view_threshold: None,
            source_span: None,
        })),
        scroll: None,
        drag: None,
        hero: None,
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

/// Tests emit_motion_triggers_for_element with an @enter trigger on an element
/// that has a paused_override signal binding (paused_override is processed
/// separately by emit_element_motion_bindings).
#[test]
fn test_paused_override() {
    let mut gen = OmniViewGenerator::test_new();
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
            states: None,
            paused_override: Some(ExprIR::IdentifierExpr {
                name: "isPaused".into(),
                type_ann: None,
            }),
            driven_by: None,
            hover_reverse: None,
            in_view_once: None,
            in_view_repeat: None,
            in_view_threshold: None,
            source_span: None,
        })),
        scroll: None,
        drag: None,
        hero: None,
    };
    gen.emit_motion_triggers_for_element("el-paused-001", &el);
    let output = format!(
        "// === mount_js ===\n{}\n// === js ===\n{}\n// === enter_trigger_nids ===\n{:?}",
        gen.mount_js, gen.js, gen.enter_trigger_nids
    );
    assert!(
        output.contains("enter trigger"),
        "should contain enter trigger output"
    );
    insta::assert_snapshot!("paused_override", &output);
}

// ═════════════════════════════════════════════════════════════════════════════
// Phase 3c: Motion JS Variations (stagger, timeline with items)
// ═════════════════════════════════════════════════════════════════════════════

/// Tests emit_motion_js with stagger_children and stagger_from in the playback config.
/// Verifies the factory function includes stagger configuration in the options.
#[test]
fn test_stagger_children() {
    let mut gen = OmniViewGenerator::test_new();
    let motion = MotionIntent {
        name: "staggerAnim".into(),
        body: MotionBodyIR::Keyframe(KeyframeIR {
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
            color_space: None,
            playback: PlaybackIR {
                stagger_children: Some(ExprIR::LiteralExpr {
                    value: serde_json::json!(0.1),
                }),
                stagger_from: Some(StaggerDirection::Center),
                ..Default::default()
            },
            path: None,
            path_rotate: None,
            callbacks: vec![],
            source_span: None,
        }),
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

/// Tests emit_motion_js with a Timeline body containing items.
/// Timeline items are handled via timeline_to_factory stub which currently
/// produces a simple keyframe-based factory function.
#[test]
fn test_timeline_with_items() {
    let mut gen = OmniViewGenerator::test_new();
    let motion = MotionIntent {
        name: "tlAnim".into(),
        body: MotionBodyIR::Timeline(TimelineIR {
            items: vec![TimelineItemIR {
                kind: TimelineItemKindIR::Direction {
                    dir: DirectionIR::To,
                    properties: FxHashMap::from_iter([(
                        "opacity".into(),
                        ExprIR::LiteralExpr {
                            value: serde_json::json!(0),
                        },
                    )]),
                    over: ExprIR::LiteralExpr {
                        value: serde_json::json!(300),
                    },
                    ease: None,
                    delay: None,
                    color_space: None,
                    start: None,
                    path: None,
                    path_rotate: None,
                },
                connector: ConnectorIR::Then,
                source_span: None,
            }],
            playback: PlaybackIR::default(),
            source_span: None,
        }),
        source_span: None,
    };
    gen.emit_motion_js(&motion);
    assert!(
        gen.js.contains("__omni_motion_tlAnim"),
        "should emit motion factory function"
    );
    insta::assert_snapshot!("timeline_with_items", &gen.js);
}
