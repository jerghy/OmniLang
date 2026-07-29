//! Regression tests for OmniView codegen modules.
//! Tests focus on motion/animation codegen output correctness.

mod list;
mod motion;
mod sanitizers;
// mod timeline;  -- removed: Timeline IR types no longer exist

/// Regression test: prop defaults must be emitted BEFORE computed declarations.
/// If a computed references a prop with a default value, the prop's readonlySignal
/// variable must exist before the computed eagerly evaluates (__omni_computed at
/// runtime.rs:85 calls run() → fn() immediately), otherwise ReferenceError at runtime.
#[test]
fn computed_references_prop_with_default() {
    use crate::codegen::omniview::generate;
    use crate::ir::*;
    use rustc_hash::FxHashMap;
    use serde_json::json;

    // ── Build IR directly (avoids type-checker prop resolution bug) ──
    let comp = ComponentIR {
        name: "Counter".into(),
        source_file: None,
        generic_params: vec![],
        props: FxHashMap::from_iter([(
            "initial".into(),
            TypeIR {
                kind: "i32".into(),
                type_args: vec![],
                is_nullable: false,
            },
        )]),
        prop_defaults: FxHashMap::from_iter([(
            "initial".into(),
            ExprIR::LiteralExpr { value: json!(0) },
        )]),
        signals: vec![],
        computeds: vec![ComputedIntent {
            name: "double".into(),
            r#type: TypeIR {
                kind: "i32".into(),
                type_args: vec![],
                is_nullable: false,
            },
            body: ExprIR::BinaryOpExpr {
                op: BinaryOpIR::Mul,
                left: Box::new(ExprIR::IdentifierExpr {
                    name: "initial".into(),
                    type_ann: None,
                }),
                right: Box::new(ExprIR::LiteralExpr { value: json!(2) }),
            },
            deps: vec!["initial".into()],
        }],
        effects: vec![],
        lifecycles: vec![],
        fns: vec![],
        store_refs: vec![],
        refs: vec![],
        event_names: vec![],
        event_param_types: vec![],
        view: None,
        styles: vec![],
        if_mode_default: None,
        source_span: None,
    };
    let mut module = ModuleIR {
        name: "main".into(),
        functions: FxHashMap::default(),
        structs: FxHashMap::default(),
        enums: FxHashMap::default(),
        type_aliases: FxHashMap::default(),
        components: FxHashMap::default(),
        stores: FxHashMap::default(),
        motions: vec![],
        imports: vec![],
        exports: vec![],
        designs: vec![],
        source_span: None,
    };
    module.components.insert("Counter".to_string(), comp);
    let ir = IRProgram {
        version: "0.1".into(),
        modules: FxHashMap::from_iter([("main".into(), module)]),
        source_map: FxHashMap::default(),
        stdlib_imports: Default::default(),
        source_span: None,
    };
    let output = generate(&ir, false, false, false);

    // The prop must be emitted as __omni_readonlySignal with prefixed name
    assert!(
        output
            .js
            .contains("__omni_readonlySignal(\"Counter_initial\""),
        "prop should be emitted via __omni_readonlySignal. JS:\n{}",
        output.js
    );

    // Verify the computed is emitted AFTER the readonlySignal declaration
    let computed_pos = output
        .js
        .find("__omni_computed(\"Counter_double\"")
        .expect("computed declaration should exist");
    let readonly_pos = output
        .js
        .find("__omni_readonlySignal(\"Counter_initial\"")
        .expect("prop readonlySignal declaration should exist");
    assert!(
        readonly_pos < computed_pos,
        "prop must be declared before computed that references it, got readonly at {} vs computed at {}",
        readonly_pos,
        computed_pos
    );
}

#[test]
fn slot_name_exact_match_not_substring() {
    // A slot named "head" should NOT match a child with slot="header"
    let src = r#"
component Card {
    view {
        <div>
            @slot head
        </div>
    }
}
component Page {
    view {
        Card {
            <div slot="header">Header content</div>
            <div slot="head">Head content</div>
        }
    }
}
"#;
    let result = crate::compile_omniview(src, false, false);
    assert!(
        result.success,
        "compilation should succeed: {:?}",
        result.errors
    );
    let html = result.files.unwrap().html;
    // With the fix, slot="head" should match named slot "head" exactly,
    // but slot="header" should NOT match (substring false-positive).
    assert!(
        html.contains("Head content"),
        "named slot 'head' should receive its child with slot=\"head\""
    );
    assert!(
        !html.contains("Header content"),
        "slot=\"header\" should NOT match named slot \"head\" (substring false-positive)"
    );
}
