//! Shared multi-file analysis pipeline.
//!
//! Unifies parse → validate → semantic → lint into a single function,
//! eliminating duplicate pipeline code across lib.rs and ffi.rs.
//! Also avoids a redundant parse pass by calling `validate_multi_ref()`
//! instead of `validate_multi()`.

use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;
use rustc_hash::FxHasher;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use rayon::prelude::*;

use crate::error::{Diagnostic, DiagnosticKind, Severity};
use crate::ir::IRProgram;
use crate::parser::ast::{ImportStmt, Program};

/// Result of multi-file analysis.
pub struct AnalyseOutput {
    pub success: bool,
    pub diagnostics: Vec<Diagnostic>,
    pub ir: Option<IRProgram>,
    pub warnings: Vec<Diagnostic>,
    /// Pre-parsed programs (available to callers that need them for codegen).
    pub programs: FxHashMap<String, Arc<Program>>,
}

/// Convert a lint diagnostic to a standard Diagnostic.
/// `pub(crate)` — reused by `run_pipeline` (single-file lint phase).
pub(crate) fn lint_to_diag(d: &crate::semantic::lints::LintDiagnostic) -> Diagnostic {
    Diagnostic {
        severity: match d.severity {
            crate::semantic::lints::LintSeverity::Warning => crate::error::Severity::Warning,
            crate::semantic::lints::LintSeverity::Info => crate::error::Severity::Info,
            _ => crate::error::Severity::Warning,
        },
        kind: DiagnosticKind::ELint(crate::error::LintRuleId::from(d.rule.as_str())),
        message: d.message.clone(),
        file: Some(d.file.clone()),
        line: d.line,
        col: d.col,
        end_line: None,
        end_col: None,
        suggestion: None,
    }
}

/// Check whether a lint diagnostic has Error severity.
/// `pub(crate)` — reused by `run_pipeline` (single-file lint phase).
pub(crate) fn is_lint_error(d: &crate::semantic::lints::LintDiagnostic) -> bool {
    d.severity == crate::semantic::lints::LintSeverity::Error
}

