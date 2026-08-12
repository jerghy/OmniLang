// poiesis/src/components/canvas/ValueEditor.tsx
// 改值浮层（M2）：点源节点卡片弹出，编辑值 → setDebugValue 管线 → 画布预览当场演。
// 调试专用：不 dirty 项目（debugValues 瞬态）。valid 语义复用 parseDebugValue（与 ComponentDefPanel commitDebugValue 同款）。
import { useState } from "react";
import { usePoiesisStore } from "../../store";
import { parseDebugValue } from "../../services/debug/parse-debug-value";
import type { SourceNode } from "./source-nodes";
import styles from "./ValueEditor.module.css";

export function ValueEditor({ node, onClose }: { node: SourceNode; onClose: () => void }) {
  const setDebugValue = usePoiesisStore((s) => s.setDebugValue);
  const debugValues = usePoiesisStore((s) => s.debugValues);

  // 回显：调试覆盖值优先（剥 String 引号），defaultValue 兜底
  const current = debugValues[node.ownerKey]?.[node.name]?.text
    ?? (typeof node.value === "string" && node.value.startsWith('"') && node.value.endsWith('"')
      ? node.value.slice(1, -1)
      : String(node.value));
  const [raw, setRaw] = useState(current);

  const handleSave = () => {
    const parsed = parseDebugValue(node.type, raw);
    setDebugValue(node.ownerKey, node.name, parsed.valid ? parsed.literal : raw, parsed.valid);
    onClose();
  };

  return (
    <div className={styles.editor} data-testid="value-editor" onClick={(e) => e.stopPropagation()}>
      <div className={styles.header}>
        <span className={styles.name}>{node.name}</span>
        <span className={styles.type}>{node.type}</span>
      </div>
      <input
        data-testid="value-editor-input"
        className={styles.input}
        value={raw}
        onChange={(e) => setRaw(e.target.value)}
        onKeyDown={(e) => { if (e.key === "Enter") handleSave(); if (e.key === "Escape") onClose(); }}
        autoFocus
      />
      <div className={styles.actions}>
        <button type="button" data-testid="value-editor-save" className={styles.saveBtn} onClick={handleSave}>保存</button>
        <button type="button" data-testid="value-editor-close" className={styles.closeBtn} onClick={onClose}>关闭</button>
      </div>
    </div>
  );
}
