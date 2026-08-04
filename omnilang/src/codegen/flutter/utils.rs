// ═══════════════════════════════════════════════════════════════
// Flutter-specific helpers
// ═══════════════════════════════════════════════════════════════

use super::capitalize_first;
use crate::ir::ExprIR;

/// Lowercase the first character of a string ("TodoStore" → "todoStore").
pub(crate) fn lower_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(first) => first.to_lowercase().to_string() + c.as_str(),
    }
}

/// Map OmniView HTML tag names to Flutter widget class names.
pub(crate) fn tag_to_flutter_widget(tag: &str) -> &str {
    match tag {
        "div" | "container" => "Container",
        "span" | "p" | "text" | "label" => "Text",
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => "Text",
        "button" => "ElevatedButton",
        "a" => "TextButton",
        "input" => "TextField",
        "img" | "image" => "Image.network",
        "row" => "Row",
        "column" => "Column",
        "stack" => "Stack",
        "center" => "Center",
        "padding" | "pad" => "Padding",
        "icon" => "Icon",
        "list" | "listview" => "ListView",
        "expanded" => "Expanded",
        "sizedbox" | "spacer" => "SizedBox",
        "card" => "Card",
        "flex" | "flexible" => "Flexible",
        "scaffold" => "Scaffold",
        "appbar" | "app_bar" => "AppBar",
        "iconbutton" | "icon_button" => "IconButton",
        "checkbox" => "Checkbox",
        "switch" => "Switch",
        "slider" => "Slider",
        "dropdown" | "dropdownbutton" | "dropdown_button" => "DropdownButton",
        // Task 3.5: `<textarea>` is a first-class multiline intent — the
        // Flutter mapping is `TextField(maxLines: 4)` (the maxLines default
        // is applied in view.rs format_element, where per-element defaults
        // live; plain `<input>` stays single-line).
        "textarea" | "textfield" => "TextField",
        "form" => "Form",
        "scrollview" | "singlechildscrollview" => "SingleChildScrollView",
        _ => "Container",
    }
}

/// Map OmniView event names to Flutter callback parameter names.
///
/// Alias resolution happens in the ONE shared table
/// (`shared::event::canonical_event_name`) — this adapter only applies the
/// Flutter callback-name convention on top of the canonical DOM name.
pub(crate) fn event_to_flutter_callback(event_name: &str) -> String {
    match crate::codegen::shared::event::canonical_event_name(event_name) {
        Some(canonical) => flutter_callback_for_canonical(canonical),
        // Unknown / custom event — legacy fallback: keep the name unchanged.
        None => event_name.to_string(),
    }
}

/// Flutter widget callback parameter name for a canonical DOM event name.
fn flutter_callback_for_canonical(canonical: &str) -> String {
    match canonical {
        "click" => "onPressed".into(),
        "change" | "input" | "toggle" => "onChanged".into(),
        "submit" => "onSubmitted".into(),
        "focus" => "onFocus".into(),
        "blur" => "onBlur".into(),
        "keydown" => "onKeyDown".into(),
        "keyup" => "onKeyUp".into(),
        "longpress" => "onLongPress".into(),
        "dblclick" => "onDoubleTap".into(),
        "hover" | "mouseenter" => "onHover".into(),
        "scroll" => "onScroll".into(),
        "drag" | "dragstart" => "onDragStart".into(),
        "dragend" => "onDragEnd".into(),
        "dragover" => "onDrag".into(),
        "drop" => "onDrop".into(),
        "select" => "onSelected".into(),
        other => format!("on{}", capitalize_first(other)),
    }
}

