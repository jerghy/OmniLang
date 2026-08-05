//! Flutter design codegen: CSS value → Dart expression mapping and typed
//! token constants (domain half of the former `design.rs` monolith).
//!
//! - Step 1 — `{Design}Tokens`: one `static const` per static token, typed
//!   by `TokenTypeIR` (`Color`, `double`, `Duration`, `FontWeight`,
//!   `Curve`, …), plus per-theme constants (`primaryDark`) for tokens with
//!   theme overrides.
//! - CSS value → Dart expression mappers shared by the tokens, classes and
//!   theme-wiring domains (`css_length_to_dart`, `css_shadow_to_dart`,
//!   `design_color_expr`, …).

use super::capitalize_first;
use super::utils::css_color_to_dart;
use crate::codegen::CodeWriter;
use crate::ir::design::*;

// ═══════════════════════════════════════════════════════════════
// Step 1 — {Design}Tokens: typed Dart constants
// ═══════════════════════════════════════════════════════════════

/// Emit the `{Design}Tokens` class: one `static const` per static token
/// (typed by `TokenTypeIR`), plus per-theme constants for tokens with theme
/// overrides (e.g. `primary` + theme `dark` → `primaryDark`).
///
/// Unmappable token values are skipped with a warning naming the token, its
/// type, and the offending value (spec §10.3 — no silent drops).
///
/// ZIndex tokens are skipped entirely with the spec §10.3 warning — a
/// `static const int` would be meaningless (Flutter z order is decided by
/// widget order/Stack, so nothing could consume the constant).
pub(crate) fn emit_tokens_class(
    w: &mut CodeWriter,
    design: &DesignIR,
    warnings: &mut Vec<crate::codegen::CodegenWarning>,
) {
    let class_name = format!("{}Tokens", design.name);
    w.emit_fmt(format_args!(
        "/// Typed design tokens of the `{}` design block.",
        design.name
    ));
    w.emit_fmt(format_args!("class {} {{", class_name));
    w.indent();
    w.emit_fmt(format_args!("{}._();", class_name));
    w.emit("");

    let mut sorted: Vec<(&String, &TokenIR)> = design.tokens.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(b.0));
    let mut emitted_any = false;
    for (name, token) in &sorted {
        let const_name = escape_dart_ident(name);
        // ZIndex (spec §10.3): the token type has no Flutter layout effect —
        // warn once and skip the constant (base + theme overrides alike).
        if token.token_type == TokenTypeIR::ZIndex {
            warnings.push(super::design::design_block_warning(
                design,
                format!(
                    "[flutter][design] token '{}'{}: Flutter z 序由 widget 顺序决定 — \
                     zIndex has no Flutter layout effect; no Dart constant emitted (skipped)",
                    name,
                    design_ctx_suffix(design)
                ),
            ));
            continue;
        }
        if let TokenValueIR::Static(v) = &token.value {
            // Task 3.10: FontFamily stacks degrade to their first font with
            // a warning before the constant is emitted.
            let effective = if token.token_type == TokenTypeIR::FontFamily {
                font_stack_first_font(design, name, v, warnings)
            } else {
                v.clone()
            };
            if let Some(decl) = token_const_decl(design, token, &const_name, &effective) {
                w.emit(&decl);
                emitted_any = true;
            } else {
                warnings.push(super::design::design_block_warning(
                    design,
                    token_skip_warning(design, name, None, &token.token_type, v),
                ));
            }
        }
        // Per-theme overrides — one constant per declared theme value.
        let mut theme_names: Vec<&String> = token.theme_values.keys().collect();
        theme_names.sort();
        for theme in theme_names {
            if let Some(TokenValueIR::Static(v)) = token.theme_values.get(theme) {
                let themed_name = theme_const_name(design, name, theme);
                let effective = if token.token_type == TokenTypeIR::FontFamily {
                    font_stack_first_font(design, name, v, warnings)
                } else {
                    v.clone()
                };
                if let Some(decl) = token_const_decl(design, token, &themed_name, &effective) {
                    w.emit(&decl);
                    emitted_any = true;
                } else {
                    warnings.push(super::design::design_block_warning(
                        design,
                        token_skip_warning(design, name, Some(theme), &token.token_type, v),
                    ));
                }
            }
        }
    }
    if !emitted_any {
        w.emit("// No static tokens were mappable to Dart constants.");
    }
    w.dedent();
    w.emit("}");
    w.emit("");
}

