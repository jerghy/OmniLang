// components/canvas/source-nodes.ts
// 源节点收集（M2）：当前组件 tab 的数据来源一览——组件 signals/computeds、引用 store
// signals/getters、inline store signals。值 = debugValues 覆盖优先（valid 时取 text），
// defaultValue 兜底（computeds/getters 无 defaultValue 时缺省 undefined）。
// 收集逻辑与 panels/inspector/shared.ts useSignalItems 同源（对齐避免漂移）；
// 故意分歧：inline store getter 不收（kind 联合无 inline-getter 变体）。
import type { ComponentDef } from "../../core/types/project-json";
import type { StoreDefinition } from "../../core/types/store";
import type { DebugValueEntry } from "../../store/types";

export interface SourceNode {
  id: string;          // `sig:${pid}` | `computed:${pid}` | `store:${storeName}.${pid}` | `inline:${key}.${pid}`
  kind: "signal" | "computed" | "store-signal" | "store-getter" | "inline-signal";
  name: string;        // 显示名（store 信号带 store 前缀；inline 不带）
  type: string;
  value: unknown;      // 当前值（debugValues 覆盖优先；defaultValue 兜底）
  ownerKey: string;    // setDebugValue 的 ownerKey（组件名 | store 名 | "Comp.Store"）
}

export interface SourceNodeState {
  componentName: string | null;
  componentDefs: Record<string, ComponentDef>;
  storeDefs: Record<string, StoreDefinition>;
  inlineStoreDefs: Record<string, StoreDefinition>;
  storeReferences: Record<string, string[]>;
  debugValues: Record<string, Record<string, DebugValueEntry>>;
}

/** debugValues 覆盖值：valid 时取回显原文 text；否则（非法/缺失）undefined → defaultValue 兜底 */
function debugValueOf(debugValues: SourceNodeState["debugValues"], ownerKey: string, name: string): unknown {
  const entry = debugValues[ownerKey]?.[name];
  return entry && entry.valid ? entry.text : undefined;
}

/** 类型防御：ComputedEntry/StoreGetterDef 无 defaultValue 字段，但 fixture/旧数据可能携带 */
function defaultOf(entry: unknown): unknown {
  return (entry as { defaultValue?: unknown } | null)?.defaultValue;
}

export function collectSourceNodes(state: SourceNodeState): SourceNode[] {
  const { componentName, componentDefs, storeDefs, inlineStoreDefs, storeReferences, debugValues } = state;
  if (!componentName) return [];
  const out: SourceNode[] = [];

  const compDef = componentDefs[componentName];
  if (compDef) {
    // 组件 signals
    for (const s of compDef.signals ?? []) {
      out.push({
        id: `sig:${s.pid}`,
        kind: "signal",
        name: s.name,
        type: s.type,
        value: debugValueOf(debugValues, componentName, s.name) ?? s.defaultValue,
        ownerKey: componentName,
      });
    }
    // 组件 computeds
    for (const c of compDef.computeds ?? []) {
      out.push({
        id: `computed:${c.pid}`,
        kind: "computed",
        name: c.name,
        type: c.returnType,
        value: debugValueOf(debugValues, componentName, c.name) ?? defaultOf(c),
        ownerKey: componentName,
      });
    }
  }

  // 引用 store 的 signals/getters（带 store 名前缀）
  // storeReferences 方向 = store → [components]（writer: registerStoreRef），
  // 读方向 = entries 过滤出包含当前组件的 store 列表（与 flow-scope-hook 同源）。
  const referencedStores = Object.entries(storeReferences)
    .filter(([, comps]) => comps.includes(componentName))
    .map(([storeName]) => storeName);
  for (const storeName of referencedStores) {
    const storeDef = storeDefs[storeName];
    if (!storeDef) continue;
    for (const sig of storeDef.signals ?? []) {
      out.push({
        id: `store:${storeName}.${sig.pid}`,
        kind: "store-signal",
        name: `${storeName}.${sig.name}`,
        type: sig.type,
        value: debugValueOf(debugValues, storeName, sig.name) ?? sig.defaultValue,
        ownerKey: storeName,
      });
    }
    for (const g of storeDef.getters ?? []) {
      out.push({
        id: `store:${storeName}.${g.pid}`,
        kind: "store-getter",
        name: `${storeName}.${g.name}`,
        type: g.returnType,
        value: debugValueOf(debugValues, storeName, g.name) ?? defaultOf(g),
        ownerKey: storeName,
      });
    }
  }

  // inline store 的 signals（key = "Component.Store"，组件内可直接引用不带前缀）
  for (const [key, storeDef] of Object.entries(inlineStoreDefs)) {
    if (!key.startsWith(componentName + ".")) continue;
    for (const sig of storeDef.signals ?? []) {
      out.push({
        id: `inline:${key}.${sig.pid}`,
        kind: "inline-signal",
        name: sig.name,
        type: sig.type,
        value: debugValueOf(debugValues, key, sig.name) ?? sig.defaultValue,
        ownerKey: key,
      });
    }
  }

  return out;
}