/// Map OmniView HTML attribute names to Flutter widget constructor parameter names.
pub(crate) fn prop_to_flutter_param_name(prop: &str) -> &str {
    match prop {
        "class" | "className" | "id" | "style" => "",
        "src" => "image",
        "alt" => "semanticLabel",
        "href" => "url",
        "placeholder" => "hintText",
        // Task 3.5: the DOM boolean `disabled` maps to Flutter's `enabled`
        // param — the value is NEGATED in convert_literal_prop_value
        // (`disabled` → `enabled: false`). Widgets whose SDK constructor has
        // no `enabled` param (Checkbox/Switch/Slider/ElevatedButton/...) fall
        // out of the whitelist and warn — honest degradation, no silent drop.
        "disabled" => "enabled",
        "readonly" | "readOnly" => "",
        "value" => "",
        "checked" => "value",
        "label" => "label",
        "name" => "",
        "type" => "",
        "key" => "key",
        "width" => "width",
        "height" => "height",
        "padding" => "padding",
        "margin" => "margin",
        "alignment" => "alignment",
        "color" => "color",
        "background" | "backgroundColor" | "bgcolor" => "color",
        "opacity" => "opacity",
        "borderradius" | "borderRadius" => "borderRadius",
        "fontsize" | "fontSize" => "fontSize",
        "fontweight" | "fontWeight" => "fontWeight",
        "textalign" | "textAlign" => "textAlign",
        "maxlines" | "maxLines" => "maxLines",
        "overflow" => "overflow",
        "fit" => "fit",
        _ => prop,
    }
}

/// Map OmniView binding names to Flutter widget constructor parameter names.
pub(crate) fn binding_to_flutter_param(bind_name: &str) -> &str {
    match bind_name {
        "value" => "value",
        "text" => "text",
        "checked" => "value",
        _ => bind_name,
    }
}

/// Task 3.3: the `onChanged` write-back closure for a non-ref `bind:value`.
///
/// `(v) { setState(() { signal = v; }); }` — user input flows back into the
/// component signal (the two-way binding the reference form gets through its
/// guarded controller listener). Checkbox/Switch callbacks are
/// `ValueChanged<bool?>`: the nullable `v` cannot be assigned to the bool
/// signal directly, so the unreachable null case falls back to the current
/// value (`v ?? signal`).
pub(crate) fn bind_value_write_back(widget: &str, signal: &str) -> String {
    if matches!(widget, "Checkbox" | "Switch" | "DropdownButton") {
        format!(
            "(v) {{ setState(() {{ {} = v ?? {}; }}); }}",
            signal, signal
        )
    } else {
        format!("(v) {{ setState(() {{ {signal} = v; }}); }}")
    }
}

// ═══════════════════════════════════════════════════════════════
// CSS color conversion (C5)
// ═══════════════════════════════════════════════════════════════

/// Convert a CSS hex color literal to a Dart `Color(0xAARRGGBB)` expression.
///
/// Supports 3-digit (`#f00`) and 6-digit (`#ff0000`) forms:
/// - `#ff0000` → `Color(0xFFFF0000)`
/// - `#f00`   → `Color(0xFFFF0000)`
///
/// Returns `None` for non-hex values (caller decides fallback).
pub(crate) fn css_hex_to_dart_color(hex: &str) -> Option<String> {
    let trimmed = hex.trim();
    let digits = trimmed.strip_prefix('#')?;
    let expanded = match digits.len() {
        3 => digits
            .chars()
            .map(|c| format!("{c}{c}"))
            .collect::<String>(),
        6 => digits.to_string(),
        _ => return None,
    };
    if !expanded.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some(format!("Color(0xFF{})", expanded.to_uppercase()))
}

