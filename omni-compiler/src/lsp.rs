//! LSP support module -- stateless analyze() for all semantic IDE operations.
//!
//! Receives AnalysisRequest (JSON), returns AnalysisResult (JSON).
//! Pure computation -- no file I/O, no caching across calls.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::error::Diagnostic;
use crate::lexer;
use crate::lexer::Span;
use crate::lexer::TokenKind;
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
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
    // Ensure offset is at a valid UTF-8 char boundary (snap back if inside multi-byte char)
    let mut offset = offset.min(source.len());
    while offset > 0 && !source.is_char_boundary(offset) {
        offset -= 1;
    }
    let prefix = &source[..offset];
    let line = prefix.chars().filter(|&c| c == '\n').count() as u32 + 1;
    let last_newline = prefix.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let col = prefix[last_newline..].chars().count() as u32 + 1;
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
            match parser::parse(&tokens, &sf.content) {
                Ok(program) => {
                    let symbols = collect_file_symbols(&sf.path, &sf.content, &program);
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
    source: &str,
    program: &Program,
) -> Vec<(String, SymbolKind, Span)> {
    let mut symbols = Vec::new();

    // Functions — span covers 'fn' keyword, not the name.
    // Compute name span: name starts right after 'fn' keyword.
    for f in &program.functions {
        let name_start = f.span.end;
        let name_end = name_start + f.name.len();
        let (name_line, name_col) = byte_offset_to_line_col(source, name_start);
        let name_span = Span { start: name_start, end: name_end, line: name_line, col: name_col };
        symbols.push((f.name.clone(), SymbolKind::Function, name_span));
    }

    // Structs — similarly, span covers 'struct' keyword
    for s in &program.structs {
        let name_start = s.span.end;
        let name_end = name_start + s.name.len();
        let (name_line, name_col) = byte_offset_to_line_col(source, name_start);
        let name_span = Span { start: name_start, end: name_end, line: name_line, col: name_col };
        symbols.push((s.name.clone(), SymbolKind::Struct, name_span));
    }

    // Type aliases
    for ta in &program.type_aliases {
        let name_start = ta.span.end;
        let name_end = name_start + ta.name.len();
        let (name_line, name_col) = byte_offset_to_line_col(source, name_start);
        let name_span = Span { start: name_start, end: name_end, line: name_line, col: name_col };
        symbols.push((ta.name.clone(), SymbolKind::TypeAlias, name_span));
    }

    // Components — span covers 'component' keyword
    for c in &program.components {
        let name_start = c.span.end;
        let name_end = name_start + c.name.len();
        let (name_line, name_col) = byte_offset_to_line_col(source, name_start);
        let name_span = Span { start: name_start, end: name_end, line: name_line, col: name_col };
        symbols.push((c.name.clone(), SymbolKind::Component, name_span));
    }

    // Stores — StoreDef has no span field, so name position can't be computed.
    // Skip until span is added to the AST.
    for st in &program.stores {
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

// ── Span helpers (since Span has no end_line/end_col methods) ─────

fn span_end_line(source: &str, span: &Span) -> u32 {
    byte_offset_to_line_col(source, span.end).0
}

fn span_end_col(source: &str, span: &Span) -> u32 {
    byte_offset_to_line_col(source, span.end).1
}

fn span_contains(source: &str, span: &Span, line: u32, col: u32) -> bool {
    let end_line = span_end_line(source, span);
    let end_col = span_end_col(source, span);
    span.line <= line && end_line >= line
        && (span.line < line || span.col <= col)
        && (end_line > line || end_col >= col)
}

// ════════════════════════════════════════════════════════
// Diagnostics
// ════════════════════════════════════════════════════════

fn compute_diagnostics(ctx: &AnalysisContext) -> Vec<LspDiagnostic> {
    let mut diags = Vec::new();

    // 1. Emit parse errors collected during AnalysisContext::new
    for (_path, parse_err) in &ctx.parse_errors {
        let d = Diagnostic::error("E0001", &parse_err.message, &parse_err.span);
        diags.push(diagnostic_to_lsp(&d));
    }

    // 2. Run semantic analysis on each successfully-parsed file
    for (_path, pf) in &ctx.files {
        let sem_result = crate::semantic::analyze(&pf.program);
        for d in &sem_result.diagnostics {
            diags.push(diagnostic_to_lsp(d));
        }
    }

    diags
}

fn compute_resolved_imports(ctx: &AnalysisContext) -> Vec<ResolvedImport> {
    let mut imports = Vec::new();
    let primary = match ctx.files.get(&ctx.primary_file) {
        Some(pf) => pf,
        None => return imports,
    };

    for imp in &primary.program.imports {
        match imp {
            ImportStmt::Named(n) => {
                let resolved = resolve_import_path(&ctx.primary_file, &n.path, &ctx.files);
                imports.push(ResolvedImport {
                    source_path: n.path.clone(),
                    resolved_path: resolved,
                });
            }
            ImportStmt::Namespace(ns) => {
                let resolved = resolve_import_path(&ctx.primary_file, &ns.path, &ctx.files);
                imports.push(ResolvedImport {
                    source_path: ns.path.clone(),
                    resolved_path: resolved,
                });
            }
            ImportStmt::TypeOnly(t) => {
                let resolved = resolve_import_path(&ctx.primary_file, &t.path, &ctx.files);
                imports.push(ResolvedImport {
                    source_path: t.path.clone(),
                    resolved_path: resolved,
                });
            }
            ImportStmt::Lib(_) => {}
        }
    }

    imports
}

fn resolve_import_path(
    from_file: &str,
    import_path: &str,
    files: &HashMap<String, ParsedFile>,
) -> String {
    use std::path::Path;
    let parent = Path::new(from_file).parent().unwrap_or(Path::new(""));
    let mut resolved = parent.join(import_path);
    // Only add .omni extension if the import path doesn't already have it
    if resolved.extension().map_or(true, |ext| ext != "omni") {
        resolved = resolved.with_extension("omni");
    }
    let normalized = resolved.to_string_lossy().replace('\\', "/");
    for key in files.keys() {
        if key.ends_with(&normalized) || normalized.ends_with(key) {
            return key.clone();
        }
    }
    normalized
}

// ════════════════════════════════════════════════════════
// Hover
// ════════════════════════════════════════════════════════

fn compute_hover(ctx: &AnalysisContext) -> Option<HoverInfo> {
    let (symbol_name, symbol_kind, symbol_span) = find_symbol_at_cursor(ctx)?;

    let contents = match symbol_kind {
        SymbolKind::Function => {
            format!("**Function `{}`**\n\n```omnilang\nfn {}(...) {{ ... }}\n```", symbol_name, symbol_name)
        }
        SymbolKind::Variable | SymbolKind::Param => {
            format!("**Variable `{}`**", symbol_name)
        }
        SymbolKind::Struct => {
            format!("**Struct `{}`**", symbol_name)
        }
        SymbolKind::Component => {
            format!("**Component `{}`**", symbol_name)
        }
        SymbolKind::Store => {
            format!("**Store `{}`**", symbol_name)
        }
        SymbolKind::Signal => {
            format!("**Signal `{}`**\n\nReactive state variable — changes trigger re-render.", symbol_name)
        }
        SymbolKind::Computed => {
            format!("**Computed `{}`**\n\nDerived reactive value — auto-updates when dependencies change.", symbol_name)
        }
        _ => format!("**`{}`**", symbol_name),
    };

    Some(HoverInfo {
        contents,
        range: Some(span_to_lsp_range(&symbol_span)),
    })
}

fn find_symbol_at_cursor(ctx: &AnalysisContext) -> Option<(String, SymbolKind, Span)> {
    let primary = ctx.files.get(&ctx.primary_file)?;
    let (cursor_line, cursor_col) = byte_offset_to_line_col(&primary.content, ctx.cursor_offset);
    let source = &primary.content;

    let symbols = ctx.cross_symbols.get(&ctx.primary_file)?;

    // Pass 1: find a declaration span containing the cursor (works at declaration sites)
    let decl_match = symbols.iter()
        .filter(|(_, _, span)| span_contains(source, span, cursor_line, cursor_col))
        .min_by_key(|(_, _, span)| {
            let s_end_line = span_end_line(source, span);
            let s_end_col = span_end_col(source, span);
            let len = (s_end_line as i64 - span.line as i64) * 1000
                + (s_end_col as i64 - span.col as i64);
            len
        })
        .map(|(name, kind, span)| (name.clone(), kind.clone(), span.clone()));

    if decl_match.is_some() {
        return decl_match;
    }

    // Pass 2: fallback — find identifier token at cursor and look up in cross_symbols
    // This handles usage sites (e.g., hovering a variable reference, not its declaration)
    for tok in &primary.tokens {
        let start_pos = byte_offset_to_line_col(source, tok.span.start);
        let end_pos = byte_offset_to_line_col(source, tok.span.end);
        if start_pos.0 <= cursor_line && end_pos.0 >= cursor_line
            && (start_pos.0 < cursor_line || start_pos.1 <= cursor_col)
            && (end_pos.0 > cursor_line || end_pos.1 >= cursor_col)
        {
            if let TokenKind::Ident(name) = &tok.kind {
                // Look up this name in the cross-symbol table (any file)
                for (_file, file_symbols) in &ctx.cross_symbols {
                    for (sym_name, sym_kind, sym_span) in file_symbols {
                        if sym_name == name {
                            return Some((sym_name.clone(), sym_kind.clone(), sym_span.clone()));
                        }
                    }
                }
            }
        }
    }

    None
}

// ════════════════════════════════════════════════════════
// Definition
// ════════════════════════════════════════════════════════

fn compute_definition(ctx: &AnalysisContext) -> Vec<Location> {
    let (symbol_name, _, _) = match find_symbol_at_cursor(ctx) {
        Some(s) => s,
        None => return vec![],
    };

    // Check if the symbol is an import — resolve to source file
    let primary = match ctx.files.get(&ctx.primary_file) {
        Some(p) => p,
        None => return vec![],
    };
    for imp in &primary.program.imports {
        match imp {
            ImportStmt::Named(n) if n.names.contains(&symbol_name) => {
                let resolved = resolve_import_path(&ctx.primary_file, &n.path, &ctx.files);
                if let Some(target_symbols) = ctx.cross_symbols.get(&resolved) {
                    for (name, _, span) in target_symbols {
                        if name == &symbol_name {
                            return vec![Location {
                                file_path: resolved.clone(),
                                range: span_to_lsp_range(span),
                            }];
                        }
                    }
                }
            }
            ImportStmt::TypeOnly(t) if t.names.contains(&symbol_name) => {
                let resolved = resolve_import_path(&ctx.primary_file, &t.path, &ctx.files);
                if let Some(target_symbols) = ctx.cross_symbols.get(&resolved) {
                    for (name, _, span) in target_symbols {
                        if name == &symbol_name {
                            return vec![Location {
                                file_path: resolved.clone(),
                                range: span_to_lsp_range(span),
                            }];
                        }
                    }
                }
            }
            ImportStmt::Namespace(ns) if ns.alias == symbol_name => {
                // Namespace import: jump to the imported file itself
                let resolved = resolve_import_path(&ctx.primary_file, &ns.path, &ctx.files);
                return vec![Location {
                    file_path: resolved.clone(),
                    range: LspRange { start_line: 1, start_col: 1, end_line: 1, end_col: 1 },
                }];
            }
            _ => {}
        }
    }

    // Search all files for the definition of this symbol
    for (file_path, symbols) in &ctx.cross_symbols {
        for (name, kind, span) in symbols {
            if name == &symbol_name && is_definition_kind(kind) {
                return vec![Location {
                    file_path: file_path.clone(),
                    range: span_to_lsp_range(span),
                }];
            }
        }
    }

    vec![]
}

fn is_definition_kind(kind: &SymbolKind) -> bool {
    matches!(kind,
        SymbolKind::Function | SymbolKind::Struct | SymbolKind::TypeAlias
        | SymbolKind::Component | SymbolKind::Store | SymbolKind::Signal
        | SymbolKind::Computed | SymbolKind::Prop | SymbolKind::Action
        | SymbolKind::Variable | SymbolKind::Param
    )
}

// ════════════════════════════════════════════════════════
// References
// ════════════════════════════════════════════════════════

fn compute_references(ctx: &AnalysisContext) -> Vec<Location> {
    let (symbol_name, _, _) = match find_symbol_at_cursor(ctx) {
        Some(s) => s,
        None => return vec![],
    };

    let mut refs = Vec::new();
    for (file_path, pf) in &ctx.files {
        find_var_refs_in_program(&pf.program, &symbol_name, file_path, &mut refs);
    }
    refs
}

fn find_var_refs_in_program(
    program: &Program,
    search_name: &str,
    file_path: &str,
    refs: &mut Vec<Location>,
) {
    for func in &program.functions {
        collect_refs_from_block(&func.body, search_name, file_path, refs);
    }
    for comp in &program.components {
        for sig in &comp.signals {
            collect_refs_from_expr(&sig.init, search_name, file_path, refs);
        }
        for c in &comp.computeds {
            collect_refs_from_expr(&c.body, search_name, file_path, refs);
        }
        for f in &comp.fns {
            collect_refs_from_block(&f.body, search_name, file_path, refs);
        }
        for a in &comp.actions {
            collect_refs_from_block(&a.body, search_name, file_path, refs);
        }
        for lc in &comp.lifecycles {
            collect_refs_from_block(&lc.body, search_name, file_path, refs);
        }
    }
    for store in &program.stores {
        for sig in &store.signals {
            collect_refs_from_expr(&sig.init, search_name, file_path, refs);
        }
        for g in &store.getters {
            collect_refs_from_expr(&g.body, search_name, file_path, refs);
        }
        for a in &store.actions {
            collect_refs_from_block(&a.body, search_name, file_path, refs);
        }
    }
}

fn collect_refs_from_block(
    block: &Block,
    search_name: &str,
    file_path: &str,
    refs: &mut Vec<Location>,
) {
    for stmt in &block.stmts {
        collect_refs_from_stmt(stmt, search_name, file_path, refs);
    }
    if let Some(ref trailing) = block.trailing_expr {
        collect_refs_from_expr(trailing, search_name, file_path, refs);
    }
}

fn collect_refs_from_stmt(
    stmt: &Stmt,
    search_name: &str,
    file_path: &str,
    refs: &mut Vec<Location>,
) {
    match stmt {
        Stmt::VarDecl { name, init, span, .. } => {
            if name == search_name {
                refs.push(Location { file_path: file_path.to_string(), range: span_to_lsp_range(span) });
            }
            if let Some(ref init_expr) = init {
                collect_refs_from_expr(init_expr, search_name, file_path, refs);
            }
        }
        Stmt::Assign { target: assign_target, value, span, .. } => {
            if let AssignTarget::Name(name) = assign_target {
                if name == search_name {
                    refs.push(Location { file_path: file_path.to_string(), range: span_to_lsp_range(span) });
                }
            }
            collect_refs_from_expr(value, search_name, file_path, refs);
        }
        Stmt::ExprStmt { expr, .. } => collect_refs_from_expr(expr, search_name, file_path, refs),
        Stmt::Return { value: Some(expr), .. } => collect_refs_from_expr(expr, search_name, file_path, refs),
        Stmt::Return { value: None, .. } => {}
        Stmt::Print { arg, .. } => collect_refs_from_expr(arg, search_name, file_path, refs),
        Stmt::If { condition, then_body, else_ifs, else_body, .. } => {
            collect_refs_from_expr(condition, search_name, file_path, refs);
            collect_refs_from_block(then_body, search_name, file_path, refs);
            for (elif_cond, elif_body) in else_ifs {
                collect_refs_from_expr(elif_cond, search_name, file_path, refs);
                collect_refs_from_block(elif_body, search_name, file_path, refs);
            }
            if let Some(ref else_b) = else_body {
                collect_refs_from_block(else_b, search_name, file_path, refs);
            }
        }
        Stmt::While { condition, body, .. } => {
            collect_refs_from_expr(condition, search_name, file_path, refs);
            collect_refs_from_block(body, search_name, file_path, refs);
        }
        Stmt::ForIn { iterable, body, .. } => {
            collect_refs_from_expr(iterable, search_name, file_path, refs);
            collect_refs_from_block(body, search_name, file_path, refs);
        }
        Stmt::EventEmit { args, .. } => {
            for arg in args {
                collect_refs_from_expr(arg, search_name, file_path, refs);
            }
        }
        Stmt::Break(_) | Stmt::Continue(_) => {}
    }
}

fn collect_refs_from_expr(
    expr: &Expr,
    search_name: &str,
    file_path: &str,
    refs: &mut Vec<Location>,
) {
    match expr {
        Expr::VarRef(name, span) if name == search_name => {
            refs.push(Location { file_path: file_path.to_string(), range: span_to_lsp_range(span) });
        }
        Expr::FuncCall { func, args, .. } => {
            collect_refs_from_expr(func, search_name, file_path, refs);
            for arg in args { collect_refs_from_expr(arg, search_name, file_path, refs); }
        }
        Expr::MethodCall { object, args, .. } => {
            collect_refs_from_expr(object, search_name, file_path, refs);
            for arg in args { collect_refs_from_expr(arg, search_name, file_path, refs); }
        }
        Expr::BinaryOp { left, right, .. } => {
            collect_refs_from_expr(left, search_name, file_path, refs);
            collect_refs_from_expr(right, search_name, file_path, refs);
        }
        Expr::UnaryOp { operand, .. } => collect_refs_from_expr(operand, search_name, file_path, refs),
        Expr::FieldAccess { object, .. } => collect_refs_from_expr(object, search_name, file_path, refs),
        Expr::IndexGet { object, index, .. } => {
            collect_refs_from_expr(object, search_name, file_path, refs);
            collect_refs_from_expr(index, search_name, file_path, refs);
        }
        Expr::IfExpr { condition, then_expr, else_expr, .. }
        | Expr::TernaryExpr { condition, then_expr, else_expr, .. } => {
            collect_refs_from_expr(condition, search_name, file_path, refs);
            collect_refs_from_expr(then_expr, search_name, file_path, refs);
            collect_refs_from_expr(else_expr, search_name, file_path, refs);
        }
        Expr::ArrayLiteral { elements, .. } => {
            for el in elements { collect_refs_from_expr(el, search_name, file_path, refs); }
        }
        Expr::MapLiteral { entries, .. } => {
            for (k, v) in entries {
                collect_refs_from_expr(k, search_name, file_path, refs);
                collect_refs_from_expr(v, search_name, file_path, refs);
            }
        }
        Expr::TemplateString { parts, .. } => {
            for part in parts {
                if let TemplatePart::Expr(e) = part {
                    collect_refs_from_expr(e, search_name, file_path, refs);
                }
            }
        }
        Expr::StructLiteral { type_name, fields, span } => {
            if type_name == search_name {
                refs.push(Location { file_path: file_path.to_string(), range: span_to_lsp_range(span) });
            }
            for (_, val) in fields {
                collect_refs_from_expr(val, search_name, file_path, refs);
            }
        }
        Expr::ClosureExpr { body, .. } => {
            collect_refs_from_expr(body, search_name, file_path, refs);
        }
        Expr::StoreFieldAccess { store_alias, span, .. } => {
            if store_alias == search_name {
                refs.push(Location { file_path: file_path.to_string(), range: span_to_lsp_range(span) });
            }
        }
        // Expr variants without sub-expressions that can contain VarRef:
        Expr::VarRef(_, _) | Expr::Literal(_, _)
        | Expr::NotYetImplemented { .. } => {}
    }
}

// ════════════════════════════════════════════════════════
// Completion
// ════════════════════════════════════════════════════════

const KIND_KEYWORD: i32 = 14;
const KIND_FUNCTION: i32 = 3;
const KIND_VARIABLE: i32 = 6;
const KIND_STRUCT: i32 = 22;
const KIND_CLASS: i32 = 7;
const KIND_FIELD: i32 = 5;
const KIND_MODULE: i32 = 9;

const OMNILANG_KEYWORDS: &[(&str, &str)] = &[
    ("fn", "fn name(params) -> Type { ... }"),
    ("let", "let name: Type = value;"),
    ("mut", "mut — mutable binding modifier"),
    ("if", "if condition { ... }"),
    ("else", "else { ... }"),
    ("for", "for var in iterable { ... }"),
    ("while", "while condition { ... }"),
    ("match", "match value { pattern => body, ... }"),
    ("return", "return value;"),
    ("struct", "struct Name { field: Type, ... }"),
    ("import", "import { Name } from \"./path\";"),
    ("export", "export fn name(...) -> Type { ... }"),
    ("type", "type Name<T> = Variant1 | Variant2"),
    ("async", "async fn name(...) -> Type { ... }"),
    ("await", "await expr"),
    ("true", "true — boolean literal"),
    ("false", "false — boolean literal"),
    ("null", "null — absence of value"),
    ("break", "break;"),
    ("continue", "continue;"),
];

const OMNIVIEW_KEYWORDS: &[(&str, &str)] = &[
    ("component", "component Name { ... }"),
    ("store", "store Name { ... }"),
    ("signal", "signal name: Type = value;"),
    ("prop", "prop name: Type;"),
    ("computed", "computed name: Type = expr;"),
    ("action", "action name(params) { ... }"),
    ("view", "view { ... }"),
    ("style", "style { ... }"),
    ("ref", "ref name: Type;"),
];

fn compute_completion(ctx: &AnalysisContext) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    // 1. Keywords (always available)
    for (kw, detail) in OMNILANG_KEYWORDS {
        items.push(CompletionItem {
            label: kw.to_string(),
            kind: Some(KIND_KEYWORD),
            detail: Some(detail.to_string()),
            documentation: None,
            insert_text: Some(kw.to_string()),
            sort_text: Some(format!("01_{}", kw)),
        });
    }

    // 2. OmniView keywords if file has omniview constructs
    let primary = ctx.files.get(&ctx.primary_file);
    let is_omniview = primary.map_or(false, |pf| {
        pf.program.imports.iter().any(|imp| {
            matches!(imp, ImportStmt::Lib(name) if name == "omniview")
        }) || !pf.program.components.is_empty()
            || !pf.program.stores.is_empty()
    });

    if is_omniview {
        for (kw, detail) in OMNIVIEW_KEYWORDS {
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: Some(KIND_KEYWORD),
                detail: Some(detail.to_string()),
                documentation: None,
                insert_text: Some(kw.to_string()),
                sort_text: Some(format!("00_{}", kw)),
            });
        }
    }

    // 3. Cross-file symbols
    for (file_path, symbols) in &ctx.cross_symbols {
        for (name, kind, _span) in symbols {
            let lsp_kind = symbol_kind_to_completion_kind(kind);
            let detail = format!("{} (from {})",
                symbol_kind_display(kind),
                short_file_name(file_path),
            );
            items.push(CompletionItem {
                label: name.clone(),
                kind: Some(lsp_kind),
                detail: Some(detail),
                documentation: None,
                insert_text: Some(name.clone()),
                sort_text: Some(format!("10_{}", name)),
            });
        }
    }

    items
}

