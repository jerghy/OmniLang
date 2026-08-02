//! Design codegen for the React backend.
//!
//! React renders plain HTML — there is no `[data-design~="..."]` wrapper
//! element, so the shared design CSS emitter runs with an empty scope
//! prefix.  Each source file that declares `design` blocks produces up to
//! three outputs (all placed in the output directory mirroring the design's
//! source file):
//!
//! - `design.css` — CSS custom properties (`:root` + `[data-theme]`
//!   overrides), class rules, and variant combination classes via
//!   `crate::codegen::shared::design::emit_design_css`
//! - `useDesignTokens.ts` — a `useDesignTokens()` hook that applies
//!   `TokenValueIR::Dynamic` token values to
//!   `document.documentElement.style` via `useEffect` (one effect per
//!   dynamic token, deps = the store signal expression)
//! - `useThemeStore.ts` — a Zustand store for theme switching
//!   (`theme`/`setTheme`) that applies `data-theme` to `<html>` whenever
//!   the theme changes, mirroring the `[data-theme="..."]` CSS scoping

use crate::codegen::json::escape_literal_str;
use crate::codegen::react::{relative_import_path, source_dir_prefix};
use crate::codegen::shared::design as shared_design;
use crate::codegen::CodeWriter;
use crate::ir::design::{DesignIR, TokenValueIR};
use crate::ir::{ExprIR, IRProgram};
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

/// One set of design outputs (`design.css`, `useDesignTokens.ts`,
/// `useThemeStore.ts`) — the designs declared in one source file (or the
/// output root for designs without a known source file).
pub(super) struct DesignGroup {
    /// Output directory prefix (`""` for the output root, `"components/"`…).
    pub dir_prefix: String,
    /// Whether any design in the group declares dynamic tokens.
    pub has_dynamic_tokens: bool,
    /// Whether any design in the group declares `@themes`.
    pub has_themes: bool,
}

/// Group the program's design blocks by output directory (designs from the
/// same source file share one set of outputs).  Sorted by directory for
/// deterministic output.
pub(super) fn design_groups(ir: &IRProgram) -> Vec<DesignGroup> {
    let mut by_dir: FxHashMap<String, Vec<&DesignIR>> = FxHashMap::default();
    for module in ir.modules.values() {
        for design in &module.designs {
            let key = source_dir_prefix(design.source_file.as_deref());
            by_dir.entry(key).or_default().push(design);
        }
    }
    let mut keys: Vec<String> = by_dir.keys().cloned().collect();
    keys.sort();

    keys.into_iter()
        .map(|dir_prefix| {
            let designs = &by_dir[&dir_prefix];
            DesignGroup {
                has_dynamic_tokens: designs.iter().any(|d| {
                    d.tokens
                        .values()
                        .any(|t| matches!(t.value, TokenValueIR::Dynamic(_)))
                }),
                has_themes: designs.iter().any(|d| !d.themes.is_empty()),
                dir_prefix,
            }
        })
        .collect()
}

/// Group the program's design blocks by source file and emit all design
/// outputs (`design.css`, `useDesignTokens.ts`, `useThemeStore.ts`) as
/// `CompiledFile`s.  Returns an empty vector when the program has no
/// designs.
pub fn gen_design_files(ir: &IRProgram) -> Vec<crate::codegen::CompiledFile> {
    let mut files = Vec::new();
    for group in design_groups(ir) {
        let mut designs: Vec<&DesignIR> = ir
            .modules
            .values()
            .flat_map(|m| &m.designs)
            .filter(|d| source_dir_prefix(d.source_file.as_deref()) == group.dir_prefix)
            .collect();
        // Deterministic order: designs sorted by name within a file.
        designs.sort_by(|a, b| a.name.cmp(&b.name));

        files.push(crate::codegen::CompiledFile {
            path: format!("{}design.css", group.dir_prefix),
            content: emit_design_css_file(&designs),
        });
        if group.has_dynamic_tokens {
            if let Some(content) = emit_use_design_tokens(&designs, ir) {
                files.push(crate::codegen::CompiledFile {
                    path: format!("{}useDesignTokens.ts", group.dir_prefix),
                    content,
                });
            }
        }
        if group.has_themes {
            if let Some(content) = emit_theme_store(&designs) {
                files.push(crate::codegen::CompiledFile {
                    path: format!("{}useThemeStore.ts", group.dir_prefix),
                    content,
                });
            }
        }
    }
    files
}