/// Convert a CSS color value (hex or rgb()/rgba()) to a Dart Color expression.
/// Returns `None` for unsupported formats.
pub(crate) fn css_color_to_dart(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if let Some(hex) = css_hex_to_dart_color(trimmed) {
        return Some(hex);
    }
    // rgb(r, g, b) / rgba(r, g, b, a) — r/g/b in 0-255 or percentages
    let (prefix, rest) = if let Some(rest) = trimmed.strip_prefix("rgb(") {
        ("", rest)
    } else if let Some(rest) = trimmed.strip_prefix("rgba(") {
        ("a", rest)
    } else {
        return None;
    };
    let inner = rest.strip_suffix(')')?;
    let parts: Vec<&str> = inner.split(',').map(|p| p.trim()).collect();
    if parts.len() != 3 && parts.len() != 4 {
        return None;
    }
    let to_channel = |p: &str| -> Option<String> {
        if let Some(pct) = p.strip_suffix('%') {
            let v = pct.parse::<f64>().ok()? * 255.0 / 100.0;
            return Some(format!("{}", v.round() as i64));
        }
        p.parse::<i64>().ok().map(|v| v.to_string())
    };
    let r = to_channel(parts[0])?;
    let g = to_channel(parts[1])?;
    let b = to_channel(parts[2])?;
    if prefix == "a" {
        let a = parts[3].parse::<f64>().ok()?;
        let alpha = (a * 255.0).round() as i64;
        Some(format!("Color.fromARGB({}, {}, {}, {})", alpha, r, g, b))
    } else {
        Some(format!("Color.fromARGB(255, {}, {}, {})", r, g, b))
    }
}

// ═══════════════════════════════════════════════════════════════
// Per-widget parameter whitelist (C6)
// ═══════════════════════════════════════════════════════════════

/// Flutter widget constructor parameter whitelist (C6).
///
/// Props/bindings/events that map to a parameter NOT in this list are skipped
/// with a compiler warning instead of silently emitting invalid Dart.
fn widget_param_whitelist(widget: &str) -> &'static [&'static str] {
    match widget {
        "Container" => &[
            "key",
            "padding",
            "margin",
            "width",
            "height",
            "color",
            "alignment",
            "constraints",
            "decoration",
            "foregroundDecoration",
            "transform",
            "clipBehavior",
            "child",
        ],
        "Text" => &[
            "key",
            "textAlign",
            "maxLines",
            "overflow",
            "style",
            "softWrap",
            "textScaleFactor",
            "semanticsLabel",
            "textDirection",
            "strutStyle",
        ],
        "Row" | "Column" => &[
            "key",
            "mainAxisAlignment",
            "crossAxisAlignment",
            "mainAxisSize",
            "textDirection",
            "verticalDirection",
            "children",
        ],
        "Stack" => &["key", "alignment", "fit", "clipBehavior", "children"],
        "Center" => &["key", "widthFactor", "heightFactor", "child"],
        "Padding" => &["key", "padding", "child"],
        // NOTE: `image` is deliberately NOT whitelisted — Image.network takes
        // the URL as a POSITIONAL argument (the `src` prop), and
        // `Image.network(image: ...)` is invalid Dart (T5.7 / review HIGH-4).
        "Icon" => &["key", "size", "color", "semanticLabel"],
        "Image.network" => &[
            "key",
            "fit",
            "width",
            "height",
            "alignment",
            "semanticLabel",
            "errorBuilder",
            "loadingBuilder",
        ],
        "ListView" => &[
            "key",
            "children",
            "itemCount",
            "itemBuilder",
            "padding",
            "scrollDirection",
            "controller",
        ],
        "Expanded" => &["key", "flex", "child"],
        "Flexible" => &["key", "flex", "child", "fit"],
        "SizedBox" => &["key", "width", "height", "child"],
        "Card" => &[
            "key",
            "child",
            "color",
            "elevation",
            "shape",
            "margin",
            "clipBehavior",
        ],
        "Scaffold" => &[
            "key",
            "appBar",
            "body",
            "floatingActionButton",
            "backgroundColor",
        ],
        "AppBar" => &["key", "title", "actions", "leading", "backgroundColor"],
        "ElevatedButton" | "TextButton" => &["key", "onPressed", "child", "style"],
        "IconButton" => &["key", "onPressed", "icon", "tooltip", "color"],
        "TextField" => &[
            "key",
            "onChanged",
            "onSubmitted",
            "controller",
            "focusNode",
            "obscureText",
            "keyboardType",
            "enabled",
            "maxLines",
            "textInputAction",
            "decoration",
        ],
        "Checkbox" => &["key", "value", "onChanged", "activeColor"],
        "Switch" => &["key", "value", "onChanged", "activeColor"],
        "Slider" => &["key", "value", "onChanged", "min", "max", "divisions"],
        // Task 3.5: the SDK declares `enabled` on DropdownButton (unlike
        // Checkbox/Switch/Slider — verified Flutter 3.44.1).
        "DropdownButton" => &["key", "value", "onChanged", "items", "hint", "enabled"],
        "Form" => &["key", "child", "onWillPop"],
        "SingleChildScrollView" => &["key", "child", "scrollDirection", "padding", "controller"],
        _ => &["key"],
    }
}

