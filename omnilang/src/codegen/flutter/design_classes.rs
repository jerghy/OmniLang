//! Flutter design codegen: BoxDecoration factories, decoration composition
//! and `ThemeExtension` emission (domain half of the former `design.rs`
//! monolith).
//!
//! - Step 2 — `{Design}Design`: a `BoxDecoration` factory per design class
//!   whose properties map onto decoration fields; `class:`/`variant:`
//!   decoration resolution at view-emit time (`DesignResolver`).
//! - Step 3 — `{Design}ThemeVariant`: a `ThemeExtension` per design (with
//!   variants): nullable fields per mappable variant property, `copyWith` +
//!   `lerp`, and a `static const` per variant branch value.

use super::capitalize_first;
use super::design_tokens::{
    css_border_radius_to_dart, css_border_to_dart, css_font_weight_to_dart, css_length_to_dart,
    css_prop_to_camel, css_shadow_to_dart, dart_string_literal, design_color_expr,
    escape_dart_ident, resolve_css_vars, token_shadow_is_list, whole_var_token_ref,
};
use super::utils::lower_first;
use crate::codegen::CodeWriter;
use crate::ir::design::*;
use crate::ir::IRProgram;
use rustc_hash::FxHashMap;
use rustc_hash::FxHashSet;

// Step 2 — {Design}Design: BoxDecoration factories
// ═══════════════════════════════════════════════════════════════

/// Emit the `{Design}Design` class: one `BoxDecoration` factory per design
/// class with at least one decoration-mappable property. Layout-only classes
/// (display, gap, padding, …) are skipped with a warning naming the class;
/// decoration properties whose VALUES cannot be mapped (spec §10.3) are
/// skipped with a warning naming the property and value.
pub(crate) fn emit_design_class(
    w: &mut CodeWriter,
    design: &DesignIR,
    warnings: &mut Vec<crate::codegen::CodegenWarning>,
) {
    let class_name = format!("{}Design", design.name);
    w.emit_fmt(format_args!(
        "/// BoxDecoration factories for the `{}` design classes.",
        design.name
    ));
    w.emit_fmt(format_args!("class {} {{", class_name));
    w.indent();
    w.emit_fmt(format_args!("{}._();", class_name));
    w.emit("");

    for class in &design.classes {
        let (fields, skipped) = resolve_class_decoration(design, class);
        for (prop, value) in &skipped {
            warnings.push(super::design::design_block_warning(
                design,
                format!(
                    "[flutter][design] design class '{}' property '{}' value '{}' is not \
                     mappable to a BoxDecoration field; skipped",
                    class.name, prop, value
                ),
            ));
        }
        if fields.is_empty() {
            warnings.push(super::design::design_block_warning(
                design,
                format!(
                    "[flutter][design] design class '{}' has no decoration-mappable \
                     properties; no BoxDecoration factory emitted (layout/text-only \
                     classes have no Flutter decoration equivalent)",
                    class.name
                ),
            ));
            continue;
        }
        let method = escape_dart_ident(&lower_first(&class.name));
        w.emit_fmt(format_args!("/// BoxDecoration for `{}`.", class.name));
        w.emit_fmt(format_args!("static BoxDecoration {}() {{", method));
        w.indent();
        w.emit("return BoxDecoration(");
        w.indent();
        for param in ["color", "borderRadius", "border", "boxShadow"] {
            if let Some(expr) = fields.get(param) {
                w.emit_fmt(format_args!("{}: {},", param, expr));
            }
        }
        w.dedent();
        w.emit(");");
        w.dedent();
        w.emit("}");
        w.emit("");
    }
    w.dedent();
    w.emit("}");
    w.emit("");
}

