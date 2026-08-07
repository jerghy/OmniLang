// poiesis/src/services/codegen/syntax-nodes.ts
// 语法节点（let/assign/store-call/call）的 props 结构与代码生成。
// CodeFlowCanvas 中"一个语法一个节点"的语句级节点核心：
//  - formatSyntaxNode: props → OmniLang 语句文本（GraphCompiler 与 UI 预览共用，单一来源）
//  - defaultSyntaxProps: 新节点默认值（占位符取自 learn-omniview 真实使用统计）

export interface SyntaxNodeProps {
  /** 节点显示标签（throw/await 等仅作占位，UI label 以 TAG_CN 为准） */
  label?: string;
  /** let: 变量名；assign: 赋值目标（signal/变量/store 信号） */
  name?: string;
  mutable?: boolean;
  varType?: string;
  value?: string;
  target?: string;
  operator?: string;
  storeName?: string;
  action?: string;
  args?: string;
  emit?: boolean;
  fnName?: string;
  /** while: 循环条件表达式 */
  condition?: string;
  /** for-in: 循环变量名 */
  varName?: string;
  /** for-in: 可迭代表达式 */
  iterable?: string;
}

export const ASSIGN_OPERATORS = ["=", "+=", "-=", "*=", "/=", "%="];

/** 新节点的默认 props（learn-omniview 高频模式：let 68×、let mut 21×、store 动作 6+、emit 4×）。 */
export function defaultSyntaxProps(tag: string): SyntaxNodeProps {
  switch (tag) {
    case "let":
      return { name: "x", mutable: false, value: "0" };
    case "assign":
      return { target: "count", operator: "=", value: "0" };
    case "store-call":
      return { storeName: "TodoStore", action: "add", args: "" };
    case "call":
      return { fnName: "close", args: "", emit: false };
    case "throw":
      return { label: "throw", value: "" };
    case "await":
      return { label: "await", value: "" };
    case "while":
      return { condition: "count < 10" };
    case "for-in":
      return { varName: "item", iterable: "items" };
    default:
      return {};
  }
}

/** props → OmniLang 语句文本（无缩进、无 @pid 注释，带尾分号）。 */
export function formatSyntaxNode(tag: string, props: SyntaxNodeProps): string {
  switch (tag) {
    case "let": {
      const mut = props.mutable ? " mut" : "";
      const type = props.varType ? `: ${props.varType}` : "";
      const value = props.value != null && props.value !== "" ? ` = ${props.value}` : "";
      return `let${mut} ${props.name ?? "x"}${type}${value};`;
    }
    case "assign":
      return `${props.target ?? "x"} ${props.operator ?? "="} ${props.value ?? ""};`;
    case "store-call":
      return `${props.storeName ?? "Store"}.${props.action ?? "action"}(${props.args ?? ""});`;
    case "call":
      return `${props.emit ? "emit " : ""}${props.fnName ?? "fn"}(${props.args ?? ""});`;
    case "throw":
      return `throw ${props.value ?? ""};`;
    case "await":
      return `await ${props.value ?? ""};`;
    case "while":
      return `while ${props.condition ?? "true"} {`;
    case "for-in":
      return `for ${props.varName ?? "item"} in ${props.iterable ?? "items"} {`;
    default:
      return "";
  }
}
