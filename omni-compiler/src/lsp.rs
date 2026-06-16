//! LSP support module -- stateless analyze() for all semantic IDE operations.
//!
//! Receives AnalysisRequest (JSON), returns AnalysisResult (JSON).
//! Pure computation -- no file I/O, no caching across calls.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::error::Diagnostic;
use crate::lexer;
use crate::lexer::Span;
use crate::parser;
use crate::parser::ast::*;

// -- Request types ---------------------------------------

#[derive(Debug, Deserialize)]
pub struct AnalysisRequest {
    pub files: Vec<SourceFile>,
    pub primary_file: String,
    pub operation: Operation,
    pub cursor_offset: usize,
    #[serde(default)]
    pub extra: Extra,
}

#[derive(Debug, Deserialize)]
pub struct SourceFile {
    pub path: String,
    pub content: String,
    #[serde(default)]
    pub is_open: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Diagnostics,
    Completion,
    Hover,
    Definition,
    References,
    Rename,
    PrepareRename,
    SemanticTokens,
    SignatureHelp,
    DocumentSymbol,
    FoldingRanges,
}

#[derive(Debug, Default, Deserialize)]
pub struct Extra {
    #[serde(default)]
    pub new_name: Option<String>,
}

// -- Response types --------------------------------------

#[derive(Debug, Serialize)]
pub struct AnalysisResult {
    pub success: bool,
    pub primary_result: PerFileResult,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub other_diagnostics: Vec<FileDiagnostics>,
}

#[derive(Debug, Default, Serialize)]
pub struct PerFileResult {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<LspDiagnostic>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resolved_imports: Vec<ResolvedImport>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<CompletionItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hover: Option<HoverInfo>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub definitions: Vec<Location>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<Location>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edits: Vec<TextEdit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rename_range: Option<LspRange>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub semantic_tokens: Vec<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<SignatureInfo>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub symbols: Vec<SymbolInfo>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub folds: Vec<FoldingRange>,
}

