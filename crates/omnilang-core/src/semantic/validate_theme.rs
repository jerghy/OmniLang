//! Program-level theme validation — v2 域 3/域 5 (design D1 语法层).
//!
//! Runs on the FULL IR program — at the end of `run_pipeline` (single-file)
//! and after IR merge in `analyse_multi` / `analyse_multi_cached`
//! (multi-file) — because the `@theme`-marked signal (top-level store) and
//! the `@themes { ... }` declaration (design block) may live in different
//! source files (the learn-omniview demo shape: `design.omni` +
//! `stores/ThemeStore.omni`). A per-file semantic pass cannot see the pair.
//!
//! Checks (semantic DSL v2 plan Task 4.1 / design D1):
//! - (a) `@theme` signal used but no design in the program declares
//!   `@themes` → error「未声明 @themes」
//! - (b) `@theme` on a non-String signal → error「@theme 信号必须是 String」
//!   (verbatim from the design doc 域 5 diagnostic table). `String?` is
//!   rejected too — nullability is `TypeIR.is_nullable`, not a TypeKind
//!   variant (fix round 1/5, F2).
//! - (c) more than one `@theme` signal in the program → error
//! - (e) duplicate theme names inside `@themes { ... }` → error
//! - (f) empty `@themes { }` → parse error (in `parser/design.rs`) — the IR
//!   cannot distinguish an explicit empty declaration (`themes: []`) from
//!   no declaration, so the check lives where the source shape is visible.
//! - (F1) a LITERAL string initializer of the `@theme` signal must be a
//!   member of the declared `@themes` enum (design doc 域 3 BNF annotation:
//!   「@theme 值必须是 @themes 枚举之一（语义验证）」). Runtime-assigned
//!   values (non-literal initializers) are NOT statically checkable and are
//!   deliberately left unchecked — the boundary is documented here and in
//!   the spec §4.5.
//! - (D-F) `@default-theme X;` (design directive) and the `@theme` signal's
//!   LITERAL initial are two declarations of the program's startup theme and
//!   must agree — otherwise React starts from @default-theme while Flutter
//!   only honors the signal initial for "dark" (silent cross-backend startup
//!   divergence, domain MEDIUM-2). Mismatch → error naming both values and
//!   both source positions.
//!
//! All diagnostics anchor at the source span of their store / design block
//! (validate_effect_deps precedent, fix round 1/5 F4).
//!
//! Also populates `DesignIR.theme_signal = "StoreName.signalName"` for every
//! design with a non-empty theme set (wiring-ready shape consumed by codegen
//! in Task 4.2).

use crate::error::{Diagnostic, DiagnosticKind, Severity};
use crate::ir::{IRProgram, TypeKind};
use crate::lexer::Span;

/// (b) verbatim from the design doc 域 5 diagnostic table.
const NON_STRING_MSG: &str = "必须是 String";

/// Non-literal store-signal initializers serialize to this sentinel in
/// `StoreIR.signal_defaults` (see `lower_store`, ir/lower/component.rs).
/// The F1 membership check treats it as "runtime value — not statically
/// checkable".
const RUNTIME_EXPR_SENTINEL: &str = "<runtime expr>";

/// A store signal marked `@theme`, collected across all modules.
struct MarkedThemeSignal {
    store: String,
    signal: String,
    file: Option<String>,
    kind: TypeKind,
    nullable: bool,
    /// Literal string initializer, when statically known. `None` for
    /// non-literal (runtime) initializers — those cannot be checked.
    initial: Option<String>,
    span: Option<Span>,
}

/// Format `file:line:col` for embedding a source position in a diagnostic
/// message (D-F: the @default-theme mismatch error carries BOTH sides'
/// positions). Empty string when the source location is unknown.
fn pos_text(file: &Option<String>, span: &Option<Span>) -> String {
    match (file.as_deref(), span) {
        (Some(f), Some(s)) => format!("{}:{}:{}", f, s.line, s.col),
        _ => String::new(),
    }
}

fn diag(message: String, file: Option<String>, span: Option<Span>) -> Diagnostic {
    let (line, col) = span.map(|s| (s.line, s.col)).unwrap_or((0, 0));
    Diagnostic {
        severity: Severity::Error,
        kind: DiagnosticKind::E0006,
        message,
        file,
        line,
        col,
        end_line: None,
        end_col: None,
        suggestion: None,
    }
}

