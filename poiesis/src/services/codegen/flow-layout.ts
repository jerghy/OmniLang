// poiesis/src/services/codegen/flow-layout.ts
// 自动整理布局：自上而下分层（主干垂直、分支右展、同层对齐、子图堆叠）。
// 纯函数——画布按钮触发 computeLayeredLayout → setElementsAndWires 批量写入（带 undo）。
// 隐式回边设计（loop body 无回边 wire）保证控制流图为 DAG，无需环处理。
import type { CodeblockElementData } from "../../core/types/element";
import type { WireData } from "../../core/types/wire";

export const LAYOUT_CONSTS = {
  /** 层内垂直间距 */
  V_GAP: 80,
  /** 列间水平间距 */
  H_GAP: 60,
  /** 子图堆叠间距 */
  SUBGRAPH_GAP: 80,
  /** 同层同列多节点纵向间距 */
  INLINE_GAP: 20,
};

export interface LayoutNodeSize {
  width: number;
  height: number;
}

// 估算尺寸以真实渲染为准（2026-08-05 实测收集：fn 125×28/let 195×79/assign 195×63/
// condition 128×71/while 114×61/for-in 122×74/return 95×23/call 133×75）——
// 估算严重偏大（旧 200 宽）会使「分支源右对齐」失效：右对齐按估算算，估算时分支源
// 恰为列内最宽则无需右移，真实渲染时右缘仍被同列下方更宽节点覆盖（wire 起点被挡）。
// 取值保守略大（列宽/行高偏大安全，偏小才重叠）；measured 就绪时布局优先真实值。
const BASE_SIZES: Record<string, LayoutNodeSize> = {
  fn: { width: 150, height: 44 },
  "code-block": { width: 200, height: 64 },
  condition: { width: 140, height: 80 },
  return: { width: 110, height: 36 },
  let: { width: 200, height: 84 },
  assign: { width: 200, height: 72 },
  "store-call": { width: 200, height: 84 },
  call: { width: 150, height: 84 },
  while: { width: 130, height: 68 },
  "for-in": { width: 140, height: 80 },
};

/** 按 tag/props 估算节点尺寸（布局坐标化的兜底；有 measured 尺寸时优先真实值）。 */
export function estimateNodeSize(tag: string, props?: Record<string, unknown>): LayoutNodeSize {
  const base = BASE_SIZES[tag] ?? { width: 200, height: 64 };
  if (tag === "condition") {
    // 每分支一行（~20px/行）+ 头尾：3 分支 ≈ 71（实测）；公式随分支数线性
    const n = Math.max(Array.isArray(props?.branches) ? (props.branches as unknown[]).length : 1, 1);
    return { width: base.width, height: 34 + n * 14 };
  }
  return base;
}

/** 端口是否分支口（brN / body）：分支子树右展一列 */
function isBranchPort(port: string | undefined): boolean {
  return port !== undefined && port !== "flow-in" && port !== "flow-out";
}

/** 分支口排序键：br0 < br1 < br2（数字后缀序）；无数字后缀的（body 等）排最后。
 *  分支 lane 按此序分配——消除 wire 数组乱序导致的兄弟分支 wire 交叉
 *  （lane 分配若跟 wire 数组序，br1 的 wire 在前时 br1 目标占左列、br0 目标占右列 → 交叉）。 */
function branchPortOrder(port: string): number {
  const m = port.match(/(\d+)$/);
  return m ? Number(m[1]) : Number.MAX_SAFE_INTEGER;
}

/**
 * 计算画布元素的新位置表。 * 算法：wires 无向连通分量 → 每子图取根（fn 优先）→ 最长路径分层（前驱在上）
 * → 分支列（flow-out 续接随主干，分支口进入 +1 列）→ y=depth×行高、x=列累计宽
 * → 子图垂直堆叠 → 孤立节点底部横排。
 */
