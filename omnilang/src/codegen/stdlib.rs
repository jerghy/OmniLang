//! Standard library integration for code generation.
//!
//! Provides functions to format stdlib function calls and validate
//! that required stdlib modules are available on the target platform.

use rustc_hash::FxHashMap;
use std::sync::LazyLock;

use crate::codegen::Target;
use crate::error::{Diagnostic, DiagnosticKind, Severity};

/// Pre-built O(1) lookup map: fn_name -> (module, fn_name_in_registry).
static STDLIB_FN_MAP: LazyLock<FxHashMap<&'static str, (&'static str, &'static str)>> =
    LazyLock::new(|| {
        let mut m = FxHashMap::default();
        for f in crate::stdlib::registry() {
            // Insert under the full prefixed name (e.g., "http_get").
            m.insert(f.name, (f.module, f.name));
        }
        m
    });

/// Try to format a stdlib function call. Returns `Some(code)` if the call
/// matches a registered stdlib function, `None` otherwise (so backends fall
/// through to their normal CallExpr handling).
#[must_use]
pub fn try_format_stdlib_call(name: &str, args: &[String], target: Target) -> Option<String> {
    // O(1) lookup via pre-built map
    if let Some(&(module, fn_name)) = STDLIB_FN_MAP.get(name) {
        if let Some(entry) = crate::stdlib::lookup(module, fn_name) {
            if entry.available_targets.contains(&target) {
                return Some((entry.codegen)(args, target));
            }
        }
    }
    None
}

/// Validate that all stdlib imports in the IR are available on the given target.
pub fn validate_stdlib_imports(
    ir: &crate::ir::IRProgram,
    target: Target,
) -> Result<(), Vec<Diagnostic>> {
    let mut errors = Vec::new();
    for module in &ir.stdlib_imports {
        if !crate::stdlib::module_available(module, target) {
            let available = crate::stdlib::available_targets_for(module);
            let available_str: Vec<String> = available
                .iter()
                .map(|t| format!("{:?}", t).to_lowercase())
                .collect();
            errors.push(Diagnostic {
                severity: Severity::Error,
                kind: DiagnosticKind::ECodegen,
                message: format!(
                    "stdlib module 'std/{}' is not available on target '{:?}'. Available targets: {}",
                    module,
                    target,
                    available_str.join(", ")
                ),
                file: None,
                line: 0,
                col: 0,
                end_line: None,
                end_col: None,
                suggestion: None,
            });
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustc_hash::{FxHashMap, FxHashSet};
    #[test]
    fn test_try_format_stdlib_call_http_get_js() {
        let result = try_format_stdlib_call("http_get", &["\"url\"".into()], Target::JavaScript);
        assert!(result.is_some(), "http_get should be recognized");
        assert!(
            result.unwrap().contains("fetch"),
            "JS http_get should use fetch"
        );
    }

    #[test]
    fn test_try_format_stdlib_call_json_parse_js() {
        let result = try_format_stdlib_call("json_parse", &["text".into()], Target::JavaScript);
        assert!(result.is_some(), "json_parse should be recognized");
        assert_eq!(result.unwrap(), "JSON.parse(text)");
    }

    #[test]
    fn test_try_format_stdlib_call_json_stringify_python() {
        let result = try_format_stdlib_call("json_stringify", &["obj".into()], Target::Python);
        assert!(result.is_some(), "json_stringify should be recognized");
        assert_eq!(result.unwrap(), "json.dumps(obj)");
    }

    #[test]
    fn test_try_format_stdlib_call_not_found() {
        let result = try_format_stdlib_call("nonexistent_fn", &[], Target::JavaScript);
        assert!(result.is_none(), "Unknown fn should return None");
    }

    #[test]
    fn test_try_format_stdlib_call_math_abs_js() {
        let result = try_format_stdlib_call("math_abs", &["x".into()], Target::JavaScript);
        assert!(result.is_some(), "math_abs should be recognized");
        assert_eq!(result.unwrap(), "Math.abs(x)");
    }

    #[test]
    fn test_try_format_stdlib_call_console_log_js() {
        let result = try_format_stdlib_call("console_log", &["\"hi\"".into()], Target::JavaScript);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "console.log(\"hi\")");
    }

    #[test]
    fn test_validate_stdlib_imports_storage_on_rust_fails() {
        let ir = crate::ir::IRProgram {
            version: "1.0".into(),
            modules: FxHashMap::default(),
            source_map: FxHashMap::default(),
            stdlib_imports: FxHashSet::from_iter(["storage".to_string()]),
        };
        let result = validate_stdlib_imports(&ir, Target::Rust);
        assert!(result.is_err(), "storage on rust should fail");
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(
            errors[0].message.contains("not available"),
            "Error should mention not available. Got: {}",
            errors[0].message
        );
    }

    #[test]
    fn test_validate_stdlib_imports_all_valid() {
        let ir = crate::ir::IRProgram {
            version: "1.0".into(),
            modules: FxHashMap::default(),
            source_map: FxHashMap::default(),
            stdlib_imports: FxHashSet::from_iter(["http".to_string(), "json".to_string()]),
        };
        let result = validate_stdlib_imports(&ir, Target::JavaScript);
        assert!(
            result.is_ok(),
            "http+json on JS should be valid. Got: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_validate_stdlib_imports_empty() {
        let ir = crate::ir::IRProgram {
            version: "1.0".into(),
            modules: FxHashMap::default(),
            source_map: FxHashMap::default(),
            stdlib_imports: Default::default(),
        };
        let result = validate_stdlib_imports(&ir, Target::Rust);
        assert!(result.is_ok(), "Empty imports should always be valid");
    }
}
