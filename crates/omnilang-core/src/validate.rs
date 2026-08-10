//! Multi-file validation for OmniLang.
//!
//! Validates cross-file imports, visibility, cycle detection, and
//! per-file consistency (e.g., omniview marker matching UI declarations).

use rustc_hash::FxHashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::error::{Diagnostic, DiagnosticKind, Severity};
use crate::parser::ast::{
    AssignTarget, Block, ClassValue, ComponentDef, ElementProp, EventHandler, Expr, ImportStmt,
    Literal, Program, Stmt, TemplatePart, ViewNode, Visibility,
};

/// The marker library import that identifies a file as an OmniView (view) file.
pub(crate) const OMNIVIEW_MARKER: &str = "omniview";

/// Build a file-level diagnostic (line/col = 0) for multi-file validation.
fn make_diag(severity: Severity, kind: DiagnosticKind, file: &str, message: String) -> Diagnostic {
    Diagnostic {
        severity,
        kind,
        message,
        file: Some(file.to_string()),
        line: 0,
        col: 0,
        end_line: None,
        end_col: None,
        suggestion: None,
    }
}

/// Result of multi-file validation.
#[derive(Debug, Serialize, Deserialize)]
pub struct MultiFileResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<Diagnostic>,
}

/// Validate multiple .omni source files with cross-file checks.
///
/// The Rust-native `validate_multi` receives:
///   - `sources`: FxHashMap<ResolvedAbsolutePath, SourceText>
///   - `path_mapping`: FxHashMap<OriginalImportPath, ResolvedAbsolutePath>
///
/// It parses all files, resolves cross-file imports using the mapping,
/// validates visibility and cycles, then returns results for downstream codegen.
#[must_use]
pub fn validate_multi(
    sources: FxHashMap<String, String>,
    path_mapping: FxHashMap<String, String>,
) -> MultiFileResult {
    let mut errors: Vec<Diagnostic> = Vec::new();
    let mut programs: FxHashMap<String, Arc<Program>> = FxHashMap::default();

    // Phase 1: Parse all sources
    for (path, source) in &sources {
        let tokens = crate::lexer::tokenize(source);
        let (program, parse_errors) = crate::parser::parse(tokens, source);
        for e in parse_errors {
            let diag: Diagnostic = e.into();
            errors.push(diag.with_file(path));
        }
        programs.insert(path.clone(), Arc::new(program));
    }

    validate_multi_ref(&programs, &path_mapping)
}

/// Cross-file validation using already-parsed programs.
/// Avoids re-parsing when callers already have parsed ASTs.
#[must_use]
pub fn validate_multi_ref(
    programs: &FxHashMap<String, Arc<Program>>,
    path_mapping: &FxHashMap<String, String>,
) -> MultiFileResult {
    let mut errors: Vec<Diagnostic> = Vec::new();

    // Phase 2: Per-file consistency checks (omniview double-check)
    for (path, program) in programs {
        if let Err(file_errors) = validate_file_consistency(path, program) {
            errors.extend(file_errors);
        }
    }

    // Phase 3: Cross-file import validation
    if let Err(more_errors) = validate_cross_file(programs, path_mapping) {
        errors.extend(more_errors);
    }

    // Phase 4: Cycle detection
    let cycles = crate::semantic::cycles::detect_cycles(programs, path_mapping);
    for cycle in &cycles {
        let file = cycle.first().map(|s| s.as_str()).unwrap_or("unknown");
        errors.push(Diagnostic {
            severity: Severity::Error,
            kind: DiagnosticKind::ECycle,
            message: format!("circular import detected: {}", cycle.join(" → ")),
            file: Some(file.to_string()),
            line: 0,
            col: 0,
            end_line: None,
            end_col: None,
            suggestion: None,
        });
    }

    // Phase 5: Whole-project route registry (V14 hard error + W2 warning)
    errors.extend(validate_route_registry(programs));

    MultiFileResult {
        // MultiFileResult has no separate warnings channel — Warning-severity
        // diagnostics (W2 未注册 navigate) ride the errors Vec. success must
        // therefore gate on Error severity only: warnings never block
        // compilation (批次 2 T5 brief：W2 不阻塞编译，测试断言处确认).
        // analyse.rs recomputes the same severity-based gate at both entries.
        success: !errors.iter().any(|d| d.severity == Severity::Error),
        errors,
    }
}

