// editor/client/src/canvas/nodeRegistry.ts

import type { PortData } from "../store/editor";

export interface NodeTypeConfig {
  tag: string;
  category: "flow" | "event" | "action" | "data" | "variable" | "code";
  label: string;
  icon: string;
  color: { bg: string; border: string };
  defaultPorts: PortData[];
  /** Search keywords for the right-click palette */
  keywords: string[];
  /** Description shown in search results */
  description: string;
  /** Default config values (op, value, targetId, etc.) */
  config?: Record<string, unknown>;
}

// ── Code block types (existing, backward compatible) ──

const CODE_BLOCK_CONFIGS: NodeTypeConfig[] = [
  {
    tag: "signal", category: "variable", label: "Signal", icon: "⚡",
    color: { bg: "#FEF3C7", border: "#D97706" },
    defaultPorts: [
      { name: "value", direction: "out", portType: "data", type: "any", label: "值" },
      { name: "set", direction: "in", portType: "data", type: "any", label: "设置" },
    ],
    keywords: ["signal", "state", "variable", "value", "状态", "变量"],
    description: "响应式状态变量，存储和读取值",
  },
  {
    tag: "computed", category: "data", label: "Computed", icon: "🔄",
    color: { bg: "#DBEAFE", border: "#2563EB" },
    defaultPorts: [
      { name: "value", direction: "out", portType: "data", type: "any", label: "计算结果" },
      { name: "deps", direction: "in", portType: "data", type: "any", label: "依赖" },
    ],
    keywords: ["computed", "derived", "calculation", "计算", "派生"],
    description: "从其他值派生计算，依赖变化时自动更新",
  },
  {
    tag: "action", category: "action", label: "Action", icon: "▶️",
    color: { bg: "#FCE7F3", border: "#DB2777" },
    defaultPorts: [
      { name: "call", direction: "in", portType: "flow", type: "void", label: "调用" },
      { name: "result", direction: "out", portType: "data", type: "any", label: "返回" },
    ],
    keywords: ["action", "handler", "event", "function", "处理", "事件"],
    description: "事件处理函数，接收事件并执行逻辑",
  },
  {
    tag: "store", category: "variable", label: "Store", icon: "🗄️",
    color: { bg: "#F5F3FF", border: "#7C3AED" },
    defaultPorts: [
      { name: "items", direction: "out", portType: "data", type: "any", label: "数据" },
    ],
    keywords: ["store", "collection", "database", "存储", "集合"],
    description: "数据集合容器",
  },
  {
    tag: "fn", category: "code", label: "Function", icon: "📦",
    color: { bg: "#ECFDF5", border: "#059669" },
    defaultPorts: [],
    keywords: ["fn", "function", "定义", "函数"],
    description: "外部函数定义（不在画布上显示端口）",
  },
  {
    tag: "fn-call", category: "action", label: "Fn Call", icon: "📞",
    color: { bg: "#F0FDF4", border: "#10B981" },
    defaultPorts: [],
    keywords: ["call", "invoke", "调用"],
    description: "调用已定义的函数",
  },
];

// ── Placeholder configs for visual scripting nodes (Round 2-3 will flesh these out) ──

