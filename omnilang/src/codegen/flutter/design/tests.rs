use super::*;
use crate::ir::{ModuleIR, StoreIR, TypeIR};

/// A `TokenIR` with a static value and optional theme overrides.
fn token(
    value: TokenValueIR,
    token_type: TokenTypeIR,
    theme_values: Vec<(String, TokenValueIR)>,
) -> TokenIR {
    TokenIR {
        value,
        token_type,
        theme_values: theme_values.into_iter().collect(),
        source_span: None,
    }
}

/// Build a `DesignIR` with static + dynamic tokens, classes, a variant,
/// and two themes (mirrors learn-omniview's design.omni shape).
fn make_design() -> DesignIR {
    let mut tokens = FxHashMap::default();
    tokens.insert(
        "surface".to_string(),
        token(
            TokenValueIR::Static("#ffffff".to_string()),
            TokenTypeIR::Color,
            vec![(
                "dark".to_string(),
                TokenValueIR::Static("#111827".to_string()),
            )],
        ),
    );
    tokens.insert(
        "primary".to_string(),
        token(
            TokenValueIR::Static("#6366f1".to_string()),
            TokenTypeIR::Color,
            vec![(
                "dark".to_string(),
                TokenValueIR::Static("#a5b4fc".to_string()),
            )],
        ),
    );
    tokens.insert(
        "danger".to_string(),
        token(
            TokenValueIR::Static("#ef4444".to_string()),
            TokenTypeIR::Color,
            vec![],
        ),
    );
    tokens.insert(
        "accent".to_string(),
        token(
            TokenValueIR::Dynamic(ExprIR::StoreFieldAccessExpr {
                store_alias: "DynamicTokenStore".to_string(),
                field_name: "liveAccent".to_string(),
                call_args: vec![],
            }),
            TokenTypeIR::Color,
            vec![],
        ),
    );
    tokens.insert(
        "spaceXs".to_string(),
        token(
            TokenValueIR::Static("4px".to_string()),
            TokenTypeIR::Size,
            vec![],
        ),
    );
    tokens.insert(
        "spaceSm".to_string(),
        token(
            TokenValueIR::Static("8px".to_string()),
            TokenTypeIR::Size,
            vec![],
        ),
    );
    tokens.insert(
        "radiusMd".to_string(),
        token(
            TokenValueIR::Static("8px".to_string()),
            TokenTypeIR::Radius,
            vec![],
        ),
    );
    tokens.insert(
        "radiusLg".to_string(),
        token(
            TokenValueIR::Static("12px".to_string()),
            TokenTypeIR::Radius,
            vec![],
        ),
    );
    tokens.insert(
        "shadowSm".to_string(),
        token(
            TokenValueIR::Static("0 1px 2px rgba(0,0,0,0.05)".to_string()),
            TokenTypeIR::Shadow,
            vec![],
        ),
    );
    tokens.insert(
        "fontFamily".to_string(),
        token(
            TokenValueIR::Static("system-ui, sans-serif".to_string()),
            TokenTypeIR::FontFamily,
            vec![],
        ),
    );
    tokens.insert(
        "fontSizeSm".to_string(),
        token(
            TokenValueIR::Static("13px".to_string()),
            TokenTypeIR::FontSize,
            vec![],
        ),
    );
    tokens.insert(
        "fontWeightSemibold".to_string(),
        token(
            TokenValueIR::Static("600".to_string()),
            TokenTypeIR::FontWeight,
            vec![],
        ),
    );
    tokens.insert(
        "lineHeight".to_string(),
        token(
            TokenValueIR::Static("1.5".to_string()),
            TokenTypeIR::LineHeight,
            vec![],
        ),
    );
    tokens.insert(
        "letterSpacing".to_string(),
        token(
            TokenValueIR::Static("0px".to_string()),
            TokenTypeIR::LetterSpacing,
            vec![],
        ),
    );
    tokens.insert(
        "durationFast".to_string(),
        token(
            TokenValueIR::Static("150ms".to_string()),
            TokenTypeIR::Duration,
            vec![],
        ),
    );
    tokens.insert(
        "easing".to_string(),
        token(
            TokenValueIR::Static("ease".to_string()),
            TokenTypeIR::Easing,
            vec![],
        ),
    );
    tokens.insert(
        "opacityDisabled".to_string(),
        token(
            TokenValueIR::Static("0.5".to_string()),
            TokenTypeIR::Opacity,
            vec![],
        ),
    );
    tokens.insert(
        "zModal".to_string(),
        token(
            TokenValueIR::Static("100".to_string()),
            TokenTypeIR::ZIndex,
            vec![],
        ),
    );
    tokens.insert(
        "dynColor".to_string(),
        token(
            TokenValueIR::Dynamic(ExprIR::StoreFieldAccessExpr {
                store_alias: "DynamicTokenStore".to_string(),
                field_name: "liveColor".to_string(),
                call_args: vec![],
            }),
            TokenTypeIR::Color,
            vec![],
        ),
    );

    let classes = vec![
        ClassIR {
            name: "CardDecoration".to_string(),
            properties: vec![
                ("background".to_string(), "var(--surface)".to_string()),
                ("border-radius".to_string(), "var(--radiusLg)".to_string()),
                ("box-shadow".to_string(), "var(--shadowSm)".to_string()),
                ("border".to_string(), "1px solid #d4d4d8".to_string()),
                // Layout-only — must not appear in the BoxDecoration.
                ("padding".to_string(), "16px".to_string()),
            ]
            .into_iter()
            .collect(),
            pseudo_states: vec![],
            source_span: None,
        },
        ClassIR {
            name: "Banner".to_string(),
            properties: vec![
                ("background".to_string(), "#6366f1".to_string()),
                // Text color — not decoration-mappable.
                ("color".to_string(), "white".to_string()),
            ]
            .into_iter()
            .collect(),
            pseudo_states: vec![],
            source_span: None,
        },
    ];

    let variants = vec![VariantIR {
        name: "Button".to_string(),
        dimensions: vec![
            VariantDimIR {
                name: "color".to_string(),
                values: vec!["primary".to_string(), "danger".to_string()],
                source_span: None,
            },
            VariantDimIR {
                name: "size".to_string(),
                values: vec!["sm".to_string(), "md".to_string()],
                source_span: None,
            },
        ],
        defaults: vec![
            ("color".to_string(), "primary".to_string()),
            ("size".to_string(), "md".to_string()),
        ]
        .into_iter()
        .collect(),
        base: vec![
            ("border-radius".to_string(), "var(--radiusMd)".to_string()),
            (
                "font-weight".to_string(),
                "var(--fontWeightSemibold)".to_string(),
            ),
        ]
        .into_iter()
        .collect(),
        branches: vec![
            VariantBranchIR {
                dimension: "color".to_string(),
                value: "primary".to_string(),
                properties: vec![
                    ("background".to_string(), "var(--primary)".to_string()),
                    ("color".to_string(), "white".to_string()),
                ]
                .into_iter()
                .collect(),
                source_span: None,
            },
            VariantBranchIR {
                dimension: "color".to_string(),
                value: "danger".to_string(),
                properties: vec![
                    ("background".to_string(), "var(--danger)".to_string()),
                    ("color".to_string(), "white".to_string()),
                ]
                .into_iter()
                .collect(),
                source_span: None,
            },
            VariantBranchIR {
                dimension: "size".to_string(),
                value: "sm".to_string(),
                properties: vec![
                    ("padding-top".to_string(), "var(--spaceXs)".to_string()),
                    ("padding-right".to_string(), "var(--spaceXs)".to_string()),
                    ("padding-bottom".to_string(), "var(--spaceXs)".to_string()),
                    ("padding-left".to_string(), "var(--spaceXs)".to_string()),
                    ("font-size".to_string(), "var(--fontSizeSm)".to_string()),
                ]
                .into_iter()
                .collect(),
                source_span: None,
            },
            VariantBranchIR {
                dimension: "size".to_string(),
                value: "md".to_string(),
                properties: vec![("font-size".to_string(), "14px".to_string())]
                    .into_iter()
                    .collect(),
                source_span: None,
            },
        ],
        source_span: None,
    }];

    DesignIR {
        name: "App".to_string(),
        themes: vec!["light".to_string(), "dark".to_string()],
        theme_signal: None,
        default_theme: Some("light".to_string()),
        tokens,
        classes,
        variants,
        source_file: Some("src/design.omni".to_string()),
        source_span: None,
    }
}

