//! Motion / Animation IR type definitions.
//! Extracted from types.rs to keep each file under 800 lines.

use super::*;
use crate::lexer::Span;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════
// Motion / Animation IR
// ═══════════════════════════════════════════════════════
//
// The enums below are `#[non_exhaustive]`: every backend (9 targets) matches
// them in codegen, so a future variant addition must not silently break
// external match arms (implementation-strategist review, Task 0.6 note —
// "motion IR 触碰时补标 #[non_exhaustive]").

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionIntent {
    pub name: String,
    pub body: MotionBodyIR,
    /// Source file of the motion definition — patched from the real path at
    /// multi-file IR merge (analyse.rs `merge_file_ir_into`, the same patch
    /// pattern as component/store/design `source_file`; the module key is a
    /// per-file placeholder `"input.omni"` and must not reach diagnostics).
    #[serde(default)]
    pub source_file: Option<String>,
    #[serde(default)]
    pub source_span: Option<Span>,
}

/// Type alias: single-variant enum inlined to its only variant.
pub type MotionBodyIR = KeyframeIR;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyframeIR {
    pub stops: Vec<KeyframeStopIR>,
    pub over: ExprIR,
    pub ease: Option<EasingIR>,
    pub playback: PlaybackIR,
    #[serde(default)]
    pub source_span: Option<Span>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyframeStopIR {
    pub position: f64,
    pub properties: FxHashMap<String, ExprIR>,
    pub ease: Option<EasingIR>,
    #[serde(default)]
    pub source_span: Option<Span>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlaybackIR {
    pub loop_count: Option<LoopValue>,
    pub direction: Option<DirectionValue>,
    pub fill: Option<FillValue>,
    pub delay: Option<ExprIR>,
    #[serde(default)]
    pub source_span: Option<Span>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum EasingIR {
    Named(String),
    CubicBezier { x1: f64, y1: f64, x2: f64, y2: f64 },
    Back { direction: Option<String> },
    Elastic { direction: Option<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerIntent {
    pub kind: TriggerKind,
    pub motion_ref: Option<String>,
    pub inline_motion: Option<MotionBodyIR>,
    #[serde(default)]
    pub source_span: Option<Span>,
}

/// Trigger kind. `@scroll` / `@drag-*` carry their v1 control dimensions
/// (restored by the v2 BNF) as optional per-parameter fields:
///
/// ```text
/// scroll_params ::= 'start:' expr 'end:' expr 'scrub:' bool
/// drag_params   ::= 'constraints:' '[' expr ',' expr ']' 'elastic:' expr
///                   'threshold:' expr ('velocity_max:' expr)?
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum TriggerKind {
    Enter,
    Hover,
    Press,
    Focus,
    InView,
    Scroll {
        start: Option<ExprIR>,
        end: Option<ExprIR>,
        scrub: Option<ExprIR>,
    },
    DragHorizontal {
        constraints: Option<(ExprIR, ExprIR)>,
        elastic: Option<ExprIR>,
        threshold: Option<ExprIR>,
        velocity_max: Option<ExprIR>,
    },
    DragVertical {
        constraints: Option<(ExprIR, ExprIR)>,
        elastic: Option<ExprIR>,
        threshold: Option<ExprIR>,
        velocity_max: Option<ExprIR>,
    },
}

/// Value type for loop_count.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum LoopValue {
    Infinite,
    Count(u32),
}

/// Value type for playback direction.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DirectionValue {
    Normal,
    Reverse,
    Alternate,
    AlternateReverse,
}

/// Value type for fill mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum FillValue {
    None,
    Forwards,
    Backwards,
    Both,
}

// ═══════════════════════════════════════════════════════
// ElementIR behavior sub-structs
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MotionBehavior {
    pub triggers: Vec<TriggerIntent>,
    pub in_view_once: Option<ExprIR>,
    pub in_view_repeat: Option<ExprIR>,
    pub in_view_threshold: Option<ExprIR>,
    #[serde(default)]
    pub source_span: Option<Span>,
}

// Note: the former ScrollBehavior / DragBehavior structs (string-based
// "top 80%" / "left: 0 right: 300" configs) were deleted — the v1 control
// dimensions now live on TriggerKind::Scroll / Drag* as typed Option fields
// (Task 5.1), so there is a single source of truth per trigger.

#[cfg(test)]
mod event_data_tests {
    use super::types::*;

    #[test]
    fn test_map_event_data_kind_pointer() {
        assert_eq!(
            map_event_data_kind("onClick"),
            Some(EventDataKind::PointerEvent)
        );
        assert_eq!(
            map_event_data_kind("onMouseMove"),
            Some(EventDataKind::PointerEvent)
        );
        assert_eq!(
            map_event_data_kind("onPointerDown"),
            Some(EventDataKind::PointerEvent)
        );
    }

    #[test]
    fn test_map_event_data_kind_keyboard() {
        assert_eq!(
            map_event_data_kind("onKeyDown"),
            Some(EventDataKind::KeyEvent)
        );
        assert_eq!(
            map_event_data_kind("onKeyUp"),
            Some(EventDataKind::KeyEvent)
        );
    }

    #[test]
    fn test_map_event_data_kind_scroll() {
        assert_eq!(
            map_event_data_kind("onScroll"),
            Some(EventDataKind::ScrollEvent)
        );
    }

    #[test]
    fn test_map_event_data_kind_none_for_unknown() {
        assert_eq!(map_event_data_kind("onUnknown"), None);
    }

    #[test]
    fn test_schema_pointer_event_has_button_and_modifiers() {
        let fields = event_data_schema(&EventDataKind::PointerEvent);
        let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"x"));
        assert!(names.contains(&"y"));
        assert!(names.contains(&"button"));
        assert!(names.contains(&"alt"));
        assert!(names.contains(&"ctrl"));
    }

    #[test]
    fn test_schema_scroll_event_only_position() {
        let fields = event_data_schema(&EventDataKind::ScrollEvent);
        let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["x", "y"]);
    }

    #[test]
    fn test_schema_twenty_kinds() {
        // All 20 variants produce non-empty field lists
        let kinds = [
            EventDataKind::PointerEvent,
            EventDataKind::TouchEvent,
            EventDataKind::WheelEvent,
            EventDataKind::KeyEvent,
            EventDataKind::ScrollEvent,
            EventDataKind::InputEvent,
            EventDataKind::ToggleEvent,
            EventDataKind::SubmitEvent,
            EventDataKind::FocusEvent,
            EventDataKind::ClipboardEvent,
            EventDataKind::DragEvent,
            EventDataKind::DropEvent,
            EventDataKind::ContextMenuEvent,
            EventDataKind::SelectEvent,
            EventDataKind::ResizeEvent,
            EventDataKind::MediaEvent,
            EventDataKind::MediaLoadEvent,
            EventDataKind::MediaErrorEvent,
            EventDataKind::AnimationEvent,
            EventDataKind::TransitionEvent,
        ];
        assert_eq!(kinds.len(), 20);
        for kind in &kinds {
            assert!(
                !event_data_schema(kind).is_empty(),
                "Schema empty for {:?}",
                kind
            );
        }
    }
}
