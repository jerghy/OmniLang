// 元素端口层（视觉）：醒态在元素右缘长出接口圆点（出口位）。
// 本阶段纯视觉、不可交互——接线交互（拖线/点名）M3 启用，形状（圆点/双竖线/菱形）后置。
// 端口随画布变换（本层渲染在设计层变换上下文内）；点位 = 容器相对坐标 ÷zoom + 滚动补偿。
import { useLayoutEffect, useState } from "react";
import { useCanvasStore } from "./canvas-store";
import { usePoiesisStore } from "../../store";
import styles from "./ElementPortLayer.module.css";

export interface PortDot {
  elId: string;
  x: number;
  y: number;
}

const RENDERER_SELECTOR = "[data-renderer='omniview']";

/** 在容器内查找带 data-omni-el-id 的元素，输出右缘中点端口位（padding-box 相对 + 滚动补偿）。
 *  挂载契约（钉死，审查修正 + 实机验证修正 + zoom 补偿修正）：ElementPortLayer 必须是
 *  OmniViewRenderer 外层滚动容器（overflow:auto + padding:8 + position:relative）的 absolute
 *  子节点；该容器整体位于 InfiniteCanvas 的 `scale(zoom) translate(panX/zoom, panY/zoom)`
 *  变换包装内（InfiniteCanvas.tsx:153，transformOrigin 0 0）——因此 getBoundingClientRect
 *  返回的是**变换后**的视口坐标，而本层 CSS 布局空间是**变换前**的内容坐标：
 *  视口 delta = 内容 delta × zoom，必须 ÷zoom 换回层本地空间（panX/panY 在差值中抵消，无需处理）；
 *  scrollLeft/Top 是容器自身（变换前）内容坐标，原样加回。坐标相对 padding-box 原点
 *  （absolute inset:0 层的原点即 padding 边缘），**不可再减 paddingLeft/Top**：padding 只移动
 *  内容流起点，层原点已包含 padding 区域，再减即双重补偿（zoom=1 实机验证 delta=0）。
 *  三项缺一即错位：absolute 子节点 / ÷zoom / 加 scroll。 */
export function collectPortPositions(container: HTMLElement, zoom = 1): PortDot[] {
  const dots: PortDot[] = [];
  const cRect = container.getBoundingClientRect();
  for (const el of container.querySelectorAll<HTMLElement>("[data-omni-el-id]")) {
    const elId = el.dataset.omniElId;
    if (!elId) continue;
    const rect = el.getBoundingClientRect();
    dots.push({
      elId,
      x: (rect.right - cRect.left) / zoom + container.scrollLeft,
      y: (rect.top + rect.height / 2 - cRect.top) / zoom + container.scrollTop,
    });
  }
  return dots;
}

export function ElementPortLayer({ visible }: { visible: boolean }) {
  const awake = usePoiesisStore((s) => s.designPage.awake);
  const selectedIds = usePoiesisStore((s) => s.selectedElementIds);
  const canvasZoom = useCanvasStore((s) => s.zoom);
  const [dots, setDots] = useState<PortDot[]>([]);

  // 醒区成员语义（§15.4"线随醒处"）：整页醒 → 全部元素；多选 → 仅选中元素长端口，未选中的睡态元素不长
  const selectedSet = new Set(selectedIds);
  const shown = visible && (awake || selectedSet.size >= 2);

  useLayoutEffect(() => {
    if (!shown) return;
    // 根不存在（未挂载/被卸载）直接退出——无兜底观察 body，避免死监听
    const root = document.querySelector<HTMLElement>(RENDERER_SELECTOR);
    if (!root) return;
    const measure = () => {
      const next = collectPortPositions(root, canvasZoom);
      // 仅集合变化时更新，避免每帧 setState 重渲染
      setDots((prev) =>
        prev.length === next.length && prev.every((d, i) => d.elId === next[i].elId && d.x === next[i].x && d.y === next[i].y)
          ? prev
          : next,
      );
    };
    measure();
    const raf = requestAnimationFrame(measure); // 等 WASM 渲染布局稳定
    // ResizeObserver 已覆盖窗口缩放引发的容器尺寸变化，无需 window resize 监听
    const observer = new ResizeObserver(measure);
    observer.observe(root);
    return () => {
      cancelAnimationFrame(raf);
      observer.disconnect();
    };
  }, [shown, awake, selectedIds, canvasZoom]);

  if (!shown) return null;

  return (
    <div className={styles.layer} aria-hidden data-testid="element-ports">
      {dots
        .filter((d) => awake || selectedSet.has(d.elId)) // 成员过滤：多选时只有醒区元素长端口
        .map((d) => (
          <span key={d.elId} className={styles.port} style={{ left: d.x, top: d.y }} />
        ))}
    </div>
  );
}
