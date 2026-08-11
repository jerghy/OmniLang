// 元素端口层（视觉）：醒态在元素右缘长出接口圆点（出口位）。
// 本阶段纯视觉、不可交互——接线交互（拖线/点名）M3 启用，形状（圆点/双竖线/菱形）后置。
// 端口随画布变换（本层渲染在设计层变换上下文内）；点位 = 容器相对坐标 + 滚动补偿。
import { useLayoutEffect, useState } from "react";
import { usePoiesisStore } from "../../store";
import styles from "./ElementPortLayer.module.css";

export interface PortDot {
  elId: string;
  x: number;
  y: number;
}

const RENDERER_SELECTOR = "[data-renderer='omniview']";

/** 在容器内查找带 data-omni-el-id 的元素，输出右缘中点端口位（padding-box 相对 + 滚动补偿）。
 *  挂载契约（钉死，审查修正 + 实机验证修正）：ElementPortLayer 必须是 OmniViewRenderer 外层滚动容器
 *  （overflow:auto + padding:8 + position:relative）的 absolute 子节点——与元素同处内容坐标系，
 *  缩放由外层 transform 统一承担（无需 ÷zoom）；层随容器滚动（-scrollLeft/Top），公式加回
 *  scroll 补偿。坐标相对 padding-box 原点（absolute inset:0 层的原点即 padding 边缘），
 *  **不可再减 paddingLeft/Top**：padding 只移动内容流起点，层原点已包含 padding 区域，
 *  再减即双重补偿（浏览器实测 dot 偏移 -8px/-8px，修正后 delta=0）。三项缺一即错位。 */
export function collectPortPositions(container: HTMLElement): PortDot[] {
  const dots: PortDot[] = [];
  const cRect = container.getBoundingClientRect();
  for (const el of container.querySelectorAll<HTMLElement>("[data-omni-el-id]")) {
    const elId = el.dataset.omniElId;
    if (!elId) continue;
    const rect = el.getBoundingClientRect();
    dots.push({
      elId,
      x: rect.right - cRect.left + container.scrollLeft,
      y: rect.top + rect.height / 2 - cRect.top + container.scrollTop,
    });
  }
  return dots;
}

export function ElementPortLayer({ visible }: { visible: boolean }) {
  const awake = usePoiesisStore((s) => s.designPage.awake);
  const selectedIds = usePoiesisStore((s) => s.selectedElementIds);
  const [dots, setDots] = useState<PortDot[]>([]);

  // 醒区成员语义（§15.4"线随醒处"）：整页醒 → 全部元素；多选 → 仅选中元素长端口，未选中的睡态元素不长
  const selectedSet = new Set(selectedIds);
  const shown = visible && (awake || selectedSet.size >= 2);

  useLayoutEffect(() => {
    if (!shown) return;
    const measure = () => {
      const container = document.querySelector<HTMLElement>(RENDERER_SELECTOR);
      if (!container) return;
      const next = collectPortPositions(container);
      // 仅集合变化时更新，避免每帧 setState 重渲染
      setDots((prev) =>
        prev.length === next.length && prev.every((d, i) => d.elId === next[i].elId && d.x === next[i].x && d.y === next[i].y)
          ? prev
          : next,
      );
    };
    measure();
    const raf = requestAnimationFrame(measure); // 等 WASM 渲染布局稳定
    const root = document.querySelector<HTMLElement>(RENDERER_SELECTOR);
    const observer = root ? new ResizeObserver(measure) : null;
    observer?.observe(root ?? document.body);
    window.addEventListener("resize", measure);
    return () => {
      cancelAnimationFrame(raf);
      observer?.disconnect();
      window.removeEventListener("resize", measure);
    };
  }, [shown, awake, selectedIds]);

  if (!shown) return null;

  return (
    <div className={styles.layer} aria-hidden data-testid="element-ports">
      {dots
        .filter((d) => awake || selectedSet.has(d.elId)) // 成员过滤：多选时只有醒区元素长端口
        .map((d) => (
          <span key={d.elId} className={styles.port} style={{ left: d.x, top: d.y }} title={d.elId} />
        ))}
    </div>
  );
}
