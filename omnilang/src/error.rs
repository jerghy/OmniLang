//! Unified error and diagnostic types for the OmniLang compiler.

use serde::{Deserialize, Serialize};

use crate::lexer::Span;

/// A user-facing diagnostic message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: String,
    pub message: String,
    /// Source file path (set for multi-file compiles; omitted for single-file).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    pub line: u32,
    pub col: u32,
    pub end_line: u32,
    pub end_col: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl Diagnostic {
    pub fn error(code: &str, message: &str, span: &Span) -> Self {
        Diagnostic {
            severity: Severity::Error,
            code: code.to_string(),
            message: message.to_string(),
            file: None,
            line: span.line,
            col: span.col,
            end_line: span.line,
            end_col: span.col + (span.end.saturating_sub(span.start)) as u32,
            suggestion: None,
        }
    }

    pub fn warning(code: &str, message: &str, span: &Span) -> Self {
        Diagnostic {
            severity: Severity::Warning,
            code: code.to_string(),
            message: message.to_string(),
            file: None,
            line: span.line,
            col: span.col,
            end_line: span.line,
            end_col: span.col + (span.end.saturating_sub(span.start)) as u32,
            suggestion: None,
        }
    }

    pub fn with_file(mut self, file: &str) -> Self {
        self.file = Some(file.to_string());
        self
    }

    pub fn with_suggestion(mut self, suggestion: &str) -> Self {
        self.suggestion = Some(suggestion.to_string());
        self
    }

    /// Convert this diagnostic to a JSON value for FFI / CLI output.
    /// Uses serde serialization (Diagnostic already derives Serialize).
    pub fn to_json_value(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_else(|_| {
            serde_json::json!({"severity":"error","code":"E_SERIALIZE","message":"failed to serialize diagnostic"})
        })
    }
}

/// Convert a slice of diagnostics to a JSON array for FFI / CLI output.
pub fn diagnostics_to_json(diagnostics: &[Diagnostic]) -> Vec<serde_json::Value> {
    diagnostics.iter().map(|d| d.to_json_value()).collect()
}

/// A raw parse error before conversion to Diagnostic.
#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl From<ParseError> for Diagnostic {
    fn from(e: ParseError) -> Self {
        Diagnostic::error("E0001", &e.message, &e.span)
    }
}
