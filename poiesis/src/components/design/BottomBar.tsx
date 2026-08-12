// poiesis/src/components/design/BottomBar.tsx
// 底栏（M2）：左 = 面包屑；右 = [组件库]（弹出面板开关，DesignPage 受控）+ [数据节点]（源节点显隐开关）。
// [数据节点] 激活态 = 开关开 && 整页醒（源节点层由 awake 门控）；开关状态 = store 瞬态（dataNodesVisible）。
import { usePoiesisStore } from "../../store";
import { BreadcrumbBar } from "./BreadcrumbBar";
import styles from "./BottomBar.module.css";

export function BottomBar({ onPaletteToggle }: { onPaletteToggle: () => void }) {
  const dataNodesVisible = usePoiesisStore((s) => s.designPage.dataNodesVisible);
  const awake = usePoiesisStore((s) => s.designPage.awake);
  const setDataNodesVisible = usePoiesisStore((s) => s.setDataNodesVisible);
  const active = dataNodesVisible && awake;

  return (
    <div className={styles.bar} data-testid="bottom-bar">
      <div className={styles.left}><BreadcrumbBar /></div>
      <div className={styles.right}>
        <button
          type="button"
          data-testid="palette-toggle"
          className={styles.btn}
          onClick={onPaletteToggle}
          title="组件库（拖拽元素到画布）"
        >
          🧩 组件库
        </button>
        <button
          type="button"
          data-testid="data-nodes-toggle"
          className={`${styles.btn} ${active ? styles.btnActive : ""}`}
          aria-pressed={active}
          onClick={() => setDataNodesVisible(!dataNodesVisible)}
          title={dataNodesVisible ? "隐藏源节点" : "显示源节点（醒态下可见）"}
        >
          🔌 数据节点
        </button>
      </div>
    </div>
  );
}
