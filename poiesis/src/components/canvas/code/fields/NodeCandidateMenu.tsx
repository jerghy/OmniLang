// poiesis/src/components/canvas/code/fields/NodeCandidateMenu.tsx
// 公式框候选弹层（受控）：父组件（NodeFormulaField）持有键盘导航状态，本组件纯展示。
// 交互模式复刻 NodeSearchMenu（role=listbox/option、aria-selected、命中项高亮）。
import { useEffect, useRef } from "react";
import styles from "./NodeCandidateMenu.module.css";

export interface CandidateItem { value: string; label: string }
export interface CandidateGroup { label: string; items: CandidateItem[] }

interface Props {
  x: number;
  y: number;
  groups: CandidateGroup[];
  activeIdx: number;
  onSelect: (item: CandidateItem) => void;
  onMouseEnter: (idx: number) => void;
  onClose: () => void;
}

export function NodeCandidateMenu({ x, y, groups, activeIdx, onSelect, onMouseEnter, onClose }: Props) {
  const rootRef = useRef<HTMLDivElement>(null);
  const flat = groups.flatMap((g) => g.items);

  // 点击菜单外关闭
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) onClose();
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [onClose]);

  // 选中项滚动可见
  useEffect(() => {
    rootRef.current?.querySelector(`[data-cand-index="${activeIdx}"]`)?.scrollIntoView?.({ block: "nearest" });
  }, [activeIdx]);

  return (
    <div ref={rootRef} className={styles.menu} style={{ left: x, top: y }} role="listbox" aria-label="表达式候选">
      {flat.length === 0 ? (
        <div className={styles.empty}>无匹配项</div>
      ) : (
        groups.map((g) => (
          <div key={g.label}>
            <div className={styles.sectionLabel}>{g.label}</div>
            {g.items.map((item) => {
              const idx = flat.indexOf(item);
              return (
                <button
                  type="button"
                  key={item.value}
                  data-cand-index={idx}
                  role="option"
                  aria-selected={idx === activeIdx}
                  className={`${styles.item} ${idx === activeIdx ? styles.itemActive : ""}`}
                  onClick={() => onSelect(item)}
                  onMouseEnter={() => onMouseEnter(idx)}
                  // 防夺焦：mousedown 不把焦点从输入框夺走，避免 input blur 提前提交草稿且菜单卸载导致 click 丢失
                  onMouseDown={(e) => e.preventDefault()}
                >
                  <span>{item.label}</span>
                </button>
              );
            })}
          </div>
        ))
      )}
    </div>
  );
}
