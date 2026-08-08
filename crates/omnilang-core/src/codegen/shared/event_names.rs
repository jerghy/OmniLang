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
//!
//! This module lives in omnilang-core so the SEMANTIC passes
//! (`semantic/validate_event_names`, Task 4.2) can consume the same table —
//! the `omnilang` crate's `codegen/shared/event.rs` re-exports it for the
//! per-backend adapters (React/OmniView/Flutter/Vue/Dioxus).

/// Resolve a semantic event name (with or without the `on` prefix, any
/// case) to its canonical lowercase DOM event name.
///
/// Alias spellings (click/press/tap, dblclick/doubleclick, ...) collapse to
/// one canonical name here — a backend can never wire different spellings to
/// different handlers. Returns `None` for names outside the table (custom /
/// unknown events), in which case callers apply their own fallback
/// convention.
#[must_use]
pub fn canonical_event_name(event_name: &str) -> Option<&'static str> {
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

/// 首批语义事件（spec 2026-08-08 §3.1）——语义化即切换，旧名由
/// `legacy_spelling` 映射到硬诊断迁移提示。新增语义事件必须在此登记，
/// 且同时更新 Task 9 的 semantic-universe 域 3 登记册（门禁双向断言）。
pub const SEMANTIC_EVENT_NAMES: &[&str] = &[
    "onPress",
    "onInput",
    "onChange",
    "onSubmit",
    "onKeyDown",
    "onKeyUp",
    "onPointerDown",
    "onPointerMove",
    "onPointerUp",
    "onScroll",
    "onFocus",
    "onBlur",
    "onEnter",
    "onLeave",
];

/// 语义事件 → 被取代的旧 DOM 拼写；无旧名（原名即语义名）返回 None。
/// 旧名在语义验证阶段产生硬错误 + 「use @onX instead」迁移提示。
#[must_use]
pub fn legacy_spelling(semantic: &str) -> Option<&'static str> {
    match semantic {
        "onPress" => Some("onClick"),
        "onEnter" => Some("onMouseEnter"),
        "onLeave" => Some("onMouseLeave"),
        _ => None,
    }
}

/// 语义事件分级查询——canonical 表合法但非语义的事件名维持透传。
#[must_use]
pub fn is_semantic_event(name: &str) -> bool {
    SEMANTIC_EVENT_NAMES.contains(&name)
}

