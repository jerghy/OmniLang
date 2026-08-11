// poiesis/src/components/canvas/LogicLayer.tsx
// 逻辑层叠加：醒态（或醒区非空）时浮现的 ReactFlow 逻辑层。
// 睡态隐藏且不拦截事件——睡 = 干净画布；醒着也能改样式（叠加层事件穿透，见 CSS）。
import type { ReactNode } from "react";
import styles from "./LogicLayer.module.css";

export function LogicLayer({ visible, children }: { visible: boolean; children: ReactNode }) {
  return (
    <div
      data-testid="logic-layer"
      className={styles.layer}
      style={{
        opacity: visible ? 1 : 0,
        pointerEvents: visible ? "auto" : "none",
        visibility: visible ? "visible" : "hidden",
      }}
    >
      {children}
    </div>
  );
}
