// poiesis/src/components/canvas/code/nodes/LifecycleNode.tsx
// 生命周期块节点：摘要渲染 @mount / @effect(deps) / @unmount，flow-out → body 链。
// 对标 LoopNode 块形态（CSS module + HandlesBoundary + data-handle-id 端口类）。

import React, { memo } from "react";
import { Handle, Position } from "@xyflow/react";
import type { FlowScope } from "../../../../services/codegen/flow-scope";
import styles from "./LifecycleNode.module.css";
import "./node-base.css";

interface LifecycleNodeData {
  tag: "lifecycle";
  props: { hookType: "mount" | "effect" | "unmount"; deps?: string };
  source?: string;
  scope?: FlowScope;
  letVars?: Array<{ name: string }>;
  onEditProp?: (id: string, prop: string, value: unknown) => void;
}

interface Props {
  id?: string;
  data: LifecycleNodeData;
}

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

export const LifecycleNode = memo(function LifecycleNode({ id, data }: Props) {
  const hookType = data.props?.hookType ?? "mount";
  const deps = data.props?.deps ?? "";
  const depsPart = hookType === "effect" && deps ? `(${deps})` : "";
  return (
    <div className={`${styles.node} codeflow-node nodrag`}>
      <HandlesBoundary>
        <Handle type="target" position={Position.Top} id="flow-in" className={`${styles.handle} codeflow-handle`} data-handle-id="flow-in" />
      </HandlesBoundary>
      <div className={`${styles.header} codeflow-header`} data-testid="lifecycle-summary">{`@${hookType}${depsPart}`}</div>
      <HandlesBoundary>
        <Handle type="source" position={Position.Bottom} id="flow-out" className={`${styles.handle} codeflow-handle`} data-handle-id="flow-out" />
      </HandlesBoundary>
    </div>
  );
});
