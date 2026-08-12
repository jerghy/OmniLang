// components/canvas/SourceNodeCard.tsx
// 源节点卡片（M2）：图标 + 名 + 当前值。@dnd-kit 可拖（数据区拖动摆放）；
// 点击 → onEdit（T5 改值浮层入口）。
import { useDraggable } from "@dnd-kit/core";
import type { SourceNode } from "./source-nodes";
import styles from "./SourceNodeCard.module.css";

export const KIND_ICONS: Record<SourceNode["kind"], string> = {
  signal: "⚡",
  computed: "🧮",
  "store-signal": "📦",
  "store-getter": "📦",
  "inline-signal": "🔌",
};

export function SourceNodeCard({ node, onEdit }: { node: SourceNode; onEdit: (node: SourceNode) => void }) {
  const { attributes, listeners, setNodeRef, isDragging } = useDraggable({ id: node.id });

  return (
    <div
      ref={setNodeRef}
      data-testid="source-node-card"
      data-node-id={node.id}
      className={`${styles.card} ${isDragging ? styles.dragging : ""}`}
      {...attributes}
      {...listeners}
      // 点击/拖拽分离依赖宿主 DndContext 的 activationConstraint（distance:5）——真实拖拽会抑制点击（dnd-kit capture 阶段 stopPropagation）
      onClick={() => onEdit(node)}
    >
      <span className={styles.icon}>{KIND_ICONS[node.kind]}</span>
      <span className={styles.name} title={node.name}>
        {node.name}
      </span>
      <span className={styles.value} data-testid="source-node-value" title={String(node.value ?? "—")}>
        {String(node.value ?? "—")}
      </span>
    </div>
  );
}