/// Pre-computed design information used by the JSX emitter to decide how
/// class names are emitted.
///
/// Design classes and variant combination classes are global CSS rules in
/// `design.css` — components must reference them with literal class names
/// (`className="Page"`).  Component-local style blocks live in the CSS
/// module and stay `styles['...']` lookups.
pub(super) struct DesignCtx<'a> {
    /// Design name → design IR (used to expand `variant:` bindings).
    pub design_map: FxHashMap<&'a str, &'a DesignIR>,
    /// Class names, variant names and variant branch values declared by any
    /// design block — emitted as literal class names.
    pub global_class_names: FxHashSet<String>,
}

impl<'a> DesignCtx<'a> {
    pub(super) fn new(ir: &'a IRProgram) -> Self {
        let mut design_map = FxHashMap::default();
        let mut global_class_names = FxHashSet::default();
        for module in ir.modules.values() {
            for design in &module.designs {
                design_map.insert(design.name.as_str(), design);
                for class in &design.classes {
                    global_class_names.insert(class.name.clone());
                }
                for variant in &design.variants {
                    global_class_names.insert(variant.name.clone());
                    for branch in &variant.branches {
                        global_class_names.insert(branch.value.clone());
                    }
                }
            }
        }
        DesignCtx {
            design_map,
            global_class_names,
        }
    }

    /// Format one class name for JSX: a literal string for design classes
    /// (global rules in `design.css`), a CSS-module lookup otherwise.
    ///
    /// Layer 2 of the class-name injection double-insurance: quotes and
    /// backslashes are escaped even though lowering already restricts class
    /// names to `[a-zA-Z0-9_-]` — a payload that bypasses charset validation
    /// (e.g. a hand-built IR) still cannot break out of the emitted
    /// `styles['...']` / `"..."` literals into executable code.
    pub(super) fn format_class_ref(&self, name: &str) -> String {
        if self.global_class_names.contains(name) {
            format!("\"{}\"", escape_literal_str(name, '"'))
        } else {
            format!("styles['{}']", escape_literal_str(name, '\''))
        }
    }

    /// Expand a `_design_variant` prop (encoded as
    /// `{ design_name, variant_name, dimension_values }`) into its class
    /// names, e.g. `LearnDesign.Button(color=primary)` → `["Button",
    /// "primary"]`.  Returns `None` when the design or variant is unknown.
    pub(super) fn expand_variant_classes(&self, prop_value: &ExprIR) -> Option<Vec<String>> {
        let obj = match prop_value {
            ExprIR::LiteralExpr { value } => value.as_object()?,
            _ => return None,
        };
        let design_name = obj.get("design_name")?.as_str()?;
        let variant_name = obj.get("variant_name")?.as_str()?;
        let design = self.design_map.get(design_name)?;
        let dim_map: FxHashMap<String, String> = obj
            .get("dimension_values")
            .and_then(|v| v.as_object())
            .map(|o| {
                o.iter()
                    .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_string()))
                    .collect()
            })
            .unwrap_or_default();
        shared_design::expand_variant(design, variant_name, &dim_map)
            .ok()
            .map(|(classes, _)| classes)
    }
}

/// Whether a component is the app root — the component that mounts the
/// design system runtime (dynamic token effects, theme subscription).
/// Preference: `App`, else `Main`, else the last-sorted component name
/// (deterministic across hash-map iteration).
pub(super) fn is_root_component(name: &str, ir: &IRProgram) -> bool {
    let all_names: Vec<&str> = ir.all_components().keys().copied().collect();
    if all_names.iter().any(|n| *n == "App") {
        return name == "App";
    }
    if all_names.iter().any(|n| *n == "Main") {
        return name == "Main";
    }
    let mut sorted = all_names;
    sorted.sort_unstable();
    sorted.last().is_some_and(|last| *last == name)
}

/// Design runtime wiring emitted into the root component so the design
/// system activates with the app:
///
/// - Named imports of every `useDesignTokens` hook (one per design group
///   with dynamic tokens) plus a `useDesignTokens()` call in the component
///   body — the hook effects apply dynamic token values to
///   `document.documentElement.style` (HIGH #2: the hook is dead code unless
///   something calls it).
/// - Side-effect imports of every `useThemeStore` — the module-level
///   `data-theme` subscription runs on import.
pub(super) struct DesignWiring {
    /// Import lines to emit at the top of the root component file.
    pub import_lines: Vec<String>,
    /// Hook names to call inside the root component body.
    pub hook_calls: Vec<String>,
}

