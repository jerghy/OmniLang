// components/canvas/code/flowConstants.ts
// Constants and types shared between FlowCanvas and CanvasContextMenu.

export interface ContextMenuState {
  x: number;
  y: number;
  flowX: number;
  flowY: number;
}

/** Chinese labels for code-element tags shown in nodes */
export const TAG_CN: Record<string, string> = {
  fn: "函数",
  "code-block": "代码块",
  condition: "判断",
  return: "返回",
};

export const CODE_TAGS = new Set([
  "fn",
  "code-block",
  "condition",
  "return",
]);

export interface MenuItem {
  label: string;
  tag: string;
  shortcut?: string;
}

export interface MenuSection {
  label: string;
  items: MenuItem[];
}

export const MENU_SECTIONS: MenuSection[] = [
  {
    label: "控制流",
    items: [
      { label: "函数定义 (fn)", tag: "fn", shortcut: "fn" },
      { label: "判断 (condition)", tag: "condition", shortcut: "cond" },
      { label: "代码块 (code-block)", tag: "code-block", shortcut: "cb" },
      { label: "返回 (return)", tag: "return", shortcut: "ret" },
    ],
  },
];

export const MENU_WIDTH = 210;
export const ITEM_HEIGHT = 30;
const HEADER_HEIGHT = 26;
