// editor/client/src/omni-gen/view-tree-builder.ts
import type { ElementData } from "../store/editor";
import type { ViewNode } from "./types";

const TAG_HTML_MAP = new Set([
  "Container", "Flex", "Grid", "Card", "Text", "Heading",
  "Button", "Input", "TextArea", "Select", "Checkbox",
  "List", "ListItem", "Image", "Spinner", "Progress",
  "Alert", "Link", "Form",
]);

export function buildViewTree(
  elements: ElementData[],
  compName: string,
): { roots: ViewNode[]; pascalCaseTags: string[] } {
  const byId = new Map<string, ElementData>();
  const byParent = new Map<string | null, ElementData[]>();
  const pascalCaseTags: string[] = [];

  for (const el of elements) {
    if (el.nodeType !== "view") continue;
    byId.set(el.id, el);
    const key = el.parentId;
    if (!byParent.has(key)) byParent.set(key, []);
    byParent.get(key)!.push(el);
  }

  function buildNode(el: ElementData, currentPath: string): ViewNode {
    const isPascal = /^[A-Z]/.test(el.tag) && !TAG_HTML_MAP.has(el.tag);
    if (isPascal && !pascalCaseTags.includes(el.tag)) {
      pascalCaseTags.push(el.tag);
    }

    const childElements = byParent.get(el.id) || [];
    const children = childElements.map((c, i) =>
      buildNode(c, `${currentPath}.${c.tag.toLowerCase()}.${i}`),
    );

    return {
      id: el.id,
      path: currentPath,
      tag: el.tag,
      props: { ...el.props },
      children,
      kind: "element",
      isFolded: el.isFolded,
      renderMode: el.renderMode,
    };
  }

  const roots = (byParent.get(null) || []).map((el) => buildNode(el, compName));

  return { roots, pascalCaseTags };
}