/// The decoration-mappable fields of one design class, keyed by BoxDecoration
/// parameter name. Only background, border-radius, border and box-shadow map
/// onto BoxDecoration; text colors, padding, margin, layout properties, … are
/// ignored. The second return value lists the (property, value) pairs whose
/// decoration mapping failed (unmappable values — never a silent drop).
pub(crate) fn resolve_class_decoration(
    design: &DesignIR,
    class: &ClassIR,
) -> (DecorationFields, Vec<(String, String)>) {
    let mut out = DecorationFields::default();
    let mut skipped: Vec<(String, String)> = Vec::new();
    for (prop, value) in &class.properties {
        match css_prop_to_camel(prop).as_str() {
            "background" | "backgroundColor" => {
                if let Some(expr) = design_color_expr(design, value) {
                    out.entry("color").or_insert(expr);
                } else {
                    skipped.push((prop.clone(), value.clone()));
                }
            }
            "borderRadius" => {
                if let Some(expr) = design_border_radius_expr(design, value) {
                    out.insert("borderRadius", expr);
                } else {
                    skipped.push((prop.clone(), value.clone()));
                }
            }
            "border" => {
                // CSS `none` = the BoxDecoration default (no border) — an
                // honest skip, not an unmappable value.
                if value.trim() == "none" {
                    continue;
                }
                if let Some(expr) = design_border_expr(design, value) {
                    out.insert("border", expr);
                } else {
                    skipped.push((prop.clone(), value.clone()));
                }
            }
            "boxShadow" => {
                // CSS `none` = the BoxDecoration default (no shadow).
                if value.trim() == "none" {
                    continue;
                }
                if let Some(expr) = design_box_shadow_expr(design, value) {
                    out.insert("boxShadow", expr);
                } else {
                    skipped.push((prop.clone(), value.clone()));
                }
            }
            _ => {}
        }
    }
    (out, skipped)
}

/// The `AppDesign.<class>()` factory expression for a single class — `Some`
/// only when the class has at least one decoration-mappable property (the
/// factory is skipped otherwise, so the reference must not be emitted).
pub(crate) fn class_factory_expr(design: &DesignIR, class: &ClassIR) -> Option<String> {
    let (fields, _) = resolve_class_decoration(design, class);
    if fields.is_empty() {
        None
    } else {
        Some(format!(
            "{}Design.{}()",
            design.name,
            escape_dart_ident(&lower_first(&class.name))
        ))
    }
}

/// BoxDecoration `borderRadius` value: whole `var(--token)` references map to
/// `BorderRadius.circular(AppTokens.radiusLg)`; literals are parsed.
fn design_border_radius_expr(design: &DesignIR, value: &str) -> Option<String> {
    if let Some(tok_ref) = whole_var_token_ref(design, value) {
        return Some(format!("BorderRadius.circular({})", tok_ref));
    }
    let resolved = resolve_css_vars(design, value)?;
    css_border_radius_to_dart(&resolved)
}

/// BoxDecoration `border` value: compound CSS shorthands ("1px solid
/// #d4d4d8") parse into `Border.all(...)`. There is no border token type, so
/// `var()` references are resolved inline rather than via `AppTokens`.
fn design_border_expr(design: &DesignIR, value: &str) -> Option<String> {
    let resolved = resolve_css_vars(design, value)?;
    css_border_to_dart(&resolved)
}

/// BoxDecoration `boxShadow` value — always a list expression:
/// `[AppTokens.shadowSm]` for single-shadow tokens, `AppTokens.shadowX` for
/// multi-shadow tokens, `[BoxShadow(...), …]` for literals.
fn design_box_shadow_expr(design: &DesignIR, value: &str) -> Option<String> {
    if let Some(tok_ref) = whole_var_token_ref(design, value) {
        let name = value.trim().strip_prefix("var(--")?.strip_suffix(')')?;
        return match token_shadow_is_list(design, name) {
            Some(true) => Some(tok_ref),
            Some(false) => Some(format!("[{}]", tok_ref)),
            None => None,
        };
    }
    let resolved = resolve_css_vars(design, value)?;
    let (is_list, expr) = css_shadow_to_dart(&resolved)?;
    Some(if is_list { expr } else { format!("[{}]", expr) })
}

// ═══════════════════════════════════════════════════════════════
// Decoration composition — class:/variant: resolution at view-emit time
// ═══════════════════════════════════════════════════════════════