fn symbol_kind_to_completion_kind(kind: &SymbolKind) -> i32 {
    match kind {
        SymbolKind::Function | SymbolKind::Action => KIND_FUNCTION,
        SymbolKind::Variable | SymbolKind::Param | SymbolKind::Signal
            | SymbolKind::Computed | SymbolKind::Prop => KIND_VARIABLE,
        SymbolKind::Struct | SymbolKind::TypeAlias => KIND_STRUCT,
        SymbolKind::Component | SymbolKind::Theme => KIND_CLASS,
        SymbolKind::Store | SymbolKind::Import | SymbolKind::Export => KIND_MODULE,
        SymbolKind::Event => KIND_FIELD,
    }
}

fn symbol_kind_display(kind: &SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Function => "fn",
        SymbolKind::Variable => "var",
        SymbolKind::Struct => "struct",
        SymbolKind::TypeAlias => "type",
        SymbolKind::Component => "component",
        SymbolKind::Store => "store",
        SymbolKind::Signal => "signal",
        SymbolKind::Computed => "computed",
        SymbolKind::Prop => "prop",
        SymbolKind::Action => "action",
        SymbolKind::Event => "event",
        SymbolKind::Theme => "theme",
        SymbolKind::Import => "import",
        SymbolKind::Export => "export",
        SymbolKind::Param => "param",
    }
}

