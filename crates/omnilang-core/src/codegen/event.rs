//! Shared event data utilities for extracting fields from DOM events.
//!
//! Provides JS-based extraction expressions for various event types
//! (Pointer, Touch, Wheel, Key, Scroll, Input, etc.) used by multiple
//! codegen backends (OmniView, Vue, JavaScript).

#[cfg(any(feature = "codegen-omniview", feature = "codegen-vue"))]
use crate::ir::event_data_schema;
#[cfg(any(feature = "codegen-omniview", feature = "codegen-vue"))]
use crate::ir::EventDataKind;

/// Emit JS extraction code for a single event data field.
/// Returns (extraction_expression, fallback_value_literal).
/// `pub` so other codegen targets (Vue, JS) can reuse the DOM-based mapping.
#[cfg(any(feature = "codegen-omniview", feature = "codegen-vue"))]
#[must_use]
pub fn field_extract_js(kind: &EventDataKind, field: &str, raw: &str) -> (String, String) {
    match (kind, field) {
        // PointerEvent — x/y are ELEMENT-LOCAL coordinates (offsetX/offsetY,
        // D1 coordinate-basis change), not viewport (clientX/clientY).
        // clientX/clientY：视口坐标——offsetX 在部分触摸实现下不可靠
        // （undefined/0），需要元素局部位置时用 clientX - measure().x。
        (EventDataKind::PointerEvent, "x") => (format!("{}.offsetX", raw), "0".into()),
        (EventDataKind::PointerEvent, "y") => (format!("{}.offsetY", raw), "0".into()),
        (EventDataKind::PointerEvent, "clientX") => (format!("{}.clientX", raw), "0".into()),
        (EventDataKind::PointerEvent, "clientY") => (format!("{}.clientY", raw), "0".into()),
        (EventDataKind::PointerEvent, "button") => (format!("{}.button", raw), "0".into()),
        (EventDataKind::PointerEvent, "pressure") => (format!("{}.pressure", raw), "0".into()),
        (EventDataKind::PointerEvent, "tiltX") => (format!("{}.tiltX", raw), "0".into()),
        (EventDataKind::PointerEvent, "tiltY") => (format!("{}.tiltY", raw), "0".into()),
        (EventDataKind::PointerEvent, "pointerType") => (format!("{}.pointerType", raw), "\"\"".into()),
        (EventDataKind::PointerEvent, "pointerId") => (format!("{}.pointerId", raw), "0".into()),
        (EventDataKind::PointerEvent, "buttons") => (format!("{}.buttons", raw), "0".into()),
        (EventDataKind::PointerEvent, m @ ("alt"|"ctrl"|"shift"|"meta")) => {
            let key = match m { "alt" => "altKey", "ctrl" => "ctrlKey", "shift" => "shiftKey", _ => "metaKey" };
            (format!("{}.{}", raw, key), "false".into())
        }

        // TouchEvent
        (EventDataKind::TouchEvent, "touches") => (
            format!("__mergeTouches({})", raw),
            "[]".into()
        ),
        (EventDataKind::TouchEvent, m @ ("alt"|"ctrl"|"shift"|"meta")) => {
            let key = match m { "alt" => "altKey", "ctrl" => "ctrlKey", "shift" => "shiftKey", _ => "metaKey" };
            (format!("{}.{}", raw, key), "false".into())
        }

        // WheelEvent
        (EventDataKind::WheelEvent, "x") => (format!("{}.clientX", raw), "0".into()),
        (EventDataKind::WheelEvent, "y") => (format!("{}.clientY", raw), "0".into()),
        (EventDataKind::WheelEvent, "deltaX") => (format!("{}.deltaX", raw), "0".into()),
        (EventDataKind::WheelEvent, "deltaY") => (format!("{}.deltaY", raw), "0".into()),
        (EventDataKind::WheelEvent, "deltaZ") => (format!("{}.deltaZ", raw), "0".into()),
        (EventDataKind::WheelEvent, "deltaMode") => (format!("{}.deltaMode", raw), "0".into()),
        (EventDataKind::WheelEvent, m @ ("alt"|"ctrl"|"shift"|"meta")) => {
            let key = match m { "alt" => "altKey", "ctrl" => "ctrlKey", "shift" => "shiftKey", _ => "metaKey" };
            (format!("{}.{}", raw, key), "false".into())
        }

        // KeyEvent
        (EventDataKind::KeyEvent, "key") => (format!("{}.key", raw), "\"\"".into()),
        (EventDataKind::KeyEvent, "code") => (format!("{}.code", raw), "\"\"".into()),
        (EventDataKind::KeyEvent, m @ ("alt"|"ctrl"|"shift"|"meta")) => {
            let key = match m { "alt" => "altKey", "ctrl" => "ctrlKey", "shift" => "shiftKey", _ => "metaKey" };
            (format!("{}.{}", raw, key), "false".into())
        }

        // ScrollEvent
        (EventDataKind::ScrollEvent, "x") => (format!("{}.target?.scrollLeft", raw), "0".into()),
        (EventDataKind::ScrollEvent, "y") => (format!("{}.target?.scrollTop", raw), "0".into()),
        // progress = 滚动进度 0..1（scrollTop/(scrollHeight-clientHeight)，
        // 无溢出/目标缺失时 0 兜底——NaN 比较为 false 自然落入 : 0 臂）。
        (EventDataKind::ScrollEvent, "progress") => {
            let t = format!("{}.target", raw);
            (
                format!(
                    "({t}?.scrollHeight - {t}?.clientHeight > 0) ? {t}?.scrollTop / ({t}?.scrollHeight - {t}?.clientHeight) : 0"
                ),
                "0".into(),
            )
        }

        // InputEvent
        (EventDataKind::InputEvent, "value") => (format!("{}.target?.value", raw), "\"\"".into()),
        (EventDataKind::InputEvent, "checked") => (format!("{}.target?.checked", raw), "false".into()),

        // ToggleEvent
        (EventDataKind::ToggleEvent, "checked") => (format!("{}.target?.checked", raw), "false".into()),

        // SubmitEvent — Web
        (EventDataKind::SubmitEvent, "values") => (
            format!("Object.fromEntries(new FormData({}.target))", raw),
            "{}".into()
        ),

        // FocusEvent
        (EventDataKind::FocusEvent, "value") => (format!("{}.target?.value", raw), "\"\"".into()),

        // ClipboardEvent
        (EventDataKind::ClipboardEvent, "text") => (
            format!("(function(){{try{{return {}.clipboardData?.getData('text')||''}}catch(_){{return''}}}})()", raw),
            "\"\"".into()
        ),

        // DragEvent
        (EventDataKind::DragEvent, "x") => (format!("{}.clientX", raw), "0".into()),
        (EventDataKind::DragEvent, "y") => (format!("{}.clientY", raw), "0".into()),
        (EventDataKind::DragEvent, m @ ("alt"|"ctrl"|"shift"|"meta")) => {
            let key = match m { "alt" => "altKey", "ctrl" => "ctrlKey", "shift" => "shiftKey", _ => "metaKey" };
            (format!("{}.{}", raw, key), "false".into())
        }

        // DropEvent
        (EventDataKind::DropEvent, "x") => (format!("{}.clientX", raw), "0".into()),
        (EventDataKind::DropEvent, "y") => (format!("{}.clientY", raw), "0".into()),
        (EventDataKind::DropEvent, "files") => (
            format!("[...({}.dataTransfer?.files ?? [])].map(f => f.name)", raw),
            "[]".into()
        ),
        (EventDataKind::DropEvent, "text") => (
            format!("(function(){{try{{return {}.dataTransfer?.getData('text')||''}}catch(_){{return''}}}})()", raw),
            "\"\"".into()
        ),
        (EventDataKind::DropEvent, m @ ("alt"|"ctrl"|"shift"|"meta")) => {
            let key = match m { "alt" => "altKey", "ctrl" => "ctrlKey", "shift" => "shiftKey", _ => "metaKey" };
            (format!("{}.{}", raw, key), "false".into())
        }

        // ContextMenuEvent
        (EventDataKind::ContextMenuEvent, "x") => (format!("{}.clientX", raw), "0".into()),
        (EventDataKind::ContextMenuEvent, "y") => (format!("{}.clientY", raw), "0".into()),
        (EventDataKind::ContextMenuEvent, m @ ("alt"|"ctrl"|"shift"|"meta")) => {
            let key = match m { "alt" => "altKey", "ctrl" => "ctrlKey", "shift" => "shiftKey", _ => "metaKey" };
            (format!("{}.{}", raw, key), "false".into())
        }

        // SelectEvent
        (EventDataKind::SelectEvent, "start") => (format!("{}.target?.selectionStart", raw), "0".into()),
        (EventDataKind::SelectEvent, "end") => (format!("{}.target?.selectionEnd", raw), "0".into()),
        (EventDataKind::SelectEvent, "text") => (
            format!("({}.target?.value ?? '').substring({}.target?.selectionStart ?? 0, {}.target?.selectionEnd ?? 0)", raw, raw, raw),
            "\"\"".into()
        ),

        // ResizeEvent
        (EventDataKind::ResizeEvent, "width") => ("window.innerWidth".into(), "0".into()),
        (EventDataKind::ResizeEvent, "height") => ("window.innerHeight".into(), "0".into()),

        // MediaEvent
        (EventDataKind::MediaEvent, "currentTime") => (format!("{}.target?.currentTime", raw), "0".into()),
        (EventDataKind::MediaEvent, "duration") => (format!("{}.target?.duration", raw), "0".into()),

        // MediaLoadEvent
        (EventDataKind::MediaLoadEvent, "naturalWidth") => (format!("{}.target?.naturalWidth", raw), "0".into()),
        (EventDataKind::MediaLoadEvent, "naturalHeight") => (format!("{}.target?.naturalHeight", raw), "0".into()),

        // MediaErrorEvent
        (EventDataKind::MediaErrorEvent, "message") => (format!("{}.target?.error?.message", raw), "\"unknown\"".into()),

        // AnimationEvent
        (EventDataKind::AnimationEvent, "animationName") => (format!("{}.animationName", raw), "\"\"".into()),
        (EventDataKind::AnimationEvent, "elapsedTime") => (format!("{}.elapsedTime", raw), "0".into()),

        // TransitionEvent
        (EventDataKind::TransitionEvent, "propertyName") => (format!("{}.propertyName", raw), "\"\"".into()),
        (EventDataKind::TransitionEvent, "elapsedTime") => (format!("{}.elapsedTime", raw), "0".into()),

        _ => (format!("undefined /* unknown field {} for {:?} */", field, kind), "undefined".into()),
    }
}

