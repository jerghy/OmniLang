// components/canvas/SourceNodeCard.tsx
// 源节点卡片（M2）：图标 + 名 + 当前值。@dnd-kit 可拖（数据区拖动摆放）；
// 点击 → onEdit（T5 改值浮层入口）。
import { useDraggable } from "@dnd-kit/core";
import type { ReactNode } from "react";
import type { SourceNode } from "./source-nodes";
import styles from "./SourceNodeCard.module.css";

// B2 形象体系（设计稿 §15.2 拍板映射）：signal 小灯 / computed 算盘 / store 账本 /
// store-getter 账本+算页签 / inline-signal 小灯+本地角标。data-kind 供测试与主题挂钩。
const KIND_ICON: Record<SourceNode["kind"], ReactNode> = {
  signal: (
    <svg data-kind="signal" viewBox="0 0 16 16" width="14" height="14" aria-hidden>
      <path d="M8 2v1M8 6v1M8 10v1M8 13v1" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" fill="none" />
      <circle cx="8" cy="4" r="1.6" fill="currentColor" />
      <circle cx="8" cy="8" r="1.6" fill="currentColor" />
      <circle cx="8" cy="12" r="1.6" fill="currentColor" />
    </svg>
  ), // 小灯（串灯）
  computed: (
    <svg data-kind="computed" viewBox="0 0 16 16" width="14" height="14" aria-hidden>
      <rect x="3" y="2" width="10" height="12" rx="1" fill="none" stroke="currentColor" />
      <path d="M5 5h6M5 8h6M5 11h4" stroke="currentColor" />
    </svg>
  ), // 算盘（框+算珠）
  "store-signal": (
    <svg data-kind="store-signal" viewBox="0 0 16 16" width="14" height="14" aria-hidden>
      <path d="M3 4h10v9H3z" fill="none" stroke="currentColor" />
      <path d="M3 4l2-2h6l2 2" fill="none" stroke="currentColor" />
    </svg>
  ), // 账本（开本）
  "store-getter": (
    <svg data-kind="store-getter" viewBox="0 0 16 16" width="14" height="14" aria-hidden>
      <path d="M3 4h10v9H3z" fill="none" stroke="currentColor" />
      <path d="M3 4l2-2h6l2 2" fill="none" stroke="currentColor" />
      <path d="M8 7v4M6 9h4" stroke="currentColor" />
    </svg>
  ), // 账本+算（+号）
  "inline-signal": (
    <svg data-kind="inline-signal" viewBox="0 0 16 16" width="14" height="14" aria-hidden>
      <path d="M8 2v1M8 6v1M8 10v1M8 13v1" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" fill="none" />
      <circle cx="8" cy="4" r="1.6" fill="currentColor" />
      <circle cx="8" cy="8" r="1.6" fill="currentColor" />
      <circle cx="8" cy="12" r="1.6" fill="currentColor" />
    </svg>
  ), // 小灯同形（本地角标区分）
};

export function SourceNodeCard({ node, onEdit }: { node: SourceNode; onEdit: (node: SourceNode) => void }) {
  const { attributes, listeners, setNodeRef, isDragging, transform } = useDraggable({ id: node.id });

  return (
    <div
      ref={setNodeRef}
      data-testid="source-node-card"
      data-node-id={node.id}
      className={`${styles.card} ${isDragging ? styles.dragging : ""}`}
      // 拖拽中实时跟随（dnd-kit transform 瞬时生效；松手后 drop delta 累积进层级 positions——两层各自独立，不重复应用）
      style={transform ? { transform: `translate(${transform.x}px, ${transform.y}px)` } : undefined}
      {...attributes}
      {...listeners}
      // 点击/拖拽分离依赖宿主 DndContext 的 activationConstraint（distance:5）——真实拖拽会抑制点击（dnd-kit capture 阶段 stopPropagation）
      onClick={() => onEdit(node)}
    >
      <span className={`${styles.iconWrap} ${styles["kind-" + node.kind]}`} data-kind={node.kind}>
        {KIND_ICON[node.kind]}
      </span>
      {node.kind === "store-getter" && (
        <span className={styles.badge} data-testid="kind-badge">
          算
        </span>
      )}
      {node.kind === "inline-signal" && <span className={styles.badge}>本地</span>}
      <span className={styles.name} title={node.name}>
        {node.name}
      </span>
      <span className={styles.value} data-testid="source-node-value" title={String(node.value ?? "—")}>
        {String(node.value ?? "—")}
      </span>
    </div>
  );
}
