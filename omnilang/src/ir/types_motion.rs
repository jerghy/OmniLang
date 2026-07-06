//! Motion / Animation IR type definitions.
//! Extracted from types.rs to keep each file under 800 lines.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::*;

// ═══════════════════════════════════════════════════════
// Motion / Animation IR
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionIntent {
    pub name: String,
    pub body: MotionBodyIR,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MotionBodyIR {
    Keyframe(KeyframeIR),
    Timeline(TimelineIR),
    CallbackOnly(CallbackBodyIR),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyframeIR {
    pub stops: Vec<KeyframeStopIR>,
    pub over: ExprIR,
    pub ease: Option<EasingIR>,
    pub color_space: Option<String>,
    pub playback: PlaybackIR,
    /// Path for motion path animation
    #[serde(default)]
    pub path: Option<String>,
    /// Whether to rotate element along the path
    #[serde(default)]
    pub path_rotate: Option<bool>,
    /// Callback blocks (onStart, onUpdate, onComplete, onRepeat, onInterrupt)
    #[serde(default)]
    pub callbacks: Vec<(CallbackKindIR, ExprIR)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyframeStopIR {
    pub position: f64,
    pub properties: HashMap<String, ExprIR>,
    pub ease: Option<EasingIR>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineIR {
    pub items: Vec<TimelineItemIR>,
    pub playback: PlaybackIR,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineItemIR {
    pub kind: TimelineItemKindIR,
    pub connector: ConnectorIR,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum TimelineItemKindIR {
    Direction {
        dir: DirectionIR,
        properties: HashMap<String, ExprIR>,
        over: ExprIR,
        ease: Option<EasingIR>,
        delay: Option<ExprIR>,
        color_space: Option<String>,
        start: Option<String>,
        /// Path for motion path animation
        path: Option<String>,
        /// Whether to rotate element along the path
        path_rotate: Option<bool>,
    },
    Set(Vec<(String, ExprIR)>),
    MotionRef(String),
    Stagger(Box<MotionExprIR>, StaggerTimingIR, StaggerDirection),
    Label(String, Option<ExprIR>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DirectionIR {
    To,
    FromTo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectorIR {
    None,
    Then,
    With,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallbackBodyIR {
    pub callbacks: Vec<(CallbackKindIR, ExprIR)>,
    pub over: ExprIR,
    pub playback: PlaybackIR,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CallbackKindIR {
    OnStart,
    OnUpdate,
    OnComplete,
    OnRepeat,
    OnInterrupt,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlaybackIR {
    pub loop_count: Option<LoopValue>,
    pub direction: Option<DirectionValue>,
    pub fill: Option<FillValue>,
    pub delay: Option<ExprIR>,
    pub paused: bool,
    pub repeat_delay: Option<ExprIR>,
    pub repeat_refresh: bool,
    pub time_scale: Option<ExprIR>,
    pub interrupt: Option<InterruptValue>,
    pub respect_reduced_motion: bool,
    pub when: Option<WhenValue>,
    pub delay_children: Option<ExprIR>,
    pub stagger_children: Option<ExprIR>,
    pub stagger_from: Option<StaggerDirection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EasingIR {
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

/// Hero intent for cross-component shared element transitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeroIntent {
    pub name: String,
    pub over: ExprIR,
    pub ease: Option<EasingIR>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerIntent {
    pub kind: TriggerKind,
    pub motion_ref: Option<String>,
    pub inline_motion: Option<MotionBodyIR>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatesIR {
    pub states: HashMap<String, StateTweenIR>,
    pub signal: Option<ExprIR>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTweenIR {
    pub properties: HashMap<String, ExprIR>,
    pub over: Option<ExprIR>,
    pub ease: Option<EasingIR>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum MotionExprIR {
    Ref(String),
    Inline(MotionBodyIR),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineIntent {
    pub name: String,
    pub items: Vec<TimelineItemIR>,
}

/// Value type for loop_count.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoopValue {
    Infinite,
    Count(u32),
}

/// Value type for playback direction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DirectionValue {
    Normal,
    Reverse,
    Alternate,
    AlternateReverse,
}

/// Value type for fill mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FillValue {
    None,
    Forwards,
    Backwards,
    Both,
}

/// Value type for interrupt behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InterruptValue {
    Compose,
    Cancel,
    Finish,
    Replace,
}

/// Value type for when condition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WhenValue {
    Idle,
    Active,
    IdleOrActive,
}

/// Value type for stagger direction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StaggerDirection {
    First,
    Last,
    Center,
    Edges,
    Random,
}

/// Timing parameters for stagger effects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaggerTimingIR {
    pub each: ExprIR,
    pub from: Option<StaggerDirection>,
}

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