/// Decoration-mappable fields of a class/variant merge, keyed by
/// `BoxDecoration` parameter name ("color", "borderRadius", "border",
/// "boxShadow").
pub(crate) type DecorationFields = FxHashMap<&'static str, String>;

/// The Dart expression for a `BoxDecoration` built from resolved fields.
pub(crate) fn decoration_fields_to_expr(fields: &DecorationFields) -> String {
    let mut args: Vec<String> = Vec::new();
    for param in ["color", "borderRadius", "border", "boxShadow"] {
        if let Some(expr) = fields.get(param) {
            args.push(format!("{}: {}", param, expr));
        }
    }
    format!("BoxDecoration({})", args.join(", "))
}

/// Compose two decorations field-wise — later fields win on conflict. This
/// mirrors the CSS cascade for multiple classes on one element: later rules
/// in source order win per property.
pub(crate) fn merge_decoration_fields(
    mut base: DecorationFields,
    over: DecorationFields,
) -> DecorationFields {
    for (k, v) in over {
        base.insert(k, v);
    }
    base
}

/// The decoration-mappable fields of a variant binding: the variant's base
/// properties merged with every branch whose value is selected by the binding
/// (later properties override earlier ones — CSS cascade). `dimension_values`
/// supplies explicit `dim=value` pairs; missing dimensions fall back to the
/// variant's declared defaults; values not declared on the dimension are
/// ignored (mirrors `shared_design::expand_variant`). The second return value
/// lists the (property, value) pairs whose decoration mapping failed.
pub(crate) fn resolve_variant_decoration(
    design: &DesignIR,
    variant: &VariantIR,
    dimension_values: &FxHashMap<String, String>,
) -> (DecorationFields, Vec<(String, String)>) {
    // Selected branch values: explicit bindings first, then defaults; only
    // values declared on the dimension apply.
    let mut selected: FxHashSet<&str> = FxHashSet::default();
    for dim in &variant.dimensions {
        let value = dimension_values
            .get(&dim.name)
            .or_else(|| variant.defaults.get(&dim.name));
        if let Some(value) = value {
            if dim.values.iter().any(|v| v == value) {
                selected.insert(value.as_str());
            }
        }
    }
    // Merge base + matching branches (later overrides earlier).
    let mut props: FxHashMap<String, String> = variant
        .base
        .iter()
        .map(|(k, v)| (css_prop_to_camel(k), v.clone()))
        .collect();
    for branch in &variant.branches {
        if selected.contains(branch.value.as_str()) {
            for (k, v) in &branch.properties {
                props.insert(css_prop_to_camel(k), v.clone());
            }
        }
    }

    let mut out = DecorationFields::default();
    let mut skipped: Vec<(String, String)> = Vec::new();
    for (prop, value) in &props {
        match prop.as_str() {
            "background" | "backgroundColor" => {
                if let Some(expr) = design_color_expr(design, value) {
                    out.insert("color", expr);
                } else {
                    skipped.push((prop.clone(), value.clone()));
                }
            }
            "borderRadius" => {
                if let Some(expr) = design_border_radius_expr(design, value) {
                    out.insert("borderRadius", expr);
                } else {
                    skipped.push((prop.clone(), value.clone()));
                }
            }
            "border" => {
                if value.trim() == "none" {
                    continue; // CSS default — no border
                }
                if let Some(expr) = design_border_expr(design, value) {
                    out.insert("border", expr);
                } else {
                    skipped.push((prop.clone(), value.clone()));
                }
            }
            "boxShadow" => {
                if value.trim() == "none" {
                    continue; // CSS default — no shadow
                }
                if let Some(expr) = design_box_shadow_expr(design, value) {
                    out.insert("boxShadow", expr);
                } else {
                    skipped.push((prop.clone(), value.clone()));
                }
            }
            _ => {}
        }
    }
    (out, skipped)
}

