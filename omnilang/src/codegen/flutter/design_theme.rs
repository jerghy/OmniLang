//! Flutter design codegen: theme switching runtime and dynamic token
//! providers (domain half of the former `design.rs` monolith).
//!
//! - Step 4 — Theme switching: a Riverpod 3.0 `ThemeModeNotifier` +
//!   `NotifierProvider<ThemeMode>` plus one `ThemeData` builder per declared
//!   theme, and the generated `main.dart` app shell wiring
//!   `MaterialApp(themeMode:)` (`gen_theme_app_shell`). Task 3.6: designs
//!   declaring MORE than light/dark switch to a `themeDataProvider`
//!   (`Provider<ThemeData>` selecting the active builder across ALL declared
//!   themes) and warn about that platform floor.
//! - Step 5 — Dynamic tokens: a `Provider<Map<String, Object>>`
//!   (`dynamicTokensProvider`) mirroring the React `useDesignTokens()` hook,
//!   plus `applyDynamicTokens`, which overrides matched `ColorScheme` slots
//!   from the provider map.

use super::design::collect_store_refs;
use super::design_tokens::token_theme_const_ref;
use super::utils::lower_first;
use super::{capitalize_first, format_expr_ir_flutter, rewrite_store_accesses};
use crate::codegen::{CodeWriter, CompiledFile};
use crate::ir::design::*;
use crate::ir::{ExprIR, IRProgram};
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

// ═══════════════════════════════════════════════════════════════
// Step 4 — Theme switching: StateProvider<ThemeMode> + ThemeData builders
// ═══════════════════════════════════════════════════════════════

/// Exact token name → `ColorScheme` slot mapping used by the `ThemeData`
/// builders and `applyDynamicTokens`. Only unambiguous semantic names are
/// mapped; everything else stays out of the color scheme.
fn color_scheme_slot(name: &str) -> Option<&'static str> {
    match name {
        "primary" => Some("primary"),
        "primaryLight" => Some("primaryContainer"),
        "secondary" => Some("secondary"),
        "accent" => Some("tertiary"),
        "danger" => Some("error"),
        "surface" => Some("surface"),
        "surfaceMuted" => Some("surfaceContainerHighest"),
        "surfaceSubtle" => Some("surfaceContainerLow"),
        "textPrimary" => Some("onSurface"),
        "textSecondary" => Some("onSurfaceVariant"),
        "textMuted" => Some("outline"),
        "border" => Some("outlineVariant"),
        _ => None,
    }
}

fn theme_builder_name(design: &DesignIR, theme: &str) -> String {
    format!("build{}{}Theme", design.name, capitalize_first(theme))
}

/// Task 3.6 (I-1): the UNION of declared themes across ALL theme-declaring
/// designs (dedup, declaration order). The multi-theme decision, the
/// warning, and the selector's switch arms must cover EVERY design's
/// themes — a program mixing `design A { light, dark }` + `design B
/// { light, dark, sepia }` used to consult only A and silently collapse
/// sepia (exactly the "unreachable theme" class this task eliminates).
fn union_themes(designs: &[&DesignIR]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for design in designs {
        for theme in &design.themes {
            if !out.contains(theme) {
                out.push(theme.clone());
            }
        }
    }
    out
}

/// Task 3.6 (I-1): the builder the multi-theme selector falls back to when
/// the @theme signal holds an unknown mode — `light` when declared by ANY
/// design, else the first union-ordered theme, resolved against the design
/// that declares it. Panic-free (unwrap-audit discipline): the placeholder
/// arm is unreachable — the caller returns early when no design declares
/// `@themes`, so the union is non-empty and the declaring design exists.
fn fallback_builder(designs: &[&DesignIR], union: &[String]) -> String {
    let Some(fallback) = union
        .iter()
        .find(|t| *t == "light")
        .or_else(|| union.first())
    else {
        return "buildAppLightTheme".to_string();
    };
    let Some(design) = designs.iter().find(|d| d.themes.contains(fallback)) else {
        return "buildAppLightTheme".to_string();
    };
    theme_builder_name(design, fallback)
}

