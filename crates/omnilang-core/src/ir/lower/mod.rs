//! AST → IR lowering: walks the parsed AST and produces IR types.
//!
//! This module is the entry point for the lowering pipeline.
//! Sub-modules handle specific domains: components, views, expressions,
//! statements, and motion/animation definitions.

mod component;
pub(crate) mod design;
mod expr;
mod lower_inline_motions;
mod motion;
mod route;
mod stmt;
mod view;

pub(crate) use component::*;
pub use design::lower_design_with_consts;
#[allow(unused_imports)]
pub(super) use motion::*;
pub(crate) use stmt::*;

use super::logic::*;
use super::types::*;
use super::types_motion::*;
use crate::parser::ast::{ImportStmt, Program};
use crate::semantic::SemanticResult;
use crate::sourcemap::{SourceMapEntry, SourceSpan};
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

/// Convert a lexer Span to a SourceSpan for source map entries.
fn span_to_source_span(span: &crate::lexer::Span) -> SourceSpan {
    SourceSpan {
        line: span.line,
        col: span.col,
        len: (span.end.saturating_sub(span.start)) as u32,
    }
}

/// Lower AST -> IR.
///
/// `external_const_values` supplies cross-file constant values (built from all
/// source files in `analyse_multi`).  When present it is merged into the
/// per-file const map so that `import { BRAND } from "./tokens.omni"` followed
/// by `token primary: color = BRAND;` resolves correctly.  Local const
/// declarations always take precedence.
pub fn lower(
    program: &Program,
    semantics: &mut SemanticResult,
    external_const_values: Option<&FxHashMap<String, String>>,
) -> IRProgram {
    let mut modules = FxHashMap::default();
    let mut source_map: FxHashMap<String, SourceMapEntry> = FxHashMap::default();

    // Build variant → union name lookup for tagged union construction
    let mut variant_map: FxHashMap<String, String> = FxHashMap::default();
    for module in program.modules.values() {
        for tu in &module.tagged_unions {
            for variant in &tu.variants {
                variant_map.insert(variant.name.clone(), tu.name.clone());
            }
        }
    }

    // Build const → value map for design token compile-time resolution.
    // External values (cross-file) are merged first; local declarations then
    // overwrite them so same-file definitions always win.
    let mut const_values: FxHashMap<String, String> = FxHashMap::default();
    if let Some(ext) = external_const_values {
        const_values.extend(ext.iter().map(|(k, v)| (k.clone(), v.clone())));
    }
    for module in program.modules.values() {
        for const_def in &module.consts {
            if let crate::parser::ast::Expr::Literal(crate::parser::ast::Literal::String(s), _) =
                &const_def.value
            {
                const_values.insert(const_def.name.clone(), s.clone());
            }
        }
    }

    for (module_name, module) in &program.modules {
        let mut components = FxHashMap::default();
        let mut stores = FxHashMap::default();
        let mut functions = FxHashMap::default();
        let mut structs = FxHashMap::default();
        let mut enums = FxHashMap::default();
        let mut type_aliases = FxHashMap::default();
        let mut imports: Vec<ImportIR> = Vec::new();
        let mut exports: Vec<ExportIR> = Vec::new();

        for comp in &module.components {
            semantics
                .symbols
                .push_scope(&format!("component:{}", comp.name));
            let ir = lower_component(comp, &variant_map, semantics);
            semantics.symbols.pop_scope();
            let node_id = format!("{}:component:{}", module_name, comp.name);
            source_map.insert(
                node_id,
                SourceMapEntry {
                    node_id: format!("{}:component:{}", module_name, comp.name),
                    omni_span: span_to_source_span(&comp.span),
                    target_spans: Vec::new(),
                },
            );
            components.insert(comp.name.clone(), ir);
        }

        for store in &module.stores {
            semantics
                .symbols
                .push_scope(&format!("store:{}", store.name));
            let ir = lower_store(store, &variant_map, semantics);
            semantics.symbols.pop_scope();
            let node_id = format!("{}:store:{}", module_name, store.name);
            source_map.insert(
                node_id.clone(),
                SourceMapEntry {
                    node_id,
                    omni_span: span_to_source_span(&store.span),
                    target_spans: Vec::new(),
                },
            );
            stores.insert(store.name.clone(), ir);
        }

        // Inline stores (`store Name { ... }` in a component body) lower to
        // the SAME module-level StoreIR as top-level stores — v2 域 5's
        // "顶层与内联统一" promise. Registered AFTER the module-level stores
        // so an inline store wins the name lookup for its owning module (the
        // owning component's `store_refs` must resolve to ITS store, not a
        // same-named module-level one); their members are also still
        // flattened into the component (parse-time behavior, unchanged).
        for comp in &module.components {
            for def in &comp.inline_store_defs {
                semantics.symbols.push_scope(&format!("store:{}", def.name));
                let ir = lower_store(def, &variant_map, semantics);
                semantics.symbols.pop_scope();
                let node_id = format!("{}:store:{}", module_name, def.name);
                source_map.insert(
                    node_id.clone(),
                    SourceMapEntry {
                        node_id,
                        omni_span: span_to_source_span(&def.span),
                        target_spans: Vec::new(),
                    },
                );
                stores.insert(def.name.clone(), ir);
            }
        }

        for func in &module.functions {
            semantics.symbols.push_scope(&format!("fn:{}", func.name));
            let ir = super::lower_logic::lower_function(func, &variant_map, semantics);
            semantics.symbols.pop_scope();
            let node_id = format!("{}:fn:{}", module_name, func.name);
            source_map.insert(
                node_id.clone(),
                SourceMapEntry {
                    node_id,
                    omni_span: span_to_source_span(&func.span),
                    target_spans: Vec::new(),
                },
            );
            functions.insert(func.name.clone(), ir);
        }

        for s in &module.structs {
            let ir = super::lower_logic::lower_struct(s);
            structs.insert(s.name.clone(), ir);
        }

        for tu in &module.tagged_unions {
            let ir = super::lower_logic::lower_enum(tu);
            enums.insert(tu.name.clone(), ir);
        }

        for ta in &module.type_aliases {
            let ir = super::lower_logic::lower_type_alias(ta);
            type_aliases.insert(ta.name.clone(), ir);
        }

        for imp in &module.imports {
            imports.push(super::lower_logic::lower_import(imp));
        }

        for exp in &module.exports {
            exports.push(super::lower_logic::lower_export(exp));
        }

        // Module motion table: module-level @motion definitions plus
        // component-level ones — both are module-scoped in the IR because
        // motion_index is name-keyed. Name collisions are diagnosed by the
        // semantic pass (validate_motion.rs); the first definition wins
        // defensively so codegen never sees duplicate names even when
        // diagnostics are ignored.
        let mut motions: Vec<MotionIntent> = Vec::new();
        let mut motion_names: FxHashSet<&str> = FxHashSet::default();
        for motion in module
            .motions
            .iter()
            .chain(module.components.iter().flat_map(|c| c.motions.iter()))
        {
            if !motion_names.insert(motion.name.as_str()) {
                continue;
            }
            let node_id = format!("{}:motion:{}", module_name, motion.name);
            source_map.insert(
                node_id.clone(),
                SourceMapEntry {
                    node_id,
                    omni_span: span_to_source_span(&motion.span),
                    target_spans: Vec::new(),
                },
            );
            motions.push(lower_motion_def(motion, &variant_map, semantics));
        }

        // Validate design blocks before lowering
        for design in &module.designs {
            let errors = crate::ir::lower::design::validate_design(design);
            for err in &errors {
                // E0005 for undefined token references, E0006 for
                // theme/dimension validation errors
                let kind = if err.contains("undefined token") {
                    crate::error::DiagnosticKind::E0005
                } else {
                    crate::error::DiagnosticKind::E0006
                };
                semantics.diagnostics.push(crate::error::Diagnostic {
                    severity: crate::error::Severity::Error,
                    kind,
                    message: format!("[design '{}'] {}", design.name, err),
                    file: None,
                    line: 0,
                    col: 0,
                    end_line: None,
                    end_col: None,
                    suggestion: None,
                });
            }
        }

        let designs: Vec<super::DesignIR> = module
            .designs
            .iter()
            .map(|d| lower_design_with_consts(d, &const_values, &variant_map, semantics))
            .collect();

        let module_ir = ModuleIR {
            name: module_name.clone(),
            functions,
            structs,
            enums,
            type_aliases,
            components,
            stores,
            imports,
            exports,
            motions,
            designs,
            source_span: None,
        };

        modules.insert(module_name.clone(), module_ir);
    }

    // Hoist inline motion bodies on element triggers into named MotionIntents
    // (__anon_m_N) in each module's motion table. Runs after component
    // lowering (so triggers' inline_motion fields are populated) and before
    // the motion_index build below, so the synthesized motions are indexed
    // too. Codegen then sees only named motions.
    for module in modules.values_mut() {
        lower_inline_motions::lower_inline_motions(module);
    }

    // ── Resolve cross-file design imports ──
    // Collect all designs from all modules into a global map (keyed by design name).
    // Designs are implicitly public (like components/stores) — no visibility gate.
    let all_designs: FxHashMap<String, super::DesignIR> = modules
        .values()
        .flat_map(|m| m.designs.iter().cloned())
        .map(|d| (d.name.clone(), d))
        .collect();

    // For each module, scan its Named imports.  If an imported name matches
    // a design in all_designs, clone the DesignIR into the importing module.
    for (module_name, module) in program.modules.iter() {
        let module_ir = modules
            .get_mut(module_name)
            .expect("module must exist in modules map (populated above)");
        let mut imported_designs: Vec<super::DesignIR> = Vec::new();

        for imp in &module.imports {
            if let ImportStmt::Named(named) = imp {
                for name in &named.names {
                    if let Some(design) = all_designs.get(name) {
                        // Avoid duplicates (design defined in this file already there)
                        if !module_ir.designs.iter().any(|d| d.name == *name) {
                            imported_designs.push(design.clone());
                        }
                    }
                }
            }
        }

        module_ir.designs.extend(imported_designs);
    }

    // Collect stdlib imports: any "import { ... } from \"std/<module>\""
    let mut stdlib_imports: FxHashSet<String> = FxHashSet::default();
    for module in program.modules.values() {
        for imp in &module.imports {
            if let ImportStmt::Named(ni) = imp {
                if ni.path.starts_with("std/") {
                    if let Some(p) = ni.path.strip_prefix("std/") {
                        stdlib_imports.insert(p.to_string());
                    }
                }
            }
        }
    }

    // Build motion name index for O(1) lookups
    let mut motion_index: FxHashMap<String, MotionIntent> = FxHashMap::default();
    for module in modules.values() {
        for motion in &module.motions {
            motion_index.insert(motion.name.clone(), motion.clone());
        }
    }

    let mut ir_program = IRProgram {
        version: "1.0".to_string(),
        modules,
        source_map,
        stdlib_imports,
        source_span: None,
        motion_index,
    };

    // IR-level semantic validation passes. Orchestrated in the unified
    // pipeline `analyse::run_ir_passes` (Task 5.5 — pass orchestration moved
    // out of lower(); the pipeline is declared in one place, in analyse.rs,
    // so every entry point shares the same pass sequence and order).
    crate::analyse::run_ir_passes(&mut ir_program, &mut semantics.diagnostics);

    ir_program
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;
    use crate::semantic::analyze;

    fn lower_source(source: &str) -> IRProgram {
        let tokens = tokenize(source);
        let (program, _) = parse(tokens, source);
        let mut semantics = analyze(&program);
        lower(&program, &mut semantics, None)
    }

    #[test]
    fn lower_simple_function() {
        let ir = lower_source("fn main() -> i32 { return 42; }");
        assert_eq!(ir.version, "1.0");
        let module = ir
            .modules
            .get("input.omni")
            .expect("should have input.omni module");
        assert!(
            module.functions.contains_key("main"),
            "should contain 'main' function, got keys: {:?}",
            module.functions.keys()
        );
    }

    #[test]
    fn lower_struct_and_function() {
        let source = "struct Point { x: i32, y: i32 }
                       fn origin() -> Point { return Point { x: 0, y: 0 }; }";
        let ir = lower_source(source);
        let module = ir.modules.get("input.omni").expect("should have module");
        assert!(
            module.structs.contains_key("Point"),
            "should contain 'Point' struct"
        );
        assert!(
            module.functions.contains_key("origin"),
            "should contain 'origin' function"
        );
    }

    #[test]
    fn lower_import_std() {
        let source = "import { sqrt } from \"std/math\"; fn main() { print(sqrt(16)); }";
        let ir = lower_source(source);
        assert!(
            !ir.stdlib_imports.is_empty(),
            "should detect stdlib imports"
        );
        assert!(
            ir.stdlib_imports.contains("math"),
            "should contain 'math' stdlib import"
        );
    }

    // ═══════════════════════════════════════════════════════
    // Component-level @motion lowering tests
    // ═══════════════════════════════════════════════════════

    #[test]
    fn lowers_component_level_motions_into_module_table() {
        // Component-level `@motion` defs are module-scoped in the IR: they
        // land in ModuleIR.motions and resolve via the name-keyed
        // motion_index, so `@enter={m}` triggers can codegen against them.
        let source = "component App { @motion slide { from { translateX: 0 } to { translateX: 100 } over: 300ms } view { <div @enter={slide}/> } }";
        let ir = lower_source(source);
        let module = ir.modules.get("input.omni").expect("module");
        assert_eq!(module.motions.len(), 1, "component motion must be lowered");
        assert_eq!(module.motions[0].name, "slide");
        assert_eq!(module.motions[0].body.stops.len(), 2);

        let found = ir
            .find_motion("slide")
            .expect("motion_index must resolve the component motion");
        assert_eq!(found.name, "slide");
    }

    #[test]
    fn component_motion_collision_is_diagnosed_and_first_definition_wins() {
        // A component motion whose name collides with a module-level motion
        // is a hard E0006 diagnostic (motion names are module-scoped); the
        // lowering keeps the first definition so codegen stays unambiguous.
        let source = "@motion fade { from { opacity: 0 } to { opacity: 1 } over: 100ms } component App { @motion fade { from { opacity: 0 } to { opacity: 1 } over: 200ms } view { <div/> } }";
        let tokens = tokenize(source);
        let (program, _) = parse(tokens, source);
        let mut semantics = analyze(&program);
        let ir = lower(&program, &mut semantics, None);

        assert!(
            semantics
                .diagnostics
                .iter()
                .any(|d| d.message.contains("collides with an existing motion name")),
            "expected collision diagnostic, got: {:?}",
            semantics
                .diagnostics
                .iter()
                .map(|d| &d.message)
                .collect::<Vec<_>>()
        );
        let module = ir.modules.get("input.omni").expect("module");
        assert_eq!(module.motions.len(), 1, "first definition must win");
        assert_eq!(module.motions[0].name, "fade");
    }

    // ═══════════════════════════════════════════════════════
    // View / Element lowering tests
    // ═══════════════════════════════════════════════════════

    /// Helper: lower a component source and return the first (only) component.
    fn lower_first_component(source: &str) -> ComponentIR {
        let ir = lower_source(source);
        let module = ir.modules.get("input.omni").expect("should have module");
        let (_, comp) = module
            .components
            .iter()
            .next()
            .expect("should have a component");
        comp.clone()
    }

    /// Helper: lower a store source and return the first (only) store.
    fn lower_first_store(source: &str) -> StoreIR {
        let ir = lower_source(source);
        let module = ir.modules.get("input.omni").expect("should have module");
        let (_, store) = module.stores.iter().next().expect("should have a store");
        store.clone()
    }

    #[test]
    fn lower_component_with_text_element() {
        let source = r#"
import omniview;
component Hello {
    view {
        <div>Hello World</div>
    }
}
"#;
        let comp = lower_first_component(source);
        assert_eq!(comp.name, "Hello");

        let view = comp.view.expect("component should have a view");
        match &view {
            ViewTree::Element(el) => {
                assert_eq!(el.tag, "div");
                assert_eq!(el.children.len(), 1, "div should have one child");
                match &el.children[0] {
                    ViewTree::Text(t) => assert_eq!(t, "Hello World"),
                    other => panic!("expected Text child, got {:?}", other),
                }
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn lower_component_with_multiple_children() {
        let source = r#"
import omniview;
component Multi {
    view {
        <div>
            <span>First</span>
            <span>Second</span>
        </div>
    }
}
"#;
        let comp = lower_first_component(source);
        let view = comp.view.expect("should have view");
        match &view {
            ViewTree::Element(el) => {
                assert_eq!(el.tag, "div");
                assert_eq!(el.children.len(), 2);
                match &el.children[0] {
                    ViewTree::Element(child) => {
                        assert_eq!(child.tag, "span");
                        assert!(
                            matches!(&child.children[0], ViewTree::Text(t) if t == "First"),
                            "first child text should be 'First'"
                        );
                    }
                    other => panic!("expected Element child, got {:?}", other),
                }
                match &el.children[1] {
                    ViewTree::Element(child) => {
                        assert_eq!(child.tag, "span");
                        assert!(
                            matches!(&child.children[0], ViewTree::Text(t) if t == "Second"),
                            "second child text should be 'Second'"
                        );
                    }
                    other => panic!("expected Element child, got {:?}", other),
                }
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn lower_component_with_interpolation() {
        let source = r#"
import omniview;
component Greet {
    signal name: String = "World";
    view {
        <div>Hello {name}</div>
    }
}
"#;
        let comp = lower_first_component(source);

        // Check state is lowered correctly
        assert_eq!(comp.signals.len(), 1, "should have one signal");
        assert_eq!(comp.signals[0].name, "name");
        // Type normalized via TypeKind::from
        assert_eq!(comp.signals[0].r#type.kind.as_str(), "string");

        // Check view has interpolation
        let view = comp.view.expect("should have view");
        match &view {
            ViewTree::Element(el) => {
                assert_eq!(el.tag, "div");
                assert_eq!(el.children.len(), 2);
                match &el.children[0] {
                    // Trailing space before `{name}` is preserved (DOM whitespace
                    // folding keeps it: "Hello {name}" renders "Hello World").
                    ViewTree::Text(t) => assert_eq!(t, "Hello "),
                    other => panic!("expected Text child, got {:?}", other),
                }
                match &el.children[1] {
                    ViewTree::Interpolation(expr) => match expr.as_ref() {
                        ExprIR::IdentifierExpr { name, .. } => {
                            assert_eq!(
                                name, "name",
                                "interpolation should reference 'name' signal"
                            );
                        }
                        other => panic!("expected IdentifierExpr, got {:?}", other),
                    },
                    other => panic!("expected Interpolation child, got {:?}", other),
                }
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    // ═══════════════════════════════════════════════════════
    // Signal binding tests
    // ═══════════════════════════════════════════════════════

    #[test]
    fn lower_component_with_signal_binding() {
        let source = r#"
import omniview;
component InputComp {
    signal text: String = "";
    view {
        <input bind:value={text} />
    }
}
"#;
        let comp = lower_first_component(source);
        let view = comp.view.expect("should have view");

        match &view {
            ViewTree::Element(el) => {
                assert_eq!(el.tag, "input");
                assert!(el.bindings.contains_key("value"), "should have bind:value");
                let binding = el.bindings.get("value").unwrap();
                assert_eq!(
                    binding.name, "text",
                    "binding should reference 'text' signal"
                );
                assert!(
                    binding.name == "text",
                    "binding should reference 'text' signal, got '{}'",
                    binding.name
                );
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn lower_component_with_multiple_bindings() {
        let source = r#"
import omniview;
component FormField {
    signal val: String = "";
    signal checked: bool = false;
    view {
        <div>
            <input bind:value={val} />
            <input type="checkbox" bind:checked={checked} />
        </div>
    }
}
"#;
        let comp = lower_first_component(source);
        let view = comp.view.expect("should have view");
        match &view {
            ViewTree::Element(el) => {
                // First child: input with bind:value
                assert_eq!(el.children.len(), 2);
                match &el.children[0] {
                    ViewTree::Element(input_el) => {
                        assert_eq!(input_el.tag, "input");
                        let val_binding = input_el
                            .bindings
                            .get("value")
                            .expect("first input should have bind:value");
                        assert_eq!(val_binding.name, "val");
                    }
                    other => panic!("expected Element, got {:?}", other),
                }
                // Second child: checkbox with bind:checked
                match &el.children[1] {
                    ViewTree::Element(chk_el) => {
                        assert_eq!(chk_el.tag, "input");
                        let chk_binding = chk_el
                            .bindings
                            .get("checked")
                            .expect("checkbox should have bind:checked");
                        assert_eq!(chk_binding.name, "checked");
                    }
                    other => panic!("expected Element, got {:?}", other),
                }
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    // ═══════════════════════════════════════════════════════
    // Semantic ref binding validation tests
    // ═══════════════════════════════════════════════════════

    /// Helper: run the full pipeline (parse → analyze → lower) and return
    /// the resulting diagnostics.
    fn lower_source_with_diagnostics(source: &str) -> Vec<crate::error::Diagnostic> {
        let tokens = tokenize(source);
        let (program, _) = parse(tokens, source);
        let mut semantics = analyze(&program);
        let _ir = lower(&program, &mut semantics, None);
        semantics.diagnostics
    }

    #[test]
    fn semantic_binding_without_ref_errors_in_pipeline() {
        // focus={sig} on an element WITHOUT @ref → error from the semantic
        // binding validation pass (wired into lower()).
        let diagnostics = lower_source_with_diagnostics(
            r#"
import omniview;
component App {
    signal isActive: bool = false;
    view {
        <div focus={isActive} />
    }
}
"#,
        );
        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.severity == crate::error::Severity::Error)
            .collect();
        assert!(
            errors
                .iter()
                .any(|d| d.message.contains("'focus' requires @ref")),
            "expected 'focus requires @ref' error, got: {:?}",
            errors
        );
    }

    #[test]
    fn semantic_binding_with_ref_passes_in_pipeline() {
        // focus={sig} on an element WITH @ref + declared ref → no semantic
        // binding errors.
        let diagnostics = lower_source_with_diagnostics(
            r#"
import omniview;
component App {
    ref inputBox: Element;
    signal isActive: bool = false;
    view {
        <input @ref={inputBox} focus={isActive} />
    }
}
"#,
        );
        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.severity == crate::error::Severity::Error)
            .collect();
        assert!(
            errors
                .iter()
                .all(|d| !d.message.contains("semantic binding")),
            "expected no semantic binding errors, got: {:?}",
            errors
        );
    }

    #[test]
    fn semantic_binding_with_undeclared_ref_warns_via_validate_refs() {
        // scroll-y={sig} with @ref={pane} but no `ref pane: Element`
        // declaration → handled by validate_refs (warning). The semantic
        // binding pass must NOT duplicate it as an error.
        let diagnostics = lower_source_with_diagnostics(
            r#"
import omniview;
component App {
    signal pos: f64 = 0;
    view {
        <div @ref={pane} scroll-y={pos} />
    }
}
"#,
        );
        assert!(
            diagnostics.iter().any(|d| {
                d.severity == crate::error::Severity::Warning
                    && d.message.contains("no `ref pane: Element` declaration")
            }),
            "expected validate_refs warning for undeclared ref, got: {:?}",
            diagnostics
        );
        assert!(
            diagnostics
                .iter()
                .all(|d| !d.message.contains("undeclared ref")),
            "semantic binding pass must not duplicate the undeclared-ref check, got: {:?}",
            diagnostics
        );
    }

    #[test]
    fn computed_value_expr_stays_static_prop_in_pipeline() {
        // value={age.to_string()} is a computed expression — a generic Static
        // prop, not a semantic binding. It must NOT trigger semantic binding
        // validation and must remain in ElementIR.props (one-way computed
        // binding pattern, e.g. FormControlsDemo).
        let source = r#"
import omniview;
component App {
    signal age: i32 = 30;
    view {
        <input type="number" value={age.to_string()} />
    }
}
"#;
        let tokens = tokenize(source);
        let (program, _) = parse(tokens, source);
        let mut semantics = analyze(&program);
        let ir = lower(&program, &mut semantics, None);
        assert!(
            semantics
                .diagnostics
                .iter()
                .all(|d| !d.message.contains("semantic binding")),
            "computed value expr must not trigger semantic binding errors, got: {:?}",
            semantics.diagnostics
        );
        let module = ir.modules.get("input.omni").expect("should have module");
        let comp = module
            .components
            .get("App")
            .expect("should have App component");
        match comp.view.as_ref() {
            Some(ViewTree::Element(el)) => {
                assert!(
                    el.props.contains_key("value"),
                    "value prop should remain in props for computed expressions"
                );
                assert!(
                    el.semantic_bindings.is_empty(),
                    "computed value expr must not lower into semantic_bindings"
                );
            }
            other => panic!("expected root Element, got {:?}", other),
        }
    }

    // ═══════════════════════════════════════════════════════
    // Event handler tests
    // ═══════════════════════════════════════════════════════

    #[test]
    fn lower_component_with_inline_event_handler() {
        let source = r#"
import omniview;
component Clicker {
    signal count: i32 = 0;
    view {
        <button @onPress={() => { count = count + 1; }}>Click</button>
    }
}
"#;
        let comp = lower_first_component(source);
        let view = comp.view.expect("should have view");
        match &view {
            ViewTree::Element(el) => {
                assert!(
                    el.events.contains_key("onPress"),
                    "should have onPress event"
                );
                let handler = el.events.get("onPress").unwrap();
                match handler {
                    EventHandlerIR::InlineFn {
                        params,
                        event_data,
                        body,
                        ..
                    } => {
                        assert!(params.is_empty(), "inline handler should have no params");
                        assert!(event_data.is_some(), "onPress should have event data");
                        // The mutation `count = count + 1` is lowered as DirectAssign
                        // (count = count + 1 is an assignment, not an increment operator)
                        let has_direct_assign = body.statements.iter().any(|s| {
                            matches!(s, StatementIR::MutationStmt { intent: Some(MutationIntent::DirectAssign { target, .. }) }
                                if target.name == "count")
                        });
                        assert!(
                            has_direct_assign,
                            "handler body should contain count assignment"
                        );
                    }
                    other => panic!("expected InlineFn handler, got {:?}", other),
                }
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn lower_component_with_fn_ref_event_handler() {
        let source = r#"
import omniview;
component FnRefDemo {
    signal msg: String = "";
    fn handlePress() {
        msg = "clicked";
    }
    view {
        <button @onPress={handlePress}>Click</button>
    }
}
"#;
        let comp = lower_first_component(source);
        let view = comp.view.expect("should have view");
        match &view {
            ViewTree::Element(el) => {
                assert!(
                    el.events.contains_key("onPress"),
                    "should have onPress event"
                );
                let handler = el.events.get("onPress").unwrap();
                match handler {
                    EventHandlerIR::FnRef {
                        r#ref,
                        event_data,
                        used_fields,
                        ..
                    } => {
                        assert_eq!(r#ref, "handlePress", "fn ref should be 'handlePress'");
                        assert!(event_data.is_some(), "onPress should have event data");
                        // used_fields should be backfilled (None initially, Some after Pass 2)
                        // In our case we expect Some(...) since backfill runs
                        assert!(
                            used_fields.is_some(),
                            "used_fields should be backfilled (None -> Some)"
                        );
                    }
                    other => panic!("expected FnRef handler, got {:?}", other),
                }
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn lower_component_with_store_action_event() {
        let source = r#"
import omniview;
import { TodoStore } from "../stores/TodoStore";
component StoreActionDemo {
    @store TodoStore;
    view {
        <button @onPress={TodoStore.remove(1)}>Remove</button>
    }
}
"#;
        let comp = lower_first_component(source);
        let view = comp.view.expect("should have view");
        match &view {
            ViewTree::Element(el) => {
                assert!(
                    el.events.contains_key("onPress"),
                    "should have onPress event"
                );
                let handler = el.events.get("onPress").unwrap();
                // Note: TodoStore.remove(1) is parsed as InlineFn (not StoreAction),
                // because the parser wraps method-call expressions as InlineFn.
                // The body contains a method call expression.
                match handler {
                    EventHandlerIR::InlineFn {
                        event_data, body, ..
                    } => {
                        assert!(event_data.is_some(), "onPress should have event data");
                        let has_call = body.statements.iter().any(|s| {
                            matches!(
                                s,
                                StatementIR::ExprStmtIR { expr }
                                    if matches!(
                                        expr,
                                        ExprIR::MethodCallExpr { method, object, .. }
                                            if method == "remove"
                                                && matches!(
                                                    object.as_ref(),
                                                    ExprIR::IdentifierExpr { name, .. }
                                                        if name == "TodoStore"
                                                )
                                    )
                            )
                        });
                        assert!(
                            has_call,
                            "handler body should contain TodoStore.remove(1) call"
                        );
                    }
                    other => panic!(
                        "expected InlineFn handler (store action call), got {:?}",
                        other
                    ),
                }
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    /// Regression: a store action whose name collides with the signal
    /// mutation method map (`increment`/`set`/`push`/...) must stay a call —
    /// previously `CounterStore.increment()` was mis-lowered to
    /// `MutationIntent::Increment { target: "CounterStore" }`, mutating a
    /// non-existent signal instead of invoking the store action.
    #[test]
    fn lower_store_action_with_mutation_method_name_stays_a_call() {
        let source = r#"
import omniview;
store CounterStore {
    signal count: i32 = 0;
    action increment() {
        count = count + 1;
    }
}
component CounterDemo {
    @store CounterStore;
    view {
        <button @onPress={CounterStore.increment()}>+</button>
    }
}
"#;
        let comp = lower_first_component(source);
        let view = comp.view.expect("should have view");
        match &view {
            ViewTree::Element(el) => {
                let handler = el.events.get("onPress").expect("should have onPress event");
                match handler {
                    EventHandlerIR::InlineFn { body, .. } => {
                        let stmts = &body.statements;
                        let has_call = stmts.iter().any(|s| {
                            matches!(s, StatementIR::ExprStmtIR { expr: ExprIR::MethodCallExpr { method, object, .. } }
                                if method == "increment"
                                    && matches!(&**object, ExprIR::IdentifierExpr { name, .. } if name == "CounterStore"))
                        });
                        assert!(
                            has_call,
                            "handler must contain the CounterStore.increment() call, got: {:?}",
                            stmts
                        );
                        assert!(
                            !stmts.iter().any(|s| matches!(
                                s,
                                StatementIR::MutationStmt {
                                    intent: Some(MutationIntent::Increment { .. })
                                }
                            )),
                            "must NOT lower the store action to an Increment mutation"
                        );
                    }
                    other => panic!("expected InlineFn handler, got {:?}", other),
                }
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    /// Regression: a store action named `set` (which collides with the signal
    /// mutation rewrite `obj.set(v)` → `ident("set")` in expression position)
    /// must stay a call — previously `CounterStore.set(5)` in an event handler
    /// was silently mangled to the bare identifier `set`.
    #[test]
    fn lower_store_action_named_set_stays_a_call() {
        let source = r#"
import omniview;
store CounterStore {
    signal count: i32 = 0;
    action set(v: i32) {
        count = v;
    }
}
component CounterDemo {
    @store CounterStore;
    view {
        <button @onPress={CounterStore.set(5)}>Set</button>
    }
}
"#;
        let comp = lower_first_component(source);
        let view = comp.view.expect("should have view");
        match &view {
            ViewTree::Element(el) => {
                let handler = el.events.get("onPress").expect("should have onPress event");
                match handler {
                    EventHandlerIR::InlineFn { body, .. } => {
                        let stmts = &body.statements;
                        let has_call = stmts.iter().any(|s| {
                            matches!(s, StatementIR::ExprStmtIR { expr: ExprIR::MethodCallExpr { method, object, .. } }
                                if method == "set"
                                    && matches!(&**object, ExprIR::IdentifierExpr { name, .. } if name == "CounterStore"))
                        });
                        assert!(
                            has_call,
                            "handler must contain the CounterStore.set(5) call, got: {:?}",
                            stmts
                        );
                        assert!(
                            !stmts.iter().any(|s| {
                                matches!(
                                    s,
                                    StatementIR::ExprStmtIR {
                                        expr: ExprIR::IdentifierExpr { name, .. }
                                    } if name == "set"
                                )
                            }),
                            "must NOT lower the store action to bare ident('set')"
                        );
                    }
                    other => panic!("expected InlineFn handler, got {:?}", other),
                }
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    /// Regression: a store action named `get` (which collides with the signal
    /// access rewrite `obj.get()` → `obj.value` in expression position) must
    /// stay a call — previously `CounterStore.get()` in an event handler was
    /// silently lowered to `CounterStore.value`.
    #[test]
    fn lower_store_action_named_get_stays_a_call() {
        let source = r#"
import omniview;
store CounterStore {
    signal count: i32 = 0;
    action get(): i32 {
        return count;
    }
}
component CounterDemo {
    @store CounterStore;
    view {
        <button @onPress={CounterStore.get()}>Get</button>
    }
}
"#;
        let comp = lower_first_component(source);
        let view = comp.view.expect("should have view");
        match &view {
            ViewTree::Element(el) => {
                let handler = el.events.get("onPress").expect("should have onPress event");
                match handler {
                    EventHandlerIR::InlineFn { body, .. } => {
                        let stmts = &body.statements;
                        let has_call = stmts.iter().any(|s| {
                            matches!(s, StatementIR::ExprStmtIR { expr: ExprIR::MethodCallExpr { method, object, .. } }
                                if method == "get"
                                    && matches!(&**object, ExprIR::IdentifierExpr { name, .. } if name == "CounterStore"))
                        });
                        assert!(
                            has_call,
                            "handler must contain the CounterStore.get() call, got: {:?}",
                            stmts
                        );
                        assert!(
                            !stmts.iter().any(|s| {
                                matches!(
                                    s,
                                    StatementIR::ExprStmtIR {
                                        expr: ExprIR::FieldAccessExpr { field, .. }
                                    } if field == "value"
                                )
                            }),
                            "must NOT lower the store action to a .value field access"
                        );
                    }
                    other => panic!("expected InlineFn handler, got {:?}", other),
                }
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn lower_component_with_event_used_fields() {
        let source = r#"
import omniview;
component EventFieldsDemo {
    signal items: Array<String> = [];
    fn handleInput(e) {
        items.push(e.value);
    }
    view {
        <input @onInput={handleInput} />
    }
}
"#;
        let comp = lower_first_component(source);
        let view = comp.view.expect("should have view");
        match &view {
            ViewTree::Element(el) => {
                let handler = el.events.get("onInput").expect("should have onInput event");
                match handler {
                    EventHandlerIR::FnRef {
                        r#ref,
                        used_fields,
                        event_data,
                        ..
                    } => {
                        assert_eq!(r#ref, "handleInput");
                        let fields = used_fields
                            .as_ref()
                            .expect("used_fields should be backfilled after Pass 2");
                        assert!(
                            fields.contains(&"value".to_string()),
                            "used_fields should contain 'value' (referenced as e.value)"
                        );
                        assert_eq!(event_data.as_ref().unwrap(), &EventDataKind::InputEvent);
                    }
                    other => panic!("expected FnRef handler, got {:?}", other),
                }
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn lower_component_with_prevent_default() {
        let source = r#"
import omniview;
component PreventDemo {
    view {
        <form @onSubmit={(e) => { e.preventDefault(); }}>
            <button type="submit">Submit</button>
        </form>
    }
}
"#;
        let comp = lower_first_component(source);
        let view = comp.view.expect("should have view");
        match &view {
            ViewTree::Element(el) => {
                let handler = el
                    .events
                    .get("onSubmit")
                    .expect("should have onSubmit event");
                match handler {
                    EventHandlerIR::InlineFn {
                        needs_prevent_default,
                        event_data,
                        ..
                    } => {
                        assert!(
                            needs_prevent_default,
                            "should set needs_prevent_default for e.preventDefault()"
                        );
                        assert_eq!(event_data.as_ref().unwrap(), &EventDataKind::SubmitEvent);
                    }
                    other => panic!("expected InlineFn handler, got {:?}", other),
                }
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    // ═══════════════════════════════════════════════════════
    // For-loop (list rendering) tests
    // ═══════════════════════════════════════════════════════

    #[test]
    fn lower_for_loop_simple() {
        let source = r#"
import omniview;
component ListDemo {
    signal items: Array<String> = [];
    view {
        <div>
            @for item in items {
                <span>{item}</span>
            }
        </div>
    }
}
"#;
        let comp = lower_first_component(source);
        let view = comp.view.expect("should have view");
        match &view {
            ViewTree::Element(el) => {
                assert_eq!(el.children.len(), 1, "div should have one child");
                match &el.children[0] {
                    ViewTree::ListRendering(list) => {
                        // Check iterable
                        match &list.iterable {
                            ExprIR::IdentifierExpr { name, .. } => {
                                assert_eq!(name, "items", "iterable should be 'items'");
                            }
                            other => {
                                panic!("expected IdentifierExpr for iterable, got {:?}", other)
                            }
                        }
                        // Check item binding
                        assert_eq!(list.item_binding.name, "item");
                        // Check body exists and has the right structure
                        assert!(list.body.is_some(), "list rendering should have a body");
                        if let Some(body) = &list.body {
                            match body {
                                ViewTree::Element(frag) => {
                                    assert_eq!(
                                        frag.tag, "fragment",
                                        "body wrapper should be fragment"
                                    );
                                    assert_eq!(frag.construction_kind, ConstructionKind::List);
                                    assert_eq!(frag.children.len(), 1);
                                    match &frag.children[0] {
                                        ViewTree::Element(span_el) => {
                                            assert_eq!(span_el.tag, "span");
                                            match &span_el.children[0] {
                                                ViewTree::Interpolation(iexpr) => {
                                                    match iexpr.as_ref() {
                                                        ExprIR::IdentifierExpr { name, .. } => {
                                                            assert_eq!(name, "item");
                                                        }
                                                        other => panic!(
                                                            "expected IdentifierExpr, got {:?}",
                                                            other
                                                        ),
                                                    }
                                                }
                                                other => panic!(
                                                    "expected Interpolation, got {:?}",
                                                    other
                                                ),
                                            }
                                        }
                                        other => panic!("expected Element child, got {:?}", other),
                                    }
                                }
                                other => panic!("expected Element(fragment) body, got {:?}", other),
                            }
                        }
                        // Key expr and empty body should be None
                        assert!(list.key_expr.is_none(), "key_expr should be None");
                        assert!(list.empty_body.is_none(), "empty_body should be None");
                    }
                    other => panic!("expected ListRendering, got {:?}", other),
                }
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn lower_for_loop_with_key_and_empty() {
        let source = r#"
import omniview;
component KeyedList {
    signal users: Array<String> = [];
    view {
        <ul>
            @for user in users key={user} {
                <li class: item>{user}</li>
            }
            @empty {
                <li class: empty>No users</li>
            }
        </ul>
    }
}
"#;
        let comp = lower_first_component(source);
        let view = comp.view.expect("should have view");
        match &view {
            ViewTree::Element(el) => {
                assert_eq!(el.children.len(), 1);
                match &el.children[0] {
                    ViewTree::ListRendering(list) => {
                        assert_eq!(list.item_binding.name, "user");
                        match &list.iterable {
                            ExprIR::IdentifierExpr { name, .. } => assert_eq!(name, "users"),
                            other => panic!("expected IdentifierExpr, got {:?}", other),
                        }
                        // Key expr: should be an IdentifierExpr("user")
                        assert!(list.key_expr.is_some(), "key_expr should be present");
                        if let Some(key) = &list.key_expr {
                            match key {
                                ExprIR::IdentifierExpr { name, .. } => {
                                    assert_eq!(name, "user", "key should be 'user'");
                                }
                                other => panic!("expected IdentifierExpr key, got {:?}", other),
                            }
                        }
                        // Empty body: should be present
                        assert!(list.empty_body.is_some(), "empty_body should be present");
                        if let Some(empty) = &list.empty_body {
                            match empty {
                                ViewTree::Element(frag) => {
                                    assert_eq!(frag.tag, "fragment");
                                    assert_eq!(frag.construction_kind, ConstructionKind::List);
                                    assert_eq!(frag.children.len(), 1);
                                    match &frag.children[0] {
                                        ViewTree::Element(el2) => {
                                            assert_eq!(el2.tag, "li");
                                            assert!(el2.props.contains_key("class"));
                                        }
                                        other => panic!("expected Element(li), got {:?}", other),
                                    }
                                }
                                other => panic!("expected Element(fragment), got {:?}", other),
                            }
                        }
                    }
                    other => panic!("expected ListRendering, got {:?}", other),
                }
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    // ═══════════════════════════════════════════════════════
    // If-conditional tests
    // ═══════════════════════════════════════════════════════

    #[test]
    fn lower_if_conditional_simple() {
        let source = r#"
import omniview;
component ConditionalDemo {
    signal show: bool = true;
    view {
        <div>
            @if show {
                <span>Visible</span>
            }
        </div>
    }
}
"#;
        let comp = lower_first_component(source);
        let view = comp.view.expect("should have view");
        match &view {
            ViewTree::Element(el) => {
                assert_eq!(el.children.len(), 1);
                match &el.children[0] {
                    ViewTree::Conditional(cond) => {
                        assert_eq!(cond.branches.len(), 1, "should have one branch");
                        // Check condition
                        match &cond.branches[0].condition {
                            ExprIR::IdentifierExpr { name, .. } => {
                                assert_eq!(name, "show", "condition should be 'show'");
                            }
                            other => panic!("expected IdentifierExpr condition, got {:?}", other),
                        }
                        // Check body
                        assert!(cond.branches[0].body.is_some(), "branch should have body");
                        if let Some(body) = &cond.branches[0].body {
                            match body {
                                ViewTree::Element(frag) => {
                                    assert_eq!(frag.tag, "fragment");
                                    assert_eq!(
                                        frag.construction_kind,
                                        ConstructionKind::Conditional
                                    );
                                    assert_eq!(frag.children.len(), 1);
                                    match &frag.children[0] {
                                        ViewTree::Element(span_el) => {
                                            assert_eq!(span_el.tag, "span");
                                            match &span_el.children[0] {
                                                ViewTree::Text(t) => assert_eq!(t, "Visible"),
                                                other => panic!("expected Text, got {:?}", other),
                                            }
                                        }
                                        other => panic!("expected Element, got {:?}", other),
                                    }
                                }
                                other => panic!("expected Element(fragment), got {:?}", other),
                            }
                        }
                        // Fallback should be None
                        assert!(
                            cond.fallback.is_none(),
                            "fallback should be None for simple @if"
                        );
                        // Transition mode default
                        assert!(
                            cond.transition_mode.is_none(),
                            "transition_mode should default to None"
                        );
                    }
                    other => panic!("expected Conditional, got {:?}", other),
                }
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn lower_if_else_conditional() {
        let source = r#"
import omniview;
component IfElseDemo {
    signal loggedIn: bool = false;
    view {
        <div>
            @if loggedIn {
                <span>Welcome</span>
            } @else {
                <span>Please login</span>
            }
        </div>
    }
}
"#;
        let comp = lower_first_component(source);
        let view = comp.view.expect("should have view");
        match &view {
            ViewTree::Element(el) => {
                match &el.children[0] {
                    ViewTree::Conditional(cond) => {
                        assert_eq!(cond.branches.len(), 1);
                        assert!(cond.fallback.is_some(), "should have else fallback");
                        // Check fallback structure
                        if let Some(fallback) = &cond.fallback {
                            match fallback {
                                ViewTree::Element(frag) => {
                                    assert_eq!(frag.tag, "fragment");
                                    assert_eq!(frag.children.len(), 1);
                                    match &frag.children[0] {
                                        ViewTree::Element(span_el) => {
                                            assert_eq!(span_el.tag, "span");
                                            match &span_el.children[0] {
                                                ViewTree::Text(t) => assert_eq!(t, "Please login"),
                                                other => panic!("expected Text, got {:?}", other),
                                            }
                                        }
                                        other => panic!("expected Element, got {:?}", other),
                                    }
                                }
                                other => panic!("expected Element(fragment), got {:?}", other),
                            }
                        }
                    }
                    other => panic!("expected Conditional, got {:?}", other),
                }
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn lower_if_else_if_chain() {
        let source = r#"
import omniview;
component MultiCond {
    signal score: i32 = 0;
    view {
        <div>
            @if score >= 90 {
                <span>A</span>
            } @else if score >= 80 {
                <span>B</span>
            } @else if score >= 70 {
                <span>C</span>
            } @else {
                <span>D</span>
            }
        </div>
    }
}
"#;
        let comp = lower_first_component(source);
        let view = comp.view.expect("should have view");
        match &view {
            ViewTree::Element(el) => {
                match &el.children[0] {
                    ViewTree::Conditional(cond) => {
                        assert_eq!(
                            cond.branches.len(),
                            3,
                            "should have 3 branches (if + 2 else-if)"
                        );
                        assert!(cond.fallback.is_some(), "should have else fallback");
                        // Check each branch has a condition
                        for (i, branch) in cond.branches.iter().enumerate() {
                            assert!(branch.body.is_some(), "branch {i} should have a body");
                            // Conditions should be BinaryOpExpr (score >= threshold)
                            match &branch.condition {
                                ExprIR::BinaryOpExpr { op: _, left, right } => {
                                    // left should be IdentifierExpr("score")
                                    match left.as_ref() {
                                        ExprIR::IdentifierExpr { name, .. } => {
                                            assert_eq!(name, "score");
                                        }
                                        other => {
                                            panic!("expected IdentifierExpr left, got {:?}", other)
                                        }
                                    }
                                    // right should be LiteralExpr with the threshold
                                    match right.as_ref() {
                                        ExprIR::LiteralExpr { value } => {
                                            assert!(
                                                value.is_number(),
                                                "threshold should be a number"
                                            );
                                        }
                                        other => {
                                            panic!("expected LiteralExpr right, got {:?}", other)
                                        }
                                    }
                                }
                                other => panic!("expected BinaryOpExpr condition, got {:?}", other),
                            }
                        }
                    }
                    other => panic!("expected Conditional, got {:?}", other),
                }
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    // ═══════════════════════════════════════════════════════
    // Store lowering tests
    // ═══════════════════════════════════════════════════════

    #[test]
    fn lower_store_with_signals() {
        let source = r#"
import omniview;
store CounterStore {
    signal count: i32 = 0;
    signal name: String = "hello";
}
"#;
        let store = lower_first_store(source);
        assert_eq!(store.name, "CounterStore");
        // Check signals
        assert_eq!(store.signals.len(), 2);
        let count_type = store
            .signals
            .get("count")
            .expect("should have 'count' signal");
        // i32 normalized to int via TypeKind::from
        assert_eq!(count_type.kind.as_str(), "int");
        let name_type = store
            .signals
            .get("name")
            .expect("should have 'name' signal");
        // String normalized to lowercase via TypeKind::from
        assert_eq!(name_type.kind.as_str(), "string");
        // Check signal_defaults
        assert_eq!(store.signal_defaults.len(), 2);
        let count_default = store
            .signal_defaults
            .get("count")
            .expect("should have default for 'count'");
        assert_eq!(*count_default, serde_json::json!(0));
        let name_default = store
            .signal_defaults
            .get("name")
            .expect("should have default for 'name'");
        assert_eq!(*name_default, serde_json::json!("hello"));
        // No getters, no actions
        assert!(store.getters.is_empty(), "store should have no getters");
        assert!(store.actions.is_empty(), "store should have no actions");
    }

    #[test]
    fn lower_store_with_theme_signal() {
        // v2 域 5: `signal mode: String @theme = "light";` — the @theme
        // marker must survive into StoreIR.theme_signal.
        let source = r#"
import omniview;
store ThemeStore {
    signal mode: String @theme = "light";
    signal toggleCount: i32 = 0;
}
"#;
        let store = lower_first_store(source);
        assert_eq!(
            store.theme_signal.as_deref(),
            Some("mode"),
            "@theme marker must lower into StoreIR.theme_signal"
        );
    }

    #[test]
    fn lower_store_without_theme_signal_keeps_none() {
        let source = r#"
import omniview;
store CounterStore {
    signal count: i32 = 0;
}
"#;
        let store = lower_first_store(source);
        assert!(store.theme_signal.is_none());
    }

    #[test]
    fn lower_store_with_getters_and_actions() {
        let source = r#"
import omniview;
store TodoStore {
    signal items: Array<String> = [];
    signal status: String = "all";

    getter itemCount: i32 = items.len();

    action addItem(text: String) {
        items.push(text);
    }

    action clearAll() {
        items = [];
    }
}
"#;
        let store = lower_first_store(source);
        assert_eq!(store.name, "TodoStore");
        // Check signals
        assert_eq!(store.signals.len(), 2);
        assert!(
            store.signals.contains_key("items"),
            "should have 'items' signal"
        );
        assert!(
            store.signals.contains_key("status"),
            "should have 'status' signal"
        );
        // Check signal_defaults
        assert_eq!(store.signal_defaults.len(), 2);
        let items_default = store
            .signal_defaults
            .get("items")
            .expect("should have default for 'items'");
        assert_eq!(
            *items_default,
            serde_json::json!([]),
            "items default should be empty array"
        );
        // Check getters
        assert_eq!(store.getters.len(), 1);
        assert!(
            store.getters.contains_key("itemCount"),
            "should have 'itemCount' getter"
        );
        // Check getter body references items.len()
        let getter_body = store.getters.get("itemCount").unwrap();
        match getter_body {
            ExprIR::MethodCallExpr { object, method, .. } => {
                assert_eq!(method, "len");
                match object.as_ref() {
                    ExprIR::IdentifierExpr { name, .. } => assert_eq!(name, "items"),
                    other => panic!("expected IdentifierExpr object, got {:?}", other),
                }
            }
            other => panic!("expected MethodCallExpr, got {:?}", other),
        }
        // Check actions
        assert_eq!(store.actions.len(), 2);
        let add_item = store
            .actions
            .iter()
            .find(|a| a.name == "addItem")
            .expect("should have 'addItem' action");
        assert_eq!(add_item.params.len(), 1);
        assert_eq!(add_item.params[0].name, "text");
        let clear_all = store
            .actions
            .iter()
            .find(|a| a.name == "clearAll")
            .expect("should have 'clearAll' action");
        assert!(
            clear_all.params.is_empty(),
            "clearAll should have no params"
        );
    }

    #[test]
    // The `ratio: 3.14` below is a deliberate fixture value (not PI) — the
    // test pins that the store lowering preserves the literal as written.
    #[allow(clippy::approx_constant)]
    fn lower_store_initial_values() {
        let source = r#"
import omniview;
store ConfigStore {
    signal count: i32 = 42;
    signal ratio: f64 = 3.14;
    signal flag: bool = true;
    signal label: String = "test";
    signal nothing: i32? = null;
}
"#;
        let store = lower_first_store(source);
        // Check all defaults
        assert_eq!(
            store.signal_defaults.get("count"),
            Some(&serde_json::json!(42)),
            "count default should be 42"
        );
        assert_eq!(
            store.signal_defaults.get("ratio"),
            Some(&serde_json::json!(3.14)),
            "ratio default should be 3.14"
        );
        assert_eq!(
            store.signal_defaults.get("flag"),
            Some(&serde_json::json!(true)),
            "flag default should be true"
        );
        assert_eq!(
            store.signal_defaults.get("label"),
            Some(&serde_json::json!("test")),
            "label default should be 'test'"
        );
        assert_eq!(
            store.signal_defaults.get("nothing"),
            Some(&serde_json::Value::Null),
            "nothing default should be null"
        );
    }

    // ═══════════════════════════════════════════════════════
    // Component reference tests
    // ═══════════════════════════════════════════════════════

    #[test]
    fn lower_component_with_child_component_ref() {
        let source = r#"
import omniview;
import { Child } from "./Child";
component Parent {
    view {
        <div>
            <Child prop1="hello" />
        </div>
    }
}
"#;
        let comp = lower_first_component(source);
        let view = comp.view.expect("should have view");
        match &view {
            ViewTree::Element(el) => {
                assert_eq!(el.tag, "div");
                match &el.children[0] {
                    ViewTree::Element(child_el) => {
                        assert_eq!(
                            child_el.tag, "Child",
                            "child component tag should be 'Child'"
                        );
                        assert_eq!(child_el.construction_kind, ConstructionKind::Component);
                        // Check that props were passed correctly
                        let prop = child_el.props.get("prop1").expect("should have prop1");
                        match prop {
                            ExprIR::LiteralExpr { value } => {
                                assert_eq!(*value, serde_json::json!("hello"));
                            }
                            other => panic!("expected LiteralExpr, got {:?}", other),
                        }
                    }
                    other => panic!("expected Element child, got {:?}", other),
                }
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn lower_component_with_slots() {
        let source = r#"
import omniview;
component Layout {
    view {
        <div>
            <header>@slot header</header>
            <main>@slot</main>
        </div>
    }
}
"#;
        let comp = lower_first_component(source);
        let view = comp.view.expect("should have view");
        match &view {
            ViewTree::Element(el) => {
                assert_eq!(el.tag, "div");
                assert_eq!(el.children.len(), 2);

                // Check named slot
                match &el.children[0] {
                    ViewTree::Element(header_el) => {
                        assert_eq!(header_el.tag, "header");
                        assert_eq!(header_el.children.len(), 1);
                        match &header_el.children[0] {
                            ViewTree::Slot { name, .. } => {
                                assert_eq!(
                                    name.as_deref(),
                                    Some("header"),
                                    "should be named slot 'header'"
                                );
                            }
                            other => panic!("expected Slot, got {:?}", other),
                        }
                    }
                    other => panic!("expected Element, got {:?}", other),
                }
                // Check default slot
                match &el.children[1] {
                    ViewTree::Element(main_el) => {
                        assert_eq!(main_el.tag, "main");
                        assert_eq!(main_el.children.len(), 1);
                        match &main_el.children[0] {
                            ViewTree::Slot { name, .. } => {
                                assert!(name.is_none(), "should be default (unnamed) slot");
                            }
                            other => panic!("expected Slot, got {:?}", other),
                        }
                    }
                    other => panic!("expected Element, got {:?}", other),
                }
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn lower_slot_preserves_fallback() {
        // v2 (design D4): the `: Type` constraint is deleted, so the plain
        // `@slot name { fallback }` form is the only one — fallback content
        // must survive lowering.
        let source = r#"
import omniview;
component Card {
    view {
        <div>
            @slot body { <p>Default body</p> }
        </div>
    }
}
"#;
        let comp = lower_first_component(source);
        let view = comp.view.expect("should have view");
        match &view {
            ViewTree::Element(el) => {
                assert_eq!(el.tag, "div");
                assert_eq!(el.children.len(), 1);
                match &el.children[0] {
                    ViewTree::Slot { name, fallback } => {
                        assert_eq!(name.as_deref(), Some("body"), "should be named slot 'body'");
                        assert_eq!(
                            fallback.len(),
                            1,
                            "fallback content must be preserved during lowering"
                        );
                        match &fallback[0] {
                            ViewTree::Element(p) => {
                                assert_eq!(p.tag, "p");
                                assert_eq!(p.children.len(), 1);
                                match &p.children[0] {
                                    ViewTree::Text(t) => assert_eq!(t, "Default body"),
                                    other => panic!("expected Text, got {:?}", other),
                                }
                            }
                            other => panic!("expected Element fallback, got {:?}", other),
                        }
                    }
                    other => panic!("expected Slot, got {:?}", other),
                }
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    // ═══════════════════════════════════════════════════════
    // Component body slots (ADR-053 D5)
    // ═══════════════════════════════════════════════════════

    #[test]
    fn lower_component_body_slots() {
        let source = r#"
import omniview;
component Modal {
    @slot header { <h4>Default header</h4> }
    @slot default
    view {
        <div>{header}</div>
    }
}
"#;
        let comp = lower_first_component(source);
        assert_eq!(comp.slots.len(), 2, "two body-level slots expected");

        let header = &comp.slots[0];
        assert_eq!(header.name.as_deref(), Some("header"));
        assert_eq!(
            header.fallback.len(),
            1,
            "fallback content must be preserved"
        );
        match &header.fallback[0] {
            ViewTree::Element(el) => assert_eq!(el.tag, "h4"),
            other => panic!("expected Element fallback, got {:?}", other),
        }

        let default_slot = &comp.slots[1];
        assert!(
            default_slot.name.is_none(),
            "`@slot default` must lower to the unnamed slot"
        );
        assert!(default_slot.fallback.is_empty());
    }

    #[test]
    fn lower_typed_body_slot_is_rejected_at_parse() {
        // v2 (design D4): `@slot footer: String` is a deleted form — the
        // parser hard-errors before lowering can see a type constraint.
        let source = r#"
import omniview;
component Modal {
    @slot footer: String { <p>Default footer</p> }
    view {
        <div>{footer}</div>
    }
}
"#;
        let tokens = crate::lexer::tokenize(source);
        let (_program, errors) = crate::parser::parse(tokens, source);
        assert!(
            errors.iter().any(|e| {
                e.kind == Some(crate::error::DiagnosticKind::ESlotRemoved)
                    && e.message.contains("类型约束与数据参数已删除")
            }),
            "expected ESlotRemoved parse error, got: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn lower_consumer_named_prop_slot() {
        let source = r#"
import omniview;
component App {
    view {
        <Modal header={<div>Title</div>}>content</Modal>
    }
}
"#;
        let comp = lower_first_component(source);
        let view = comp.view.expect("should have view");
        match &view {
            ViewTree::Element(el) => {
                assert_eq!(el.tag, "Modal");
                assert_eq!(
                    el.children.len(),
                    2,
                    "default children + named slot projection expected"
                );
                // Default children first
                match &el.children[0] {
                    ViewTree::Text(t) => assert_eq!(t, "content"),
                    other => panic!("expected Text child, got {:?}", other),
                }
                // Named slot projection as ViewTree::Slot
                match &el.children[1] {
                    ViewTree::Slot { name, fallback } => {
                        assert_eq!(name.as_deref(), Some("header"));
                        assert_eq!(fallback.len(), 1, "projected content must be preserved");
                        match &fallback[0] {
                            ViewTree::Element(p) => assert_eq!(p.tag, "div"),
                            other => panic!("expected Element, got {:?}", other),
                        }
                    }
                    other => panic!("expected Slot child, got {:?}", other),
                }
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn lower_consumer_named_prop_slot_multiple_nodes() {
        let source = r#"
import omniview;
component App {
    view {
        <Modal header={<div>a</div><div>b</div>} />
    }
}
"#;
        let comp = lower_first_component(source);
        let view = comp.view.expect("should have view");
        match &view {
            ViewTree::Element(el) => match &el.children[0] {
                ViewTree::Slot { name, fallback, .. } => {
                    assert_eq!(name.as_deref(), Some("header"));
                    assert_eq!(fallback.len(), 2, "all sibling nodes must be projected");
                }
                other => panic!("expected Slot child, got {:?}", other),
            },
            other => panic!("expected Element, got {:?}", other),
        }
    }

    // ═══════════════════════════════════════════════════════
    // Comprehensive integration test
    // ═══════════════════════════════════════════════════════

    #[test]
    fn lower_full_component_integration() {
        let source = r#"
import omniview;
import { TodoStore } from "../stores/TodoStore";
component TodoApp {
    @store TodoStore;
    signal input: String = "";
    signal filter: String = "all";
    computed filtered: Array<String> = if filter == "all" { items } else { [] };
    event onNotify: String;

    fn addItem() {
        if input != "" {
            TodoStore.add(input);
            input = "";
        }
    }

    fn handleKey(e) {
        if e.key == "Enter" {
            addItem();
        }
    }

    @mount {
        println("TodoApp mounted");
    }

    view {
        <div class: app>
            <h1>{title}</h1>
            <input bind:value={input} @onKeyDown={handleKey} />
            <button @onPress={addItem}>Add</button>
            <div>
                @for item in filtered {
                    <span>{item}</span>
                }
                @empty {
                    <p>No items</p>
                }
            </div>
            @if input.len() > 0 {
                <p>Typing...</p>
            }
        </div>
    }
    style {
        .app { padding: 16px; }
    }
}
"#;
        let comp = lower_first_component(source);
        assert_eq!(comp.name, "TodoApp");

        // Check props
        assert!(comp.props.is_empty(), "TodoApp has no props");

        // Check store refs
        assert_eq!(comp.store_refs, vec!["TodoStore"]);

        // Check event declarations
        assert!(
            comp.events.iter().any(|e| e.name == "onNotify"),
            "component should declare onNotify event"
        );

        // Check signals (states)
        assert_eq!(comp.signals.len(), 2, "should have 2 signals");
        assert_eq!(comp.signals[0].name, "input");
        assert_eq!(comp.signals[1].name, "filter");

        // Check computeds
        assert_eq!(comp.computeds.len(), 1, "should have 1 computed");
        assert_eq!(comp.computeds[0].name, "filtered");

        // Check lifecycle hooks
        assert_eq!(comp.lifecycles.len(), 1);
        assert_eq!(comp.lifecycles[0].kind, LifecycleKind::Mount);

        // Check fns
        assert_eq!(comp.fns.len(), 2, "should have 2 functions");
        let fn_names: Vec<&str> = comp.fns.iter().map(|f| f.name.as_str()).collect();
        assert!(fn_names.contains(&"addItem"));
        assert!(fn_names.contains(&"handleKey"));

        // Check view structure
        let view = comp.view.expect("should have view");
        match &view {
            ViewTree::Element(el) => {
                assert_eq!(el.tag, "div");
                assert_eq!(el.class_map_bindings.len(), 0);
                // Should have 5 children: h1, input, button, div, @if
                assert_eq!(el.children.len(), 5);

                // h1 child
                match &el.children[0] {
                    ViewTree::Element(h1) => {
                        assert_eq!(h1.tag, "h1");
                    }
                    other => panic!("expected h1 Element, got {:?}", other),
                }

                // input with binding
                match &el.children[1] {
                    ViewTree::Element(input_el) => {
                        assert_eq!(input_el.tag, "input");
                        assert!(input_el.bindings.contains_key("value"));
                        assert!(input_el.events.contains_key("onKeyDown"));
                    }
                    other => panic!("expected input Element, got {:?}", other),
                }

                // button
                match &el.children[2] {
                    ViewTree::Element(btn) => {
                        assert_eq!(btn.tag, "button");
                        assert!(btn.events.contains_key("onPress"));
                    }
                    other => panic!("expected button Element, got {:?}", other),
                }

                // div with @for
                match &el.children[3] {
                    ViewTree::Element(inner_div) => {
                        assert_eq!(inner_div.tag, "div");
                        assert_eq!(inner_div.children.len(), 1);
                        match &inner_div.children[0] {
                            ViewTree::ListRendering(list) => {
                                assert_eq!(list.item_binding.name, "item");
                                assert!(list.empty_body.is_some(), "should have @empty body");
                            }
                            other => panic!("expected ListRendering, got {:?}", other),
                        }
                    }
                    other => panic!("expected div Element, got {:?}", other),
                }

                // @if conditional
                match &el.children[4] {
                    ViewTree::Conditional(cond) => {
                        assert_eq!(cond.branches.len(), 1);
                        match &cond.branches[0].condition {
                            ExprIR::BinaryOpExpr { op, left, right } => {
                                assert_eq!(*op, BinaryOpIR::Gt, "condition should be >");
                                match left.as_ref() {
                                    ExprIR::MethodCallExpr { object, method, .. } => {
                                        assert_eq!(method, "len");
                                        match object.as_ref() {
                                            ExprIR::IdentifierExpr { name, .. } => {
                                                assert_eq!(name, "input");
                                            }
                                            other => {
                                                panic!("expected IdentifierExpr, got {:?}", other)
                                            }
                                        }
                                    }
                                    other => panic!("expected MethodCallExpr, got {:?}", other),
                                }
                                match right.as_ref() {
                                    ExprIR::LiteralExpr { value } => {
                                        assert_eq!(*value, serde_json::json!(0));
                                    }
                                    other => panic!("expected LiteralExpr, got {:?}", other),
                                }
                            }
                            other => panic!("expected BinaryOpExpr condition, got {:?}", other),
                        }
                    }
                    other => panic!("expected Conditional, got {:?}", other),
                }
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }
}