/// Run the IR-level semantic validation pipeline on a lowered program.
///
/// Single-point declaration of the 7 IR passes that used to be orchestrated
/// inline at the end of `ir::lower` (Task 5.5 — pass orchestration moved out
/// of lower()). `ir::lower` invokes this pipeline before returning, so every
/// entry point — single-file `run_pipeline`, multi-file
/// `analyse_multi` / `analyse_multi_cached`, and direct `lower()` callers —
/// sees identical diagnostics in identical order.
///
/// The pass sequence (order is load-bearing):
///
/// 1. `validate_required_props` — required component props at call sites.
/// 2. `validate_refs` — ref declarations ↔ `@ref` bindings (undeclared ref
///    warnings, duplicate bindings, @ref-in-@for warnings).
/// 3. `validate_ref_ops` — REWRITES legal ref method calls / field reads to
///    `ExprIR::RefOpExpr` and errors on DOM-specific APIs with migration
///    hints. Must run before `validate_array_methods` so declared-ref
///    subjects keep their unknown-ref-operation diagnostics.
/// 4. `validate_array_methods` — REWRITES legal `subject.filter/map/count`
///    method calls to `ExprIR::ArrayMethod` (cross-target mapping) and
///    errors on malformed uses. Runs after `validate_ref_ops` (see above).
/// 5. `validate_semantic_bindings` — `focus={sig}` / `scroll-x={sig}` /
///    `scroll-y={sig}` must appear only on elements with a `@ref` binding;
///    the v1 双向形状 `value={sig}` (only reachable with `@ref`, v4 5.4
///    parser discriminator) hard-errors with the `bind:value` migration
///    hint — `value={sig}` without `@ref` is a plain one-way prop.
/// 6. `validate_effect_deps` — `@effect(deps)` declarations must cover the
///    store-field reads in the effect body (incl. closure-internal reads),
///    else the effect silently never re-fires — E_EFFECT_DEPS_MISSING.
/// 7. `validate_a11y` — `a11y: { ... }` blocks (design doc 域 10): `role`
///    must be in the Flutter-mappable subset (warning otherwise), `live`
///    accepts only "polite"/"assertive" (error otherwise), raw `aria-*`
///    props are a deleted form, `alt`/`hidden` misuse warns.
/// 8. `validate_drag` — `@drag` / `@dragEnd` draggable-intent directives
///    (ADR-073): the element must carry `@ref`, handlers must resolve to
///    component functions, and a handler takes at most one parameter (the
///    DragIntent event object `{ x, y, dx, dy, pointerId, startX, startY }`).
/// 9. `validate_semantic_keys` — KeyEvent handler (onKeyDown/onKeyUp) 中
///    `e.key == "X"` 的字符串字面量必须是语义键名（Enter/Escape/... 或单
///    字母/单数字），否则 E0006 硬错误 + 最近合法建议（D2，Task 5）。
///
/// Passes 3 and 4 mutate the IR (rewrite expression nodes); the rest are
/// read-only. All diagnostics land in `diagnostics` in pass order.
pub(crate) fn run_ir_passes(ir_program: &mut IRProgram, diagnostics: &mut Vec<Diagnostic>) {
    // Validate required props at component call sites
    let prop_errors = crate::semantic::validate_props::validate_required_props(ir_program);
    for err in prop_errors {
        diagnostics.push(err);
    }

    // Validate ref declarations and bindings
    let ref_errors = crate::semantic::validate_refs::validate_refs(ir_program);
    for err in ref_errors {
        diagnostics.push(err);
    }

    // Validate ref operations: rewrite legal ref method calls / field reads
    // to ExprIR::RefOpExpr, error on DOM-specific APIs with migration hints.
    for module in ir_program.modules.values_mut() {
        for component in module.components.values_mut() {
            crate::semantic::validate_ref_ops::validate_ref_ops(component, diagnostics);
        }
    }

    // Validate array language methods: rewrite `subject.filter/map/count(...)`
    // method calls to ExprIR::ArrayMethod (cross-target mapping), error on
    // malformed uses (count without predicate, provably non-array subjects).
    // Runs after validate_ref_ops so declared-ref subjects keep their
    // unknown-ref-operation diagnostics.
    for module in ir_program.modules.values_mut() {
        crate::semantic::validate_array_methods::validate_array_methods(module, diagnostics);
    }

    // Validate semantic ref bindings: focus={sig}, scroll-x={sig},
    // scroll-y={sig} must appear only on elements with a @ref binding whose
    // ref is declared on the component. The v1 value={sig} 双向形状
    // (parser keeps it a RefBinding only when @ref is present, v4 5.4)
    // hard-errors here with the bind:value migration hint.
    for module in ir_program.modules.values_mut() {
        for component in module.components.values_mut() {
            crate::semantic::validate_semantic_bindings::validate_semantic_bindings(
                component,
                diagnostics,
            );
        }
    }

    // Validate @effect(deps): the declared deps must cover the store-field
    // reads in the effect body (including closure-internal reads), else the
    // effect silently never re-fires — E_EFFECT_DEPS_MISSING compile error.
    for module in ir_program.modules.values() {
        for component in module.components.values() {
            crate::semantic::validate_effect_deps::validate_effect_deps(
                component,
                ir_program,
                diagnostics,
            );
        }
    }

    // Validate a11y blocks (design doc 域 10): `role` must be in the
    // Flutter-mappable subset (warning otherwise — Flutter degrades with a
    // generated-code comment, the web targets pass the role through);
    // `live` accepts only "polite"/"assertive" (error otherwise).
    for module in ir_program.modules.values() {
        for component in module.components.values() {
            crate::semantic::validate_a11y::validate_a11y(component, diagnostics);
        }
    }

    // Validate @drag / @dragEnd draggable-intent directives (ADR-073):
    // element must carry @ref; handlers must resolve to component
    // functions; a handler takes at most one parameter (DragIntent data).
    for module in ir_program.modules.values() {
        for component in module.components.values() {
            crate::semantic::validate_drag::validate_drag(component, diagnostics);
        }
    }

    // Validate KeyEvent `e.key` semantic-key enum values (D2, Task 5):
    // string literals compared against `e.key` in onKeyDown/onKeyUp
    // handlers must be semantic key names (Enter/Escape/... or a single
    // letter/digit), else error with the nearest legal suggestion.
    for module in ir_program.modules.values() {
        for component in module.components.values() {
            crate::semantic::validate_semantic_keys::validate_semantic_keys(component, diagnostics);
        }
    }
}

/// Fast non-cryptographic hash for content comparison.
///
/// `FxHasher` (rustc-hash) is roughly 2-3x faster than `DefaultHasher`
/// (SipHash-1-3) for short source strings. This is a storage-type hash and
/// is safe to switch: `content_hash` is never used as a persistent key, it
/// is only compared against a freshly computed hash of the same source
/// in-process (see `analyse_multi_cached`). A persisted cache state from an
/// older hasher (e.g. WASM `seed_cache_from_state`) simply fails the
/// comparison and triggers a benign one-time re-analysis of the affected
/// files.
fn fast_hash(content: &str) -> u64 {
    let mut hasher = FxHasher::default();
    content.hash(&mut hasher);
    hasher.finish()
}

