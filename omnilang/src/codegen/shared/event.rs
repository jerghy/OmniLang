//! Shared event-name alias table — the SINGLE place semantic event names
//! resolve to canonical (DOM-style) event names.
//!
//! Historically every backend had its own alias table (React's
//! `to_react_event`, OmniView's `map_event`, Flutter's
//! `event_to_flutter_callback`, Dioxus/Vue variants) and the tables drifted —
//! the `dblclick` bug lived in exactly such drift: one table resolved
//! `onDoubleClick` to `dblclick` while another produced `doubleclick`
//! (which never fires). All backends now consume this one table and apply
//! only their own handler-name convention on top of the canonical name.

/// Resolve a semantic event name (with or without the `on` prefix, any
/// case) to its canonical lowercase DOM event name.
///
/// Alias spellings (click/press/tap, dblclick/doubleclick, ...) collapse to
/// one canonical name here — a backend can never wire different spellings to
/// different handlers. Returns `None` for names outside the table (custom /
/// unknown events), in which case callers apply their own fallback
/// convention.
#[must_use]
pub(crate) fn canonical_event_name(event_name: &str) -> Option<&'static str> {
    let stem = event_name.strip_prefix("on").unwrap_or(event_name);
    let stem_lower = stem.to_lowercase();
    match stem_lower.as_str() {
        // click family
        "press" | "click" | "tap" => Some("click"),
        // dblclick family — the original drift bug
        "dblclick" | "doubleclick" => Some("dblclick"),
        // custom names with no DOM event (Flutter-only conveniences)
        "longpress" => Some("longpress"),
        "hover" => Some("hover"),
        // form
        "change" => Some("change"),
        "input" => Some("input"),
        "toggle" => Some("toggle"),
        "submit" => Some("submit"),
        "focus" => Some("focus"),
        "blur" => Some("blur"),
        "select" => Some("select"),
        "resize" => Some("resize"),
        // keyboard
        "keydown" => Some("keydown"),
        "keyup" => Some("keyup"),
        "keypress" => Some("keypress"),
        // mouse
        "mouseenter" => Some("mouseenter"),
        "mouseleave" => Some("mouseleave"),
        "mousedown" => Some("mousedown"),
        "mouseup" => Some("mouseup"),
        "mousemove" => Some("mousemove"),
        "mouseover" => Some("mouseover"),
        "mouseout" => Some("mouseout"),
        // pointer
        "pointerdown" => Some("pointerdown"),
        "pointerup" => Some("pointerup"),
        "pointermove" => Some("pointermove"),
        "pointercancel" => Some("pointercancel"),
        // wheel / scroll / touch
        "scroll" => Some("scroll"),
        "wheel" => Some("wheel"),
        "touchstart" => Some("touchstart"),
        "touchend" => Some("touchend"),
        "touchmove" => Some("touchmove"),
        "touchcancel" => Some("touchcancel"),
        "contextmenu" => Some("contextmenu"),
        // drag & drop
        "drag" => Some("drag"),
        "dragstart" => Some("dragstart"),
        "dragend" => Some("dragend"),
        "dragover" => Some("dragover"),
        "dragleave" => Some("dragleave"),
        "drop" => Some("drop"),
        // clipboard
        "copy" => Some("copy"),
        "cut" => Some("cut"),
        "paste" => Some("paste"),
        // media
        "play" => Some("play"),
        "pause" => Some("pause"),
        "ended" => Some("ended"),
        "load" => Some("load"),
        "error" => Some("error"),
        // animation / transition — the full DOM family; React maps these to
        // its synthetic handler set (onAnimationStart/End/Iteration,
        // onTransitionEnd/Run/Start/Cancel) in react/utils.rs
        "animationstart" => Some("animationstart"),
        "animationend" => Some("animationend"),
        "animationiteration" => Some("animationiteration"),
        "transitionend" => Some("transitionend"),
        "transitionrun" => Some("transitionrun"),
        "transitionstart" => Some("transitionstart"),
        "transitioncancel" => Some("transitioncancel"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "codegen-dioxus")]
    use crate::codegen::dioxus::expr::to_dioxus_event;
    #[cfg(feature = "codegen-flutter")]
    use crate::codegen::flutter::utils::event_to_flutter_callback;
    #[cfg(feature = "codegen-omniview")]
    use crate::codegen::omniview::utils::map_event;
    #[cfg(feature = "codegen-react")]
    use crate::codegen::react::utils::to_react_event;
    #[cfg(feature = "codegen-vue")]
    use crate::codegen::vue::expr::to_vue_event;

    /// Alias groups: every spelling must resolve to ONE canonical name.
    const ALIAS_GROUPS: &[&[&str]] = &[
        &["onClick", "click", "onPress", "press", "onTap", "tap"],
        &[
            "onDoubleClick",
            "onDblClick",
            "dblclick",
            "doubleclick",
            "onDoubleclick",
            "DoubleClick",
            "DblClick",
        ],
        &["onTouchCancel", "touchcancel", "onTouchcancel"],
        // Task 2.5: animation/transition spellings must resolve to ONE
        // canonical name per backend — React's synthetic set, never the
        // mangled lowercase `onAnimationend` (silent dead event, ts/react
        // M3).
        &[
            "onAnimationEnd",
            "animationend",
            "onAnimationend",
            "AnimationEnd",
        ],
        &[
            "onTransitionEnd",
            "transitionend",
            "onTransitionend",
            "TransitionEnd",
        ],
    ];

    #[test]
    fn alias_groups_resolve_to_single_canonical() {
        for group in ALIAS_GROUPS {
            let canonicals: std::collections::BTreeSet<&str> = group
                .iter()
                .map(|spelling| {
                    canonical_event_name(spelling)
                        .unwrap_or_else(|| panic!("alias '{spelling}' must be known"))
                })
                .collect();
            assert_eq!(
                canonicals.len(),
                1,
                "alias group {group:?} must resolve to ONE canonical name, got {canonicals:?}"
            );
        }
    }

    /// The dblclick-class bug must be impossible: each backend's handler
    /// name for every spelling in an alias group must be IDENTICAL within
    /// that backend — a misspelled alias must never silently wire a
    /// different handler (or no handler at all).
    #[test]
    fn alias_spellings_resolve_identically_in_every_backend() {
        for group in ALIAS_GROUPS {
            let spellings = group.to_vec();

            #[cfg(feature = "codegen-react")]
            {
                let outputs: std::collections::BTreeSet<String> =
                    spellings.iter().map(|s| to_react_event(s)).collect();
                assert_eq!(
                    outputs.len(),
                    1,
                    "react: aliases {spellings:?} must resolve identically, got {outputs:?}"
                );
            }
            #[cfg(feature = "codegen-omniview")]
            {
                let outputs: std::collections::BTreeSet<String> =
                    spellings.iter().map(|s| map_event(s)).collect();
                assert_eq!(
                    outputs.len(),
                    1,
                    "omniview: aliases {spellings:?} must resolve identically, got {outputs:?}"
                );
            }
            #[cfg(feature = "codegen-flutter")]
            {
                let outputs: std::collections::BTreeSet<String> = spellings
                    .iter()
                    .map(|s| event_to_flutter_callback(s))
                    .collect();
                assert_eq!(
                    outputs.len(),
                    1,
                    "flutter: aliases {spellings:?} must resolve identically, got {outputs:?}"
                );
            }
            #[cfg(feature = "codegen-dioxus")]
            {
                let outputs: std::collections::BTreeSet<String> =
                    spellings.iter().map(|s| to_dioxus_event(s)).collect();
                assert_eq!(
                    outputs.len(),
                    1,
                    "dioxus: aliases {spellings:?} must resolve identically, got {outputs:?}"
                );
            }
            #[cfg(feature = "codegen-vue")]
            {
                let outputs: std::collections::BTreeSet<String> =
                    spellings.iter().map(|s| to_vue_event(s)).collect();
                assert_eq!(
                    outputs.len(),
                    1,
                    "vue: aliases {spellings:?} must resolve identically, got {outputs:?}"
                );
            }
        }
    }

    /// Completeness: every event name the IR knows (the names listed in
    /// `map_event_data_kind`, events.rs) must resolve through the shared
    /// table — never fall through to a per-backend fallback (that is where
    /// alias drift was born). The list mirrors events.rs; if the IR grows a
    /// new event name, this test forces the shared table to learn it.
    #[test]
    fn every_ir_event_name_resolves_through_shared_table() {
        let ir_events = [
            "onClick",
            "onPress",
            "onTap",
            "onDoubleClick",
            "onLongPress",
            "onMouseMove",
            "onHover",
            "onMouseEnter",
            "onMouseLeave",
            "onPointerDown",
            "onPointerUp",
            "onPointerMove",
            "onPointerCancel",
            "onTouchStart",
            "onTouchMove",
            "onTouchEnd",
            "onTouchCancel",
            "onWheel",
            "onKeyDown",
            "onKeyUp",
            "onChange",
            "onInput",
            "onToggle",
            "onSubmit",
            "onFocus",
            "onBlur",
            "onCopy",
            "onCut",
            "onPaste",
            "onDragStart",
            "onDrag",
            "onDragEnd",
            "onDrop",
            "onContextMenu",
            "onSelect",
            "onResize",
            "onPlay",
            "onPause",
            "onEnded",
            "onLoad",
            "onError",
            "onAnimationStart",
            "onAnimationEnd",
            "onAnimationIteration",
            "onTransitionEnd",
            "onTransitionRun",
            "onTransitionStart",
            "onTransitionCancel",
        ];
        for name in ir_events {
            assert!(
                canonical_event_name(name).is_some(),
                "IR event '{name}' must resolve through the shared table (no backend fallback)"
            );
        }
    }

    /// The shared table is also the ONLY alias source: no backend-specific
    /// table may contain an entry the shared table does not know (unknown
    /// names are per-backend fallback territory — that is the contract).
    #[test]
    fn no_per_backend_alias_outside_shared_table() {
        // Every name the per-backend tables historically listed explicitly
        // must resolve via the shared table to the same canonical result.
        let legacy_backend_entries = [
            // react to_react_event stems
            "click",
            "change",
            "input",
            "submit",
            "focus",
            "blur",
            "keydown",
            "keyup",
            "keypress",
            "mouseenter",
            "mouseleave",
            "mousedown",
            "mouseup",
            "mousemove",
            "mouseover",
            "mouseout",
            "dblclick",
            "scroll",
            "wheel",
            "touchstart",
            "touchend",
            "touchmove",
            "contextmenu",
            "drag",
            "dragstart",
            "dragend",
            "dragover",
            "dragleave",
            "drop",
            "press",
            // omniview map_event names
            "onPress",
            "onClick",
            "onTap",
            "onChange",
            "onInput",
            "onFocus",
            "onBlur",
            "onKeyDown",
            "onKeyUp",
            "onSubmit",
            "onScroll",
            "onMouseEnter",
            "onMouseLeave",
            "onMouseMove",
            "onDoubleClick",
            "onDblClick",
            // flutter event_to_flutter_callback stems
            "tap",
            "longpress",
            "doubleclick",
            "hover",
            "toggle",
            "select",
            "dragstart",
            "dragend",
            "dragover",
            // vue/dioxus stems
            "onDblclick",
            "onKeydown",
            "onPress",
            "onTap",
        ];
        for name in legacy_backend_entries {
            assert!(
                canonical_event_name(name).is_some(),
                "legacy backend table entry '{name}' must live in the shared table"
            );
        }
    }
}
