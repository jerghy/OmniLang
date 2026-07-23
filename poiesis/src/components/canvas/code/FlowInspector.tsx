// FlowInspector.tsx
// Right-side panel in CodeFlowCanvas that edits selected FN or Return node properties.
// Reusable across store action editing and component fn editing.

import { usePoiesisStore } from "../../../store";
import type { CodeblockElementData } from "../../../core/types/element";

const OMNI_TYPES = [
  "String", "i32", "i64", "f64", "f32",
  "bool", "Array<String>", "Array<i32>", "Array<f64>",
  "Map<String, String>",
];

const S = {
  panel: {
    width: 240,
    minWidth: 200,
    height: "100%",
    borderLeft: "1px solid var(--ol-border)",
    background: "var(--ol-gray-50)",
    overflow: "auto",
    fontSize: "var(--ol-font-size-12)",
  } as React.CSSProperties,

  header: {
    padding: "8px 12px",
    fontSize: "var(--ol-font-size-11)",
    fontWeight: "var(--ol-font-weight-600)",
    color: "var(--ol-text-secondary)",
    textTransform: "uppercase",
    letterSpacing: "0.05em",
    borderBottom: "1px solid var(--ol-border-subtle)",
  } as React.CSSProperties,

  field: {
    padding: "6px 12px",
    borderBottom: "1px solid var(--ol-border-subtle)",
  } as React.CSSProperties,

  label: {
    fontSize: "var(--ol-font-size-11)",
    color: "var(--ol-text-muted)",
    marginBottom: 3,
  } as React.CSSProperties,

  input: {
    width: "100%",
    border: "1px solid var(--ol-border)",
    borderRadius: "var(--ol-radius-sm)",
    padding: "3px 6px",
    fontSize: "var(--ol-font-size-12)",
    fontFamily: "var(--ol-font-family-mono)",
    background: "var(--ol-surface)",
    color: "var(--ol-text)",
    outline: "2px solid transparent",
    boxSizing: "border-box" as const,
  } as React.CSSProperties,

  select: {
    width: "100%",
    border: "1px solid var(--ol-border)",
    borderRadius: "var(--ol-radius-sm)",
    padding: "3px 4px",
    fontSize: "var(--ol-font-size-12)",
    fontFamily: "var(--ol-font-family-mono)",
    background: "var(--ol-surface)",
    color: "var(--ol-text)",
    outline: "2px solid transparent",
    cursor: "pointer",
    boxSizing: "border-box" as const,
  } as React.CSSProperties,

  btn: {
    padding: "2px 8px",
    fontSize: "var(--ol-font-size-11)",
    border: "1px solid var(--ol-border)",
    borderRadius: "var(--ol-radius-sm)",
    background: "var(--ol-surface)",
    color: "var(--ol-text-secondary)",
    cursor: "pointer",
  } as React.CSSProperties,

  removeBtn: {
    border: "none",
    background: "transparent",
    color: "var(--ol-text-muted)",
    cursor: "pointer",
    fontSize: 14,
    padding: "0 2px",
  } as React.CSSProperties,
};

interface Props {
  selectedNodeIds: string[];
}

export function FlowInspector({ selectedNodeIds }: Props) {
  const elements = usePoiesisStore((s) => s.elements);
  const setProp = usePoiesisStore((s) => s.setProp);
  const setElementSource = usePoiesisStore((s) => s.setElementSource);

  if (selectedNodeIds.length !== 1) {
    return (
      <div style={S.panel}>
        <div style={{ padding: 16, color: "var(--ol-text-muted)", textAlign: "center" }}>
          {selectedNodeIds.length === 0
            ? "Select a node to edit"
            : "Select one node at a time"}
        </div>
      </div>
    );
  }

  const el = elements.find((e) => e.id === selectedNodeIds[0] && e.nodeType === "codeblock") as CodeblockElementData | undefined;
  if (!el) {
    return (
      <div style={S.panel}>
        <div style={{ padding: 16, color: "var(--ol-text-muted)", textAlign: "center" }}>
          Node not found
        </div>
      </div>
    );
  }

  if (el.tag === "fn") {
    return <FnInspector el={el} setProp={setProp} />;
  }

  if (el.tag === "return") {
    return <ReturnInspector el={el} setElementSource={setElementSource} />;
  }

  return (
    <div style={S.panel}>
      <div style={{ padding: 16, color: "var(--ol-text-muted)", textAlign: "center" }}>
        No properties for "{el.tag}"
      </div>
    </div>
  );
}

