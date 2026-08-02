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
//!   (verbatim from the design doc 域 5 diagnostic table)
//! - (c) more than one `@theme` signal in the program → error
//! - (e) duplicate theme names inside `@themes { ... }` → error
//! - (f) empty `@themes { }` → parse error (in `parser/design.rs`) — the IR
//!   cannot distinguish an explicit empty declaration (`themes: []`) from
//!   no declaration, so the check lives where the source shape is visible.
//!
//! Also populates `DesignIR.theme_signal = "StoreName.signalName"` for every
//! design with a non-empty theme set (wiring-ready shape consumed by codegen
//! in Task 4.2).

use crate::error::{Diagnostic, DiagnosticKind, Severity};
use crate::ir::{IRProgram, TypeKind};

/// (b) verbatim from the design doc 域 5 diagnostic table.
const NON_STRING_MSG: &str = "必须是 String";

/// A store signal marked `@theme`, collected across all modules.
struct MarkedThemeSignal {
    store: String,
    signal: String,
    file: Option<String>,
    kind: TypeKind,
}

fn diag(message: String, file: Option<String>) -> Diagnostic {
    Diagnostic {
        severity: Severity::Error,
        kind: DiagnosticKind::E0006,
        message,
        file,
        line: 0,
        col: 0,
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
                marked.push(MarkedThemeSignal {
                    store: store.name.clone(),
                    signal: sig.clone(),
                    file: store.source_file.clone(),
                    kind: store.signals.get(sig).map(|t| t.kind.clone()).unwrap_or(
                        TypeKind::Named {
                            name: "<unknown>".into(),
                        },
                    ),
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
                ));
            }
        }
    }

    // ── (b) @theme signal must be String ──
    for m in &marked {
        if !matches!(m.kind, TypeKind::String) {
            diagnostics.push(diag(
                format!("@theme 信号 '{}.{}' {}", m.store, m.signal, NON_STRING_MSG),
                m.file.clone(),
            ));
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
            ));
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
