use serde::{Deserialize, Serialize};

/// Source location in a file. 1-based line and column numbers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Span {
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

impl Span {
    pub fn new(file: impl Into<String>, line: u32, col: u32, end_line: u32, end_col: u32) -> Self {
        Self {
            file: file.into(),
            line,
            col,
            end_line,
            end_col,
        }
    }

    /// Create a dummy span for tests
    pub fn dummy() -> Self {
        Self {
            file: String::new(),
            line: 0,
            col: 0,
            end_line: 0,
            end_col: 0,
        }
    }

    pub fn is_dummy(&self) -> bool {
        self.line == 0
    }
}

/// Used for diagnostic locations (can reference a specific position or a range).
/// Mirrors Python's SourceLocation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
    /// (start_line, start_col, end_line, end_col)
    pub span: (u32, u32, u32, u32),
}

impl From<&Span> for SourceLocation {
    fn from(span: &Span) -> Self {
        Self {
            file: span.file.clone(),
            line: span.line,
            column: span.col,
            span: (span.line, span.col, span.end_line, span.end_col),
        }
    }
}