/// Emit a JS object literal for event data extraction from a raw event variable.
/// Only emits fields in `used_fields` (or all fields from schema if None).
/// `pub` so other codegen targets (Vue, JS) can reuse the DOM-based mapping.
#[cfg(any(feature = "codegen-omniview", feature = "codegen-vue"))]
#[must_use]
pub fn emit_event_object(
    event_data: &Option<EventDataKind>,
    used_fields: &Option<Vec<String>>,
    raw_var: &str,
) -> String {
    let kind = match event_data {
        Some(k) => k,
        // Custom events (not in the known DOM event list) store their
        // payload in CustomEvent.detail. Pass it directly so the handler
        // receives the value (e.g. onAdd(text) → text = "value").
        None => return format!("{}.detail", raw_var),
    };

    let schema = event_data_schema(kind);
    let fields: Vec<String> = match used_fields {
        Some(fields) if !fields.is_empty() => {
            // Only emit used fields, in schema order
            schema
                .iter()
                .filter(|f| fields.contains(&f.name))
                .map(|f| {
                    let (extract, fallback) = field_extract_js(kind, &f.name, raw_var);
                    format!("{}: {} ?? {}", f.name, extract, fallback)
                })
                .collect()
        }
        Some(_) => {
            // Empty vec — no fields used
            return "{}".to_string();
        }
        None => {
            // Conservative — emit all fields
            schema
                .iter()
                .map(|f| {
                    let (extract, fallback) = field_extract_js(kind, &f.name, raw_var);
                    format!("{}: {} ?? {}", f.name, extract, fallback)
                })
                .collect()
        }
    };

    format!("{{ {} }}", fields.join(", "))
}

