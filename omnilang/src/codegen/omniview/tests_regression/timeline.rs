//! Regression tests for OmniView timeline codegen.
//!
//! Covers:
//! - Direction::To items with default cursor behavior
//! - Start mode delay calculation (overlap, absolute, with-prev)
//! - Label emission with optional callbacks
//! - Warning accumulation for invalid timing values

use super::super::*;
use rustc_hash::FxHashMap;

// ═════════════════════════════════════════════════════════════════════════════
// Timeline Codegen Tests
// ═════════════════════════════════════════════════════════════════════════════

/// Tests a basic timeline with a single Direction::To item.
/// Verifies the factory function structure and default timing values.
#[test]
fn test_timeline_basic_direction_to() {
    let mut gen = OmniViewGenerator::test_new();
    let timeline = TimelineIntent {
        name: "fadeIn".into(),
        items: vec![TimelineItemIR {
            kind: TimelineItemKindIR::Direction {
                dir: DirectionIR::To,
                properties: FxHashMap::from_iter([(
                    "opacity".into(),
                    ExprIR::LiteralExpr {
                        value: serde_json::json!(1),
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
        source_span: None,
    };
    gen.emit_timeline_js(&timeline);

    // Factory function structure
    assert!(
        gen.js.contains("function __omni_motion_fadeIn"),
        "should define factory function"
    );
    assert!(
        gen.js.contains("var steps = []"),
        "should initialize steps array"
    );
    assert!(
        gen.js.contains("Omni.timeline(steps)"),
        "should call Omni.timeline"
    );

    // Delay calculation: first item at 0
    assert!(gen.js.contains("at: 0"), "first item should start at 0");
    assert!(
        gen.js.contains("children[0]"),
        "should reference children[0]"
    );

    // Options with duration and default easing
    assert!(gen.js.contains("duration: 300"), "duration should be 300ms");
    assert!(
        gen.js.contains("easing: \"easeOut\""),
        "default easing should be easeOut"
    );

    // Mount call present
    assert!(
        gen.mount_js.contains("__omni_motion_fadeIn(null, {});"),
        "mount_js should contain mount call"
    );

    // Guard clause against null element
    assert!(
        gen.js.contains("if (!el) return { kill: function() {} }"),
        "should guard null element"
    );

    // No warnings
    assert!(gen.warnings.is_empty(), "no warnings expected");
}

/// Tests timeline delay calculation with various start modes:
///   0. Direction::To (x→100, over=300ms, no start) → at: 0.0
///   1. Direction::To (opacity→0.5, over=200ms, start=overlap(100))
///      → pair-based: ref_cursor(0)-100 = 0.0
///   2. Direction::To (x→200, over=500ms, start=absolute(200))
///      → absolute(200) = 200.0
///   3. Direction::To (y→50, over=300ms, start=with-prev)
///      → pair-based: ref_prev_start(0) = 0.0
#[test]
fn test_timeline_start_modes() {
    let mut gen = OmniViewGenerator::test_new();
    let timeline = TimelineIntent {
        name: "seq".into(),
        items: vec![
            TimelineItemIR {
                kind: TimelineItemKindIR::Direction {
                    dir: DirectionIR::To,
                    properties: FxHashMap::from_iter([(
                        "x".into(),
                        ExprIR::LiteralExpr {
                            value: serde_json::json!(100),
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
            },
            TimelineItemIR {
                kind: TimelineItemKindIR::Direction {
                    dir: DirectionIR::To,
                    properties: FxHashMap::from_iter([(
                        "opacity".into(),
                        ExprIR::LiteralExpr {
                            value: serde_json::json!(0.5),
                        },
                    )]),
                    over: ExprIR::LiteralExpr {
                        value: serde_json::json!(200),
                    },
                    ease: None,
                    delay: None,
                    color_space: None,
                    start: Some("overlap(100)".into()),
                    path: None,
                    path_rotate: None,
                },
                connector: ConnectorIR::Then,

                source_span: None,
            },
            TimelineItemIR {
                kind: TimelineItemKindIR::Direction {
                    dir: DirectionIR::To,
                    properties: FxHashMap::from_iter([(
                        "x".into(),
                        ExprIR::LiteralExpr {
                            value: serde_json::json!(200),
                        },
                    )]),
                    over: ExprIR::LiteralExpr {
                        value: serde_json::json!(500),
                    },
                    ease: None,
                    delay: None,
                    color_space: None,
                    start: Some("absolute(200)".into()),
                    path: None,
                    path_rotate: None,
                },
                connector: ConnectorIR::Then,

                source_span: None,
            },
            TimelineItemIR {
                kind: TimelineItemKindIR::Direction {
                    dir: DirectionIR::To,
                    properties: FxHashMap::from_iter([(
                        "y".into(),
                        ExprIR::LiteralExpr {
                            value: serde_json::json!(50),
                        },
                    )]),
                    over: ExprIR::LiteralExpr {
                        value: serde_json::json!(300),
                    },
                    ease: None,
                    delay: None,
                    color_space: None,
                    start: Some("with-prev".into()),
                    path: None,
                    path_rotate: None,
                },
                connector: ConnectorIR::Then,

                source_span: None,
            },
        ],
        source_span: None,
    };
    gen.emit_timeline_js(&timeline);

    // Count steps.push lines to verify 4 items
    let steps_count = gen.js.matches("steps.push({").count();
    assert_eq!(steps_count, 4, "should have 4 timeline items");

    // Verify children indices
    assert!(gen.js.contains("children[0]"), "should have children[0]");
    assert!(gen.js.contains("children[1]"), "should have children[1]");
    assert!(gen.js.contains("children[2]"), "should have children[2]");
    assert!(gen.js.contains("children[3]"), "should have children[3]");

    // Verify durations
    assert!(gen.js.contains("duration: 300"), "item 0 should be 300ms");
    assert!(gen.js.contains("duration: 200"), "item 1 should be 200ms");
    assert!(gen.js.contains("duration: 500"), "item 2 should be 500ms");

    // Verify that absolute(200) produced at: 200
    assert!(
        gen.js.contains("at: 200"),
        "absolute(200) item should have at: 200"
    );

    // No warnings for valid input
    assert!(gen.warnings.is_empty(), "no warnings expected");
}

/// Tests timeline items that delegate to named motions and stagger children.
/// Verifies MotionRef and Stagger item emission.
#[test]
fn test_timeline_motion_ref_and_stagger() {
    let mut gen = OmniViewGenerator::test_new();
    let timeline = TimelineIntent {
        name: "composite".into(),
        items: vec![
            TimelineItemIR {
                kind: TimelineItemKindIR::Direction {
                    dir: DirectionIR::To,
                    properties: FxHashMap::from_iter([(
                        "opacity".into(),
                        ExprIR::LiteralExpr {
                            value: serde_json::json!(1),
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
            },
            TimelineItemIR {
                kind: TimelineItemKindIR::MotionRef("fadeIn".into()),
                connector: ConnectorIR::Then,

                source_span: None,
            },
            TimelineItemIR {
                kind: TimelineItemKindIR::Stagger(
                    Box::new(MotionExprIR::Ref("slideIn".into())),
                    StaggerTimingIR {
                        each: ExprIR::LiteralExpr {
                            value: serde_json::json!(80),
                        },
                        from: None,

                        source_span: None,
                    },
                    StaggerDirection::First,
                ),
                connector: ConnectorIR::Then,

                source_span: None,
            },
        ],
        source_span: None,
    };
    gen.emit_timeline_js(&timeline);

    // MotionRef: forEach with named motion
    assert!(
        gen.js.contains("children.forEach(function(child)"),
        "MotionRef should use forEach"
    );
    assert!(
        gen.js.contains("motion: \"fadeIn\""),
        "MotionRef should reference motion name"
    );
    assert!(
        gen.js.contains("motion: \"slideIn\""),
        "Stagger should reference motion name"
    );

    // Stagger: forEach with idx-based delay
    assert!(
        gen.js.contains("children.forEach(function(child, idx)"),
        "Stagger should use forEach with idx"
    );
    assert!(
        gen.js.contains("idx * 80"),
        "Stagger should multiply idx by each delay"
    );

    // No warnings
    assert!(gen.warnings.is_empty(), "no warnings expected");
}

/// Tests timeline labels with and without callbacks.
/// Verifies label map emission (`window.__omni_timeline_labels`)
/// and callback steps for labels with on_enter.
#[test]
fn test_timeline_labels_and_callbacks() {
    let mut gen = OmniViewGenerator::test_new();
    let timeline = TimelineIntent {
        name: "labeledTimeline".into(),
        items: vec![
            TimelineItemIR {
                kind: TimelineItemKindIR::Direction {
                    dir: DirectionIR::To,
                    properties: FxHashMap::from_iter([(
                        "opacity".into(),
                        ExprIR::LiteralExpr {
                            value: serde_json::json!(1),
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
            },
            TimelineItemIR {
                kind: TimelineItemKindIR::Label(
                    "afterFade".into(),
                    Some(ExprIR::LiteralExpr {
                        value: serde_json::json!("markerReached"),
                    }),
                ),
                connector: ConnectorIR::Then,

                source_span: None,
            },
            TimelineItemIR {
                kind: TimelineItemKindIR::Label("endMarker".into(), None),
                connector: ConnectorIR::Then,

                source_span: None,
            },
        ],
        source_span: None,
    };
    gen.emit_timeline_js(&timeline);

    // Label map should be emitted
    assert!(
        gen.js.contains("window.__omni_timeline_labels"),
        "should emit label map"
    );
    assert!(
        gen.js.contains("afterFade"),
        "should contain 'afterFade' label"
    );
    assert!(
        gen.js.contains("endMarker"),
        "should contain 'endMarker' label"
    );

    // Callback step should be emitted for the label with on_enter
    assert!(
        gen.js.contains("steps.push({ call: function()"),
        "should emit callback step for label with on_enter"
    );

    // No warnings
    assert!(gen.warnings.is_empty(), "no warnings expected");
}

/// Tests warning accumulation for invalid timing values:
/// - Negative `over` duration → should warn and default to 300ms
/// - Non-numeric `over` literal → should warn and default to 300ms
/// - Stagger with negative `each` delay
#[test]
fn test_timeline_warnings_invalid_timing() {
    let mut gen = OmniViewGenerator::test_new();
    let timeline = TimelineIntent {
        name: "badTiming".into(),
        items: vec![
            // Item 0: negative over → warning, defaults to 300ms
            TimelineItemIR {
                kind: TimelineItemKindIR::Direction {
                    dir: DirectionIR::To,
                    properties: FxHashMap::from_iter([(
                        "opacity".into(),
                        ExprIR::LiteralExpr {
                            value: serde_json::json!(1),
                        },
                    )]),
                    over: ExprIR::LiteralExpr {
                        value: serde_json::json!(-100),
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
            },
            // Item 1: non-numeric over literal → warning, defaults to 300ms
            TimelineItemIR {
                kind: TimelineItemKindIR::Direction {
                    dir: DirectionIR::To,
                    properties: FxHashMap::from_iter([(
                        "x".into(),
                        ExprIR::LiteralExpr {
                            value: serde_json::json!(50),
                        },
                    )]),
                    over: ExprIR::LiteralExpr {
                        value: serde_json::json!("not-a-number"),
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
            },
            // Item 2: Direction with invalid stagger each via a Stagger item
            // (not in this test — separate from Direction items)
        ],
        source_span: None,
    };
    gen.emit_timeline_js(&timeline);

    // Exact warning count
    assert_eq!(gen.warnings.len(), 2, "should have 2 warnings");
    assert!(
        gen.warnings[0].contains("positive"),
        "first warning should mention positive duration"
    );
    assert!(
        gen.warnings[1].contains("not a valid number"),
        "second warning should mention invalid number"
    );

    // Default duration of 300ms should be used for both items
    assert!(
        gen.js.contains("duration: 300"),
        "default 300ms should be applied for invalid over values"
    );
}

/// Tests timeline Direction items with Set and FromTo variants.
/// Set items apply immediate property changes (zero-duration, fill:forwards).
#[test]
fn test_timeline_set_and_fromto() {
    let mut gen = OmniViewGenerator::test_new();
    let timeline = TimelineIntent {
        name: "setDemo".into(),
        items: vec![
            // FromTo: opacity from 0 → x to 100 (FromTo pairs: first=from, second=to)
            TimelineItemIR {
                kind: TimelineItemKindIR::Direction {
                    dir: DirectionIR::FromTo,
                    properties: FxHashMap::from_iter([
                        (
                            "opacity".into(),
                            ExprIR::LiteralExpr {
                                value: serde_json::json!(0),
                            },
                        ),
                        (
                            "x".into(),
                            ExprIR::LiteralExpr {
                                value: serde_json::json!(100),
                            },
                        ),
                    ]),
                    over: ExprIR::LiteralExpr {
                        value: serde_json::json!(400),
                    },
                    ease: Some(EasingIR::Named("ease-in".into())),
                    delay: None,
                    color_space: None,
                    start: None,
                    path: None,
                    path_rotate: None,
                },
                connector: ConnectorIR::Then,

                source_span: None,
            },
            // Set: scale → 2 immediately
            TimelineItemIR {
                kind: TimelineItemKindIR::Set(vec![(
                    "scale".into(),
                    ExprIR::LiteralExpr {
                        value: serde_json::json!(2),
                    },
                )]),
                connector: ConnectorIR::Then,

                source_span: None,
            },
        ],
        source_span: None,
    };
    gen.emit_timeline_js(&timeline);

    // FromTo keyframe format: [{from}, {to}] with flip-flop pair assignment
    assert!(
        gen.js
            .contains("keyframes: [{\"opacity\": 0}, {\"x\": 100}]"),
        "FromTo should produce from/to keyframes from pairs"
    );
    assert!(
        gen.js.contains("duration: 400"),
        "FromTo duration should be 400ms"
    );
    assert!(
        gen.js.contains("easing: \"ease-in\""),
        "FromTo should preserve custom easing"
    );

    // Set item: zero-duration, fill:forwards
    assert!(
        gen.js.contains("duration: 0, fill: \"forwards\""),
        "Set items should have zero duration with fill:forwards"
    );
    assert!(
        gen.js.contains("\"scale\": 2"),
        "Set items should contain property value"
    );

    // Cursor should advance correctly:
    // Item 0: at=0, duration=400 → cursor=400
    // Item 1: Set at cursor=400
    // (Set uses cursor_delay which is tracked from previous item)
    let at_400_count = gen.js.matches("at: 400").count();
    assert_eq!(at_400_count, 1, "Set item should be at cursor 400");

    assert!(gen.warnings.is_empty(), "no warnings expected");
}
