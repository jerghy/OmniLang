//! Unified error and diagnostic types for the OmniLang compiler.

use serde::de::{self, Deserializer, Visitor};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};
use std::fmt;
use ts_rs::TS;

use crate::lexer::Span;

// ═══════════════════════════════════════════════════════════
// LintRuleId
// ═══════════════════════════════════════════════════════════

/// Typed lint rule identifier, replacing raw string rule names.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LintRuleId {
    #[error("undefined-ref")]
    UndefinedRef,
    #[error("type-mismatch")]
    TypeMismatch,
    #[error("store-field-direct-mutation")]
    StoreFieldDirectMutation,
    #[error("prop-immutability")]
    PropImmutability,
    #[error("computed-purity")]
    ComputedPurity,
    #[error("missing-return")]
    MissingReturn,
    #[error("dead-code")]
    DeadCode,
    #[error("unused-import")]
    UnusedImport,
    #[error("invalid-event-field")]
    InvalidEventField,
    #[error("store-action-no-event-data")]
    StoreActionNoEventData,
    #[error("for-key")]
    ForKey,
    /// Unknown/unrecognized lint rule (fallback for forward compatibility).
    #[error("{0}")]
    Unknown(String),
}

impl From<&str> for LintRuleId {
    fn from(s: &str) -> Self {
        match s {
            "undefined-ref" => Self::UndefinedRef,
            "type-mismatch" => Self::TypeMismatch,
            "store-field-direct-mutation" => Self::StoreFieldDirectMutation,
            "prop-immutability" => Self::PropImmutability,
            "computed-purity" => Self::ComputedPurity,
            "missing-return" => Self::MissingReturn,
            "dead-code" => Self::DeadCode,
            "unused-import" => Self::UnusedImport,
            "invalid-event-field" => Self::InvalidEventField,
            "store-action-no-event-data" => Self::StoreActionNoEventData,
            "for-key" => Self::ForKey,
            _ => Self::Unknown(s.to_string()),
        }
    }
}

impl From<String> for LintRuleId {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

impl Serialize for LintRuleId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for LintRuleId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s))
    }
}

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
    #[error("semantic type error")]
    E0010,
    #[error("missing required prop")]
    E0011,
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
    #[error("closure syntax error")]
    EClosureArrow,
    #[error("array method error")]
    EArrayMethod,
    #[error("effect dependency error")]
    EEffectDepsMissing,
    #[error("paused playback keyword removed")]
    EPausedRemoved,
    #[error("omniview warning")]
    WOmniviewMarker,
    #[error("motion warning")]
    WMotion,
    #[error("lint rule error")]
    ELint(LintRuleId),
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
            Self::E0010 => "E0010",
            Self::E0011 => "E0011",
            Self::EVisibility => "E_VISIBILITY",
            Self::EImport => "E_IMPORT",
            Self::EBoundary => "E_BOUNDARY",
            Self::ECycle => "E_CYCLE",
            Self::ECodegen => "E_CODEGEN",
            Self::EClosureArrow => "E_CLOSURE_ARROW",
            Self::EArrayMethod => "E_ARRAY_METHOD",
            Self::EEffectDepsMissing => "E_EFFECT_DEPS_MISSING",
            Self::EPausedRemoved => "E_PAUSED_REMOVED",
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
                    "E0010" => Ok(DiagnosticKind::E0010),
                    "E0011" => Ok(DiagnosticKind::E0011),
                    "E_VISIBILITY" => Ok(DiagnosticKind::EVisibility),
                    "E_IMPORT" => Ok(DiagnosticKind::EImport),
                    "E_BOUNDARY" => Ok(DiagnosticKind::EBoundary),
                    "E_CYCLE" => Ok(DiagnosticKind::ECycle),
                    "E_CODEGEN" => Ok(DiagnosticKind::ECodegen),
                    "E_CLOSURE_ARROW" => Ok(DiagnosticKind::EClosureArrow),
                    "E_ARRAY_METHOD" => Ok(DiagnosticKind::EArrayMethod),
                    "E_EFFECT_DEPS_MISSING" => Ok(DiagnosticKind::EEffectDepsMissing),
                    "E_PAUSED_REMOVED" => Ok(DiagnosticKind::EPausedRemoved),
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
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
#[derive(Debug, Clone, Deserialize, TS)]
pub struct Diagnostic {
    pub severity: Severity,
    /// The typed diagnostic kind (serialized as `"code"` for backward compat).
    #[serde(rename = "code")]
    #[ts(type = "string")]
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

// ═══════════════════════════════════════════════════════════
// DiagnosticJson — unified FFI/WASM diagnostic envelope
// ═══════════════════════════════════════════════════════════

/// Canonical diagnostic envelope for the FFI/WASM boundary.
///
/// All 9 fields are always present (missing values serialize as explicit
/// `null`), so every consumer sees ONE shape regardless of which code path
/// produced the diagnostic. The `Diagnostic` type keeps its own manual
/// `Serialize` impl for serde consumers (compile-result JSON etc.).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiagnosticJson {
    /// Lowercase severity: "error" | "warning" | "info".
    pub severity: String,
    /// Backward-compatible code string (e.g. "E0001", "E_VISIBILITY").
    pub code: String,
    pub message: String,
    pub file: Option<String>,
    pub line: u32,
    pub col: u32,
    pub end_line: Option<u32>,
    pub end_col: Option<u32>,
    pub suggestion: Option<String>,
}

