//! Shared multi-file analysis pipeline.
//!
//! Unifies parse → validate → semantic → lint into a single function,
//! eliminating duplicate pipeline code across lib.rs and ffi.rs.
//! Also avoids a redundant parse pass by calling `validate_multi_ref()`
//! instead of `validate_multi()`.

use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;
use std::collections::hash_map::DefaultHasher;
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
fn lint_to_diag(d: &crate::semantic::lints::LintDiagnostic) -> Diagnostic {
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
fn is_lint_error(d: &crate::semantic::lints::LintDiagnostic) -> bool {
    d.severity == crate::semantic::lints::LintSeverity::Error
}

/// Fast non-cryptographic hash for content comparison.
fn fast_hash(content: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}

/// Cached compilation data for a single file.
pub struct CachedFile {
    pub content_hash: u64,
    pub program: Arc<Program>,
    pub diagnostics: Vec<Diagnostic>,
    pub ir: IRProgram,
    /// Files that this file imports (for invalidation propagation).
    pub imports: Vec<String>,
    /// Files that import this file (reverse edges).
    pub dependents: Vec<String>,
}

/// Incremental compilation cache that avoids re-parsing/re-analyzing unchanged files.
pub struct IncrementalCache {
    pub entries: FxHashMap<String, CachedFile>,
}

impl IncrementalCache {
    pub fn new() -> Self {
        Self {
            entries: FxHashMap::default(),
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
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
        existing
            .type_aliases
            .extend(std::mem::take(&mut mod_ir.type_aliases));
        existing.imports.extend(std::mem::take(&mut mod_ir.imports));
        existing.exports.extend(std::mem::take(&mut mod_ir.exports));
        existing.motions.extend(std::mem::take(&mut mod_ir.motions));
        existing
            .timelines
            .extend(std::mem::take(&mut mod_ir.timelines));
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
    let parse_results: Vec<(String, Result<Program, Vec<Diagnostic>>)> = sources
        .par_iter()
        .map(|(path, source)| {
            let tokens = crate::lexer::tokenize(source);
            match crate::parser::parse(tokens, source) {
                Ok(program) => (path.clone(), Ok(program)),
                Err(e) => {
                    let d: Diagnostic = e.into();
                    (path.clone(), Err(vec![d.with_file(path)]))
                }
            }
        })
        .collect();

    for (path, result) in parse_results {
        match result {
            Ok(program) => {
                programs.insert(path, Arc::new(program));
            }
            Err(errs) => {
                diags.extend(errs);
            }
        }
    }

    if !diags.is_empty() {
        return AnalyseOutput {
            success: false,
            diagnostics: diags,
            ir: None,
            warnings: vec![],
            programs,
        };
    }

    // Phase 2: Cross-file validation using pre-parsed programs
    let validation = crate::validate::validate_multi_ref(&programs, path_mapping);
    diags.extend(validation.errors);

    // Phase 3a: Parallel per-file semantic analysis + IR lowering
    let ir_results: Vec<(String, IRProgram, Vec<Diagnostic>)> = programs
        .par_iter()
        .map(|(path, program)| {
            let program_ref: &Program = program.as_ref();
            let mut semantics = crate::semantic::analyze(program_ref);
            let mut local_diags: Vec<Diagnostic> = Vec::new();
            for d in &semantics.diagnostics {
                let mut d = d.clone();
                if d.file.is_none() {
                    d.file = Some(path.clone());
                }
                local_diags.push(d);
            }
            let ir_program = crate::ir::lower(program_ref, &mut semantics);
            (path.clone(), ir_program, local_diags)
        })
        .collect();

    // Phase 3b: Sequential IR merge
    let mut merged_ir = IRProgram {
        version: "1.0".to_string(),
        modules: FxHashMap::default(),
        source_map: FxHashMap::default(),
        stdlib_imports: Default::default(),
    };

    let mut merge_warnings: Vec<Diagnostic> = Vec::new();
    for (path, mut ir_program, local_diags) in ir_results {
        diags.extend(local_diags);
        merge_file_ir_into(&mut merged_ir, &path, &mut ir_program, &mut merge_warnings);
    }

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
            if cached.content_hash == hash {
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
    if !to_reprocess.is_empty() {
        let parse_results: Vec<(String, Result<Program, Vec<Diagnostic>>)> = to_reprocess
            .par_iter()
            .map(|path| {
                let source = &sources[path];
                let tokens = crate::lexer::tokenize(source);
                match crate::parser::parse(tokens, source) {
                    Ok(program) => (path.clone(), Ok(program)),
                    Err(e) => {
                        let d: Diagnostic = e.into();
                        (path.clone(), Err(vec![d.with_file(path)]))
                    }
                }
            })
            .collect();

        for (path, result) in parse_results {
            match result {
                Ok(program) => {
                    all_programs.insert(path, Arc::new(program));
                }
                Err(errs) => {
                    diags.extend(errs);
                }
            }
        }
    }

    if !diags.is_empty() {
        // Clean up stale cache entries before returning early
        cache.entries.retain(|k, _| sources.contains_key(k));
        return AnalyseOutput {
            success: false,
            diagnostics: diags,
            ir: None,
            warnings: vec![],
            programs: all_programs,
        };
    }

    // Phase 2: Cross-file validation using the full program set
    let validation = crate::validate::validate_multi_ref(&all_programs, path_mapping);
    diags.extend(validation.errors);

    // Phase 3a: Parallel per-file semantic analysis + IR lowering for changed files only
    let mut per_file_ir: FxHashMap<String, (IRProgram, Vec<Diagnostic>)> = to_reprocess
        .par_iter()
        .map(|path| {
            let program: &Program = all_programs[path].as_ref();
            let mut semantics = crate::semantic::analyze(program);
            let mut local_diags: Vec<Diagnostic> = Vec::new();
            for d in &semantics.diagnostics {
                let mut d = d.clone();
                if d.file.is_none() {
                    d.file = Some(path.clone());
                }
                local_diags.push(d);
            }
            let ir_program = crate::ir::lower(program, &mut semantics);
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
                cache.entries.insert(
                    path.clone(),
                    CachedFile {
                        content_hash,
                        program: program.clone(),
                        diagnostics: local_diags.clone(),
                        ir: ir.clone(),
                        imports,
                        dependents: Vec::new(),
                    },
                );
            }
        } else {
            // Parse failed — remove stale cache entry
            cache.entries.remove(path);
        }
    }

    // Remove stale entries for files no longer in sources
    cache.entries.retain(|k, _| sources.contains_key(k));

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
    };

    let mut merge_warnings: Vec<Diagnostic> = Vec::new();

    // Merge cached IR for files NOT in to_reprocess (clone to preserve cache)
    for (path, cached) in &cache.entries {
        if !to_reprocess.contains(path) {
            let mut ir_clone = cached.ir.clone();
            merge_file_ir_into(&mut merged_ir, path, &mut ir_clone, &mut merge_warnings);
        }
    }

    // Merge freshly computed IR (consume per_file_ir — no clone needed)
    for (path, (ir_program, local_diags)) in per_file_ir.drain() {
        diags.extend(local_diags);
        let mut ir = ir_program;
        merge_file_ir_into(&mut merged_ir, &path, &mut ir, &mut merge_warnings);
    }

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
