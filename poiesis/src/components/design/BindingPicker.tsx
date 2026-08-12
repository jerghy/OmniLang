// BindingPicker（M3 T3 五类槽点名绑定）：点槽弹出的来源清单浮层。
// 来源三类：组件信号/store 信号（collectSourceNodes 映射）、fn 出口、醒着的画布元素。
// 分组展示（信号 / fn 出口 / 画布元素）；点击来源 → onPick(来源)；右上关闭 → onClose。
// 纯 props 组件，无 store 依赖（来源由 ContentTab 组装）。
import styles from "./BindingPicker.module.css";

export interface BindingSource {
  id: string; // `sig:${pid}` | `fn:${pid}` | `el:${elId}`
  kind: "signal" | "fn" | "element";
  name: string; // 显示名（信号原名 / fn 名 / 元素名）
  type: string; // 类型标注（fn 出口为签名）
}

interface BindingPickerProps {
  sources: BindingSource[];
  onPick: (source: BindingSource) => void;
  onClose: () => void;
}

// 分组顺序与标题；kind 联合完整覆盖（signal 含 store/computed 变体归一）
const GROUPS: Array<{ kind: BindingSource["kind"]; label: string }> = [
  { kind: "signal", label: "信号" },
  { kind: "fn", label: "fn 出口" },
  { kind: "element", label: "画布元素" },
];

export function BindingPicker({ sources, onPick, onClose }: BindingPickerProps) {
  return (
    <div className={styles.overlay} data-testid="binding-picker" role="dialog" aria-label="选择绑定来源">
      <div className={styles.header}>
        <span className={styles.title}>选择来源</span>
        <button type="button" className={styles.close} data-testid="binding-picker-close" onClick={onClose} aria-label="关闭">
          ✕
        </button>
      </div>
      <div className={styles.list}>
        {GROUPS.map((g) => {
          const group = sources.filter((s) => s.kind === g.kind);
          if (group.length === 0) return null;
          return (
            <div key={g.kind} className={styles.group}>
              <div className={styles.groupLabel}>{g.label}</div>
              {group.map((s) => (
                <button type="button" key={s.id} className={styles.item} data-source-id={s.id} onClick={() => onPick(s)}>
                  <span className={styles.itemName}>{s.name}</span>
                  <span className={styles.itemType}>{s.type}</span>
                </button>
              ))}
            </div>
          );
        })}
        {sources.length === 0 && <div className={styles.empty}>暂无可用来源</div>}
      </div>
    </div>
  );
}