/// `" of design 'App'"` (or the empty string for the anonymous design) —
/// the design-owner suffix shared by design-channel warning/error messages.
fn design_ctx_suffix(design: &DesignIR) -> String {
    if design.name.is_empty() {
        String::new()
    } else {
        format!(" of design '{}'", design.name)
    }
}

/// Task 3.10 (flutter LOW): a CSS font stack (`"system-ui, sans-serif"`)
/// cannot be expressed as Flutter's single `fontFamily` string — a raw stack
/// silently falls back to the default font at runtime. Degrade to the FIRST
/// font with an honest warning (ADR-054); single-font values pass through.
fn font_stack_first_font(
    design: &DesignIR,
    name: &str,
    value: &str,
    warnings: &mut Vec<crate::codegen::CodegenWarning>,
) -> String {
    if !value.contains(',') {
        return value.to_string();
    }
    let first = value.split(',').next().unwrap_or("").trim();
    warnings.push(super::design::design_block_warning(
        design,
        format!(
            "[flutter][design] token '{name}'{}: font stack '{value}' has no \
             single-font equivalent in Flutter — using first font '{first}' \
             (multi-font CSS stacks are not expressible)",
            design_ctx_suffix(design)
        ),
    ));
    first.to_string()
}

/// One `[flutter][design]` warning for a token constant that could not be
/// emitted: names the token, its type, the offending value, and (for theme
/// overrides) the theme.
fn token_skip_warning(
    design: &DesignIR,
    name: &str,
    theme: Option<&str>,
    token_type: &TokenTypeIR,
    value: &str,
) -> String {
    let design_name = design_ctx_suffix(design);
    let type_name = match token_type {
        TokenTypeIR::Color => "Color",
        TokenTypeIR::Size => "Size",
        TokenTypeIR::Radius => "Radius",
        TokenTypeIR::FontFamily => "FontFamily",
        TokenTypeIR::FontSize => "FontSize",
        TokenTypeIR::FontWeight => "FontWeight",
        TokenTypeIR::LineHeight => "LineHeight",
        TokenTypeIR::LetterSpacing => "LetterSpacing",
        TokenTypeIR::Shadow => "Shadow",
        TokenTypeIR::Duration => "Duration",
        TokenTypeIR::Easing => "Easing",
        TokenTypeIR::Opacity => "Opacity",
        TokenTypeIR::ZIndex => "ZIndex",
    };
    match theme {
        Some(theme) => format!(
            "[flutter][design] token '{name}'{design_name} theme '{theme}' value \
             '{value}' is not mappable to a Dart '{type_name}' constant; skipped"
        ),
        None => format!(
            "[flutter][design] token '{name}'{design_name} value '{value}' is not \
             mappable to a Dart '{type_name}' constant; skipped"
        ),
    }
}

/// One `static const` declaration for a token value (base or theme override).
/// Returns `None` when the value cannot be mapped to Dart.
fn token_const_decl(
    design: &DesignIR,
    token: &TokenIR,
    const_name: &str,
    value: &str,
) -> Option<String> {
    let expr = token_static_expr(design, &token.token_type, value)?;
    let ann = token_const_annotation(design, token, value)?;
    Some(format!("static const {} {} = {};", ann, const_name, expr))
}

/// The Dart annotation for a token constant — shadow tokens are `BoxShadow`
/// or `List<BoxShadow>` depending on how many shadows the value declares.
fn token_const_annotation(design: &DesignIR, token: &TokenIR, value: &str) -> Option<&'static str> {
    match token.token_type {
        TokenTypeIR::Color => Some("Color"),
        TokenTypeIR::Size | TokenTypeIR::Radius | TokenTypeIR::FontSize => Some("double"),
        TokenTypeIR::Duration => Some("Duration"),
        TokenTypeIR::FontFamily => Some("String"),
        TokenTypeIR::FontWeight => Some("FontWeight"),
        TokenTypeIR::LineHeight | TokenTypeIR::LetterSpacing | TokenTypeIR::Opacity => {
            Some("double")
        }
        // ZIndex constants are never emitted (spec §10.3 — emit_tokens_class
        // skips the token before this is reached; the arm exists to keep the
        // match exhaustive).
        TokenTypeIR::ZIndex => None,
        TokenTypeIR::Easing => Some("Curve"),
        TokenTypeIR::Shadow => {
            let resolved = resolve_css_vars(design, value)?;
            let (is_list, _) = css_shadow_to_dart(&resolved)?;
            Some(if is_list {
                "List<BoxShadow>"
            } else {
                "BoxShadow"
            })
        }
    }
}

