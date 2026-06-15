// editor/client/src/canvas/nodeRegistry.ts

import type { PortData } from "../store/editor";

export interface NodeTypeConfig {
  tag: string;
  category: "data" | "variable" | "flow" | "function" | "code" | "ui";
  label: string;
  icon: string;
  /** Neutral node background — ports carry type color */
  color: { bg: string; border: string };
  defaultPorts: PortData[];
  /** Search keywords for the right-click palette */
  keywords: string[];
  /** Description shown in search results */
  description: string;
  /** Default config values */
  config?: Record<string, unknown>;
  /** Panel editor hint for PropertyPanel rendering */
  panelEditor?: "literal" | "signal-selector" | "element-selector" | "function-signature" | "return-validator" | "compare" | "variable" | null;
}

// ── Unified Node Configs (data-flow only, no flow edges) ──

const ALL_CONFIGS: NodeTypeConfig[] = [
  // ═══ Variables ═══
  {
    tag: "signal", category: "variable", label: "Signal", icon: "⚡",
    color: { bg: "#FAFAFA", border: "#E5E7EB" },
    defaultPorts: [
      { name: "value", direction: "out", portType: "data", type: "any", label: "值" },
      { name: "set", direction: "in", portType: "data", type: "any", label: "设置" },
    ],
    keywords: ["signal", "state", "variable", "value", "状态", "变量"],
    description: "响应式状态变量，存储和读取值",
    panelEditor: "variable",
  },
  {
    tag: "store", category: "variable", label: "Store", icon: "🗄️",
    color: { bg: "#FAFAFA", border: "#E5E7EB" },
    defaultPorts: [
      { name: "items", direction: "out", portType: "data", type: "any", label: "数据" },
    ],
    keywords: ["store", "collection", "database", "存储", "集合"],
    description: "数据集合容器",
    panelEditor: "variable",
  },
  {
    tag: "computed", category: "data", label: "Computed", icon: "🔄",
    color: { bg: "#FAFAFA", border: "#E5E7EB" },
    defaultPorts: [
      { name: "value", direction: "out", portType: "data", type: "any", label: "计算结果" },
      { name: "deps", direction: "in", portType: "data", type: "any", label: "依赖" },
    ],
    keywords: ["computed", "derived", "calculation", "计算", "派生"],
    description: "从其他值派生计算，依赖变化时自动更新",
    panelEditor: "variable",
  },

  // ═══ Data Access ═══
  {
    tag: "getVariable", category: "data", label: "Get Variable", icon: "📌",
    color: { bg: "#FAFAFA", border: "#E5E7EB" },
    defaultPorts: [
      { name: "value", direction: "out", portType: "data", type: "any", label: "value" },
    ],
    keywords: ["get", "read", "variable", "读取", "变量"],
    description: "读取变量的当前值",
    config: { variableName: "" },
  },
  {
    tag: "setVariable", category: "data", label: "Set Variable", icon: "⚡",
    color: { bg: "#FAFAFA", border: "#E5E7EB" },
    defaultPorts: [
      { name: "value", direction: "in", portType: "data", type: "any", label: "新值" },
    ],
    keywords: ["set", "assign", "variable", "设置", "赋值"],
    description: "设置变量的值。点击节点选择目标变量",
    config: { targetSignal: "" },
    panelEditor: "signal-selector",
  },

  // ═══ Arithmetic ═══
  {
    tag: "add", category: "data", label: "Add", icon: "➕",
    color: { bg: "#FAFAFA", border: "#E5E7EB" },
    defaultPorts: [
      { name: "a", direction: "in", portType: "data", type: "any", label: "A" },
      { name: "b", direction: "in", portType: "data", type: "any", label: "B" },
      { name: "result", direction: "out", portType: "data", type: "any", label: "A+B" },
    ],
    keywords: ["add", "plus", "sum", "加", "加法"],
    description: "两数相加",
  },
  {
    tag: "sub", category: "data", label: "Subtract", icon: "➖",
    color: { bg: "#FAFAFA", border: "#E5E7EB" },
    defaultPorts: [
      { name: "a", direction: "in", portType: "data", type: "any", label: "A" },
      { name: "b", direction: "in", portType: "data", type: "any", label: "B" },
      { name: "result", direction: "out", portType: "data", type: "any", label: "A-B" },
    ],
    keywords: ["sub", "subtract", "minus", "减", "减法"],
    description: "两数相减",
  },
  {
    tag: "mul", category: "data", label: "Multiply", icon: "✖️",
    color: { bg: "#FAFAFA", border: "#E5E7EB" },
    defaultPorts: [
      { name: "a", direction: "in", portType: "data", type: "any", label: "A" },
      { name: "b", direction: "in", portType: "data", type: "any", label: "B" },
      { name: "result", direction: "out", portType: "data", type: "any", label: "A×B" },
    ],
    keywords: ["mul", "multiply", "times", "乘", "乘法"],
    description: "两数相乘",
  },
  {
    tag: "div", category: "data", label: "Divide", icon: "➗",
    color: { bg: "#FAFAFA", border: "#E5E7EB" },
    defaultPorts: [
      { name: "a", direction: "in", portType: "data", type: "any", label: "A" },
      { name: "b", direction: "in", portType: "data", type: "any", label: "B" },
      { name: "result", direction: "out", portType: "data", type: "any", label: "A÷B" },
    ],
    keywords: ["div", "divide", "division", "除", "除法"],
    description: "两数相除",
  },

  // ═══ Comparison & Logic ═══
  {
    tag: "compare", category: "data", label: "Compare", icon: "⚖️",
    color: { bg: "#FAFAFA", border: "#E5E7EB" },
    defaultPorts: [
      { name: "a", direction: "in", portType: "data", type: "any", label: "A" },
      { name: "b", direction: "in", portType: "data", type: "any", label: "B" },
      { name: "result", direction: "out", portType: "data", type: "bool", label: "A==B" },
    ],
    keywords: ["compare", "equal", "greater", "less", "比较", "等于"],
    description: "比较两个值，返回布尔结果",
    config: { op: "==", ops: ["==", "!=", ">", "<", ">=", "<="] },
    panelEditor: "compare",
  },
  {
    tag: "and", category: "data", label: "And", icon: "∧",
    color: { bg: "#FAFAFA", border: "#E5E7EB" },
    defaultPorts: [
      { name: "a", direction: "in", portType: "data", type: "bool", label: "A" },
      { name: "b", direction: "in", portType: "data", type: "bool", label: "B" },
      { name: "result", direction: "out", portType: "data", type: "bool", label: "A∧B" },
    ],
    keywords: ["and", "&&", "logic", "与", "逻辑"],
    description: "逻辑与：两个输入都为 true 时输出 true",
  },
  {
    tag: "or", category: "data", label: "Or", icon: "∨",
    color: { bg: "#FAFAFA", border: "#E5E7EB" },
    defaultPorts: [
      { name: "a", direction: "in", portType: "data", type: "bool", label: "A" },
      { name: "b", direction: "in", portType: "data", type: "bool", label: "B" },
      { name: "result", direction: "out", portType: "data", type: "bool", label: "A∨B" },
    ],
    keywords: ["or", "||", "logic", "或", "逻辑"],
    description: "逻辑或：任一输入为 true 时输出 true",
  },
  {
    tag: "not", category: "data", label: "Not", icon: "¬",
    color: { bg: "#FAFAFA", border: "#E5E7EB" },
    defaultPorts: [
      { name: "a", direction: "in", portType: "data", type: "bool", label: "A" },
      { name: "result", direction: "out", portType: "data", type: "bool", label: "¬A" },
    ],
    keywords: ["not", "!", "invert", "非", "取反"],
    description: "逻辑非：取反输入值",
  },
  {
    tag: "concat", category: "data", label: "Concat", icon: "🔗",
    color: { bg: "#FAFAFA", border: "#E5E7EB" },
    defaultPorts: [
      { name: "a", direction: "in", portType: "data", type: "String", label: "A" },
      { name: "b", direction: "in", portType: "data", type: "String", label: "B" },
      { name: "result", direction: "out", portType: "data", type: "String", label: "A+B" },
    ],
    keywords: ["concat", "join", "string", "连接", "拼接", "字符串"],
    description: "拼接两个字符串",
  },
  {
    tag: "literal", category: "data", label: "Literal", icon: "📝",
    color: { bg: "#FAFAFA", border: "#E5E7EB" },
    defaultPorts: [
      { name: "value", direction: "out", portType: "data", type: "any", label: "值" },
    ],
    keywords: ["literal", "constant", "value", "常量", "字面量"],
    description: "常量值，点击节点在右侧面板配置",
    config: { value: "0", type: "i32" },
    panelEditor: "literal",
  },

  // ═══ Flow Control (branch/sequence/forEach — data ports only) ═══
  {
    tag: "branch", category: "flow", label: "Branch", icon: "🔀",
    color: { bg: "#FAFAFA", border: "#E5E7EB" },
    defaultPorts: [
      { name: "condition", direction: "in", portType: "data", type: "bool", label: "条件" },
      { name: "true", direction: "out", portType: "data", type: "any", label: "True" },
      { name: "false", direction: "out", portType: "data", type: "any", label: "False" },
    ],
    keywords: ["if", "else", "branch", "condition", "条件", "分支"],
    description: "根据条件选择输出。True/False 端口分别输出上游值",
  },
  {
    tag: "sequence", category: "flow", label: "Sequence", icon: "📋",
    color: { bg: "#FAFAFA", border: "#E5E7EB" },
    defaultPorts: [
      { name: "input", direction: "in", portType: "data", type: "any", label: "输入" },
      { name: "output0", direction: "out", portType: "data", type: "any", label: "1" },
      { name: "output1", direction: "out", portType: "data", type: "any", label: "2" },
    ],
    keywords: ["sequence", "chain", "order", "顺序", "序列"],
    description: "将输入值依次传递到多个输出",
    config: { outputs: 2 },
  },
  {
    tag: "forEach", category: "flow", label: "For Each", icon: "🔁",
    color: { bg: "#FAFAFA", border: "#E5E7EB" },
    defaultPorts: [
      { name: "collection", direction: "in", portType: "data", type: "any", label: "items" },
      { name: "current", direction: "out", portType: "data", type: "any", label: "item" },
      { name: "index", direction: "out", portType: "data", type: "i32", label: "index" },
    ],
    keywords: ["for", "each", "loop", "iterate", "循环", "遍历"],
    description: "遍历集合，输出当前元素和索引",
  },

  // ═══ UI Operations ═══
  {
    tag: "updateUIProperty", category: "ui", label: "Update UI", icon: "🎨",
    color: { bg: "#FAFAFA", border: "#E5E7EB" },
    defaultPorts: [
      { name: "value", direction: "in", portType: "data", type: "String", label: "新值" },
    ],
    keywords: ["ui", "update", "text", "display", "界面", "更新", "显示"],
    description: "更新 UI 元素的显示属性。点击节点选择目标元素和属性",
    config: { targetId: "", property: "label" },
    panelEditor: "element-selector",
  },

  // ═══ Function / Code ═══
  {
    tag: "fn", category: "code", label: "Fn Block", icon: "📦",
    color: { bg: "#FAFAFA", border: "#E5E7EB" },
    defaultPorts: [],
    keywords: ["fn", "function", "code", "inline", "函数", "代码"],
    description: "内联代码块。双击用 Monaco 编辑器编写复杂逻辑。端口从源码签名自动生成",
  },
  {
    tag: "customFn", category: "function", label: "Custom Function", icon: "🔧",
    color: { bg: "#FAFAFA", border: "#7C3AED" },
    defaultPorts: [
      { name: "return", direction: "out", portType: "data", type: "void", label: "返回" },
    ],
    keywords: ["custom", "function", "define", "自定义", "函数", "定义"],
    description: "自定义函数定义。点击节点设置名称、参数、返回类型。后方连接节点链作为函数体",
    config: { fnName: "", fnParams: [], fnReturnType: "void", description: "" },
    panelEditor: "function-signature",
  },
  {
    tag: "return", category: "function", label: "Return", icon: "↩",
    color: { bg: "#FAFAFA", border: "#9CA3AF" },
    defaultPorts: [
      { name: "value", direction: "in", portType: "data", type: "any", label: "返回值" },
    ],
    keywords: ["return", "output", "result", "返回", "输出"],
    description: "标记函数返回点。函数体末端必须有 return 节点。多分支 return 类型必须一致",
    panelEditor: "return-validator",
  },
];

