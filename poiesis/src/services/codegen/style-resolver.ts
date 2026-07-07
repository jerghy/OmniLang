// poiesis/src/services/codegen/style-resolver.ts
import type { ResolvedStyle, ElementViewNode } from "./types";

interface TagMapping {
  tag: string;
  propsToContent?: string;
  tagByProp?: string;
}

const TAG_HTML_MAP: Record<string, TagMapping> = {
  Container: { tag: "div" }, Flex: { tag: "div" }, Grid: { tag: "div" },
  Card: { tag: "div" }, Text: { tag: "p", propsToContent: "label" },
  Heading: { tag: "h1", propsToContent: "label", tagByProp: "level" },
  Button: { tag: "button", propsToContent: "label" },
  Input: { tag: "input" }, TextArea: { tag: "textarea" },
  Select: { tag: "select" }, Checkbox: { tag: "input" },
  List: { tag: "ul" }, ListItem: { tag: "li" },
  Image: { tag: "img" }, Spinner: { tag: "div" },
  Progress: { tag: "progress" }, Alert: { tag: "div" },
  Link: { tag: "a", propsToContent: "label" }, Form: { tag: "form" },
};

const STYLE_PROP_CSS_MAP: Record<string, string> = {
  background_color: "background", border_radius: "border-radius",
  font_size: "font-size", font_weight: "font-weight",
  text_align: "text-align", shadow: "box-shadow",
  padding: "padding", margin: "margin", gap: "gap",
  width: "width", height: "height", color: "color",
};

const STYLE_PROPS: Record<string, string[]> = {
  Container: ["padding","margin","gap","background_color","border_radius","width","height"],
  Button: ["width","padding","background_color","border_radius","font_size"],
  Flex: ["padding","margin","width","height"],
  Card: ["padding","margin","border_radius","shadow","background_color","width","height"],
  Text: ["font_size","font_weight","color","text_align"],
  Heading: ["font_size","font_weight","color"],
  Image: ["width","height","border_radius"],
  Input: ["width","padding","border_radius","font_size"],
  TextArea: ["width","padding"],
  Select: ["width","padding"],
  List: ["padding","margin","gap"],
  ListItem: ["padding","margin"],
  Alert: ["padding","border_radius"],
  Link: ["color","font_size"],
  Form: ["padding","gap","width"],
  Spinner: ["width","height"],
  Progress: ["width"],
};

export function resolveStyle(node: ElementViewNode): ResolvedStyle {
  const mapping = TAG_HTML_MAP[node.tag];
  const htmlTag = mapping ? mapping.tag : node.tag;

  const styleProps = STYLE_PROPS[node.tag] || [];
  const cssClassName = sanitizeClassName(node.path);

  const cssRules: Record<string, string> = {};
  const htmlAttrs: Record<string, string> = {};
  const eventBindings: Record<string, string> = {};
  let contentText: string | undefined;

  for (const [key, val] of Object.entries(node.props)) {
    if (val === undefined || val === null || val === "") continue;

    // Events
    if (key.startsWith("on") && typeof val === "string") {
      eventBindings[key] = val;
      continue;
    }

    // Style props
    if (styleProps.includes(key)) {
      const cssProp = STYLE_PROP_CSS_MAP[key] || key;
      cssRules[cssProp] = String(val);
      continue;
    }

    // Props to content
    if (mapping?.propsToContent === key) {
      contentText = String(val);
      continue;
    }

    // For raw HTML tags (no mapping), treat 'label' prop as content text
    // (p, button, h1-h6, etc. can contain text from parsed .omni source)
    if (!mapping && key === "label" && typeof val === "string") {
      contentText = val;
      continue;
    }

    // CSS custom property (-- prefixed) or regular HTML attr
    if (key.startsWith("--")) {
      cssRules[key] = String(val);
    } else {
      htmlAttrs[key] = String(val);
    }
  }

  // tagByProp: level="2" → <h2>
  let finalTag = htmlTag;
  if (mapping?.tagByProp && node.props[mapping.tagByProp]) {
    const level = String(node.props[mapping.tagByProp]!);
    if (mapping.tagByProp === "level") finalTag = `h${level}`;
  }

  return {
    htmlTag: finalTag,
    cssClassName,
    contentText,
    htmlAttrs,
    eventBindings,
    bindings: {},
    cssRules,
    usedClassNames: [cssClassName],
  };
}

function sanitizeClassName(path: string): string {
  return path.replace(/[^a-zA-Z0-9_]/g, "_").replace(/^_+/, "");
}