pub(super) fn root_design_wiring(ir: &IRProgram, source_file: Option<&str>) -> DesignWiring {
    let groups = design_groups(ir);
    let mut import_lines = Vec::new();
    let mut hook_calls = Vec::new();
    let mut hook_idx = 0usize;
    for group in &groups {
        let to_source = format!("{}design.omni", group.dir_prefix);
        if group.has_dynamic_tokens {
            // Multiple design groups each get their own hook — alias the
            // imports so the names stay unique in one module.
            let local_name = if hook_idx == 0 {
                "useDesignTokens".to_string()
            } else {
                format!("useDesignTokens{}", hook_idx + 1)
            };
            let import_path =
                relative_import_path(source_file, Some(&to_source), "useDesignTokens");
            if local_name == "useDesignTokens" {
                import_lines.push(format!(
                    "import {{ useDesignTokens }} from '{}';",
                    import_path
                ));
            } else {
                import_lines.push(format!(
                    "import {{ useDesignTokens as {} }} from '{}';",
                    local_name, import_path
                ));
            }
            hook_calls.push(local_name);
            hook_idx += 1;
        }
        if group.has_themes {
            let import_path = relative_import_path(source_file, Some(&to_source), "useThemeStore");
            import_lines.push(format!("import '{}';", import_path));
        }
    }
    DesignWiring {
        import_lines,
        hook_calls,
    }
}

/// Emit `design.css` for a group of design blocks (one per source file).
///
/// The shared emitter runs with an empty scope prefix — React output is
/// plain HTML without a `[data-design~="..."]` wrapper element, so class
/// and variant rules are global CSS classes.
fn emit_design_css_file(designs: &[&DesignIR]) -> String {
    let mut css = String::new();
    for design in designs {
        if let Ok(block) = shared_design::emit_design_css(design, "") {
            if !block.is_empty() {
                css.push_str(&format!("/* ── Design: {} ── */\n", design.name));
                css.push_str(&block);
            }
        }
    }
    css
}

/// Emit `useDesignTokens.ts` — a hook applying all dynamic token values of
/// the given designs to `document.documentElement.style` via `useEffect`.
///
/// Returns `None` when no design declares a dynamic token.
///
/// For each dynamic token a separate effect is emitted:
/// ```tsx
/// useEffect(() => {
///   document.documentElement.style.setProperty('--dynColor', sanitizeCss(CartStore.liveColor));
/// }, [CartStore.liveColor]);
/// ```
/// The effect deps are the exact store-signal expression, so unrelated
/// store updates do not re-fire it.  Store hooks are instantiated once per
/// referenced store (`const CartStore = useCartStore();`), matching the
/// component codegen convention.
fn emit_use_design_tokens(designs: &[&DesignIR], ir: &IRProgram) -> Option<String> {
    let mut dynamic_tokens: Vec<(String, &ExprIR)> = Vec::new();
    for design in designs {
        let mut sorted: Vec<_> = design.tokens.iter().collect();
        sorted.sort_by(|a, b| a.0.cmp(b.0));
        for (name, token) in sorted {
            if let TokenValueIR::Dynamic(expr) = &token.value {
                dynamic_tokens.push((name.clone(), expr));
            }
        }
    }
    if dynamic_tokens.is_empty() {
        return None;
    }

    // Stores referenced by the dynamic token expressions.
    let all_stores = ir.all_stores();
    let mut store_names: FxHashSet<String> = FxHashSet::default();
    for (_, expr) in &dynamic_tokens {
        collect_store_refs(expr, &all_stores, &mut store_names);
    }
    let mut known_stores: Vec<String> = store_names
        .iter()
        .filter(|n| all_stores.contains_key(n.as_str()))
        .cloned()
        .collect();
    known_stores.sort();

    let source_file = designs.first().and_then(|d| d.source_file.as_deref());

    let mut w = CodeWriter::new();
    w.emit("// Generated by OmniLang — dynamic design token hook");
    w.emit("import { useEffect } from 'react';");
    for store_name in &known_stores {
        let hook_name = format!("use{}", store_name);
        let store = all_stores.get(store_name.as_str()).copied();
        let import_path = relative_import_path(
            source_file,
            store.and_then(|s| s.source_file.as_deref()),
            &hook_name,
        );
        w.emit_fmt(format_args!(
            "import {{ {} }} from '{}';",
            hook_name, import_path
        ));
    }
    w.emit("");
    emit_sanitize_css(&mut w);
    w.emit("");
    w.emit("export function useDesignTokens() {");
    w.indent();
    for store_name in &known_stores {
        w.emit_fmt(format_args!("const {} = use{}();", store_name, store_name));
    }
    for (name, expr) in &dynamic_tokens {
        let js_expr = super::expr::format_expr_ir_tsx(expr, &FxHashSet::default());
        w.emit("useEffect(() => {");
        w.indent();
        w.emit_fmt(format_args!(
            "document.documentElement.style.setProperty('--{}', sanitizeCss({}));",
            name, js_expr
        ));
        w.dedent();
        w.emit_fmt(format_args!("}}, [{}]);", js_expr));
        w.emit("");
    }
    w.dedent();
    w.emit("}");
    Some(w.into_string())
}

