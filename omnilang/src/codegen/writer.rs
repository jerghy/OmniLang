//! CodeWriter, CompiledFile, and CompileOutput.

use std::fmt;

use crate::error::{Diagnostic, DiagnosticKind, Severity};

/// CodeWriter: tracks indentation and emits formatted lines.
#[derive(Debug)]
pub struct CodeWriter {
    lines: Vec<String>,
    indent_level: usize,
    indent_str: &'static str,
}

impl CodeWriter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            lines: vec![],
            indent_level: 0,
            indent_str: "    ",
        }
    }

    /// Emit a line at the current indent level.
    /// An empty line emits a blank line (no indent).
    pub fn emit(&mut self, line: &str) {
        if line.is_empty() {
            self.lines.push(String::new());
        } else {
            let pad = self.indent_str.repeat(self.indent_level);
            self.lines.push(format!("{}{}", pad, line));
        }
    }

    /// Emit a line with current indentation, using fmt::Arguments.
    /// More efficient than `emit(&format!(...))` — avoids intermediate String allocation.
    ///
    /// # Example
    /// ```ignore
    /// w.emit_fmt(format_args!("let {name}{ty} = {rhs};"));
    /// ```
    pub fn emit_fmt(&mut self, args: fmt::Arguments<'_>) {
        let indent = self.indent_str.repeat(self.indent_level);
        let mut line = indent;
        // write! to String never fails
        let _ = fmt::write(&mut line, args);
        self.lines.push(line);
    }

    /// Increase indentation by one level.
    pub fn indent(&mut self) {
        self.indent_level += 1;
    }

    /// Decrease indentation by one level (clamped at 0).
    pub fn dedent(&mut self) {
        self.indent_level = self.indent_level.saturating_sub(1);
    }

    /// Finalize the output string (lines joined by newline, trailing newline).
    pub fn into_string(self) -> String {
        self.lines.join("\n") + "\n"
    }
}

impl Default for CodeWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// A single compiled output file.
#[derive(Debug, Clone)]
pub struct CompiledFile {
    pub path: String,
    pub content: String,
}

/// Result of code generation for a target.
#[derive(Debug, Clone)]
pub struct CompileOutput {
    pub files: Vec<CompiledFile>,
    pub errors: Vec<Diagnostic>,
}

impl CompileOutput {
    /// Build a successful output with the given files.
    #[must_use]
    pub fn success(files: Vec<CompiledFile>) -> Self {
        Self {
            files,
            errors: vec![],
        }
    }

    /// Build an error output with a message.
    #[must_use]
    pub fn error(msg: String) -> Self {
        Self {
            files: vec![],
            errors: vec![Diagnostic {
                severity: Severity::Error,
                kind: DiagnosticKind::ECodegen,
                message: msg,
                file: None,
                line: 0,
                col: 0,
                end_line: None,
                end_col: None,
                suggestion: None,
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_writer() {
        let mut w = CodeWriter::new();
        w.emit("fn main() {");
        w.indent();
        w.emit("println!(\"hello\");");
        w.dedent();
        w.emit("}");
        let out = w.into_string();
        assert_eq!(out, "fn main() {\n    println!(\"hello\");\n}\n");
    }

    #[test]
    fn test_emit_fmt_basic() {
        let mut w = CodeWriter::new();
        w.emit_fmt(format_args!("let x = {};", 42));
        assert_eq!(w.into_string(), "let x = 42;\n");
    }

    #[test]
    fn test_emit_fmt_with_indent() {
        let mut w = CodeWriter::new();
        w.emit("fn main() {");
        w.indent();
        w.emit_fmt(format_args!("let x = {};", 1));
        w.dedent();
        w.emit("}");
        assert_eq!(w.into_string(), "fn main() {\n    let x = 1;\n}\n");
    }
}