fn short_file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

// ════════════════════════════════════════════════════════
// Semantic Tokens
// ════════════════════════════════════════════════════════

/// Known type identifiers that should receive type highlighting.
const TYPE_IDENTS: &[&str] = &[
    "Int", "Float", "Double", "String", "Bool", "Void",
    "List", "Map", "Set", "Option", "Result",
];

fn compute_semantic_tokens(ctx: &AnalysisContext) -> Vec<u32> {
    let primary = match ctx.files.get(&ctx.primary_file) {
        Some(pf) => pf,
        None => return vec![],
    };

    let mut tokens: Vec<u32> = Vec::new();
    let mut prev_line: u32 = 0;
    let mut prev_col: u32 = 0;

    for tok in &primary.tokens {
        let (token_type, modifiers): (u32, u32) = match &tok.kind {
            // Keywords → type 0
            TokenKind::Fn | TokenKind::Const | TokenKind::Let | TokenKind::Mut
            | TokenKind::If | TokenKind::Else | TokenKind::While | TokenKind::For | TokenKind::In
            | TokenKind::Return | TokenKind::Break | TokenKind::Continue
            | TokenKind::Print | TokenKind::Println
            | TokenKind::Async | TokenKind::Await
            | TokenKind::Match | TokenKind::Type
            | TokenKind::Struct | TokenKind::Class
            | TokenKind::Import | TokenKind::From | TokenKind::Export | TokenKind::As
            | TokenKind::Pub | TokenKind::Private | TokenKind::Static | TokenKind::Override
            | TokenKind::True | TokenKind::False | TokenKind::Null
            | TokenKind::Component | TokenKind::Store | TokenKind::Signal
            | TokenKind::Computed | TokenKind::Prop | TokenKind::Event | TokenKind::Ref
            | TokenKind::View | TokenKind::Style | TokenKind::Action | TokenKind::Getter
            | TokenKind::Theme | TokenKind::Bind => (0, 0),

            // String literals → type 4
            TokenKind::StringLiteral(_) | TokenKind::TemplateString(_) => (4, 0),

            // Number literals → type 5
            TokenKind::IntLiteral(_) | TokenKind::FloatLiteral(_) => (5, 0),

            // Identifiers → check if known type (type 1) or variable/function name (type 3)
            TokenKind::Ident(name) => {
                if TYPE_IDENTS.contains(&name.as_str()) {
                    (1, 0) // type
                } else {
                    (3, 0) // variable/function
                }
            }

            _ => continue,
        };

        let line = tok.span.line.saturating_sub(1);
        let col = tok.span.col.saturating_sub(1);

        let delta_line = line.saturating_sub(prev_line);
        let delta_col = if delta_line == 0 { col.saturating_sub(prev_col) } else { col };
        let length = (tok.span.end - tok.span.start) as u32;

        tokens.push(delta_line);
        tokens.push(delta_col);
        tokens.push(length);
        tokens.push(token_type);
        tokens.push(modifiers);

        prev_line = line;
        prev_col = col;
    }

    tokens
}