/// Emit the module-local `sanitizeCss` + `normalizeCssEscapes` helpers from
/// the SHARED sanitizer emitter (single source of truth with the OmniView
/// runtime `__omni_sanitizeCss`, task 3.2): hex-escape normalization,
/// CR/LF/TAB pre/post-strip, the full keyword block (including
/// `data:text/html`, `document.`, `window.`) and the url() context checks,
/// so store-driven token values cannot inject CSS. See
/// `crate::codegen::shared::sanitize`.
fn emit_sanitize_css(w: &mut CodeWriter) {
    w.emit("// Strip dangerous CSS sequences from runtime values (shared sanitizer");
    w.emit("// emitter — identical logic to the OmniView runtime __omni_sanitizeCss)");
    w.emit("// so store-driven token values cannot inject CSS.");
    let helpers = crate::codegen::shared::sanitize::sanitize_css_helpers_js(
        "sanitizeCss",
        "normalizeCssEscapes",
        true,
    );
    for line in helpers.lines() {
        w.emit(line);
    }
}

/// Emit `useThemeStore.ts` — a Zustand store for theme switching.
///
/// Emitted when any design in the group declares `@themes`.  The store
/// carries `theme` + `setTheme`; a module-level `subscribe` applies
/// `data-theme` to `<html>` on every change, and the initial theme is
/// applied at import time so the `[data-theme="..."]` CSS overrides work
/// immediately.
///
/// Returns `None` when no design declares themes.
fn emit_theme_store(designs: &[&DesignIR]) -> Option<String> {
    // Union of theme names across all designs in the group (sorted).
    let mut theme_names: Vec<String> = Vec::new();
    for design in designs {
        for theme in &design.themes {
            if !theme_names.contains(theme) {
                theme_names.push(theme.clone());
            }
        }
    }
    theme_names.sort();
    if theme_names.is_empty() {
        return None;
    }

    // Default theme: the first design's @default-theme, else the first
    // declared theme, else "light".
    let default_theme = designs
        .iter()
        .find_map(|d| d.default_theme.clone())
        .unwrap_or_else(|| {
            theme_names
                .first()
                .cloned()
                .unwrap_or_else(|| "light".to_string())
        });

    let mut w = CodeWriter::new();
    w.emit("// Generated by OmniLang — theme store");
    w.emit("import { create } from 'zustand';");
    w.emit("");
    w.emit("interface ThemeState {");
    w.indent();
    w.emit("theme: string;");
    w.emit("setTheme: (theme: string) => void;");
    w.dedent();
    w.emit("}");
    w.emit("");
    w.emit("export const useThemeStore = create<ThemeState>()((set) => ({");
    w.indent();
    w.emit_fmt(format_args!("theme: '{}',", default_theme));
    w.emit("setTheme: (theme) => set({ theme }),");
    w.dedent();
    w.emit("}));");
    w.emit("");
    w.emit("// Apply the current theme to <html data-theme=\"...\"> whenever it changes,");
    w.emit("// matching the [data-theme=\"...\"] overrides in design.css.");
    w.emit("useThemeStore.subscribe((state) => {");
    w.indent();
    w.emit("document.documentElement.setAttribute('data-theme', state.theme);");
    w.dedent();
    w.emit("});");
    w.emit("");
    w.emit("// Apply the default theme immediately (subscribe only fires on changes).");
    w.emit("document.documentElement.setAttribute(");
    w.indent();
    w.emit("'data-theme',");
    w.emit("useThemeStore.getState().theme,");
    w.dedent();
    w.emit(");");
    Some(w.into_string())
}

