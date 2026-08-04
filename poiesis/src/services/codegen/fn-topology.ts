// poiesis/src/services/codegen/fn-topology.ts
// 按 wire 拓扑推断 let 变量归属（多 fn 隔离）：从每个 fn 元素沿 flow-out 正向 BFS，
// 收集 tag==="let" 节点；findOwnerFnId 沿 flow-in 反向反推所属 fn。
import type { CodeblockElementData } from "../../core/types/element";
import type { WireData } from "../../core/types/wire";

/** 每个 fn 元素可达的 let 节点（按拓扑顺序）。孤岛节点不入任何 fn 的集合。 */
export function collectLetVarsPerFn(elements: CodeblockElementData[], wires: WireData[]): Map<string, Array<{ name: string }>> {
  const out = new Map<string, Array<{ name: string }>>();
  const outgoing = new Map<string, WireData[]>();
  for (const w of wires) {
    const list = outgoing.get(w.fromId) ?? [];
    list.push(w);
    outgoing.set(w.fromId, list);
  }
  // id → element 索引：BFS 内查找从 O(N) 摊到 O(1)（elements 不可变，单次预处理）
  const elemById = new Map(elements.map((e) => [e.id, e]));
  const fnEls = elements.filter((e) => e.tag === "fn");
  for (const fnEl of fnEls) {
    const visited = new Set<string>([fnEl.id]);
    const queue = [fnEl.id];
    const lets: Array<{ name: string }> = [];
    while (queue.length > 0) {
      const cur = queue.shift()!;
      for (const w of outgoing.get(cur) ?? []) {
        if (visited.has(w.toId)) continue;
        visited.add(w.toId);
        const target = elemById.get(w.toId);
        // fn 边界截断：命中另一 fn 节点时停止下探——其子树归该 fn 自身 BFS 收集
        // （否则本 fn 名下会混入他 fn 的 let 变量，候选列表串流）
        if (target?.tag === "fn") continue;
        if (target?.tag === "let") {
          const name = (target.props?.name as string) ?? "";
          if (name) lets.push({ name });
        }
        queue.push(w.toId);
      }
    }
    out.set(fnEl.id, lets);
  }
  return out;
}

/** 从 elId 沿 flow-in 反向 BFS 反推所属 fn 元素 id；fn 自身返回自身；无 fn 可达返回 null。 */
export function findOwnerFnId(elId: string, elements: CodeblockElementData[], wires: WireData[]): string | null {
  const incoming = new Map<string, WireData[]>();
  for (const w of wires) {
    const list = incoming.get(w.toId) ?? [];
    list.push(w);
    incoming.set(w.toId, list);
  }
  // id → element 索引（同 collectLetVarsPerFn：BFS 内查找 O(1)）
  const elemById = new Map(elements.map((e) => [e.id, e]));
  const visited = new Set<string>([elId]);
  const queue = [elId];
  while (queue.length > 0) {
    const cur = queue.shift()!;
    if (elemById.get(cur)?.tag === "fn") return cur;
    for (const w of incoming.get(cur) ?? []) {
      if (visited.has(w.fromId)) continue;
      visited.add(w.fromId);
      queue.push(w.fromId);
    }
  }
  return null;
}
