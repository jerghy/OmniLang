//! Shared HTML tag and attribute mapping across codegen backends.
//!
//! Provides unified mappings from Omni-language component tags and prop names
//! to their HTML/JSX equivalents.  Reduces duplication across OmniView, React,
//! Dioxus, and Vue backends.

/// Map an Omni component tag name to the equivalent HTML tag.
///
/// Omni uses PascalCase aliases (Button, Card, Text, etc.) for HTML elements.
/// Backends that need different handling for custom tags (kebab-case for
/// OmniView, lowercase-first-letter for React/JSX) should call this function
/// and then post-process the result.
pub(crate) fn to_html_tag(tag: &str) -> String {
    match tag {
        "Text" => "span".into(),
        "Button" => "button".into(),
        "Container" | "Card" => "div".into(),
        "Flex" => "div".into(),
        "Heading" => "h2".into(),
        "Image" | "Img" => "img".into(),
        "TextInput" | "Input" => "input".into(),
        "Link" => "a".into(),
        "List" => "ul".into(),
        "ListItem" => "li".into(),
        "CheckBox" | "Checkbox" => "input".into(),
        "Select" => "select".into(),
        "TextArea" | "Textarea" => "textarea".into(),
        // For other PascalCase tags, return as-is — the caller decides
        // the output style (kebab-case, lowercase, etc.)
        _ => tag.to_string(),
    }
}

/// Map an Omni prop/attribute name to the equivalent HTML attribute name.
///
/// Handles React-specific casing (`className` -> `class`, `htmlFor` -> `for`)
/// as well as Omni-to-HTML attribute renames.
pub(crate) fn to_html_attr(prop_name: &str) -> &str {
    match prop_name {
        "className" | "class" => "class",
        "htmlFor" | "for" => "for",
        "tabIndex" | "tabindex" => "tabindex",
        "readOnly" | "readonly" => "readonly",
        "maxLength" | "maxlength" => "maxlength",
        "minLength" | "minlength" => "minlength",
        "autoComplete" | "autocomplete" => "autocomplete",
        "autoFocus" | "autofocus" => "autofocus",
        "autoPlay" | "autoplay" => "autoplay",
        "encType" | "enctype" => "enctype",
        "formAction" | "formaction" => "formaction",
        "formEncType" | "formenctype" => "formenctype",
        "formMethod" | "formmethod" => "formmethod",
        "formNoValidate" | "formnovalidate" => "formnovalidate",
        "formTarget" | "formtarget" => "formtarget",
        _ => prop_name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_html_tag_known() {
        assert_eq!(to_html_tag("Text"), "span");
        assert_eq!(to_html_tag("Button"), "button");
        assert_eq!(to_html_tag("Container"), "div");
        assert_eq!(to_html_tag("Image"), "img");
        assert_eq!(to_html_tag("Link"), "a");
    }

    #[test]
    fn test_to_html_tag_passthrough() {
        assert_eq!(to_html_tag("div"), "div");
        assert_eq!(to_html_tag("CustomComponent"), "CustomComponent");
    }

    #[test]
    fn test_to_html_attr_known() {
        assert_eq!(to_html_attr("className"), "class");
        assert_eq!(to_html_attr("class"), "class");
        assert_eq!(to_html_attr("htmlFor"), "for");
        assert_eq!(to_html_attr("tabIndex"), "tabindex");
        assert_eq!(to_html_attr("readOnly"), "readonly");
    }

    #[test]
    fn test_to_html_attr_passthrough() {
        assert_eq!(to_html_attr("id"), "id");
        assert_eq!(to_html_attr("style"), "style");
        assert_eq!(to_html_attr("data-foo"), "data-foo");
    }
}
