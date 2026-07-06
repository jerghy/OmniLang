// poiesis/src/services/codegen/style-parser.ts
// Parse style { } blocks from a component AST and populate ElementStyle on view elements.
// Called when loading .omni files to populate element styles before storing.

import { uid } from "../../core/utils";
import { classNameToVariable, variableToClassName } from "../../core/types";
import type { StyleProperty, ElementData } from "../../core/types";
import type { ViewNode } from "./types";
import { useStyleRegistryStore } from "../../core/state/styleRegistry";

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

function exprToString(expr: unknown): string {
  if (!expr) return "";
  if (typeof expr === "string") return expr;
  if (typeof expr === "number") return String(expr);
  if (typeof expr === "object" && expr !== null) {
    const e = expr as Record<string, unknown>;
    if (e.value !== undefined) return String(e.value);
    if (e.literal !== undefined) return String(e.literal);
  }
  return String(expr);
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
 * Also populates the StyleRegistry (useStyleRegistryStore) from style { } blocks.
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
  const componentName = componentNode?.name ?? "App";
  const store = useStyleRegistryStore.getState();

  // 1. Parse style { } blocks → populate StyleRegistry
  const styleBlock = componentNode?.style;
  const hasStyleBlock = Array.isArray(styleBlock);
  if (hasStyleBlock) {
    for (const rule of styleBlock) {
      const className: string = rule.class_name;
      const properties: StyleProperty[] = Object.entries(rule.properties).map(
        ([name, valueExpr]: [string, string]) => ({
          id: generateId(),
          name,
          segments: [{ type: "text" as const, value: exprToString(valueExpr) }],
        })
      );

      const varName = classNameToVariable(className);
      if (varName) {
        store.addVariable(componentName, {
          id: generateId(),
          name: varName,
          segments: properties[0]?.segments ?? [],
        });
      } else {
        store.addClass(componentName, {
          id: generateId(),
          name: className,
          properties,
        });
      }
    }
  }

  // 2. Walk view elements → map class: and style: to ElementStyle
  for (const [elId, el] of viewElements) {
    const classRefs: string[] = [];
    const inlineProperties: StyleProperty[] = [];
    const viewNode = findViewNode(componentNode?.view, elId);

    if (viewNode) {
      // Parse class: attribute → classRefs
      const classAttr = viewNode.props?.["class"];
      if (classAttr && typeof classAttr === "string") {
        const allClasses = classAttr.split(/\s+/);
        if (hasStyleBlock && store.getRegistry(componentName).classes.length > 0) {
          const registry = store.getRegistry(componentName);
          const registryNames = new Set((registry.classes as Array<{ name: string }>).map((c) => c.name));
          for (const v of registry.variables) {
            registryNames.add(variableToClassName(v.name));
          }
          const compEscaped = componentName.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
          const autoGenRe = new RegExp(`^${compEscaped}_[a-z]+(_\\d+)?$`);
          for (const c of allClasses) {
            if (registryNames.has(c) || autoGenRe.test(c)) classRefs.push(c);
          }
        } else {
          classRefs.push(...allClasses);
        }
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
