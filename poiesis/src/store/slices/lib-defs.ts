// poiesis/src/store/slices/lib-defs.ts
// Code tab（lib 文件）声明集合 + fn 图物化（对标 element-crud 的 materializeFnGraphs）。

import type { LibFileDef } from "../../services/codegen/types";
import type { ElementData, WireData } from "../../core/types";
import type { PoiesisState } from "../types";

type SetFn = (partial: Partial<PoiesisState> | ((state: PoiesisState) => Partial<PoiesisState>)) => void;
type GetFn = () => PoiesisState;

export function createLibDefsSlice(set: SetFn, get: GetFn) {
  return {
    setLibDef: (filePath: string, def: LibFileDef) => {
      set((s) => ({ libDefs: { ...s.libDefs, [filePath]: def } }));
    },

    /** 物化 lib 文件的 fn 图到当前画布（对标 materializeFnGraphs，element-crud.ts:176）。
     *  幂等：pid 去重、已存在不覆盖；缺 position 时确定性行布局。
     *  空合并短路：无新元素不重建数组引用。
     *  MUST-FIX 1（最终 review）：ROOT action 补登——parseComponentAction 只把 action
     *  图登记进 codeFlowGraphs，ROOT action（无组件上下文）从不进入 libDef.fns
     *  （fns.push 仅 parseRootFn 独占）。不补登 → 重载后本函数跳过 → 画布丢失 action →
     *  保存对齐（useSaveProject workingPids 过滤）把 action 声明从文件永久删除（静默丢失链）。
     *  pid 归属校验与 fn 路径对称：入口元素 tag "action" 即图的所有权凭证（id 即 pid）；
     *  被组件 fnRegistry 认领的 action（componentDefs[comp].fns 含其 pid）是组件成员，
     *  不得补登 lib（防跨容器污染）。store action 入口 tag 为 "fn"（parser.ts store 路径
     *  synthesizeFnEntry 默认 entryTag），天然不在此扫描范围内。 */
    materializeLibGraphs: (filePath: string) => {
      const s = get();
      const def = s.libDefs[filePath];
      if (!def) return;
      const newElements: ElementData[] = [...s.elements];
      const newWires: WireData[] = [...s.wires];
      const existIds = new Set(newElements.map((e) => e.id));
      const claimedActionPids = new Set(
        Object.values(s.componentDefs).flatMap((c) => (c.fns ?? []).map((f) => f.pid).filter((p): p is string => !!p)),
      );
      const knownPids = new Set(def.fns.map((f) => f.pid).filter((p): p is string => !!p));
      let fnsChanged = false;
      const fns = [...def.fns];
      for (const [name, graph] of Object.entries(s.codeFlowGraphs)) {
        const actionEl = graph.elements.find((e) => e.tag === "action");
        if (!actionEl) continue;
        if (knownPids.has(actionEl.id) || claimedActionPids.has(actionEl.id)) continue;
        fns.push({
          pid: actionEl.id,
          name,
          params: ((actionEl.props?.params as string) || "").split(",").flatMap((p) => {
            const [pName, pType] = p.trim().split(":").map((x) => x.trim());
            return pName ? [{ name: pName, type: pType || "" }] : [];
          }),
          returnType: "",
          body: "",
          pub: false,
          isAsync: Boolean(actionEl.props?.isAsync),
          declKind: "action",
        });
        knownPids.add(actionEl.id);
        fnsChanged = true;
      }
      let layoutIdx = 0;
      for (const f of fns) {
        const graph = s.codeFlowGraphs[f.name];
        if (!graph) continue;
        // pid 归属校验：fn 入口元素须以 f.pid 为 id——跨文件同名 fn 不混入
        const fnEl = graph.elements.find((e) => e.id === f.pid);
        if (!fnEl) continue;
        for (const e of graph.elements) {
          if (existIds.has(e.id)) continue;
          newElements.push({
            ...e,
            position: e.position ?? s.persistedPositions[e.id] ?? { x: 0, y: layoutIdx++ * 60 },
          });
          existIds.add(e.id);
        }
        for (const w of graph.wires) {
          if (!newWires.some((nw) => nw.id === w.id)) newWires.push(w);
        }
      }
      if (newElements.length === s.elements.length && newWires.length === s.wires.length && !fnsChanged) return;
      set({
        elements: newElements,
        wires: newWires,
        ...(fnsChanged ? { libDefs: { ...s.libDefs, [filePath]: { ...def, fns } } } : {}),
      });
    },
  };
}
