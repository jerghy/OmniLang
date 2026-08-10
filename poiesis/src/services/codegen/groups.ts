// poiesis/src/services/codegen/groups.ts
// 分组纯函数：成员校验 / fn 归属 / 包围盒 / 折叠态连线重路由。DOM 测量与 store 写入在组件层完成。
import type { Edge } from "@xyflow/react";
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

/** 所有折叠组的成员 id 集合（渲染层过滤用）。 */
export function collectCollapsedMemberIds(groups: CodeFlowGroup[]): Set<string> {
  const out = new Set<string>();
  for (const g of groups) {
    if (g.collapsed) for (const mid of g.memberIds) out.add(mid);
  }
  return out;
}

/** 折叠组连线重路由：进线接到组节点左侧（flow-in）、出线发自组节点右侧（flow-out）、
 *  组内连线隐藏。保留 wire id（防 key 冲突）；展开后从 wires 重推自动还原。 */
export function buildFlowEdges(wires: WireData[], groups: CodeFlowGroup[]): Edge[] {
  const collapsed = collectCollapsedMemberIds(groups);
  if (collapsed.size === 0) {
    return wires.map((w) => ({ id: w.id, type: "default", source: w.fromId, target: w.toId, sourceHandle: w.fromPort, targetHandle: w.toPort }));
  }
  const memberToGroup = new Map<string, string>();
  for (const g of groups) {
    if (!g.collapsed) continue;
    for (const mid of g.memberIds) memberToGroup.set(mid, g.id);
  }
  const out: Edge[] = [];
  for (const w of wires) {
    const fromGroup = memberToGroup.get(w.fromId);
    const toGroup = memberToGroup.get(w.toId);
    if (fromGroup && toGroup && fromGroup === toGroup) continue; // 组内连线隐藏
    out.push({
      id: w.id,
      type: "default",
      source: fromGroup ?? w.fromId,
      target: toGroup ?? w.toId,
      sourceHandle: fromGroup ? "flow-out" : w.fromPort,
      targetHandle: toGroup ? "flow-in" : w.toPort,
    });
  }
  return out;
}

/** 全键扫描定位组：返回所在 fn 键名与组对象；找不到返回 null。
 *  FlowInspector 分组面板与 FlowCanvas 全键操作共用（成员断链后 fn 键不可信，
 *  定位一律以组 id 为准）。 */
export function findGroupById(
  groups: Record<string, CodeFlowGroup[]>,
  groupId: string,
): { fnName: string; group: CodeFlowGroup } | null {
  for (const fnName of Object.keys(groups)) {
    const g = groups[fnName].find((x) => x.id === groupId);
    if (g) return { fnName, group: g };
  }
  return null;
}

/** 不可变 patch：按组 id 全键扫描替换，返回新 groups 对象（整体替换某 fn 键的
 *  setCodeFlowGroups 消费；未命中时返回等价新对象，不修改输入）。 */
export function patchGroupById(
  groups: Record<string, CodeFlowGroup[]>,
  groupId: string,
  patch: Partial<CodeFlowGroup>,
): Record<string, CodeFlowGroup[]> {
  const out: Record<string, CodeFlowGroup[]> = {};
  for (const fnName of Object.keys(groups)) {
    out[fnName] = groups[fnName].map((x) => (x.id === groupId ? { ...x, ...patch } : x));
  }
  return out;
}

/** 不可变 remove：按组 id 全键扫描删除，空数组键剔除（toProjectJson 保存侧同款清理语义）。 */
export function removeGroupById(
  groups: Record<string, CodeFlowGroup[]>,
  groupId: string,
): Record<string, CodeFlowGroup[]> {
  const out: Record<string, CodeFlowGroup[]> = {};
  for (const fnName of Object.keys(groups)) {
    const filtered = groups[fnName].filter((x) => x.id !== groupId);
    if (filtered.length > 0) out[fnName] = filtered;
  }
  return out;
}