/// Flutter widget event callback whitelist (C6).
/// Events that map to a callback NOT in this list are skipped with a warning.
fn widget_event_whitelist(widget: &str) -> &'static [&'static str] {
    match widget {
        "ElevatedButton" | "TextButton" => &["onPressed", "onLongPress"],
        "IconButton" => &["onPressed", "onLongPress"],
        "TextField" => &["onChanged", "onSubmitted", "onTap"],
        "Checkbox" | "Switch" | "Slider" | "DropdownButton" => &["onChanged"],
        _ => &[],
    }
}

/// Whether `param` is a valid constructor parameter for `widget`.
pub(crate) fn widget_allows_param(widget: &str, param: &str) -> bool {
    widget_param_whitelist(widget).contains(&param)
}

/// Task 3.6: tap-family callbacks that NO widget whitelist accepts — on
/// generic elements (`div` → Container, `span` → Text, ...) these wrap the
/// rendered element in a `GestureDetector` (whose param name this returns)
/// instead of being dropped with a warning (flutter HIGH: ThemeSwitcher's
/// theme cards emitted no tap target at all). The click family
/// (press/click/tap all canonicalize to `onPressed`) maps to
/// `GestureDetector.onTap`; long-press/double-tap keep their names.
pub(crate) fn generic_tap_wrapper_callback(callback: &str) -> Option<&'static str> {
    match callback {
        "onPressed" | "onTap" => Some("onTap"),
        "onLongPress" => Some("onLongPress"),
        "onDoubleTap" => Some("onDoubleTap"),
        _ => None,
    }
}

/// Whether the Flutter SDK's constructor for `widget` is `const` —
/// const injection (T5.7) must never prefix a non-const constructor
/// (`const_with_non_const` analyzer error). Verified against the SDK:
/// `Container`, `AppBar`, `DropdownButton`, `ListView`,
/// `SingleChildScrollView`, and `Image.network` constructors are NOT const.
pub(crate) fn widget_has_const_constructor(widget: &str) -> bool {
    match widget {
        "Text" | "Row" | "Column" | "Stack" | "Center" | "Padding" | "SizedBox" | "Expanded"
        | "Flexible" | "Icon" | "Card" | "Scaffold" | "ElevatedButton" | "TextButton"
        | "IconButton" | "Checkbox" | "Switch" | "Slider" | "TextField" | "Form" => true,
        _ => false,
    }
}

/// Rename a mapped prop param for a specific widget.
///
/// E.g. `alt` maps to `semanticLabel` generically, but Text's correct
/// parameter is `semanticsLabel`.
pub(crate) fn widget_param_rename<'a>(widget: &str, param: &'a str) -> &'a str {
    match (widget, param) {
        ("Text", "semanticLabel") => "semanticsLabel",
        _ => param,
    }
}

/// Map a widget param that must be wrapped in a sub-object instead of being a
/// direct constructor parameter (C6). Returns `(wrapper_type, member_name)`:
///
/// - `Text` `fontSize`/`fontWeight`/`fontStyle` → `TextStyle(...)`
/// - `TextField` `hintText`/`label` → `InputDecoration(...)`
///
/// The wrapper is emitted as a single `style:`/`decoration:` argument with all
/// wrapped members merged (e.g. `style: TextStyle(fontSize: 16, fontWeight: w)`).
pub(crate) fn widget_param_wrapper(
    widget: &str,
    param: &str,
) -> Option<(&'static str, &'static str)> {
    match widget {
        "Text" => match param {
            "fontSize" => Some(("TextStyle", "fontSize")),
            "fontWeight" => Some(("TextStyle", "fontWeight")),
            "fontStyle" => Some(("TextStyle", "fontStyle")),
            _ => None,
        },
        "TextField" => match param {
            "hintText" => Some(("InputDecoration", "hintText")),
            "label" => Some(("InputDecoration", "labelText")),
            _ => None,
        },
        _ => None,
    }
}