/// Task 3.6 (I-1): the builder the multi-theme selector holds when NO
/// @theme signal is wired — the FIRST theme-declaring design's
/// `@default-theme` builder when it names one of that design's declared
/// themes, else the union fallback (mirrors the notifier's
/// `ThemeMode.dark`/light mapping for the two-state path). Panic-free like
/// `fallback_builder`.
fn default_builder(first: &DesignIR, designs: &[&DesignIR], union: &[String]) -> String {
    if let Some(t) = first.default_theme.as_deref() {
        if first.themes.iter().any(|x| x == t) {
            return theme_builder_name(first, t);
        }
    }
    fallback_builder(designs, union)
}

/// Emit the theme switching runtime: a Riverpod 3.0 `ThemeModeNotifier` +
/// `NotifierProvider<ThemeMode>` plus one `ThemeData` builder per declared
/// theme. Emitted only when at least one design declares `@themes`.
///
/// v2 (design D1): when a store signal is marked `@theme`, the notifier
/// watches the USER's store (`ref.watch(<store>Provider.select(...))`) — an
/// ordinary store action switches the theme on Flutter too, and the
/// generated app shell (see `gen_theme_app_shell`) wires
/// `MaterialApp(themeMode: ref.watch(themeModeProvider))`. Flutter's
/// `ThemeMode` expresses light/dark only; non-dark themes map to
/// `ThemeMode.light` (honest platform floor). Without a `@theme` signal the
/// notifier holds the `@default-theme` mode and the app-shell wiring stays
/// manual (pre-v2 behavior).
///
/// Task 3.6 (>2 themes, D-G): Flutter's native `ThemeMode` is two-state, so
/// when the UNION of themes across ALL theme-declaring designs (I-1) has
/// more than light/dark, emission switches to a `themeDataProvider` — a
/// `Provider<ThemeData>` selecting the active `ThemeData` builder for the
/// @theme signal across the whole union, each arm resolved against the
/// design that declares its theme (every theme reachable; the app shell
/// wires `MaterialApp(theme: ref.watch(themeDataProvider))`). The extra
/// themes do not participate in `ThemeMode` semantics (no
/// system-following) — that platform floor is documented by a compiler
/// warning covering all declaring designs.
pub(crate) fn emit_theme_switch(
    w: &mut CodeWriter,
    designs: &[&DesignIR],
    ir: &IRProgram,
    warnings: &mut Vec<String>,
) {
    let Some(first) = designs.iter().find(|d| !d.themes.is_empty()) else {
        return;
    };
    // Task 3.6 (I-1): the multi-theme decision covers the UNION of themes
    // across ALL theme-declaring designs — a program mixing `design A
    // { light, dark }` + `design B { light, dark, sepia }` must warn and
    // reach sepia, not silently collapse to the two-state path on A.
    let union = union_themes(designs);
    let multi_theme = union.len() > 2;
    if multi_theme {
        let declaring: Vec<String> = designs
            .iter()
            .filter(|d| !d.themes.is_empty())
            .map(|d| format!("'{}'", d.name))
            .collect();
        warnings.push(format!(
            "[flutter][design] design(s) {} declare {} themes ({}): Flutter 原生 \
             ThemeMode 仅支持 light/dark 两态——其余主题通过 themeDataProvider \
             的 ThemeData 索引选择接线（不参与系统亮暗跟随）",
            declaring.join(", "),
            union.len(),
            union.join(", ")
        ));
    }
    let mut default_mode = match first.default_theme.as_deref() {
        Some("dark") => "ThemeMode.dark",
        _ => "ThemeMode.light",
    };
    // The @theme signal's own initial value is authoritative when wired.
    let mut store_watch: Option<(String, String, String)> = None;
    if let Some(path) = designs.iter().find_map(|d| d.theme_signal.as_deref()) {
        if let Some((store_name, signal_name)) = path.split_once('.') {
            if let Some(store) = ir.all_stores().get(store_name) {
                if store.signals.contains_key(signal_name) {
                    if store
                        .signal_defaults
                        .get(signal_name)
                        .and_then(|v| v.as_str())
                        == Some("dark")
                    {
                        default_mode = "ThemeMode.dark";
                    }
                    store_watch = Some((
                        store_name.to_string(),
                        format!("{}Provider", lower_first(store_name)),
                        signal_name.to_string(),
                    ));
                }
            }
        }
    }
    if multi_theme {
        // Task 3.6: >2 declared themes — the @theme signal selects the active
        // ThemeData builder directly (Flutter's ThemeMode cannot express
        // sepia/highContrast). `themeDataProvider` is what the app shell
        // watches; `themeModeProvider` is not emitted (no ThemeMode at all).
        w.emit("/// Theme switching (Task 3.6): >2 declared themes cannot be");
        w.emit("/// expressed by Flutter's native two-state `ThemeMode` — the");
        w.emit("/// @theme signal selects the active `ThemeData` builder");
        w.emit("/// directly so EVERY declared theme is reachable. The");
        w.emit("/// generated app shell (main.dart) wires");
        w.emit("/// `MaterialApp(theme: ref.watch(themeDataProvider))`.");
        w.emit("final themeDataProvider = Provider<ThemeData>((ref) {");
        w.indent();
        if let Some((store_name, provider_name, signal_name)) = &store_watch {
            w.emit_fmt(format_args!(
                "/// Watches `{provider_name}` (the `{store_name}` store) — an"
            ));
            w.emit("/// ordinary store action on the @theme signal switches the");
            w.emit("/// theme.");
            w.emit_fmt(format_args!(
                "final {} = ref.watch({}.select((s) => s.{}));",
                signal_name, provider_name, signal_name
            ));
            w.emit("return switch (mode) {");
            w.indent();
            // I-1: one arm per union theme, resolved against the design
            // that DECLARES it (`'sepia' => buildBSepiaTheme()` when only
            // design B declares sepia).
            for theme in &union {
                if let Some(design) = designs.iter().find(|d| d.themes.contains(theme)) {
                    w.emit_fmt(format_args!(
                        "'{}' => {}(),",
                        theme,
                        theme_builder_name(design, theme)
                    ));
                }
            }
            w.emit_fmt(format_args!(
                "_ => {}(),",
                fallback_builder(designs, &union)
            ));
            w.dedent();
            w.emit("};");
        } else {
            w.emit("/// No @theme signal — the provider holds the default");
            w.emit("/// theme's builder; wire `MaterialApp(theme:");
            w.emit("/// ref.watch(themeDataProvider))` manually (pre-v2");
            w.emit("/// behavior).");
            w.emit_fmt(format_args!(
                "return {}();",
                default_builder(first, designs, &union)
            ));
        }
        w.dedent();
        w.emit("});");
    } else {
        w.emit("/// Theme switching (v2 D1): the @theme-marked store signal is THE");
        w.emit("/// current theme mode. The generated app shell (main.dart) wires");
        w.emit("/// `MaterialApp(themeMode: ref.watch(themeModeProvider))`.");
        w.emit("final themeModeProvider = NotifierProvider<ThemeModeNotifier, ThemeMode>(ThemeModeNotifier.new);");
        w.emit("");
        w.emit("class ThemeModeNotifier extends Notifier<ThemeMode> {");
        w.indent();
        if let Some((store_name, provider_name, signal_name)) = &store_watch {
            w.emit_fmt(format_args!(
                "/// Watches `{provider_name}` (the `{store_name}` store) — an"
            ));
            w.emit("/// ordinary store action on the @theme signal switches the");
            w.emit("/// theme.");
            w.emit("@override");
            w.emit("ThemeMode build() {");
            w.indent();
            w.emit_fmt(format_args!(
                "final {} = ref.watch({}.select((s) => s.{}));",
                signal_name, provider_name, signal_name
            ));
            w.emit("return mode == 'dark' ? ThemeMode.dark : ThemeMode.light;");
            w.dedent();
            w.emit("}");
        } else {
            w.emit("/// No @theme signal — the notifier holds the default mode;");
            w.emit("/// wire `MaterialApp(themeMode: ref.watch(themeModeProvider))`");
            w.emit("/// manually (pre-v2 behavior).");
            w.emit("@override");
            w.emit_fmt(format_args!("ThemeMode build() => {};", default_mode));
        }
        w.dedent();
        w.emit("}");
    }
    w.emit("");
    for design in designs {
        for theme in &design.themes {
            emit_theme_data_builder(w, design, theme);
        }
    }
}

