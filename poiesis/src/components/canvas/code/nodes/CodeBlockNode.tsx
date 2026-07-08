import { Handle, Position } from "@xyflow/react";
import React from "react";

interface Props {
  data: {
    label: string;
    source?: string;
    onDoubleClick?: () => void;
  };
}

/** Map raw element tags to Chinese display labels */
const TAG_LABELS: Record<string, string> = {
  let: "let 声明",
  const: "const 声明",
  signal: "信号",
  computed: "计算属性",
  store: "Store",
  literal: "字面量",
  getVariable: "读取变量",
  setVariable: "赋值",
  add: "加法",
  sub: "减法",
  mul: "乘法",
  div: "除法",
  compare: "比较",
  and: "与",
  or: "或",
  not: "非",
  concat: "拼接",
  customFn: "函数调用",
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
      className="flow-node"
      data-node-type="code-block"
      onDoubleClick={data.onDoubleClick}
      style={{
        background: "#F8FAFC",
        border: "1.5px solid #94A3B8",
        borderRadius: 6,
        padding: "10px 14px",
        minWidth: 180,
        maxWidth: 280,
        fontSize: 11,
        fontFamily: "monospace",
        cursor: "pointer",
        whiteSpace: "pre-wrap",
        wordBreak: "break-all",
        color: "#334155",
        lineHeight: 1.4,
      }}
    >
      <code>{preview}</code>
      <Handle
        type="target"
        position={Position.Top}
        id="flow-in"
        style={{ background: "#94A3B8" }}
      />
      <Handle
        type="source"
        position={Position.Bottom}
        id="flow-out"
        style={{ background: "#94A3B8" }}
      />
      <Handle
        type="target"
        position={Position.Left}
        id="data-in"
        style={{ background: "#CBD5E1" }}
      />
      <Handle
        type="source"
        position={Position.Right}
        id="data-out"
        style={{ background: "#CBD5E1" }}
      />
    </div>
  );
});
export { CodeBlockNode };