/// Whether `callback` is a valid event callback parameter for `widget`.
pub(crate) fn widget_allows_event(widget: &str, callback: &str) -> bool {
    widget_event_whitelist(widget).contains(&callback)
}

/// Whether a widget callback parameter receives NO event argument in
/// Flutter (the `VoidCallback` family). Handlers bound to these params
/// must emit parameterless closures — `(e) { ... }` would not match the
/// signature. All other mapped callbacks (onChanged, onSubmitted,
/// onSelected, onHover, onScroll, ...) receive a value argument.
pub(crate) fn callback_is_void(callback: &str) -> bool {
    matches!(
        callback,
        "onPressed" | "onLongPress" | "onTap" | "onDoubleTap"
    )
}

// ═══════════════════════════════════════════════════════════════
// Per-widget literal prop value conversion (T5.7)
// ═══════════════════════════════════════════════════════════════

/// Convert a LITERAL prop value into a typed Dart expression for a widget
/// parameter (review HIGH-4: `fontWeight="bold"` emitted `fontWeight: 'bold'`
/// — a String where `FontWeight?` was required, breaking dart analyze).
///
/// The table covers the common enum-ish / numeric TextStyle and widget
/// params the semantic DSL surfaces as strings:
///
/// - `fontWeight`: "bold" → `FontWeight.w700`, "normal" → `FontWeight.w400`,
///   `w100`–`w900` / `100`–`900` → `FontWeight.wXXX`
/// - `fontSize`: CSS length ("16px") or number → Dart double literal
/// - `color`/`backgroundColor`: CSS hex/rgb → `Color(...)`
/// - `textAlign` / `overflow` / `fit`: CSS values → Flutter enum constants
/// - `width`/`height`/`size`: CSS length or number → double
/// - `padding`/`margin`: single CSS length → `EdgeInsets.all(...)`
///
/// Returns `None` for non-literal expressions and unmappable values — the
/// caller keeps the raw expression (a dynamic binding must not be coerced).
/// Params whose target type REQUIRES a non-String value (fontWeight, color,
/// ...) must convert; unmappable strings degrade via the caller's
/// diagnostic channel instead of emitting invalid Dart.
pub(crate) fn convert_literal_prop_value(
    widget: &str,
    param: &str,
    value: &ExprIR,
) -> Option<String> {
    let ExprIR::LiteralExpr { value } = value else {
        return None;
    };
    match value {
        serde_json::Value::String(s) => convert_string_prop_value(widget, param, s),
        serde_json::Value::Number(n) => convert_number_prop_value(param, n),
        // Task 3.5: the DOM boolean `disabled` (bare attr lowers to
        // `Bool(true)`, `disabled={false}` to `Bool(false)`) must map to the
        // NEGATED `enabled` value — `disabled` → `enabled: false`.
        serde_json::Value::Bool(b) if param == "enabled" => Some((!b).to_string()),
        _ => None,
    }
}