impl DiagnosticJson {
    /// Build a synthetic (non-source) error envelope for boundary failures
    /// (size limits, invalid inputs, missing sources).
    #[must_use]
    pub fn synthetic(code: &str, message: impl Into<String>) -> Self {
        Self {
            severity: "error".to_string(),
            code: code.to_string(),
            message: message.into(),
            file: None,
            line: 0,
            col: 0,
            end_line: None,
            end_col: None,
            suggestion: None,
        }
    }
}

impl From<&Diagnostic> for DiagnosticJson {
    fn from(d: &Diagnostic) -> Self {
        Self {
            severity: match d.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Info => "info",
            }
            .to_string(),
            code: d.kind.code_str().to_string(),
            message: d.message.clone(),
            file: d.file.clone(),
            line: d.line,
            col: d.col,
            end_line: d.end_line,
            end_col: d.end_col,
            suggestion: d.suggestion.clone(),
        }
    }
}

/// Returns the closest matching name within edit distance <= 3, if any.
pub fn suggest_similar_name<'a>(attempt: &str, known_names: &[&'a str]) -> Option<&'a str> {
    let mut best_dist = usize::MAX;
    let mut best_name = None;
    for &name in known_names {
        let dist = edit_distance(attempt, name);
        if dist <= 3 && dist < best_dist {
            best_dist = dist;
            best_name = Some(name);
        }
    }
    best_name
}

/// Compute Levenshtein edit distance between two strings.
fn edit_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=a_chars.len()).collect();
    let mut curr = vec![0; a_chars.len() + 1];
    for &bc in &b_chars {
        curr[0] = prev[0] + 1;
        for (i, &ac) in a_chars.iter().enumerate() {
            curr[i + 1] = if ac == bc {
                prev[i]
            } else {
                1 + prev[i].min(curr[i]).min(prev[i + 1])
            };
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[a_chars.len()]
}

/// A raw parse error before conversion to Diagnostic.
#[derive(Debug, Serialize)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
    /// Typed diagnostic kind for this parse error (e.g. `EClosureArrow`).
    /// `None` maps to `E0001` when converted to a `Diagnostic`.
    #[serde(skip)]
    pub kind: Option<DiagnosticKind>,
}

impl ParseError {
    /// Construct a typed parse error — `kind` is carried into the
    /// `Diagnostic` at conversion (untyped errors stay `E0001`).
    #[must_use]
    pub fn with_kind(kind: DiagnosticKind, message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
            kind: Some(kind),
        }
    }
}

