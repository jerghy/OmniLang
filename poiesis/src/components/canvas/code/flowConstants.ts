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
  if: "如果",
  else: "否则",
  for: "计数循环",
  while: "条件循环",
  return: "返回",
  let: "let 声明",
  const: "const 声明",
  signal: "信号",
  computed: "计算属性",
  store: "Store",
  literal: "字面量",
  getVariable: "读取变量",
  setVariable: "赋值",
  add: "加法",
  sub: "减法",
  mul: "乘法",
  div: "除法",
  compare: "比较",
  and: "与",
  or: "或",
  not: "非",
  concat: "拼接",
  customFn: "函数调用",
};

export const CODE_TAGS = new Set([
  // Control flow
  "fn", "if", "else", "for", "while", "return",
  // Variables
  "let", "const", "literal", "getVariable", "setVariable",
  // Arithmetic
  "add", "sub", "mul", "div",
  // Comparison & logic
  "compare", "and", "or", "not",
  // String
  "concat",
  // Reactive
  "signal", "computed", "store",
  // Functions
  "customFn",
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
      { label: "函数定义", tag: "fn", shortcut: "fn" },
      { label: "条件判断 (if)", tag: "if", shortcut: "if" },
      { label: "否则分支 (else)", tag: "else", shortcut: "else" },
      { label: "计数循环 (for)", tag: "for", shortcut: "for" },
      { label: "条件循环 (while)", tag: "while", shortcut: "while" },
      { label: "返回 (return)", tag: "return", shortcut: "ret" },
    ],
  },
  {
    label: "变量",
    items: [
      { label: "变量声明 (let)", tag: "let", shortcut: "let" },
      { label: "常量声明 (const)", tag: "const", shortcut: "const" },
      { label: "读取变量 (get)", tag: "getVariable", shortcut: "get" },
      { label: "赋值 (set)", tag: "setVariable", shortcut: "set" },
    ],
  },
  {
    label: "运算",
    items: [
      { label: "加法 (+)", tag: "add", shortcut: "+" },
      { label: "减法 (−)", tag: "sub", shortcut: "−" },
      { label: "乘法 (×)", tag: "mul", shortcut: "×" },
      { label: "除法 (÷)", tag: "div", shortcut: "÷" },
    ],
  },
  {
    label: "逻辑",
    items: [
      { label: "比较 (==, >, <)", tag: "compare", shortcut: "cmp" },
      { label: "与 (&&)", tag: "and", shortcut: "&&" },
      { label: "或 (||)", tag: "or", shortcut: "||" },
      { label: "非 (!)", tag: "not", shortcut: "!" },
    ],
  },
  {
    label: "值 & 字符串",
    items: [
      { label: "字面量 (literal)", tag: "literal", shortcut: "lit" },
      { label: "字符串拼接 (concat)", tag: "concat", shortcut: "++" },
    ],
  },
  {
    label: "响应式",
    items: [
      { label: "信号 (signal)", tag: "signal", shortcut: "sig" },
      { label: "计算属性 (computed)", tag: "computed", shortcut: "comp" },
      { label: "Store 引用", tag: "store", shortcut: "str" },
    ],
  },
  {
    label: "函数调用",
    items: [], // dynamically populated from registered customFn / fn nodes
  },
];

export const MENU_WIDTH = 210;
export const ITEM_HEIGHT = 30; // approximate
export const HEADER_HEIGHT = 26;
