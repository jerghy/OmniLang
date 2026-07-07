//! Multi-file validation for OmniLang.
//!
//! Validates cross-file imports, visibility, cycle detection, and
//! per-file consistency (e.g., omniview marker matching UI declarations).

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::error::{Diagnostic, DiagnosticKind, Severity};
use crate::parser::ast::{ImportStmt, Program, Visibility};

/// The marker library import that identifies a file as an OmniView (view) file.
const OMNIVIEW_MARKER: &str = "omniview";

/// Build a file-level diagnostic (line/col = 0) for multi-file validation.
fn make_diag(severity: Severity, kind: DiagnosticKind, file: &str, message: String) -> Diagnostic {
    Diagnostic {
        severity,
        kind,
        message,
        file: Some(file.to_string()),
        line: 0,
        col: 0,
        end_line: None,
        end_col: None,
        suggestion: None,
    }
}

/// Result of multi-file validation.
#[derive(Debug, Serialize, Deserialize)]
pub struct MultiFileResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<Diagnostic>,
}

/// Validate multiple .omni source files with cross-file checks.
///
/// The Rust-native `validate_multi` receives:
///   - `sources`: HashMap<ResolvedAbsolutePath, SourceText>
///   - `path_mapping`: HashMap<OriginalImportPath, ResolvedAbsolutePath>
///
/// It parses all files, resolves cross-file imports using the mapping,
/// validates visibility and cycles, then returns results for downstream codegen.
#[must_use]
pub fn validate_multi(
    sources: HashMap<String, String>,
    path_mapping: HashMap<String, String>,
) -> MultiFileResult {
    let mut errors: Vec<Diagnostic> = Vec::new();
    let mut programs: HashMap<String, Arc<Program>> = HashMap::new();

    // Phase 1: Parse all sources
    for (path, source) in &sources {
        let tokens = crate::lexer::tokenize(source);
        match crate::parser::parse(&tokens, source) {
            Ok(program) => {
                programs.insert(path.clone(), Arc::new(program));
            }
            Err(e) => {
                let diag: Diagnostic = e.into();
                errors.push(diag.with_file(path));
            }
        }
    }

    if !errors.is_empty() {
        return MultiFileResult {
            success: false,
            errors,
        };
    }

    validate_multi_ref(&programs, &path_mapping)
}

/// Cross-file validation using already-parsed programs.
/// Avoids re-parsing when callers already have parsed ASTs.
#[must_use]
pub fn validate_multi_ref(
    programs: &HashMap<String, Arc<Program>>,
    path_mapping: &HashMap<String, String>,
) -> MultiFileResult {
    let mut errors: Vec<Diagnostic> = Vec::new();

    // Phase 2: Per-file consistency checks (omniview double-check)
    for (path, program) in programs {
        if let Err(file_errors) = validate_file_consistency(path, program) {
            errors.extend(file_errors);
        }
    }

    // Phase 3: Cross-file import validation
    if let Err(more_errors) = validate_cross_file(programs, path_mapping) {
        errors.extend(more_errors);
    }

    // Phase 4: Cycle detection
    let cycles = crate::semantic::cycles::detect_cycles(programs, path_mapping);
    for cycle in &cycles {
        let file = cycle.first().map(|s| s.as_str()).unwrap_or("unknown");
        errors.push(Diagnostic {
            severity: Severity::Error,
            kind: DiagnosticKind::ECycle,
            message: format!("circular import detected: {}", cycle.join(" → ")),
            file: Some(file.to_string()),
            line: 0,
            col: 0,
            end_line: None,
            end_col: None,
            suggestion: None,
        });
    }

    MultiFileResult {
        success: errors.is_empty(),
        errors,
    }
}

