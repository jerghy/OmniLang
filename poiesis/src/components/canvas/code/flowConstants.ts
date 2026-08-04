// components/canvas/code/flowConstants.ts
// Constants shared between FlowCanvas and NodeSearchMenu.

/** Chinese labels for code-element tags shown in nodes */
export const TAG_CN: Record<string, string> = {
  fn: "函数",
  "code-block": "代码块",
  condition: "判断",
  return: "返回",
  let: "变量声明",
  assign: "赋值",
  "store-call": "store 调用",
  call: "调用",
};

export const CODE_TAGS = new Set([
  "fn",
  "code-block",
  "condition",
  "return",
  "let",
  "assign",
  "store-call",
  "call",
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
  {
    label: "变量与赋值",
    items: [
      { label: "变量声明 (let)", tag: "let", shortcut: "let" },
      { label: "赋值 (assign)", tag: "assign", shortcut: "=" },
    ],
  },
  {
    label: "调用",
    items: [
      { label: "store 调用", tag: "store-call", shortcut: "st" },
      { label: "函数调用 (call)", tag: "call", shortcut: "fn()" },
    ],
  },
];
