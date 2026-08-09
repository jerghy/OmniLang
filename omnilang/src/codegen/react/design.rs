//! Design codegen for the React backend.
//!
//! React renders plain HTML — there is no `[data-design~="..."]` wrapper
//! element, so the shared design CSS emitter runs with an empty scope
//! prefix.  Each source file that declares `design` blocks produces up to
//! three outputs (all placed in a dedicated `__design/` subdirectory of the
//! output directory mirroring the design's source file — Task 4.4 naming
//! isolation, so the runtime files can never collide with a user store's
//! hook file):
//!
//! - `__design/design.css` — CSS custom properties (`:root` + `[data-theme]`
//!   overrides), class rules, and variant combination classes via
//!   `crate::codegen::shared::design::emit_design_css`
//! - `__design/useDesignTokens.ts` — a `useDesignTokens()` hook that applies
//!   `TokenValueIR::Dynamic` token values to
//!   `document.documentElement.style` via `useEffect` (one effect per
//!   dynamic token, deps = the per-field slice local — Task 2.7)
//! - `__design/useThemeStore.ts` — the theme runtime: when a store signal is
//!   marked `@theme` (v2 D1), a hook that subscribes to the USER's store and
//!   applies `data-theme` to `<html>` (CSS scoping); without `@theme`, the
//!   legacy standalone Zustand store (see `emit_theme_store`)

use crate::codegen::json::escape_literal_str;
use crate::codegen::react::component::store_slices;
use crate::codegen::react::{relative_import_path, source_dir_prefix};
use crate::codegen::shared::design as shared_design;
use crate::codegen::CodeWriter;
use crate::ir::design::{DesignIR, TokenValueIR};
use crate::ir::{ComponentIR, ExprIR, IRProgram};
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

/// One set of design outputs (`__design/design.css`,
/// `__design/useDesignTokens.ts`, `__design/useThemeStore.ts`) — the designs
/// declared in one source file (or the output root for designs without a
/// known source file).
///
/// All three files land in the group's dedicated `__design/` subdirectory
/// (Task 4.4 naming isolation): the runtime basenames (`useThemeStore.ts`,
/// `useDesignTokens.ts`) can never collide with a user store's hook file
/// (e.g. `stores/useThemeStore.ts`), so the theme runtime is emitted
/// unconditionally — no collision guard needed (the T4.2 guard was removed
/// with the directory isolation, per YAGNI).
pub(super) struct DesignGroup {
    /// Output directory prefix (`""` for the output root, `"components/"`…).
    pub dir_prefix: String,
    /// Whether any design in the group declares dynamic tokens.
    pub has_dynamic_tokens: bool,
    /// Whether any design in the group declares `@themes`.
    pub has_themes: bool,
    /// The program's `@theme`-marked store signal (`"StoreName.signal"`,
    /// DesignIR.theme_signal) — when present the theme runtime subscribes
    /// to the USER's store instead of the legacy standalone store (Task 4.2,
    /// design D1).
    pub theme_signal: Option<String>,
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
                theme_signal: designs.iter().find_map(|d| d.theme_signal.clone()),
                dir_prefix,
            }
        })
        .collect()
}

/// A synthetic source path whose output directory is the design group's
/// `__design/` runtime subdirectory — the anchor for relative imports
/// emitted INTO the runtime files. `relative_import_path` derives the
/// `__design/` directory from it, so store-hook imports resolve as seen from
/// inside the emitted runtime module (e.g. `src/__design/design.omni` for a
/// root-level design → `../stores/useThemeStore`).
fn design_runtime_source(dir_prefix: &str) -> Option<String> {
    Some(format!("src/{}__design/design.omni", dir_prefix))
}

