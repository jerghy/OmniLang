// ═══════════════════════════════════════════════════════════════
// Flutter-specific helpers
// ═══════════════════════════════════════════════════════════════

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
        "textfield" => "TextField",
        "form" => "Form",
        "scrollview" | "singlechildscrollview" => "SingleChildScrollView",
        _ => "Container",
    }
}

/// Map OmniView event names to Flutter callback parameter names.
pub(crate) fn event_to_flutter_callback(event_name: &str) -> &str {
    // Strip any existing "on" prefix to normalize (IR may carry "onPress" etc.)
    let stem = event_name
        .strip_prefix("on")
        .unwrap_or(event_name)
        .to_lowercase();
    match stem.as_str() {
        "click" | "tap" | "press" => "onPressed",
        "change" | "input" => "onChanged",
        "submit" => "onSubmitted",
        "focus" => "onFocus",
        "blur" => "onBlur",
        "keydown" => "onKeyDown",
        "keyup" => "onKeyUp",
        "longpress" => "onLongPress",
        "doubleclick" | "dblclick" => "onDoubleTap",
        "hover" | "mouseenter" => "onHover",
        "scroll" => "onScroll",
        "drag" | "dragstart" => "onDragStart",
        "dragend" => "onDragEnd",
        "dragover" => "onDrag",
        "drop" => "onDrop",
        "toggle" => "onChanged",
        "select" => "onSelected",
        _ => event_name,
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
        "disabled" => "",
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
            "children",
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
        "Icon" => &["key", "icon", "size", "color", "semanticLabel"],
        "Image.network" => &[
            "key",
            "image",
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
        "DropdownButton" => &["key", "value", "onChanged", "items", "hint"],
        "Form" => &["key", "child", "onWillPop"],
        "SingleChildScrollView" => &["key", "child", "scrollDirection", "padding"],
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
