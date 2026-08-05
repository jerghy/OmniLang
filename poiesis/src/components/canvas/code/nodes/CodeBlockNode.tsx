import { Handle, Position } from "@xyflow/react";
import React from "react";
import styles from "./CodeBlockNode.module.css";
import "./node-base.css";

interface Props {
  data: {
    label: string;
    source?: string;
    description?: string;
    onDoubleClick?: () => void;
  };
}

const TAG_LABELS: Record<string, string> = {
  "code-block": "代码块",
};

const CodeBlockNode = React.memo(function CodeBlockNode({ data }: Props) {
  const chineseLabel = TAG_LABELS[data.label] || data.label;
  const preview = data.source
    ? data.source.length > 50
      ? data.source.slice(0, 50) + "..."
      : data.source
    : chineseLabel;

  return (
    <div
      className={`flow-node ${styles.node} codeflow-node`}
      data-node-type="code-block"
      onDoubleClick={data.onDoubleClick}
    >
      <code>{preview}</code>
      {data.description && (
        <div className={styles.description}>
          {data.description}
        </div>
      )}
      <Handle
        className="codeflow-handle"
        type="target"
        position={Position.Top}
        id="flow-in"
        style={{ background: "var(--ol-text-muted)" }}
      />
      <Handle
        className="codeflow-handle"
        type="source"
        position={Position.Bottom}
        id="flow-out"
        style={{ background: "var(--ol-text-muted)" }}
      />
    </div>
  );
});
export { CodeBlockNode };
