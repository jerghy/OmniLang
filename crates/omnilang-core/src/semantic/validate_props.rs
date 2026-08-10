//! Validate that required component props are provided at call sites.
//!
//! Components declare props with optional defaults:
//!   prop title: String = "Untitled"  // optional (has default)
//!   prop count: i32                   // required (no default)
//!
//! This pass checks that every component instantiation provides all
//! required props. Missing required props produce Diagnostic errors.

use crate::error::{Diagnostic, DiagnosticKind, Severity};
use crate::ir::{ComponentIR, IRProgram, ViewTree};
use crate::lexer::Span;
use rustc_hash::FxHashMap;

/// Validate required props for all component instantiations in the IR program.
///
/// Returns a list of diagnostics for each missing required prop found.
pub fn validate_required_props(program: &IRProgram) -> Vec<Diagnostic> {
    let mut errors = Vec::new();
    let component_map: FxHashMap<&str, &ComponentIR> = program.iter_all_components().collect();

    for (_comp_name, comp) in program.iter_all_components() {
        if let Some(ref view) = comp.view {
            validate_view_tree(view, comp, &component_map, &mut errors);
        }
    }

    errors
}

/// Recursively walk a ViewTree, checking every Element for missing required props.
fn validate_view_tree(
    view: &ViewTree,
    component: &ComponentIR,
    component_map: &FxHashMap<&str, &ComponentIR>,
    errors: &mut Vec<Diagnostic>,
) {
    match view {
        ViewTree::Element(el) => {
            // v4 domain HIGH-2: `theme="..."` is deleted fictional syntax
            // (ADR-056 D1 — theme switching uses @themes declarations + @theme
            // signal bindings). The language cannot distinguish fictional
            // syntax from a custom component's legitimate `theme` prop — warn
            // on known DOM tags only (custom components are exempt); the
            // diagnostics table in the design doc reflects this honestly.
            if el.props.contains_key("theme") && is_known_html_tag(&el.tag) {
                errors.push(warning_diag(
                    component,
                    el.source_span,
                    "`theme=\"...\"` 是已删除的虚构语法（主题切换请用 @themes 声明 + @theme 信号绑定）——此属性在已知 DOM 标签上无任何效果".to_string(),
                ));
            }
            // Check if this element is a component instantiation
            if let Some(child_comp) = component_map.get(el.tag.as_str()) {
                for prop_name in child_comp.props.keys() {
                    if !child_comp.prop_defaults.contains_key(prop_name)
                        && !el.props.contains_key(prop_name)
                    {
                        let (line, col) = el.source_span.map(|s| (s.line, s.col)).unwrap_or((0, 0));
                        errors.push(Diagnostic {
                            severity: Severity::Error,
                            kind: DiagnosticKind::E0011,
                            message: format!(
                                "required prop '{}' not provided for component '{}'",
                                prop_name, el.tag
                            ),
                            file: None,
                            line,
                            col,
                            end_line: None,
                            end_col: None,
                            suggestion: Some(format!(
                                "add '{}' to this component instantiation",
                                prop_name
                            )),
                        });
                    }
                }
            }
            // Recurse into children
            for child in &el.children {
                validate_view_tree(child, component, component_map, errors);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    validate_view_tree(body, component, component_map, errors);
                }
            }
            if let Some(ref fallback) = cond.fallback {
                validate_view_tree(fallback, component, component_map, errors);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                validate_view_tree(body, component, component_map, errors);
            }
            if let Some(ref empty) = list.empty_body {
                validate_view_tree(empty, component, component_map, errors);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                validate_view_tree(child, component, component_map, errors);
            }
        }
        // Interpolation, Text, Slot — no component instances possible
        _ => {}
    }
}