// ════════════════════════════════════════════════════════
// Document Symbols
// ════════════════════════════════════════════════════════

const SYM_FUNCTION: i32 = 12;
const SYM_STRUCT: i32 = 23;
const SYM_CLASS: i32 = 5;
const SYM_NAMESPACE: i32 = 3;

fn compute_document_symbols(ctx: &AnalysisContext) -> Vec<SymbolInfo> {
    let primary = match ctx.files.get(&ctx.primary_file) {
        Some(pf) => pf,
        None => return vec![],
    };

    let mut symbols = Vec::new();

    for comp in &primary.program.components {
        symbols.push(SymbolInfo {
            name: comp.name.clone(),
            kind: SYM_CLASS,
            range: Some(span_to_lsp_range(&comp.span)),
            selection_range: Some(span_to_lsp_range(&comp.span)),
            children: vec![],
        });
    }

    for st in &primary.program.stores {
        // StoreDef has no span field — omit range
        symbols.push(SymbolInfo {
            name: st.name.clone(),
            kind: SYM_NAMESPACE,
            range: None,
            selection_range: None,
            children: vec![],
        });
    }

    for func in &primary.program.functions {
        symbols.push(SymbolInfo {
            name: func.name.clone(),
            kind: SYM_FUNCTION,
            range: Some(span_to_lsp_range(&func.span)),
            selection_range: Some(span_to_lsp_range(&func.span)),
            children: vec![],
        });
    }

    for s in &primary.program.structs {
        symbols.push(SymbolInfo {
            name: s.name.clone(),
            kind: SYM_STRUCT,
            range: Some(span_to_lsp_range(&s.span)),
            selection_range: Some(span_to_lsp_range(&s.span)),
            children: vec![],
        });
    }

    symbols
}