// ────────────────────────────────────────────
// FN Node Inspector
// ────────────────────────────────────────────

interface FnInspectorProps {
  el: CodeblockElementData;
  setProp: (id: string, prop: string, value: unknown) => void;
}

function FnInspector({ el, setProp }: FnInspectorProps) {
  const fnName = (el.props?.name as string) || "";
  const paramsStr = (el.props?.params as string) || "";
  const parsedParams: Array<{ name: string; type: string }> = paramsStr
    ? paramsStr.split(",").map((p) => {
        const [pName, pType] = p.trim().split(":").map((s) => s.trim());
        return { name: pName || "", type: pType || "String" };
      })
    : [];

  const handleNameChange = (name: string) => {
    setProp(el.id, "name", name);
  };

  const handleParamChange = (index: number, field: "name" | "type", value: string) => {
    const newParams = [...parsedParams];
    newParams[index] = { ...newParams[index], [field]: value };
    const newParamsStr = newParams.map((p) => `${p.name}: ${p.type}`).join(", ");
    setProp(el.id, "params", newParamsStr);
  };

  const handleAddParam = () => {
    const newParams = [...parsedParams, { name: "param", type: "String" }];
    const newParamsStr = newParams.map((p) => `${p.name}: ${p.type}`).join(", ");
    setProp(el.id, "params", newParamsStr);
  };

  const handleRemoveParam = (index: number) => {
    const newParams = parsedParams.filter((_, i) => i !== index);
    const newParamsStr = newParams.map((p) => `${p.name}: ${p.type}`).join(", ");
    setProp(el.id, "params", newParamsStr);
  };

  return (
    <div style={S.panel}>
      <div style={S.header}>ƒ Function</div>

      {/* Name */}
      <div style={S.field}>
        <div style={S.label}>Name</div>
        <input style={S.input} value={fnName}
          onChange={(e) => handleNameChange(e.target.value)} />
      </div>

      {/* Parameters */}
      <div style={S.field}>
        <div style={{ ...S.label, display: "flex", justifyContent: "space-between" }}>
          <span>Parameters</span>
          <button type="button" style={S.btn} onClick={handleAddParam}>+ Add</button>
        </div>
        {parsedParams.map((p, i) => (
          <div key={p.name + "-" + p.type} style={{ display: "flex", gap: 4, marginBottom: 4, alignItems: "center" }}>
            <input
              style={{ ...S.input, width: 72 }}
              value={p.name}
              placeholder="name"
              onChange={(e) => handleParamChange(i, "name", e.target.value)}
            />
            <span style={{ color: "var(--ol-text-muted)", fontSize: 12 }}>:</span>
            <select
              style={{ ...S.select, flex: 1 }}
              value={p.type}
              onChange={(e) => handleParamChange(i, "type", e.target.value)}
            >
              {OMNI_TYPES.map((t) => <option key={t} value={t}>{t}</option>)}
              {!OMNI_TYPES.includes(p.type) && <option value={p.type}>{p.type}</option>}
            </select>
            <button type="button" style={S.removeBtn} onClick={() => handleRemoveParam(i)}>×</button>
          </div>
        ))}
        {parsedParams.length === 0 && (
          <div style={{ color: "var(--ol-text-muted)", fontSize: "var(--ol-font-size-11)" }}>
            No parameters
          </div>
        )}
      </div>
    </div>
  );
}

// ────────────────────────────────────────────
// Return Node Inspector
// ────────────────────────────────────────────

interface ReturnInspectorProps {
  el: CodeblockElementData;
  setElementSource: (id: string, source: string) => void;
}

function ReturnInspector({ el, setElementSource }: ReturnInspectorProps) {
  const source = el.source || "";

  return (
    <div style={S.panel}>
      <div style={S.header}>← Return</div>
      <div style={S.field}>
        <div style={S.label}>Expression</div>
        <input
          style={S.input}
          value={source}
          placeholder="e.g. items.len()"
          onChange={(e) => setElementSource(el.id, e.target.value)}
        />
      </div>
    </div>
  );
}