#[cfg(all(test, any(feature = "codegen-omniview", feature = "codegen-vue")))]
mod tests {
    use super::*;

    /// D1: PointerEvent x/y switch from viewport (clientX/clientY) to
    /// element-local coordinates (offsetX/offsetY); pointerId/buttons added.
    #[test]
    fn pointer_field_extract_js_element_local_coords() {
        let cases = [
            ("x", "e.offsetX"),
            ("y", "e.offsetY"),
            ("pointerId", "e.pointerId"),
            ("buttons", "e.buttons"),
        ];
        for (field, expected) in cases {
            let (extract, fallback) = field_extract_js(&EventDataKind::PointerEvent, field, "e");
            assert_eq!(extract, expected, "field {field}");
            assert_eq!(fallback, "0", "field {field} fallback");
        }
    }

    /// The coordinate-basis change is scoped to PointerEvent — other
    /// kinds keep viewport coordinates.
    #[test]
    fn non_pointer_events_keep_viewport_coords() {
        let (x, _) = field_extract_js(&EventDataKind::WheelEvent, "x", "e");
        assert_eq!(x, "e.clientX");
    }

    /// Task 6: ScrollEvent progress = scrollTop / (scrollHeight - clientHeight),
    /// 0 when not measurable (no scrollable overflow / missing target).
    #[test]
    fn scroll_field_extract_js_progress() {
        let (extract, fallback) = field_extract_js(&EventDataKind::ScrollEvent, "progress", "e");
        assert_eq!(
            extract,
            "(e.target?.scrollHeight - e.target?.clientHeight > 0) ? e.target?.scrollTop / (e.target?.scrollHeight - e.target?.clientHeight) : 0"
        );
        assert_eq!(fallback, "0");
    }
}
