#![cfg(feature = "codegen-omniview")]
//! Integration tests for design block CSS codegen.
//!
//! Tests `emit_design_css` by constructing `DesignIR` directly
//! (post-lowering) and verifying CSS output.
//!
//! Duplicate tests removed — unique scenarios only.
//! See `omnilang/src/codegen/omniview/design.rs` for the unit test suite.

use omnilang::codegen::omniview::design::*;
use omnilang::ir::design::*;
use rustc_hash::FxHashMap;

#[test]
fn emit_class_with_multiple_pseudo_states() {
    let classes = vec![ClassIR {
        name: "Input".to_string(),
        properties: vec![
            ("border".to_string(), "1px solid #ccc".to_string()),
            ("padding".to_string(), "8px".to_string()),
        ]
        .into_iter()
        .collect(),
        pseudo_states: vec![
            PseudoStateIR {
                pseudo: "hover".to_string(),
                properties: vec![("border-color".to_string(), "#999".to_string())]
                    .into_iter()
                    .collect(),
                source_span: None,
            },
            PseudoStateIR {
                pseudo: "focus".to_string(),
                properties: vec![
                    ("border-color".to_string(), "#6366f1".to_string()),
                    ("outline".to_string(), "none".to_string()),
                ]
                .into_iter()
                .collect(),
                source_span: None,
            },
        ],
        source_span: None,
    }];

    let design = DesignIR {
        name: "FormDesign".to_string(),
        themes: vec![],
        default_theme: None,
        tokens: FxHashMap::default(),
        classes,
        variants: vec![],
        source_span: None,
    };

    let css = emit_design_css(&design, "FormDesign").expect("should emit CSS");

    assert!(css.contains("[data-design~=\"FormDesign\"] .Input {"));
    assert!(css.contains("[data-design~=\"FormDesign\"] .Input:hover {"));
    assert!(css.contains("border-color: #999;"));
    assert!(css.contains("[data-design~=\"FormDesign\"] .Input:focus {"));
    assert!(css.contains("border-color: #6366f1;"));
}
