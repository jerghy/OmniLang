// poiesis/src/components/canvas/ValueEditor.tsx
// 改值浮层（M2）：点源节点卡片弹出，编辑值 → setDebugValue 管线 → 画布预览当场演。
// 调试专用：不 dirty 项目（debugValues 瞬态）。valid 语义复用 parseDebugValue（与 ComponentDefPanel commitDebugValue 同款）。
import { useState } from "react";
import { usePoiesisStore } from "../../store";
import { parseDebugValue } from "../../services/debug/parse-debug-value";
import type { SourceNode } from "./source-nodes";
import styles from "./ValueEditor.module.css";

/** 回显剥引号（T5 复审 Minor 3/4）：成对 '"' / "'" 剥除（长度≥2 守卫），与 ComponentDefPanel displayDebugText 同约定；其余原文透传 */
function displayValue(text: string): string {
  if (
    text.length >= 2 &&
    ((text.startsWith('"') && text.endsWith('"')) || (text.startsWith("'") && text.endsWith("'")))
  ) {
    return text.slice(1, -1);
  }
  return text;
}

export function ValueEditor({ node, onClose }: { node: SourceNode; onClose: () => void }) {
  const setDebugValue = usePoiesisStore((s) => s.setDebugValue);
  const debugValues = usePoiesisStore((s) => s.debugValues);

  // 回显：调试覆盖值优先（valid 剥引号；invalid 存的是原文 → 不剥），defaultValue 兜底（undefined → "—"）
  const entry = debugValues[node.ownerKey]?.[node.name];
  const current = entry
    ? (entry.valid ? displayValue(entry.text) : entry.text)
    : (node.value == null ? "—" : displayValue(String(node.value)));
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
        onKeyDown={(e) => {
          // IME 组合期 Enter 不得保存（拼音选字回车）；逐事件判定，免疫粘滞态（NodeFormulaField 同款）
          if (e.key === "Enter" && !e.nativeEvent.isComposing) handleSave();
          if (e.key === "Escape") onClose();
        }}
        autoFocus
      />
      <div className={styles.actions}>
        <button type="button" data-testid="value-editor-save" className={styles.saveBtn} onClick={handleSave}>保存</button>
        <button type="button" data-testid="value-editor-close" className={styles.closeBtn} onClick={onClose}>关闭</button>
      </div>
    </div>
  );
}
