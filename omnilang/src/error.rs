//! Unified error and diagnostic types for the OmniLang compiler.

use serde::de::{self, Deserializer, Visitor};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};
use std::fmt;

use crate::lexer::Span;

// ═══════════════════════════════════════════════════════════
// DiagnosticKind
// ═══════════════════════════════════════════════════════════

/// Typed diagnostic kind — replaces string-based error codes.
///
/// The `Display` impl returns a human-readable description (e.g. "parse error").
/// Use `code_str()` to get the backward-compatible error code string (e.g. "E0001").
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DiagnosticKind {
    #[error("parse error")]
    E0001,
    #[error("duplicate symbol")]
    E0002,
    #[error("type mismatch")]
    E0003,
    #[error("invalid type")]
    E0004,
    #[error("undefined reference")]
    E0005,
    #[error("validation error")]
    E0006,
    #[error("motion validation error")]
    E0007,
    #[error("store field error")]
    E0008,
    #[error("store action error")]
    E0009,
    #[error("FFI error")]
    E0000,
    #[error("visibility error")]
    EVisibility,
    #[error("import error")]
    EImport,
    #[error("boundary error")]
    EBoundary,
    #[error("cycle detected")]
    ECycle,
    #[error("code generation error")]
    ECodegen,
    #[error("omniview warning")]
    WOmniviewMarker,
    #[error("motion warning")]
    WMotion,
    #[error("lint rule error")]
    ELint(String),
}

impl DiagnosticKind {
    /// Return the backward-compatible error code string (e.g., `"E_VISIBILITY"`).
    pub fn code_str(&self) -> &'static str {
        match self {
            Self::E0000 => "E0000",
            Self::E0001 => "E0001",
            Self::E0002 => "E0002",
            Self::E0003 => "E0003",
            Self::E0004 => "E0004",
            Self::E0005 => "E0005",
            Self::E0006 => "E0006",
            Self::E0007 => "E0007",
            Self::E0008 => "E0008",
            Self::E0009 => "E0009",
            Self::EVisibility => "E_VISIBILITY",
            Self::EImport => "E_IMPORT",
            Self::EBoundary => "E_BOUNDARY",
            Self::ECycle => "E_CYCLE",
            Self::ECodegen => "E_CODEGEN",
            Self::WOmniviewMarker => "W_OMNIVIEW_MARKER",
            Self::WMotion => "W0001",
            Self::ELint(_) => "E_LINT",
        }
    }
}

/// Serialize `DiagnosticKind` as its backward-compatible code string (e.g. `"E0001"`).
impl Serialize for DiagnosticKind {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.code_str())
    }
}

/// Deserialize `DiagnosticKind` from its backward-compatible code string.
impl<'de> Deserialize<'de> for DiagnosticKind {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct KindVisitor;
        impl<'de> Visitor<'de> for KindVisitor {
            type Value = DiagnosticKind;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a diagnostic code string (e.g. E0001, E_VISIBILITY)")
            }
            fn visit_str<E: de::Error>(self, s: &str) -> Result<DiagnosticKind, E> {
                match s {
                    "E0000" => Ok(DiagnosticKind::E0000),
                    "E0001" => Ok(DiagnosticKind::E0001),
                    "E0002" => Ok(DiagnosticKind::E0002),
                    "E0003" => Ok(DiagnosticKind::E0003),
                    "E0004" => Ok(DiagnosticKind::E0004),
                    "E0005" => Ok(DiagnosticKind::E0005),
                    "E0006" => Ok(DiagnosticKind::E0006),
                    "E0007" => Ok(DiagnosticKind::E0007),
                    "E0008" => Ok(DiagnosticKind::E0008),
                    "E0009" => Ok(DiagnosticKind::E0009),
                    "E_VISIBILITY" => Ok(DiagnosticKind::EVisibility),
                    "E_IMPORT" => Ok(DiagnosticKind::EImport),
                    "E_BOUNDARY" => Ok(DiagnosticKind::EBoundary),
                    "E_CYCLE" => Ok(DiagnosticKind::ECycle),
                    "E_CODEGEN" => Ok(DiagnosticKind::ECodegen),
                    "W_OMNIVIEW_MARKER" => Ok(DiagnosticKind::WOmniviewMarker),
                    "W0001" => Ok(DiagnosticKind::WMotion),
                    "E_LINT" => Ok(DiagnosticKind::ELint("E_LINT".into())),
                    other => Err(E::custom(format!("unknown diagnostic code: {}", other))),
                }
            }
        }
        deserializer.deserialize_str(KindVisitor)
    }
}

