// Tree utilities for ComponentTree.

export interface TreeNode {
  id: string;
  tag: string;
  htmlTag: string;
  displayLabel: string;
  componentRef?: string;
  children: TreeNode[];
}

export function resolveHtmlTag(tag: string, _props: Record<string, unknown>): string {
  return tag.toLowerCase();
}

export function getDisplayLabel(props: Record<string, unknown>): string {
  if (props.text) return String(props.text).slice(0, 24);
  if (props.label) return String(props.label).slice(0, 24);
  if (props.placeholder) return String(props.placeholder).slice(0, 24);
  return "";
}

export function getComponentMeta(_tag: string) {
  return undefined;
}