// ── Registry ──

const byTag = new Map<string, NodeTypeConfig>();
const byCategory = new Map<string, NodeTypeConfig[]>();

for (const c of ALL_CONFIGS) {
  byTag.set(c.tag, c);
  const cat = byCategory.get(c.category) || [];
  cat.push(c);
  byCategory.set(c.category, cat);
}

/** Get visual config for a node tag. Falls back to a generic config. */
export function getNodeConfig(tag: string): NodeTypeConfig {
  return byTag.get(tag) || {
    tag,
    category: "code",
    label: tag,
    icon: "📦",
    color: { bg: "#FAFAFA", border: "#9CA3AF" },
    defaultPorts: [],
    keywords: [tag],
    description: "",
  };
}

/** All registered node configs (for search palette). */
export function getAllNodeConfigs(): NodeTypeConfig[] {
  return ALL_CONFIGS;
}

/** Get all configs in a category. */
export function getNodeConfigsByCategory(cat: string): NodeTypeConfig[] {
  return byCategory.get(cat) || [];
}

/** Category display metadata. */
export const CATEGORY_META: Record<string, { label: string; icon: string; order: number }> = {
  function: { label: "Functions",   icon: "🔧", order: 1 },
  variable: { label: "Variables",   icon: "📌", order: 2 },
  data:     { label: "Data & Logic", icon: "📊", order: 3 },
  flow:     { label: "Flow Control", icon: "🔀", order: 4 },
  ui:       { label: "UI Actions",  icon: "🎨", order: 5 },
  code:     { label: "Code Blocks", icon: "📦", order: 6 },
};

/** Tab type → allowed categories in NodePalette. */
export const TAB_CATEGORY_FILTER: Record<string, string[]> = {
  event: ["function", "variable", "data", "flow", "ui", "code"],
  logic: ["variable", "data", "flow", "ui", "code", "function"],
};
