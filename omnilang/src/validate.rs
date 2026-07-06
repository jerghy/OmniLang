//! Multi-file validation for OmniLang.
//!
//! Validates cross-file imports, visibility, cycle detection, and
//! per-file consistency (e.g., omniview marker matching UI declarations).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::{Diagnostic, Severity};
use crate::parser::ast::{ImportStmt, Program, Visibility};

/// Build a file-level diagnostic (line/col = 0) for multi-file validation.
fn make_diag(severity: Severity, code: &str, file: &str, message: String) -> Diagnostic {
    Diagnostic {
        severity,
        code: code.to_string(),
        message,
        file: Some(file.to_string()),
        line: 0,
        col: 0,
        end_line: 0,
        end_col: 0,
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
/// Python does all file I/O and path resolution, then passes:
///   - `sources`: HashMap<ResolvedAbsolutePath, SourceText>
///   - `path_mapping`: HashMap<OriginalImportPath, ResolvedAbsolutePath>
///
/// The compiler parses all files, resolves cross-file imports using the mapping,
/// validates visibility and cycles. Codegen is done separately in Python.
pub fn validate_multi(
    sources: HashMap<String, String>,
    path_mapping: HashMap<String, String>,
) -> MultiFileResult {
    let mut errors: Vec<Diagnostic> = Vec::new();
    let mut programs: HashMap<String, Program> = HashMap::new();

    // Phase 1: Parse all sources
    for (path, source) in &sources {
        let tokens = crate::lexer::tokenize(source);
        match crate::parser::parse(&tokens, source) {
            Ok(program) => {
                programs.insert(path.clone(), program);
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

    // Phase 2: Per-file consistency checks (omniview double-check)
    for (path, program) in &programs {
        if let Err(file_errors) = validate_file_consistency(path, program) {
            errors.extend(file_errors);
        }
    }

    // Phase 3: Cross-file import validation
    if let Err(more_errors) = validate_cross_file(&programs, &path_mapping) {
        errors.extend(more_errors);
    }

    // Phase 4: Cycle detection
    let cycles = crate::semantic::cycles::detect_cycles(&programs, &path_mapping);
    for cycle in &cycles {
        let file = cycle.first().map(|s| s.as_str()).unwrap_or("unknown");
        errors.push(Diagnostic {
            severity: Severity::Error,
            code: "E_CYCLE".to_string(),
            message: format!("circular import detected: {}", cycle.join(" → ")),
            file: Some(file.to_string()),
            line: 0,
            col: 0,
            end_line: 0,
            end_col: 0,
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
            .any(|imp| matches!(imp, ImportStmt::Lib(name) if name == "omniview"))
    });

    let has_ui_declarations = program.modules.values().any(|module| {
        !module.components.is_empty() || !module.stores.is_empty() || !module.themes.is_empty()
    });

    if !has_omniview_marker && has_ui_declarations {
        errors.push(Diagnostic {
            severity: Severity::Warning,
            code: "W_OMNIVIEW_MARKER".to_string(),
            message: "has UI declarations (component/store/theme) but missing 'import omniview;'. \
             Add 'import omniview;' to mark this as a view file."
                .to_string(),
            file: Some(file_path.to_string()),
            line: 0,
            col: 0,
            end_line: 0,
            end_col: 0,
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
#[allow(dead_code)]
enum CrossFileSymbolKind {
    Function,
    Struct,
    TypeAlias,
    Component,
    Store,
    Theme, // reserved for future theme support
    ReExport(String),
}

/// Build pub-visible symbol tables for cross-file validation.
fn build_pub_symbols(
    programs: &HashMap<String, Program>,
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
    programs: &HashMap<String, Program>,
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
                                "E_IMPORT",
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
                                errors.push(make_diag(Severity::Error, "E_IMPORT", file_path, format!(
                                    "imported file '{}' (resolved to '{}') not found in compilation",
                                    ns.path, target
                                )));
                            }
                        } else {
                            errors.push(make_diag(
                                Severity::Error,
                                "E_IMPORT",
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
                                "E_IMPORT",
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
                .any(|imp| matches!(imp, ImportStmt::Lib(name) if name == "omniview"));
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
                                |imp| matches!(imp, ImportStmt::Lib(name) if name == "omniview"),
                            )
                        });
                        if target_is_view {
                            errors.push(make_diag(
                                Severity::Error,
                                "E_BOUNDARY",
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
                "E_IMPORT",
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
                "E_VISIBILITY",
                importing_file,
                format!("'{}' is not pub-visible in '{}'", name, target_path),
            ));
        }
    }

    errors
}