/// Validate `@themes` ↔ `@theme` pairing and populate `DesignIR.theme_signal`.
pub fn validate_theme(ir: &mut IRProgram, diagnostics: &mut Vec<Diagnostic>) {
    // ── Collect @theme-marked store signals (deterministic order) ──
    let mut marked: Vec<MarkedThemeSignal> = Vec::new();
    for module in ir.modules.values() {
        for store in module.stores.values() {
            if let Some(sig) = &store.theme_signal {
                let ty = store.signals.get(sig);
                marked.push(MarkedThemeSignal {
                    store: store.name.clone(),
                    signal: sig.clone(),
                    file: store.source_file.clone(),
                    span: store.source_span,
                    nullable: ty.map(|t| t.is_nullable).unwrap_or(false),
                    kind: ty.map(|t| t.kind.clone()).unwrap_or(TypeKind::Named {
                        name: "<unknown>".into(),
                    }),
                    // F1: statically-known literal initializer only. The
                    // "<runtime expr>" sentinel marks non-literal inits.
                    initial: match store.signal_defaults.get(sig) {
                        Some(serde_json::Value::String(v)) if v != RUNTIME_EXPR_SENTINEL => {
                            Some(v.clone())
                        }
                        _ => None,
                    },
                });
            }
        }
    }
    marked.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then_with(|| a.store.cmp(&b.store))
            .then_with(|| a.signal.cmp(&b.signal))
    });

    // ── (e) duplicate theme names within each design block ──
    let mut has_themes_declaration = false;
    for design in ir.modules.values().flat_map(|m| m.designs.iter()) {
        if design.themes.is_empty() {
            continue;
        }
        has_themes_declaration = true;
        let mut seen = std::collections::HashSet::with_capacity(design.themes.len());
        for theme in &design.themes {
            if !seen.insert(theme.as_str()) {
                diagnostics.push(diag(
                    format!("[design '{}'] @themes 重复主题 '{}'", design.name, theme),
                    design.source_file.clone(),
                    design.source_span,
                ));
            }
        }
    }

    // ── (b) @theme signal must be String (F2: String? also rejected) ──
    for m in &marked {
        if !matches!(m.kind, TypeKind::String) || m.nullable {
            let msg = if m.nullable && matches!(m.kind, TypeKind::String) {
                format!(
                    "@theme 信号 '{}.{}' 必须是 String（String? 可空类型不允许）",
                    m.store, m.signal
                )
            } else {
                format!("@theme 信号 '{}.{}' {}", m.store, m.signal, NON_STRING_MSG)
            };
            diagnostics.push(diag(msg, m.file.clone(), m.span));
        }
    }

    // ── (a) @theme without any @themes declaration in the program ──
    if !marked.is_empty() && !has_themes_declaration {
        for m in &marked {
            diagnostics.push(diag(
                format!(
                    "@theme 信号 '{}.{}' 未声明 @themes；在 design 块中添加 \
                     @themes {{ ... }} 声明",
                    m.store, m.signal
                ),
                m.file.clone(),
                m.span,
            ));
        }
    }

    // ── F1: literal initializer must be a member of the declared themes ──
    // design doc 域 3 BNF annotation:「@theme 值必须是 @themes 枚举之一（语义验证）」.
    // Only statically-known literals are checked — runtime-assigned values
    // (non-literal initializers) cannot be validated statically and are
    // deliberately left unchecked (boundary documented in spec §4.5).
    // Skipped when no @themes exists anywhere — (a) already covers that.
    if has_themes_declaration {
        let mut all_themes: Vec<&str> = ir
            .modules
            .values()
            .flat_map(|m| m.designs.iter())
            .flat_map(|d| d.themes.iter().map(String::as_str))
            .collect();
        all_themes.sort_unstable();
        all_themes.dedup();
        for m in &marked {
            if matches!(m.kind, TypeKind::String) && !m.nullable {
                if let Some(initial) = &m.initial {
                    if !all_themes.iter().any(|t| t == initial) {
                        diagnostics.push(diag(
                            format!(
                                "@theme 信号 '{}.{}' 初始值 '{}' 必须是 @themes 枚举之一：{}",
                                m.store,
                                m.signal,
                                initial,
                                all_themes.join(", ")
                            ),
                            m.file.clone(),
                            m.span,
                        ));
                    }
                }
            }
        }
    }

    // ── D-F (MEDIUM-2): @default-theme ↔ @theme literal initial must agree ──
    // The design's `@default-theme X;` directive and the @theme signal's
    // literal initial are TWO declarations of the program's startup theme.
    // Without this check React starts from @default-theme while Flutter only
    // honors the signal initial for "dark" — silent cross-backend startup
    // divergence (domain MEDIUM-2). When both are statically known they must
    // match; the error carries BOTH sides' values and source positions
    // (D-F decision). Runtime-assigned initializers stay unchecked — same
    // boundary as F1.
    if marked.len() == 1 && has_themes_declaration {
        if let Some(initial) = &marked[0].initial {
            for design in ir.modules.values().flat_map(|m| m.designs.iter()) {
                if let Some(dt) = &design.default_theme {
                    if dt != initial {
                        diagnostics.push(diag(
                            format!(
                                "@theme 信号 '{}.{}' 初始值 '{}'（{}）与 design '{}' \
                                 的 @default-theme '{}'（{}）不一致——启动主题双声明必须一致 \
                                 （React 从 @default-theme 起，Flutter 从信号 initial 起），请统一两处声明",
                                marked[0].store,
                                marked[0].signal,
                                initial,
                                pos_text(&marked[0].file, &marked[0].span),
                                design.name,
                                dt,
                                pos_text(&design.source_file, &design.source_span),
                            ),
                            marked[0].file.clone(),
                            marked[0].span,
                        ));
                    }
                }
            }
        }
    }

    // ── (c) at most one @theme signal per program ──
    if marked.len() > 1 {
        for m in marked.iter().skip(1) {
            diagnostics.push(diag(
                format!(
                    "重复 @theme 信号：'{}.{}'（一个程序只能有一个当前主题；删除多余的 @theme 标记）",
                    m.store, m.signal
                ),
                m.file.clone(),
                m.span,
            ));
        }
    }

    // ── Wire DesignIR.theme_signal (Task 4.2 consumer contract) ──
    // Only when the pairing is valid: exactly one marked signal AND at least
    // one design declares themes. Error cases leave theme_signal = None
    // (compilation aborts on errors anyway).
    if let Some(only) = marked.first() {
        if marked.len() == 1 && has_themes_declaration {
            let path = format!("{}.{}", only.store, only.signal);
            for design in ir.modules.values_mut().flat_map(|m| m.designs.iter_mut()) {
                if !design.themes.is_empty() {
                    design.theme_signal = Some(path.clone());
                }
            }
        }
    }
}
