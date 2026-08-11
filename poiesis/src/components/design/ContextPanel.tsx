// poiesis/src/components/design/ContextPanel.tsx
// 语境面板（§15.5）：选中元素 → 浮动在主元素旁（跟随/换侧/可拖走）；空选收成小浮标。
// 主面板 = [行为][内容][结构]；副面板 = 仅 [内容]；最多 2 个面板。
// [内容] 标签 = 意图驱动醒（自动醒，store 内实现）。
import { useEffect, useMemo, useRef, useState } from "react";
import { usePoiesisStore } from "../../store";
import { computePanelPlacement, type PanelPlacement } from "./panel-position";
import { StructureTab } from "./StructureTab";
import { ContentTab } from "./ContentTab";
import { BehaviorTab } from "./BehaviorTab";
import styles from "./ContextPanel.module.css";

const RENDERER_SELECTOR = "[data-renderer='omniview']";
const PANEL_WIDTH = 280;

interface DraggedPanel { dx: number; dy: number; }

export function ContextPanel() {
  const selectedIds = usePoiesisStore((s) => s.selectedElementIds);
  const mainPanelTab = usePoiesisStore((s) => s.designPage.mainPanelTab);
  const setMainPanelTab = usePoiesisStore((s) => s.setMainPanelTab);
  const [stubOpen, setStubOpen] = useState(false);
  const [placement, setPlacement] = useState<PanelPlacement | null>(null);
  const [secondaryPlacement, setSecondaryPlacement] = useState<PanelPlacement | null>(null);
  const [drag, setDrag] = useState<DraggedPanel | null>(null);
  const panelRef = useRef<HTMLDivElement>(null);

  // 面板集合：主 = 选中[0]，副 = 选中[1]（≤2，按 id 去重）
  const panelIds = useMemo(() => {
    const seen = new Set<string>();
    const out: string[] = [];
    for (const id of selectedIds) {
      if (seen.has(id)) continue;
      seen.add(id);
      out.push(id);
      if (out.length === 2) break;
    }
    return out;
  }, [selectedIds]);

  // 落位跟随：rAF 循环读元素 DOM 矩形（跟随元素移动/布局变化）；拖拽时叠加偏移
  // 审查修正：主面板锚定选中[0]元素、副面板锚定选中[1]元素——各自浮在自己元素旁边（§15.5）
  useEffect(() => {
    if (panelIds.length === 0) { setPlacement(null); setSecondaryPlacement(null); return; }
    let raf = 0;
    const tick = () => {
      const container = document.querySelector<HTMLElement>(RENDERER_SELECTOR);
      // 元素 id 为 uid 生成（字母数字），可安全用于属性选择器
      const measureFor = (elId: string): PanelPlacement | null => {
        const target = container?.querySelector<HTMLElement>(`[data-omni-el-id="${elId}"]`);
        if (!target) return null;
        const rect = target.getBoundingClientRect();
        return computePanelPlacement(
          { x: rect.left, y: rect.top, width: rect.width, height: rect.height },
          { width: PANEL_WIDTH, height: Math.min(rect.bottom - rect.top + 120, 420) },
          { width: window.innerWidth, height: window.innerHeight },
        );
      };
      setPlacement((prev) => measureFor(panelIds[0]) ?? prev);
      if (panelIds[1]) setSecondaryPlacement((prev) => measureFor(panelIds[1]) ?? prev);
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, [panelIds, selectedIds]);

  // 元素不可见（未渲染/在折叠区）时的兜底：视口右上
  const resolved: PanelPlacement | null = placement ?? (panelIds.length > 0
    ? { x: window.innerWidth - PANEL_WIDTH - 16, y: 64, side: "right" }
    : null);

  // 副面板同款兜底（jsdom/未渲染副元素时 placement 恒 null——渲染门禁须用兜底值，
  // 否则副面板永不显示，测试 4 必红）
  const secondaryResolved: PanelPlacement | null = secondaryPlacement ?? (panelIds.length > 1
    ? { x: window.innerWidth - PANEL_WIDTH - 16, y: 64 + PANEL_WIDTH, side: "right" }
    : null);

  // ── 拖拽（头部按住拖动）──
  const onHeaderPointerDown = (e: React.PointerEvent) => {
    if (e.button !== 0) return;
    const startX = e.clientX;
    const startY = e.clientY;
    const onMove = (ev: PointerEvent) => {
      setDrag({ dx: ev.clientX - startX, dy: ev.clientY - startY });
    };
    const onUp = () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
  };

  if (panelIds.length === 0) {
    // 空选 → 小浮标（应用的面板入口；本阶段点击展开空态面板）
    return (
      <button
        type="button"
        data-testid="context-panel-stub"
        className={styles.stub}
        onClick={() => setStubOpen((v) => !v)}
        title="应用面板"
      >
        面板
        {stubOpen && (
          <span className={styles.stubEmpty} onClick={(e) => e.stopPropagation()}>
            选择元素以编辑属性
          </span>
        )}
      </button>
    );
  }

  const mainId = panelIds[0];
  const secondaryId = panelIds[1];

  // TS 窄化：空选已早退（stub），此处 panelIds 非空 → resolved 恒为兜底值，非 null
  if (!resolved) return null;

  return (
    <>
      <div
        ref={panelRef}
        data-testid="context-panel"
        data-panel-for={mainId}
        className={styles.panel}
        style={{
          left: resolved.x + (drag?.dx ?? 0),
          top: resolved.y + (drag?.dy ?? 0),
          zIndex: 30,
        }}
      >
        <div className={styles.header} onPointerDown={onHeaderPointerDown}>
          <span className={styles.title}>元素</span>
          <div className={styles.tabs} role="tablist">
            <button type="button" role="tab" aria-selected={mainPanelTab === "structure"}
              className={mainPanelTab === "structure" ? styles.tabActive : styles.tab}
              onClick={() => setMainPanelTab("structure")}>结构</button>
            <button type="button" role="tab" aria-selected={mainPanelTab === "content"}
              className={mainPanelTab === "content" ? styles.tabActive : styles.tab}
              onClick={() => setMainPanelTab("content")}>内容</button>
            <button type="button" role="tab" aria-selected={mainPanelTab === "behavior"}
              className={mainPanelTab === "behavior" ? styles.tabActive : styles.tab}
              onClick={() => setMainPanelTab("behavior")}>行为</button>
          </div>
        </div>
        <div className={styles.body}>
          {mainPanelTab === "structure" && <StructureTab elementId={mainId} />}
          {mainPanelTab === "content" && <ContentTab elementId={mainId} />}
          {mainPanelTab === "behavior" && <BehaviorTab elementId={mainId} />}
        </div>
      </div>

      {secondaryId && secondaryResolved && (
        <div
          data-testid="context-panel"
          data-panel-for={secondaryId}
          className={styles.panel}
          style={{
            left: secondaryResolved.x + (drag?.dx ?? 0),
            top: secondaryResolved.y + (drag?.dy ?? 0),
            zIndex: 29,
          }}
        >
          <div className={styles.header}>
            <span className={styles.title}>元素（内容）</span>
            <span className={styles.subNote}>仅 [内容]</span>
          </div>
          <div className={styles.body}>
            <ContentTab elementId={secondaryId} />
          </div>
        </div>
      )}
    </>
  );
}