/// Whether the tag is a known (built-in) DOM element.
///
/// Built-in PascalCase aliases (Card → div, Button → button, Text → span …)
/// are first normalized through codegen's shared `to_html_tag` mapping —
/// single source of truth, same codegen::shared import pattern as
/// `validate_event_names` / `motion`. Non-alias tags pass through unchanged.
/// The lowercase list mirrors `validate_semantic_bindings`'s
/// `binding_applies_to_tag`, extended with focusable/form tags (a, button,
/// input, textarea, select, option) and common flow tags. Custom components
/// (PascalCase, not aliases) never match — that non-overlap is the exemption
/// that keeps legitimate `theme` props on custom components warning-free.
fn is_known_html_tag(tag: &str) -> bool {
    let tag = crate::codegen::shared::html::to_html_tag(tag);
    matches!(
        tag.as_str(),
        "div"
            | "span"
            | "p"
            | "section"
            | "article"
            | "header"
            | "footer"
            | "nav"
            | "main"
            | "aside"
            | "ul"
            | "ol"
            | "li"
            | "form"
            | "label"
            | "img"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "table"
            | "tr"
            | "td"
            | "th"
            | "strong"
            | "em"
            | "b"
            | "i"
            | "blockquote"
            | "figure"
            | "figcaption"
            | "br"
            | "hr"
            | "a"
            | "button"
            | "input"
            | "textarea"
            | "select"
            | "option"
            | "code"
            | "pre"
            | "small"
            | "video"
            | "audio"
            | "canvas"
    )
}

