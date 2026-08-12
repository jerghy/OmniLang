// 端口手柄节点（M3 T5）：画布元素右缘端口 = 0×0 ReactFlow 节点内的两只叠加 Handle。
// 源/目标两 Handle 同钉元素原点（node 0×0 → 默认 .react-flow__handle-right
// `translate(50%,-50%)` 将 10px 圆精确居中于端口点），source 后渲染压在上层：
// 拖线以 source 类型起手，配合 ConnectionMode.Loose（PortHandleLayer）任意他端可落。
// 手柄静息即可交互（拖动起线）由 PortHandleLayer.module.css:10-12 的覆盖规则保证：
// `.layer :global(.react-flow__handle) { pointer-events: auto }` —— 层 pointer-events:
// none 事件穿透下手柄必须显式恢复 auto。该规则负载全部拖线交互，删除将静默失效。
// （ReactFlow 的 `connectionindicator` 类仅连线进行中生效，静息态不兜底。）
import { memo } from "react";
import { Handle, Position, type Node, type NodeProps } from "@xyflow/react";
import styles from "./PortHandleLayer.module.css";

// type 别名（非 interface）：隐式索引签名满足 Node<NodeData extends Record<string, unknown>> 约束
export type PortNodeData = {
  elId: string;
  slotKey: string;
  label: string;
};

export const PortHandleNode = memo(function PortHandleNode({ data }: NodeProps<Node<PortNodeData>>) {
  const srcId = `${data.slotKey}-src`;
  const tgtId = `${data.slotKey}-tgt`;
  return (
    // 1×1 最小尺寸：ReactFlow 0×0 节点不初始化 edges 渲染层（实机验证，2026-08-12）
    <div data-testid="port-handle-node" style={{ width: 1, height: 1 }}>
      <Handle type="target" id={tgtId} position={Position.Right} className={styles.handle} />
      <Handle type="source" id={srcId} position={Position.Right} className={styles.handle} />
    </div>
  );
});
