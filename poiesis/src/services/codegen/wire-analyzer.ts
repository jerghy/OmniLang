// poiesis/src/services/codegen/wire-analyzer.ts
// Analyzes wire connections between elements to extract signal/ref/dep bindings.

import type { ElementData, WireData } from "../../core/types";
import type { BindingResult } from "./types";

export function analyzeWires(
  elements: ElementData[],
  wires: WireData[],
  knownStores?: string[],
): BindingResult {
  const signalBindings = new Map<string, Array<{ elementId: string; propName: string }>>();
  const storeRefs = new Map<string, string[]>();
  const computedDeps = new Map<string, string[]>();

  const elMap = new Map(elements.map(e => [e.id, e]));

  for (const w of wires) {
    const src = elMap.get(w.fromId);
    const tgt = elMap.get(w.toId);
    if (!src || !tgt) continue;
    if (!("tag" in src) || !("tag" in tgt)) continue;

    const srcProps = src.props;

    // signal.value → view.prop
    if ((src.tag === "signal" || src.tag === "computed") && w.fromPort === "value" && tgt.nodeType === "view") {
      const name = srcProps.name as string;
      const bindings = signalBindings.get(name);
      if (bindings) {
        bindings.push({ elementId: tgt.id, propName: w.toPort });
      } else {
        signalBindings.set(name, [{ elementId: tgt.id, propName: w.toPort }]);
      }
    }
    // store.items → view
    if (src.tag === "store" && w.fromPort === "items" && tgt.nodeType === "view") {
      const name = srcProps.name as string;
      if (!storeRefs.has(name)) storeRefs.set(name, []);
      storeRefs.get(name)!.push(tgt.id);
    }
    // → computed.deps
    if (tgt.tag === "computed" && w.toPort === "deps") {
      const srcName = srcProps.name as string;
      const tgtProps = tgt.props;
      const tgtName = tgtProps.name as string;
      if (!computedDeps.has(tgtName)) computedDeps.set(tgtName, []);
      computedDeps.get(tgtName)!.push(srcName);
    }
  }

  // 语法节点（assign/for-in/condition/return 等）的 props/source 表达式里可能引用
  // "Store.信号"（公式框候选含 store 信号）——wire 上无痕，但生成的 .omni 需要
  // import 该 store（否则 WASM 门禁 Undeclared variable 拦截保存）。
  // 只收集 knownStores 名单内的 PascalCase 引用（防止 Math.random() 类误报
  // 生成指向不存在 store 文件的 import）。
  const STORE_REF_RE = /([A-Z][A-Za-z0-9_]*)\.[A-Za-z_]\w*/g;
  if (knownStores && knownStores.length > 0) {
    const known = new Set(knownStores);
    for (const el of elements) {
      if (el.nodeType !== "codeblock") continue;
      const candidates = new Set<string>();
      const scan = (v: unknown) => {
        if (typeof v === "string") {
          for (const m of v.matchAll(STORE_REF_RE)) candidates.add(m[1]!);
        }
      };
      for (const v of Object.values(el.props ?? {})) scan(v);
      if (typeof el.source === "string") scan(el.source);
      for (const name of candidates) {
        if (known.has(name) && !storeRefs.has(name)) storeRefs.set(name, []);
      }
    }
  }

  // unwired props
  const unwiredProps: BindingResult["unwiredProps"] = [];
  const wiredProps = new Set<string>();
  for (const [, bindings] of signalBindings) {
    for (const b of bindings) wiredProps.add(`${b.elementId}:${b.propName}`);
  }
  for (const el of elements.filter(e => e.nodeType === "view")) {
    for (const key of Object.keys(el.props)) {
      if (!wiredProps.has(`${el.id}:${key}`) && !key.startsWith("on")) {
        unwiredProps.push({ elementId: el.id, tag: el.tag, propName: key });
      }
    }
  }

  return { signalBindings, storeRefs, computedDeps, unwiredProps };
}