// ════════════════════════════════════════════════════════
// Folding Ranges
// ════════════════════════════════════════════════════════

fn compute_folding_ranges(ctx: &AnalysisContext) -> Vec<FoldingRange> {
    let primary = match ctx.files.get(&ctx.primary_file) {
        Some(pf) => pf,
        None => return vec![],
    };

    let mut folds = Vec::new();
    let source = &primary.content;
    let mut brace_stack: Vec<(usize, u32)> = Vec::new();

    // Precompute line start offsets for O(1) line-number lookups
    let line_starts: Vec<usize> = std::iter::once(0)
        .chain(source.char_indices().filter_map(|(i, c)| {
            if c == '\n' { Some(i + 1) } else { None }
        }))
        .collect();

    for (char_pos, ch) in source.char_indices() {
        // Binary search for line number
        let line = match line_starts.binary_search(&char_pos) {
            Ok(idx) => idx as u32 + 1,
            Err(idx) => idx as u32, // idx = number of newlines before this position
        };
        match ch {
            '{' => brace_stack.push((char_pos, line)),
            '}' => {
                if let Some((_, open_line)) = brace_stack.pop() {
                    if open_line < line {
                        folds.push(FoldingRange {
                            start_line: open_line,
                            end_line: line,
                            kind: Some("region".to_string()),
                        });
                    }
                }
            }
            _ => {}
        }
    }

    folds
}