/// Pre-computed design lookups for view-emit time class:/variant: resolution.
///
/// Class names arrive at codegen bare (the `Design.` prefix is dropped during
/// lowering — `class: LearnDesign.Card` becomes the `class` prop "Card"), so
/// the owning design is resolved here by class name: the FIRST design (sorted
/// by name) declaring the class wins when several designs share a class name.
pub(crate) struct DesignResolver<'a> {
    design_by_name: FxHashMap<&'a str, &'a DesignIR>,
    /// Class name → (owning design, class).
    class_owner: FxHashMap<&'a str, (&'a DesignIR, &'a ClassIR)>,
}

impl<'a> DesignResolver<'a> {
    pub(crate) fn new(ir: &'a IRProgram) -> Self {
        let mut design_by_name = FxHashMap::default();
        let mut class_owner = FxHashMap::default();
        let mut designs: Vec<&DesignIR> = ir.modules.values().flat_map(|m| &m.designs).collect();
        designs.sort_by(|a, b| a.name.cmp(&b.name));
        for design in designs {
            design_by_name.insert(design.name.as_str(), design);
            for class in &design.classes {
                class_owner
                    .entry(class.name.as_str())
                    .or_insert((design, class));
            }
        }
        Self {
            design_by_name,
            class_owner,
        }
    }

    /// The owning design + class for a bare class name, if any design
    /// declares it.
    pub(crate) fn class_owner(&self, class_name: &str) -> Option<(&'a DesignIR, &'a ClassIR)> {
        self.class_owner.get(class_name).copied()
    }

    /// A design by block name.
    pub(crate) fn design(&self, name: &str) -> Option<&'a DesignIR> {
        self.design_by_name.get(name).copied()
    }

    /// A variant by name within a design.
    pub(crate) fn variant(&self, design: &'a DesignIR, name: &str) -> Option<&'a VariantIR> {
        design.variants.iter().find(|v| v.name == name)
    }

    /// The `{Design}Tokens.<name>` constant reference for a design token,
    /// across ALL designs (the first design declaring the token wins).
    /// `None` when no design declares it or its constant was not emitted.
    /// Task 5.8: motion keyframe color stops referencing `var(--textPrimary)`
    /// previously emitted the raw string — String where Color was expected.
    pub(crate) fn token_constant_ref(&self, token_name: &str) -> Option<String> {
        for design in self.design_by_name.values() {
            if let Some(r) = whole_var_token_ref(design, &format!("var(--{token_name})")) {
                return Some(r);
            }
        }
        None
    }
}

// ═══════════════════════════════════════════════════════════════
// Step 3 — {Design}ThemeVariant: ThemeExtension + branch consts
// ═══════════════════════════════════════════════════════════════

/// The Dart kind of a ThemeExtension field, driving the annotation and the
/// `lerp` strategy (Color/double/EdgeInsets lerp; the rest pick a side).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ThemeFieldKind {
    Color,
    Double,
    FontWeight,
    Border,
    ShadowList,
    String,
    EdgeInsets,
}

/// One ThemeExtension field derived from the design's variant properties.
#[derive(Debug, Clone)]
struct ThemeField {
    /// Dart field name.
    name: String,
    kind: ThemeFieldKind,
    /// CamelCase CSS property keys contributing to this field (all four
    /// padding-* sides merge into `padding`).
    props: Vec<String>,
}

/// Union of mappable variant properties (base + branches) across all
/// variants of a design, in deterministic (sorted) field order.
fn collect_theme_fields(design: &DesignIR) -> Vec<ThemeField> {
    let mut by_name: FxHashMap<String, ThemeField> = FxHashMap::default();
    for variant in &design.variants {
        for (prop, _) in variant
            .base
            .iter()
            .chain(variant.branches.iter().flat_map(|b| b.properties.iter()))
        {
            let key = css_prop_to_camel(prop);
            let mapped = match key.as_str() {
                "background" => Some(("background", ThemeFieldKind::Color)),
                "color" => Some(("color", ThemeFieldKind::Color)),
                "borderRadius" => Some(("borderRadius", ThemeFieldKind::Double)),
                "border" => Some(("border", ThemeFieldKind::Border)),
                "boxShadow" => Some(("boxShadow", ThemeFieldKind::ShadowList)),
                "fontSize" => Some(("fontSize", ThemeFieldKind::Double)),
                "fontWeight" => Some(("fontWeight", ThemeFieldKind::FontWeight)),
                "fontFamily" => Some(("fontFamily", ThemeFieldKind::String)),
                "paddingTop" | "paddingRight" | "paddingBottom" | "paddingLeft" => {
                    Some(("padding", ThemeFieldKind::EdgeInsets))
                }
                _ => None,
            };
            let Some((name, kind)) = mapped else {
                continue;
            };
            by_name
                .entry(name.to_string())
                .or_insert_with(|| ThemeField {
                    name: name.to_string(),
                    kind,
                    props: Vec::new(),
                })
                .props
                .push(key);
        }
    }
    let mut fields: Vec<ThemeField> = by_name.into_values().collect();
    fields.sort_by(|a, b| a.name.cmp(&b.name));
    fields
}