/// Cached compilation data for a single file.
///
/// This cache is memory-only. `program` and `ir` are skipped during
/// serialization because they contain complex types (AST, IR) that don't
/// round-trip well via serde. After deserialization, `program_valid` is
/// `false` and all files will be re-analyzed on the next pass.
///
/// Use `save_cache_state()` / `seed_cache_from_state()` only for page-navigation
/// persistence (e.g. `beforeunload`), never expecting cache efficiency across
/// serialization boundaries.
#[derive(Serialize, Deserialize)]
pub struct CachedFile {
    pub content_hash: u64,
    /// In-memory parsed program (not serialized — skipped during serialization).
    /// On deserialization this will be `Arc::new(Program::default())`, so
    /// consumers must check `program_valid` before using it.
    #[serde(skip)]
    pub program: Arc<Program>,
    #[serde(skip_serializing)]
    #[serde(default)]
    pub(crate) program_valid: bool,
    pub diagnostics: Vec<Diagnostic>,
    /// In-memory IR program (not serialized).
    #[serde(skip)]
    pub ir: IRProgram,
    /// Files that this file imports (for invalidation propagation).
    pub imports: Vec<String>,
    /// Files that import this file (reverse edges).
    pub dependents: Vec<String>,
}

/// Maximum number of files retained in the incremental cache.
/// Evicts the oldest entry (by insertion order) when exceeded.
/// Value chosen to balance memory (each CachedFile holds AST + IR)
/// with hit rate for typical project sizes.
pub const MAX_CACHE_ENTRIES: usize = 50;

/// Incremental compilation cache that avoids re-parsing/re-analyzing unchanged files.
#[derive(Serialize, Deserialize)]
pub struct IncrementalCache {
    pub entries: FxHashMap<String, CachedFile>,
    /// Insertion-order tracking for LRU eviction.
    #[serde(default)]
    insertion_order: Vec<String>,
}

