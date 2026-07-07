// poiesis/src/services/codegen/style-parser.ts
// Parse style { } blocks from a component AST and populate ElementStyle on view elements.
// Called when loading .omni files to populate element styles before storing.

import { uid } from "../../core/utils";
import type { StyleProperty, ElementData } from "../../core/types";
import type { ViewNode } from "./types";

function generateId(): string {
  return uid("s");
}

function findViewNode(view: ViewNode | null | undefined, elId: string): ViewNode | null {
  if (!view) return null;
  if (view.id === elId) return view;
  for (const child of view.children || []) {
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
    const classRefs: string[] = [];
    const inlineProperties: StyleProperty[] = [];
    const viewNode = findViewNode(componentNode?.view, elId);

    if (viewNode) {
      // Parse class: attribute → classRefs
      const classAttr = viewNode.props?.["class"];
      if (classAttr && typeof classAttr === "string") {
        const allClasses = classAttr.split(/\s+/);
        classRefs.push(...allClasses);
      }

      // Parse inline_style → inlineProperties
      const inlineStyle = viewNode.inline_style;
      if (inlineStyle && typeof inlineStyle === "object") {
        for (const [name, value] of Object.entries(inlineStyle)) {
          inlineProperties.push({
            id: generateId(),
            name,
            segments: [{ type: "text" as const, value: String(value) }],
          });
        }
      }
    }

    // 3. Backward compatibility: no style { } block and no class:/style: attributes
    //    → extract known style keys from el.props and create inline properties
    if (classRefs.length === 0 && inlineProperties.length === 0 && !hasStyleBlock) {
      for (const key of Object.keys(el.props)) {
        if (/^(background|color|padding|margin|border|radius|width|height|display|flex|gap|font|text|align|justify)/i.test(key)) {
          inlineProperties.push({
            id: generateId(),
            name: key,
            segments: [{ type: "text" as const, value: String(el.props[key]) }],
          });
        }
      }
    }

    el.style = { classRefs, inlineProperties };

    // Cleanup: remove old props.style and props.className when style
    // has been set, to avoid duplicate style/class attributes in emitted source
    if ((classRefs.length > 0 || inlineProperties.length > 0) && (el.props?.style || el.props?.className)) {
      const { style: _oldStyle, className: _oldClass, ...restProps } = el.props;
      el.props = restProps;
    }
  }
}