// ═══════════════════════════════════════════════════════════
// Severity
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

// ═══════════════════════════════════════════════════════════
// Diagnostic
// ═══════════════════════════════════════════════════════════

/// A user-facing diagnostic message.
#[derive(Debug, Clone, Deserialize)]
pub struct Diagnostic {
    pub severity: Severity,
    /// The typed diagnostic kind (serialized as `"code"` for backward compat).
    #[serde(rename = "code")]
    pub kind: DiagnosticKind,
    pub message: String,
    /// Source file path (set for multi-file compiles; omitted for single-file).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    pub line: u32,
    pub col: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_col: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

/// Manual `Serialize` impl to keep the JSON `"code"` field regardless of
/// whether `kind` is present in the struct (for backward compatibility).
impl Serialize for Diagnostic {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let n = 7
            + (self.file.is_some() as u8)
            + (self.end_line.is_some() as u8)
            + (self.end_col.is_some() as u8)
            + (self.suggestion.is_some() as u8);
        let mut state = serializer.serialize_struct("Diagnostic", n as usize)?;
        state.serialize_field("severity", &self.severity)?;
        state.serialize_field("code", &self.kind)?;
        state.serialize_field("message", &self.message)?;
        if let Some(ref file) = self.file {
            state.serialize_field("file", file)?;
        }
        state.serialize_field("line", &self.line)?;
        state.serialize_field("col", &self.col)?;
        if let Some(ref end_line) = self.end_line {
            state.serialize_field("end_line", end_line)?;
        }
        if let Some(ref end_col) = self.end_col {
            state.serialize_field("end_col", end_col)?;
        }
        if let Some(ref suggestion) = self.suggestion {
            state.serialize_field("suggestion", suggestion)?;
        }
        state.end()
    }
}

impl Diagnostic {
    #[must_use]
    pub fn error(kind: DiagnosticKind, message: &str, span: &Span) -> Self {
        Diagnostic {
            severity: Severity::Error,
            kind,
            message: message.to_string(),
            file: None,
            line: span.line,
            col: span.col,
            end_line: Some(span.line),
            end_col: Some(span.col + (span.end.saturating_sub(span.start)) as u32),
            suggestion: None,
        }
    }

    #[must_use]
    pub fn warning(kind: DiagnosticKind, message: &str, span: &Span) -> Self {
        Diagnostic {
            severity: Severity::Warning,
            kind,
            message: message.to_string(),
            file: None,
            line: span.line,
            col: span.col,
            end_line: Some(span.line),
            end_col: Some(span.col + (span.end.saturating_sub(span.start)) as u32),
            suggestion: None,
        }
    }

    #[must_use]
    pub fn with_file(mut self, file: &str) -> Self {
        self.file = Some(file.to_string());
        self
    }

    #[must_use]
    pub fn with_suggestion(mut self, suggestion: &str) -> Self {
        self.suggestion = Some(suggestion.to_string());
        self
    }

    /// Convert this diagnostic to a JSON value for FFI / CLI output.
    /// Uses serde serialization (Diagnostic already derives Serialize).
    #[must_use]
    pub fn to_json_value(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_else(|_| {
            serde_json::json!({"severity":"error","code":"E_SERIALIZE","message":"failed to serialize diagnostic"})
        })
    }
}

/// Convert a slice of diagnostics to a JSON array for FFI / CLI output.
#[must_use]
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
        Diagnostic::error(DiagnosticKind::E0001, &e.message, &e.span)
    }
}
