//! Event helpers — event data kinds, field definitions, and schema functions.
//! Extracted from types.rs for better file organization.

use serde::{Deserialize, Serialize};

/// Event payload categories. Each variant has a fixed set of fields
/// defined in `event_data_schema()`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventDataKind {
    PointerEvent,
    TouchEvent,
    WheelEvent,
    KeyEvent,
    ScrollEvent,
    InputEvent,
    ToggleEvent,
    SubmitEvent,
    FocusEvent,
    ClipboardEvent,
    DragEvent,
    DropEvent,
    ContextMenuEvent,
    SelectEvent,
    ResizeEvent,
    MediaEvent,
    MediaLoadEvent,
    MediaErrorEvent,
    AnimationEvent,
    TransitionEvent,
}

/// Typed event field type — replaces the old freeform string.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EventFieldType {
    Float,
    Int,
    Bool,
    String,
    TouchArray,
    StringArray,
    StringMap,
}

impl EventFieldType {
    /// Return the string representation of this field type,
    /// matching the old freeform string format.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Float => "float",
            Self::Int => "int",
            Self::Bool => "bool",
            Self::String => "string",
            Self::TouchArray => "{x,y,id}[]",
            Self::StringArray => "string[]",
            Self::StringMap => "{string: string}",
        }
    }
}

impl std::fmt::Display for EventFieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A single field within an event data payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFieldDef {
    pub name: String,
    pub field_type: EventFieldType,
}

/// Full schema: kind -> ordered field definitions.
/// Used by both lowering (field validation) and codegen (extraction).
#[must_use]
pub fn event_data_schema(kind: &EventDataKind) -> Vec<EventFieldDef> {
    let f = |name: &str, field_type: EventFieldType| EventFieldDef {
        name: name.to_string(),
        field_type,
    };
    match kind {
        EventDataKind::PointerEvent => vec![
            f("x", EventFieldType::Float),
            f("y", EventFieldType::Float),
            f("button", EventFieldType::Int),
            f("pressure", EventFieldType::Float),
            f("tiltX", EventFieldType::Float),
            f("tiltY", EventFieldType::Float),
            f("pointerType", EventFieldType::String),
            f("alt", EventFieldType::Bool),
            f("ctrl", EventFieldType::Bool),
            f("shift", EventFieldType::Bool),
            f("meta", EventFieldType::Bool),
        ],
        EventDataKind::TouchEvent => vec![
            f("touches", EventFieldType::TouchArray),
            f("alt", EventFieldType::Bool),
            f("ctrl", EventFieldType::Bool),
            f("shift", EventFieldType::Bool),
            f("meta", EventFieldType::Bool),
        ],
        EventDataKind::WheelEvent => vec![
            f("x", EventFieldType::Float),
            f("y", EventFieldType::Float),
            f("deltaX", EventFieldType::Float),
            f("deltaY", EventFieldType::Float),
            f("deltaZ", EventFieldType::Float),
            f("deltaMode", EventFieldType::Int),
            f("alt", EventFieldType::Bool),
            f("ctrl", EventFieldType::Bool),
            f("shift", EventFieldType::Bool),
            f("meta", EventFieldType::Bool),
        ],
        EventDataKind::KeyEvent => vec![
            f("key", EventFieldType::String),
            f("code", EventFieldType::String),
            f("alt", EventFieldType::Bool),
            f("ctrl", EventFieldType::Bool),
            f("shift", EventFieldType::Bool),
            f("meta", EventFieldType::Bool),
        ],
        EventDataKind::ScrollEvent => {
            vec![f("x", EventFieldType::Float), f("y", EventFieldType::Float)]
        }
        EventDataKind::InputEvent => vec![
            f("value", EventFieldType::String),
            f("checked", EventFieldType::Bool),
        ],
        EventDataKind::ToggleEvent => vec![f("checked", EventFieldType::Bool)],
        EventDataKind::SubmitEvent => vec![f("values", EventFieldType::StringMap)],
        EventDataKind::FocusEvent => vec![f("value", EventFieldType::String)],
        EventDataKind::ClipboardEvent => vec![f("text", EventFieldType::String)],
        EventDataKind::DragEvent => vec![
            f("x", EventFieldType::Float),
            f("y", EventFieldType::Float),
            f("alt", EventFieldType::Bool),
            f("ctrl", EventFieldType::Bool),
            f("shift", EventFieldType::Bool),
            f("meta", EventFieldType::Bool),
        ],
        EventDataKind::DropEvent => vec![
            f("x", EventFieldType::Float),
            f("y", EventFieldType::Float),
            f("files", EventFieldType::StringArray),
            f("text", EventFieldType::String),
            f("alt", EventFieldType::Bool),
            f("ctrl", EventFieldType::Bool),
            f("shift", EventFieldType::Bool),
            f("meta", EventFieldType::Bool),
        ],
        EventDataKind::ContextMenuEvent => vec![
            f("x", EventFieldType::Float),
            f("y", EventFieldType::Float),
            f("alt", EventFieldType::Bool),
            f("ctrl", EventFieldType::Bool),
            f("shift", EventFieldType::Bool),
            f("meta", EventFieldType::Bool),
        ],
        EventDataKind::SelectEvent => vec![
            f("start", EventFieldType::Int),
            f("end", EventFieldType::Int),
            f("text", EventFieldType::String),
        ],
        EventDataKind::ResizeEvent => vec![
            f("width", EventFieldType::Float),
            f("height", EventFieldType::Float),
        ],
        EventDataKind::MediaEvent => vec![
            f("currentTime", EventFieldType::Float),
            f("duration", EventFieldType::Float),
        ],
        EventDataKind::MediaLoadEvent => {
            vec![
                f("naturalWidth", EventFieldType::Float),
                f("naturalHeight", EventFieldType::Float),
            ]
        }
        EventDataKind::MediaErrorEvent => vec![f("message", EventFieldType::String)],
        EventDataKind::AnimationEvent => {
            vec![
                f("animationName", EventFieldType::String),
                f("elapsedTime", EventFieldType::Float),
            ]
        }
        EventDataKind::TransitionEvent => {
            vec![
                f("propertyName", EventFieldType::String),
                f("elapsedTime", EventFieldType::Float),
            ]
        }
    }
}