/// Emit `main.dart` — the app shell wiring `MaterialApp` to the theme
/// runtime (Task 4.2, design D1).
///
/// Emitted only when the program declares `@themes` AND a store signal is
/// marked `@theme` — the wiring target (`themeModeProvider`, the light/dark
/// `ThemeData` builders, the root component) is then fully determined by
/// the IR:
///
/// ```dart
/// void main() {
///   runApp(const ProviderScope(child: OmniApp()));
/// }
///
/// class OmniApp extends ConsumerWidget {
///   ...
///   Widget build(BuildContext context, WidgetRef ref) {
///     return MaterialApp(
///       themeMode: ref.watch(themeModeProvider),
///       theme: buildAppLightTheme(),
///       darkTheme: buildAppDarkTheme(),
///       home: const App(),
///     );
///   }
/// }
/// ```
///
/// Task 3.6 (I-1): when the UNION of declared themes across all designs
/// exceeds light/dark (>2 themes) the shell watches `themeDataProvider`
/// instead — `MaterialApp(theme: ref.watch(themeDataProvider))` — because
/// Flutter's `ThemeMode` cannot express sepia/highContrast. The provider
/// selects the active `ThemeData` builder across ALL declared themes. The
/// gate mirrors `emit_theme_switch`'s union decision so the shell and the
/// emitted runtime always agree.
///
/// Projects without a `@theme` signal keep their own main.dart (pre-v2
/// behavior — the notifier holds the default mode and the wiring stays
/// manual).
pub fn gen_theme_app_shell(ir: &IRProgram) -> Option<CompiledFile> {
    let first = ir
        .modules
        .values()
        .flat_map(|m| &m.designs)
        .find(|d| !d.themes.is_empty() && d.theme_signal.is_some())?;
    let theme_signal = first.theme_signal.as_deref()?;
    let (store_name, signal_name) = theme_signal.split_once('.')?;
    let all_stores = ir.all_stores();
    let store = all_stores.get(store_name)?;
    if !store.signals.contains_key(signal_name) {
        return None;
    }

    // Root component: App → Main → last sorted (mirrors the React backend).
    let all_components = ir.all_components();
    let mut all_comp_names: Vec<&str> = all_components.keys().copied().collect();
    let root_name = if all_comp_names.contains(&"App") {
        "App".to_string()
    } else if all_comp_names.contains(&"Main") {
        "Main".to_string()
    } else {
        all_comp_names.sort_unstable();
        all_comp_names.last().map(|n| n.to_string())?
    };
    let root = all_components.get(root_name.as_str())?;
    // Const-instance when the root takes no props/slots, so the generated
    // shell stays lint-clean (`prefer_const_constructors`).
    let home = if root.props.is_empty() && root.slots.is_empty() {
        format!("const {}()", root_name)
    } else {
        format!("{}()", root_name)
    };

    // Task 3.6 (I-1): the multi-theme gate covers the UNION of themes
    // across ALL designs — a second design can push the program past
    // light/dark even when the @theme-signal design stays two-state. The
    // shell MUST agree with the runtime emitted by `emit_theme_switch`
    // (which also gates on the union) or main.dart would reference a
    // provider that does not exist.
    let all_designs: Vec<&DesignIR> = ir.modules.values().flat_map(|m| &m.designs).collect();
    let union = union_themes(&all_designs);
    let multi_theme = union.len() > 2;
    // light/dark builders resolve against the design that DECLARES them
    // (light may live in design A and dark in design B).
    let light_theme = union
        .iter()
        .find(|t| *t == "light")
        .or_else(|| union.first())
        .cloned()?;
    let light_builder = theme_builder_name(
        all_designs
            .iter()
            .find(|d| d.themes.contains(&light_theme))?,
        &light_theme,
    );
    let dark_builder = union
        .iter()
        .find(|t| *t == "dark")
        .and_then(|t| all_designs.iter().find(|d| d.themes.contains(t)))
        .map(|d| theme_builder_name(d, "dark"));

    let mut w = CodeWriter::new();
    w.emit("// Generated by OmniLang \u{2014} app shell with @theme wiring");
    w.emit("");
    w.emit("import 'package:flutter/material.dart';");
    w.emit("import 'package:flutter_riverpod/flutter_riverpod.dart';");
    w.emit("");
    w.emit_fmt(format_args!("import '{}.dart';", root_name));
    w.emit("import 'design.dart';");
    w.emit("");
    w.emit("void main() {");
    w.indent();
    w.emit("runApp(const ProviderScope(child: OmniApp()));");
    w.dedent();
    w.emit("}");
    w.emit("");
    w.emit("class OmniApp extends ConsumerWidget {");
    w.indent();
    w.emit("const OmniApp({super.key});");
    w.emit("");
    w.emit("@override");
    w.emit("Widget build(BuildContext context, WidgetRef ref) {");
    w.indent();
    w.emit("return MaterialApp(");
    w.indent();
    if multi_theme {
        w.emit("theme: ref.watch(themeDataProvider),");
    } else {
        w.emit("themeMode: ref.watch(themeModeProvider),");
        w.emit_fmt(format_args!("theme: {}(),", light_builder));
        if let Some(dark_builder) = &dark_builder {
            w.emit_fmt(format_args!("darkTheme: {}(),", dark_builder));
        }
    }
    // Task 3.9: the root gets a Scaffold + SafeArea shell — status-bar /
    // display-notch insets are honored (SafeArea) and the root component
    // renders on a Material background (ink effects, Material text style).
    // The F1 component-root Material (input-bearing components) nests
    // inside without conflict — Flutter allows arbitrary Material nesting.
    w.emit("home: Scaffold(");
    w.indent();
    w.emit("body: SafeArea(");
    w.indent();
    w.emit_fmt(format_args!("child: {},", home));
    w.dedent();
    w.emit("),");
    w.dedent();
    w.emit("),");
    w.dedent();
    w.emit(");");
    w.dedent();
    w.emit("}");
    w.dedent();
    w.emit("}");
    Some(CompiledFile {
        path: "main.dart".to_string(),
        content: w.into_string(),
    })
}

