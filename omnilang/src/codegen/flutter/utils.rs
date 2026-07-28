// ═══════════════════════════════════════════════════════════════
// Flutter-specific helpers
// ═══════════════════════════════════════════════════════════════

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