impl From<ParseError> for Diagnostic {
    fn from(e: ParseError) -> Self {
        let kind = e.kind.unwrap_or(DiagnosticKind::E0001);
        Diagnostic::error(kind, &e.message, &e.span)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_json_full_shape() {
        let d = Diagnostic::error(
            DiagnosticKind::E0001,
            "bad token",
            &Span {
                start: 0,
                end: 4,
                line: 3,
                col: 5,
            },
        )
        .with_file("main.omni")
        .with_suggestion("try again");
        let json = serde_json::to_value(DiagnosticJson::from(&d)).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "severity": "error",
                "code": "E0001",
                "message": "bad token",
                "file": "main.omni",
                "line": 3,
                "col": 5,
                "end_line": 3,
                "end_col": 9,
                "suggestion": "try again",
            })
        );
    }

    #[test]
    fn diagnostic_json_explicit_nulls_when_absent() {
        // Missing optional fields serialize as explicit nulls — the unified
        // 9-field shape is the same for every diagnostic on the boundary.
        // Built as a struct literal (not Diagnostic::warning, which always
        // fills end_line/end_col from the span) so the Nones are real.
        let d = Diagnostic {
            severity: Severity::Warning,
            kind: DiagnosticKind::WMotion,
            message: "unused".to_string(),
            file: None,
            line: 1,
            col: 1,
            end_line: None,
            end_col: None,
            suggestion: None,
        };
        let json = serde_json::to_value(DiagnosticJson::from(&d)).unwrap();
        assert_eq!(json["severity"], "warning");
        assert_eq!(json["code"], "W0001");
        assert!(json["file"].is_null());
        assert!(json["end_line"].is_null());
        assert!(json["end_col"].is_null());
        assert!(json["suggestion"].is_null());
        // Same key set as the full-shape case — no key is ever dropped.
        let full = DiagnosticJson::from(&Diagnostic::error(
            DiagnosticKind::E0002,
            "dup",
            &Span {
                start: 0,
                end: 1,
                line: 1,
                col: 1,
            },
        ));
        let full_json = serde_json::to_value(&full).unwrap();
        let null_json = serde_json::to_value(&DiagnosticJson::from(&d)).unwrap();
        let mut full_keys: Vec<&str> = full_json
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        let mut null_keys: Vec<&str> = null_json
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        full_keys.sort_unstable();
        null_keys.sort_unstable();
        assert_eq!(full_keys, null_keys);
    }

    #[test]
    fn diagnostic_json_lowercase_severity() {
        // Severity is always lowercase — "Error"/"Warning" capitalizations
        // are not allowed to leak through the boundary.
        for (sev, expected) in [
            (Severity::Error, "error"),
            (Severity::Warning, "warning"),
            (Severity::Info, "info"),
        ] {
            let d = Diagnostic {
                severity: sev,
                kind: DiagnosticKind::E0001,
                message: "m".into(),
                file: None,
                line: 0,
                col: 0,
                end_line: None,
                end_col: None,
                suggestion: None,
            };
            let json = serde_json::to_value(DiagnosticJson::from(&d)).unwrap();
            assert_eq!(json["severity"], expected);
        }
    }

    #[test]
    fn closure_arrow_code_registered_and_carried() {
        // E_CLOSURE_ARROW must be a registered diagnostic code: code_str,
        // serde round-trip, and ParseError → Diagnostic propagation.
        assert_eq!(DiagnosticKind::EClosureArrow.code_str(), "E_CLOSURE_ARROW");
        let json = serde_json::to_value(DiagnosticKind::EClosureArrow).unwrap();
        assert_eq!(json, "E_CLOSURE_ARROW");
        let back: DiagnosticKind = serde_json::from_value(json).unwrap();
        assert_eq!(back, DiagnosticKind::EClosureArrow);

        let diag: Diagnostic = ParseError::with_kind(
            DiagnosticKind::EClosureArrow,
            "通用闭包用管道 |x|；箭头仅用于事件内联",
            Span {
                start: 0,
                end: 2,
                line: 1,
                col: 1,
            },
        )
        .into();
        assert_eq!(diag.kind, DiagnosticKind::EClosureArrow);
        assert!(diag.message.contains("|x|"));

        // E_ARRAY_METHOD must be a registered diagnostic code too (semantic
        // array-method rules carry the same named-code convention).
        assert_eq!(DiagnosticKind::EArrayMethod.code_str(), "E_ARRAY_METHOD");
        let json = serde_json::to_value(DiagnosticKind::EArrayMethod).unwrap();
        assert_eq!(json, "E_ARRAY_METHOD");
        let back: DiagnosticKind = serde_json::from_value(json).unwrap();
        assert_eq!(back, DiagnosticKind::EArrayMethod);
        // Untyped ParseErrors still map to E0001.
        let plain: Diagnostic = ParseError {
            message: "generic".into(),
            span: Span {
                start: 0,
                end: 1,
                line: 1,
                col: 1,
            },
            kind: None,
        }
        .into();
        assert_eq!(plain.kind, DiagnosticKind::E0001);
    }

    #[test]
    fn effect_deps_missing_code_registered_and_carried() {
        // E_EFFECT_DEPS_MISSING must be a registered diagnostic code:
        // code_str, serde round-trip, and Diagnostic construction.
        assert_eq!(
            DiagnosticKind::EEffectDepsMissing.code_str(),
            "E_EFFECT_DEPS_MISSING"
        );
        let json = serde_json::to_value(DiagnosticKind::EEffectDepsMissing).unwrap();
        assert_eq!(json, "E_EFFECT_DEPS_MISSING");
        let back: DiagnosticKind = serde_json::from_value(json).unwrap();
        assert_eq!(back, DiagnosticKind::EEffectDepsMissing);

        let diag = Diagnostic::error(
            DiagnosticKind::EEffectDepsMissing,
            "effect 依赖漏掉：NotificationStore.unreadCount",
            &Span {
                start: 0,
                end: 2,
                line: 1,
                col: 1,
            },
        );
        assert_eq!(diag.kind, DiagnosticKind::EEffectDepsMissing);
        assert_eq!(diag.severity, Severity::Error);
        assert_eq!(
            diag.message,
            "effect 依赖漏掉：NotificationStore.unreadCount"
        );
        let json = serde_json::to_value(DiagnosticJson::from(&diag)).unwrap();
        assert_eq!(json["code"], "E_EFFECT_DEPS_MISSING");
        assert_eq!(json["severity"], "error");
    }

    #[test]
    fn paused_removed_code_registered_and_carried() {
        // E_PAUSED_REMOVED must be a registered diagnostic code: code_str,
        // serde round-trip, and ParseError → Diagnostic propagation.
        assert_eq!(
            DiagnosticKind::EPausedRemoved.code_str(),
            "E_PAUSED_REMOVED"
        );
        let json = serde_json::to_value(DiagnosticKind::EPausedRemoved).unwrap();
        assert_eq!(json, "E_PAUSED_REMOVED");
        let back: DiagnosticKind = serde_json::from_value(json).unwrap();
        assert_eq!(back, DiagnosticKind::EPausedRemoved);

        let diag: Diagnostic = ParseError::with_kind(
            DiagnosticKind::EPausedRemoved,
            "paused 已删除，用 @if 条件渲染替代",
            Span {
                start: 0,
                end: 2,
                line: 1,
                col: 1,
            },
        )
        .into();
        assert_eq!(diag.kind, DiagnosticKind::EPausedRemoved);
        assert_eq!(diag.severity, Severity::Error);
        assert!(diag.message.contains("@if"));
    }

    #[test]
    fn diagnostic_json_synthetic() {
        let json = serde_json::to_value(DiagnosticJson::synthetic("E_WASM", "boom")).unwrap();
        assert_eq!(json["severity"], "error");
        assert_eq!(json["code"], "E_WASM");
        assert_eq!(json["message"], "boom");
        assert!(json["file"].is_null());
        assert_eq!(json["line"], 0);
        assert_eq!(json["col"], 0);
        assert!(json["end_line"].is_null());
        assert!(json["end_col"].is_null());
        assert!(json["suggestion"].is_null());
    }
}
