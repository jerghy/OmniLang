// poiesis/src/services/codegen/style-parser.ts
// Parse style { } blocks from a component AST and populate ElementStyle on view elements.
// Called when loading .omni files to populate element styles before storing.

import { uid } from "../../core/utils/id";
import type { StyleProperty, ElementData } from "../../core/types";
import type { ViewNode } from "./types";
import { parseStyleValue } from "./design-value-parser";

function generateId(): string {
  return uid("s");
}

/** Extract children from a ViewNode for tree walking — handles all union variants. */
function getViewNodeChildren(node: ViewNode): ViewNode[] {
  switch (node.kind) {
    case "element":
    case "for":
    case "slot":
      return node.children;
    default:
      return [];
  }
}

function findViewNode(view: ViewNode | null | undefined, elId: string): ViewNode | null {
  if (!view) return null;
  if (view.id === elId) return view;
  for (const child of getViewNodeChildren(view)) {
    const found = findViewNode(child, elId);
    if (found) return found;
  }
  return null;
}

interface StyleRule {
  class_name: string;
  properties: Record<string, string>;
}

interface ComponentAST {
  name?: string;
  style?: StyleRule[];
  view?: ViewNode;
}

/**
 * Parse style { } blocks from a component AST and populate ElementStyle
 * (classRefs + inlineProperties) on the given view elements via el.style.
 *
 * The old StyleRegistry has been removed — its methods were all no-ops.
 *
 * Handles:
 *   - class: attributes on view nodes → classRefs
 *   - inline_style on view nodes → inlineProperties
 *   - Backward compatibility: old files without style { } blocks
 *     → extracts known style property keys from element props as inlineProperties
 */
export function parseStylesFromAST(
  componentNode: ComponentAST | undefined | null,
  viewElements: Map<string, ElementData>,
): void {
  const styleBlock = componentNode?.style;
  const hasStyleBlock = Array.isArray(styleBlock);

  // Walk view elements → map class: and style: to ElementStyle
  for (const [elId, el] of viewElements) {
    // Must be a ViewElementData or CodeblockElementData to have props/style
    if (!("tag" in el)) continue;

    const classRefs: string[] = [];
    const inlineProperties: StyleProperty[] = [];
    const viewNode = findViewNode(componentNode?.view, elId);

    if (viewNode && viewNode.kind === "element") {
      // Parse class: attribute → classRefs
      // Falls back to className (stored by parser from class: attribute in source)
      const classAttr = viewNode.props?.["class"] || viewNode.props?.["className"];
      if (classAttr && typeof classAttr === "string") {
        const allClasses = classAttr.split(/\s+/).filter(c => {
          if (!c) return false;
          // Filter auto-generated class names from old style-resolver.ts:
          // Patterns: main_div_0, main_div_0_p_0, app_view, etc.
          if (/^[a-z][a-z0-9_]*_\d+$/.test(c)) return false;
          if (/^[a-z]+_[a-z]+\d*_\d+_[a-z]+\d*_\d+$/.test(c)) return false;
          return true;
        });
        classRefs.push(...allClasses);
      }

      // Parse inline_style → inlineProperties
      const inlineStyle = viewNode.inline_style;
      if (inlineStyle && typeof inlineStyle === "object") {
        for (const [name, value] of Object.entries(inlineStyle)) {
          const segments = parseStyleValue(String(value));
          if (segments.length > 0) {
            inlineProperties.push({
              id: generateId(),
              name,
              segments,
            });
          }
        }
      }
    }

    // 3. Backward compatibility: no style { } block and no class:/style: attributes
    //    → extract known style keys from el.props and create inline properties
    if (classRefs.length === 0 && inlineProperties.length === 0 && !hasStyleBlock) {
      for (const key of Object.keys(el.props)) {
        if (/^(background|color|padding|margin|border|radius|width|height|display|flex|gap|font|text|align|justify)/i.test(key)) {
          const segments = parseStyleValue(String(el.props[key]));
          if (segments.length > 0) {
            inlineProperties.push({
              id: generateId(),
              name: key,
              segments,
            });
          }
        }
      }
    }

    // el.style may not exist yet on freshly-parsed elements (optional prop),
    // so assign unconditionally for view elements instead of checking with `in`.
    // Group elements are filtered by the "tag" in el check above.
    if (el.nodeType !== "view") continue;
    el.style = { classRefs, inlineProperties };
    // Sync to top-level classRefs for hasDesignClassUsage check
    el.classRefs = classRefs;

    // Cleanup: remove old props.style and props.className when style
    // has been set, to avoid duplicate style/class attributes in emitted source
    if ((classRefs.length > 0 || inlineProperties.length > 0) && el.props) {
      const restProps = { ...el.props };
      delete restProps.style;
      delete restProps.className;
      el.props = restProps;
    }
  }
}
