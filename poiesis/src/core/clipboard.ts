/** 剪贴板操作：复制、粘贴、剪切、复制。
 *
 *  使用模块级单例（非 DOM Clipboard API），因为：
 *  - ElementData 是复杂对象，navigator.clipboard 仅支持 text/MIME
 *  - file:// 协议下 DOM clipboard API 不可用
 *  - 同步操作，无需 async 权限请求 */

import type { ElementData, WireData } from "./types";
import { usePoiesisStore } from "../store";
import { uid } from "../lib/utils/id";

// ── Clipboard data types ────────────────────────────────────────────

export interface ClipboardData {
  elements: ElementData[];
  wires: WireData[];
  sourceMode: "design" | "code";
}

// ── Module-level clipboard singleton ────────────────────────────────

let clipboardBuffer: ClipboardData | null = null;

export function getClipboard(): ClipboardData | null {
  return clipboardBuffer;
}

export function hasClipboard(): boolean {
  return clipboardBuffer !== null && clipboardBuffer.elements.length > 0;
}

// ── Copy ────────────────────────────────────────────────────────────

/**
 * 将选中元素（以及它们之间的连线）复制到模块级剪贴板。
 * 对于 view 元素会递归收集子元素。
 */
export function copySelection(
  selectedIds: string[],
): ClipboardData | null {
  if (selectedIds.length === 0) return null;

  const idSet = new Set(selectedIds);
  const { elements, wires } = usePoiesisStore.getState();

  // 递归收集子元素（仅 view 元素有 children 层级）
  const elMap = new Map(elements.map(e => [e.id, e]));
  const stack = [...selectedIds];
  while (stack.length > 0) {
    const id = stack.pop()!;
    const el = elMap.get(id);
    if (el && "children" in el && el.children.length > 0) {
      for (const childId of el.children) {
        if (!idSet.has(childId)) {
          idSet.add(childId);
          stack.push(childId);
        }
      }
    }
  }

  // 收集元素
  const clipboardElements = elements.filter((e) => idSet.has(e.id));

  // 收集连线：仅包含两端都在选中集合内的连线
  const clipboardWires = wires.filter(
    (w) => idSet.has(w.fromId) && idSet.has(w.toId),
  );

  clipboardBuffer = { elements: clipboardElements, wires: clipboardWires, sourceMode: "code" };

  return clipboardBuffer;
}

// ── Paste ───────────────────────────────────────────────────────────

/**
 * 从模块级剪贴板粘贴元素。
 * 为每个元素生成新 ID，重新映射 parentId/children 引用，偏移位置。
 * 返回粘贴元素的 ID 列表。
 */
export function pasteFromClipboard(
  offsetX = 40,
  offsetY = 40,
): string[] | null {
  if (!clipboardBuffer || clipboardBuffer.elements.length === 0) return null;

  const { elements: srcElements, wires: srcWires } = clipboardBuffer;

  // 1. 生成新 ID 映射
  const idMap = new Map<string, string>();
  for (const el of srcElements) {
    idMap.set(el.id, uid("tag" in el ? el.tag : "node"));
  }

  // 2. 映射元素
  const newElements: ElementData[] = srcElements.map((el) => {
    const newId = idMap.get(el.id)!;
    const newParentId = el.parentId ? (idMap.get(el.parentId) ?? null) : null;
    const newChildren = "children" in el
      ? el.children.map((c) => idMap.get(c)).filter((c): c is string => c !== undefined)
      : [];
    const newPosition = "position" in el && el.position
      ? { x: el.position.x + offsetX, y: el.position.y + offsetY }
      : undefined;

    return {
      ...el,
      id: newId,
      parentId: newParentId,
      children: newChildren,
      position: newPosition,
    };
  });

  // 3. 映射连线
  const newWires: WireData[] = srcWires
    .map((w) => {
      const newFromId = idMap.get(w.fromId);
      const newToId = idMap.get(w.toId);
      if (!newFromId || !newToId) return null;
      return {
        ...w,
        id: uid("wire"),
        fromId: newFromId,
        toId: newToId,
      };
    })
    .filter((w): w is WireData => w !== null);

  // 4. 直接通过 store 添加元素和连线
  const store = usePoiesisStore.getState();
  for (const el of newElements) {
    store.addElement(el);
  }
  for (const w of newWires) {
    store.addWire(w);
  }

  return newElements.map((el) => el.id);
}

// ── Cut ─────────────────────────────────────────────────────────────

/**
 * 剪切：复制选中元素到剪贴板，然后删除它们。
 */
export function cutSelection(
  selectedIds: string[],
): string[] | null {
  const data = copySelection(selectedIds);
  if (!data) return null;

  // 收集所有要删除的 ID（含子元素）
  const idSet = new Set(data.elements.map((e) => e.id));
  const store = usePoiesisStore.getState();

  // 删除与这些元素相关的所有连线
  const affectedWires = store.wires.filter(
    (w) => idSet.has(w.fromId) || idSet.has(w.toId),
  );
  for (const w of affectedWires) {
    store.removeWire(w.id);
  }

  // 删除元素
  for (const id of selectedIds) {
    store.removeElement(id);
  }

  return data.elements.map((e) => e.id);
}

// ── Duplicate ───────────────────────────────────────────────────────

/**
 * 复制：原地复制选中元素（偏移 45px），不走剪贴板。
 * 等价于 copy + paste，但不修改剪贴板内容。
 */
export function duplicateSelection(
  selectedIds: string[],
): string[] | null {
  if (selectedIds.length === 0) return null;

  // 临时保存剪贴板
  const saved = clipboardBuffer;
  const data = copySelection(selectedIds);
  if (!data) {
    clipboardBuffer = saved;
    return null;
  }

  const pastedIds = pasteFromClipboard(45, 45);

  // 恢复剪贴板（复制不应该影响剪贴板）
  clipboardBuffer = saved;

  return pastedIds;
}