fn theme_field_annotation(kind: ThemeFieldKind) -> &'static str {
    match kind {
        ThemeFieldKind::Color => "Color",
        ThemeFieldKind::Double => "double",
        ThemeFieldKind::FontWeight => "FontWeight",
        ThemeFieldKind::Border => "Border",
        ThemeFieldKind::ShadowList => "List<BoxShadow>",
        ThemeFieldKind::String => "String",
        ThemeFieldKind::EdgeInsets => "EdgeInsets",
    }
}

/// Merge the base properties with every branch whose value matches `value`
/// (later properties override earlier ones). Keys are normalized to camelCase
/// so lookups are consistent regardless of source spelling.
fn merged_branch_props(variant: &VariantIR, value: &str) -> FxHashMap<String, String> {
    let mut props: FxHashMap<String, String> = variant
        .base
        .iter()
        .map(|(k, v)| (css_prop_to_camel(k), v.clone()))
        .collect();
    for branch in &variant.branches {
        if branch.value == value {
            for (k, v) in &branch.properties {
                props.insert(css_prop_to_camel(k), v.clone());
            }
        }
    }
    props
}

/// Format one ThemeExtension field value from a merged (base ∪ branches)
/// property map. Returns `None` when the property is absent or unmappable;
/// unmappable (property, value) pairs are recorded on `skips` so the caller
/// can surface a diagnostic (spec §10.3 — no silent drops).
fn theme_field_value(
    design: &DesignIR,
    field: &ThemeField,
    props: &FxHashMap<String, String>,
    skips: &mut Vec<(String, String)>,
) -> Option<String> {
    // Records the failing (property, value) for the warning channel.
    let fail = |skips: &mut Vec<(String, String)>| -> Option<String> {
        if let Some(prop) = field.props.last() {
            if let Some(value) = props.get(prop.as_str()) {
                skips.push((prop.clone(), value.clone()));
            }
        }
        None
    };
    match field.kind {
        ThemeFieldKind::Color => {
            let value = field.props.iter().find_map(|p| props.get(p))?;
            design_color_expr(design, value).or_else(|| fail(skips))
        }
        ThemeFieldKind::Double => {
            let value = field.props.iter().find_map(|p| props.get(p))?;
            if let Some(tok_ref) = whole_var_token_ref(design, value) {
                Some(tok_ref)
            } else {
                let resolved = resolve_css_vars(design, value)?;
                css_length_to_dart(&resolved).or_else(|| fail(skips))
            }
        }
        ThemeFieldKind::FontWeight => {
            let value = field.props.iter().find_map(|p| props.get(p))?;
            if let Some(tok_ref) = whole_var_token_ref(design, value) {
                Some(tok_ref)
            } else {
                let resolved = resolve_css_vars(design, value)?;
                css_font_weight_to_dart(&resolved).or_else(|| fail(skips))
            }
        }
        ThemeFieldKind::Border => {
            let value = field.props.iter().find_map(|p| props.get(p))?;
            if value.trim() == "none" {
                return None; // CSS default — field stays null (no border)
            }
            let resolved = resolve_css_vars(design, value)?;
            css_border_to_dart(&resolved).or_else(|| fail(skips))
        }
        ThemeFieldKind::ShadowList => {
            let value = field.props.iter().find_map(|p| props.get(p))?;
            if value.trim() == "none" {
                return None; // CSS default — field stays null (no shadow)
            }
            if let Some(tok_ref) = whole_var_token_ref(design, value) {
                let name = value.trim().strip_prefix("var(--")?.strip_suffix(')')?;
                match token_shadow_is_list(design, name) {
                    Some(true) => Some(tok_ref),
                    Some(false) => Some(format!("[{}]", tok_ref)),
                    None => fail(skips),
                }
            } else {
                let resolved = resolve_css_vars(design, value)?;
                let (is_list, expr) = css_shadow_to_dart(&resolved)?;
                Some(if is_list { expr } else { format!("[{}]", expr) })
            }
        }
        ThemeFieldKind::String => {
            let value = field.props.iter().find_map(|p| props.get(p))?;
            if let Some(tok_ref) = whole_var_token_ref(design, value) {
                Some(tok_ref)
            } else {
                let resolved = resolve_css_vars(design, value)?;
                Some(dart_string_literal(resolved.trim()))
            }
        }
        ThemeFieldKind::EdgeInsets => {
            // All four sides merge into one EdgeInsets; missing sides
            // default to 0.0. When no side is present at all the field is
            // omitted from the const. Whole var() references stay typed
            // references.
            let mut args: [Option<String>; 4] = [None, None, None, None];
            for (prop, idx) in [
                ("paddingLeft", 0usize),
                ("paddingTop", 1usize),
                ("paddingRight", 2usize),
                ("paddingBottom", 3usize),
            ] {
                let Some(value) = props.get(prop) else {
                    continue;
                };
                if let Some(tok_ref) = whole_var_token_ref(design, value) {
                    args[idx] = Some(tok_ref);
                    continue;
                }
                let resolved = resolve_css_vars(design, value)?;
                args[idx] = css_length_to_dart(&resolved).or_else(|| fail(skips));
            }
            if args.iter().all(Option::is_none) {
                return None;
            }
            let arg = |i: usize| args[i].clone().unwrap_or_else(|| "0.0".to_string());
            Some(format!(
                "EdgeInsets.fromLTRB({}, {}, {}, {})",
                arg(0),
                arg(1),
                arg(2),
                arg(3)
            ))
        }
    }
}