/// One `ThemeData` builder for a declared theme. Color tokens map to
/// `ColorScheme` slots by exact token name; tokens with a value override for
/// this theme resolve to their per-theme constant (`AppTokens.primaryDark`),
/// others to the base constant. Dynamic tokens have no const — they are
/// applied at runtime via `applyDynamicTokens`.
fn emit_theme_data_builder(w: &mut CodeWriter, design: &DesignIR, theme: &str) {
    let builder = theme_builder_name(design, theme);
    let is_dark = theme == "dark";

    let mut tokens: Vec<(&String, &TokenIR)> = design.tokens.iter().collect();
    tokens.sort_by(|a, b| a.0.cmp(b.0));
    let mut slots: Vec<(&str, String)> = Vec::new();
    for (name, token) in &tokens {
        if token.token_type == TokenTypeIR::Color {
            if let Some(slot) = color_scheme_slot(name) {
                if let Some(const_ref) = token_theme_const_ref(design, token, name, theme) {
                    slots.push((slot, const_ref));
                }
            }
        }
    }
    let surface_ref = tokens
        .iter()
        .find(|(n, t)| n.as_str() == "surface" && t.token_type == TokenTypeIR::Color)
        .and_then(|(n, t)| token_theme_const_ref(design, t, n, theme));
    let font_family_ref = tokens
        .iter()
        .find(|(n, _)| n.as_str() == "fontFamily")
        .and_then(|(n, t)| token_theme_const_ref(design, t, n, theme));

    w.emit_fmt(format_args!(
        "/// `ThemeData` for the `{}` theme of `{}`.",
        theme, design.name
    ));
    w.emit_fmt(format_args!("ThemeData {}() {{", builder));
    w.indent();
    if slots.is_empty() && surface_ref.is_none() && font_family_ref.is_none() {
        w.emit_fmt(format_args!(
            "return ThemeData.{}();",
            if is_dark { "dark" } else { "light" }
        ));
        w.dedent();
        w.emit("}");
        w.emit("");
        return;
    }
    w.emit("return ThemeData(");
    w.indent();
    w.emit_fmt(format_args!(
        "colorScheme: {}(",
        if is_dark {
            "ColorScheme.dark"
        } else {
            "ColorScheme.light"
        }
    ));
    w.indent();
    for (slot, const_ref) in &slots {
        w.emit_fmt(format_args!("{}: {},", slot, const_ref));
    }
    w.dedent();
    w.emit("),");
    if let Some(surface) = surface_ref {
        w.emit_fmt(format_args!("scaffoldBackgroundColor: {},", surface));
    }
    if let Some(font) = font_family_ref {
        w.emit_fmt(format_args!("fontFamily: {},", font));
    }
    w.dedent();
    w.emit(");");
    w.dedent();
    w.emit("}");
    w.emit("");
}

