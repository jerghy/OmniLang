// poiesis/src/components/canvas/code/nodes/GroupBox.tsx
// 分组框节点：展开态 = 半透明彩色框 + 标题栏；折叠态 = 紧凑组卡。
// 折叠态组卡渲染 flow-in/flow-out Handle（error008 修复：连线重路由到组节点后，
// ReactFlow 要求节点存在对应 id 的 Handle，缺失即静默丢弃边）。
import React, { useState } from "react";
import { Handle, Position } from "@xyflow/react";
import type { CodeFlowGroup } from "../../../../core/types/project-json";
import { GROUP_COLORS } from "../../../../services/codegen/groups";
import { NodeTextField } from "../fields/NodeField";
import styles from "./GroupBox.module.css";

// Handle 依赖 ReactFlowProvider（连线渲染）；单元测试直接渲染节点时静默跳过，避免抛错。
class HandlesBoundary extends React.Component<{ children: React.ReactNode }, { failed: boolean }> {
  state = { failed: false };
  static getDerivedStateFromError() {
    return { failed: true };
  }
  render() {
    return this.state.failed ? null : this.props.children;
  }
}

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
        {/* error008 修复：折叠态连线重路由到组节点（source/target=g1，handle=flow-in/out），
            ReactFlow 要求 target/source 节点渲染对应 id 的 Handle，缺失即静默丢弃边。
            进线接组卡左缘（flow-in）、出线发自右缘（flow-out）——设计文档 §4。 */}
        <HandlesBoundary>
          <Handle type="target" position={Position.Left} id="flow-in" className="codeflow-handle" style={{ background: "var(--ol-text-muted)" }} />
          <Handle type="source" position={Position.Right} id="flow-out" className="codeflow-handle" style={{ background: "var(--ol-text-muted)" }} />
        </HandlesBoundary>
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