/// Collect store names referenced by a dynamic token expression.
///
/// Design-token expressions are parsed standalone (the design parser cannot
/// resolve store symbols), so `Store.signal` lowers to
/// `FieldAccessExpr { object: IdentifierExpr("Store"), .. }` rather than
/// `StoreFieldAccessExpr`.  Both shapes are recognized; the field-access
/// shape is only treated as a store reference when the object identifier
/// matches a known store (so plain object fields like `event.foo` are not
/// misclassified).
fn collect_store_refs(
    expr: &ExprIR,
    all_stores: &FxHashMap<&str, &crate::ir::StoreIR>,
    acc: &mut FxHashSet<String>,
) {
    match expr {
        ExprIR::StoreFieldAccessExpr { store_alias, .. } => {
            let name = store_alias.trim_start_matches('$');
            if all_stores.contains_key(name) {
                acc.insert(name.to_string());
            }
        }
        ExprIR::FieldAccessExpr { object, .. } | ExprIR::OptionalChainExpr { object, .. } => {
            if let ExprIR::IdentifierExpr { name, .. } = object.as_ref() {
                let stripped = name.trim_start_matches('$');
                if all_stores.contains_key(stripped) {
                    acc.insert(stripped.to_string());
                }
            }
            collect_store_refs(object, all_stores, acc);
        }
        ExprIR::BinaryOpExpr { left, right, .. } => {
            collect_store_refs(left, all_stores, acc);
            collect_store_refs(right, all_stores, acc);
        }
        ExprIR::UnaryOpExpr { operand, .. } => collect_store_refs(operand, all_stores, acc),
        ExprIR::CallExpr { args, .. } | ExprIR::MethodCallExpr { args, .. } => {
            for arg in args {
                collect_store_refs(arg, all_stores, acc);
            }
        }
        ExprIR::IndexExpr { object, index } => {
            collect_store_refs(object, all_stores, acc);
            collect_store_refs(index, all_stores, acc);
        }
        ExprIR::TernaryIR {
            cond,
            then_expr,
            else_expr,
        } => {
            collect_store_refs(cond, all_stores, acc);
            collect_store_refs(then_expr, all_stores, acc);
            collect_store_refs(else_expr, all_stores, acc);
        }
        ExprIR::ArrayExpr { elements } => {
            for elem in elements {
                collect_store_refs(elem, all_stores, acc);
            }
        }
        ExprIR::MapExpr { entries } => {
            for (k, v) in entries {
                collect_store_refs(k, all_stores, acc);
                collect_store_refs(v, all_stores, acc);
            }
        }
        ExprIR::ClosureExpr { body, .. } => collect_store_refs(body, all_stores, acc),
        ExprIR::BlockExpr { statements, tail } => {
            for stmt in statements {
                collect_stmt_store_refs(stmt, all_stores, acc);
            }
            collect_store_refs(tail, all_stores, acc);
        }
        ExprIR::MatchExpr { matched, arms } => {
            collect_store_refs(matched, all_stores, acc);
            for arm in arms {
                for stmt in &arm.body.statements {
                    collect_stmt_store_refs(stmt, all_stores, acc);
                }
            }
        }
        ExprIR::VariantLiteral { fields, .. } | ExprIR::StructExpr { fields, .. } => {
            for (_, v) in fields {
                collect_store_refs(v, all_stores, acc);
            }
        }
        ExprIR::SpreadExpr { operand } => collect_store_refs(operand, all_stores, acc),
        ExprIR::AwaitExpr { operand } => collect_store_refs(operand, all_stores, acc),
        ExprIR::ArrayMethod {
            subject, predicate, ..
        } => {
            collect_store_refs(subject, all_stores, acc);
            collect_store_refs(predicate, all_stores, acc);
        }
        ExprIR::RefOpExpr { op, .. } => {
            collect_ref_op_store_refs(op, all_stores, acc);
        }
        ExprIR::IdentifierExpr { .. } | ExprIR::LiteralExpr { .. } => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown ExprIR variant — update codegen for new variant"),
    }
}

