//! Shared event-name alias table — re-exported from omnilang-core
//! (`codegen/shared/event_names`), which is the SINGLE source of truth since
//! Task 4.2 (event-name validation): the semantic pass in omnilang-core
//! consumes the same table, so the whitelist can never drift from what the
//! backends resolve (the dblclick-class drift bug must stay impossible).
//!
//! The table itself and its pure-table tests live in omnilang-core; this
//! module keeps the module path `crate::codegen::shared::event::...` working
//! for the per-backend adapters and hosts the cross-backend identity test
//! (which needs the per-backend handler-name functions from this crate).

pub(crate) use omnilang_core::codegen::shared::event_names::canonical_event_name;

#[cfg(test)]
mod tests {
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
    /// (Mirrors the group list in omnilang-core's event_names tests — kept
    /// here because this test cross-checks the per-backend adapters.)
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
                // Option<String> — None-ness is part of the resolution
                // (unmapped canonicals, v4 3.3) and must also be identical
                // across aliases.
                let outputs: std::collections::BTreeSet<Option<String>> =
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
}
