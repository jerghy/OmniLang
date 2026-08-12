// components/design/PaletteDrawer.tsx
// [组件库] 弹出面板（M2 T6）：薄壳承载 ComponentPalette（拖拽能力原样复用，零拷贝）；
// 底栏 [组件库] 按钮开关（DesignPage 受控 paletteOpen）。fixed 定位，左栏位置弹出。
// 复审（M2 T6）：aria-label（StoreDrawer 先例）+ Escape 关闭 + 错误边界包裹。
import { useEffect } from "react";
import { ComponentPalette } from "../panels/ComponentPalette";
import { PanelErrorBoundary } from "./PanelErrorBoundary";
import styles from "./PaletteDrawer.module.css";

export function PaletteDrawer({ onClose }: { onClose: () => void }) {
  // ESC key to close（StoreDrawer 先例）
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div className={styles.drawer} data-testid="palette-drawer">
      <div className={styles.header}>
        <span className={styles.title}>组件库</span>
        <button
          type="button"
          data-testid="palette-drawer-close"
          className={styles.closeBtn}
          onClick={onClose}
          aria-label="关闭组件库"
          title="关闭组件库"
        >
          ×
        </button>
      </div>
      <div className={styles.body}>
        <PanelErrorBoundary name="组件面板">
          <ComponentPalette />
        </PanelErrorBoundary>
      </div>
    </div>
  );
}
