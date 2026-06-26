/** VariablePanel — visual variable manager for the visual scripting canvas.
 *  Shows all signals and computed values.  Drag a variable to the canvas to
 *  auto-create a GetVariable node.  Click to highlight its node on the canvas. */

import { useState, useCallback } from "react";
import { useEditorStore } from "../../store/editor";
import { getNodeConfig } from "./shared/nodeRegistry";

export interface VariablePanelProps {
  /** Call when user drags a variable to create a GetVariable node on canvas */
  onDragToCanvas?: (variableName: string, variableType: string) => void;
  /** Call to create a new signal */
  onCreateSignal?: () => void;
  /** Call to create a new computed */
  onCreateComputed?: () => void;
}

export function VariablePanel({
  onDragToCanvas,
  onCreateSignal,
  onCreateComputed,
}: VariablePanelProps) {
  const elements = useEditorStore((s) => s.elements);
  const [filter, setFilter] = useState("");

  // Extract signals and computeds from elements
  const signals = elements.filter(
    (e) => e.nodeType === "codeblock" && (e.tag === "signal" || e.tag === "computed"),
  );

  const filtered = filter
    ? signals.filter((s) => {
        const name = String(s.props.name || "").toLowerCase();
        return name.includes(filter.toLowerCase());
      })
    : signals;

  const handleDragStart = useCallback(
    (e: React.DragEvent, vName: string, vType: string) => {
      e.dataTransfer.setData(
        "application/omni-variable",
        JSON.stringify({ name: vName, type: vType }),
      );
      e.dataTransfer.effectAllowed = "copy";
    },
    [],
  );

  return (
    <div
      style={{
        fontFamily: "system-ui, sans-serif",
        fontSize: 12,
        height: "100%",
        display: "flex",
        flexDirection: "column",
      }}
      data-panel="variables"
    >
      {/* Header */}
      <div
        style={{
          padding: "8px 12px",
          borderBottom: "1px solid #E5E7EB",
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
        }}
      >
        <span style={{ fontWeight: 600, color: "#374151" }}>📌 Variables</span>
        <div style={{ display: "flex", gap: 4 }}>
          <button
            onClick={onCreateSignal}
            style={{
              padding: "2px 8px",
              fontSize: 11,
              background: "#FEF3C7",
              border: "1px solid #D97706",
              borderRadius: 4,
              cursor: "pointer",
              color: "#92400E",
            }}
            title="Add Signal"
          >
            +Signal
          </button>
          <button
            onClick={onCreateComputed}
            style={{
              padding: "2px 8px",
              fontSize: 11,
              background: "#DBEAFE",
              border: "1px solid #2563EB",
              borderRadius: 4,
              cursor: "pointer",
              color: "#1E40AF",
            }}
            title="Add Computed"
          >
            +Computed
          </button>
        </div>
      </div>

      {/* Filter */}
      <div style={{ padding: "6px 12px" }}>
        <input
          type="text"
          placeholder="🔍 Filter variables..."
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          style={{
            width: "100%",
            border: "1px solid #E5E7EB",
            borderRadius: 6,
            padding: "4px 8px",
            fontSize: 11,
            outline: "none",
            boxSizing: "border-box",
          }}
        />
      </div>

      {/* Variable list */}
      <div style={{ flex: 1, overflowY: "auto", padding: "4px 0" }}>
        {filtered.length === 0 && (
          <div
            style={{
              padding: "16px",
              textAlign: "center",
              color: "#9CA3AF",
              fontSize: 11,
            }}
          >
            {signals.length === 0
              ? "No variables yet. Click +Signal or +Computed to create one."
              : "No variables match the filter."}
          </div>
        )}

        {filtered.map((v) => {
          const config = getNodeConfig(v.tag);
          const name = String(v.props.name || "");
          const typeAnn = (v.props.typeAnnotation as string) || v.ports?.find((p) => p.name === "value")?.type || "any";

          return (
            <div
              key={v.id}
              draggable
              onDragStart={(e) => handleDragStart(e, name, typeAnn)}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 8,
                padding: "6px 12px",
                cursor: "grab",
                userSelect: "none",
                transition: "background 0.08s",
                borderLeft: `3px solid ${config.color.border}`,
              }}
              onMouseEnter={(e) => (e.currentTarget.style.background = "#F9FAFB")}
              onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}
              title={`Drag to canvas to create GetVariable node\nType: ${typeAnn}`}
            >
              <span style={{ fontSize: 14 }}>{config.icon}</span>
              <div style={{ minWidth: 0 }}>
                <div style={{ fontWeight: 600, color: "#1F2937", fontSize: 12 }}>
                  {name}
                </div>
                <div style={{ fontSize: 10, color: "#9CA3AF" }}>
                  {v.tag} : {typeAnn}
                </div>
              </div>
            </div>
          );
        })}
      </div>

      {/* Footer */}
      <div
        style={{
          padding: "6px 12px",
          borderTop: "1px solid #E5E7EB",
          color: "#D1D5DB",
          fontSize: 10,
        }}
      >
        {signals.length} variable{signals.length !== 1 ? "s" : ""} —
        drag to canvas to create Get node
      </div>
    </div>
  );
}