// ════════════════════════════════════════════════════════
// Signature Help
// ════════════════════════════════════════════════════════

fn compute_signature_help(ctx: &AnalysisContext) -> Option<SignatureInfo> {
    let primary = ctx.files.get(&ctx.primary_file)?;
    let (cursor_line, cursor_col) = byte_offset_to_line_col(&primary.content, ctx.cursor_offset);
    find_func_call_at(&primary.program, &primary.content, cursor_line, cursor_col)
}

fn find_func_call_at(program: &Program, source: &str, line: u32, col: u32) -> Option<SignatureInfo> {
    for func in &program.functions {
        if let Some(sig) = find_call_in_block(&func.body, source, line, col) {
            return Some(sig);
        }
    }
    for comp in &program.components {
        for f in &comp.fns {
            if let Some(sig) = find_call_in_block(&f.body, source, line, col) {
                return Some(sig);
            }
        }
        for a in &comp.actions {
            if let Some(sig) = find_call_in_block(&a.body, source, line, col) {
                return Some(sig);
            }
        }
        if let Some(ref view) = comp.view {
            for child in &view.children {
                if let Some(sig) = find_call_in_view_node(child, source, line, col) {
                    return Some(sig);
                }
            }
        }
    }
    None
}

fn find_call_in_view_node(node: &ViewNode, source: &str, line: u32, col: u32) -> Option<SignatureInfo> {
    match node {
        ViewNode::Element(el) => {
            for prop in &el.props {
                if let ElementProp::Static { value, .. } = prop {
                    if let Some(sig) = find_call_in_expr(value, source, line, col) {
                        return Some(sig);
                    }
                }
            }
            for child in &el.children {
                if let Some(sig) = find_call_in_view_node(child, source, line, col) {
                    return Some(sig);
                }
            }
            None
        }
        ViewNode::Interpolation(expr) => find_call_in_expr(expr, source, line, col),
        ViewNode::IfDirective(if_dir) => {
            if let Some(sig) = find_call_in_expr(&if_dir.condition, source, line, col) {
                return Some(sig);
            }
            for node in &if_dir.body {
                if let Some(sig) = find_call_in_view_node(node, source, line, col) {
                    return Some(sig);
                }
            }
            for (cond, body) in &if_dir.else_ifs {
                if let Some(sig) = find_call_in_expr(cond, source, line, col) {
                    return Some(sig);
                }
                for node in body {
                    if let Some(sig) = find_call_in_view_node(node, source, line, col) {
                        return Some(sig);
                    }
                }
            }
            if let Some(ref body) = if_dir.else_body {
                for node in body {
                    if let Some(sig) = find_call_in_view_node(node, source, line, col) {
                        return Some(sig);
                    }
                }
            }
            None
        }
        ViewNode::ForDirective(for_dir) => {
            if let Some(sig) = find_call_in_expr(&for_dir.iterable, source, line, col) {
                return Some(sig);
            }
            for node in &for_dir.body {
                if let Some(sig) = find_call_in_view_node(node, source, line, col) {
                    return Some(sig);
                }
            }
            if let Some(ref empty) = for_dir.empty_body {
                for node in empty {
                    if let Some(sig) = find_call_in_view_node(node, source, line, col) {
                        return Some(sig);
                    }
                }
            }
            None
        }
        ViewNode::Text(_) | ViewNode::SlotDirective { .. } => None,
    }
}