// ═══════════════════════════════════════════════════════════════
// Step 5 — Dynamic tokens: Provider<Map<String, Object>> from store signals
// ═══════════════════════════════════════════════════════════════

/// Emit `dynamicTokensProvider` — store signal values keyed by token name,
/// mirroring the React `useDesignTokens()` hook:
///
/// ```dart
/// final dynamicTokensProvider = Provider<Map<String, Object>>((ref) {
///   final dynamicTokenStore = ref.watch(dynamicTokenStoreProvider);
///   return {
///     '--dynColor': dynamicTokenStore.liveColor,
///   };
/// });
/// ```
pub(crate) fn emit_dynamic_token_provider(
    w: &mut CodeWriter,
    designs: &[&DesignIR],
    ir: &IRProgram,
) {
    // (token name, expr) for every dynamic token across the designs.
    let mut dynamic_tokens: Vec<(String, &ExprIR)> = Vec::new();
    for design in designs {
        let mut tokens: Vec<(&String, &TokenIR)> = design.tokens.iter().collect();
        tokens.sort_by(|a, b| a.0.cmp(b.0));
        for (name, token) in tokens {
            if let TokenValueIR::Dynamic(expr) = &token.value {
                dynamic_tokens.push((name.clone(), expr));
            }
        }
    }
    if dynamic_tokens.is_empty() {
        return;
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
    let provider_map: FxHashMap<String, String> = known_stores
        .iter()
        .map(|s| (s.clone(), format!("{}Provider", lower_first(s))))
        .collect();

    w.emit("/// Dynamic design tokens: store signal values keyed by token name.");
    w.emit("/// Mirrors the React `useDesignTokens()` hook — watch this provider");
    w.emit("/// where a ThemeData is built and pass the map to `applyDynamicTokens`.");
    w.emit("final dynamicTokensProvider = Provider<Map<String, Object>>((ref) {");
    w.indent();
    for store in &known_stores {
        let local = lower_first(store);
        w.emit_fmt(format_args!(
            "final {} = ref.watch({}Provider);",
            local, local
        ));
    }
    w.emit("return {");
    w.indent();
    for (name, expr) in &dynamic_tokens {
        let value = simple_store_field(expr, &known_stores)
            .map(|(store, field)| format!("{}.{}", lower_first(store), field))
            .unwrap_or_else(|| {
                format_expr_ir_flutter(&rewrite_store_accesses(expr, &provider_map, ir))
            });
        w.emit_fmt(format_args!("'--{}': {},", name, value));
    }
    w.dedent();
    w.emit("};");
    w.dedent();
    w.emit("});");
    w.emit("");
}

/// `Store.signal` (either lowering shape) → `(store name, field name)` when
/// the store is a known store. Other expression shapes return `None` and are
/// formatted through the full expression pipeline instead.
fn simple_store_field<'a>(expr: &'a ExprIR, known_stores: &[String]) -> Option<(&'a str, &'a str)> {
    let is_known = |name: &str| known_stores.iter().any(|s| s == name);
    match expr {
        ExprIR::StoreFieldAccessExpr {
            store_alias,
            field_name,
            call_args,
        } if call_args.is_empty() => {
            let name = store_alias.trim_start_matches('$');
            is_known(name).then_some((name, field_name))
        }
        ExprIR::FieldAccessExpr { object, field } => {
            if let ExprIR::IdentifierExpr { name, .. } = object.as_ref() {
                let stripped = name.trim_start_matches('$');
                is_known(stripped).then_some((stripped, field))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Emit `applyDynamicTokens` — override `ColorScheme` slots from the dynamic
/// token map, using the same exact-name slot mapping as the `ThemeData`
/// builders. Emitted only when at least one dynamic token matches a slot.
pub(crate) fn emit_apply_dynamic_tokens(w: &mut CodeWriter, designs: &[&DesignIR]) {
    let mut slots: Vec<(&str, &str)> = Vec::new(); // (slot, token name)
    for design in designs {
        let mut tokens: Vec<(&String, &TokenIR)> = design.tokens.iter().collect();
        tokens.sort_by(|a, b| a.0.cmp(b.0));
        for (name, token) in tokens {
            if token.token_type == TokenTypeIR::Color
                && matches!(token.value, TokenValueIR::Dynamic(_))
            {
                if let Some(slot) = color_scheme_slot(name) {
                    slots.push((slot, name));
                }
            }
        }
    }
    if slots.is_empty() {
        return;
    }

    w.emit("/// Apply dynamic token values (from `dynamicTokensProvider`) on top");
    w.emit("/// of a statically built ThemeData. Dynamic color tokens whose names");
    w.emit("/// match a color token used by the theme override that ColorScheme");
    w.emit("/// slot; the rest stay available via the provider map.");
    w.emit("ThemeData applyDynamicTokens(ThemeData base, Map<String, Object> dynamicTokens) {");
    w.indent();
    w.emit("final scheme = base.colorScheme;");
    w.emit("return base.copyWith(");
    w.indent();
    w.emit("colorScheme: scheme.copyWith(");
    w.indent();
    for (slot, name) in &slots {
        w.emit_fmt(format_args!(
            "{}: _parseHexColor(dynamicTokens['--{}']) ?? scheme.{},",
            slot, name, slot
        ));
    }
    w.dedent();
    w.emit("),");
    w.dedent();
    w.emit(");");
    w.dedent();
    w.emit("}");
    w.emit("");

    w.emit("/// Parse a '#rrggbb' hex string (as stored in a dynamic token's");
    w.emit("/// store signal) into a Color; null when the value is not hex.");
    w.emit("Color? _parseHexColor(Object? value) {");
    w.indent();
    w.emit("if (value is String && value.startsWith('#')) {");
    w.indent();
    w.emit("final hex = value.substring(1);");
    w.emit("if (hex.length == 6) {");
    w.indent();
    w.emit("final parsed = int.tryParse('FF$hex', radix: 16);");
    w.emit("if (parsed != null) {");
    w.indent();
    w.emit("return Color(parsed);");
    w.dedent();
    w.emit("}");
    w.dedent();
    w.emit("}");
    w.dedent();
    w.emit("}");
    w.emit("return null;");
    w.dedent();
    w.emit("}");
    w.emit("");
}
