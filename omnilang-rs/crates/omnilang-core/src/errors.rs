use crate::span::Span;
use colored::*;

/// A single compiler error or warning.
#[derive(Debug, Clone)]
pub struct CompilerError {
    pub message: String,
    pub span: Option<Span>,
    pub filename: Option<String>,
}

impl CompilerError {
    pub fn new(message: impl Into<String>, span: Option<Span>, filename: Option<String>) -> Self {
        Self {
            message: message.into(),
            span,
            filename,
        }
    }
}

/// Accumulates compilation errors across passes.
/// Uses the same side-effect pattern as Python's ErrorCollector.
#[derive(Debug, Default)]
pub struct ErrorCollector {
    pub errors: Vec<CompilerError>,
}

impl ErrorCollector {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    pub fn add(&mut self, message: impl Into<String>, span: Option<Span>, filename: Option<String>) {
        self.errors.push(CompilerError::new(message, span, filename));
    }

    pub fn add_error(&mut self, message: impl Into<String>) {
        self.errors.push(CompilerError {
            message: message.into(),
            span: None,
            filename: None,
        });
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn len(&self) -> usize {
        self.errors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Format all errors with source context, similar to rustc output.
    /// If source_lines is provided, shows the relevant lines with caret indicators.
    pub fn report(&self, source_lines: Option<&std::collections::HashMap<String, Vec<String>>>) -> String {
        let mut output = String::new();
        for error in &self.errors {
            if let (Some(span), Some(filename)) = (&error.span, &error.filename) {
                let file = filename;
                output.push_str(&format!(
                    "{} {}:{}:{}: {}\n",
                    "Error:".red().bold(),
                    file,
                    span.line,
                    span.col,
                    error.message
                ));
                if let Some(lines) = source_lines {
                    if let Some(file_lines) = lines.get(file) {
                        let line_idx = span.line as usize;
                        if line_idx > 0 && line_idx <= file_lines.len() {
                            let line = &file_lines[line_idx - 1];
                            output.push_str(&format!("  {} |\n", format!("{:>4}", "")));
                            output.push_str(&format!("{} | {}\n", format!("{:>4}", span.line).blue().bold(), line));
                            // Caret indicator
                            let padding = span.col as usize;
                            output.push_str(&format!("  {} |{}\n", format!("{:>4}", ""), " ".repeat(padding + 1) + &"^".red().bold().to_string()));
                        }
                    }
                }
            } else if let Some(span) = &error.span {
                output.push_str(&format!(
                    "{} {}:{}:{}: {}\n",
                    "Error:".red().bold(),
                    span.file,
                    span.line,
                    span.col,
                    error.message
                ));
            } else {
                output.push_str(&format!("{} {}\n", "Error:".red().bold(), error.message));
            }
        }
        output
    }

    /// Print errors to stderr and exit with code 1 if there are errors.
    /// source_lines is an optional map from filename to lines for context display.
    pub fn report_and_exit(
        &self,
        source_lines: Option<&std::collections::HashMap<String, Vec<String>>>,
    ) -> ! {
        eprintln!("{}", self.report(source_lines));
        std::process::exit(1);
    }
}