/// Check per-file consistency: omniview marker must match file contents.
fn validate_file_consistency(file_path: &str, program: &Program) -> Result<(), Vec<Diagnostic>> {
    let mut errors = Vec::new();

    let has_omniview_marker = program.modules.values().any(|module| {
        module
            .imports
            .iter()
            .any(|imp| matches!(imp, ImportStmt::Lib(name) if name == OMNIVIEW_MARKER))
    });

    let has_ui_declarations = program.modules.values().any(|module| {
        !module.components.is_empty() || !module.stores.is_empty() || !module.themes.is_empty()
    });

    if !has_omniview_marker && has_ui_declarations {
        errors.push(Diagnostic {
            severity: Severity::Warning,
            kind: DiagnosticKind::WOmniviewMarker,
            message: "has UI declarations (component/store/theme) but missing 'import omniview;'. \
             Add 'import omniview;' to mark this as a view file."
                .to_string(),
            file: Some(file_path.to_string()),
            line: 0,
            col: 0,
            end_line: None,
            end_col: None,
            suggestion: Some("Add 'import omniview;' at the top of the file.".to_string()),
        });
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum CrossFileSymbolKind {
    Function,
    Struct,
    TypeAlias,
    Component,
    Store,
    ReExport(String),
}

/// Build pub-visible symbol tables for cross-file validation.
fn build_pub_symbols(
    programs: &HashMap<String, Arc<Program>>,
) -> HashMap<String, HashMap<String, CrossFileSymbolKind>> {
    let mut pub_symbols: HashMap<String, HashMap<String, CrossFileSymbolKind>> = HashMap::new();

    for (path, program) in programs {
        let mut symbols: HashMap<String, CrossFileSymbolKind> = HashMap::new();

        for module in program.modules.values() {
            for func in &module.functions {
                if func.visibility == Visibility::Public {
                    symbols.insert(func.name.clone(), CrossFileSymbolKind::Function);
                }
            }
            for strukt in &module.structs {
                if strukt.visibility == Visibility::Public {
                    symbols.insert(strukt.name.clone(), CrossFileSymbolKind::Struct);
                }
            }
            for ta in &module.type_aliases {
                if ta.visibility == Visibility::Public {
                    symbols.insert(ta.name.clone(), CrossFileSymbolKind::TypeAlias);
                }
            }
            for comp in &module.components {
                symbols.insert(comp.name.clone(), CrossFileSymbolKind::Component);
            }
            for store in &module.stores {
                symbols.insert(store.name.clone(), CrossFileSymbolKind::Store);
            }
            // Re-exports
            for exp in &module.exports {
                for name in &exp.names {
                    symbols.insert(
                        name.clone(),
                        CrossFileSymbolKind::ReExport(exp.path.clone()),
                    );
                }
            }
        }

        pub_symbols.insert(path.clone(), symbols);
    }

    pub_symbols
}

fn validate_cross_file(
    programs: &HashMap<String, Arc<Program>>,
    path_mapping: &HashMap<String, String>,
) -> Result<(), Vec<Diagnostic>> {
    let mut errors: Vec<Diagnostic> = Vec::new();
    let pub_symbols = build_pub_symbols(programs);

    for (file_path, program) in programs {
        for module in program.modules.values() {
            for import in &module.imports {
                match import {
                    ImportStmt::Named(named) => {
                        // stdlib imports (std/http, std/json, etc.) are resolved
                        // by the IR/stdlib layer, not via file path mapping
                        if named.path.starts_with("std/") {
                            continue;
                        }
                        if let Some(target) = path_mapping.get(&named.path) {
                            errors.extend(validate_named_import(
                                file_path,
                                &named.names,
                                target,
                                &pub_symbols,
                            ));
                        } else {
                            errors.push(make_diag(
                                Severity::Error,
                                DiagnosticKind::EImport,
                                file_path,
                                format!("could not resolve import path '{}'", named.path),
                            ));
                        }
                    }
                    ImportStmt::Namespace(ns) => {
                        if ns.path.starts_with("std/") {
                            continue;
                        }
                        if let Some(target) = path_mapping.get(&ns.path) {
                            if !programs.contains_key(target.as_str()) {
                                errors.push(make_diag(Severity::Error, DiagnosticKind::EImport, file_path, format!(
                                    "imported file '{}' (resolved to '{}') not found in compilation",
                                    ns.path, target
                                )));
                            }
                        } else {
                            errors.push(make_diag(
                                Severity::Error,
                                DiagnosticKind::EImport,
                                file_path,
                                format!("could not resolve import path '{}'", ns.path),
                            ));
                        }
                    }
                    ImportStmt::TypeOnly(ti) => {
                        if ti.path.starts_with("std/") {
                            continue;
                        }
                        if let Some(target) = path_mapping.get(&ti.path) {
                            errors.extend(validate_named_import(
                                file_path,
                                &ti.names,
                                target,
                                &pub_symbols,
                            ));
                        } else {
                            errors.push(make_diag(
                                Severity::Error,
                                DiagnosticKind::EImport,
                                file_path,
                                format!("could not resolve import path '{}'", ti.path),
                            ));
                        }
                    }
                    ImportStmt::Lib(_) => {
                        // 'import omniview;' — validated in validate_file_consistency
                    }
                }
            }
        }
    }

    // Enforce logic→view boundary
    for (file_path, program) in programs {
        for module in program.modules.values() {
            let is_logic = !module
                .imports
                .iter()
                .any(|imp| matches!(imp, ImportStmt::Lib(name) if name == OMNIVIEW_MARKER));
            if !is_logic {
                continue;
            }
            for import in &module.imports {
                let target_path: Option<&String> = match import {
                    ImportStmt::Named(n) => path_mapping.get(&n.path),
                    ImportStmt::Namespace(ns) => path_mapping.get(&ns.path),
                    ImportStmt::TypeOnly(t) => path_mapping.get(&t.path),
                    ImportStmt::Lib(_) => None,
                };
                if let Some(target) = target_path {
                    if let Some(target_prog) = programs.get(target.as_str()) {
                        let target_is_view = target_prog.modules.values().any(|tm| {
                            tm.imports.iter().any(
                                |imp| matches!(imp, ImportStmt::Lib(name) if name == OMNIVIEW_MARKER),
                            )
                        });
                        if target_is_view {
                            errors.push(make_diag(
                                Severity::Error,
                                DiagnosticKind::EBoundary,
                                file_path,
                                format!("logic file cannot import view file '{}'", target),
                            ));
                        }
                    }
                }
            }
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

    fn make_sources(files: Vec<(&str, &str)>) -> HashMap<String, String> {
        files
            .into_iter()
            .map(|(path, content)| (path.to_string(), content.to_string()))
            .collect()
    }

    /// Helper: build sources and derive their path mapping, then validate.
    fn validate_sources(sources: HashMap<String, String>) -> MultiFileResult {
        let path_mapping = crate::path_mapping::derive_path_mapping(&sources);
        validate_multi(sources, path_mapping)
    }

    #[test]
    fn validate_single_file_no_issues() {
        let sources = make_sources(vec![("main.omni", "fn main() -> i32 { return 42; }")]);
        let result = validate_sources(sources);
        assert!(result.success, "single function file should validate ok");
        assert!(
            result.errors.is_empty(),
            "expected no errors, got: {:?}",
            result.errors
        );
    }

    #[test]
    fn validate_cross_file_import_resolves() {
        let sources = make_sources(vec![
            ("a.omni", "pub fn helper() -> i32 { return 1; }"),
            (
                "b.omni",
                "import { helper } from \"./a.omni\"; fn main() -> i32 { return helper(); }",
            ),
        ]);
        let result = validate_sources(sources);
        assert!(
            result.success,
            "cross-file import of pub symbol should succeed, got errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn validate_cross_file_private_symbol_fails() {
        // helper is NOT pub — should produce visibility error
        let sources = make_sources(vec![
            ("a.omni", "fn helper() -> i32 { return 1; }"),
            (
                "b.omni",
                "import { helper } from \"./a.omni\"; fn main() -> i32 { return helper(); }",
            ),
        ]);
        let result = validate_sources(sources);
        assert!(!result.success, "import of private symbol should fail");
        let has_vis_error = result
            .errors
            .iter()
            .any(|d| d.kind == DiagnosticKind::EVisibility);
        assert!(
            has_vis_error,
            "expected E_VISIBILITY error, got kinds: {:?}",
            result.errors.iter().map(|d| &d.kind).collect::<Vec<_>>()
        );
    }

    #[test]
    fn validate_unresolved_import_path_fails() {
        let sources = make_sources(vec![(
            "main.omni",
            "import { stuff } from \"./missing.omni\"; fn main() {}",
        )]);
        let path_mapping = make_sources(vec![("main.omni", "main.omni")]);
        // Don't include "./missing.omni" in the path mapping, so it can't be resolved
        let result = validate_multi(sources, path_mapping);
        assert!(!result.success, "unresolvable import should fail");
        let has_import_error = result
            .errors
            .iter()
            .any(|d| d.kind == DiagnosticKind::EImport);
        assert!(
            has_import_error,
            "expected E_IMPORT error, got kinds: {:?}",
            result.errors.iter().map(|d| &d.kind).collect::<Vec<_>>()
        );
    }
}

fn validate_named_import(
    importing_file: &str,
    names: &[String],
    target_path: &str,
    pub_symbols: &HashMap<String, HashMap<String, CrossFileSymbolKind>>,
) -> Vec<Diagnostic> {
    let mut errors = Vec::new();

    let target_symbols = match pub_symbols.get(target_path) {
        Some(s) => s,
        None => {
            errors.push(make_diag(
                Severity::Error,
                DiagnosticKind::EImport,
                importing_file,
                format!("imported file '{}' not found in compilation", target_path),
            ));
            return errors;
        }
    };

    for name in names {
        if !target_symbols.contains_key(name) {
            errors.push(make_diag(
                Severity::Error,
                DiagnosticKind::EVisibility,
                importing_file,
                format!("'{}' is not pub-visible in '{}'", name, target_path),
            ));
        }
    }

    errors
}