/// Map .omni event name string -> EventDataKind.
/// Returns None for store-only or non-data events.
#[must_use]
pub fn map_event_data_kind(event_name: &str) -> Option<EventDataKind> {
    match event_name {
        "onClick" | "onPress" | "onTap" | "onDoubleClick" | "onLongPress" | "onMouseMove"
        | "onHover" | "onMouseEnter" | "onMouseLeave" | "onPointerDown" | "onPointerUp"
        | "onPointerMove" | "onPointerCancel" => Some(EventDataKind::PointerEvent),
        "onTouchStart" | "onTouchMove" | "onTouchEnd" | "onTouchCancel" => {
            Some(EventDataKind::TouchEvent)
        }
        "onWheel" => Some(EventDataKind::WheelEvent),
        "onKeyDown" | "onKeyUp" => Some(EventDataKind::KeyEvent),
        "onScroll" => Some(EventDataKind::ScrollEvent),
        "onChange" | "onInput" => Some(EventDataKind::InputEvent),
        "onToggle" => Some(EventDataKind::ToggleEvent),
        "onSubmit" => Some(EventDataKind::SubmitEvent),
        "onFocus" | "onBlur" => Some(EventDataKind::FocusEvent),
        "onCopy" | "onCut" | "onPaste" => Some(EventDataKind::ClipboardEvent),
        "onDragStart" | "onDrag" | "onDragEnd" => Some(EventDataKind::DragEvent),
        "onDrop" => Some(EventDataKind::DropEvent),
        "onContextMenu" => Some(EventDataKind::ContextMenuEvent),
        "onSelect" => Some(EventDataKind::SelectEvent),
        "onResize" => Some(EventDataKind::ResizeEvent),
        "onPlay" | "onPause" | "onEnded" => Some(EventDataKind::MediaEvent),
        "onLoad" => Some(EventDataKind::MediaLoadEvent),
        "onError" => Some(EventDataKind::MediaErrorEvent),
        "onAnimationStart" | "onAnimationEnd" | "onAnimationIteration" => {
            Some(EventDataKind::AnimationEvent)
        }
        "onTransitionEnd" | "onTransitionRun" | "onTransitionStart" | "onTransitionCancel" => {
            Some(EventDataKind::TransitionEvent)
        }
        _ => None,
    }
}

/// Map EventDataKind to a string label for the @for JS kind-lookup map.
#[must_use]
pub fn event_data_kind_label(kind: &EventDataKind) -> &'static str {
    match kind {
        EventDataKind::PointerEvent => "PointerEvent",
        EventDataKind::TouchEvent => "TouchEvent",
        EventDataKind::WheelEvent => "WheelEvent",
        EventDataKind::KeyEvent => "KeyEvent",
        EventDataKind::ScrollEvent => "ScrollEvent",
        EventDataKind::InputEvent => "InputEvent",
        EventDataKind::ToggleEvent => "ToggleEvent",
        EventDataKind::SubmitEvent => "SubmitEvent",
        EventDataKind::FocusEvent => "FocusEvent",
        EventDataKind::ClipboardEvent => "ClipboardEvent",
        EventDataKind::DragEvent => "DragEvent",
        EventDataKind::DropEvent => "DropEvent",
        EventDataKind::ContextMenuEvent => "ContextMenuEvent",
        EventDataKind::SelectEvent => "SelectEvent",
        EventDataKind::ResizeEvent => "ResizeEvent",
        EventDataKind::MediaEvent => "MediaEvent",
        EventDataKind::MediaLoadEvent => "MediaLoadEvent",
        EventDataKind::MediaErrorEvent => "MediaErrorEvent",
        EventDataKind::AnimationEvent => "AnimationEvent",
        EventDataKind::TransitionEvent => "TransitionEvent",
    }
}
