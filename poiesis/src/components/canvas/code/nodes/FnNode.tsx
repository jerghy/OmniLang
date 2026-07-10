import { useState } from "react";
import { Handle, Position } from "@xyflow/react";

interface Props {
  data: {
    label: string;
    fnName?: string;
    params?: string;
    /** Callback when params change. Called with new params string like "text: String, id: i32". */
    onParamsChange?: (newParams: string) => void;
    onDoubleClick?: () => void;
  };
}

export function FnNode({ data }: Props) {
  const name = data.fnName || data.label || "fn";
  const params = data.params || "";
  const [showParams, setShowParams] = useState(false);

  // Parse current params string into array
  const parsedParams = params
    ? params.split(",").map((p) => {
        const [pName, pType] = p.trim().split(":").map((s) => s.trim());
        return { name: pName || "", type: pType || "" };
      })
    : [];

  const handleToggleParams = (e: React.MouseEvent) => {
    e.stopPropagation();
    setShowParams(!showParams);
  };

  const handleParamChange = (index: number, field: "name" | "type", value: string) => {
    const newParams = [...parsedParams];
    newParams[index] = { ...newParams[index], [field]: value };
    const paramsStr = newParams.map((p) => `${p.name}: ${p.type}`).join(", ");
    data.onParamsChange?.(paramsStr);
  };

  const handleAddParam = () => {
    const newParams = [...parsedParams, { name: "param", type: "String" }];
    const paramsStr = newParams.map((p) => `${p.name}: ${p.type}`).join(", ");
    data.onParamsChange?.(paramsStr);
  };

  const handleRemoveParam = (index: number) => {
    const newParams = parsedParams.filter((_, i) => i !== index);
    const paramsStr = newParams.map((p) => `${p.name}: ${p.type}`).join(", ");
    data.onParamsChange?.(paramsStr);
  };

  return (
    <div
      className="flow-node"
      data-node-type="fn"
      onDoubleClick={data.onDoubleClick}
      style={{
        background: "#EEF2FF",
        border: "2px solid #6366F1",
        borderRadius: "12px 12px 4px 4px",
        padding: "10px 18px",
        minWidth: 160,
        textAlign: "center",
        fontSize: 13,
        fontWeight: 600,
        color: "#3730A3",
        cursor: "pointer",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", justifyContent: "center", gap: 4 }}>
        <span style={{ fontSize: 14 }}>ƒ</span>
        <span>{name}</span>
        {params && <span style={{ fontWeight: 400 }}>({params})</span>}
        <button
          onClick={handleToggleParams}
          title="Edit params"
          style={{
            marginLeft: 6,
            padding: "1px 5px",
            fontSize: 11,
            border: "1px solid #6366F1",
            borderRadius: 3,
            background: showParams ? "#6366F1" : "#EEF2FF",
            color: showParams ? "#fff" : "#6366F1",
            cursor: "pointer",
          }}
        >
          {showParams ? "✕" : "+params"}
        </button>
      </div>

      {showParams && (
        <div
          style={{ marginTop: 8, textAlign: "left", fontSize: 12, fontWeight: 400 }}
          onClick={(e) => e.stopPropagation()}
        >
          {parsedParams.map((p, i) => (
            <div key={i} style={{ display: "flex", gap: 4, marginBottom: 3, alignItems: "center" }}>
              <input
                style={{ width: 60, border: "1px solid #d4d4d8", borderRadius: 3, padding: "1px 4px" }}
                value={p.name}
                onChange={(e) => handleParamChange(i, "name", e.target.value)}
                placeholder="name"
              />
              <span>:</span>
              <input
                style={{ width: 55, border: "1px solid #d4d4d8", borderRadius: 3, padding: "1px 4px" }}
                value={p.type}
                onChange={(e) => handleParamChange(i, "type", e.target.value)}
                placeholder="type"
              />
              <button
                onClick={() => handleRemoveParam(i)}
                style={{ border: "none", background: "none", color: "#ef4444", cursor: "pointer", fontSize: 12 }}
              >
                ✕
              </button>
            </div>
          ))}
          <button
            onClick={handleAddParam}
            style={{ marginTop: 4, padding: "1px 8px", fontSize: 11, border: "1px dashed #6366F1", borderRadius: 3, background: "transparent", color: "#6366F1", cursor: "pointer" }}
          >
            + add param
          </button>
        </div>
      )}

      <Handle
        type="source"
        position={Position.Bottom}
        id="flow-out"
        style={{ background: "#6366F1" }}
      />
    </div>
  );
}
