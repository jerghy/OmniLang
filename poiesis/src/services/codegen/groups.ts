// poiesis/src/services/codegen/groups.ts
// 分组纯函数：成员校验 / fn 归属 / 包围盒。DOM 测量与 store 写入在组件层完成。
import type { CodeblockElementData } from "../../core/types/element";
import type { WireData } from "../../core/types/wire";
import type { CodeFlowGroup } from "../../core/types/project-json";
import { findOwnerFnId } from "./fn-topology";

/** 分组框 8 色调色板（与 --ol-* 设计 token 协调的靛蓝/青/琥珀/玫红系） */
export const GROUP_COLORS = [
  "#6366f1", "#06b6d4", "#f59e0b", "#ef4444",
  "#10b981", "#8b5cf6", "#ec4899", "#64748b",
] as const;

/** 分组框内边距（px，画布单位） */
export const GROUP_PADDING = 24;

const CONTAINER_TAGS = new Set(["fn", "action", "lifecycle"]);

export interface GroupValidation {
  valid: string[];
  ownerFnId: string | null;
  error?: string;
}

/** 校验成员：非空、不含容器节点（fn/action/lifecycle）、全部同一 fn 连通域。
 *  ownerFnId = 首个有效成员的 fn 元素 id（供调用方查 fn 名）。 */
export function validateGroupMembers(
  memberIds: string[],
  elements: CodeblockElementData[],
  wires: WireData[],
): GroupValidation {
  if (memberIds.length < 2) return { valid: [], ownerFnId: null, error: "分组至少需要 2 个节点" };
  const elemById = new Map(elements.map((e) => [e.id, e]));
  const owner = findOwnerFnId(memberIds[0], elements, wires);
  if (!owner) return { valid: [], ownerFnId: null, error: "所选节点不属于任何函数" };
  for (const mid of memberIds) {
    const e = elemById.get(mid);
    if (!e) return { valid: [], ownerFnId: null, error: "所选节点不存在" };
    if (CONTAINER_TAGS.has(e.tag)) return { valid: [], ownerFnId: null, error: "函数节点不能加入分组" };
    if (findOwnerFnId(mid, elements, wires) !== owner) {
      return { valid: [], ownerFnId: null, error: "分组成员必须属于同一个函数" };
    }
  }
  return { valid: [...memberIds], ownerFnId: owner };
}

/** fn 元素名（codeFlowGroups 键），缺失回退 "unnamed"。 */
export function fnNameOf(fnElId: string, elements: CodeblockElementData[]): string {
  const fnEl = elements.find((e) => e.id === fnElId);
  return (fnEl?.props?.name as string) || "unnamed";
}

export interface GroupBBox { x: number; y: number; width: number; height: number; }

/** 成员 position + size（含 padding）求包围盒。无成员或任一成员缺尺寸 → 该组跳过（画布无法可靠摆放）。 */
export function computeGroupBoxes(
  groups: CodeFlowGroup[],
  elements: CodeblockElementData[],
  sizes: Map<string, { width: number; height: number }>,
): Map<string, GroupBBox> {
  const elemById = new Map(elements.map((e) => [e.id, e]));
  const out = new Map<string, GroupBBox>();
  for (const g of groups) {
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    let ok = true;
    for (const mid of g.memberIds) {
      const e = elemById.get(mid);
      const s = sizes.get(mid);
      if (!e || !e.position || !s) { ok = false; break; }
      minX = Math.min(minX, e.position.x);
      minY = Math.min(minY, e.position.y);
      maxX = Math.max(maxX, e.position.x + s.width);
      maxY = Math.max(maxY, e.position.y + s.height);
    }
    if (!ok || !Number.isFinite(minX)) continue;
    out.set(g.id, { x: minX - GROUP_PADDING, y: minY - GROUP_PADDING, width: maxX - minX + GROUP_PADDING * 2, height: maxY - minY + GROUP_PADDING * 2 });
  }
  return out;
}