/// Format a static token value as a Dart const expression based on the
/// token's declared type. Returns `None` when the value cannot be mapped
/// (unknown easing, non-px length, unresolvable `var()` reference, …) — the
/// constant is skipped in that case.
fn token_static_expr(design: &DesignIR, token_type: &TokenTypeIR, value: &str) -> Option<String> {
    match token_type {
        TokenTypeIR::Color => design_color_expr(design, value),
        TokenTypeIR::Size | TokenTypeIR::Radius | TokenTypeIR::FontSize => {
            let resolved = resolve_css_vars(design, value)?;
            css_length_to_dart(&resolved)
        }
        TokenTypeIR::Duration => css_duration_to_dart(value),
        TokenTypeIR::FontFamily => {
            let resolved = resolve_css_vars(design, value)?;
            Some(dart_string_literal(resolved.trim()))
        }
        TokenTypeIR::FontWeight => {
            let resolved = resolve_css_vars(design, value)?;
            css_font_weight_to_dart(&resolved)
        }
        TokenTypeIR::LineHeight => {
            let resolved = resolve_css_vars(design, value)?;
            css_unitless_double_to_dart(&resolved)
        }
        TokenTypeIR::LetterSpacing => {
            let resolved = resolve_css_vars(design, value)?;
            css_length_to_dart(&resolved)
        }
        TokenTypeIR::Shadow => {
            let resolved = resolve_css_vars(design, value)?;
            css_shadow_to_dart(&resolved).map(|(_, expr)| expr)
        }
        TokenTypeIR::Opacity => {
            let resolved = resolve_css_vars(design, value)?;
            css_opacity_to_dart(&resolved)
        }
        // ZIndex constants are never emitted (spec §10.3 warning instead —
        // see emit_tokens_class), so no reference may resolve either.
        TokenTypeIR::ZIndex => None,
        TokenTypeIR::Easing => {
            let resolved = resolve_css_vars(design, value)?;
            css_easing_to_dart(&resolved)
        }
    }
}

/// Whether the token's base constant was emitted (static + mappable value).
/// Guards `AppTokens.xxx` references against dangling identifiers.
fn token_const_emitted(design: &DesignIR, token: &TokenIR) -> bool {
    match &token.value {
        TokenValueIR::Static(v) => token_static_expr(design, &token.token_type, v).is_some(),
        TokenValueIR::Dynamic(_) => false,
        #[allow(unreachable_patterns)]
        _ => false,
    }
}

/// Which constant to reference for `token` under `theme`: the per-theme
/// constant (`AppTokens.primaryDark`) when a static override exists for that
/// theme, else the base constant (`AppTokens.primary`). `None` when neither
/// was emitted.
pub(crate) fn token_theme_const_ref(
    design: &DesignIR,
    token: &TokenIR,
    name: &str,
    theme: &str,
) -> Option<String> {
    if !token_const_emitted(design, token) {
        return None;
    }
    let base = format!("{}Tokens.{}", design.name, escape_dart_ident(name));
    if let Some(TokenValueIR::Static(v)) = token.theme_values.get(theme) {
        if token_static_expr(design, &token.token_type, v).is_some() {
            return Some(format!(
                "{}Tokens.{}",
                design.name,
                theme_const_name(design, name, theme)
            ));
        }
    }
    Some(base)
}

/// The per-theme constant name for `(token, theme)`: `primary` + `dark` →
/// `primaryDark`. When a token of that exact name already exists (e.g. a
/// token literally named `primaryDark`), the `Theme` suffix is appended
/// (`primaryDarkTheme`) so the constant stays unique.
fn theme_const_name(design: &DesignIR, token_name: &str, theme: &str) -> String {
    let candidate = format!(
        "{}{}",
        escape_dart_ident(token_name),
        capitalize_first(theme)
    );
    if design.tokens.contains_key(&candidate) {
        format!("{}Theme", candidate)
    } else {
        candidate
    }
}

// ═══════════════════════════════════════════════════════════════
// CSS value → Dart expression mappers
// ═══════════════════════════════════════════════════════════════