/// Static const branch names: one per variant branch value in declaration
/// order. A value used on two dimensions (or colliding with a field name) is
/// disambiguated with the dimension suffix, then the variant suffix.
fn branch_const_names(design: &DesignIR, fields: &[ThemeField]) -> Vec<(String, String)> {
    // (variant, dim, value) triples in declaration order.
    let triples: Vec<(&str, &str, &str)> = design
        .variants
        .iter()
        .flat_map(|v| {
            v.branches
                .iter()
                .map(|b| (v.name.as_str(), b.dimension.as_str(), b.value.as_str()))
        })
        .collect();
    let mut by_value: FxHashMap<&str, Vec<usize>> = FxHashMap::default();
    for (i, (_, _, value)) in triples.iter().enumerate() {
        by_value.entry(value).or_default().push(i);
    }
    let mut taken: FxHashSet<String> = fields.iter().map(|f| f.name.clone()).collect();
    let mut result: Vec<(String, String)> = Vec::with_capacity(triples.len());
    for (variant, dim, value) in triples {
        let unique = by_value[value].len() == 1 && !taken.contains(value);
        let name = if unique {
            (*value).to_string()
        } else {
            let mut candidate = format!("{}{}", value, capitalize_first(dim));
            if taken.contains(&candidate) {
                candidate = format!(
                    "{}{}{}",
                    value,
                    capitalize_first(variant),
                    capitalize_first(dim)
                );
            }
            candidate
        };
        taken.insert(name.clone());
        result.push((name, (*value).to_string()));
    }
    result
}