/// Collect store references from a `RefOpIR`'s child expressions.
fn collect_ref_op_store_refs(
    op: &crate::ir::RefOpIR,
    all_stores: &FxHashMap<&str, &crate::ir::StoreIR>,
    acc: &mut FxHashSet<String>,
) {
    match op {
        crate::ir::RefOpIR::Focus
        | crate::ir::RefOpIR::Blur
        | crate::ir::RefOpIR::Measure { .. }
        | crate::ir::RefOpIR::Select
        | crate::ir::RefOpIR::Value => {}
        crate::ir::RefOpIR::ScrollTo { x, y } => {
            if let Some(x) = x {
                collect_store_refs(x, all_stores, acc);
            }
            if let Some(y) = y {
                collect_store_refs(y, all_stores, acc);
            }
        }
        crate::ir::RefOpIR::ScrollBy { dx, dy } => {
            if let Some(dx) = dx {
                collect_store_refs(dx, all_stores, acc);
            }
            if let Some(dy) = dy {
                collect_store_refs(dy, all_stores, acc);
            }
        }
        crate::ir::RefOpIR::SetSelection { start, end } => {
            collect_store_refs(start, all_stores, acc);
            collect_store_refs(end, all_stores, acc);
        }
        crate::ir::RefOpIR::CapturePointer | crate::ir::RefOpIR::ReleasePointer => {}
        // RefOpIR is #[non_exhaustive] — cross-crate matches need a wildcard.
        #[allow(unreachable_patterns)]
        _ => {}
    }
}

/// Collect store references from a `StatementIR` (used inside block/closure
/// expressions that may appear in dynamic token values).
fn collect_stmt_store_refs(
    stmt: &crate::ir::StatementIR,
    all_stores: &FxHashMap<&str, &crate::ir::StoreIR>,
    acc: &mut FxHashSet<String>,
) {
    match stmt {
        crate::ir::StatementIR::ExprStmtIR { expr } => collect_store_refs(expr, all_stores, acc),
        crate::ir::StatementIR::LetStmtIR { value, .. } => {
            if let Some(v) = value {
                collect_store_refs(v, all_stores, acc);
            }
        }
        crate::ir::StatementIR::ReturnStmtIR { value } => {
            if let Some(v) = value {
                collect_store_refs(v, all_stores, acc);
            }
        }
        crate::ir::StatementIR::IfStmtIR {
            condition,
            then_body,
            else_ifs,
            else_body,
        } => {
            collect_store_refs(condition, all_stores, acc);
            for s in &then_body.statements {
                collect_stmt_store_refs(s, all_stores, acc);
            }
            for (c, b) in else_ifs {
                collect_store_refs(c, all_stores, acc);
                for s in &b.statements {
                    collect_stmt_store_refs(s, all_stores, acc);
                }
            }
            if let Some(eb) = else_body {
                for s in &eb.statements {
                    collect_stmt_store_refs(s, all_stores, acc);
                }
            }
        }
        crate::ir::StatementIR::WhileStmtIR { condition, body } => {
            collect_store_refs(condition, all_stores, acc);
            for s in &body.statements {
                collect_stmt_store_refs(s, all_stores, acc);
            }
        }
        crate::ir::StatementIR::ForInStmtIR { iterable, body, .. } => {
            collect_store_refs(iterable, all_stores, acc);
            for s in &body.statements {
                collect_stmt_store_refs(s, all_stores, acc);
            }
        }
        crate::ir::StatementIR::BlockStmtIR { body } => {
            for s in &body.statements {
                collect_stmt_store_refs(s, all_stores, acc);
            }
        }
        crate::ir::StatementIR::MutationStmt { intent } => {
            if let Some(intent) = intent {
                collect_mutation_store_refs(intent, all_stores, acc);
            }
        }
        crate::ir::StatementIR::ThrowStmtIR { expr } => collect_store_refs(expr, all_stores, acc),
        crate::ir::StatementIR::TryCatchStmtIR {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            for s in &try_body.statements {
                collect_stmt_store_refs(s, all_stores, acc);
            }
            if let Some(cb) = catch_body {
                for s in &cb.statements {
                    collect_stmt_store_refs(s, all_stores, acc);
                }
            }
            if let Some(fb) = finally_body {
                for s in &fb.statements {
                    collect_stmt_store_refs(s, all_stores, acc);
                }
            }
        }
        crate::ir::StatementIR::EventEmitStmtIR { args, .. } => {
            for arg in args {
                collect_store_refs(arg, all_stores, acc);
            }
        }
        crate::ir::StatementIR::BreakStmtIR | crate::ir::StatementIR::ContinueStmtIR => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown StatementIR variant — update codegen for new variant"),
    }
}