/// Build an IR program containing the design plus the referenced store.
fn make_ir(design: DesignIR) -> IRProgram {
    let store = StoreIR {
        name: "DynamicTokenStore".to_string(),
        signals: vec![
            (
                "liveColor".to_string(),
                TypeIR {
                    kind: "String".into(),
                    type_args: vec![],
                    is_nullable: false,
                },
            ),
            (
                "liveAccent".to_string(),
                TypeIR {
                    kind: "String".into(),
                    type_args: vec![],
                    is_nullable: false,
                },
            ),
        ]
        .into_iter()
        .collect(),
        signal_defaults: vec![
            ("liveColor".to_string(), serde_json::json!("#6366f1")),
            ("liveAccent".to_string(), serde_json::json!("#f97316")),
        ]
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
            ModuleIR {
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
fn emits_tokens_class_with_typed_constants() {
    let ir = make_ir(make_design());
    let (files, warnings, _errors) = gen_design_files(&ir);
    let content = &files[0].content;
    assert_eq!(files[0].path, "design.dart");
    // ZIndex → the spec §10.3 warning naming the token + design.
    assert!(
        warnings
            .iter()
            .any(|w| w.message.contains("token 'zModal' of design 'App'")
                && w.message.contains("Flutter z 序由 widget 顺序决定")),
        "ZIndex must warn with the spec §10.3 message. Got:\n{:#?}",
        warnings
    );
    assert!(content.contains("class AppTokens {"));
    assert!(content.contains("static const Color primary = Color(0xFF6366F1);"));
    assert!(content.contains("static const Color surface = Color(0xFFFFFFFF);"));
    assert!(content.contains("static const double spaceSm = 8.0;"));
    assert!(content.contains("static const double radiusLg = 12.0;"));
    assert!(content.contains("static const Duration durationFast = Duration(milliseconds: 150);"));
    assert!(content.contains("static const FontWeight fontWeightSemibold = FontWeight.w600;"));
    assert!(content.contains("static const Curve easing = Curves.easeOut;"));
    // ZIndex (spec §10.3): no meaningless `static const int` — the token
    // is skipped with a warning instead.
    assert!(
        !content.contains("zModal"),
        "ZIndex tokens must not emit Dart constants (spec §10.3). Got:\n{}",
        content
    );
    assert!(content.contains("static const double opacityDisabled = 0.5;"));
    assert!(content.contains("static const double lineHeight = 1.5;"));
    assert!(content.contains("static const double letterSpacing = 0.0;"));
    // Task 3.10: a CSS font stack degrades to its FIRST font with an
    // honest warning — a raw stack silently falls back to the default.
    assert!(content.contains("static const String fontFamily = 'system-ui';"));
    assert!(
        warnings
            .iter()
            .any(|w| w.message.contains("fontFamily") && w.message.contains("font stack")),
        "font stacks must warn via the design channel. Got:\n{:#?}",
        warnings
    );
    assert!(content.contains(
        "static const BoxShadow shadowSm = BoxShadow(color: Color.fromARGB(13, 0, 0, 0), \
             offset: Offset(0.0, 1.0), blurRadius: 2.0);"
    ));
    assert!(
        !content.contains("static const Color dynColor"),
        "dynamic tokens must not become constants"
    );
}

#[test]
fn emits_per_theme_constants() {
    let ir = make_ir(make_design());
    let (files, _warnings, _errors) = gen_design_files(&ir);
    let content = &files[0].content;
    assert!(content.contains("static const Color primaryDark = Color(0xFFA5B4FC);"));
    assert!(content.contains("static const Color surfaceDark = Color(0xFF111827);"));
    assert!(
        !content.contains("dangerDark"),
        "tokens without a dark override must not get per-theme constants"
    );
}

#[test]
fn emits_box_decoration_factories() {
    let ir = make_ir(make_design());
    let (files, _warnings, _errors) = gen_design_files(&ir);
    let content = &files[0].content;
    assert!(content.contains("class AppDesign {"));
    assert!(content.contains("static BoxDecoration cardDecoration() {"));
    // Whole var() references resolve to typed AppTokens constants.
    assert!(content.contains("color: AppTokens.surface,"));
    assert!(content.contains("borderRadius: BorderRadius.circular(AppTokens.radiusLg),"));
    assert!(content.contains("boxShadow: [AppTokens.shadowSm],"));
    assert!(content.contains(
        "border: Border.fromBorderSide(BorderSide(color: Color(0xFFD4D4D8), width: 1.0)),"
    ));
    // Layout-only properties are skipped (the Card class declares
    // `padding: 16px` — it must not become a BoxDecoration field).
    assert!(!content.contains("padding: 16.0"));
    assert!(content.contains("static BoxDecoration banner() {"));
    assert!(content.contains("color: Color(0xFF6366F1),"));
    // The Banner class also declares `color: white` (a text color) — it
    // must not map onto the BoxDecoration (only `background` does).
    let banner_body = content
        .split("static BoxDecoration banner() {")
        .nth(1)
        .expect("banner factory should exist");
    let banner_body = &banner_body[..banner_body.find('}').expect("closing brace")];
    assert!(banner_body.contains("color: Color(0xFF6366F1)"));
    assert!(
        !banner_body.contains("Color(0xFFFFFFFF)"),
        "text color `white` must not map onto a BoxDecoration color"
    );
}

#[test]
fn emits_theme_variant_extension() {
    let ir = make_ir(make_design());
    let (files, _warnings, _errors) = gen_design_files(&ir);
    let content = &files[0].content;
    assert!(content.contains("class AppThemeVariant extends ThemeExtension<AppThemeVariant> {"));
    assert!(content.contains("final Color? background;"));
    assert!(content.contains("final double? borderRadius;"));
    assert!(content.contains("final FontWeight? fontWeight;"));
    assert!(content.contains("final EdgeInsets? padding;"));
    assert!(content.contains("AppThemeVariant copyWith({"));
    assert!(content.contains("background: background ?? this.background,"));
    assert!(content
        .contains("AppThemeVariant lerp(ThemeExtension<AppThemeVariant>? other, double t) {"));
    assert!(content.contains("Color.lerp(background, other.background, t)"));
    assert!(content.contains("lerpDouble(fontSize, other.fontSize, t)"));
    // Non-lerpable kinds (FontWeight, Border, List<BoxShadow>, String)
    // fall back to picking one side.
    assert!(content.contains("t < 0.5 ? fontWeight : other.fontWeight"));
    // Branch consts: primary/danger merge base + their color branch;
    // sm/md merge base + their size branch.
    assert!(content.contains("static const AppThemeVariant primary = AppThemeVariant("));
    assert!(content.contains("background: AppTokens.primary,"));
    assert!(content.contains("color: Color(0xFFFFFFFF),"));
    // ThemeExtension fields are raw doubles — no BorderRadius wrapper.
    assert!(content.contains("borderRadius: AppTokens.radiusMd,"));
    assert!(content.contains("fontWeight: AppTokens.fontWeightSemibold,"));
    assert!(content.contains("static const AppThemeVariant danger = AppThemeVariant("));
    assert!(content.contains("background: AppTokens.danger,"));
    assert!(content.contains("static const AppThemeVariant sm = AppThemeVariant("));
    assert!(content.contains("fontSize: AppTokens.fontSizeSm,"));
    assert!(content.contains(
        "padding: EdgeInsets.fromLTRB(AppTokens.spaceXs, AppTokens.spaceXs, \
             AppTokens.spaceXs, AppTokens.spaceXs),"
    ));
    assert!(content.contains("static const AppThemeVariant md = AppThemeVariant("));
    assert!(content.contains("fontSize: 14.0,"));
}

/// Build an IR program containing the design plus a `ThemeStore` whose
/// `mode` signal is marked `@theme` and an `App` root component (the v2
/// D1 wiring shape — Flutter app shell).
fn make_theme_ir(design: DesignIR) -> IRProgram {
    make_theme_ir_multi(vec![design])
}

/// Same as `make_theme_ir` but with MULTIPLE design blocks — the
/// Task 3.6 I-1 multi-design theme-program shape.
fn make_theme_ir_multi(designs: Vec<DesignIR>) -> IRProgram {
    let theme_store = StoreIR {
        name: "ThemeStore".to_string(),
        signals: vec![(
            "mode".to_string(),
            TypeIR {
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
    let mut module = ModuleIR {
        name: "main".to_string(),
        stores: vec![("ThemeStore".into(), theme_store)]
            .into_iter()
            .collect(),
        designs,
        ..Default::default()
    };
    module.components.insert(
        "App".to_string(),
        crate::ir::ComponentIR {
            name: "App".to_string(),
            view: None,
            ..Default::default()
        },
    );
    IRProgram {
        version: "0.1".to_string(),
        modules: vec![("main".into(), module)].into_iter().collect(),
        source_map: FxHashMap::default(),
        stdlib_imports: FxHashSet::default(),
        source_span: None,
        motion_index: FxHashMap::default(),
    }
}

#[test]
fn emits_theme_switch_and_builders() {
    let ir = make_ir(make_design());
    let (files, _warnings, _errors) = gen_design_files(&ir);
    let content = &files[0].content;
    // Riverpod 3.0: Notifier<ThemeMode> + NotifierProvider — the legacy
    // StateProvider import is gone (Task 4.2).
    assert!(
        !content.contains("legacy.dart"),
        "no legacy import. Got:\n{}",
        content
    );
    assert!(content.contains("import 'package:flutter_riverpod/flutter_riverpod.dart';"));
    assert!(content.contains("import 'dart:ui' show lerpDouble;"));
    assert!(content.contains(
            "final themeModeProvider = NotifierProvider<ThemeModeNotifier, ThemeMode>(ThemeModeNotifier.new);"
        ));
    assert!(content.contains("class ThemeModeNotifier extends Notifier<ThemeMode> {"));
    assert!(content.contains("ThemeMode build() => ThemeMode.light;"));
    assert!(content.contains("ThemeData buildAppLightTheme() {"));
    assert!(content.contains("colorScheme: ColorScheme.light("));
    assert!(content.contains("primary: AppTokens.primary,"));
    assert!(content.contains("surface: AppTokens.surface,"));
    assert!(content.contains("error: AppTokens.danger,"));
    assert!(content.contains("scaffoldBackgroundColor: AppTokens.surface,"));
    assert!(content.contains("fontFamily: AppTokens.fontFamily,"));
    assert!(content.contains("ThemeData buildAppDarkTheme() {"));
    assert!(content.contains("colorScheme: ColorScheme.dark("));
    // Dark theme resolves per-theme constants where overrides exist.
    assert!(content.contains("primary: AppTokens.primaryDark,"));
    assert!(content.contains("surface: AppTokens.surfaceDark,"));
    assert!(
        !content.contains("tertiary: AppTokens"),
        "dynamic color tokens have no const and must not appear in ThemeData"
    );
}

/// Task 4.2: with a @theme signal the ThemeModeNotifier watches the
/// USER's store (ordinary store action = theme switch on Flutter too).
#[test]
fn emits_theme_notifier_watching_theme_signal() {
    let mut design = make_design();
    design.theme_signal = Some("ThemeStore.mode".to_string());
    let ir = make_theme_ir(design);
    let (files, _warnings, _errors) = gen_design_files(&ir);
    let content = &files[0].content;
    assert!(
        content.contains("import './ThemeStore_store.dart';"),
        "must import the @theme store. Got:\n{}",
        content
    );
    assert!(content.contains(
            "final themeModeProvider = NotifierProvider<ThemeModeNotifier, ThemeMode>(ThemeModeNotifier.new);"
        ));
    assert!(content.contains("class ThemeModeNotifier extends Notifier<ThemeMode> {"));
    assert!(
        content.contains("final mode = ref.watch(themeStoreProvider.select((s) => s.mode));"),
        "the notifier must watch the @theme signal. Got:\n{}",
        content
    );
    assert!(
        content.contains("return mode == 'dark' ? ThemeMode.dark : ThemeMode.light;"),
        "the notifier must map the signal value to ThemeMode. Got:\n{}",
        content
    );
    assert!(
        !content.contains("legacy.dart"),
        "no legacy import. Got:\n{}",
        content
    );
}

/// Task 4.2: when themes + a @theme signal are declared, an app shell
/// (main.dart) wires MaterialApp to the themeModeProvider.
#[test]
fn emits_app_shell_with_theme_wiring() {
    let mut design = make_design();
    design.theme_signal = Some("ThemeStore.mode".to_string());
    let ir = make_theme_ir(design);
    let shell = gen_theme_app_shell(&ir)
        .expect("main.dart should be emitted when themes + @theme signal exist");
    assert_eq!(shell.path, "main.dart");
    let content = &shell.content;
    assert!(content.contains("void main() {"));
    assert!(content.contains("runApp(const ProviderScope(child: OmniApp()));"));
    assert!(content.contains("class OmniApp extends ConsumerWidget {"));
    assert!(content.contains("MaterialApp("));
    assert!(content.contains("themeMode: ref.watch(themeModeProvider),"));
    assert!(content.contains("theme: buildAppLightTheme(),"));
    assert!(content.contains("darkTheme: buildAppDarkTheme(),"));
    // Task 3.9: the root sits in a Scaffold + SafeArea shell (status-bar
    // insets + Material background) instead of a bare `home:`.
    assert!(content.contains("home: Scaffold("));
    assert!(content.contains("body: SafeArea("));
    assert!(content.contains("child: const App(),"));
}

/// Task 3.6 (D-G): >2 declared themes switch the runtime from the
/// two-state ThemeModeNotifier to a `themeDataProvider` selecting the
/// active ThemeData builder across ALL declared themes — sepia /
/// highContrast builders are reachable (no dead code) and the platform
/// floor (Flutter ThemeMode is two-state) is documented by a warning.
#[test]
fn emits_theme_data_provider_for_multi_theme() {
    let mut design = make_design();
    design.themes = vec![
        "light".to_string(),
        "dark".to_string(),
        "sepia".to_string(),
        "highContrast".to_string(),
    ];
    design.theme_signal = Some("ThemeStore.mode".to_string());
    let ir = make_theme_ir(design);
    let (files, warnings, _errors) = gen_design_files(&ir);
    let content = &files[0].content;
    assert!(content.contains("final themeDataProvider = Provider<ThemeData>((ref) {"));
    assert!(
        !content.contains("themeModeProvider"),
        "the multi-theme path must not emit the two-state ThemeMode runtime"
    );
    assert!(content.contains("'sepia' => buildAppSepiaTheme(),"));
    assert!(content.contains("'highContrast' => buildAppHighContrastTheme(),"));
    assert!(content.contains("ThemeData buildAppSepiaTheme() {"));
    assert!(content.contains("ThemeData buildAppHighContrastTheme() {"));
    assert!(
        warnings.iter().any(|w| w.message.contains("light/dark")),
        "the >2-theme platform floor must warn, got: {:?}",
        warnings
    );
}

/// Task 3.6: the multi-theme app shell watches `themeDataProvider` —
/// `MaterialApp(theme:)` — instead of themeMode/darkTheme (Flutter's
/// ThemeMode cannot express the extra themes).
#[test]
fn emits_multi_theme_app_shell() {
    let mut design = make_design();
    design.themes = vec![
        "light".to_string(),
        "dark".to_string(),
        "sepia".to_string(),
        "highContrast".to_string(),
    ];
    design.theme_signal = Some("ThemeStore.mode".to_string());
    let ir = make_theme_ir(design);
    let shell = gen_theme_app_shell(&ir).expect("main.dart should be emitted");
    let content = &shell.content;
    assert!(content.contains("theme: ref.watch(themeDataProvider),"));
    assert!(
        !content.contains("themeMode:"),
        "multi-theme shell must not wire themeMode. Got:\n{}",
        content
    );
    assert!(
        !content.contains("darkTheme:"),
        "multi-theme shell must not wire darkTheme. Got:\n{}",
        content
    );
    assert!(content.contains("home: Scaffold("));
    assert!(content.contains("body: SafeArea("));
    assert!(content.contains("child: const App(),"));
}

/// Task 3.6 (I-1): the multi-theme decision covers the UNION of themes
/// across ALL theme-declaring designs — `design A { light, dark }` +
/// `design B { light, dark, sepia }` must warn and reach sepia (its
/// builder belongs to B), NOT silently collapse to the two-state path
/// on A. The shell must stay consistent with the emitted runtime
/// (themeDataProvider — never a missing themeModeProvider).
#[test]
fn multi_theme_union_across_designs() {
    let mut design_a = make_design();
    design_a.name = "A".to_string();
    design_a.themes = vec!["light".to_string(), "dark".to_string()];
    let mut design_b = make_design();
    design_b.name = "B".to_string();
    design_b.themes = vec!["light".to_string(), "dark".to_string(), "sepia".to_string()];
    design_b.theme_signal = Some("ThemeStore.mode".to_string());
    let ir = make_theme_ir_multi(vec![design_a, design_b]);
    let (files, warnings, _errors) = gen_design_files(&ir);
    let content = &files[0].content;
    // Union = {light, dark, sepia} → the multi-theme runtime is
    // emitted (A alone would have kept the two-state path).
    assert!(content.contains("final themeDataProvider = Provider<ThemeData>((ref) {"));
    assert!(
        content.contains("'sepia' => buildBSepiaTheme(),"),
        "sepia must resolve against its declaring design B, got:\n{}",
        content
    );
    assert!(
        content.contains("'light' => buildALightTheme(),"),
        "light must resolve against its declaring design A, got:\n{}",
        content
    );
    assert!(
        warnings.iter().any(|w| w.message.contains("light/dark")),
        "the union of themes must warn, got: {:?}",
        warnings
    );
    // The shell gate mirrors the union: themeDataProvider wiring, never
    // a reference to a non-emitted themeModeProvider.
    let shell = gen_theme_app_shell(&ir).expect("main.dart should be emitted");
    assert!(shell
        .content
        .contains("theme: ref.watch(themeDataProvider),"));
    assert!(
        !shell.content.contains("themeModeProvider"),
        "shell must not reference a provider the runtime does not emit"
    );
}

/// Task 4.2: no app shell without a @theme signal (existing projects
/// keep their own main.dart) and none without themes at all.
#[test]
fn no_app_shell_without_theme_signal() {
    let ir = make_theme_ir(make_design());
    assert!(
        gen_theme_app_shell(&ir).is_none(),
        "no main.dart when no @theme signal is wired"
    );
    let mut design = make_design();
    design.themes = vec![];
    design.default_theme = None;
    design.theme_signal = None;
    let ir = make_theme_ir(design);
    assert!(
        gen_theme_app_shell(&ir).is_none(),
        "no main.dart without @themes"
    );
}

#[test]
fn per_theme_const_collides_with_token_name() {
    // learn-omniview has both a `primaryDark` token AND a `primary` token
    // with a dark override — the per-theme constant must be renamed so
    // the generated class keeps unique identifiers.
    let mut design = make_design();
    design.tokens.insert(
        "primaryDark".to_string(),
        token(
            TokenValueIR::Static("#4338ca".to_string()),
            TokenTypeIR::Color,
            vec![],
        ),
    );
    let ir = make_ir(design);
    let content = gen_design_files(&ir).0[0].content.clone();
    assert!(content.contains("static const Color primaryDark = Color(0xFF4338CA);"));
    assert!(content.contains("static const Color primaryDarkTheme = Color(0xFFA5B4FC);"));
    assert_eq!(content.matches("static const Color primaryDark").count(), 2);
    // The dark theme builder references the disambiguated constant.
    assert!(content.contains("primary: AppTokens.primaryDarkTheme,"));
}

#[test]
fn emits_dynamic_token_provider() {
    let ir = make_ir(make_design());
    let (files, _warnings, _errors) = gen_design_files(&ir);
    let content = &files[0].content;
    assert!(content.contains("import './DynamicTokenStore_store.dart';"));
    assert!(content.contains("final dynamicTokensProvider = Provider<Map<String, Object>>((ref) {"));
    assert!(content.contains("final dynamicTokenStore = ref.watch(dynamicTokenStoreProvider);"));
    assert!(content.contains("'--dynColor': dynamicTokenStore.liveColor,"));
    assert!(content.contains("'--accent': dynamicTokenStore.liveAccent,"));
    // The `accent` dynamic token matches the tertiary slot.
    assert!(content.contains(
        "ThemeData applyDynamicTokens(ThemeData base, Map<String, Object> dynamicTokens) {"
    ));
    assert!(
        content.contains("tertiary: _parseHexColor(dynamicTokens['--accent']) ?? scheme.tertiary,")
    );
    assert!(content.contains("Color? _parseHexColor(Object? value) {"));
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
    assert!(gen_design_files(&ir).0.is_empty());
}

#[test]
fn design_without_themes_skips_theme_switch() {
    let mut design = make_design();
    design.themes = vec![];
    design.default_theme = None;
    let ir = make_ir(design);
    let (files, _warnings, _errors) = gen_design_files(&ir);
    let content = &files[0].content;
    assert!(content.contains("class AppTokens {"));
    assert!(
        !content.contains("themeModeProvider"),
        "no theme switch without @themes"
    );
    assert!(
        !content.contains("buildAppLightTheme"),
        "no ThemeData builders without @themes"
    );
}

#[test]
fn design_without_dynamic_tokens_skips_provider() {
    let mut design = make_design();
    design.tokens.remove("dynColor");
    design.tokens.remove("accent");
    let ir = make_ir(design);
    let (files, _warnings, _errors) = gen_design_files(&ir);
    let content = &files[0].content;
    assert!(
        !content.contains("dynamicTokensProvider"),
        "no provider without dynamic tokens"
    );
    assert!(
        !content.contains("DynamicTokenStore"),
        "no store import without dynamic tokens"
    );
}

#[test]
fn unmappable_values_are_skipped_not_crashed() {
    // Unknown easing + non-px size: constants are skipped, the rest of
    // the design still compiles.
    let mut design = make_design();
    design.tokens.insert(
        "badEasing".to_string(),
        token(
            TokenValueIR::Static("cubic-bezier(0.1, 0.2, 0.3, 0.4)".to_string()),
            TokenTypeIR::Easing,
            vec![],
        ),
    );
    design.tokens.insert(
        "badSize".to_string(),
        token(
            TokenValueIR::Static("12.5%".to_string()),
            TokenTypeIR::Size,
            vec![],
        ),
    );
    let ir = make_ir(design);
    let (files, warnings, _errors) = gen_design_files(&ir);
    let content = &files[0].content;
    assert!(!content.contains("badEasing"));
    assert!(!content.contains("badSize"));
    assert!(content.contains("static const Color primary = Color(0xFF6366F1);"));
    // Spec §10.3: unmappable token values must surface on the warnings
    // channel, naming the token and the reason.
    assert!(
        warnings.iter().any(|w| {
            w.message.contains("[flutter][design]")
                && w.message.contains("'badEasing'")
                && w.message.contains("cubic-bezier")
                && w.message.contains("skipped")
        }),
        "unmappable easing must warn: {:?}",
        warnings
    );
    assert!(
        warnings.iter().any(|w| {
            w.message.contains("'badSize'")
                && w.message.contains("12.5%")
                && w.message.contains("skipped")
        }),
        "unmappable size must warn: {:?}",
        warnings
    );
}

#[test]
fn unmappable_class_property_warns() {
    // A class whose box-shadow value cannot be parsed (inset shadows are
    // unsupported) must warn naming the class, property and value — and
    // the factory still emits with the remaining mappable fields.
    let mut design = make_design();
    design.classes.push(ClassIR {
        name: "WeirdShadow".to_string(),
        properties: vec![
            ("background".to_string(), "#ffffff".to_string()),
            (
                "box-shadow".to_string(),
                "inset 0 1px 2px rgba(0,0,0,0.5)".to_string(),
            ),
        ]
        .into_iter()
        .collect(),
        pseudo_states: vec![],
        source_span: None,
    });
    let ir = make_ir(design);
    let (files, warnings, _errors) = gen_design_files(&ir);
    let content = &files[0].content;
    assert!(
        content.contains("static BoxDecoration weirdShadow() {"),
        "mappable fields must still emit a factory. Got:\n{}",
        content
    );
    assert!(
        content.contains("color: Color(0xFFFFFFFF),"),
        "background must survive the unmappable shadow"
    );
    assert!(
        warnings.iter().any(|w| {
            w.message.contains("'WeirdShadow'")
                && w.message.contains("box-shadow")
                && w.message.contains("inset 0 1px 2px rgba(0,0,0,0.5)")
        }),
        "unmappable class property must warn: {:?}",
        warnings
    );
}

#[test]
fn layout_only_class_warns_and_skips_factory() {
    let mut design = make_design();
    design.classes.push(ClassIR {
        name: "RowLayout".to_string(),
        properties: vec![
            ("display".to_string(), "flex".to_string()),
            ("gap".to_string(), "8px".to_string()),
        ]
        .into_iter()
        .collect(),
        pseudo_states: vec![],
        source_span: None,
    });
    let ir = make_ir(design);
    let (files, warnings, _errors) = gen_design_files(&ir);
    assert!(
        !files[0].content.contains("rowLayout"),
        "layout-only classes must not emit a factory"
    );
    assert!(
        warnings.iter().any(|w| {
            w.message.contains("'RowLayout'")
                && w.message.contains("no decoration-mappable properties")
        }),
        "layout-only class must warn: {:?}",
        warnings
    );
}

#[test]
fn variant_resolution_merges_base_and_selected_branches() {
    let design = make_design();
    let variant = &design.variants[0]; // Button: color(primary|danger), size(sm|md)
                                       // color=primary, size omitted → defaults: primary + md.
    let dims: FxHashMap<String, String> = vec![("color".to_string(), "primary".to_string())]
        .into_iter()
        .collect();
    let (fields, skipped) = resolve_variant_decoration(&design, variant, &dims);
    assert!(skipped.is_empty(), "no skips expected: {:?}", skipped);
    // base border-radius + primary branch background; size:md contributes
    // only font-size/padding (not decoration fields).
    assert_eq!(
        fields.get("color").map(String::as_str),
        Some("AppTokens.primary")
    );
    assert_eq!(
        fields.get("borderRadius").map(String::as_str),
        Some("BorderRadius.circular(AppTokens.radiusMd)")
    );
    // A decoration with REAL values round-trips into a BoxDecoration expr.
    let expr = decoration_fields_to_expr(&fields);
    assert!(expr.contains("color: AppTokens.primary"));
    assert!(expr.contains("borderRadius: BorderRadius.circular(AppTokens.radiusMd)"));
    assert!(expr.starts_with("BoxDecoration(") && expr.ends_with(')'));
}

#[test]
fn variant_resolution_unknown_dimension_value_is_ignored() {
    let design = make_design();
    let variant = &design.variants[0];
    // "color=neon" is not a declared value → the branch is ignored; only
    // base props apply (border-radius).
    let dims: FxHashMap<String, String> = vec![("color".to_string(), "neon".to_string())]
        .into_iter()
        .collect();
    let (fields, _) = resolve_variant_decoration(&design, variant, &dims);
    assert!(
        !fields.contains_key("color"),
        "undeclared branch must not apply"
    );
    assert!(fields.contains_key("borderRadius"));
}

#[test]
fn class_factory_expr_names_design_and_lowercased_class() {
    let design = make_design();
    let class = &design.classes[0]; // CardDecoration (decoration-mappable)
    assert_eq!(
        class_factory_expr(&design, class).as_deref(),
        Some("AppDesign.cardDecoration()")
    );
    let layout_only = &design.classes[1]; // Banner has background → mappable
    assert!(
        class_factory_expr(&design, layout_only).is_some(),
        "Banner has a background — factory must exist"
    );
}

#[test]
fn merge_decoration_fields_later_wins() {
    let base: DecorationFields = [("color", "A")]
        .into_iter()
        .map(|(k, v)| (k, v.to_string()))
        .collect();
    let over: DecorationFields = [("color", "B"), ("borderRadius", "C")]
        .into_iter()
        .map(|(k, v)| (k, v.to_string()))
        .collect();
    let merged = merge_decoration_fields(base, over);
    assert_eq!(merged.get("color").map(String::as_str), Some("B"));
    assert_eq!(merged.get("borderRadius").map(String::as_str), Some("C"));
}

// ── Spec §10.3 layout diagnostics (Task 4.4) ─────────────────

#[test]
fn display_grid_class_emits_error() {
    let mut design = make_design();
    design.classes.push(ClassIR {
        name: "GridPanel".to_string(),
        properties: vec![("display".to_string(), "grid".to_string())]
            .into_iter()
            .collect(),
        pseudo_states: vec![],
        source_span: None,
    });
    let ir = make_ir(design);
    let (files, _warnings, errors) = gen_design_files(&ir);
    assert!(
        errors.iter().any(|e| {
            e.severity == Severity::Error
                && e.message.contains("'GridPanel'")
                && e.message.contains("'display'")
                && e.message.contains("Flutter 不支持 CSS grid")
        }),
        "display:grid must hard-error with the spec §10.3 message. Got:\n{:#?}",
        errors
    );
    assert!(
        !files[0].content.contains("gridPanel"),
        "the grid class must not emit a factory"
    );
}

#[test]
fn float_class_emits_error() {
    let mut design = make_design();
    design.classes.push(ClassIR {
        name: "Floaty".to_string(),
        properties: vec![("float".to_string(), "left".to_string())]
            .into_iter()
            .collect(),
        pseudo_states: vec![],
        source_span: None,
    });
    let ir = make_ir(design);
    let (_files, _warnings, errors) = gen_design_files(&ir);
    assert!(
        errors.iter().any(|e| {
            e.severity == Severity::Error
                && e.message.contains("'Floaty'")
                && e.message.contains("'float'")
                && e.message.contains("Flutter widget 布局无 float 概念")
        }),
        "float must hard-error with the spec §10.3 message. Got:\n{:#?}",
        errors
    );
}

#[test]
fn cursor_class_emits_warning() {
    let mut design = make_design();
    design.classes.push(ClassIR {
        name: "Clickable".to_string(),
        properties: vec![("cursor".to_string(), "pointer".to_string())]
            .into_iter()
            .collect(),
        pseudo_states: vec![],
        source_span: None,
    });
    let ir = make_ir(design);
    let (files, warnings, _errors) = gen_design_files(&ir);
    assert!(
        warnings.iter().any(|w| w.message.contains("'Clickable'")
            && w.message.contains("'cursor'")
            && w.message.contains("cursor 在 Flutter 中无效果")),
        "cursor must warn with the spec §10.3 message. Got:\n{:#?}",
        warnings
    );
    assert!(
        !files[0].content.contains("cursor"),
        "cursor must not leak into generated Dart"
    );
}

/// Variants surface the same layout diagnostics from base + branches.
#[test]
fn variant_layout_diagnostics_cover_base_and_branches() {
    let mut design = make_design();
    design.variants[0]
        .base
        .insert("cursor".to_string(), "pointer".to_string());
    design.variants[0].branches[0]
        .properties
        .insert("display".to_string(), "grid".to_string());
    let ir = make_ir(design);
    let (_files, warnings, errors) = gen_design_files(&ir);
    assert!(
        warnings
            .iter()
            .any(|w| w.message.contains("variant 'Button'")
                && w.message.contains("cursor 在 Flutter 中无效果")),
        "variant base cursor must warn. Got:\n{:#?}",
        warnings
    );
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("Flutter 不支持 CSS grid")),
        "variant branch display:grid must error. Got:\n{:#?}",
        errors
    );
}

/// Pseudo states are scanned for the layout diagnostics too.
#[test]
fn pseudo_state_layout_diagnostics_are_emitted() {
    let mut design = make_design();
    design.classes[0]
        .pseudo_states
        .push(crate::ir::design::PseudoStateIR {
            pseudo: "hover".to_string(),
            properties: vec![("cursor".to_string(), "pointer".to_string())]
                .into_iter()
                .collect(),
            source_span: None,
        });
    let ir = make_ir(design);
    let (_files, warnings, _errors) = gen_design_files(&ir);
    assert!(
        warnings
            .iter()
            .any(|w| w.message.contains("'CardDecoration'")
                && w.message.contains("cursor 在 Flutter 中无效果")),
        "pseudo-state cursor must warn. Got:\n{:#?}",
        warnings
    );
}

/// `display: flex` is NOT a §10.3 error — only `grid` is (spec verbatim).
#[test]
fn display_flex_is_not_an_error() {
    let mut design = make_design();
    design.classes.push(ClassIR {
        name: "RowLayout".to_string(),
        properties: vec![("display".to_string(), "flex".to_string())]
            .into_iter()
            .collect(),
        pseudo_states: vec![],
        source_span: None,
    });
    let ir = make_ir(design);
    let (_files, _warnings, errors) = gen_design_files(&ir);
    assert!(
        !errors
            .iter()
            .any(|e| e.message.contains("Flutter 不支持 CSS grid")),
        "display:flex must not error. Got:\n{:#?}",
        errors
    );
}