/// Check per-file consistency: omniview marker must match file contents.
fn validate_file_consistency(file_path: &str, program: &Program) -> Result<(), Vec<Diagnostic>> {
    let mut errors = Vec::new();

    let has_omniview_marker = program.modules.values().any(|module| {
        module
            .imports
            .iter()
            .any(|imp| matches!(imp, ImportStmt::Lib(name) if name == OMNIVIEW_MARKER))
    });

    let has_ui_declarations = program
        .modules
        .values()
        .any(|module| !module.components.is_empty() || !module.stores.is_empty());

    if !has_omniview_marker && has_ui_declarations {
        errors.push(Diagnostic {
            severity: Severity::Warning,
            kind: DiagnosticKind::WOmniviewMarker,
            message: "has UI declarations (component/store/theme) but missing 'import omniview;'. \
             Add 'import omniview;' to mark this as a view file."
                .to_string(),
            file: Some(file_path.to_string()),
            line: 0,
            col: 0,
            end_line: None,
            end_col: None,
            suggestion: Some("Add 'import omniview;' at the top of the file.".to_string()),
        });
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum CrossFileSymbolKind {
    Function,
    Struct,
    TypeAlias,
    Component,
    Store,
    Design,
    Const,
    ReExport(String),
}

/// Build pub-visible symbol tables for cross-file validation.
fn build_pub_symbols(
    programs: &FxHashMap<String, Arc<Program>>,
) -> FxHashMap<String, FxHashMap<String, CrossFileSymbolKind>> {
    let mut pub_symbols: FxHashMap<String, FxHashMap<String, CrossFileSymbolKind>> =
        FxHashMap::default();

    for (path, program) in programs {
        let mut symbols: FxHashMap<String, CrossFileSymbolKind> = FxHashMap::default();

        for module in program.modules.values() {
            for func in &module.functions {
                if func.visibility == Visibility::Public {
                    symbols.insert(func.name.clone(), CrossFileSymbolKind::Function);
                }
            }
            for strukt in &module.structs {
                if strukt.visibility == Visibility::Public {
                    symbols.insert(strukt.name.clone(), CrossFileSymbolKind::Struct);
                }
            }
            for ta in &module.type_aliases {
                if ta.visibility == Visibility::Public {
                    symbols.insert(ta.name.clone(), CrossFileSymbolKind::TypeAlias);
                }
            }
            for comp in &module.components {
                symbols.insert(comp.name.clone(), CrossFileSymbolKind::Component);
            }
            for store in &module.stores {
                symbols.insert(store.name.clone(), CrossFileSymbolKind::Store);
            }
            for design in &module.designs {
                symbols.insert(design.name.clone(), CrossFileSymbolKind::Design);
            }
            for const_def in &module.consts {
                symbols.insert(const_def.name.clone(), CrossFileSymbolKind::Const);
            }
            // Re-exports
            for exp in &module.exports {
                for name in &exp.names {
                    symbols.insert(
                        name.clone(),
                        CrossFileSymbolKind::ReExport(exp.path.clone()),
                    );
                }
            }
        }

        pub_symbols.insert(path.clone(), symbols);
    }

    pub_symbols
}

fn validate_cross_file(
    programs: &FxHashMap<String, Arc<Program>>,
    path_mapping: &FxHashMap<String, String>,
) -> Result<(), Vec<Diagnostic>> {
    let mut errors: Vec<Diagnostic> = Vec::new();
    let pub_symbols = build_pub_symbols(programs);

    for (file_path, program) in programs {
        for module in program.modules.values() {
            for import in &module.imports {
                match import {
                    ImportStmt::Named(named) => {
                        // stdlib imports (std/http, std/json, etc.) are resolved
                        // by the IR/stdlib layer, not via file path mapping
                        if named.path.starts_with("std/") {
                            continue;
                        }
                        if let Some(target) = path_mapping.get(&named.path) {
                            errors.extend(validate_named_import(
                                file_path,
                                &named.names,
                                target,
                                &pub_symbols,
                            ));
                        } else {
                            errors.push(make_diag(
                                Severity::Error,
                                DiagnosticKind::EImport,
                                file_path,
                                format!("could not resolve import path '{}'", named.path),
                            ));
                        }
                    }
                    ImportStmt::Namespace(ns) => {
                        if ns.path.starts_with("std/") {
                            continue;
                        }
                        if let Some(target) = path_mapping.get(&ns.path) {
                            if !programs.contains_key(target.as_str()) {
                                errors.push(make_diag(Severity::Error, DiagnosticKind::EImport, file_path, format!(
                                    "imported file '{}' (resolved to '{}') not found in compilation",
                                    ns.path, target
                                )));
                            }
                        } else {
                            errors.push(make_diag(
                                Severity::Error,
                                DiagnosticKind::EImport,
                                file_path,
                                format!("could not resolve import path '{}'", ns.path),
                            ));
                        }
                    }
                    ImportStmt::TypeOnly(ti) => {
                        if ti.path.starts_with("std/") {
                            continue;
                        }
                        if let Some(target) = path_mapping.get(&ti.path) {
                            errors.extend(validate_named_import(
                                file_path,
                                &ti.names,
                                target,
                                &pub_symbols,
                            ));
                        } else {
                            errors.push(make_diag(
                                Severity::Error,
                                DiagnosticKind::EImport,
                                file_path,
                                format!("could not resolve import path '{}'", ti.path),
                            ));
                        }
                    }
                    ImportStmt::Lib(_) => {
                        // 'import omniview;' — validated in validate_file_consistency
                    }
                }
            }
        }
    }

    // Enforce logic→view boundary
    for (file_path, program) in programs {
        for module in program.modules.values() {
            let is_logic = !module
                .imports
                .iter()
                .any(|imp| matches!(imp, ImportStmt::Lib(name) if name == OMNIVIEW_MARKER));
            if !is_logic {
                continue;
            }
            for import in &module.imports {
                let target_path: Option<&String> = match import {
                    ImportStmt::Named(n) => path_mapping.get(&n.path),
                    ImportStmt::Namespace(ns) => path_mapping.get(&ns.path),
                    ImportStmt::TypeOnly(t) => path_mapping.get(&t.path),
                    ImportStmt::Lib(_) => None,
                };
                if let Some(target) = target_path {
                    if let Some(target_prog) = programs.get(target.as_str()) {
                        let target_is_view = target_prog.modules.values().any(|tm| {
                            tm.imports.iter().any(
                                |imp| matches!(imp, ImportStmt::Lib(name) if name == OMNIVIEW_MARKER),
                            )
                        });
                        if target_is_view {
                            errors.push(make_diag(
                                Severity::Error,
                                DiagnosticKind::EBoundary,
                                file_path,
                                format!("logic file cannot import view file '{}'", target),
                            ));
                        }
                    }
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_named_import(
    importing_file: &str,
    names: &[String],
    target_path: &str,
    pub_symbols: &FxHashMap<String, FxHashMap<String, CrossFileSymbolKind>>,
) -> Vec<Diagnostic> {
    let mut errors = Vec::new();

    let target_symbols = match pub_symbols.get(target_path) {
        Some(s) => s,
        None => {
            errors.push(make_diag(
                Severity::Error,
                DiagnosticKind::EImport,
                importing_file,
                format!("imported file '{}' not found in compilation", target_path),
            ));
            return errors;
        }
    };

    for name in names {
        if !target_symbols.contains_key(name) {
            errors.push(make_diag(
                Severity::Error,
                DiagnosticKind::EVisibility,
                importing_file,
                format!("'{}' is not pub-visible in '{}'", name, target_path),
            ));
        }
    }

    errors
}

/// One whole-project registry claim: the component that declared a route
/// path or layout prefix first (deterministic: sorted files/modules, then
/// declaration order within a module).
struct RouteClaim {
    component: String,
}

/// Phase 5: whole-project route registry validation (spec 2026-08-09 §3
/// V14/W2 + §5.1 白名单).
///
/// Collects every `@route` path and `@layout` prefix across all compiled
/// files into one registry, then:
/// - V14 (hard error): a path is claimed by two *different* components —
///   two `@route`s with the same path, or a `@route` path equal to a
///   `@layout` prefix. Reported at the second claiming component's file,
///   naming the first claimant. Same-component collisions (duplicate
///   `@route` on one component, or `@route` + `@layout` on one component)
///   are V3/V4 territory of the IR-level pass (validate_route_navigation)
///   and are not repeated here.
/// - W2 (warning): every literal `navigate("/path")` call inside component
///   bodies whose path matches no declared `@route`/`@layout`. Dynamic
///   first arguments are skipped (W1 covers those); non-`/` literals are
///   skipped (V12).
///
/// Pure AST level — no IR/lowering needed. W2 is Warning severity so it
/// flows into `errors` without blocking compilation (the success gate in
/// `analyse_multi`/`analyse_multi_cached` is severity-based).
#[must_use]
fn validate_route_registry(programs: &FxHashMap<String, Arc<Program>>) -> Vec<Diagnostic> {
    let mut diagnostics: Vec<Diagnostic> = Vec::new();

    let mut claimed: FxHashMap<String, RouteClaim> = FxHashMap::default();

    // Pass A/B: collect claims in deterministic order, emit V14 on conflict
    // (registry is file-agnostic — same path from two files or one file
    // both collide).
    for file in sorted_file_paths(programs) {
        let program = &programs[file];
        let mut module_names: Vec<&String> = program.modules.keys().collect();
        module_names.sort();
        for module_name in module_names {
            for component in &program.modules[module_name].components {
                for route in &component.route_decls {
                    check_claim(&mut claimed, &mut diagnostics, file, &component.name, &route.path);
                }
                if let Some(layout) = &component.layout {
                    check_claim(
                        &mut claimed,
                        &mut diagnostics,
                        file,
                        &component.name,
                        &layout.prefix,
                    );
                }
            }
        }
    }

    // Pass C: W2 — literal navigate() paths that match no registry entry.
    for file in sorted_file_paths(programs) {
        let program = &programs[file];
        let mut module_names: Vec<&String> = program.modules.keys().collect();
        module_names.sort();
        for module_name in module_names {
            for component in &program.modules[module_name].components {
                let mut paths: Vec<String> = Vec::new();
                collect_navigate_paths(component, &mut paths);
                for path in paths {
                    if !claimed.contains_key(&path) {
                        diagnostics.push(make_diag(
                            Severity::Warning,
                            DiagnosticKind::E0006,
                            file,
                            format!(
                                "路径 '{}' 未匹配到任何 @route/@layout 声明——跳转将落到空路由（目标端行为差异见注记）",
                                path
                            ),
                        ));
                    }
                }
            }
        }
    }

    diagnostics
}

/// Deterministic iteration order for registry passes: sorted file paths.
fn sorted_file_paths(
    programs: &FxHashMap<String, Arc<Program>>,
) -> Vec<&String> {
    let mut files: Vec<&String> = programs.keys().collect();
    files.sort();
    files
}

/// Record one claim; if the same path was already claimed by a *different*
/// component, emit V14 at the current (second claimant's) file, naming the
/// first claimant. Re-claims by the same component are V3/V4 concerns and
/// are ignored here (the first claim stays authoritative).
fn check_claim(
    claimed: &mut FxHashMap<String, RouteClaim>,
    diagnostics: &mut Vec<Diagnostic>,
    file: &str,
    component: &str,
    path: &str,
) {
    match claimed.get(path) {
        Some(first) if first.component != component => {
            diagnostics.push(make_diag(
                Severity::Error,
                DiagnosticKind::E0006,
                file,
                format!(
                    "路径 '{}' 已被组件 '{}' 声明——同一路径/前缀不得由多个组件声明（@route 与 @layout 前缀冲突检查）",
                    path, first.component
                ),
            ));
        }
        _ => {
            claimed.insert(
                path.to_string(),
                RouteClaim {
                    component: component.to_string(),
                },
            );
        }
    }
}

/// Collect every literal `navigate("/path")` target inside one component's
/// executable positions: fns, actions, lifecycles, computeds, getters,
/// signal inits, prop defaults, query defaults, inline stores, slot
/// fallbacks, and the view template.
fn collect_navigate_paths(component: &ComponentDef, out: &mut Vec<String>) {
    for f in &component.fns {
        walk_block(&f.body, out);
    }
    for a in &component.actions {
        walk_block(&a.body, out);
    }
    for lc in &component.lifecycles {
        walk_block(&lc.body, out);
    }
    for c in &component.computeds {
        walk_expr(&c.body, out);
    }
    for g in &component.getters {
        walk_expr(&g.body, out);
    }
    for s in &component.signals {
        walk_expr(&s.init, out);
    }
    for p in &component.props {
        if let Some(default) = &p.default {
            walk_expr(default, out);
        }
    }
    for q in &component.queries {
        if let Some(default) = &q.default {
            walk_expr(default, out);
        }
    }
    for store in &component.inline_store_defs {
        for s in &store.signals {
            walk_expr(&s.init, out);
        }
        for g in &store.getters {
            walk_expr(&g.body, out);
        }
        for a in &store.actions {
            walk_block(&a.body, out);
        }
    }
    for slot in &component.slots {
        if let Some(content) = &slot.default_content {
            for node in content {
                walk_view_node(node, out);
            }
        }
    }
    if let Some(view) = &component.view {
        for node in &view.children {
            walk_view_node(node, out);
        }
    }
}

/// Recurse an expression tree, recording literal `navigate("/...")` targets.
fn walk_expr(expr: &Expr, out: &mut Vec<String>) {
    match expr {
        Expr::FuncCall { func, args, .. } => {
            if let Expr::VarRef(name, _) = &**func {
                if name == "navigate" {
                    if let Some(Expr::Literal(Literal::String(path), _)) = args.first() {
                        if path.starts_with('/') {
                            out.push(path.clone());
                        }
                    }
                    // Dynamic first arg → W1; non-`/` literal → V12.
                }
            }
            // The call target itself may contain expressions; walk it too.
            walk_expr(func, out);
            for arg in args {
                walk_expr(arg, out);
            }
        }
        Expr::Literal(..) | Expr::VarRef(..) | Expr::StoreFieldAccess { .. } => {}
        Expr::BinaryOp { left, right, .. } => {
            walk_expr(left, out);
            walk_expr(right, out);
        }
        Expr::UnaryOp { operand, .. } => walk_expr(operand, out),
        Expr::MethodCall { object, args, .. } => {
            walk_expr(object, out);
            for arg in args {
                walk_expr(arg, out);
            }
        }
        Expr::IndexGet { object, index, .. } => {
            walk_expr(object, out);
            walk_expr(index, out);
        }
        Expr::FieldAccess { object, .. } => walk_expr(object, out),
        Expr::OptionalChain { object, .. } => walk_expr(object, out),
        Expr::ArrayLiteral { elements, .. } => {
            for e in elements {
                walk_expr(e, out);
            }
        }
        Expr::MapLiteral { entries, .. } => {
            for (key, value) in entries {
                walk_expr(key, out);
                walk_expr(value, out);
            }
        }
        Expr::IfExpr { condition, then_expr, else_expr, .. } => {
            walk_expr(condition, out);
            walk_expr(then_expr, out);
            walk_expr(else_expr, out);
        }
        Expr::TernaryExpr { condition, then_expr, else_expr, .. } => {
            walk_expr(condition, out);
            walk_expr(then_expr, out);
            walk_expr(else_expr, out);
        }
        Expr::TemplateString { parts, .. } => {
            for part in parts {
                if let TemplatePart::Expr(e) = part {
                    walk_expr(e, out);
                }
            }
        }
        Expr::StructLiteral { fields, .. } => {
            for (_, value) in fields {
                walk_expr(value, out);
            }
        }
        Expr::Block { body, .. } => walk_block(body, out),
        Expr::ClosureExpr { body, .. } => walk_expr(body, out),
        Expr::AwaitExpr { operand, .. } => walk_expr(operand, out),
        Expr::MatchExpr { matched, arms, .. } => {
            walk_expr(matched, out);
            for arm in arms {
                walk_block(&arm.body, out);
            }
        }
        Expr::SpreadExpr { operand, .. } => walk_expr(operand, out),
        Expr::IsCheck { value, .. } => walk_expr(value, out),
        Expr::BuiltinToastCall { msg, kind, duration, .. } => {
            walk_expr(msg, out);
            if let Some(k) = kind {
                walk_expr(k, out);
            }
            if let Some(d) = duration {
                walk_expr(d, out);
            }
        }
        Expr::NotYetImplemented { .. } | Expr::Error { .. } => {}
    }
}

fn walk_block(block: &Block, out: &mut Vec<String>) {
    for stmt in &block.stmts {
        walk_stmt(stmt, out);
    }
    if let Some(trailing) = &block.trailing_expr {
        walk_expr(trailing, out);
    }
}

fn walk_stmt(stmt: &Stmt, out: &mut Vec<String>) {
    match stmt {
        Stmt::VarDecl { init, .. } => {
            if let Some(init) = init {
                walk_expr(init, out);
            }
        }
        Stmt::Assign { target, value, .. } => {
            walk_assign_target(target, out);
            walk_expr(value, out);
        }
        Stmt::If {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            walk_expr(condition, out);
            walk_block(then_body, out);
            for (cond, body) in else_ifs {
                walk_expr(cond, out);
                walk_block(body, out);
            }
            if let Some(body) = else_body {
                walk_block(body, out);
            }
        }
        Stmt::While { condition, body, .. } => {
            walk_expr(condition, out);
            walk_block(body, out);
        }
        Stmt::ForIn { iterable, body, .. } => {
            walk_expr(iterable, out);
            walk_block(body, out);
        }
        Stmt::Return { value, .. } => {
            if let Some(value) = value {
                walk_expr(value, out);
            }
        }
        Stmt::Break(_) | Stmt::Continue(_) => {}
        Stmt::Print { arg, .. } => walk_expr(arg, out),
        Stmt::ExprStmt { expr, .. } => walk_expr(expr, out),
        Stmt::EventEmit { args, .. } => {
            for arg in args {
                walk_expr(arg, out);
            }
        }
        Stmt::Throw { expr, .. } => walk_expr(expr, out),
        Stmt::TryCatch {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            walk_block(try_body, out);
            if let Some(body) = catch_body {
                walk_block(body, out);
            }
            if let Some(body) = finally_body {
                walk_block(body, out);
            }
        }
        Stmt::Error { .. } => {}
    }
}

fn walk_assign_target(target: &AssignTarget, out: &mut Vec<String>) {
    match target {
        AssignTarget::Name(_) | AssignTarget::Field { .. } => {}
        AssignTarget::Index { index, .. } => walk_expr(index, out),
    }
}

fn walk_view_node(node: &ViewNode, out: &mut Vec<String>) {
    match node {
        ViewNode::Element(element) => {
            for prop in &element.props {
                walk_element_prop(prop, out);
            }
            for child in &element.children {
                walk_view_node(child, out);
            }
        }
        ViewNode::IfDirective(directive) => {
            walk_expr(&directive.condition, out);
            for node in &directive.body {
                walk_view_node(node, out);
            }
            for (cond, body) in &directive.else_ifs {
                walk_expr(cond, out);
                for node in body {
                    walk_view_node(node, out);
                }
            }
            if let Some(body) = &directive.else_body {
                for node in body {
                    walk_view_node(node, out);
                }
            }
        }
        ViewNode::ForDirective(directive) => {
            walk_expr(&directive.iterable, out);
            for node in &directive.body {
                walk_view_node(node, out);
            }
            if let Some(empty) = &directive.empty_body {
                for node in empty {
                    walk_view_node(node, out);
                }
            }
            if let Some(key) = &directive.key_expr {
                walk_expr(key, out);
            }
        }
        ViewNode::SlotDirective { default_content, .. } => {
            if let Some(content) = default_content {
                for node in content {
                    walk_view_node(node, out);
                }
            }
        }
        ViewNode::Interpolation(expr) => walk_expr(expr, out),
        ViewNode::Text(_) | ViewNode::LoadDirective { .. } => {}
    }
}

fn walk_element_prop(prop: &ElementProp, out: &mut Vec<String>) {
    match prop {
        ElementProp::Static { value, .. } => walk_expr(value, out),
        ElementProp::Bind { .. } => {}
        ElementProp::RefBinding { signal, .. } => walk_expr(signal, out),
        ElementProp::Event { handler, .. } => {
            if let EventHandler::InlineFn { body, .. } = handler {
                walk_block(body, out);
            }
        }
        ElementProp::Class { value } => {
            if let ClassValue::ClassMap(entries) = value {
                for (_, cond) in entries {
                    walk_expr(cond, out);
                }
            }
        }
        ElementProp::Variant { .. } => {}
        ElementProp::SlotContent { content, .. } => {
            for node in content {
                walk_view_node(node, out);
            }
        }
        ElementProp::Style { entries } => {
            for (_, value) in entries {
                walk_expr(value, out);
            }
        }
        ElementProp::A11y(spec) => {
            if let Some(hidden) = &spec.hidden {
                walk_expr(hidden, out);
            }
            if let Some(alt) = &spec.alt {
                walk_expr(alt, out);
            }
        }
        ElementProp::Ref(_) => {}
        ElementProp::MotionTrigger(trigger) => walk_expr(&trigger.motion_expr, out),
        ElementProp::Form => {}
        ElementProp::Validate { rules } => {
            for rule in rules {
                walk_expr(&rule.value, out);
            }
        }
        ElementProp::Drag { .. } => {}
        ElementProp::Modal { open_signal } => walk_expr(open_signal, out),
        ElementProp::Skeleton { .. } | ElementProp::ErrorBoundary => {}
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn make_sources(files: Vec<(&str, &str)>) -> FxHashMap<String, String> {
        files
            .into_iter()
            .map(|(path, content)| (path.to_string(), content.to_string()))
            .collect()
    }

    /// Helper: build sources and derive their path mapping, then validate.
    fn validate_sources(sources: FxHashMap<String, String>) -> MultiFileResult {
        let path_mapping = crate::path_mapping::derive_path_mapping(&sources);
        validate_multi(sources, path_mapping)
    }

    #[test]
    fn validate_single_file_no_issues() {
        let sources = make_sources(vec![("main.omni", "fn main() -> i32 { return 42; }")]);
        let result = validate_sources(sources);
        assert!(result.success, "single function file should validate ok");
        assert!(
            result.errors.is_empty(),
            "expected no errors, got: {:?}",
            result.errors
        );
    }

    #[test]
    fn validate_cross_file_import_resolves() {
        let sources = make_sources(vec![
            ("a.omni", "pub fn helper() -> i32 { return 1; }"),
            (
                "b.omni",
                "import { helper } from \"./a.omni\"; fn main() -> i32 { return helper(); }",
            ),
        ]);
        let result = validate_sources(sources);
        assert!(
            result.success,
            "cross-file import of pub symbol should succeed, got errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn validate_cross_file_private_symbol_fails() {
        // helper is NOT pub — should produce visibility error
        let sources = make_sources(vec![
            ("a.omni", "fn helper() -> i32 { return 1; }"),
            (
                "b.omni",
                "import { helper } from \"./a.omni\"; fn main() -> i32 { return helper(); }",
            ),
        ]);
        let result = validate_sources(sources);
        assert!(!result.success, "import of private symbol should fail");
        let has_vis_error = result
            .errors
            .iter()
            .any(|d| d.kind == DiagnosticKind::EVisibility);
        assert!(
            has_vis_error,
            "expected E_VISIBILITY error, got kinds: {:?}",
            result.errors.iter().map(|d| &d.kind).collect::<Vec<_>>()
        );
    }

    #[test]
    fn validate_unresolved_import_path_fails() {
        let sources = make_sources(vec![(
            "main.omni",
            "import { stuff } from \"./missing.omni\"; fn main() {}",
        )]);
        let path_mapping = make_sources(vec![("main.omni", "main.omni")]);
        // Don't include "./missing.omni" in the path mapping, so it can't be resolved
        let result = validate_multi(sources, path_mapping);
        assert!(!result.success, "unresolvable import should fail");
        let has_import_error = result
            .errors
            .iter()
            .any(|d| d.kind == DiagnosticKind::EImport);
        assert!(
            has_import_error,
            "expected E_IMPORT error, got kinds: {:?}",
            result.errors.iter().map(|d| &d.kind).collect::<Vec<_>>()
        );
    }

    // ── Phase 5: route registry (V14 + W2) ─────────────────────────────

    const V14_COMPONENT_X: &str =
        "import omniview; component X { @route \"/todos\"; view { <div/> } }";
    const V14_COMPONENT_Y: &str =
        "import omniview; component Y { @route \"/todos\"; view { <div/> } }";

    /// Assert the result carries exactly one V14 error naming `path` and
    /// `first_component`, and that validation failed.
    fn assert_v14(result: &MultiFileResult, path: &str, first_component: &str) {
        let v14: Vec<&Diagnostic> = result
            .errors
            .iter()
            .filter(|d| d.message.contains("已被组件"))
            .collect();
        assert_eq!(
            v14.len(),
            1,
            "expected exactly one V14 error, got: {:?}",
            result.errors
        );
        let msg = &v14[0].message;
        assert!(
            msg.contains(path) && msg.contains(first_component),
            "V14 must name the path and the first claimant, got: {msg}"
        );
        assert!(
            matches!(v14[0].severity, Severity::Error),
            "V14 must be a hard error"
        );
        assert!(!result.success, "V14 must fail validation");
    }

    #[test]
    fn test_v14_duplicate_route_path_cross_file() {
        let sources = make_sources(vec![("a.omni", V14_COMPONENT_X), ("b.omni", V14_COMPONENT_Y)]);
        let result = validate_sources(sources);
        // Sorted files: a.omni first → X is the first claimant, Y the second.
        assert_v14(&result, "/todos", "X");
    }

    #[test]
    fn test_v14_route_layout_prefix_conflict() {
        let sources = make_sources(vec![(
            "app.omni",
            "import omniview; component X { @route \"/admin\"; view { <div/> } } \
             component Y { @layout \"/admin\"; view { <div/> } }",
        )]);
        let result = validate_sources(sources);
        assert_v14(&result, "/admin", "X");
    }

    #[test]
    fn test_v14_same_file_duplicate() {
        let sources = make_sources(vec![(
            "app.omni",
            "import omniview; component A { @route \"/home\"; view { <div/> } } \
             component B { @route \"/home\"; view { <div/> } }",
        )]);
        let result = validate_sources(sources);
        // Registry is file-agnostic: same-file duplicates collide too.
        assert_v14(&result, "/home", "A");
    }

    #[test]
    fn test_w2_navigate_unregistered_path() {
        let sources = make_sources(vec![(
            "app.omni",
            "import omniview; component App { fn go() { navigate(\"/nonexistent\"); } \
             view { <div/> } }",
        )]);
        let result = validate_sources(sources);
        let w2: Vec<&Diagnostic> = result
            .errors
            .iter()
            .filter(|d| d.message.contains("未匹配到任何"))
            .collect();
        assert_eq!(
            w2.len(),
            1,
            "expected exactly one W2 warning, got: {:?}",
            result.errors
        );
        assert!(
            w2[0].message.contains("/nonexistent"),
            "W2 must name the unmatched path, got: {}",
            w2[0].message
        );
        assert!(
            matches!(w2[0].severity, Severity::Warning),
            "W2 must be a warning (not blocking)"
        );
        // Brief requirement: W2 must NOT block compilation. success is flipped
        // only by Error-severity diagnostics (severity-based gate in both
        // analyse entries), so a Warning-only result must stay successful.
        assert!(
            result.success,
            "W2 must not block compilation, got success={} with errors: {:?}",
            result.success,
            result.errors
        );
    }

    #[test]
    fn test_w2_navigate_registered_path() {
        let sources = make_sources(vec![(
            "app.omni",
            "import omniview; component App { @route \"/todos\"; \
             fn go() { navigate(\"/todos\"); } view { <div/> } }",
        )]);
        let result = validate_sources(sources);
        assert!(
            !result.errors.iter().any(|d| d.message.contains("未匹配到任何")),
            "registered navigate must not W2, got: {:?}",
            result.errors
        );
        assert!(
            !result.errors.iter().any(|d| d.message.contains("已被组件")),
            "single route must not V14, got: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_registry_scope_both_entries() {
        let sources = make_sources(vec![("a.omni", V14_COMPONENT_X), ("b.omni", V14_COMPONENT_Y)]);
        let path_mapping = crate::path_mapping::derive_path_mapping(&sources);

        // Entry 1: analyse_multi (lib.rs analyse_multi entry)
        let fresh = crate::analyse::analyse_multi(&sources, &path_mapping);
        assert!(
            fresh.diagnostics.iter().any(|d| d.message.contains("已被组件")),
            "analyse_multi must surface V14, got: {:?}",
            fresh.diagnostics
        );
        assert!(!fresh.success, "V14 must fail analyse_multi");

        // Entry 2: analyse_multi_cached (lib.rs analyse_multi_cached entry)
        let mut cache = crate::analyse::IncrementalCache::new();
        let cached = crate::analyse::analyse_multi_cached(&sources, &path_mapping, &mut cache);
        assert!(
            cached.diagnostics.iter().any(|d| d.message.contains("已被组件")),
            "analyse_multi_cached must surface V14, got: {:?}",
            cached.diagnostics
        );
        assert!(!cached.success, "V14 must fail analyse_multi_cached");
    }

    #[test]
    fn test_w2_does_not_block_analyse_entries() {
        // Analyse-level counterpart of test_w2_navigate_unregistered_path:
        // with only a W2 warning present, BOTH analyse entries must keep
        // success=true (severity-based success gate, not MultiFileResult.success).
        let sources = make_sources(vec![(
            "app.omni",
            "import omniview; component App { fn go() { navigate(\"/nowhere\"); } \
             view { <div/> } }",
        )]);
        let path_mapping = crate::path_mapping::derive_path_mapping(&sources);

        let fresh = crate::analyse::analyse_multi(&sources, &path_mapping);
        assert!(
            fresh.diagnostics.iter().any(|d| d.message.contains("未匹配到任何")),
            "analyse_multi must surface W2, got: {:?}",
            fresh.diagnostics
        );
        assert!(
            fresh.success,
            "W2 must not flip analyse_multi success, got diagnostics: {:?}",
            fresh.diagnostics
        );

        let mut cache = crate::analyse::IncrementalCache::new();
        let cached = crate::analyse::analyse_multi_cached(&sources, &path_mapping, &mut cache);
        assert!(
            cached.diagnostics.iter().any(|d| d.message.contains("未匹配到任何")),
            "analyse_multi_cached must surface W2, got: {:?}",
            cached.diagnostics
        );
        assert!(
            cached.success,
            "W2 must not flip analyse_multi_cached success, got diagnostics: {:?}",
            cached.diagnostics
        );
    }
}