const VISUAL_NODE_CONFIGS: NodeTypeConfig[] = [
  // Event nodes
  {
    tag: "onPress", category: "event", label: "On Press", icon: "🔘",
    color: { bg: "#FCE7F3", border: "#DB2777" },
    defaultPorts: [
      { name: "flowOut", direction: "out", portType: "flow", type: "void", label: "→" },
      { name: "event", direction: "out", portType: "data", type: "MouseEvent", label: "event" },
    ],
    keywords: ["press", "click", "tap", "button", "点击", "按钮"],
    description: "按钮点击时触发执行流",
  },
  {
    tag: "onChange", category: "event", label: "On Change", icon: "📝",
    color: { bg: "#FCE7F3", border: "#DB2777" },
    defaultPorts: [
      { name: "flowOut", direction: "out", portType: "flow", type: "void", label: "→" },
      { name: "value", direction: "out", portType: "data", type: "String", label: "value" },
    ],
    keywords: ["change", "input", "type", "text", "输入", "变化"],
    description: "输入框内容变化时触发执行流",
  },
  // Action nodes
  {
    tag: "setVariable", category: "action", label: "Set Variable", icon: "⚡",
    color: { bg: "#FEF3C7", border: "#D97706" },
    defaultPorts: [
      { name: "flowIn", direction: "in", portType: "flow", type: "void", label: "→" },
      { name: "flowOut", direction: "out", portType: "flow", type: "void", label: "→" },
      { name: "value", direction: "in", portType: "data", type: "any", label: "value" },
    ],
    keywords: ["set", "assign", "variable", "设置", "赋值"],
    description: "设置变量的值",
  },
  {
    tag: "runCodeBlock", category: "action", label: "Run Code Block", icon: "▶️",
    color: { bg: "#FCE7F3", border: "#DB2777" },
    defaultPorts: [
      { name: "flowIn", direction: "in", portType: "flow", type: "void", label: "→" },
      { name: "flowOut", direction: "out", portType: "flow", type: "void", label: "→" },
    ],
    keywords: ["run", "execute", "code", "block", "运行", "执行", "代码"],
    description: "运行一个代码块，可从 fn 库拖入自动生成输入端口",
  },
  // Data nodes
  {
    tag: "getVariable", category: "data", label: "Get Variable", icon: "📌",
    color: { bg: "#ECFDF5", border: "#059669" },
    defaultPorts: [
      { name: "value", direction: "out", portType: "data", type: "any", label: "value" },
    ],
    keywords: ["get", "read", "variable", "读取", "变量"],
    description: "读取变量的当前值",
  },
  {
    tag: "add", category: "data", label: "Add", icon: "➕",
    color: { bg: "#ECFDF5", border: "#059669" },
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
    color: { bg: "#ECFDF5", border: "#059669" },
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
    color: { bg: "#ECFDF5", border: "#059669" },
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
    color: { bg: "#ECFDF5", border: "#059669" },
    defaultPorts: [
      { name: "a", direction: "in", portType: "data", type: "any", label: "A" },
      { name: "b", direction: "in", portType: "data", type: "any", label: "B" },
      { name: "result", direction: "out", portType: "data", type: "any", label: "A÷B" },
    ],
    keywords: ["div", "divide", "division", "除", "除法"],
    description: "两数相除",
  },
  // Round 3: More data nodes
  {
    tag: "compare", category: "data", label: "Compare", icon: "⚖️",
    color: { bg: "#FEF3C7", border: "#D97706" },
    defaultPorts: [
      { name: "a", direction: "in", portType: "data", type: "any", label: "A" },
      { name: "b", direction: "in", portType: "data", type: "any", label: "B" },
      { name: "result", direction: "out", portType: "data", type: "bool", label: "A==B" },
    ],
    keywords: ["compare", "equal", "greater", "less", "比较", "等于"],
    description: "比较两个值，返回布尔结果",
    config: { op: "==", ops: ["==", "!=", ">", "<", ">=", "<="] },
  },
  {
    tag: "and", category: "data", label: "And", icon: "∧",
    color: { bg: "#DBEAFE", border: "#2563EB" },
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
    color: { bg: "#DBEAFE", border: "#2563EB" },
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
    color: { bg: "#DBEAFE", border: "#2563EB" },
    defaultPorts: [
      { name: "a", direction: "in", portType: "data", type: "bool", label: "A" },
      { name: "result", direction: "out", portType: "data", type: "bool", label: "¬A" },
    ],
    keywords: ["not", "!", "invert", "非", "取反"],
    description: "逻辑非：取反输入值",
  },
  {
    tag: "concat", category: "data", label: "Concat", icon: "🔗",
    color: { bg: "#FCE7F3", border: "#DB2777" },
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
    color: { bg: "#F3F4F6", border: "#6B7280" },
    defaultPorts: [
      { name: "value", direction: "out", portType: "data", type: "any", label: "值" },
    ],
    keywords: ["literal", "constant", "value", "常量", "字面量"],
    description: "常量值，双击或右键配置",
    config: { value: "0", type: "i32" },
  },
  // Round 3: Flow control nodes
  {
    tag: "sequence", category: "flow", label: "Sequence", icon: "📋",
    color: { bg: "#DBEAFE", border: "#2563EB" },
    defaultPorts: [
      { name: "flowIn", direction: "in", portType: "flow", type: "void", label: "→" },
      { name: "flowOut0", direction: "out", portType: "flow", type: "void", label: "1" },
      { name: "flowOut1", direction: "out", portType: "flow", type: "void", label: "2" },
    ],
    keywords: ["sequence", "chain", "order", "顺序", "序列"],
    description: "按顺序依次执行多个输出",
    config: { outputs: 2 },
  },
  {
    tag: "forEach", category: "flow", label: "For Each", icon: "🔁",
    color: { bg: "#DBEAFE", border: "#2563EB" },
    defaultPorts: [
      { name: "flowIn", direction: "in", portType: "flow", type: "void", label: "→" },
      { name: "loopBody", direction: "out", portType: "flow", type: "void", label: "Loop" },
      { name: "completed", direction: "out", portType: "flow", type: "void", label: "Done" },
      { name: "collection", direction: "in", portType: "data", type: "any", label: "items" },
      { name: "current", direction: "out", portType: "data", type: "any", label: "item" },
      { name: "index", direction: "out", portType: "data", type: "i32", label: "index" },
    ],
    keywords: ["for", "each", "loop", "iterate", "循环", "遍历"],
    description: "遍历集合，为每个元素执行循环体",
  },
  // Round 4: UI Property node
  {
    tag: "updateUIProperty", category: "action", label: "Update UI", icon: "🎨",
    color: { bg: "#FCE7F3", border: "#DB2777" },
    defaultPorts: [
      { name: "flowIn", direction: "in", portType: "flow", type: "void", label: "→" },
      { name: "flowOut", direction: "out", portType: "flow", type: "void", label: "→" },
      { name: "value", direction: "in", portType: "data", type: "String", label: "text" },
    ],
    keywords: ["ui", "update", "text", "display", "界面", "更新", "显示"],
    description: "更新 UI 元素的显示文本。目标元素在 Inspector 中选择",
    config: { targetId: "", property: "label" },
  },
  // Flow nodes
  {
    tag: "branch", category: "flow", label: "Branch", icon: "🔀",
    color: { bg: "#DBEAFE", border: "#2563EB" },
    defaultPorts: [
      { name: "flowIn", direction: "in", portType: "flow", type: "void", label: "→" },
      { name: "true", direction: "out", portType: "flow", type: "void", label: "True" },
      { name: "false", direction: "out", portType: "flow", type: "void", label: "False" },
      { name: "condition", direction: "in", portType: "data", type: "bool", label: "条件" },
    ],
    keywords: ["if", "else", "branch", "condition", "条件", "分支"],
    description: "根据条件分支执行",
  },
];

// ── Registry ──

const ALL_CONFIGS = [...CODE_BLOCK_CONFIGS, ...VISUAL_NODE_CONFIGS];
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
    color: { bg: "#F3F4F6", border: "#9CA3AF" },
    defaultPorts: [],
    keywords: [tag],
    description: "",
  };
}

/** All registered node configs (for search palette — Round 2). */
export function getAllNodeConfigs(): NodeTypeConfig[] {
  return ALL_CONFIGS;
}

/** Get all configs in a category. */
export function getNodeConfigsByCategory(cat: string): NodeTypeConfig[] {
  return byCategory.get(cat) || [];
}

/** Category display metadata. */
export const CATEGORY_META: Record<string, { label: string; icon: string; order: number }> = {
  event:    { label: "Events",     icon: "⚡", order: 1 },
  action:   { label: "Actions",    icon: "⚙️", order: 2 },
  flow:     { label: "Flow Control", icon: "🔀", order: 3 },
  data:     { label: "Data",       icon: "📊", order: 4 },
  variable: { label: "Variables",  icon: "📌", order: 5 },
  code:     { label: "Code Blocks", icon: "📦", order: 6 },
};
