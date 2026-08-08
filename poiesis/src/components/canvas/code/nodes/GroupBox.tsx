// poiesis/src/components/canvas/code/nodes/GroupBox.tsx
// 分组框节点：展开态 = 半透明彩色框 + 标题栏；折叠态 = 紧凑组卡。
// 不渲染 Handle（折叠态的连线重路由在 FlowCanvas 的 edges 层完成）。
import { useState } from "react";
import type { CodeFlowGroup } from "../../../../core/types/project-json";
import { GROUP_COLORS } from "../../../../services/codegen/groups";
import { NodeTextField } from "../fields/NodeField";
import styles from "./GroupBox.module.css";

// data 型 props：与仓库既有节点（FnNode/LoopNode 等）同款轻量 Props 惯例——
// 本版本 @xyflow/react 的 NodeProps 泛型要求 data 满足 Record 索引签名，直接泛型化
// GroupBoxData 无法通过 tsc，故用 `{ data: GroupBoxData }`（nodeTypes 契约要求
// 组件接收 data prop，flat props 会 TS2322）。
export interface GroupBoxData {
  group: CodeFlowGroup;
  editing: boolean;
  onRename: (name: string) => void;
  onColorChange: (color: string) => void;
  onToggleCollapsed: () => void;
  onDelete: () => void;
}

interface Props {
  data: GroupBoxData;
}

export function GroupBox({ data }: Props) {
  const { group, editing, onRename, onColorChange, onToggleCollapsed, onDelete } = data;
  const [colorOpen, setColorOpen] = useState(false);
  // 双击标题进入本地改名态（editing prop 只管「新建组后的自动命名」；双击改名用本地 state，
  // 避免 FlowCanvas 维护双击入口——二者共用 NodeTextField，提交都走 onRename）
  const [localEditing, setLocalEditing] = useState(false);
  const naming = editing || localEditing;

  if (group.collapsed) {
    return (
      <div className={styles.card} style={{ borderColor: group.color }} data-group-id={group.id}>
        <button type="button" className={styles.foldBtn} onClick={onToggleCollapsed} aria-label="展开分组">▸</button>
        <span className={styles.cardName}>{group.name}</span>
        <span className={styles.cardMeta}>{group.memberIds.length} 条语句</span>
      </div>
    );
  }

  return (
    <div className={styles.box} style={{ borderColor: group.color, background: `${group.color}14` }} data-group-id={group.id}>
      <div className={styles.titleBar}>
        {naming ? (
          <NodeTextField value={group.name} onCommit={(v) => { onRename(v); setLocalEditing(false); }} placeholder="分组名" ariaLabel="分组名" className={styles.nameInput} />
        ) : (
          <span className={styles.name} onDoubleClick={() => setLocalEditing(true)}>{group.name}</span>
        )}
        <button type="button" className={styles.foldBtn} onClick={onToggleCollapsed} aria-label="折叠分组">▾</button>
        <div className={styles.colorWrap}>
          <button type="button" className={styles.colorBtn} onClick={() => setColorOpen((o) => !o)} aria-label="分组颜色" style={{ background: group.color }} />
          {colorOpen && (
            <div className={styles.palette}>
              {GROUP_COLORS.map((c) => (
                <button key={c} type="button" aria-label={`颜色 ${c}`} className={styles.swatch}
                  style={{ background: c }} onClick={() => { onColorChange(c); setColorOpen(false); }} />
              ))}
            </div>
          )}
        </div>
        <button type="button" className={styles.deleteBtn} onClick={onDelete} aria-label="删除分组">×</button>
      </div>
    </div>
  );
}
