// components/canvas/code/NodeContextMenu.tsx
// Right-click context menu for nodes in the flow canvas.

import { type RefObject } from "react";
import styles from "./NodeContextMenu.module.css";

interface NodeContextMenuProps {
  nodeMenu: { x: number; y: number; nodeId: string } | null;
  menuRef: RefObject<HTMLDivElement | null>;
  onDelete: (nodeId: string) => void;
  onClose: () => void;
  deleteBtnClass?: string;
}

export function NodeContextMenu({
  nodeMenu,
  menuRef,
  onDelete,
  onClose,
  deleteBtnClass,
}: NodeContextMenuProps) {
  if (!nodeMenu) return null;

  return (
    <div
      ref={menuRef}
      className={styles.menu}
      style={{ top: nodeMenu.y, left: nodeMenu.x }}
      onContextMenu={(e) => e.preventDefault()}
    >
      <button type="button"
        onClick={() => { onDelete(nodeMenu.nodeId); onClose(); }}
        className={deleteBtnClass || styles.deleteBtn}
      >
        🗑 删除节点
      </button>
    </div>
  );
}
