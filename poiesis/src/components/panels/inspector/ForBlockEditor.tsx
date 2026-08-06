// components/panels/inspector/ForBlockEditor.tsx
// 最小编辑器 for view @for 块（2026-08-06）：itemName/collection/keyExpr 输入。
// @empty 子元素经元素树管理（"@empty" 子节点挂 children）。

import { useState, useCallback } from "react";
import { usePoiesisStore } from "../../../store";
import type { ForBlockData } from "../../../core/types/element";

interface ForBlockEditorProps {
  blockId: string;
}

const inputStyle: React.CSSProperties = {
  width: "100%",
  boxSizing: "border-box",
  border: "1px solid var(--ol-border)",
  borderRadius: 4,
  padding: "4px 8px",
  fontSize: 12,
  fontFamily: "var(--ol-font-family-mono)",
  background: "var(--ol-surface)",
  color: "var(--ol-text)",
  outline: "2px solid transparent",
};

const labelStyle: React.CSSProperties = {
  display: "block",
  fontSize: 11,
  color: "var(--ol-text-secondary)",
  marginBottom: 4,
  fontWeight: 600,
};

export function ForBlockEditor({ blockId }: ForBlockEditorProps) {
  const forBlock = usePoiesisStore((s) =>
    s.elements.find((e) => e.id === blockId && e.nodeType === "for")
  ) as ForBlockData | undefined;

  const [itemName, setItemName] = useState(forBlock?.itemName ?? "item");
  const [collection, setCollection] = useState(forBlock?.collection ?? "items");
  const [keyExpr, setKeyExpr] = useState(forBlock?.keyExpr ?? "");

  // 块可能在编辑中变化（如解析往返）——同步本地状态
  const current = usePoiesisStore((s) =>
    s.elements.find((e) => e.id === blockId && e.nodeType === "for")
  ) as ForBlockData | undefined;

  const handleCommit = useCallback(() => {
    if (!current) return;
    usePoiesisStore.getState().updateForBlock(blockId, {
      itemName: itemName.trim() || "item",
      collection: collection.trim() || "items",
      ...(keyExpr.trim() ? { keyExpr: keyExpr.trim() } : { keyExpr: undefined }),
    });
  }, [blockId, itemName, collection, keyExpr, current]);

  if (!forBlock) {
    return <div style={{ padding: 12, color: "var(--ol-text-muted)", fontSize: 12 }}>未选择循环块</div>;
  }

  return (
    <div className="context-inspector" data-section="for-editor" style={{ padding: 12, overflow: "auto", height: "100%", boxSizing: "border-box", fontSize: 12 }}>
      <div style={{ marginBottom: 10 }}>
        <label style={labelStyle}>循环项名（@for {"{item}"} in ...）</label>
        <input
          aria-label="循环项名"
          style={inputStyle}
          value={itemName}
          onChange={(e) => setItemName(e.target.value)}
          onBlur={handleCommit}
          onKeyDown={(e) => { if (e.key === "Enter") handleCommit(); }}
        />
      </div>
      <div style={{ marginBottom: 10 }}>
        <label style={labelStyle}>遍历集合（items / TodoStore.items）</label>
        <input
          aria-label="遍历集合"
          style={inputStyle}
          value={collection}
          onChange={(e) => setCollection(e.target.value)}
          onBlur={handleCommit}
          onKeyDown={(e) => { if (e.key === "Enter") handleCommit(); }}
        />
      </div>
      <div style={{ marginBottom: 10 }}>
        <label style={labelStyle}>key 表达式（可选，如 item.id）</label>
        <input
          aria-label="key 表达式"
          style={inputStyle}
          value={keyExpr}
          placeholder="item.id"
          onChange={(e) => setKeyExpr(e.target.value)}
          onBlur={handleCommit}
          onKeyDown={(e) => { if (e.key === "Enter") handleCommit(); }}
        />
      </div>
      <div style={{ color: "var(--ol-text-muted)", fontSize: 11, lineHeight: 1.6 }}>
        循环体内添加子元素：在元素树中选中循环块 → 添加子元素。
        @empty 空态：元素树中「@empty」节点下添加。
      </div>
    </div>
  );
}
