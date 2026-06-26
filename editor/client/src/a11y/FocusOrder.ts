/**
 * Tab 键焦点顺序可视化叠加层。
 *
 * 在画布上以数字标签显示每个可聚焦元素的 Tab 顺序，
 * 帮助开发者检查键盘导航是否符合预期。
 */

import type { ElementData } from "../core/types";

// ── 可聚焦元素识别 ────────────────────────────────

const FOCUSABLE_TAGS = [
  "Input",
  "TextArea",
  "Button",
  "Select",
  "Checkbox",
  "Radio",
  "Link",
  "Slider",
];

function getFocusableElements(elements: ElementData[]): ElementData[] {
  return elements.filter((e) => FOCUSABLE_TAGS.includes(e.tag));
}

// ── Tab 顺序计算 ─────────────────────────────────

/**
 * 按深度优先遍历元素树计算 Tab 顺序。
 * 如果元素有 `tabIndex` prop，按值排序（负值跳过）。
 */
function computeTabOrder(
  elements: ElementData[],
  rootIds: string[]
): ElementData[] {
  const ordered: ElementData[] = [];

  function walk(id: string) {
    const el = elements.find((e) => e.id === id);
    if (!el) return;
    // 跳过 tabIndex < 0 的元素（不可通过 Tab 聚焦）
    const tabIdx = el.props["tabIndex"] as number | undefined;
    if (tabIdx !== undefined && tabIdx < 0) return;
    if (FOCUSABLE_TAGS.includes(el.tag)) {
      ordered.push(el);
    }
    // 按 tabIndex 排序子元素
    const children = el.children
      .map((cid) => elements.find((e) => e.id === cid))
      .filter((c): c is ElementData => !!c);
    children.sort((a, b) => {
      const aIdx = typeof a.props["tabIndex"] === "number" ? a.props["tabIndex"] as number : 0;
      const bIdx = typeof b.props["tabIndex"] === "number" ? b.props["tabIndex"] as number : 0;
      return aIdx - bIdx;
    });
    for (const child of children) {
      walk(child.id);
    }
  }

  for (const rootId of rootIds) {
    walk(rootId);
  }

  return ordered;
}

// ── 位置收集 — 在组件中调用 ──────────────────────

export interface FocusOrderEntry {
  id: string;
  tag: string;
  index: number;
  rect: DOMRect;
}

/**
 * 从 DOM 收集当前可聚焦元素的位置信息。
 * 在请求动画帧或 overlay 渲染时调用。
 */
export function collectFocusOrder(
  elements: ElementData[]
): FocusOrderEntry[] {
  const rootIds = elements
    .filter((e) => !e.parentId)
    .map((e) => e.id);
  const ordered = computeTabOrder(elements, rootIds);
  const entries: FocusOrderEntry[] = [];

  for (let i = 0; i < ordered.length; i++) {
    const el = ordered[i];
    const dom = document.querySelector(`[data-omni-id="${el.id}"]`);
    if (!dom) continue;
    const rect = dom.getBoundingClientRect();
    entries.push({ id: el.id, tag: el.tag, index: i + 1, rect });
  }

  return entries;
}

// ── React render helper — 返回样式片段 ───────────

export interface FocusOverlayStyle {
  container: React.CSSProperties;
  box: React.CSSProperties;
  label: React.CSSProperties;
}

export function focusOverlayStyles(
  entry: FocusOrderEntry,
  containerRect?: DOMRect
): FocusOverlayStyle {
  const offsetLeft = containerRect?.left ?? 0;
  const offsetTop = containerRect?.top ?? 0;

  return {
    container: {
      position: "absolute",
      left: entry.rect.left - offsetLeft - 4,
      top: entry.rect.top - offsetTop - 4,
      width: entry.rect.width + 8,
      height: entry.rect.height + 8,
      border: "1px dashed #4F46E5",
      borderRadius: 4,
      pointerEvents: "none",
      animation: "omnilang-focus-fadein 300ms ease-out",
    },
    box: {
      position: "absolute",
      top: -20,
      left: 0,
      fontSize: 10,
      fontWeight: 700,
      background: "#4F46E5",
      color: "#FFF",
      padding: "1px 5px",
      borderRadius: 3,
      whiteSpace: "nowrap",
    },
    label: {
      fontSize: 10,
      fontWeight: 700,
      background: "#4F46E5",
      color: "#FFF",
      padding: "1px 5px",
      borderRadius: 3,
      whiteSpace: "nowrap",
    },
  };
}
