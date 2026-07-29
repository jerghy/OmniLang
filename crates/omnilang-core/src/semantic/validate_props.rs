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
use rustc_hash::FxHashMap;

/// Validate required props for all component instantiations in the IR program.
///
/// Returns a list of diagnostics for each missing required prop found.
pub fn validate_required_props(program: &IRProgram) -> Vec<Diagnostic> {
    let mut errors = Vec::new();
    let component_map: FxHashMap<&str, &ComponentIR> = program.iter_all_components().collect();

    for (_comp_name, comp) in program.iter_all_components() {
        if let Some(ref view) = comp.view {
            validate_view_tree(view, &component_map, &mut errors);
        }
    }

    errors
}

/// Recursively walk a ViewTree, checking every Element for missing required props.
fn validate_view_tree(
    view: &ViewTree,
    component_map: &FxHashMap<&str, &ComponentIR>,
    errors: &mut Vec<Diagnostic>,
) {
    match view {
        ViewTree::Element(el) => {
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
                validate_view_tree(child, component_map, errors);
            }
        }
        ViewTree::Conditional(cond) => {
            for branch in &cond.branches {
                if let Some(ref body) = branch.body {
                    validate_view_tree(body, component_map, errors);
                }
            }
            if let Some(ref fallback) = cond.fallback {
                validate_view_tree(fallback, component_map, errors);
            }
        }
        ViewTree::ListRendering(list) => {
            if let Some(ref body) = list.body {
                validate_view_tree(body, component_map, errors);
            }
            if let Some(ref empty) = list.empty_body {
                validate_view_tree(empty, component_map, errors);
            }
        }
        ViewTree::Fragment(children) => {
            for child in children {
                validate_view_tree(child, component_map, errors);
            }
        }
        // Interpolation, Text, Slot — no component instances possible
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;
    use crate::semantic::analyze;

    /// Helper: compile source and return all diagnostics (parse + analysis + lowering + prop validation).
    fn compile_to_diagnostics(source: &str) -> Vec<Diagnostic> {
        let tokens = tokenize(source);
        let program = parse(tokens, source).expect("parse should succeed");
        let mut semantics = analyze(&program);
        let _ir = crate::ir::lower(&program, &mut semantics);
        semantics.diagnostics
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