/// Collect store references from a `MutationIntent`.
fn collect_mutation_store_refs(
    intent: &crate::ir::MutationIntent,
    all_stores: &FxHashMap<&str, &crate::ir::StoreIR>,
    acc: &mut FxHashSet<String>,
) {
    match intent {
        crate::ir::MutationIntent::DirectAssign { value, .. } => {
            if let Some(v) = value {
                collect_store_refs(v, all_stores, acc);
            }
        }
        crate::ir::MutationIntent::Increment { amount, .. }
        | crate::ir::MutationIntent::Decrement { amount, .. }
        | crate::ir::MutationIntent::Multiply { amount, .. }
        | crate::ir::MutationIntent::Divide { amount, .. }
        | crate::ir::MutationIntent::Modulo { amount, .. } => {
            if let Some(v) = amount {
                collect_store_refs(v, all_stores, acc);
            }
        }
        crate::ir::MutationIntent::Push { item, .. } => {
            if let Some(v) = item {
                collect_store_refs(v, all_stores, acc);
            }
        }
        crate::ir::MutationIntent::Pop { .. } => {}
        #[allow(unreachable_patterns)]
        _ => unreachable!("Unknown MutationIntent variant — update codegen for new variant"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::StoreIR;
    use crate::ir::TokenIR;

    /// Build a `DesignIR` with static + dynamic tokens, a class, a variant,
    /// and two themes.
    fn make_design() -> DesignIR {
        let mut tokens = FxHashMap::default();
        tokens.insert(
            "surface".to_string(),
            TokenIR {
                value: TokenValueIR::Static("#ffffff".to_string()),
                token_type: crate::ir::design::TokenTypeIR::Color,
                theme_values: FxHashMap::default(),
                source_span: None,
            },
        );
        tokens.insert(
            "primary".to_string(),
            TokenIR {
                value: TokenValueIR::Static("#6366f1".to_string()),
                token_type: crate::ir::design::TokenTypeIR::Color,
                theme_values: vec![(
                    "dark".to_string(),
                    TokenValueIR::Static("#a5b4fc".to_string()),
                )]
                .into_iter()
                .collect(),
                source_span: None,
            },
        );
        tokens.insert(
            "dynColor".to_string(),
            TokenIR {
                value: TokenValueIR::Dynamic(ExprIR::StoreFieldAccessExpr {
                    store_alias: "DynamicTokenStore".to_string(),
                    field_name: "liveColor".to_string(),
                    call_args: vec![],
                }),
                token_type: crate::ir::design::TokenTypeIR::Color,
                theme_values: FxHashMap::default(),
                source_span: None,
            },
        );
        DesignIR {
            name: "AppDesign".to_string(),
            themes: vec!["light".to_string(), "dark".to_string()],
            default_theme: Some("light".to_string()),
            tokens,
            classes: vec![crate::ir::design::ClassIR {
                name: "Card".to_string(),
                properties: vec![("background".to_string(), "#ffffff".to_string())]
                    .into_iter()
                    .collect(),
                pseudo_states: vec![],
                source_span: None,
            }],
            variants: vec![],
            source_file: Some("src/design.omni".to_string()),
            source_span: None,
        }
    }

    /// Build an IR program containing the design plus the referenced store.
    fn make_ir(design: DesignIR) -> IRProgram {
        let store = StoreIR {
            name: "DynamicTokenStore".to_string(),
            signals: vec![(
                "liveColor".to_string(),
                crate::ir::TypeIR {
                    kind: "String".into(),
                    type_args: vec![],
                    is_nullable: false,
                },
            )]
            .into_iter()
            .collect(),
            signal_defaults: vec![("liveColor".to_string(), serde_json::json!("#6366f1"))]
                .into_iter()
                .collect(),
            getters: FxHashMap::default(),
            actions: vec![],
            source_file: Some("src/stores/DynamicTokenStore.omni".to_string()),
            ..Default::default()
        };
        IRProgram {
            version: "0.1".to_string(),
            modules: vec![(
                "main".to_string(),
                crate::ir::ModuleIR {
                    name: "main".to_string(),
                    stores: vec![("DynamicTokenStore".into(), store)]
                        .into_iter()
                        .collect(),
                    designs: vec![design],
                    ..Default::default()
                },
            )]
            .into_iter()
            .collect(),
            source_map: FxHashMap::default(),
            stdlib_imports: FxHashSet::default(),
            source_span: None,
            motion_index: FxHashMap::default(),
        }
    }

    #[test]
    fn emits_design_css_with_theme_overrides() {
        let ir = make_ir(make_design());
        let files = gen_design_files(&ir);
        let css = files
            .iter()
            .find(|f| f.path == "design.css")
            .expect("design.css should be emitted");
        assert!(css.content.contains("--surface: #ffffff;"));
        assert!(css.content.contains("--primary: #6366f1;"));
        assert!(css.content.contains("[data-theme=\"dark\"] {"));
        assert!(css.content.contains("--primary: #a5b4fc;"));
        assert!(css.content.contains("\n.Card {"));
        assert!(
            !css.content.contains("[data-design~="),
            "React design CSS must be unscoped (plain HTML)"
        );
        assert!(
            !css.content.contains("--dynColor"),
            "dynamic tokens must not appear in CSS"
        );
    }

    #[test]
    fn emits_use_design_tokens_hook() {
        let ir = make_ir(make_design());
        let files = gen_design_files(&ir);
        let hook = files
            .iter()
            .find(|f| f.path == "useDesignTokens.ts")
            .expect("useDesignTokens.ts should be emitted");
        let content = &hook.content;
        assert!(content.contains("import { useEffect } from 'react';"));
        assert!(
            content
                .contains("import { useDynamicTokenStore } from './stores/useDynamicTokenStore';"),
            "store hook import should use the relative path from the design file. Got:\n{}",
            content
        );
        assert!(content.contains("const DynamicTokenStore = useDynamicTokenStore();"));
        assert!(content.contains(
            "document.documentElement.style.setProperty('--dynColor', sanitizeCss(DynamicTokenStore.liveColor));"
        ));
        assert!(content.contains("}, [DynamicTokenStore.liveColor]);"));
        assert!(
            content.contains("function sanitizeCss(value: unknown): string {"),
            "sanitizer must be emitted"
        );
        // The React helper must come from the SHARED sanitizer emitter — the
        // full keyword block + hex-escape normalization + url() context
        // checks (the copies drifted before task 3.2).
        assert!(
            content.contains("function normalizeCssEscapes(value: string): string {"),
            "shared helper must emit the normalize function. Got:\n{}",
            content
        );
        assert!(
            content.contains("str = normalizeCssEscapes(str);"),
            "sanitizeCss must normalize hex escapes. Got:\n{}",
            content
        );
        for needle in ["data:text/html", "document.", "window.", "afterUrl"] {
            assert!(
                content.contains(needle),
                "React sanitizer must contain {needle:?} (drift fix). Got:\n{}",
                content
            );
        }
    }

    #[test]
    fn emits_theme_store() {
        let ir = make_ir(make_design());
        let files = gen_design_files(&ir);
        let store = files
            .iter()
            .find(|f| f.path == "useThemeStore.ts")
            .expect("useThemeStore.ts should be emitted");
        let content = &store.content;
        assert!(content.contains("import { create } from 'zustand';"));
        assert!(content.contains("theme: 'light',"));
        assert!(content.contains("setTheme: (theme) => set({ theme }),"));
        assert!(
            content.contains("document.documentElement.setAttribute('data-theme', state.theme);")
        );
        assert!(content.contains("useThemeStore.getState().theme"));
    }

    #[test]
    fn no_designs_emits_no_files() {
        let ir = IRProgram {
            version: "0.1".to_string(),
            modules: FxHashMap::default(),
            source_map: FxHashMap::default(),
            stdlib_imports: FxHashSet::default(),
            source_span: None,
            motion_index: FxHashMap::default(),
        };
        assert!(gen_design_files(&ir).is_empty());
    }

    #[test]
    fn design_without_themes_skips_theme_store() {
        let mut design = make_design();
        design.themes = vec![];
        design.default_theme = None;
        let ir = make_ir(design);
        let files = gen_design_files(&ir);
        assert!(
            files.iter().any(|f| f.path == "design.css"),
            "design.css should still be emitted"
        );
        assert!(
            !files.iter().any(|f| f.path == "useThemeStore.ts"),
            "no theme store without @themes"
        );
    }

    #[test]
    fn design_without_dynamic_tokens_skips_hook() {
        let mut design = make_design();
        design.tokens.remove("dynColor");
        let ir = make_ir(design);
        let files = gen_design_files(&ir);
        assert!(
            !files.iter().any(|f| f.path == "useDesignTokens.ts"),
            "no hook without dynamic tokens"
        );
    }

    #[test]
    fn design_files_land_in_source_directory() {
        let mut design = make_design();
        design.source_file = Some("src/components/design.omni".to_string());
        let ir = make_ir(design);
        let files = gen_design_files(&ir);
        assert!(files.iter().any(|f| f.path == "components/design.css"));
        assert!(files
            .iter()
            .any(|f| f.path == "components/useDesignTokens.ts"));
        assert!(files
            .iter()
            .any(|f| f.path == "components/useThemeStore.ts"));
    }
}
