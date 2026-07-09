//! Style resolver: traverses IR ViewTrees and resolves per-element computed CSS.
//!
//! Used by the Poiesis editor to render the visual canvas with accurate styling.
//! Unlike the full codegen pipeline, this does NOT generate JS/HTML/CSS strings —
//! it returns structured data that the editor consumes directly.

use crate::codegen::omniview::design::{expand_variant, resolve_class_properties};
use crate::ir::*;
use rustc_hash::FxHashMap;

use super::{
    ClassSummary, DesignSummary, ResolvedElementStyle, StyleResolveResult, TokenSummary,
    VariantDimSummary, VariantSummary,
};

/// Build a design map from all `design` blocks in the IR program.
///
/// Each design block is indexed by its name for O(1) lookup during style resolution.
/// Since `ModuleIR.designs` is a `Vec<DesignIR>`, this flattens across all modules.
pub fn build_design_map(ir: &IRProgram) -> FxHashMap<String, DesignIR> {
    let mut map = FxHashMap::default();
    for module in ir.modules.values() {
        for design in &module.designs {
            map.insert(design.name.clone(), design.clone());
        }
    }
    map
}

/// Resolve per-element computed styles from IR.
///
/// Traverses the ViewTree of every component across all modules and computes
/// the resolved CSS for each element. The `design_map` is built via
/// [`build_design_map`] and provides the design-system context for style lookup.
pub fn resolve_styles_from_ir(
    ir: &IRProgram,
    design_map: &FxHashMap<String, DesignIR>,
) -> StyleResolveResult {
    let mut components: FxHashMap<String, Vec<ResolvedElementStyle>> = FxHashMap::default();

    for (module_name, module) in &ir.modules {
        for (comp_name, _component) in &module.components {
            let view_tree = match find_component_view(ir, module_name, comp_name) {
                Some(v) => v,
                None => continue,
            };

            let mut elements: Vec<ResolvedElementStyle> = Vec::new();
            let path_prefix = comp_name.clone();
            resolve_view_tree(&view_tree, &path_prefix, 0, design_map, &mut elements);
            components.insert(comp_name.clone(), elements);
        }
    }

    // Build design summary from the first available design
    let design = ir
        .modules
        .values()
        .flat_map(|m| &m.designs)
        .next()
        .map(|d| build_design_summary(d));

    StyleResolveResult {
        success: true,
        components,
        design,
        errors: Vec::new(),
    }
}

/// Find a component's view tree in the IR.
///
/// Looks up the component by module name and component name, then clones
/// its optional view tree. Returns `None` if either lookup fails.
fn find_component_view(ir: &IRProgram, module_name: &str, comp_name: &str) -> Option<ViewTree> {
    ir.modules
        .get(module_name)?
        .components
        .get(comp_name)?
        .view
        .clone()
}

/// Recursively resolve styles for a ViewTree node and its children.
///
/// - `Element` nodes produce a `ResolvedElementStyle` pushed to `out`.
/// - `Conditional` nodes recurse into their branches and fallback.
/// - `ListRendering` nodes recurse into their body and empty_body.
/// - `Interpolation`, `Text`, and `Slot` are leaf nodes with no style data.
fn resolve_view_tree(
    tree: &ViewTree,
    parent_path: &str,
    index: usize,
    design_map: &FxHashMap<String, DesignIR>,
    out: &mut Vec<ResolvedElementStyle>,
) {
    match tree {
        ViewTree::Element(el) => {
            let path = format!("{}.{}.{}", parent_path, el.tag, index);
            let resolved = resolve_element(el, &path, design_map);

            let mut child_paths: Vec<String> = Vec::new();
            for (i, child) in el.children.iter().enumerate() {
                let child_path = format!("{}.{}.{}", path, child_tag(child), i);
                child_paths.push(child_path);
                resolve_view_tree(child, &path, i, design_map, out);
            }

            let mut result = resolved;
            result.children = child_paths;
            out.push(result);
        }
        ViewTree::Conditional(cond) => {
            for (i, branch) in cond.branches.iter().enumerate() {
                if let Some(ref body) = branch.body {
                    let branch_path = format!("{}.if.{}", parent_path, i);
                    resolve_view_tree(body, &branch_path, 0, design_map, out);
                }
            }
            if let Some(ref fallback) = cond.fallback {
                let fb_path = format!("{}.else", parent_path);
                resolve_view_tree(fallback, &fb_path, 0, design_map, out);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                let list_path = format!("{}.for", parent_path);
                resolve_view_tree(body, &list_path, 0, design_map, out);
            }
            if let Some(ref empty_body) = list.empty_body {
                let empty_path = format!("{}.empty", parent_path);
                resolve_view_tree(empty_body, &empty_path, 0, design_map, out);
            }
        }
        ViewTree::Interpolation(_) | ViewTree::Text(_) | ViewTree::Slot(_) => {
            // Non-element nodes — no styles to resolve
        }
    }
}