fn find_call_in_block(block: &Block, source: &str, line: u32, col: u32) -> Option<SignatureInfo> {
    for stmt in &block.stmts {
        match stmt {
            Stmt::ExprStmt { expr, .. } | Stmt::Return { value: Some(expr), .. } => {
                if let Some(sig) = find_call_in_expr(expr, source, line, col) {
                    return Some(sig);
                }
            }
            Stmt::VarDecl { init: Some(init), .. } => {
                if let Some(sig) = find_call_in_expr(init, source, line, col) {
                    return Some(sig);
                }
            }
            _ => {}
        }
    }
    if let Some(ref trailing) = block.trailing_expr {
        return find_call_in_expr(trailing, source, line, col);
    }
    None
}

fn find_call_in_expr(expr: &Expr, source: &str, line: u32, col: u32) -> Option<SignatureInfo> {
    match expr {
        Expr::FuncCall { func, args, span } => {
            if span_contains(source, span, line, col) {
                let func_name = match func.as_ref() {
                    Expr::VarRef(name, _) => name.clone(),
                    _ => "unknown".to_string(),
                };
                let params: Vec<ParameterInfo> = args.iter().enumerate().map(|(i, _)| {
                    ParameterInfo { label: format!("arg{}", i), documentation: None }
                }).collect();
                return Some(SignatureInfo {
                    label: format!("{}({})", func_name,
                        args.iter().enumerate().map(|(i, _)| format!("arg{}", i))
                            .collect::<Vec<_>>().join(", ")),
                    parameters: params,
                    active_parameter: Some(args.len().saturating_sub(1) as i32),
                });
            }
            for arg in args {
                if let Some(sig) = find_call_in_expr(arg, source, line, col) { return Some(sig); }
            }
            None
        }
        Expr::MethodCall { method, args, span, .. } => {
            if span_contains(source, span, line, col) {
                let params: Vec<ParameterInfo> = args.iter().enumerate().map(|(i, _)| {
                    ParameterInfo { label: format!("arg{}", i), documentation: None }
                }).collect();
                return Some(SignatureInfo {
                    label: format!(".{}({})", method,
                        args.iter().enumerate().map(|(i, _)| format!("arg{}", i))
                            .collect::<Vec<_>>().join(", ")),
                    parameters: params,
                    active_parameter: Some(args.len().saturating_sub(1) as i32),
                });
            }
            None
        }
        Expr::BinaryOp { left, right, span, .. } => {
            if span_contains(source, span, line, col) {
                return find_call_in_expr(left, source, line, col)
                    .or_else(|| find_call_in_expr(right, source, line, col));
            }
            None
        }
        _ => None,
    }
}

// ════════════════════════════════════════════════════════
// Rename
// ════════════════════════════════════════════════════════

fn compute_rename(ctx: &AnalysisContext) -> (Vec<TextEdit>, Option<LspRange>) {
    let refs = compute_references(ctx);
    let rename_range = find_symbol_at_cursor(ctx).map(|(_, _, span)| span_to_lsp_range(&span));

    let edits = refs.into_iter().map(|loc| TextEdit {
        range: loc.range,
        new_text: String::new(), // caller fills from Extra.new_name
        file_path: Some(loc.file_path),
    }).collect();

    (edits, rename_range)
}

// ════════════════════════════════════════════════════════
// Main dispatch
// ════════════════════════════════════════════════════════

/// Main entry point — stateless analysis for all LSP operations.
pub fn analyze(request: AnalysisRequest) -> AnalysisResult {
    let ctx = AnalysisContext::new(&request);

    let (diagnostics, resolved_imports) = if matches!(request.operation, Operation::Diagnostics) {
        (compute_diagnostics(&ctx), compute_resolved_imports(&ctx))
    } else {
        (vec![], vec![])
    };

    let mut result = PerFileResult {
        diagnostics,
        resolved_imports,
        ..Default::default()
    };

    match request.operation {
        Operation::Diagnostics => {} // already populated above

        Operation::Hover => {
            result.hover = compute_hover(&ctx);
        }

        Operation::Definition => {
            result.definitions = compute_definition(&ctx);
        }

        Operation::References => {
            result.references = compute_references(&ctx);
        }

        Operation::Completion => {
            result.items = compute_completion(&ctx);
        }

        Operation::SemanticTokens => {
            result.semantic_tokens = compute_semantic_tokens(&ctx);
        }

        Operation::DocumentSymbol => {
            result.symbols = compute_document_symbols(&ctx);
        }

        Operation::FoldingRanges => {
            result.folds = compute_folding_ranges(&ctx);
        }

        Operation::SignatureHelp => {
            result.signature = compute_signature_help(&ctx);
        }

        Operation::Rename => {
            let (edits, range) = compute_rename(&ctx);
            let new_name = match request.extra.new_name {
                Some(ref n) if !n.is_empty() => n.clone(),
                _ => String::new(), // invalid/missing → edits remain empty below
            };
            if !new_name.is_empty() {
                result.edits = edits.into_iter().map(|mut e| {
                    e.new_text = new_name.clone();
                    e
                }).collect();
            }
            result.rename_range = range;
        }

        Operation::PrepareRename => {
            result.rename_range = find_symbol_at_cursor(&ctx)
                .map(|(_, _, span)| span_to_lsp_range(&span));
        }
    }

    AnalysisResult {
        success: true,
        primary_result: result,
        other_diagnostics: vec![],
    }
}

// ════════════════════════════════════════════════════════
// Formatting
// ════════════════════════════════════════════════════════

