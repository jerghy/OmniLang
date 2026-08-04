// poiesis/src/services/codegen/flow-scope.ts
// 节点内联编辑的补全候选数据层 + 轻量即时校验。
// 纯函数：从 store 现有数据（componentDefs/storeDefs/inlineStoreDefs/elements）收集候选，零新数据源。
import type { ComponentDef } from "../../core/types/project-json";
import type { StoreDefinition } from "../../core/types/store";
import type { CodeblockElementData } from "../../core/types/element";

export interface ScopeParam { name: string; type: string }
export interface ScopeStore {
  name: string;
  actions: Array<{ name: string; params: ScopeParam[] }>;
  signals: Array<{ name: string; type: string }>;
}
export interface FlowScope {
  signals: Array<{ name: string; type: string }>;
  props: Array<{ name: string; type: string }>;
  computeds: Array<{ name: string; returnType: string }>;
  events: Array<{ name: string }>;
  fns: Array<{ name: string; params: ScopeParam[] }>;
  stores: ScopeStore[];
  fnParams: ScopeParam[];
  letVars: Array<{ name: string }>;
}

/** 与 FnInspector 相同的 params 字符串格式（"name: Type, name: Type"）。 */
export function parseFnParamsStr(s: string): ScopeParam[] {
  if (!s) return [];
  return s.split(",").map((p) => {
    const [pName, pType] = p.trim().split(":").map((t) => t.trim());
    return { name: pName || "", type: pType || "String" };
  });
}

export function buildFlowScope(input: {
  componentDef?: ComponentDef;
  storeDefs: Record<string, StoreDefinition>;
  inlineStoreDefs: Record<string, StoreDefinition>;
  storesUsed: string[];
  fnParamsStr?: string;
  fnElements: CodeblockElementData[];
}): FlowScope {
  const { componentDef, storeDefs, inlineStoreDefs, storesUsed } = input;
  const allStoreDefs = { ...inlineStoreDefs, ...storeDefs };
  const storeNames = new Set([...Object.keys(allStoreDefs), ...storesUsed]);

  return {
    signals: componentDef?.signals?.map((s) => ({ name: s.name, type: s.type })) ?? [],
    props: componentDef?.props?.map((p) => ({ name: p.name, type: p.type })) ?? [],
    computeds: componentDef?.computeds?.map((c) => ({ name: c.name, returnType: c.returnType })) ?? [],
    events: componentDef?.events?.map((e) => ({ name: e.name })) ?? [],
    fns: componentDef?.fns?.map((f) => ({ name: f.name, params: f.params ?? [] })) ?? [],
    stores: [...storeNames]
      .filter(Boolean)
      .map((name) => {
        const def = allStoreDefs[name];
        return {
          name,
          actions: def?.actions?.map((a) => ({ name: a.name, params: a.params ?? [] })) ?? [],
          signals: def?.signals?.map((s) => ({ name: s.name, type: s.type })) ?? [],
        };
      }),
    fnParams: parseFnParamsStr(input.fnParamsStr ?? ""),
    letVars: input.fnElements
      .filter((e) => e.tag === "let")
      .map((e) => ({ name: (e.props?.name as string) ?? "" }))
      .filter((v) => v.name !== ""),
  };
}

/** 前缀子串匹配候选，空 query 返回全部（限 8 项）。 */
export function filterCandidates(query: string, candidates: Array<{ name: string; type?: string }>): Array<{ name: string; type?: string }> {
  const q = query.trim();
  if (!q) return candidates.slice(0, 8);
  return candidates.filter((c) => c.name.startsWith(q)).slice(0, 8);
}

const IDENT = /^[a-zA-Z_]\w*$/;
const TARGET = /^[a-zA-Z_]\w*(\.[a-zA-Z_]\w*)?$/;