/// Dart reserved words — a design token/class/variant named after one of
/// these gets a trailing underscore so the generated identifiers are valid.
pub(crate) fn escape_dart_ident(name: &str) -> String {
    const RESERVED: &[&str] = &[
        "abstract",
        "as",
        "assert",
        "async",
        "await",
        "break",
        "case",
        "catch",
        "class",
        "const",
        "continue",
        "covariant",
        "default",
        "deferred",
        "do",
        "dynamic",
        "else",
        "enum",
        "export",
        "extends",
        "extension",
        "external",
        "factory",
        "false",
        "final",
        "finally",
        "for",
        "function",
        "get",
        "hide",
        "if",
        "implements",
        "import",
        "in",
        "interface",
        "is",
        "late",
        "library",
        "mixin",
        "new",
        "null",
        "of",
        "on",
        "operator",
        "part",
        "required",
        "rethrow",
        "return",
        "set",
        "show",
        "static",
        "super",
        "switch",
        "sync",
        "this",
        "throw",
        "true",
        "try",
        "type",
        "typedef",
        "var",
        "void",
        "while",
        "with",
        "yield",
    ];
    if RESERVED.contains(&name) {
        format!("{name}_")
    } else {
        name.to_string()
    }
}

/// Single-quoted Dart string literal with escaping (backslash, quote, and
/// `$` so no string interpolation can occur).
pub(crate) fn dart_string_literal(value: &str) -> String {
    let mut s = String::with_capacity(value.len() + 2);
    s.push('\'');
    for c in value.chars() {
        match c {
            '\\' => s.push_str("\\\\"),
            '\'' => s.push_str("\\'"),
            '$' => s.push_str("\\$"),
            _ => s.push(c),
        }
    }
    s.push('\'');
    s
}

/// Format an f64 as a Dart double literal (`8.0`, `0.5`, `12.75`).
fn f64_to_dart(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{:.1}", v)
    } else {
        format!("{}", v)
    }
}

/// Parse a CSS length token ("16px", "0", "-1px") as f64. Only px and
/// unitless values are supported.
fn css_length_f64(tok: &str) -> Option<f64> {
    let t = tok.strip_suffix("px").unwrap_or(tok);
    t.parse::<f64>().ok().filter(|v| v.is_finite())
}

/// "16px" / "0" → Dart double literal ("16.0" / "0.0"). Non-px values
/// (percentages, rem, …) return `None`.
pub(crate) fn css_length_to_dart(value: &str) -> Option<String> {
    css_length_f64(value.trim()).map(f64_to_dart)
}

/// Unitless number ("1.5") → Dart double literal.
fn css_unitless_double_to_dart(value: &str) -> Option<String> {
    value
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .map(f64_to_dart)
}

/// "150ms" / "0.3s" / "300" → `Duration(milliseconds: N)`.
fn css_duration_to_dart(value: &str) -> Option<String> {
    let v = value.trim();
    let (num, factor) = if let Some(ms) = v.strip_suffix("ms") {
        (ms, 1.0)
    } else if let Some(sec) = v.strip_suffix('s') {
        (sec, 1000.0)
    } else {
        (v, 1.0)
    };
    let ms = num.trim().parse::<f64>().ok()? * factor;
    Some(format!("Duration(milliseconds: {})", ms.round() as i64))
}

/// "500" / "bold" / "normal" → `FontWeight.w500` / `w700` / `w400`.
pub(crate) fn css_font_weight_to_dart(value: &str) -> Option<String> {
    match value.trim() {
        "normal" => Some("FontWeight.w400".to_string()),
        "bold" => Some("FontWeight.w700".to_string()),
        w => {
            let n = w.parse::<i64>().ok()?;
            if (100..=900).contains(&n) && n % 100 == 0 {
                Some(format!("FontWeight.w{}", n))
            } else {
                None
            }
        }
    }
}

/// "0.5" / "50%" → Dart double literal.
fn css_opacity_to_dart(value: &str) -> Option<String> {
    let v = value.trim();
    if let Some(pct) = v.strip_suffix('%') {
        let n = pct.parse::<f64>().ok()?;
        return Some(f64_to_dart(n / 100.0));
    }
    css_unitless_double_to_dart(v)
}

/// CSS easing name → Flutter `Curves` constant. `ease` maps to
/// `Curves.easeOut` per the Flutter backend convention.
fn css_easing_to_dart(value: &str) -> Option<String> {
    match value.trim() {
        "linear" => Some("Curves.linear".to_string()),
        "ease" => Some("Curves.easeOut".to_string()),
        "ease-in" | "easeIn" => Some("Curves.easeIn".to_string()),
        "ease-out" | "easeOut" => Some("Curves.easeOut".to_string()),
        "ease-in-out" | "easeInOut" => Some("Curves.easeInOut".to_string()),
        _ => None,
    }
}