impl IncrementalCache {
    pub fn new() -> Self {
        Self {
            entries: FxHashMap::default(),
            insertion_order: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.insertion_order.clear();
    }
}

impl Default for IncrementalCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract non-stdlib import paths from a parsed program.
fn collect_import_paths(program: &Program) -> Vec<String> {
    let mut paths = Vec::new();
    for module in program.modules.values() {
        for import in &module.imports {
            let import_path = match import {
                ImportStmt::Named(n) => &n.path,
                ImportStmt::Namespace(ns) => &ns.path,
                ImportStmt::TypeOnly(t) => &t.path,
                ImportStmt::Lib(_) => continue,
            };
            // Skip stdlib imports (resolved by IR/stdlib layer, not via file mapping)
            if import_path.starts_with("std/") {
                continue;
            }
            paths.push(import_path.clone());
        }
    }
    paths
}

/// Report a duplicate name warning during IR merge.
fn report_duplicate(warnings: &mut Vec<Diagnostic>, kind: &str, name: &str, path: &str) {
    warnings.push(Diagnostic {
        severity: Severity::Warning,
        kind: DiagnosticKind::E0000,
        message: format!("duplicate {} name '{}'", kind, name),
        file: Some(path.to_string()),
        line: 0,
        col: 0,
        end_line: None,
        end_col: None,
        suggestion: None,
    });
}

/// Report duplicates in a collection before extending.
fn extend_and_warn<K, V>(
    target: &mut FxHashMap<K, V>,
    source: &mut FxHashMap<K, V>,
    warnings: &mut Vec<Diagnostic>,
    kind: &str,
    path: &str,
) where
    K: std::cmp::Eq + std::hash::Hash + std::fmt::Display,
{
    for (name, _) in source.iter() {
        if target.contains_key(name) {
            report_duplicate(warnings, kind, &name.to_string(), path);
        }
    }
    target.extend(std::mem::take(source));
}

/// Merge a single file's IR into the merged IR program.
/// Takes `&mut IRProgram` so it can move data out of the per-file IR,
/// avoiding clones for newly generated IR.
fn merge_file_ir_into(
    merged: &mut IRProgram,
    path: &str,
    ir: &mut IRProgram,
    warnings: &mut Vec<Diagnostic>,
) {
    for (mod_name, mut mod_ir) in std::mem::take(&mut ir.modules) {
        for comp in mod_ir.components.values_mut() {
            if comp.source_file.is_none() {
                comp.source_file = Some(path.to_string());
            }
        }
        for store in mod_ir.stores.values_mut() {
            if store.source_file.is_none() {
                store.source_file = Some(path.to_string());
            }
        }
        for design in mod_ir.designs.iter_mut() {
            if design.source_file.is_none() {
                design.source_file = Some(path.to_string());
            }
        }
        for motion in mod_ir.motions.iter_mut() {
            if motion.source_file.is_none() {
                motion.source_file = Some(path.to_string());
            }
        }
        let existing = merged.modules.entry(mod_name).or_default();
        extend_and_warn(
            &mut existing.components,
            &mut mod_ir.components,
            warnings,
            "component",
            path,
        );
        extend_and_warn(
            &mut existing.stores,
            &mut mod_ir.stores,
            warnings,
            "store",
            path,
        );
        extend_and_warn(
            &mut existing.functions,
            &mut mod_ir.functions,
            warnings,
            "function",
            path,
        );
        existing.structs.extend(std::mem::take(&mut mod_ir.structs));
        // Tagged unions (deferral #30): per-file enums were silently dropped
        // here, so no backend ever saw union declarations from the merged IR
        // (Flutter match variant arms / variant literals reference them).
        existing.enums.extend(std::mem::take(&mut mod_ir.enums));
        existing
            .type_aliases
            .extend(std::mem::take(&mut mod_ir.type_aliases));
        existing.imports.extend(std::mem::take(&mut mod_ir.imports));
        existing.exports.extend(std::mem::take(&mut mod_ir.exports));
        existing.motions.extend(std::mem::take(&mut mod_ir.motions));
        existing.designs.extend(std::mem::take(&mut mod_ir.designs));
    }
    for (key, entry) in std::mem::take(&mut ir.source_map) {
        merged.source_map.entry(key).or_insert(entry);
    }
}

/// Parse + cross-file validation + semantic analysis + lint for multiple source files.
///
/// Pipeline:
///   1. Parse all sources into ASTs
///   2. Cross-file validation (imports, visibility, cycles)
///   3. Per-file semantic analysis + IR lowering → merged IR
///   4. Lint rules (AST-level and IR-level)
///
/// Returns early on parse failure (ir = None, success = false).
#[must_use]
pub fn analyse_multi(
    sources: &FxHashMap<String, String>,
    path_mapping: &FxHashMap<String, String>,
) -> AnalyseOutput {
    let mut diags: Vec<Diagnostic> = Vec::new();
    let mut programs: FxHashMap<String, Arc<Program>> = FxHashMap::default();

    // Phase 1: Parallel parse all sources
    let parse_results: Vec<(String, Program, Vec<Diagnostic>)> = sources
        .par_iter()
        .map(|(path, source)| {
            let tokens = crate::lexer::tokenize(source);
            let (program, parse_errors) = crate::parser::parse(tokens, source);
            let file_diags: Vec<Diagnostic> = parse_errors
                .into_iter()
                .map(|e| {
                    let d: Diagnostic = e.into();
                    d.with_file(path)
                })
                .collect();
            (path.clone(), program, file_diags)
        })
        .collect();

    for (path, program, file_diags) in parse_results {
        diags.extend(file_diags);
        programs.insert(path, Arc::new(program));
    }

    // Phase 2: Cross-file validation using pre-parsed programs
    let validation = crate::validate::validate_multi_ref(&programs, path_mapping);
    diags.extend(validation.errors);

    // Build cross-file const → value map for design token resolution
    let mut global_const_values: FxHashMap<String, String> = FxHashMap::default();
    for program in programs.values() {
        for module in program.modules.values() {
            for const_def in &module.consts {
                if let crate::parser::ast::Expr::Literal(
                    crate::parser::ast::Literal::String(s),
                    _,
                ) = &const_def.value
                {
                    global_const_values.insert(const_def.name.clone(), s.clone());
                }
            }
        }
    }

    // Phase 3a: Parallel per-file semantic analysis + IR lowering
    let ir_results: Vec<(String, IRProgram, Vec<Diagnostic>)> = programs
        .par_iter()
        .map(|(path, program)| {
            let program_ref: &Program = program.as_ref();
            let mut semantics = crate::semantic::analyze(program_ref);
            let mut local_diags: Vec<Diagnostic> = Vec::new();
            for d in std::mem::take(&mut semantics.diagnostics) {
                let mut d = d;
                if d.file.is_none() {
                    d.file = Some(path.clone());
                }
                local_diags.push(d);
            }
            let ir_program =
                crate::ir::lower(program_ref, &mut semantics, Some(&global_const_values));
            // Re-collect diagnostics emitted during lowering (validate_props,
            // validate_refs, validate_ref_ops, design validation).
            for d in std::mem::take(&mut semantics.diagnostics) {
                let mut d = d;
                if d.file.is_none() {
                    d.file = Some(path.clone());
                }
                local_diags.push(d);
            }
            (path.clone(), ir_program, local_diags)
        })
        .collect();

    // Phase 3b: Sequential IR merge
    let mut merged_ir = IRProgram {
        version: "1.0".to_string(),
        modules: FxHashMap::default(),
        source_map: FxHashMap::default(),
        stdlib_imports: Default::default(),
        source_span: None,
        motion_index: FxHashMap::default(),
    };

    let mut merge_warnings: Vec<Diagnostic> = Vec::new();
    for (path, mut ir_program, local_diags) in ir_results {
        diags.extend(local_diags);
        merge_file_ir_into(&mut merged_ir, &path, &mut ir_program, &mut merge_warnings);
    }

    // Phase 3c: IR optimization passes
    crate::ir::optimize::optimize(&mut merged_ir);

    // Phase 3d: Program-level theme validation (@themes ↔ @theme pairing,
    // design D1). Runs on the merged IR so cross-file pairing is visible;
    // also populates DesignIR.theme_signal for every design.
    crate::semantic::validate_theme::validate_theme(&mut merged_ir, &mut diags);

    // Phase 3e: Event-name validation (Task 4.2, D-B) — unknown event names
    // warn with the nearest legal suggestion. Runs on the merged IR so
    // cross-file custom `event` declarations and component tags are visible
    // (a per-file pass cannot tell an imported component's custom-event
    // binding apart from a typo'd DOM event).
    crate::semantic::validate_event_names::validate_event_names(&merged_ir, &mut diags);

    // Phase 3f: Form/data tri-state validation (Domain A, Task 3) — consumes
    // FormIntentIR/LoadIntentIR collected during lowering, checks V-F2~F8.
    // V-F1/V-F6 are reported at the lowering scan (ir/lower/component.rs).
    crate::semantic::validate_form_load::validate_form_load(&merged_ir, &mut diags);

    // Phase 3f2: Feedback-modal validation (batch 1, domain 10) — consumes
    // ModalIntentIR/SkeletonBindingIR/ErrorBoundaryBindingIR, checks V1-V14
    // (V4/V11 duplicate bindings are reported at lowering).
    crate::semantic::validate_feedback_modal::validate_feedback_modal(&merged_ir, &mut diags);

    // Phase 4: Run lint rules against ASTs and merged IR
    let lint_diags = crate::semantic::lints::run_all(&programs, &merged_ir);
    let mut warnings: Vec<Diagnostic> = merge_warnings;
    warnings.extend(
        lint_diags
            .iter()
            .filter(|d| !is_lint_error(d))
            .map(lint_to_diag),
    );

    // Lint errors are also included in diagnostics so that all callers
    // (compile_multi_to_target, omni_analyse, analyse_direct) see them.
    for d in &lint_diags {
        if is_lint_error(d) {
            let mut diag = lint_to_diag(d);
            diag.severity = crate::error::Severity::Error;
            diags.push(diag);
        }
    }

    let success = !diags
        .iter()
        .any(|d| d.severity == crate::error::Severity::Error);
    AnalyseOutput {
        success,
        diagnostics: diags,
        ir: Some(merged_ir),
        warnings,
        programs,
    }
}

/// Multi-file analysis with incremental compilation cache.
///
/// Same pipeline as `analyse_multi`, but skips parsing and semantic analysis
/// for files whose content hash matches the cache.  Changed files trigger
/// transitive invalidation of their dependents (files that import them).
#[must_use]
pub fn analyse_multi_cached(
    sources: &FxHashMap<String, String>,
    path_mapping: &FxHashMap<String, String>,
    cache: &mut IncrementalCache,
) -> AnalyseOutput {
    let mut diags: Vec<Diagnostic> = Vec::new();
    let mut all_programs: FxHashMap<String, Arc<Program>> = FxHashMap::default();

    // Phase 0: Identify changed files via content hash
    let mut changed: FxHashSet<String> = FxHashSet::default();
    for (path, source) in sources {
        let hash = fast_hash(source);
        if let Some(cached) = cache.entries.get(path) {
            // Only treat as cache hit if the program is valid (in-memory, not
            // deserialized from serialized cache_state) AND hash matches.
            if cached.content_hash == hash && cached.program_valid {
                // Cache hit — reuse Arc'd program
                all_programs.insert(path.clone(), cached.program.clone());
                continue;
            }
        }
        changed.insert(path.clone());
    }

    // Phase 0.5: Transitively invalidate dependents of changed files
    let mut to_reprocess: FxHashSet<String> = changed.clone();
    let mut worklist: Vec<String> = changed.iter().cloned().collect();
    while let Some(file) = worklist.pop() {
        if let Some(cached) = cache.entries.get(&file) {
            for dependent in &cached.dependents {
                if to_reprocess.insert(dependent.clone()) {
                    worklist.push(dependent.clone());
                }
            }
        }
    }
    // Prune any paths that are no longer in the sources (e.g. deleted files)
    to_reprocess.retain(|p| sources.contains_key(p));

    // Phase 1: Parse only changed + transitively affected files
    // Parse-phase diagnostics are retained per-path so they can be persisted
    // into the cache below (otherwise cache hits would drop them entirely).
    let mut parse_diags: FxHashMap<String, Vec<Diagnostic>> = FxHashMap::default();
    if !to_reprocess.is_empty() {
        let parse_results: Vec<(String, Program, Vec<Diagnostic>)> = to_reprocess
            .par_iter()
            .map(|path| {
                let source = &sources[path];
                let tokens = crate::lexer::tokenize(source);
                let (program, parse_errors) = crate::parser::parse(tokens, source);
                let file_diags: Vec<Diagnostic> = parse_errors
                    .into_iter()
                    .map(|e| {
                        let d: Diagnostic = e.into();
                        d.with_file(path)
                    })
                    .collect();
                (path.clone(), program, file_diags)
            })
            .collect();

        for (path, program, file_diags) in parse_results {
            diags.extend(file_diags.iter().cloned());
            parse_diags.insert(path.clone(), file_diags);
            all_programs.insert(path, Arc::new(program));
        }
    }

    // Phase 2: Cross-file validation using the full program set
    let validation = crate::validate::validate_multi_ref(&all_programs, path_mapping);
    diags.extend(validation.errors);

    // Build cross-file const → value map (cached path)
    let mut global_const_values: FxHashMap<String, String> = FxHashMap::default();
    for program in all_programs.values() {
        for module in program.modules.values() {
            for const_def in &module.consts {
                if let crate::parser::ast::Expr::Literal(
                    crate::parser::ast::Literal::String(s),
                    _,
                ) = &const_def.value
                {
                    global_const_values.insert(const_def.name.clone(), s.clone());
                }
            }
        }
    }

    // Phase 3a: Parallel per-file semantic analysis + IR lowering for changed files only
    let mut per_file_ir: FxHashMap<String, (IRProgram, Vec<Diagnostic>)> = to_reprocess
        .par_iter()
        .map(|path| {
            let program: &Program = all_programs[path].as_ref();
            let mut semantics = crate::semantic::analyze(program);
            let mut local_diags: Vec<Diagnostic> = Vec::new();
            for d in std::mem::take(&mut semantics.diagnostics) {
                let mut d = d;
                if d.file.is_none() {
                    d.file = Some(path.clone());
                }
                local_diags.push(d);
            }
            let ir_program = crate::ir::lower(program, &mut semantics, Some(&global_const_values));
            // Re-collect diagnostics emitted during lowering (validate_props,
            // validate_refs, validate_ref_ops, design validation).
            for d in std::mem::take(&mut semantics.diagnostics) {
                let mut d = d;
                if d.file.is_none() {
                    d.file = Some(path.clone());
                }
                local_diags.push(d);
            }
            (path.clone(), (ir_program, local_diags))
        })
        .collect();

    // Collect import paths from newly parsed programs for cache update
    let mut new_imports: FxHashMap<String, Vec<String>> = FxHashMap::default();
    for path in &to_reprocess {
        if let Some(program) = all_programs.get(path) {
            new_imports.insert(path.clone(), collect_import_paths(program));
        }
    }

    // ── Update cache with freshly processed files ──
    for path in &to_reprocess {
        if let Some(program) = all_programs.get(path) {
            let content_hash = fast_hash(&sources[path]);
            let imports = new_imports.remove(path).unwrap_or_default();
            if let Some((ir, local_diags)) = per_file_ir.get(path) {
                // Persist parse-phase diagnostics alongside lowering diagnostics
                // so cache hits replay the complete error set — parse errors
                // must not silently vanish on the second run.
                let mut cached_diags = parse_diags.remove(path).unwrap_or_default();
                cached_diags.extend(local_diags.iter().cloned());
                cache.entries.insert(
                    path.clone(),
                    CachedFile {
                        content_hash,
                        program: program.clone(),
                        program_valid: true,
                        diagnostics: cached_diags,
                        ir: ir.clone(),
                        imports,
                        dependents: Vec::new(),
                    },
                );
                // Track insertion order for LRU eviction
                cache.insertion_order.retain(|p| p != path);
                cache.insertion_order.push(path.clone());
                // Evict oldest entries if cache exceeds capacity
                while cache.insertion_order.len() > MAX_CACHE_ENTRIES {
                    if let Some(oldest) = cache.insertion_order.first().cloned() {
                        cache.entries.remove(&oldest);
                        cache.insertion_order.remove(0);
                    } else {
                        break;
                    }
                }
            }
        }
    }

    // Remove stale entries for files no longer in sources
    cache.entries.retain(|k, _| sources.contains_key(k));
    cache.insertion_order.retain(|k| sources.contains_key(k));

    // Rebuild dependents (reverse import edges) for all cache entries
    for cached in cache.entries.values_mut() {
        cached.dependents.clear();
    }
    let cache_paths: Vec<String> = cache.entries.keys().cloned().collect();
    let mut dep_edges: Vec<(String, String)> = Vec::new();
    for path in &cache_paths {
        let imports = cache
            .entries
            .get(path)
            .map(|c| c.imports.clone())
            .unwrap_or_default();
        for imp in &imports {
            if let Some(target) = path_mapping.get(imp) {
                dep_edges.push((path.clone(), target.clone()));
            }
        }
    }
    for (dependent, target) in dep_edges {
        if let Some(entry) = cache.entries.get_mut(&target) {
            if !entry.dependents.contains(&dependent) {
                entry.dependents.push(dependent);
            }
        }
    }

    // Phase 3b: Sequential IR merge
    let mut merged_ir = IRProgram {
        version: "1.0".to_string(),
        modules: FxHashMap::default(),
        source_map: FxHashMap::default(),
        stdlib_imports: Default::default(),
        source_span: None,
        motion_index: FxHashMap::default(),
    };

    let mut merge_warnings: Vec<Diagnostic> = Vec::new();

    // Merge cached IR for files NOT in to_reprocess (clone to preserve cache)
    for (path, cached) in &cache.entries {
        if !to_reprocess.contains(path) {
            let mut ir_clone = cached.ir.clone();
            merge_file_ir_into(&mut merged_ir, path, &mut ir_clone, &mut merge_warnings);
        }
    }

    // Merge cached diagnostics for files NOT in to_reprocess (shallow clone;
    // hit cost traded for diagnostic completeness — unchanged files keep
    // surfacing their errors).
    for (path, cached) in &cache.entries {
        if !to_reprocess.contains(path) {
            diags.extend(cached.diagnostics.iter().cloned());
        }
    }

    // Merge freshly computed IR (consume per_file_ir — no clone needed)
    for (path, (ir_program, local_diags)) in per_file_ir.drain() {
        diags.extend(local_diags);
        let mut ir = ir_program;
        merge_file_ir_into(&mut merged_ir, &path, &mut ir, &mut merge_warnings);
    }

    // Phase 3c: IR optimization passes
    crate::ir::optimize::optimize(&mut merged_ir);

    // Phase 3d: Program-level theme validation (@themes ↔ @theme pairing,
    // design D1). Runs on the merged IR so cross-file pairing is visible;
    // also populates DesignIR.theme_signal for every design.
    crate::semantic::validate_theme::validate_theme(&mut merged_ir, &mut diags);

    // Phase 3e: Event-name validation (Task 4.2, D-B) — unknown event names
    // warn with the nearest legal suggestion. Runs on the merged IR so
    // cross-file custom `event` declarations and component tags are visible
    // (a per-file pass cannot tell an imported component's custom-event
    // binding apart from a typo'd DOM event).
    crate::semantic::validate_event_names::validate_event_names(&merged_ir, &mut diags);

    // Phase 3f: Form/data tri-state validation (Domain A, Task 3) — consumes
    // FormIntentIR/LoadIntentIR collected during lowering, checks V-F2~F8.
    // V-F1/V-F6 are reported at the lowering scan (ir/lower/component.rs).
    crate::semantic::validate_form_load::validate_form_load(&merged_ir, &mut diags);

    // Phase 3f2: Feedback-modal validation (batch 1, domain 10) — consumes
    // ModalIntentIR/SkeletonBindingIR/ErrorBoundaryBindingIR, checks V1-V14
    // (V4/V11 duplicate bindings are reported at lowering).
    crate::semantic::validate_feedback_modal::validate_feedback_modal(&merged_ir, &mut diags);

    // Phase 4: Run lint rules against ASTs and merged IR
    let lint_diags = crate::semantic::lints::run_all(&all_programs, &merged_ir);
    let mut warnings: Vec<Diagnostic> = merge_warnings;
    warnings.extend(
        lint_diags
            .iter()
            .filter(|d| !is_lint_error(d))
            .map(lint_to_diag),
    );

    let success = !diags
        .iter()
        .any(|d| d.severity == crate::error::Severity::Error);
    AnalyseOutput {
        success,
        diagnostics: diags,
        ir: Some(merged_ir),
        warnings,
        programs: all_programs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression: cache hits must not drop diagnostics of unchanged files.
    ///
    /// `CachedFile.diagnostics` is written on cache update but was never
    /// merged into the output on hits, so `success` could be `true` while
    /// errors existed — the poiesis editor runs this path on every keystroke.
    #[test]
    fn cached_run_keeps_diagnostics_of_unchanged_files() {
        // File "a.omni" contains an illegal ref op — a lower-phase error that
        // must surface on every run, cached or not (same source as the
        // lib.rs pipeline_tests regression).
        let err_src = "component App {\n    ref box: Element;\n    view {\n        <div @ref={box} />\n    }\n    fn go() {\n        box.getBoundingClientRect();\n    }\n}";
        let ok_src = "component Clean {\n    view {\n        <div>ok</div>\n    }\n}";
        let mut sources: FxHashMap<String, String> = FxHashMap::default();
        sources.insert("a.omni".to_string(), err_src.to_string());
        sources.insert("b.omni".to_string(), ok_src.to_string());
        let path_mapping: FxHashMap<String, String> = FxHashMap::default();
        let mut cache = IncrementalCache::new();

        let out1 = analyse_multi_cached(&sources, &path_mapping, &mut cache);
        assert!(
            out1.diagnostics
                .iter()
                .any(|d| d.message.contains("getBoundingClientRect")),
            "run 1 must surface the ref-op error, got: {:?}",
            out1.diagnostics
        );
        assert!(!out1.success, "run 1 must not report success with errors");

        // Second run: both files are cache hits. The error must survive.
        let out2 = analyse_multi_cached(&sources, &path_mapping, &mut cache);
        assert!(
            out2.diagnostics
                .iter()
                .any(|d| d.message.contains("getBoundingClientRect")),
            "cache hit dropped diagnostics of unchanged file, run 2 got: {:?}",
            out2.diagnostics
        );
        assert!(!out2.success, "success must not lie while errors exist");
    }

    /// Regression: parse-phase diagnostics must survive cache hits.
    ///
    /// `CachedFile.diagnostics` only ever stored lowering-phase diagnostics;
    /// parse-phase diagnostics (collected in Phase 1) were emitted to the
    /// output but never cached, so on the next run — where the file is a
    /// cache hit and skipped — `success` flipped to `true` and the parse
    /// error vanished. The poiesis editor runs this path on every keystroke,
    /// so bad code would show as green.
    #[test]
    fn cached_run_keeps_parse_diagnostics() {
        // File "a.omni" triggers a parse error (E0001); "b.omni" is clean.
        let bad = "view A { @click { } }";
        let clean = "view B { }";
        let mut sources: FxHashMap<String, String> = FxHashMap::default();
        sources.insert("a.omni".to_string(), bad.to_string());
        sources.insert("b.omni".to_string(), clean.to_string());
        let path_mapping: FxHashMap<String, String> = FxHashMap::default();
        let mut cache = IncrementalCache::new();

        let out1 = analyse_multi_cached(&sources, &path_mapping, &mut cache);
        assert!(
            !out1.success,
            "run 1 must fail on parse error, got success=true, diags: {:?}",
            out1.diagnostics
        );
        assert!(
            out1.diagnostics
                .iter()
                .any(|d| d.kind == DiagnosticKind::E0001),
            "run 1 must carry the parse diag, got: {:?}",
            out1.diagnostics
        );

        // Second run: both files are cache hits. The parse error must survive.
        let out2 = analyse_multi_cached(&sources, &path_mapping, &mut cache);
        assert!(
            !out2.success,
            "cache hit must not flip success on parse error, got success=true, diags: {:?}",
            out2.diagnostics
        );
        assert!(
            out2.diagnostics
                .iter()
                .any(|d| d.kind == DiagnosticKind::E0001),
            "parse diag dropped on cache hit, run 2 got: {:?}",
            out2.diagnostics
        );
    }
}