/// Emit the `{Design}ThemeVariant` ThemeExtension: nullable fields per
/// mappable variant property, `copyWith`, `lerp`, and one `static const` per
/// variant branch value. Branch property values that cannot be mapped onto a
/// field are skipped with a warning (spec §10.3).
pub(crate) fn emit_theme_variant_class(
    w: &mut CodeWriter,
    design: &DesignIR,
    warnings: &mut Vec<crate::codegen::CodegenWarning>,
) {
    if design.variants.is_empty() {
        return;
    }
    let fields = collect_theme_fields(design);
    if fields.is_empty() {
        return;
    }
    let class_name = format!("{}ThemeVariant", design.name);
    let ann = |kind: ThemeFieldKind| theme_field_annotation(kind);

    w.emit_fmt(format_args!(
        "/// Theme extension for the variant(s) of the `{}` design block.",
        design.name
    ));
    w.emit_fmt(format_args!(
        "class {} extends ThemeExtension<{}> {{",
        class_name, class_name
    ));
    w.indent();
    w.emit_fmt(format_args!("const {}({{", class_name));
    w.indent();
    for f in &fields {
        w.emit_fmt(format_args!("this.{},", f.name));
    }
    w.dedent();
    w.emit("});");
    w.emit("");
    for f in &fields {
        w.emit_fmt(format_args!("final {}? {};", ann(f.kind), f.name));
    }
    w.emit("");

    // copyWith
    w.emit("@override");
    w.emit_fmt(format_args!("{} copyWith({{", class_name));
    w.indent();
    for f in &fields {
        w.emit_fmt(format_args!("{}? {},", ann(f.kind), f.name));
    }
    w.dedent();
    w.emit("}) {");
    w.indent();
    w.emit_fmt(format_args!("return {}(", class_name));
    w.indent();
    for f in &fields {
        w.emit_fmt(format_args!("{}: {} ?? this.{},", f.name, f.name, f.name));
    }
    w.dedent();
    w.emit(");");
    w.dedent();
    w.emit("}");
    w.emit("");

    // lerp
    w.emit("@override");
    w.emit_fmt(format_args!(
        "{} lerp(ThemeExtension<{}>? other, double t) {{",
        class_name, class_name
    ));
    w.indent();
    w.emit_fmt(format_args!("if (other is! {}) {{", class_name));
    w.indent();
    w.emit("return this;");
    w.dedent();
    w.emit("}");
    w.emit_fmt(format_args!("return {}(", class_name));
    w.indent();
    for f in &fields {
        let expr = match f.kind {
            ThemeFieldKind::Color => format!("Color.lerp({}, other.{}, t)", f.name, f.name),
            ThemeFieldKind::Double => format!("lerpDouble({}, other.{}, t)", f.name, f.name),
            ThemeFieldKind::EdgeInsets => {
                format!("EdgeInsets.lerp({}, other.{}, t)", f.name, f.name)
            }
            _ => format!("t < 0.5 ? {} : other.{}", f.name, f.name),
        };
        w.emit_fmt(format_args!("{}: {},", f.name, expr));
    }
    w.dedent();
    w.emit(");");
    w.dedent();
    w.emit("}");
    w.emit("");

    // Static const branches for the variant dimensions.
    w.emit("// Static const branches for the variant dimensions:");
    let names = branch_const_names(design, &fields);
    let mut idx = 0usize;
    for variant in &design.variants {
        for branch in &variant.branches {
            let (const_name, _) = &names[idx];
            idx += 1;
            let merged = merged_branch_props(variant, &branch.value);
            let mut skips: Vec<(String, String)> = Vec::new();
            w.emit_fmt(format_args!(
                "static const {} {} = {}(",
                class_name, const_name, class_name
            ));
            w.indent();
            for f in &fields {
                if let Some(value) = theme_field_value(design, f, &merged, &mut skips) {
                    w.emit_fmt(format_args!("{}: {},", f.name, value));
                }
            }
            w.dedent();
            w.emit(");");
            for (prop, value) in &skips {
                warnings.push(super::design::design_block_warning(
                    design,
                    format!(
                        "[flutter][design] variant '{}' branch '{}' property '{}' value \
                         '{}' is not mappable to a ThemeExtension field; skipped",
                        variant.name, branch.value, prop, value
                    ),
                ));
            }
        }
    }
    w.dedent();
    w.emit("}");
    w.emit("");
}