/// Convert a CSS color value (hex / rgb() / rgba() / named) to a Dart
/// `Color(...)` expression. `var()` references must already be resolved.
fn css_color_expr(value: &str) -> Option<String> {
    let v = value.trim();
    css_color_to_dart(v).or_else(|| match v {
        "white" => Some("Color(0xFFFFFFFF)".to_string()),
        "black" => Some("Color(0xFF000000)".to_string()),
        "transparent" => Some("Color(0x00000000)".to_string()),
        _ => None,
    })
}

/// Color expression for a design value: whole `var(--token)` references
/// become typed `AppTokens.xxx` references; everything else resolves
/// `var()`s and parses the resulting CSS color.
pub(crate) fn design_color_expr(design: &DesignIR, value: &str) -> Option<String> {
    if let Some(tok_ref) = whole_var_token_ref(design, value) {
        return Some(tok_ref);
    }
    let resolved = resolve_css_vars(design, value)?;
    css_color_expr(&resolved)
}

/// If the whole value is exactly a `var(--token)` reference, return the
/// `AppTokens` constant reference for that token (a typed constant resolved
/// at compile time). `None` when the value is a compound expression or the
/// token's constant was not emitted.
pub(crate) fn whole_var_token_ref(design: &DesignIR, value: &str) -> Option<String> {
    let trimmed = value.trim();
    let name = trimmed.strip_prefix("var(--")?.strip_suffix(')')?;
    let token = design.tokens.get(name)?;
    if token_const_emitted(design, token) {
        Some(format!("{}Tokens.{}", design.name, escape_dart_ident(name)))
    } else {
        None
    }
}

/// Resolve `var(--name)` references to the referenced token's static CSS
/// value. Returns `None` when any referenced token is dynamic, missing, or
/// itself unresolvable (the depth limit guards against cycles).
pub(crate) fn resolve_css_vars(design: &DesignIR, value: &str) -> Option<String> {
    let mut result = value.to_string();
    // Depth limit: token values can reference other tokens via the const
    // fallback path; 8 levels is far beyond any real chain.
    for _ in 0..8 {
        let mut replaced = false;
        let mut next = String::with_capacity(result.len());
        let mut rest = result.as_str();
        while let Some(start) = rest.find("var(--") {
            next.push_str(&rest[..start]);
            let after = &rest[start + "var(--".len()..];
            let end = after.find(')')?;
            let name = &after[..end];
            let val = match design.tokens.get(name)?.value {
                TokenValueIR::Static(ref s) => s.clone(),
                TokenValueIR::Dynamic(_) => return None,
                #[allow(unreachable_patterns)]
                _ => return None,
            };
            next.push_str(&val);
            rest = &after[end + 1..];
            replaced = true;
        }
        next.push_str(rest);
        result = next;
        if !replaced {
            return Some(result);
        }
    }
    None
}

/// Whether a shadow token constant is a `List<BoxShadow>` (multiple shadows)
/// rather than a single `BoxShadow`.
pub(crate) fn token_shadow_is_list(design: &DesignIR, name: &str) -> Option<bool> {
    let token = design.tokens.get(name)?;
    let TokenValueIR::Static(v) = &token.value else {
        return None;
    };
    let resolved = resolve_css_vars(design, v)?;
    css_shadow_to_dart(&resolved).map(|(is_list, _)| is_list)
}