/** 轻量即时校验：只提示不阻塞（保存时 WASM 门禁最终裁决）。 */
export function validateSyntaxProps(
  tag: string,
  props: Record<string, unknown>,
  scope: FlowScope,
): Array<{ field: string; message: string }> {
  const errors: Array<{ field: string; message: string }> = [];
  if (tag === "let") {
    const name = (props.name as string) ?? "";
    if (!IDENT.test(name)) errors.push({ field: "name", message: "变量名需为合法标识符（字母/下划线开头）" });
  }
  if (tag === "assign") {
    const target = (props.target as string) ?? "";
    if (!TARGET.test(target)) errors.push({ field: "target", message: "目标需为标识符或 Store.信号" });
  }
  if (tag === "store-call") {
    const storeName = (props.storeName as string) ?? "";
    const action = (props.action as string) ?? "";
    const store = scope.stores.find((s) => s.name === storeName);
    if (store && !store.actions.some((a) => a.name === action)) {
      errors.push({ field: "action", message: `action「${action}」不在 ${storeName} 中` });
    }
  }
  if (tag === "call") {
    const fnName = (props.fnName as string) ?? "";
    if (!fnName) errors.push({ field: "fnName", message: "函数名不能为空" });
  }
  return errors;
}

export interface FormulaCandidate { value: string; label: string }
export interface FormulaGroup { label: string; items: FormulaCandidate[] }

/** 「store 信号」候选分组 label：flow-scope（组装）与 SyntaxNode（消费）共用，避免跨模块字符串耦合。 */
export const STORE_SIGNALS_LABEL = "store 信号";

/** 公式框候选分组：变量/参数/信号/属性/计算属性/store 信号/store action/component fn。 */
export function buildExpressionCandidates(scope: FlowScope, letVars: Array<{ name: string }>): FormulaGroup[] {
  const groups: FormulaGroup[] = [];
  const vars = [
    ...letVars.map((v) => v.name),
    ...scope.fnParams.map((p) => p.name),
  ].filter(Boolean);
  if (vars.length > 0) groups.push({ label: "变量", items: vars.map((n) => ({ value: n, label: n })) });
  if (scope.signals.length > 0) groups.push({ label: "信号", items: scope.signals.map((s) => ({ value: s.name, label: `${s.name}: ${s.type}` })) });
  if (scope.props.length > 0) groups.push({ label: "属性", items: scope.props.map((p) => ({ value: p.name, label: `${p.name}: ${p.type}` })) });
  if (scope.computeds.length > 0) groups.push({ label: "计算属性", items: scope.computeds.map((c) => ({ value: c.name, label: `${c.name}: ${c.returnType}` })) });
  const storeSignals = scope.stores.flatMap((s) => s.signals.map((sig) => ({ value: `${s.name}.${sig.name}`, label: `${s.name}.${sig.name}: ${sig.type}` })));
  if (storeSignals.length > 0) groups.push({ label: STORE_SIGNALS_LABEL, items: storeSignals });
  const calls = [
    ...scope.fns.map((f) => ({ value: f.name, label: `${f.name}(${f.params.map((p) => p.name).join(", ")})` })),
    ...scope.stores.flatMap((s) => s.actions.map((a) => ({ value: `${s.name}.${a.name}`, label: `${s.name}.${a.name}(${a.params.map((p) => `${p.name}: ${p.type}`).join(", ")})` }))),
  ];
  if (calls.length > 0) groups.push({ label: "函数", items: calls });
  return groups;
}

/** assign target 下拉候选：变量类 + store 信号（Store.信号 格式）。 */
export function buildTargetOptions(scope: FlowScope, letVars: Array<{ name: string }>): FormulaCandidate[] {
  const names = [
    ...letVars.map((v) => v.name),
    ...scope.fnParams.map((p) => p.name),
    ...scope.signals.map((s) => s.name),
    ...scope.props.map((p) => p.name),
    ...scope.computeds.map((c) => c.name),
  ].filter(Boolean);
  const storeSignals = scope.stores.flatMap((s) => s.signals.map((sig) => `${s.name}.${sig.name}`));
  return [...new Set([...names, ...storeSignals])].map((n) => ({ value: n, label: n }));
}