/// Get a short tag-like name for a ViewTree child (for path generation).
fn child_tag(tree: &ViewTree) -> &str {
    match tree {
        ViewTree::Element(el) => &el.tag,
        ViewTree::Conditional(_) => "if",
        ViewTree::ListRendering(_) => "for",
        ViewTree::Interpolation(_) => "interp",
        ViewTree::Text(_) => "text",
        ViewTree::Slot(_) => "slot",
    }
}

/// Extract a handler name string from an EventHandlerIR, if available.
///
/// Returns the function reference or store action reference for named handlers.
/// Returns `None` for inline function handlers (which have no simple name).
fn get_handler_name(handler: &EventHandlerIR) -> Option<&str> {
    match handler {
        EventHandlerIR::FnRef { r#ref, .. } => Some(r#ref.as_str()),
        EventHandlerIR::StoreAction { r#ref, .. } => Some(r#ref.as_str()),
        EventHandlerIR::InlineFn { .. } => None,
    }
}

/// Resolve all style sources for a single ElementIR into computed CSS.
///
/// Resolution order (later sources override earlier ones):
/// 1. Collect class names from the `class` prop
/// 2. Expand `_design_variant` bindings into additional class names
/// 3. Look up class properties from all registered designs
/// 4. Extract event handler names from lowered event IR
/// 5. Extract semantic HTML attributes from string-literal props
/// 6. Extract signal bindings from the element's binding map
/// 7. Pass through inline_style
fn resolve_element(
    el: &ElementIR,
    path: &str,
    design_map: &FxHashMap<String, DesignIR>,
) -> ResolvedElementStyle {
    let mut classes: Vec<String> = Vec::new();
    let mut computed_css: FxHashMap<String, String> = FxHashMap::default();
    let mut html_attrs: FxHashMap<String, String> = FxHashMap::default();
    let mut events: FxHashMap<String, String> = FxHashMap::default();
    let mut bindings: FxHashMap<String, String> = FxHashMap::default();

    // 1. Collect class names from `class` prop
    if let Some(class_expr) = el.props.get("class") {
        if let ExprIR::LiteralExpr { value } = class_expr {
            if let Some(class_str) = value.as_str() {
                for name in class_str.split_whitespace() {
                    if !name.is_empty() {
                        classes.push(name.to_string());
                    }
                }
            }
        }
    }

    // 2. Expand variant binding -> class names
    //    The `_design_variant` prop is a synthetic JSON literal set during lowering.
    if let Some(variant_prop) = el.props.get("_design_variant") {
        if let ExprIR::LiteralExpr { value } = variant_prop {
            let design_name = value.get("design_name").and_then(|v| v.as_str());
            let variant_name = value.get("variant_name").and_then(|v| v.as_str());
            let dim_values = value.get("dimension_values");

            if let (Some(dn), Some(vn), Some(dv)) = (design_name, variant_name, dim_values) {
                let dim_map: FxHashMap<String, String> = dv
                    .as_object()
                    .map(|obj| {
                        obj.iter()
                            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                            .collect()
                    })
                    .unwrap_or_default();

                if let Some(design) = design_map.get(dn) {
                    if let Ok((variant_classes, _extra_css)) = expand_variant(design, vn, &dim_map)
                    {
                        classes.extend(variant_classes);
                    }
                }
            }
        }
    }

    // 3. Resolve CSS properties from referenced design classes.
    //    Merges properties from all designs that define the class.
    for class_name in &classes {
        for design in design_map.values() {
            if let Some(props) = resolve_class_properties(design, class_name) {
                for (prop, val) in props {
                    computed_css.insert(prop.clone(), val.clone());
                }
            }
        }
    }

    // 4. Extract events from lowered event handlers.
    //    Events are stored in `el.events` as EventHandlerIR values, keyed by
    //    the original prop name (e.g. "onClick", "onChange").
    for (name, handler) in &el.events {
        let event_name = name
            .strip_prefix("on")
            .map(|n| n.to_lowercase())
            .unwrap_or_else(|| name.to_lowercase());
        if let Some(handler_name) = get_handler_name(handler) {
            events.insert(event_name, handler_name.to_string());
        }
    }

    // 5. Extract semantic HTML attrs from string-literal props.
    //    Skips internal props (those starting with `_`), `class`, and `_design_variant`.
    for (name, value) in &el.props {
        if name == "class" || name == "_design_variant" || name.starts_with('_') {
            continue;
        }
        if let ExprIR::LiteralExpr { value: v } = value {
            if let Some(s) = v.as_str() {
                if is_html_attr(name) {
                    html_attrs.insert(name.clone(), s.to_string());
                }
            }
        }
    }

    // 6. Extract signal bindings
    for (prop, signal_ref) in &el.bindings {
        bindings.insert(prop.clone(), signal_ref.name.clone());
    }

    // 7. Pass through inline_style
    let inline_style = el.inline_style.clone();

    ResolvedElementStyle {
        path: path.to_string(),
        tag: el.tag.clone(),
        classes,
        computed_css,
        html_attrs,
        events,
        bindings,
        inline_style,
        children: Vec::new(), // filled by caller
        source_line: el.source_line,
    }
}

/// Check if a prop name is a known HTML attribute (not a style or internal prop).
fn is_html_attr(name: &str) -> bool {
    matches!(
        name,
        "id" | "placeholder"
            | "type"
            | "value"
            | "disabled"
            | "checked"
            | "required"
            | "readonly"
            | "href"
            | "src"
            | "alt"
            | "title"
            | "name"
            | "rows"
            | "cols"
            | "max"
            | "min"
            | "step"
            | "label"
            | "for"
            | "action"
            | "method"
    )
}

/// Build a design summary from DesignIR for the Poiesis design panel.
fn build_design_summary(design: &DesignIR) -> DesignSummary {
    DesignSummary {
        name: design.name.clone(),
        tokens: design
            .tokens
            .iter()
            .map(|(name, t)| TokenSummary {
                name: name.clone(),
                token_type: format!("{:?}", t.token_type),
                value: t.value.clone(),
            })
            .collect(),
        classes: design
            .classes
            .iter()
            .map(|c| ClassSummary {
                name: c.name.clone(),
                properties: c.properties.clone(),
            })
            .collect(),
        variants: design
            .variants
            .iter()
            .map(|v| VariantSummary {
                name: v.name.clone(),
                dimensions: v
                    .dimensions
                    .iter()
                    .map(|d| VariantDimSummary {
                        name: d.name.clone(),
                        values: d.values.clone(),
                    })
                    .collect(),
                defaults: v.defaults.clone(),
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustc_hash::FxHashMap;

    /// Helper to construct a `design::VariantIR` with the full path,
    /// avoiding the `ir::VariantIR` (logic::VariantIR) name conflict.
    fn make_design_variant(
        name: &str,
        dimensions: Vec<crate::ir::design::VariantDimIR>,
        defaults: Vec<(String, String)>,
        base: Vec<(String, String)>,
        branches: Vec<crate::ir::design::VariantBranchIR>,
    ) -> crate::ir::design::VariantIR {
        crate::ir::design::VariantIR {
            name: name.to_string(),
            dimensions,
            defaults: defaults.into_iter().collect(),
            base: base.into_iter().collect(),
            branches,
        }
    }

    /// Create a minimal DesignIR for testing style resolution.
    fn test_design() -> DesignIR {
        let mut tokens = FxHashMap::default();
        tokens.insert(
            "surface".to_string(),
            TokenIR {
                value: "#ffffff".to_string(),
                token_type: TokenTypeIR::Color,
            },
        );
        tokens.insert(
            "primary".to_string(),
            TokenIR {
                value: "#6366f1".to_string(),
                token_type: TokenTypeIR::Color,
            },
        );

        let classes = vec![ClassIR {
            name: "Card".to_string(),
            properties: vec![
                ("background".to_string(), "#ffffff".to_string()),
                ("border-radius".to_string(), "8px".to_string()),
            ]
            .into_iter()
            .collect(),
            pseudo_states: vec![PseudoStateIR {
                pseudo: "hover".to_string(),
                properties: vec![(
                    "box-shadow".to_string(),
                    "0 4px 12px rgba(0,0,0,0.08)".to_string(),
                )]
                .into_iter()
                .collect(),
            }],
        }];

        let variants = vec![make_design_variant(
            "Button",
            vec![
                crate::ir::design::VariantDimIR {
                    name: "color".to_string(),
                    values: vec!["primary".to_string(), "danger".to_string()],
                },
                crate::ir::design::VariantDimIR {
                    name: "size".to_string(),
                    values: vec!["sm".to_string(), "md".to_string()],
                },
            ],
            vec![
                ("color".to_string(), "primary".to_string()),
                ("size".to_string(), "md".to_string()),
            ],
            vec![
                ("display".to_string(), "inline-flex".to_string()),
                ("border-radius".to_string(), "var(--radiusMd)".to_string()),
            ],
            vec![crate::ir::design::VariantBranchIR {
                dimension: "color".to_string(),
                value: "primary".to_string(),
                properties: vec![("background".to_string(), "#6366f1".to_string())]
                    .into_iter()
                    .collect(),
            }],
        )];

        DesignIR {
            name: "AppDesign".to_string(),
            tokens,
            classes,
            variants,
        }
    }

    /// Create a minimal ElementIR with a given tag and class name.
    fn make_element(tag: &str, class_name: Option<&str>) -> ElementIR {
        let mut props = FxHashMap::default();
        if let Some(cn) = class_name {
            props.insert(
                "class".to_string(),
                ExprIR::LiteralExpr {
                    value: serde_json::Value::String(cn.to_string()),
                },
            );
        }
        ElementIR {
            tag: tag.to_string(),
            props,
            bindings: FxHashMap::default(),
            events: FxHashMap::default(),
            style_classes: None,
            inline_style: None,
            children: Vec::new(),
            test_id: None,
            ref_name: None,
            construction_kind: ConstructionKind::Normal,
            class_map_bindings: FxHashMap::default(),
            source_line: 42,
            source_col: 0,
            motion: None,
            scroll: None,
            drag: None,
            hero: None,
        }
    }

    #[test]
    fn build_design_map_from_single_design() {
        let design = test_design();
        let mut module = ModuleIR::default();
        module.designs.push(design);
        let mut modules = FxHashMap::default();
        modules.insert("main".to_string(), module);

        let ir = IRProgram {
            modules,
            ..Default::default()
        };

        let map = build_design_map(&ir);
        assert_eq!(map.len(), 1, "should have one design");
        assert!(map.contains_key("AppDesign"), "should contain AppDesign");
    }

    #[test]
    fn build_design_map_from_multiple_modules() {
        let mut modules = FxHashMap::default();

        let mut mod_a = ModuleIR::default();
        mod_a.designs.push(DesignIR {
            name: "DesignA".to_string(),
            tokens: FxHashMap::default(),
            classes: vec![],
            variants: vec![],
        });
        modules.insert("mod_a".to_string(), mod_a);

        let mut mod_b = ModuleIR::default();
        mod_b.designs.push(DesignIR {
            name: "DesignB".to_string(),
            tokens: FxHashMap::default(),
            classes: vec![],
            variants: vec![],
        });
        modules.insert("mod_b".to_string(), mod_b);

        let ir = IRProgram {
            modules,
            ..Default::default()
        };

        let map = build_design_map(&ir);
        assert_eq!(map.len(), 2);
        assert!(map.contains_key("DesignA"));
        assert!(map.contains_key("DesignB"));
    }

    #[test]
    fn resolve_element_with_class_lookup() {
        let design = test_design();
        let mut design_map = FxHashMap::default();
        design_map.insert("AppDesign".to_string(), design);

        let el = make_element("div", Some("Card"));
        let resolved = resolve_element(&el, "Test.div.0", &design_map);

        assert_eq!(resolved.tag, "div");
        assert_eq!(resolved.path, "Test.div.0");
        assert!(resolved.classes.contains(&"Card".to_string()));
        assert_eq!(
            resolved.computed_css.get("background").map(|s| s.as_str()),
            Some("#ffffff")
        );
        assert_eq!(
            resolved
                .computed_css
                .get("border-radius")
                .map(|s| s.as_str()),
            Some("8px")
        );
        assert_eq!(resolved.source_line, 42);
    }

    #[test]
    fn resolve_element_no_class() {
        let design_map = FxHashMap::default();
        let el = make_element("span", None);

        let resolved = resolve_element(&el, "Test.span.0", &design_map);
        assert_eq!(resolved.tag, "span");
        assert!(resolved.classes.is_empty());
        assert!(resolved.computed_css.is_empty());
    }

    #[test]
    fn resolve_element_unknown_class_no_crash() {
        let design = test_design();
        let mut design_map = FxHashMap::default();
        design_map.insert("AppDesign".to_string(), design);

        let el = make_element("div", Some("NonExistentClass"));
        let resolved = resolve_element(&el, "Test.div.0", &design_map);

        assert!(resolved.classes.contains(&"NonExistentClass".to_string()));
        // Non-existent class should produce no CSS properties
        assert!(resolved.computed_css.is_empty());
    }

    #[test]
    fn resolve_element_with_html_attrs() {
        let design_map = FxHashMap::default();
        let mut props = FxHashMap::default();
        props.insert(
            "id".to_string(),
            ExprIR::LiteralExpr {
                value: serde_json::Value::String("my-id".to_string()),
            },
        );
        props.insert(
            "placeholder".to_string(),
            ExprIR::LiteralExpr {
                value: serde_json::Value::String("Enter text".to_string()),
            },
        );
        // Non-html attr should be skipped
        props.insert(
            "data-custom".to_string(),
            ExprIR::LiteralExpr {
                value: serde_json::Value::String("custom".to_string()),
            },
        );

        let el = ElementIR {
            props,
            ..make_element("input", Some("input-field"))
        };

        let resolved = resolve_element(&el, "Test.input.0", &design_map);
        assert_eq!(
            resolved.html_attrs.get("id").map(|s| s.as_str()),
            Some("my-id")
        );
        assert_eq!(
            resolved.html_attrs.get("placeholder").map(|s| s.as_str()),
            Some("Enter text")
        );
        // data-custom is not in the html_attr list and doesn't start with "on", so it's skipped
        assert!(resolved.html_attrs.get("data-custom").is_none());
    }

    #[test]
    fn resolve_element_with_bindings() {
        let design_map = FxHashMap::default();
        let mut bindings = FxHashMap::default();
        bindings.insert(
            "value".to_string(),
            SignalRefIR {
                owner: None,
                name: "count".to_string(),
            },
        );

        let el = ElementIR {
            bindings,
            ..make_element("input", None)
        };

        let resolved = resolve_element(&el, "Test.input.0", &design_map);
        assert_eq!(
            resolved.bindings.get("value").map(|s| s.as_str()),
            Some("count")
        );
    }

    #[test]
    fn resolve_element_with_events() {
        let design_map = FxHashMap::default();
        let mut events = FxHashMap::default();
        events.insert(
            "onClick".to_string(),
            EventHandlerIR::FnRef {
                r#ref: "handleClick".to_string(),
                params: vec![],
                event_data: None,
                used_fields: None,
                needs_prevent_default: false,
                needs_stop_propagation: false,
            },
        );
        events.insert(
            "onChange".to_string(),
            EventHandlerIR::StoreAction {
                r#ref: "FormStore.update".to_string(),
                event_data: None,
            },
        );
        // InlineFn handlers have no simple name and should be skipped
        events.insert(
            "onFocus".to_string(),
            EventHandlerIR::InlineFn {
                body: BlockIR { statements: vec![] },
                params: vec![],
                event_data: None,
                used_fields: None,
                needs_prevent_default: false,
                needs_stop_propagation: false,
            },
        );

        let el = ElementIR {
            events,
            ..make_element("button", None)
        };

        let resolved = resolve_element(&el, "Test.button.0", &design_map);
        assert_eq!(
            resolved.events.get("click").map(|s| s.as_str()),
            Some("handleClick")
        );
        assert_eq!(
            resolved.events.get("change").map(|s| s.as_str()),
            Some("FormStore.update")
        );
        // InlineFn should not appear in events
        assert!(resolved.events.get("focus").is_none());
    }

    #[test]
    fn resolve_view_tree_single_element() {
        let design_map = FxHashMap::default();
        let el = make_element("div", Some("root"));
        let tree = ViewTree::Element(el);

        let mut out = Vec::new();
        resolve_view_tree(&tree, "App", 0, &design_map, &mut out);

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].tag, "div");
        assert_eq!(out[0].path, "App.div.0");
    }

    #[test]
    fn element_children_paths_are_correct() {
        let design_map = FxHashMap::default();
        let child = ViewTree::Element(make_element("span", None));
        let parent_el = ElementIR {
            children: vec![child],
            ..make_element("div", None)
        };
        let tree = ViewTree::Element(parent_el);

        let mut out = Vec::new();
        resolve_view_tree(&tree, "App", 0, &design_map, &mut out);

        // Two elements: parent div and child span
        assert_eq!(out.len(), 2, "should have parent and child");

        // Parent should have child path listed
        let parent = &out[0];
        assert_eq!(parent.path, "App.div.0");
        assert_eq!(parent.children.len(), 1);
        assert_eq!(parent.children[0], "App.div.0.span.0");

        // Child path
        assert_eq!(out[1].path, "App.div.0.span.0");
    }

    #[test]
    fn build_design_summary_from_design() {
        let design = test_design();
        let summary = build_design_summary(&design);

        assert_eq!(summary.name, "AppDesign");
        assert_eq!(summary.tokens.len(), 2);
        assert_eq!(summary.classes.len(), 1);
        assert_eq!(summary.classes[0].name, "Card");
        assert_eq!(summary.variants.len(), 1);
        assert_eq!(summary.variants[0].name, "Button");
        assert_eq!(summary.variants[0].dimensions.len(), 2);
    }

    #[test]
    fn resolve_styles_from_ir_with_component() {
        let design = test_design();
        let mut design_map = FxHashMap::default();
        design_map.insert("AppDesign".to_string(), design);

        // Build a component with a view
        let el = make_element("div", Some("Card"));
        let el_children = el.children;
        let el_with_children = ElementIR {
            children: el_children,
            ..make_element("div", Some("Card"))
        };
        let view = ViewTree::Element(el_with_children);

        let comp = ComponentIR {
            name: "MyComponent".to_string(),
            view: Some(view),
            ..ComponentIR::default()
        };

        let mut components = FxHashMap::default();
        components.insert("MyComponent".to_string(), comp);

        let mut module = ModuleIR::default();
        module.name = "main".to_string();
        module.components = components;

        let mut modules = FxHashMap::default();
        modules.insert("main".to_string(), module);

        let ir = IRProgram {
            modules,
            ..Default::default()
        };

        let result = resolve_styles_from_ir(&ir, &design_map);
        assert!(result.success, "should succeed");
        assert!(
            result.components.contains_key("MyComponent"),
            "should contain MyComponent"
        );

        let elements = result.components.get("MyComponent").unwrap();
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].tag, "div");
        assert_eq!(elements[0].classes.len(), 1);
        assert!(elements[0].classes.contains(&"Card".to_string()));
    }

    #[test]
    fn resolve_styles_from_ir_skips_component_without_view() {
        let design_map = FxHashMap::default();

        let comp = ComponentIR {
            name: "NoView".to_string(),
            view: None,
            ..ComponentIR::default()
        };

        let mut components = FxHashMap::default();
        components.insert("NoView".to_string(), comp);

        let mut module = ModuleIR::default();
        module.components = components;

        let mut modules = FxHashMap::default();
        modules.insert("main".to_string(), module);

        let ir = IRProgram {
            modules,
            ..Default::default()
        };

        let result = resolve_styles_from_ir(&ir, &design_map);
        assert!(result.success);
        // Component without view should not appear in output
        assert!(result.components.is_empty());
    }

    #[test]
    fn child_tag_returns_correct_values() {
        assert_eq!(
            child_tag(&ViewTree::Element(ElementIR {
                tag: "section".to_string(),
                ..make_element("section", None)
            })),
            "section"
        );
        assert_eq!(
            child_tag(&ViewTree::Conditional(Box::new(ConditionalIntent {
                branches: vec![],
                fallback: None,
                transition_mode: None,
            }))),
            "if"
        );
        assert_eq!(
            child_tag(&ViewTree::ListRendering(Box::new(ListRenderingIntent {
                iterable: ExprIR::LiteralExpr {
                    value: serde_json::Value::Array(vec![])
                },
                item_binding: BindingIR {
                    name: "item".to_string(),
                    r#type: None,
                },
                body: None,
                key_expr: None,
                empty_body: None,
            }))),
            "for"
        );
        assert_eq!(
            child_tag(&ViewTree::Interpolation(Box::new(ExprIR::LiteralExpr {
                value: serde_json::Value::Null
            }))),
            "interp"
        );
        assert_eq!(child_tag(&ViewTree::Text("hello".to_string())), "text");
        assert_eq!(
            child_tag(&ViewTree::Slot(Some("default".to_string()))),
            "slot"
        );
    }
}
