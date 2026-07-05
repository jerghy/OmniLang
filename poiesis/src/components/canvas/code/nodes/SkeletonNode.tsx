import { Handle, Position } from "@xyflow/react";

interface Props {
  data: {
    label: string;
    skeletonType: "if" | "else" | "for" | "while";
    condition?: string;
  };
}

const LABELS: Record<string, string> = {
  if: "如果",
  else: "否则",
  for: "计数循环",
  while: "条件循环",
};

const ICONS: Record<string, string> = {
  if: "◇",
  else: "◇",
  for: "↻",
  while: "⟳",
};

export function SkeletonNode({ data }: Props) {
  const label = LABELS[data.skeletonType] || data.label;
  const icon = ICONS[data.skeletonType] || "";
  const hasTwoBranches = data.skeletonType === "if";

  return (
    <div
      className="flow-node"
      data-node-type="skeleton"
      style={{
        background: "#FFFBEB",
        border: "2px solid #F59E0B",
        borderRadius: 8,
        padding: "8px 14px",
        minWidth: 130,
        textAlign: "center",
        fontSize: 12,
        fontWeight: 500,
      }}
    >
      <div>
        {icon} {label}
      </div>
      {data.condition && (
        <div
          style={{
            fontSize: 10,
            color: "#92400E",
            marginTop: 2,
            maxWidth: 140,
            overflow: "hidden",
            textOverflow: "ellipsis",
          }}
        >
          {data.condition}
        </div>
      )}
      <Handle
        type="target"
        position={Position.Top}
        id="flow-in"
        style={{ background: "#F59E0B" }}
      />
      <Handle
        type="source"
        position={Position.Bottom}
        id="flow-true"
        style={{ background: "#10B981", left: hasTwoBranches ? "30%" : "50%" }}
        title={data.skeletonType === "if" ? "是" : "循环体"}
      />
      {hasTwoBranches && (
        <Handle
          type="source"
          position={Position.Bottom}
          id="flow-false"
          style={{ background: "#EF4444", left: "70%" }}
          title="否"
        />
      )}
      <Handle
        type="target"
        position={Position.Left}
        id="data-in"
        style={{ background: "#FBBF24" }}
        title={data.skeletonType === "if" ? "条件" : "循环条件"}
      />
    </div>
  );
}
