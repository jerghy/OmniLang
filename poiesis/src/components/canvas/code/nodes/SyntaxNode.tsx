import { Handle, Position } from "@xyflow/react";
import React from "react";
import { formatSyntaxNode } from "../../../../services/codegen/syntax-nodes";
import styles from "./SyntaxNode.module.css";

interface Props {
  data: {
    label: string;
    tag: string;
    props?: Record<string, unknown>;
  };
}

/** 语法节点（let/assign/store-call/call）：卡片显示 props 生成的代码预览。 */
const SyntaxNode = React.memo(function SyntaxNode({ data }: Props) {
  const code = formatSyntaxNode(data.tag, (data.props ?? {}) as never);
  return (
    <div
      className={`flow-node ${styles.node}`}
      data-node-type={data.tag}
      data-syntax-code={code}
    >
      <code>{code}</code>
      <Handle
        type="target"
        position={Position.Top}
        id="flow-in"
        style={{ background: "var(--ol-text-muted)" }}
      />
      <Handle
        type="source"
        position={Position.Bottom}
        id="flow-out"
        style={{ background: "var(--ol-text-muted)" }}
      />
    </div>
  );
});
export { SyntaxNode };