/// Format .omni source code -- used by both LSP and CLI.
/// Performs consistent 4-space indentation based on brace depth,
/// removes trailing whitespace, ensures single trailing newline.
pub fn format_source(source: &str) -> String {
    let tokens = crate::lexer::tokenize(source);
    let mut formatted: Vec<String> = Vec::new();
    let mut indent_level: usize = 0;
    let indent_str = "    "; // 4 spaces
    let mut last_pos = 0usize;

    for tok in &tokens {
        // Use safe byte-slicing to avoid panics on non-UTF-8 boundaries
        let text = source
            .get(tok.span.start..tok.span.end)
            .unwrap_or("");

        // Emit any newlines between tokens as line breaks
        let between = source
            .get(last_pos..tok.span.start)
            .unwrap_or("");
        for ch in between.chars() {
            if ch == '\n' {
                formatted.push(String::new()); // empty = newline marker
            }
        }

        match &tok.kind {
            TokenKind::LBrace => {
                // Opening brace on the same line
                if let Some(last) = formatted.last_mut() {
                    last.push_str(" {");
                } else {
                    formatted.push(format!("{}{{", indent_str.repeat(indent_level)));
                }
                indent_level += 1;
            }
            TokenKind::RBrace => {
                indent_level = indent_level.saturating_sub(1);
                formatted.push(format!("{}}}", indent_str.repeat(indent_level)));
            }
            TokenKind::Semicolon => {
                if let Some(last) = formatted.last_mut() {
                    last.push(';');
                }
            }
            _ => {
                // Regular token -- emit with current indentation
                if formatted.is_empty() {
                    formatted.push(format!("{}{}", indent_str.repeat(indent_level), text));
                } else if formatted.last().map_or(true, |s| s.is_empty()) {
                    // After a newline, indent
                    formatted.push(format!("{}{}", indent_str.repeat(indent_level), text));
                } else if let Some(last) = formatted.last_mut() {
                    last.push(' ');
                    last.push_str(text);
                }
            }
        }

        last_pos = tok.span.end;
    }

    // Join lines, trim trailing whitespace, ensure single trailing newline
    let result = formatted.join("\n");
    let result = result
        .lines()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n");
    result.trim_end().to_string() + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_offset_to_line_col_ascii() {
        let source = "fn greet(name: String) -> String {\n    return name;\n}\n";
        // offset 3 should be 'g' of greet: line 1, col 4
        let (line, col) = byte_offset_to_line_col(source, 3);
        assert_eq!((line, col), (1, 4), "offset 3");
        // Test span end
        let (end_line, end_col) = byte_offset_to_line_col(source, source.len());
        println!("end: line={}, col={}", end_line, end_col);
    }

    #[test]
    fn test_find_symbol_at_cursor_ascii() {
        let source = "fn greet(name: String) -> String {\n    return name;\n}\n";
        let request = AnalysisRequest {
            files: vec![SourceFile {
                path: "test.omni".to_string(),
                content: source.to_string(),
                is_open: true,
            }],
            primary_file: "test.omni".to_string(),
            operation: Operation::Hover,
            cursor_offset: 3,
            extra: Extra::default(),
        };
        let ctx = AnalysisContext::new(&request);

        // Check cross_symbols
        let syms = ctx.cross_symbols.get("test.omni").unwrap();
        println!("Symbols count: {}", syms.len());
        for (name, kind, span) in syms {
            println!("  {}: line={} col={} start={} end={}",
                name, span.line, span.col, span.start, span.end);
        }

        // Try find_symbol_at_cursor
        let result = find_symbol_at_cursor(&ctx);
        println!("find_symbol_at_cursor at offset 3: {:?}", result.is_some());
        assert!(result.is_some(), "Should find 'greet' at offset 3");

        // Test at offset 3 (should find 'greet')
        let ctx3 = AnalysisContext::new(&AnalysisRequest {
            cursor_offset: 3,
            ..request
        });
        let result3 = find_symbol_at_cursor(&ctx3);
        assert!(result3.is_some(), "Should find 'greet' at offset 3");
        assert_eq!(result3.unwrap().0, "greet");

        // Test at usage site: "greet" is called at byte offset 53
        // Source: "fn greet(name: String) -> String {\n    return name;\n}\n"
        // Line 2: "    return name;" — 'name' is at byte 49-53
        // Let's test with a source that has a usage:
        let source2 = "fn greet(name: String) -> String {\n    return name;\n}\n\nfn main() {\n    greet(\"World\");\n}\n";
        // 'greet' at usage: "    greet(\"World\");" — 'g' at col 5 (0-indexed)
        // Line 5 is the 6th line: offset = 66 (5 lines before) + 5 (col 4, 0-indexed) = 71
        // Wait, debug showed offset 70 = col 4 (space), offset 71 = col 5 ('g')
        let greet_usage_offset = 71usize;
        let ctx_usage = AnalysisContext::new(&AnalysisRequest {
            files: vec![SourceFile {
                path: "test.omni".to_string(),
                content: source2.to_string(),
                is_open: true,
            }],
            primary_file: "test.omni".to_string(),
            operation: Operation::Hover,
            cursor_offset: greet_usage_offset,
            extra: Extra::default(),
        });
        let syms2 = ctx_usage.cross_symbols.get("test.omni").unwrap();
        println!("Symbols count: {}", syms2.len());
        for (name, kind, span) in syms2 {
            println!("  {}: line={} col={} start={} end={}",
                name, span.line, span.col, span.start, span.end);
        }
        let result_usage = find_symbol_at_cursor(&ctx_usage);
        println!("find_symbol_at_cursor at usage offset {}: {:?}", greet_usage_offset, result_usage);
        assert!(result_usage.is_some(), "Should find 'greet' at usage site offset {}", greet_usage_offset);
        assert_eq!(result_usage.unwrap().0, "greet");
    }
}
