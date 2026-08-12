// design/binding-sources.ts
// [内容] 槽位点名绑定的来源组装（M3 T3）：信号（组件/store/inline——collectSourceNodes
// 归一为 signal kind）+ fn 出口（画布 CodeFlow fn 节点）+ 醒着的画布元素。
// 纯函数，便于测试；ContentTab 注入 store 状态调用。
import type { ComponentDef } from "../../core/types/project-json";
import type { StoreDefinition } from "../../core/types/store";
import type { DebugValueEntry } from "../../store/types";
import { collectSourceNodes } from "../canvas/source-nodes";
import { fnsFromElements } from "../../services/codegen/flow-scope";
import type { BindingSource } from "./BindingPicker";

export interface BindingSourceState {
  componentName: string | null;
  componentDefs: Record<string, ComponentDef>;
  storeDefs: Record<string, StoreDefinition>;
  inlineStoreDefs: Record<string, StoreDefinition>;
  storeReferences: Record<string, string[]>;
  debugValues: Record<string, Record<string, DebugValueEntry>>;
  /** 画布全部元素（fn 出口 + 画布元素来源） */
  elements: Array<{ id: string; tag?: string; nodeType?: string; props?: Record<string, unknown> }>;
}

/** fn 元素 → 签名类型串（与 fixture 同形："() => void"） */
function fnTypeOf(props: Record<string, unknown> | undefined): string {
  const params = String(props?.params ?? "");
  const returnType = String(props?.returnType ?? "void");
  return `(${params}) => ${returnType}`;
}

export function buildBindingSources(state: BindingSourceState): BindingSource[] {
  const out: BindingSource[] = [];
  const { componentName, componentDefs, storeDefs, inlineStoreDefs, storeReferences, debugValues, elements } = state;

  // 1) 信号来源（组件 signals/computeds、引用 store、inline store）——kind 归一为 signal
  for (const n of collectSourceNodes({ componentName, componentDefs, storeDefs, inlineStoreDefs, storeReferences, debugValues })) {
    out.push({ id: `sig:${n.name}`, kind: "signal", name: n.name, type: n.type });
  }

  // 2) fn 出口（画布 CodeFlow fn 节点；未命名节点 fnsFromElements 已跳过）
  for (const f of fnsFromElements(elements as never)) {
    out.push({ id: `fn:${f.pid}`, kind: "fn", name: f.name, type: fnTypeOf(elements.find((e) => e.id === f.pid)?.props) });
  }

  // 3) 画布元素（醒着的 view 元素；显示名取 label 首行，兜底 id）
  for (const e of elements) {
    if (e.nodeType !== "view") continue;
    const label = typeof e.props?.label === "string" ? e.props.label : "";
    const name = label.trim().split("\n")[0] || e.id;
    out.push({ id: `el:${e.id}`, kind: "element", name, type: e.tag ?? "元素" });
  }

  return out;
}
