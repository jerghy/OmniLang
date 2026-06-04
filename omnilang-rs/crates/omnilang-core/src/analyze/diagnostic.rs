use serde::{Deserialize, Serialize};
pub use crate::span::SourceLocation;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    Error,
    Warning,
    Port,
    Style,
    Perf,
    Sec,
    Hint,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Port => "port",
            Severity::Style => "style",
            Severity::Perf => "perf",
            Severity::Sec => "sec",
            Severity::Hint => "hint",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeAction {
    pub message: String,
    pub edit: (String, String),
    pub applicable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub severity: Severity,
    pub category: String,
    pub message: String,
    pub location: Option<SourceLocation>,
    pub help: Option<String>,
    pub suggestions: Vec<CodeAction>,
}

impl Diagnostic {
    pub fn new(code: &str, severity: Severity, category: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            severity,
            category: category.to_string(),
            message: message.to_string(),
            location: None,
            help: None,
            suggestions: vec![],
        }
    }

    pub fn with_location(mut self, loc: SourceLocation) -> Self {
        self.location = Some(loc);
        self
    }

    pub fn with_help(mut self, help: &str) -> Self {
        self.help = Some(help.to_string());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticSummary {
    pub errors: usize,
    pub warnings: usize,
    pub ports: usize,
    pub styles: usize,
    pub perfs: usize,
    pub secs: usize,
    pub hints: usize,
}

impl DiagnosticSummary {
    pub fn from_diagnostics(diags: &[Diagnostic]) -> Self {
        let mut s = Self { errors: 0, warnings: 0, ports: 0, styles: 0, perfs: 0, secs: 0, hints: 0 };
        for d in diags {
            match d.severity {
                Severity::Error => s.errors += 1,
                Severity::Warning => s.warnings += 1,
                Severity::Port => s.ports += 1,
                Severity::Style => s.styles += 1,
                Severity::Perf => s.perfs += 1,
                Severity::Sec => s.secs += 1,
                Severity::Hint => s.hints += 1,
            }
        }
        s
    }

    pub fn exit_code(&self) -> i32 {
        if self.errors > 0 { 1 } else { 0 }
    }
}