/// String-literal prop conversion (see [`convert_literal_prop_value`]).
fn convert_string_prop_value(widget: &str, param: &str, s: &str) -> Option<String> {
    match param {
        "fontWeight" => font_weight_to_dart(s),
        "fontSize" => css_length_to_dart_double(s),
        "color" | "backgroundColor" => css_color_to_dart(s),
        "textAlign" => match s.trim() {
            "left" => Some("TextAlign.left".into()),
            "right" => Some("TextAlign.right".into()),
            "center" => Some("TextAlign.center".into()),
            "justify" => Some("TextAlign.justify".into()),
            "start" => Some("TextAlign.start".into()),
            "end" => Some("TextAlign.end".into()),
            _ => None,
        },
        "overflow" => match s.trim() {
            "visible" => Some("TextOverflow.visible".into()),
            "hidden" | "clip" => Some("TextOverflow.clip".into()),
            "ellipsis" => Some("TextOverflow.ellipsis".into()),
            "fade" => Some("TextOverflow.fade".into()),
            _ => None,
        },
        "fit" => match s.trim() {
            "contain" => Some("BoxFit.contain".into()),
            "cover" => Some("BoxFit.cover".into()),
            "fill" => Some("BoxFit.fill".into()),
            "fitWidth" | "fit-width" => Some("BoxFit.fitWidth".into()),
            "fitHeight" | "fit-height" => Some("BoxFit.fitHeight".into()),
            "none" => Some("BoxFit.none".into()),
            "scaleDown" | "scale-down" => Some("BoxFit.scaleDown".into()),
            _ => None,
        },
        "width" | "height" => css_length_to_dart_double(s),
        "size" if widget == "Icon" => css_length_to_dart_double(s),
        "padding" | "margin" => {
            css_length_to_dart_double(s).map(|n| format!("EdgeInsets.all({})", n))
        }
        _ => None,
    }
}

/// Number-literal prop conversion — integral values get a `.0` suffix so
/// they type-check as `double` (fontSize/width/height/size take doubles).
fn convert_number_prop_value(param: &str, n: &serde_json::Number) -> Option<String> {
    match param {
        "fontSize" | "width" | "height" | "size" => n.as_f64().map(|f| {
            if f.fract() == 0.0 {
                format!("{:.1}", f)
            } else {
                f.to_string()
            }
        }),
        _ => None,
    }
}

/// CSS `font-weight` → Dart `FontWeight.*` constant.
fn font_weight_to_dart(s: &str) -> Option<String> {
    let trimmed = s.trim().to_ascii_lowercase();
    match trimmed.as_str() {
        "normal" => Some("FontWeight.w400".into()),
        "bold" => Some("FontWeight.w700".into()),
        w if w.starts_with('w') && w.len() == 4 => {
            let n: u32 = w[1..].parse().ok()?;
            if (100..=900).contains(&n) && n % 100 == 0 {
                Some(format!("FontWeight.{}", w))
            } else {
                None
            }
        }
        w => {
            let n: u32 = w.parse().ok()?;
            if (100..=900).contains(&n) && n % 100 == 0 {
                Some(format!("FontWeight.w{}", n))
            } else {
                None
            }
        }
    }
}

/// CSS length ("16px", "1.5em", "8", "50%") → Dart double literal.
/// Returns `None` for malformed or multi-part values.
fn css_length_to_dart_double(s: &str) -> Option<String> {
    let trimmed = s.trim();
    let split = trimmed
        .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
        .unwrap_or(trimmed.len());
    let (numeric, unit) = trimmed.split_at(split);
    if numeric.is_empty() || matches!(numeric, "-" | "." | "-.") {
        return None;
    }
    if !unit.is_empty() && !matches!(unit, "px" | "em" | "rem" | "%" | "pt") {
        return None;
    }
    numeric.parse::<f64>().ok()?;
    Some(if numeric.contains('.') {
        numeric.to_string()
    } else {
        format!("{}.0", numeric)
    })
}

