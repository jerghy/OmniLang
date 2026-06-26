/** OmniLang 组件名 → HTML 元素真名映射 */
const TAG_TO_HTML: Record<string, string> = {
  Container: "div", Flex: "div", Grid: "div", Card: "div",
  Text: "p", Heading: "h1",
  Button: "button", Input: "input", TextArea: "textarea",
  Select: "select", Checkbox: "input[checkbox]",
  List: "ul", ListItem: "li",
  Image: "img", Spinner: "div", Progress: "progress",
  Alert: "div", Link: "a", Form: "form",
};

/** 根据 tag + props 推导 HTML 元素真名（处理动态 tagByProp 如 Heading level → h2） */
export function resolveHtmlTag(tag: string, props: Record<string, unknown>): string {
  const base = TAG_TO_HTML[tag];
  if (!base) return tag;
  // Heading: level="2" → h2
  if (tag === "Heading" && props.level) return `h${props.level}`;
  return base;
}

/** 从 props 中提取元素的可读标识文字 */
export function getDisplayLabel(props: Record<string, unknown>): string {
  const p = props as Record<string, unknown>;
  return String(p.label ?? p.placeholder ?? p.alt ?? p.href ?? "");
}

export interface TreeNode {
  id: string;
  tag: string;
  htmlTag: string;
  displayLabel: string;
  componentRef?: string;
  children: TreeNode[];
}