#[derive(Debug, Serialize)]
pub struct LspDiagnostic {
    pub severity: i32,        // 1=Error, 2=Warning, 3=Info
    pub range: LspRange,      // 1-based
    pub message: String,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LspRange {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

#[derive(Debug, Serialize)]
pub struct ResolvedImport {
    pub source_path: String,
    pub resolved_path: String,
}

#[derive(Debug, Serialize)]
pub struct CompletionItem {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insert_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_text: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HoverInfo {
    pub contents: String,     // markdown
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<LspRange>,
}

#[derive(Debug, Serialize)]
pub struct Location {
    pub file_path: String,
    pub range: LspRange,
}

#[derive(Debug, Serialize)]
pub struct TextEdit {
    pub range: LspRange,
    pub new_text: String,
}

#[derive(Debug, Serialize)]
pub struct SignatureInfo {
    pub label: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<ParameterInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_parameter: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct ParameterInfo {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: i32,             // LSP SymbolKind
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<LspRange>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selection_range: Option<LspRange>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<SymbolInfo>,
}

#[derive(Debug, Serialize)]
pub struct FoldingRange {
    pub start_line: u32,
    pub end_line: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FileDiagnostics {
    pub file_path: String,
    pub diagnostics: Vec<LspDiagnostic>,
}

// -- Internal parse state --------------------------------

struct ParsedFile {
    path: String,
    content: String,    // original source -- needed for cursor offset calculations
    program: Program,
    tokens: Vec<lexer::Token>,
}

struct AnalysisContext {
    files: HashMap<String, ParsedFile>,
    primary_file: String,
    cursor_offset: usize,
    /// Cross-file symbol table: file_path -> [(name, kind, span)]
    cross_symbols: HashMap<String, Vec<(String, SymbolKind, Span)>>,
    /// Parse errors collected during initial parse, available for diagnostics
    parse_errors: HashMap<String, crate::error::ParseError>,
}

#[derive(Debug, Clone)]
enum SymbolKind {
    Function,
    Variable,
    Struct,
    TypeAlias,
    Component,
    Store,
    Signal,
    Computed,
    Prop,
    Action,
    Event,
    Theme,
    Import,
    Export,
    Param,
}

// -- Helpers ---------------------------------------------

/// Convert a lexer Span to an LSP range.
///
/// Note: the lexer Span stores only the start line/col. We approximate the
/// end column from the byte-offset length.
fn span_to_lsp_range(span: &Span) -> LspRange {
    // Approximate end column from byte offset difference
    let approx_len = span.end.saturating_sub(span.start) as u32;
    LspRange {
        start_line: span.line,
        start_col: span.col,
        end_line: span.line,
        end_col: span.col + approx_len,
    }
}

fn diagnostic_to_lsp(d: &Diagnostic) -> LspDiagnostic {
    LspDiagnostic {
        severity: match d.severity {
            crate::error::Severity::Error => 1,
            crate::error::Severity::Warning => 2,
            crate::error::Severity::Info => 3,
        },
        range: LspRange {
            start_line: d.line,
            start_col: d.col,
            end_line: d.end_line,
            end_col: d.end_col,
        },
        message: d.message.clone(),
        code: d.code.clone(),
        source: Some("omnilang".to_string()),
    }
}

fn byte_offset_to_line_col(source: &str, offset: usize) -> (u32, u32) {
    let offset = offset.min(source.len());
    let prefix = &source[..offset];
    let line = prefix.chars().filter(|&c| c == '\n').count() as u32 + 1;
    let last_newline = prefix.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let col = (offset - last_newline) as u32 + 1;
    (line, col)
}

// -- Multi-file parsing + symbol table -------------------

impl AnalysisContext {
    fn new(request: &AnalysisRequest) -> Self {
        let mut files: HashMap<String, ParsedFile> = HashMap::new();
        let mut cross_symbols: HashMap<String, Vec<(String, SymbolKind, Span)>> = HashMap::new();
        let mut parse_errors: HashMap<String, crate::error::ParseError> = HashMap::new();

        for sf in &request.files {
            let tokens = lexer::tokenize(&sf.content);
            match parser::parse(&tokens) {
                Ok(program) => {
                    let symbols = collect_file_symbols(&sf.path, &program);
                    cross_symbols.insert(sf.path.clone(), symbols);
                    files.insert(sf.path.clone(), ParsedFile {
                        path: sf.path.clone(),
                        content: sf.content.clone(),
                        program,
                        tokens,
                    });
                }
                Err(e) => {
                    // Store empty program but preserve parse error for diagnostics
                    parse_errors.insert(sf.path.clone(), e);
                    files.insert(sf.path.clone(), ParsedFile {
                        path: sf.path.clone(),
                        content: sf.content.clone(),
                        program: Program::default(),
                        tokens,
                    });
                }
            }
        }

        AnalysisContext {
            files,
            primary_file: request.primary_file.clone(),
            cursor_offset: request.cursor_offset,
            cross_symbols,
            parse_errors,
        }
    }
}

/// Collect file-level symbols (name, kind, span) for cross-file resolution.
///
/// Adaptations from spec:
/// - AST types do not have `name_span` fields; use the full definition `.span` instead
/// - Inner declarations (Param, PropDecl, SignalDecl, ComputedDecl) and StoreDef
///   lack span fields entirely, so they are omitted from the cross-symbol table
fn collect_file_symbols(
    _file_path: &str,
    program: &Program,
) -> Vec<(String, SymbolKind, Span)> {
    let mut symbols = Vec::new();

    // Functions and their parameters
    for f in &program.functions {
        symbols.push((f.name.clone(), SymbolKind::Function, f.span.clone()));
        // Param has no span -- skip inner params
    }

    // Structs
    for s in &program.structs {
        symbols.push((s.name.clone(), SymbolKind::Struct, s.span.clone()));
    }

    // Type aliases
    for ta in &program.type_aliases {
        symbols.push((ta.name.clone(), SymbolKind::TypeAlias, ta.span.clone()));
    }

    // Components and their inner declarations
    for c in &program.components {
        symbols.push((c.name.clone(), SymbolKind::Component, c.span.clone()));
        // SignalDecl, PropDecl, ComputedDecl lack spans -- skip inner symbols
    }

    // Stores (StoreDef has no span field -- skip until span is added to the AST)
    for st in &program.stores {
        // st.name exists but no span available -- omit from cross-symbol table
        let _ = st;
    }

    // Import symbols: collect each imported name/alias with the import's span
    for imp in &program.imports {
        match imp {
            ImportStmt::Named(n) => {
                for name in &n.names {
                    symbols.push((name.clone(), SymbolKind::Import, n.span.clone()));
                }
            }
            ImportStmt::Namespace(ns) => {
                symbols.push((ns.alias.clone(), SymbolKind::Import, ns.span.clone()));
            }
            ImportStmt::TypeOnly(t) => {
                for name in &t.names {
                    symbols.push((name.clone(), SymbolKind::Import, t.span.clone()));
                }
            }
            ImportStmt::Lib(_) => {} // bare import marker, no user-visible symbols
        }
    }

    symbols
}