/// Split a CSS value on top-level commas (respecting parentheses so
/// `rgba(0,0,0,0.5)` is not split).
fn split_top_level_commas(value: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (i, c) in value.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                parts.push(&value[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(&value[start..]);
    parts
}

/// Parse one CSS shadow ("0 1px 2px rgba(0,0,0,0.05)") into a Dart
/// `BoxShadow(...)` const expression. `inset` shadows are not supported.
fn parse_single_shadow(value: &str) -> Option<String> {
    let mut numbers: Vec<f64> = Vec::new();
    let mut color_parts: Vec<&str> = Vec::new();
    for tok in value.split_whitespace() {
        if tok == "inset" {
            return None;
        }
        if let Some(n) = css_length_f64(tok) {
            numbers.push(n);
        } else {
            color_parts.push(tok);
        }
    }
    // offset-x, offset-y required; blurRadius and spreadRadius optional.
    if numbers.len() < 2 {
        return None;
    }
    let color = css_color_expr(&color_parts.join(" "))?;
    let offset_x = f64_to_dart(numbers[0]);
    let offset_y = f64_to_dart(numbers[1]);
    let blur = numbers.get(2).copied().unwrap_or(0.0);
    let spread = numbers.get(3).copied();
    match spread {
        Some(s) => Some(format!(
            "BoxShadow(color: {}, offset: Offset({}, {}), blurRadius: {}, spreadRadius: {})",
            color,
            offset_x,
            offset_y,
            f64_to_dart(blur),
            f64_to_dart(s)
        )),
        None => Some(format!(
            "BoxShadow(color: {}, offset: Offset({}, {}), blurRadius: {})",
            color,
            offset_x,
            offset_y,
            f64_to_dart(blur)
        )),
    }
}

/// Parse a CSS shadow list into `(is_list, Dart expression)`: a single
/// shadow → `(false, "BoxShadow(...)")`; several → `(true, "[…]")`.
pub(crate) fn css_shadow_to_dart(value: &str) -> Option<(bool, String)> {
    let parts: Vec<&str> = split_top_level_commas(value)
        .into_iter()
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    if parts.is_empty() {
        return None;
    }
    let shadows: Vec<String> = parts
        .iter()
        .map(|p| parse_single_shadow(p))
        .collect::<Option<_>>()?;
    if shadows.len() == 1 {
        Some((false, shadows[0].clone()))
    } else {
        Some((true, format!("[{}]", shadows.join(", "))))
    }
}

/// Parse a CSS border shorthand ("1px solid #d4d4d8") into a Dart
/// `Border.fromBorderSide(BorderSide(...))` const expression.
///
/// `Border.fromBorderSide` is used instead of `Border.all` because the
/// latter is not a const constructor — the border must be usable inside the
/// `static const` ThemeExtension branches.
pub(crate) fn css_border_to_dart(value: &str) -> Option<String> {
    let tokens: Vec<&str> = value.split_whitespace().collect();
    let mut width: Option<f64> = None;
    let mut color: Option<String> = None;
    let mut i = 0usize;
    while i < tokens.len() {
        let tok = tokens[i];
        if width.is_none() {
            if let Some(n) = css_length_f64(tok) {
                width = Some(n);
                i += 1;
                continue;
            }
        }
        if tok == "none" {
            return None;
        }
        // BorderStyle words are skipped; everything else is the color
        // (e.g. "1px solid #d4d4d8").
        if !matches!(
            tok,
            "solid" | "dashed" | "dotted" | "double" | "groove" | "ridge"
        ) {
            color = css_color_expr(&tokens[i..].join(" "));
            break;
        }
        i += 1;
    }
    match (width, color) {
        (Some(w), Some(c)) => Some(format!(
            "Border.fromBorderSide(BorderSide(color: {}, width: {}))",
            c,
            f64_to_dart(w)
        )),
        (None, Some(c)) => Some(format!("Border.fromBorderSide(BorderSide(color: {}))", c)),
        _ => None,
    }
}

/// Parse a CSS border-radius (1, 2 or 4 values) into a Dart `BorderRadius…`
/// const expression. CSS 4-value order is top-left, top-right, bottom-right,
/// bottom-left.
pub(crate) fn css_border_radius_to_dart(value: &str) -> Option<String> {
    let vals: Vec<f64> = value
        .split_whitespace()
        .filter_map(css_length_f64)
        .collect();
    match vals.as_slice() {
        [r] => Some(format!("BorderRadius.circular({})", f64_to_dart(*r))),
        [t, b] => Some(format!(
            "BorderRadius.vertical(top: Radius.circular({}), bottom: Radius.circular({}))",
            f64_to_dart(*t),
            f64_to_dart(*b)
        )),
        [tl, tr, br, bl] => Some(format!(
            "BorderRadius.only(topLeft: Radius.circular({}), topRight: Radius.circular({}), \
             bottomRight: Radius.circular({}), bottomLeft: Radius.circular({}))",
            f64_to_dart(*tl),
            f64_to_dart(*tr),
            f64_to_dart(*br),
            f64_to_dart(*bl)
        )),
        _ => None,
    }
}

/// Normalize a CSS property name to camelCase ("border-radius" →
/// "borderRadius"). Existing camelCase names pass through unchanged.
pub(crate) fn css_prop_to_camel(prop: &str) -> String {
    let mut out = String::with_capacity(prop.len());
    let mut upper_next = false;
    for c in prop.chars() {
        if c == '-' {
            upper_next = true;
        } else if upper_next {
            out.extend(c.to_uppercase());
            upper_next = false;
        } else {
            out.push(c);
        }
    }
    out
}