export function computeLayeredLayout(
  elements: Array<Pick<CodeblockElementData, "id" | "tag" | "props">>,
  wires: WireData[],
  nodeSizes: Map<string, LayoutNodeSize>,
): Record<string, { x: number; y: number }> {
  const sizeOf = (id: string, tag: string, props?: Record<string, unknown>): LayoutNodeSize =>
    nodeSizes.get(id) ?? estimateNodeSize(tag, props);

  // 1. 无向连通分量（并查集）
  const parent = new Map<string, string>();
  const find = (a: string): string => {
    const p = parent.get(a);
    if (p === undefined) { parent.set(a, a); return a; }
    if (p !== a) { parent.set(a, find(p)); }
    return parent.get(a)!;
  };
  const union = (a: string, b: string) => {
    const ra = find(a), rb = find(b);
    if (ra !== rb) parent.set(ra, rb);
  };
  elements.forEach((e) => find(e.id));
  wires.forEach((w) => union(w.fromId, w.toId));

  const comps = new Map<string, string[]>();
  for (const e of elements) {
    const r = find(e.id);
    const list = comps.get(r) ?? [];
    list.push(e.id);
    comps.set(r, list);
  }

  // 2. 邻接：前驱（in-edges）与后继（out-edges），按端口分类
  const inEdges = new Map<string, Array<{ from: string; port: string }>>();
  const outEdges = new Map<string, Array<{ to: string; port: string }>>();
  wires.forEach((w) => {
    if (!inEdges.has(w.toId)) inEdges.set(w.toId, []);
    inEdges.get(w.toId)!.push({ from: w.fromId, port: w.fromPort });
    if (!outEdges.has(w.fromId)) outEdges.set(w.fromId, []);
    outEdges.get(w.fromId)!.push({ to: w.toId, port: w.fromPort });
  });

  const elemById = new Map(elements.map((e) => [e.id, e]));
  const pos: Record<string, { x: number; y: number }> = {};

  // 3. 每分量：分层 + 分列 + 坐标化
  let stackedY = 0;
  for (const ids of comps.values()) {
    const isConnected = ids.length > 1;
    if (!isConnected) continue; // 孤立集最后统一排

    // 根：fn 优先（无入边），否则任意无入边节点
    const noIn = ids.filter((id) => !(inEdges.get(id)?.length));
    const rootId = noIn.find((id) => elemById.get(id)?.tag === "fn") ?? noIn[0] ?? ids[0];

    // 最长路径分层：depth(node) = max(depth(preds)) + 1；根 = 0
    const depth = new Map<string, number>();
    const visited = new Set<string>();
    const dfs = (id: string): number => {
      if (visited.has(id)) return depth.get(id) ?? 0;
      visited.add(id);
      const d = (inEdges.get(id) ?? []).reduce((m, p) => Math.max(m, dfs(p.from) + 1), 0);
      depth.set(id, d);
      return d;
    };
    ids.forEach((id) => dfs(id));
    // 无入边但非根的节点（如孤立 fn）从 0 起，无碍

    // 分支列：flow-out/flow-in 随主干；分支口（br*/body）进入右展——
    // 同一节点的各分支边依次占用递增列（分支车道），同层分支首句各自独列（同层同 y 且不重叠）。
    // 分支边按端口 id 排序分配 lane（br0→最左）：wire 数组乱序时兄弟分支 wire 仍不交叉
    // （lane 分配跟 wire 数组序 → br1 的 wire 在前时 br1 目标占左列、br0 目标占右列 → 交叉）。
    const column = new Map<string, number>();
    const colVisited = new Set<string>();
    const walk = (id: string, col: number) => {
      if (colVisited.has(id)) return;
      colVisited.add(id);
      column.set(id, col);
      const out = outEdges.get(id) ?? [];
      // 非分支边（flow-out 续接）随主干列
      for (const o of out) {
        if (!isBranchPort(o.port)) walk(o.to, col);
      }
      // 分支边按端口 id 排序序逐一分配递增 lane（br0→最左）
      const branches = out.filter((o) => isBranchPort(o.port)).sort((a, b) => branchPortOrder(a.port) - branchPortOrder(b.port));
      branches.forEach((o, lane) => walk(o.to, col + 1 + lane));
    };
    walk(rootId, 0);
    ids.forEach((id) => { if (!column.has(id)) column.set(id, 0); });

    // 桶统计：同 (depth, column) 为一桶（inline 均布单元）
    const bucketKey = (id: string) => `${depth.get(id)}:${column.get(id)}`;
    const inlineIdx = new Map<string, number>();
    const inlineCount = new Map<string, number>();
    for (const id of ids) {
      const k = bucketKey(id);
      inlineCount.set(k, (inlineCount.get(k) ?? 0) + 1);
    }
    // 每层最满桶的节点数：行高按桶容量预留，防同桶 idx 偏移穿入下一行（2×2 网格合并形态）
    const maxBucketPerDepth = new Map<number, number>();
    for (const id of ids) {
      const d = depth.get(id) ?? 0;
      const n = inlineCount.get(bucketKey(id)) ?? 1;
      maxBucketPerDepth.set(d, Math.max(maxBucketPerDepth.get(d) ?? 1, n));
    }

    // 行高/列宽：按层/列取最大估算尺寸
    const maxDepth = Math.max(...ids.map((id) => depth.get(id) ?? 0));
    const rowHeight = new Map<number, number>();
    for (let d = 0; d <= maxDepth; d++) {
      const maxH = Math.max(...ids.filter((id) => depth.get(id) === d).map((id) => sizeOf(id, elemById.get(id)!.tag, elemById.get(id)!.props).height));
      const n = maxBucketPerDepth.get(d) ?? 1;
      // 单节点桶：maxH + V_GAP；多节点桶：为最满桶的 inline 均布预留整行（idx 最大 = n-1，且 maxH ≥ 任意节点高）
      rowHeight.set(d, maxH + (n - 1) * (maxH + LAYOUT_CONSTS.INLINE_GAP) + LAYOUT_CONSTS.V_GAP);
    }
    const colWidth = new Map<number, number>();
    const maxCol = Math.max(...ids.map((id) => column.get(id) ?? 0));
    for (let c = 0; c <= maxCol; c++) {
      colWidth.set(c, Math.max(...ids.filter((id) => column.get(id) === c).map((id) => sizeOf(id, elemById.get(id)!.tag, elemById.get(id)!.props).width)) + LAYOUT_CONSTS.H_GAP);
    }
    // 列内对齐：无分支口的节点左缘对齐（x = 列左缘）——主干/线性链左缘成一条垂直脊柱，
    // 代码缩进感；有分支口的节点（condition/loop，br0/br1/body 出口）右缘对齐到列右缘
    // （x = 列左缘 + 列内最大宽 − 自身宽）——分支 wire 起点（节点右侧分支 handle）落在
    // 列右缘，其 smoothstep 垂直段在列间空隙（列右缘 + offset），不被同列下方更宽节点
    // 覆盖（ADR-063 实测 while→body wire 起点被 for-in 遮挡）。
    // 历史：列内统一右缘对齐——主干节点左缘参差、无脊柱感（用户实测"整理后更乱"）；
    // 主干 wire 垂直改由 flow-in/out handle 钉节点左缘内侧保证（node-base.css），
    // 与节点宽度无关，无需右缘对齐。
    const hasBranchOut = (id: string) => (outEdges.get(id) ?? []).some((o) => isBranchPort(o.port));
    for (const id of ids) {
      const k = `${depth.get(id)}:${column.get(id)}`;
      const idx = inlineIdx.get(k) ?? 0;
      inlineIdx.set(k, idx + 1);
      const d = depth.get(id) ?? 0;
      const yBase = d === 0 ? 0 : Array.from({ length: d }, (_, i) => rowHeight.get(i) ?? 0).reduce((a, b) => a + b, 0);
      const xBase = (column.get(id) ?? 0) === 0 ? 0 : Array.from({ length: column.get(id) ?? 0 }, (_, i) => colWidth.get(i) ?? 0).reduce((a, b) => a + b, 0);
      const col = column.get(id) ?? 0;
      const w = sizeOf(id, elemById.get(id)!.tag, elemById.get(id)!.props).width;
      const x = hasBranchOut(id) ? xBase + (colWidth.get(col) ?? 0) - LAYOUT_CONSTS.H_GAP - w : xBase;
      pos[id] = {
        x,
        y: stackedY + yBase + idx * (sizeOf(id, elemById.get(id)!.tag, elemById.get(id)!.props).height + LAYOUT_CONSTS.INLINE_GAP),
      };
    }

    // 子图高度：中间行累加 + 最深行预留行高（−V_GAP 为行尾冗余间隙）。
    // 最深行必须含桶内 inline 预留（rowHeight[maxDepth] 已含 (n-1)×(maxH+INLINE_GAP)），否则 idx≥1 节点底穿入下一子图
    const compHeight = (maxDepth === 0 ? 0 : Array.from({ length: maxDepth }, (_, i) => rowHeight.get(i) ?? 0).reduce((a, b) => a + b, 0)) +
      (rowHeight.get(maxDepth) ?? 0) - LAYOUT_CONSTS.V_GAP;
    stackedY += compHeight + LAYOUT_CONSTS.SUBGRAPH_GAP;
  }

  // 4. 孤立节点：所有子图之后横排
  const lonely = elements.filter((e) => {
    const r = find(e.id);
    return (comps.get(r)?.length ?? 0) <= 1;
  });
  let lonelyX = 0;
  for (const e of lonely) {
    const s = sizeOf(e.id, e.tag, e.props);
    pos[e.id] = { x: lonelyX, y: stackedY };
    lonelyX += s.width + LAYOUT_CONSTS.H_GAP;
  }

  return pos;
}
