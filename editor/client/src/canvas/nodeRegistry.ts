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
