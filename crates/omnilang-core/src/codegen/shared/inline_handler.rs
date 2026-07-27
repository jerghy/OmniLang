//! Shared inline handler formatting utility.
//!
//! Eliminates structurally identical `format_inline_handler` functions
//! across React, Vue, Flutter, and Dioxus codegen targets.

use crate::codegen::writer::CodeWriter;
use crate::ir::BlockIR;
use crate::ir::StatementIR;

/// Configuration for building an inline handler (closure / arrow function) string
/// from a `BlockIR` body.
pub struct InlineHandlerConfig {
    /// Opening delimiter, e.g. `"() => {\n"` or `"() {"`.
    pub opener: String,
    /// Closing delimiter, e.g. `"\n      }"` or `"}"`.
    pub closer: String,
    /// Value to return when the body is empty, e.g. `"() => {}"`.
    pub empty_handler: String,
    /// Per-line indentation prefix used in non-writer mode (e.g. `"        "`).
    pub indent: &'static str,
    /// When `true`, use `CodeWriter::indent`/`dedent` for indentation;
    /// when `false`, apply `indent` as a per-line prefix manually.
    pub use_writer: bool,
}

/// Build an inline handler (closure / arrow function / callback) string
/// from a `BlockIR` body, parameterised by `config` and an `emit_stmt`
/// callback that renders each statement into the `CodeWriter`.
pub fn build_inline_handler(
    body: &BlockIR,
    config: &InlineHandlerConfig,
    emit_stmt: impl Fn(&mut CodeWriter, &StatementIR),
) -> String {
    if body.statements.is_empty() {
        return config.empty_handler.clone();
    }

    if config.use_writer {
        // Writer-mode: let CodeWriter handle indentation
        let mut hw = CodeWriter::new();
        hw.emit(&config.opener);
        hw.indent();
        for stmt in &body.statements {
            emit_stmt(&mut hw, stmt);
        }
        hw.dedent();
        hw.emit(&config.closer);
        hw.into_string().trim().to_string()
    } else {
        // Non-writer mode: collect flat content and apply per-line indent
        let mut hw = CodeWriter::new();
        for stmt in &body.statements {
            emit_stmt(&mut hw, stmt);
        }
        let content = hw.into_string();
        let mut result = config.opener.clone();
        for line in content.lines() {
            result.push_str(&format!("{}{}\n", config.indent, line));
        }
        result.push_str(&config.closer);
        result
    }
}