/// Representative `onXxx` spellings for every name the shared table accepts
/// — the suggestion candidate set for `nearest_event_suggestion` (Task 4.2).
/// One representative per canonical stem, spelled the way the docs write
/// them (`onMouseOver` not `onMouseover`). The `suggestions_resolve_through_shared_table`
/// test below pins this list to the table: a new table stem without a
/// suggestion here fails the build.
pub const EVENT_NAME_SUGGESTIONS: &[&str] = &[
    "onClick",
    "onPress",
    "onTap",
    "onDoubleClick",
    "onLongPress",
    "onHover",
    "onChange",
    "onInput",
    "onToggle",
    "onSubmit",
    "onFocus",
    "onBlur",
    "onSelect",
    "onResize",
    "onKeyDown",
    "onKeyUp",
    "onKeyPress",
    "onMouseEnter",
    "onMouseLeave",
    "onMouseDown",
    "onMouseUp",
    "onMouseMove",
    "onMouseOver",
    "onMouseOut",
    "onPointerDown",
    "onPointerUp",
    "onPointerMove",
    "onPointerCancel",
    "onScroll",
    "onWheel",
    "onTouchStart",
    "onTouchEnd",
    "onTouchMove",
    "onTouchCancel",
    "onContextMenu",
    "onDrag",
    "onDragStart",
    "onDragEnd",
    "onDragOver",
    "onDragLeave",
    "onDrop",
    "onCopy",
    "onCut",
    "onPaste",
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

/// Nearest legal event suggestion for an unknown event name (Task 4.2).
///
/// The metric rewards BOTH a shared prefix and a small length gap:
/// `score = len(u) + len(k) − 2·lcp(u, k)` over the lowercase `on`-stripped
/// stems. TIES keep the FIRST (earliest) suggestion in `EVENT_NAME_SUGGESTIONS`
/// order — e.g. `onFcous` ties `onCut`/`onFocus`/`onTap` at score 8 and
/// resolves to `onTap` (listed first). Purely Levenshtein distances would
/// rank short-but-unrelated names first for wildly wrong inputs (e.g.
/// `onTotallyBogus` → `onToggle`); the prefix-weighted metric surfaces the
/// readable click-family names instead (`onTap`).
#[must_use]
pub fn nearest_event_suggestion(unknown: &str) -> Option<&'static str> {
    let u = event_stem(unknown);
    let mut best: Option<(&'static str, usize)> = None;
    for known in EVENT_NAME_SUGGESTIONS {
        let k = event_stem(known);
        let lcp = common_prefix_len(&u, &k);
        let score = u.len() + k.len() - 2 * lcp;
        if best.is_none_or(|(_, s)| score < s) {
            best = Some((known, score));
        }
    }
    best.map(|(known, _)| known)
}

/// Lowercased comparison stem — strip a leading `on` prefix (any case),
/// lowercase the rest. `onTotallyBogus` → `totallybogus`.
fn event_stem(name: &str) -> String {
    name.strip_prefix("on")
        .or_else(|| name.strip_prefix("On"))
        .or_else(|| name.strip_prefix("ON"))
        .unwrap_or(name)
        .to_lowercase()
}

/// Longest common prefix length of two strings (case already folded).
fn common_prefix_len(a: &str, b: &str) -> usize {
    a.as_bytes()
        .iter()
        .zip(b.as_bytes())
        .take_while(|(x, y)| x == y)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// Every event name the IR knows (mirrors `map_event_data_kind`,
    /// events.rs) — shared by the table-coverage and suggestion-coverage
    /// tests below. If the IR grows a new event name, both tests force the
    /// shared table AND the suggestion list to learn it.
    const IR_EVENT_NAMES: &[&str] = &[
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
            let canonicals: BTreeSet<&str> = group
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

    /// Completeness: every event name the IR knows must resolve through the
    /// shared table — never fall through to a per-backend fallback (that is
    /// where alias drift was born). If the IR grows a new event name, this
    /// test forces the shared table to learn it.
    #[test]
    fn every_ir_event_name_resolves_through_shared_table() {
        for name in IR_EVENT_NAMES {
            assert!(
                canonical_event_name(name).is_some(),
                "IR event '{name}' must resolve through the shared table (no backend fallback)"
            );
        }
    }

    /// Every IR event name must ALSO have a suggestion representative
    /// (Task 4.2 review M-2) — a typo'd spelling of ANY legal event must be
    /// able to land on a suggestion for it (canonical-resolving, so the
    /// suggestion is itself legal). `suggestions_resolve_through_shared_table`
    /// above pins the list to the table; this pins the list to the IR.
    #[test]
    fn every_ir_event_has_suggestion_representative() {
        for name in IR_EVENT_NAMES {
            let canonical = canonical_event_name(name).expect("IR event is legal");
            assert!(
                EVENT_NAME_SUGGESTIONS
                    .iter()
                    .any(|s| { canonical_event_name(s) == Some(canonical) }),
                "IR event '{name}' (canonical '{canonical}') has no suggestion representative"
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

    /// Every suggestion candidate must itself be a legal event name —
    /// otherwise the "nearest legal suggestion" could recommend something
    /// the whitelist would reject. A new table stem without a suggestion
    /// entry fails here, forcing EVENT_NAME_SUGGESTIONS to stay complete.
    #[test]
    fn suggestions_resolve_through_shared_table() {
        for spelling in EVENT_NAME_SUGGESTIONS {
            assert!(
                canonical_event_name(spelling).is_some(),
                "suggestion '{spelling}' must be a legal event name"
            );
        }
        // The suggestion set must cover every canonical stem (not just a
        // subset) — spot-check the families that matter for suggestions.
        assert!(EVENT_NAME_SUGGESTIONS.contains(&"onClick"));
        assert!(EVENT_NAME_SUGGESTIONS.contains(&"onPress"));
        assert!(EVENT_NAME_SUGGESTIONS.contains(&"onTap"));
        assert!(EVENT_NAME_SUGGESTIONS.contains(&"onDoubleClick"));
        assert!(EVENT_NAME_SUGGESTIONS.contains(&"onToggle"));
    }

    /// Task 4.2 — the brief's exact case: `onTotallyBogus` must suggest a
    /// click-family spelling (prefix-weighted distance, not raw Levenshtein
    /// which would surface `onToggle`).
    #[test]
    fn totally_bogus_suggests_click_family() {
        let s = nearest_event_suggestion("onTotallyBogus").expect("must suggest");
        assert!(
            s == "onTap" || s == "onClick" || s == "onPress",
            "onTotallyBogus should suggest a click-family event, got {s}"
        );
    }

    /// Real-world typos land on the right event.
    #[test]
    fn typos_suggest_the_intended_event() {
        assert_eq!(nearest_event_suggestion("onClik"), Some("onClick"));
        assert_eq!(nearest_event_suggestion("onPresss"), Some("onPress"));
        assert_eq!(nearest_event_suggestion("onSubmti"), Some("onSubmit"));
        assert_eq!(
            nearest_event_suggestion("onDoubleclik"),
            Some("onDoubleClick")
        );
        assert_eq!(nearest_event_suggestion("onKeydwon"), Some("onKeyDown"));
        // Bare stems (no `on` prefix) also resolve.
        assert_eq!(nearest_event_suggestion("clik"), Some("onClick"));
        // Case variants are folded.
        assert_eq!(nearest_event_suggestion("ONMOUSEDOWN"), Some("onMouseDown"));
    }

    /// 语义事件全集——Task 2 硬诊断与 Task 9 登记册锚点。
    /// press/input/change/submit/key(down/up)/pointer(down/move/up)/scroll/focus/blur/enter/leave
    const SEMANTIC_NAMES: &[&str] = &[
        "onPress",
        "onInput",
        "onChange",
        "onSubmit",
        "onKeyDown",
        "onKeyUp",
        "onPointerDown",
        "onPointerMove",
        "onPointerUp",
        "onScroll",
        "onFocus",
        "onBlur",
        "onEnter",
        "onLeave",
    ];

    #[test]
    fn semantic_names_match_design_contract() {
        for n in SEMANTIC_NAMES {
            assert!(SEMANTIC_EVENT_NAMES.contains(n), "{n} 必须是语义事件");
        }
        assert_eq!(SEMANTIC_EVENT_NAMES.len(), 14, "首批 14 个事件名/9 语义族");
    }

    #[test]
    fn legacy_spelling_maps_old_names() {
        assert_eq!(legacy_spelling("onPress"), Some("onClick"));
        assert_eq!(legacy_spelling("onEnter"), Some("onMouseEnter"));
        assert_eq!(legacy_spelling("onLeave"), Some("onMouseLeave"));
        assert_eq!(legacy_spelling("onInput"), None, "onInput 无旧名");
        assert_eq!(legacy_spelling("onKeyDown"), None);
    }
}
