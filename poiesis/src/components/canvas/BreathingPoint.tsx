// poiesis/src/components/canvas/BreathingPoint.tsx
// 呼吸点：设计窗口角落的小脉搏——点 = 整页醒（逻辑层浮现），再点 = 睡（干净画布）。
import { usePoiesisStore } from "../../store";
import styles from "./BreathingPoint.module.css";

export function BreathingPoint() {
  const awake = usePoiesisStore((s) => s.designPage.awake);
  const setAwake = usePoiesisStore((s) => s.setAwake);
  const clearSelection = usePoiesisStore((s) => s.clearSelection);

  return (
    <button
      type="button"
      data-testid="breathing-point"
      className={`${styles.pulse} ${awake ? styles.awake : ""}`}
      title={awake ? "睡回（隐藏逻辑层）" : "醒来（显示逻辑层）"}
      onClick={() => { setAwake(!awake); clearSelection(); }}
    >
      <span className={styles.dot} />
    </button>
  );
}