/// Material Icons name map for `<icon name="...">` (T5.7 / review HIGH-4):
/// the name becomes the required POSITIONAL `Icon(Icons.<name>)` argument.
/// Known common names map; unknown names return `None` and the caller warns
/// and falls back to a compile-able placeholder (`Icons.help_outline`) so the
/// required positional `icon` is always satisfied.
pub(crate) fn icon_name_to_dart(name: &str) -> Option<&'static str> {
    let icon = match name.trim() {
        "home" => "Icons.home",
        "search" => "Icons.search",
        "settings" => "Icons.settings",
        "favorite" => "Icons.favorite",
        "favorite_border" | "favorite-border" => "Icons.favorite_border",
        "star" => "Icons.star",
        "star_border" | "star-border" => "Icons.star_border",
        "star_half" | "star-half" => "Icons.star_half",
        "add" => "Icons.add",
        "add_circle" | "add-circle" => "Icons.add_circle",
        "remove" => "Icons.remove",
        "close" => "Icons.close",
        "clear" => "Icons.clear",
        "check" => "Icons.check",
        "check_circle" | "check-circle" => "Icons.check_circle",
        "done" => "Icons.done",
        "delete" => "Icons.delete",
        "edit" => "Icons.edit",
        "menu" => "Icons.menu",
        "more_vert" | "more-vert" => "Icons.more_vert",
        "arrow_back" | "arrow-back" => "Icons.arrow_back",
        "arrow_forward" | "arrow-forward" => "Icons.arrow_forward",
        "arrow_upward" | "arrow-upward" => "Icons.arrow_upward",
        "arrow_downward" | "arrow-downward" => "Icons.arrow_downward",
        "arrow_drop_down" | "arrow-drop-down" => "Icons.arrow_drop_down",
        "chevron_left" | "chevron-left" => "Icons.chevron_left",
        "chevron_right" | "chevron-right" => "Icons.chevron_right",
        "expand_more" | "expand-more" => "Icons.expand_more",
        "expand_less" | "expand-less" => "Icons.expand_less",
        "keyboard_arrow_down" | "keyboard-arrow-down" => "Icons.keyboard_arrow_down",
        "keyboard_arrow_up" | "keyboard-arrow-up" => "Icons.keyboard_arrow_up",
        "keyboard_arrow_left" | "keyboard-arrow-left" => "Icons.keyboard_arrow_left",
        "keyboard_arrow_right" | "keyboard-arrow-right" => "Icons.keyboard_arrow_right",
        "info" => "Icons.info",
        "info_outline" | "info-outline" => "Icons.info_outline",
        "warning" => "Icons.warning",
        "error" => "Icons.error",
        "error_outline" | "error-outline" => "Icons.error_outline",
        "help" => "Icons.help",
        "help_outline" | "help-outline" => "Icons.help_outline",
        "person" => "Icons.person",
        "people" => "Icons.people",
        "account_circle" | "account-circle" => "Icons.account_circle",
        "shopping_cart" | "shopping-cart" => "Icons.shopping_cart",
        "shopping_bag" | "shopping-bag" => "Icons.shopping_bag",
        "email" => "Icons.email",
        "phone" => "Icons.phone",
        "location_on" | "location-on" => "Icons.location_on",
        "place" => "Icons.place",
        "room" => "Icons.room",
        "date_range" | "date-range" => "Icons.date_range",
        "schedule" => "Icons.schedule",
        "lock" => "Icons.lock",
        "lock_open" | "lock-open" => "Icons.lock_open",
        "visibility" => "Icons.visibility",
        "visibility_off" | "visibility-off" => "Icons.visibility_off",
        "refresh" => "Icons.refresh",
        "share" => "Icons.share",
        "thumb_up" | "thumb-up" => "Icons.thumb_up",
        "thumb_down" | "thumb-down" => "Icons.thumb_down",
        "download" => "Icons.download",
        "upload" => "Icons.upload",
        "play_arrow" | "play-arrow" => "Icons.play_arrow",
        "pause" => "Icons.pause",
        "fast_forward" | "fast-forward" => "Icons.fast_forward",
        "fast_rewind" | "fast-rewind" => "Icons.fast_rewind",
        "list" => "Icons.list",
        "grid_on" | "grid-on" => "Icons.grid_on",
        "dashboard" => "Icons.dashboard",
        "notifications" => "Icons.notifications",
        "logout" => "Icons.logout",
        "login" => "Icons.login",
        "dark_mode" | "dark-mode" => "Icons.dark_mode",
        "light_mode" | "light-mode" => "Icons.light_mode",
        "send" => "Icons.send",
        "attach_file" | "attach-file" => "Icons.attach_file",
        "attach_money" | "attach-money" => "Icons.attach_money",
        "bookmark" => "Icons.bookmark",
        "bookmark_border" | "bookmark-border" => "Icons.bookmark_border",
        "build" => "Icons.build",
        "extension" => "Icons.extension",
        "face" => "Icons.face",
        "fingerprint" => "Icons.fingerprint",
        "filter_list" | "filter-list" => "Icons.filter_list",
        "grade" => "Icons.grade",
        "history" => "Icons.history",
        "home_work" | "home-work" => "Icons.home_work",
        "language" => "Icons.language",
        "layers" => "Icons.layers",
        "map" => "Icons.map",
        "mic" => "Icons.mic",
        "movie" => "Icons.movie",
        "music_note" | "music-note" => "Icons.music_note",
        "palette" => "Icons.palette",
        "photo" => "Icons.photo",
        "print" => "Icons.print",
        "radio_button_checked" | "radio-button-checked" => "Icons.radio_button_checked",
        "reorder" => "Icons.reorder",
        "save" => "Icons.save",
        "store" => "Icons.store",
        "trending_up" | "trending-up" => "Icons.trending_up",
        "trending_down" | "trending-down" => "Icons.trending_down",
        "tune" => "Icons.tune",
        "verified_user" | "verified-user" => "Icons.verified_user",
        "volume_up" | "volume-up" => "Icons.volume_up",
        "volume_down" | "volume-down" => "Icons.volume_down",
        "volume_mute" | "volume-mute" => "Icons.volume_mute",
        "volume_off" | "volume-off" => "Icons.volume_off",
        "work" => "Icons.work",
        "wifi" => "Icons.wifi",
        "chat" => "Icons.chat",
        "comment" => "Icons.comment",
        "forum" => "Icons.forum",
        "block" => "Icons.block",
        "cancel" => "Icons.cancel",
        "check_box" | "check-box" => "Icons.check_box",
        "check_box_outline_blank" | "check-box-outline-blank" => "Icons.check_box_outline_blank",
        "circle" => "Icons.circle",
        "color_lens" | "color-lens" => "Icons.color_lens",
        "create" => "Icons.create",
        "description" => "Icons.description",
        "event" => "Icons.event",
        "exit_to_app" | "exit-to-app" => "Icons.exit_to_app",
        "file_download" | "file-download" => "Icons.file_download",
        "file_upload" | "file-upload" => "Icons.file_upload",
        "flag" => "Icons.flag",
        "folder" => "Icons.folder",
        "group" => "Icons.group",
        "headset" => "Icons.headset",
        "image" => "Icons.image",
        "insert_chart" | "insert-chart" => "Icons.insert_chart",
        "keyboard" => "Icons.keyboard",
        "link" => "Icons.link",
        "mail" => "Icons.mail",
        "menu_book" | "menu-book" => "Icons.menu_book",
        "notifications_active" | "notifications-active" => "Icons.notifications_active",
        "notifications_none" | "notifications-none" => "Icons.notifications_none",
        "payment" => "Icons.payment",
        "perm_identity" | "perm-identity" => "Icons.perm_identity",
        "pets" => "Icons.pets",
        "power_settings_new" | "power-settings-new" => "Icons.power_settings_new",
        "public" => "Icons.public",
        "question_answer" | "question-answer" => "Icons.question_answer",
        "receipt" => "Icons.receipt",
        "record_voice_over" | "record-voice-over" => "Icons.record_voice_over",
        "reply" => "Icons.reply",
        "report" => "Icons.report",
        "rowing" => "Icons.rowing",
        "school" => "Icons.school",
        "security" => "Icons.security",
        "thumb_up_alt" | "thumb-up-alt" => "Icons.thumb_up_alt",
        "timer" => "Icons.timer",
        "touch_app" | "touch-app" => "Icons.touch_app",
        "translate" => "Icons.translate",
        "vpn_key" | "vpn-key" => "Icons.vpn_key",
        "web" => "Icons.web",
        "widgets" => "Icons.widgets",
        "zoom_in" | "zoom-in" => "Icons.zoom_in",
        "zoom_out" | "zoom-out" => "Icons.zoom_out",
        _ => return None,
    };
    Some(icon)
}
