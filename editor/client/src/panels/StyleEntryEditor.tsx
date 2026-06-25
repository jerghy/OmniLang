import React from "react";
import type { StyleEntry, StyleProperty } from "../constants/style-properties";
import { StylePropertyEditor } from "./StylePropertyEditor";

interface Props {
  entry: StyleEntry;
  elementId: string;
  onChange: (patch: Partial<StyleEntry>) => void;
  onClose: () => void;
}

const styles = {
  container: {
    background: "#181825", border: "1px solid #313244", borderRadius: 6,
    padding: 8, marginBottom: 6,
  } as React.CSSProperties,
  typeBtn: (active: boolean): React.CSSProperties => ({
    background: active ? "#45475a" : "transparent",
    border: "1px solid #585b70", borderRadius: 4,
    color: "#cdd6f4", cursor: "pointer", fontSize: 11,
    padding: "2px 8px", marginRight: 4,
  }),
  input: {
    background: "#1e1e2e", color: "#cdd6f4", border: "1px solid #45475a",
    borderRadius: 4, padding: "4px 8px", fontSize: 12, width: "100%",
    marginTop: 4, marginBottom: 8,
  } as React.CSSProperties,
  addBtn: {
    background: "none", border: "1px dashed #585b70", borderRadius: 4,
    color: "#a6adc8", cursor: "pointer", fontSize: 11,
    padding: "4px 8px", width: "100%", marginTop: 4,
  } as React.CSSProperties,
};

export function StyleEntryEditor({ entry, onChange, onClose }: Props) {
  const isVariable = entry.type === "variable";

  const handleTypeChange = (newType: StyleEntry["type"]) => {
    onChange({ type: newType });
  };

  const handlePropertyChange = (index: number, prop: StyleProperty) => {
    const newProps = [...entry.properties];
    newProps[index] = prop;
    onChange({ properties: newProps });
  };

  const handlePropertyDelete = (index: number) => {
    const newProps = entry.properties.filter((_, i) => i !== index);
    onChange({ properties: newProps });
  };

  const handleAddProperty = () => {
    onChange({
      properties: [...entry.properties, { name: "padding", value: "" }],
    });
  };

  return (
    <div style={styles.container}>
      {/* Type toggle */}
      <div style={{ display: "flex", marginBottom: 8 }}>
        <button style={styles.typeBtn(entry.type === "class")} onClick={() => handleTypeChange("class")}>
          class
        </button>
        <button style={styles.typeBtn(entry.type === "inline")} onClick={() => handleTypeChange("inline")}>
          inline
        </button>
        <button style={styles.typeBtn(entry.type === "variable")} onClick={() => handleTypeChange("variable")}>
          variable
        </button>
      </div>

      {/* Name field (class and variable) */}
      {entry.type !== "inline" && (
        <input
          type="text"
          value={entry.name}
          onChange={(e) => onChange({ name: e.target.value })}
          style={styles.input}
          placeholder={isVariable ? "--variable-name" : "class-name"}
        />
      )}

      {/* Properties */}
      {isVariable ? (
        /* Variable mode: single value */
        entry.properties.length > 0 && (
          <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
            <span style={{ fontSize: 11, color: "#a6adc8", whiteSpace: "nowrap" }}>
              {entry.name || "--var"}:
            </span>
            <input
              type="text"
              value={entry.properties[0]?.value ?? ""}
              onChange={(e) =>
                onChange({ properties: [{ name: "value", value: e.target.value }] })
              }
              style={{ ...styles.input, marginTop: 0, marginBottom: 0 }}
              placeholder="#333"
            />
          </div>
        )
      ) : (
        /* Class/inline mode: property list */
        <>
          {entry.properties.map((prop, i) => (
            <StylePropertyEditor
              key={i}
              property={prop}
              onChange={(p) => handlePropertyChange(i, p)}
              onDelete={() => handlePropertyDelete(i)}
            />
          ))}
          <button style={styles.addBtn} onClick={handleAddProperty}>
            + Add Property
          </button>
        </>
      )}
    </div>
  );
}