/// Group the program's design blocks by source file and emit all design
/// outputs (`__design/design.css`, `__design/useDesignTokens.ts`,
/// `__design/useThemeStore.ts`) as `CompiledFile`s.  Returns an empty vector
/// when the program has no designs.
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
        let dir_prefix = &group.dir_prefix;

        files.push(crate::codegen::CompiledFile {
            path: format!("{}__design/design.css", dir_prefix),
            content: emit_design_css_file(&designs),
        });
        if group.has_dynamic_tokens {
            if let Some(content) = emit_use_design_tokens(&designs, dir_prefix, ir) {
                files.push(crate::codegen::CompiledFile {
                    path: format!("{}__design/useDesignTokens.ts", dir_prefix),
                    content,
                });
            }
        }
        if group.has_themes {
            if let Some(content) = emit_theme_store(&designs, dir_prefix, ir) {
                files.push(crate::codegen::CompiledFile {
                    path: format!("{}__design/useThemeStore.ts", dir_prefix),
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
    if all_names.contains(&"App") {
        return name == "App";
    }
    if all_names.contains(&"Main") {
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
/// - A named import + call of every `useThemeStore` hook when the program
///   wires a `@theme` signal (the useEffect subscription lives in a
///   component), or a side-effect import of the legacy store otherwise
///   (its module-level `data-theme` subscription runs on import).
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
    let mut theme_hook_idx = 0usize;
    for group in &groups {
        // The runtime files live in the group's dedicated `__design/`
        // subdirectory (Task 4.4 naming isolation) — the import target is
        // the synthetic source path whose output dir is that subdirectory.
        let to_source = format!("{}__design/design.omni", group.dir_prefix);
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
            if group.theme_signal.is_some() {
                // v2 D1: the theme runtime is a real hook (useEffect inside
                // a component) — import it NAMED and call it, like
                // useDesignTokens. Aliased so it cannot collide with the
                // @theme store's own hook (`use{StoreName}`), which the root
                // component may import for its own store refs.
                let local_name = if theme_hook_idx == 0 {
                    "useThemeStoreDesign".to_string()
                } else {
                    format!("useThemeStoreDesign{}", theme_hook_idx + 1)
                };
                import_lines.push(format!(
                    "import {{ useThemeStore as {} }} from '{}';",
                    local_name, import_path
                ));
                hook_calls.push(local_name);
                theme_hook_idx += 1;
            } else {
                // Legacy shape: module-level subscription runs on import.
                import_lines.push(format!("import '{}';", import_path));
            }
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
///   document.documentElement.style.setProperty('--dynColor', sanitizeCss(liveColor));
/// }, [liveColor]);
/// ```
/// The effect deps are the exact store-signal slice local, so unrelated
/// store updates do not re-fire it.  Stores are subscribed PER FIELD
/// (`const liveColor = useCartStore((s) => s.liveColor);` — Task 2.7,
/// perf M2), mirroring the component slice plan: a store write only
/// re-renders the root when the CHANGED field feeds a dynamic token.
fn emit_use_design_tokens(
    designs: &[&DesignIR],
    dir_prefix: &str,
    ir: &IRProgram,
) -> Option<String> {
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

    // The hook is emitted into the group's `__design/` directory — imports
    // resolve from there (Task 4.4 naming isolation).
    let source_file = design_runtime_source(dir_prefix);

    let mut w = CodeWriter::new();
    w.emit("// Generated by OmniLang — dynamic design token hook");
    w.emit("import { useEffect } from 'react';");
    for store_name in &known_stores {
        let hook_name = format!("use{}", store_name);
        let store = all_stores.get(store_name.as_str()).copied();
        let import_path = relative_import_path(
            source_file.as_deref(),
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
    // Per-field slice subscriptions (Task 2.7, perf M2): subscribe to each
    // referenced store field — `const liveColor = useDynamicTokenStore((s) => s.liveColor);` —
    // so a store write only re-renders the root when the CHANGED field feeds
    // a dynamic token. Previously the whole-store object (`const
    // DynamicTokenStore = useDynamicTokenStore();`) re-rendered App on every
    // store write (ts/react M2 + perf M2). Mirrors the component slice plan
    // (component/store_slices.rs); a store falls back to the whole-store
    // object when a referenced field is unknown, collides with another
    // store's slice local, or captures a name used by this module.
    let mut token_field_refs: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
    let mut bare_refs: FxHashSet<String> = FxHashSet::default();
    for (_, expr) in &dynamic_tokens {
        store_slices::collect_expr_store_field_refs(expr, &mut token_field_refs, &mut bare_refs);
    }
    let hook_shadowing: FxHashSet<String> = [
        "sanitizeCss",
        "normalizeCssEscapes",
        "useEffect",
        "document",
        "window",
        "String",
        "isNaN",
        "parseInt",
    ]
    .into_iter()
    .map(str::to_string)
    .collect();
    let (renames, slice_fields) = store_slices::build_slice_plan(
        &ComponentIR::default(),
        &token_field_refs,
        &hook_shadowing,
        &bare_refs,
        &all_stores,
    );
    for store_name in &known_stores {
        if let Some(fields) = slice_fields.get(store_name.as_str()) {
            if let Some(store) = all_stores.get(store_name.as_str()) {
                store_slices::emit_store_slices(&mut w, store_name, store, fields);
                continue;
            }
        }
        w.emit_fmt(format_args!("const {} = use{}();", store_name, store_name));
    }
    for (name, expr) in &dynamic_tokens {
        let js_expr =
            super::expr::format_expr_ir_tsx_renamed(expr, &FxHashSet::default(), &renames);
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
pub(crate) fn emit_sanitize_css(w: &mut CodeWriter) {
    w.emit("// Strip dangerous CSS sequences from runtime values (shared sanitizer");
    w.emit("// emitter — identical logic to the OmniView runtime __omni_sanitizeCss)");
    w.emit("// so store-driven token values cannot inject CSS.");
    let helpers = crate::codegen::shared::sanitize::sanitize_css_helpers_js(
        "sanitizeCss",
        "normalizeCssEscapes",
        true,
    );
    // The shared emitter serves BOTH runtimes from one body: the OmniView
    // runtime enforces ES5 (`var`), while the React .ts output gets
    // `let`/`const` from the same emitter (no-var, task 2.11 — `const` for
    // bindings never reassigned, prefer-const clean, task 5.1).
    for line in helpers.lines() {
        w.emit(line);
    }
}

/// Emit `useThemeStore.ts` — the theme runtime for the design group.
///
/// Emitted when any design in the group declares `@themes`. Two shapes
/// (Task 4.2, design D1):
///
/// 1. **Wired** (`DesignIR.theme_signal` present): a `useThemeStore()` hook
///    that subscribes to the USER's store (the store owning the `@theme`
///    signal) with a T3.3 slice selector and writes `data-theme` to `<html>`
///    in a `useEffect` deps on the slice — clicking the theme switcher (an
///    ordinary store action) changes the attribute and the
///    `[data-theme="..."]` CSS overrides apply. The root component imports
///    the hook (aliased) and calls it.
/// 2. **Legacy** (no `@theme` signal): the pre-v2 standalone Zustand store
///    (`theme`/`setTheme` + module-level `subscribe`). Kept for backward
///    compatibility.
///
/// The file lands in the group's `__design/` directory (Task 4.4 naming
/// isolation) — since no user file can be named `__design/useThemeStore.ts`,
/// the runtime is emitted even when the `@theme` store hook has the same
/// basename elsewhere.
///
/// Returns `None` when no design declares themes.
fn emit_theme_store(designs: &[&DesignIR], dir_prefix: &str, ir: &IRProgram) -> Option<String> {
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

    if let Some(signal_path) = designs.iter().find_map(|d| d.theme_signal.clone()) {
        return emit_theme_subscription(&signal_path, dir_prefix, ir);
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

/// Emit the WIRED theme runtime: a hook that subscribes to the `@theme`
/// signal's store with a slice selector (T3.3 machinery) and writes
/// `data-theme` in a `useEffect` deps on the slice.
///
/// Returns `None` when the signal path is malformed or the store/signal is
/// unknown (defensive — validate_theme guarantees the pairing).
fn emit_theme_subscription(signal_path: &str, dir_prefix: &str, ir: &IRProgram) -> Option<String> {
    let (store_name, signal_name) = signal_path.split_once('.')?;
    let all_stores = ir.all_stores();
    let store = all_stores.get(store_name)?;
    if !store.signals.contains_key(signal_name) {
        return None;
    }
    // The runtime is emitted into the group's `__design/` directory — the
    // user store hook import resolves from there (Task 4.4 naming isolation).
    let source_file = design_runtime_source(dir_prefix);
    let hook_name = format!("use{}", store_name);
    let import_path = relative_import_path(
        source_file.as_deref(),
        store.source_file.as_deref(),
        &hook_name,
    );

    let mut w = CodeWriter::new();
    w.emit("// Generated by OmniLang — theme data-theme subscription");
    w.emit("// (wired to the @theme-marked store signal — switching the store's");
    w.emit("// theme signal changes <html data-theme=\"...\"> and the CSS overrides)");
    w.emit("import { useEffect } from 'react';");
    w.emit_fmt(format_args!(
        "import {{ {} as useAppThemeStore }} from '{}';",
        hook_name, import_path
    ));
    w.emit("");
    w.emit("export function useThemeStore() {");
    w.indent();
    w.emit_fmt(format_args!(
        "const {} = useAppThemeStore((s) => s.{});",
        signal_name, signal_name
    ));
    w.emit("useEffect(() => {");
    w.indent();
    w.emit_fmt(format_args!(
        "document.documentElement.setAttribute('data-theme', {});",
        signal_name
    ));
    w.dedent();
    w.emit_fmt(format_args!("}}, [{}]);", signal_name));
    w.dedent();
    w.emit("}");
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
        ExprIR::BuiltinToastCall { msg, .. } => collect_store_refs(msg, all_stores, acc),
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
            theme_signal: None,
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
            .find(|f| f.path == "__design/design.css")
            .expect("__design/design.css should be emitted");
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
            .find(|f| f.path == "__design/useDesignTokens.ts")
            .expect("__design/useDesignTokens.ts should be emitted");
        let content = &hook.content;
        assert!(content.contains("import { useEffect } from 'react';"));
        assert!(
            content
                .contains("import { useDynamicTokenStore } from '../stores/useDynamicTokenStore';"),
            "store hook import must resolve from inside __design/ (Task 4.4). Got:\n{}",
            content
        );
        // Task 2.7 (ts/react M2 + perf M2): the hook must subscribe PER FIELD
        // — the whole-store object re-rendered the root App on every store
        // write, and the per-field slice local also keeps the effect deps
        // precise.
        assert!(content.contains("const liveColor = useDynamicTokenStore((s) => s.liveColor);"));
        assert!(
            !content.contains("const DynamicTokenStore = useDynamicTokenStore();"),
            "whole-store subscription must be gone. Got:\n{}",
            content
        );
        assert!(content.contains(
            "document.documentElement.style.setProperty('--dynColor', sanitizeCss(liveColor));"
        ));
        assert!(content.contains("}, [liveColor]);"));
        assert!(
            content.contains("function sanitizeCss(value: unknown): string {"),
            "sanitizer must be emitted"
        );
        // Task 2.11 (no-var): the shared sanitizer emitter produces one ES5
        // body serving BOTH runtimes — the React .ts copy must be emitted
        // with `let` (the shared source stays ES5 for the OmniView runtime).
        assert!(
            !content.contains("var "),
            "the React copy must not contain ES5 var declarations (no-var). Got:\n{}",
            content
        );
        assert!(
            content.contains("let result = '';") && content.contains("let str = String(value);"),
            "the React copy must emit let declarations. Got:\n{}",
            content
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
            .find(|f| f.path == "__design/useThemeStore.ts")
            .expect("__design/useThemeStore.ts should be emitted");
        let content = &store.content;
        assert!(content.contains("import { create } from 'zustand';"));
        assert!(content.contains("theme: 'light',"));
        assert!(content.contains("setTheme: (theme) => set({ theme }),"));
        assert!(
            content.contains("document.documentElement.setAttribute('data-theme', state.theme);")
        );
        assert!(content.contains("useThemeStore.getState().theme"));
    }

    /// Build an IR program containing the design plus a `ThemeStore` whose
    /// `mode` signal is marked `@theme` (the v2 D1 wiring shape).
    fn make_theme_ir(design: DesignIR) -> IRProgram {
        let theme_store = StoreIR {
            name: "ThemeStore".to_string(),
            signals: vec![(
                "mode".to_string(),
                crate::ir::TypeIR {
                    kind: "String".into(),
                    type_args: vec![],
                    is_nullable: false,
                },
            )]
            .into_iter()
            .collect(),
            signal_defaults: vec![("mode".to_string(), serde_json::json!("light"))]
                .into_iter()
                .collect(),
            getters: FxHashMap::default(),
            actions: vec![],
            source_file: Some("src/stores/ThemeStore.omni".to_string()),
            ..Default::default()
        };
        let mut ir = make_ir(design);
        ir.modules
            .get_mut("main")
            .expect("main module")
            .stores
            .insert("ThemeStore".into(), theme_store);
        ir
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

    /// Task 4.2 (review HIGH-2): when a store signal is marked `@theme`
    /// (DesignIR.theme_signal), the emitted theme runtime subscribes to the
    /// USER's store with a slice selector (T3.3 machinery) and writes
    /// `data-theme` in a useEffect — no more orphan Zustand store.
    #[test]
    fn emits_theme_subscription_when_signal_wired() {
        let mut design = make_design();
        design.theme_signal = Some("ThemeStore.mode".to_string());
        let ir = make_theme_ir(design);
        let files = gen_design_files(&ir);
        let store = files
            .iter()
            .find(|f| f.path == "__design/useThemeStore.ts")
            .expect("__design/useThemeStore.ts should be emitted");
        let content = &store.content;
        assert!(content.contains("import { useEffect } from 'react';"));
        assert!(
            content.contains(
                "import { useThemeStore as useAppThemeStore } from '../stores/useThemeStore';"
            ),
            "must import the user's theme store hook from inside __design/ (Task 4.4). Got:\n{}",
            content
        );
        assert!(
            content.contains("const mode = useAppThemeStore((s) => s.mode);"),
            "must subscribe with a slice selector. Got:\n{}",
            content
        );
        assert!(
            content.contains("document.documentElement.setAttribute('data-theme', mode);"),
            "must write data-theme from the slice. Got:\n{}",
            content
        );
        assert!(
            content.contains("}, [mode]);"),
            "useEffect deps must be the theme slice. Got:\n{}",
            content
        );
        assert!(
            !content.contains("create"),
            "the orphan Zustand store must NOT be emitted when a @theme signal is wired. \
             Got:\n{}",
            content
        );
    }

    /// Task 4.4 (naming isolation): the design group and the @theme store
    /// sharing a source directory was the old `useThemeStore.ts` collision —
    /// now the runtime lands in the group's `__design/` subdirectory, so
    /// BOTH the user's store hook (`useThemeStore.ts`) and the design
    /// runtime (`stores/__design/useThemeStore.ts`) are emitted and can
    /// never overwrite each other. No guard needed.
    #[test]
    fn theme_runtime_and_store_hook_coexist_in_same_directory() {
        let mut design = make_design();
        design.theme_signal = Some("ThemeStore.mode".to_string());
        design.source_file = Some("src/stores/ThemeStore.omni".to_string());
        let mut ir = make_theme_ir(design);
        // Move the store into the SAME source file as the design.
        ir.modules
            .get_mut("main")
            .expect("main module")
            .stores
            .get_mut("ThemeStore")
            .expect("ThemeStore")
            .source_file = Some("src/stores/ThemeStore.omni".to_string());
        let files = gen_design_files(&ir);
        let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
        assert!(
            paths.contains(&"stores/__design/useThemeStore.ts"),
            "the design runtime must be emitted in __design/. Got:\n{:#?}",
            paths
        );
        assert!(
            paths.contains(&"stores/__design/design.css"),
            "design.css must be emitted in __design/. Got:\n{:#?}",
            paths
        );
        assert!(
            !paths.contains(&"stores/useThemeStore.ts"),
            "the design runtime must NOT claim the user store hook's path. Got:\n{:#?}",
            paths
        );
    }

    /// Task 4.2: projects WITHOUT a @theme signal keep the previous theme
    /// store shape (backward compatible) — the standalone store is the
    /// fallback when no @theme signal exists.
    #[test]
    fn emits_legacy_theme_store_without_theme_signal() {
        let ir = make_theme_ir(make_design());
        let files = gen_design_files(&ir);
        let store = files
            .iter()
            .find(|f| f.path == "__design/useThemeStore.ts")
            .expect("__design/useThemeStore.ts should be emitted");
        let content = &store.content;
        assert!(content.contains("import { create } from 'zustand';"));
        assert!(content.contains("export const useThemeStore = create<ThemeState>()"));
        assert!(
            !content.contains("useEffect"),
            "no hook form without a @theme signal"
        );
    }

    #[test]
    fn design_without_themes_skips_theme_store() {
        let mut design = make_design();
        design.themes = vec![];
        design.default_theme = None;
        let ir = make_ir(design);
        let files = gen_design_files(&ir);
        assert!(
            files.iter().any(|f| f.path == "__design/design.css"),
            "__design/design.css should still be emitted"
        );
        assert!(
            !files.iter().any(|f| f.path == "__design/useThemeStore.ts"),
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
            !files
                .iter()
                .any(|f| f.path == "__design/useDesignTokens.ts"),
            "no hook without dynamic tokens"
        );
    }

    #[test]
    fn design_files_land_in_source_directory() {
        let mut design = make_design();
        design.source_file = Some("src/components/design.omni".to_string());
        let ir = make_ir(design);
        let files = gen_design_files(&ir);
        assert!(files
            .iter()
            .any(|f| f.path == "components/__design/design.css"));
        assert!(files
            .iter()
            .any(|f| f.path == "components/__design/useDesignTokens.ts"));
        assert!(files
            .iter()
            .any(|f| f.path == "components/__design/useThemeStore.ts"));
    }
}
