import { Handle, Position } from "@xyflow/react";

interface Props {
  data: {
    label: string;
    skeletonType: "if" | "else" | "for" | "while";
    condition?: string;
  };
}

export function SkeletonNode({ data }: Props) {
  const icons: Record<string, string> = {
    if: "◇",
    else: "◇",
    for: "↻",
    while: "⟳",
  };

  const hasTwoBranches = data.skeletonType === "if";

  return (
    <div
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
        {icons[data.skeletonType]} {data.label}
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
        title="true"
      />
      {hasTwoBranches && (
        <Handle
          type="source"
          position={Position.Bottom}
          id="flow-false"
          style={{ background: "#EF4444", left: "70%" }}
          title="false"
        />
      )}
      <Handle
        type="target"
        position={Position.Left}
        id="data-in"
        style={{ background: "#FBBF24" }}
      />
    </div>
  );
}