/// Build a warning diagnostic at the element's source location (falling back
/// to the component's location), following the convention used by
/// `validate_semantic_bindings`.
fn warning_diag(component: &ComponentIR, span: Option<Span>, message: String) -> Diagnostic {
    let (line, col) = span
        .or(component.source_span)
        .map(|s| (s.line, s.col))
        .unwrap_or((0, 0));
    Diagnostic {
        severity: Severity::Warning,
        kind: DiagnosticKind::E0006,
        message,
        file: component.source_file.clone(),
        line,
        col,
        end_line: None,
        end_col: None,
        suggestion: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: compile source and return all diagnostics (parse + semantic
    /// analysis + lowering + program-level IR passes). Routes through
    /// `analyse_multi` with one synthetic file (same convention as
    /// `validate_semantic_bindings::analyse_src`) so required-prop validation
    /// — moved to `run_program_passes` on the merged IR (E0011 must fire for
    /// cross-file call sites too) — is exercised.
    fn compile_to_diagnostics(source: &str) -> Vec<Diagnostic> {
        let mut sources = FxHashMap::default();
        sources.insert("test.omni".to_string(), source.to_string());
        let path_mapping = FxHashMap::default();
        let output = crate::analyse::analyse_multi(&sources, &path_mapping);
        output.diagnostics
    }

    fn collect_prop_errors(diagnostics: &[Diagnostic]) -> Vec<&Diagnostic> {
        diagnostics
            .iter()
            .filter(|d| d.kind == DiagnosticKind::E0011)
            .collect()
    }

    #[test]
    fn detects_missing_required_prop() {
        let source = r#"
import omniview;
component Child {
    prop title: String = "Default"
    prop count: i32
    view { <div>{count}</div> }
}
component App {
    view { <Child title="Hello" /> }
}
"#;
        let diagnostics = compile_to_diagnostics(source);
        let prop_errors = collect_prop_errors(&diagnostics);
        assert_eq!(
            prop_errors.len(),
            1,
            "expected 1 prop error for missing 'count', got {}: {:?}",
            prop_errors.len(),
            prop_errors
        );
        let msg = &prop_errors[0].message;
        assert!(
            msg.contains("count"),
            "error should mention 'count', got: {msg}"
        );
        assert!(
            msg.contains("Child"),
            "error should mention component 'Child', got: {msg}"
        );
    }

    #[test]
    fn allows_optional_prop_omission() {
        let source = r#"
import omniview;
component Child {
    prop title: String = "Default"
    view { <div>{title}</div> }
}
component App {
    view { <Child /> }
}
"#;
        let diagnostics = compile_to_diagnostics(source);
        let prop_errors = collect_prop_errors(&diagnostics);
        assert!(
            prop_errors.is_empty(),
            "expected no prop errors for optional-only props, got: {:?}",
            prop_errors
        );
    }

    #[test]
    fn allows_all_required_props_provided() {
        let source = r#"
import omniview;
component Counter {
    prop count: i32
    view { <div>{count}</div> }
}
component App {
    view { <Counter count={42} /> }
}
"#;
        let diagnostics = compile_to_diagnostics(source);
        let prop_errors = collect_prop_errors(&diagnostics);
        assert!(
            prop_errors.is_empty(),
            "expected no prop errors when all required props provided, got: {:?}",
            prop_errors
        );
    }

    #[test]
    fn detects_multiple_missing_props() {
        let source = r#"
import omniview;
component Profile {
    prop name: String
    prop age: i32
    prop email: String = "unknown"
    view { <div>{name}</div> }
}
component App {
    view { <Profile /> }
}
"#;
        let diagnostics = compile_to_diagnostics(source);
        let prop_errors = collect_prop_errors(&diagnostics);
        assert_eq!(
            prop_errors.len(),
            2,
            "expected 2 prop errors for missing 'name' and 'age', got: {:?}",
            prop_errors
        );
        let msgs: Vec<&str> = prop_errors.iter().map(|d| d.message.as_str()).collect();
        assert!(
            msgs.iter().any(|m| m.contains("'name'")),
            "should mention 'name': {msgs:?}"
        );
        assert!(
            msgs.iter().any(|m| m.contains("'age'")),
            "should mention 'age': {msgs:?}"
        );
    }

    #[test]
    fn valid_component_no_props_no_errors() {
        let source = r#"
import omniview;
component Empty {
    view { <div>No props</div> }
}
"#;
        let diagnostics = compile_to_diagnostics(source);
        let prop_errors = collect_prop_errors(&diagnostics);
        assert!(
            prop_errors.is_empty(),
            "expected no prop errors for component without props, got: {:?}",
            prop_errors
        );
    }

    #[test]
    fn theme_prop_on_known_tag_warns() {
        // v4 domain HIGH-2: `theme="dark"` 是已删除虚构语法（ADR-056 D1）——
        // 文档宣称「自然解析错误」实为静默惰性属性。已知 DOM 标签上的
        // theme= 属性必须 warning（自定义组件豁免——合法 prop 不误伤）。
        let diagnostics = compile_to_diagnostics(
            "import omniview;\ncomponent App { view { <div theme=\"dark\" /> } }",
        );
        assert!(
            diagnostics.iter().any(|d| {
                d.severity == Severity::Warning
                    && d.message.contains("theme")
                    && d.message.contains("@themes")
            }),
            "无 theme= 警告：{:?}",
            diagnostics
        );
    }

    #[test]
    fn theme_prop_on_pascalcase_alias_tag_warns() {
        // Review round 1 (Important): built-in PascalCase aliases (Card →
        // div, Button → button, Text → span — codegen/shared/html.rs) are
        // treated as component calls by the parser but map to known DOM tags
        // at codegen. `theme=` on them must warn like any known tag —
        // otherwise the silent lazy attribute resurfaces on the alias surface.
        let diagnostics = compile_to_diagnostics(
            "import omniview;\ncomponent App { view { <Card theme=\"dark\" /> } }",
        );
        assert!(
            diagnostics.iter().any(|d| {
                d.severity == Severity::Warning
                    && d.message.contains("theme")
                    && d.message.contains("@themes")
            }),
            "无 theme= 警告（PascalCase 别名 <Card>）：{:?}",
            diagnostics
        );
    }

    #[test]
    fn known_tag_without_theme_prop_no_warning() {
        // Negative: a known tag without a theme prop must not warn (review
        // round 1, Minor).
        let diagnostics = compile_to_diagnostics(
            "import omniview;\ncomponent App { view { <div class=\"card\">hi</div> } }",
        );
        assert!(
            !diagnostics.iter().any(|d| d.message.contains("theme")),
            "无 theme prop 的已知标签不应有 theme 警告：{:?}",
            diagnostics
        );
    }

    #[test]
    fn theme_prop_on_custom_component_allowed() {
        // A custom component may legitimately declare a `theme` prop — the
        // language cannot distinguish deleted fictional syntax from a legal
        // prop, so only known DOM tags warn (custom components exempt).
        let diagnostics = compile_to_diagnostics(
            "import omniview;\n\
             component Modal { prop theme: String = \"\"; view { <div>{theme}</div> } }\n\
             component App { view { <Modal theme=\"dark\" /> } }",
        );
        assert!(
            !diagnostics.iter().any(|d| d.message.contains("theme")),
            "自定义组件 theme= 被误伤：{:?}",
            diagnostics
        );
    }

    #[test]
    fn nested_component_prop_validation() {
        let source = r#"
import omniview;
component Item {
    prop key: String
    view { <div>{key}</div> }
}
component List {
    view {
        <div>
            <Item key="a" />
            <Item key="b" />
            <Item />  {# missing 'key' #}
        </div>
    }
}
"#;
        let diagnostics = compile_to_diagnostics(source);
        let prop_errors = collect_prop_errors(&diagnostics);
        assert_eq!(
            prop_errors.len(),
            1,
            "expected 1 prop error for the third <Item /> without 'key', got: {:?}",
            prop_errors
        );
    }
}
