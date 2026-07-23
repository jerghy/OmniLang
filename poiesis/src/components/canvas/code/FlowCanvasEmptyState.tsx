// components/canvas/code/FlowCanvasEmptyState.tsx
// Empty state shown when the flow canvas has no nodes.

import styles from "./FlowCanvasEmptyState.module.css";

interface FlowCanvasEmptyStateProps {
  codeFlowElementId: string | null;
  onContextMenu: (event: React.MouseEvent | MouseEvent) => void;
}

export function FlowCanvasEmptyState({
  codeFlowElementId,
  onContextMenu,
}: FlowCanvasEmptyStateProps) {
  return (
    <div
      onContextMenu={onContextMenu}
      className={styles.container}
    >
      <div>
        {codeFlowElementId
          ? "双击组件元素进入代码节点流编辑"
          : "右键画布添加节点，或从组件标签页双击进入"}
      </div>
      <div className={styles.hintText}>右键 → 添加节点</div>
    </div>
  );
}
