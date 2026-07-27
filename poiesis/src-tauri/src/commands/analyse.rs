//! Analyse-source command for server-side semantic analysis.

use serde::{Deserialize, Serialize};

use crate::MAX_SOURCE_SIZE;

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalyseResult {
    pub success: bool,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: String,
    pub code: String,
    pub message: String,
    pub file: String,
    pub line: u32,
    pub col: u32,
}

#[tauri::command]
pub(crate) fn analyse_source(source: String, file_path: String) -> AnalyseResult {
    if source.len() > MAX_SOURCE_SIZE {
        return AnalyseResult {
            success: false,
            diagnostics: vec![Diagnostic {
                severity: "error".to_string(),
                code: "INPUT_TOO_LARGE".to_string(),
                message: format!("Source exceeds maximum size of {} bytes", MAX_SOURCE_SIZE),
                file: file_path,
                line: 0,
                col: 0,
            }],
        };
    }
    let tokens = omnilang::lexer::tokenize(&source);
    let program = match omnilang::parser::parse(tokens, &source) {
        Ok(p) => p,
        Err(e) => {
            return AnalyseResult {
                success: false,
                diagnostics: vec![Diagnostic {
                    severity: "error".to_string(),
                    code: "PARSE_ERROR".to_string(),
                    message: e.message,
                    file: file_path,
                    line: e.span.line,
                    col: e.span.col,
                }],
            };
        }
    };

    let semantics = omnilang::semantic::analyze(&program);
    let diagnostics: Vec<Diagnostic> = semantics
        .diagnostics
        .iter()
        .map(|d| Diagnostic {
            severity: match d.severity {
                omnilang::error::Severity::Error => "error".to_string(),
                omnilang::error::Severity::Warning => "warning".to_string(),
                omnilang::error::Severity::Info => "info".to_string(),
            },
            code: d.kind.code_str().to_string(),
            message: d.message.clone(),
            file: file_path.clone(),
            line: d.line,
            col: d.col,
        })
        .collect();

    let has_errors = diagnostics.iter().any(|d| d.severity == "error");
    AnalyseResult {
        success: !has_errors,
        diagnostics,
    }
}
