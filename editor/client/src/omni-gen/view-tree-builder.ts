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
  rootOrder?: string[],
): { roots: ViewNode[]; pascalCaseTags: string[] } {
  const byId = new Map<string, ElementData>();
  const pascalCaseTags: string[] = [];

  for (const el of elements) {
    if (el.nodeType !== "view") continue;
    byId.set(el.id, el);
  }

  function buildNode(el: ElementData, currentPath: string): ViewNode {
    const isPascal = /^[A-Z]/.test(el.tag) && !TAG_HTML_MAP.has(el.tag);
    if (isPascal && !pascalCaseTags.includes(el.tag)) {
      pascalCaseTags.push(el.tag);
    }

    // Use el.children order instead of flat-array iteration order
    const childElements = (el.children || [])
      .map((cid) => byId.get(cid))
      .filter((c): c is ElementData => c !== undefined);
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

  // Use rootOrder for root element ordering; fall back to byParent(null) order
  let rootElements: ElementData[];
  if (rootOrder) {
    rootElements = rootOrder
      .map((id) => byId.get(id))
      .filter((el): el is ElementData => el !== undefined);
  } else {
    rootElements = elements.filter((el) => el.parentId === null && el.nodeType === "view");
  }
  const roots = rootElements.map((el) => buildNode(el, compName));

  return { roots, pascalCaseTags };
}
