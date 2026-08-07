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
     *  空合并短路：无新元素不重建数组引用。 */
    materializeLibGraphs: (filePath: string) => {
      const s = get();
      const def = s.libDefs[filePath];
      if (!def) return;
      const newElements: ElementData[] = [...s.elements];
      const newWires: WireData[] = [...s.wires];
      const existIds = new Set(newElements.map((e) => e.id));
      let layoutIdx = 0;
      for (const f of def.fns) {
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
      if (newElements.length === s.elements.length && newWires.length === s.wires.length) return;
      set({ elements: newElements, wires: newWires });
    },
  };
}
