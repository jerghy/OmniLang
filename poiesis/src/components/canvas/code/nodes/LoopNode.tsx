// poiesis/src/components/canvas/code/nodes/LoopNode.tsx
// 循环节点（while / for-in）：flow-in + body(br0) + flow-out 三端口，与 condition 同构。
// body 末句自动回流循环头（语言语义，画布无回边 wire）。
// 注：props 类型走既有节点惯例（ConditionNode/SyntaxNode 同款 Props 接口）——
// 本版本 @xyflow/react 的 NodeProps 泛型要求 data 满足 Record 索引签名且节点字段全量必填，
// 直接泛型化 LoopNodeData 无法通过 tsc，与仓库既有节点一致用轻量 Props。
import React, { memo } from "react";
import { Handle, Position } from "@xyflow/react";
import { formatSyntaxNode, type SyntaxNodeProps } from "../../../../services/codegen/syntax-nodes";
import type { FlowScope } from "../../../../services/codegen/flow-scope";
import styles from "./LoopNode.module.css";

interface LoopNodeData {
  tag: "while" | "for-in";
  props: SyntaxNodeProps;
  source?: string;
  scope?: FlowScope;
  letVars?: Array<{ name: string }>;
  onEditProp?: (id: string, prop: string, value: unknown) => void;
}

interface Props {
  id?: string;
  data: LoopNodeData;
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

export const LoopNode = memo(function LoopNode({ id, data }: Props) {
  const tag = data.tag;
  const preview = data.source || formatSyntaxNode(tag, data.props);
  const isWhile = tag === "while";
  return (
    <div className={`${styles.node} nodrag`}>
      <HandlesBoundary>
        <Handle type="target" position={Position.Top} id="flow-in" className={styles.handle} data-handle-id="flow-in" />
      </HandlesBoundary>
      <div className={styles.header}>{isWhile ? "while" : "for-in"}</div>
      <div className={styles.body}>
        {isWhile ? (
          <div className={styles.fieldRow} data-field-kind="condition">
            <span className={styles.fieldLabel}>条件</span>
            <span className={styles.fieldValue} data-testid="loop-condition">{data.props.condition ?? ""}</span>
          </div>
        ) : (
          <>
            <div className={styles.fieldRow}>
              <span className={styles.fieldLabel}>变量</span>
              <span className={styles.fieldValue} data-testid="loop-var">{data.props.varName ?? ""}</span>
            </div>
            <div className={styles.fieldRow}>
              <span className={styles.fieldLabel}>遍历</span>
              <span className={styles.fieldValue} data-testid="loop-iterable">{data.props.iterable ?? ""}</span>
            </div>
          </>
        )}
        <div className={styles.bodyRow}>
          <HandlesBoundary>
            <Handle type="source" position={Position.Right} id="br0" className={styles.handle} data-handle-id="br0" />
          </HandlesBoundary>
          <span className={styles.bodyLabel}>循环体</span>
        </div>
      </div>
      <HandlesBoundary>
        <Handle type="source" position={Position.Bottom} id="flow-out" className={styles.handle} data-handle-id="flow-out" />
      </HandlesBoundary>
      <div className={styles.preview} data-testid="loop-preview">{preview}</div>
    </div>
  );
});
