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
    background: "#fff", border: "1px solid #e5e7eb", borderRadius: 6,
    padding: 8, marginBottom: 6,
  } as React.CSSProperties,
  typeBtn: (active: boolean): React.CSSProperties => ({
    background: active ? "#EEF2FF" : "transparent",
    border: active ? "1px solid #C7D2FE" : "1px solid #d1d5db",
    borderRadius: 4,
    color: active ? "#4F46E5" : "#6B7280",
    cursor: "pointer", fontSize: 11,
    padding: "2px 8px", marginRight: 4,
  }),
  input: {
    background: "#fff", color: "#374151", border: "1px solid #d1d5db",
    borderRadius: 4, padding: "4px 8px", fontSize: 12, width: "100%",
    marginTop: 4, marginBottom: 8, boxSizing: "border-box",
  } as React.CSSProperties,
  addBtn: {
    background: "none", border: "1px dashed #C7D2FE", borderRadius: 4,
    color: "#4F46E5", cursor: "pointer", fontSize: 11,
    padding: "4px 8px", width: "100%", marginTop: 4,
  } as React.CSSProperties,
};

export function StyleEntryEditor({ entry, onChange, onClose }: Props) {
  const isVariable = entry.type === "variable";

  const handleTypeChange = (newType: StyleEntry["type"]) => {
    // No-op guard: clicking the already-active type should not create undo entries
    if (newType === entry.type) return;

    const patch: Partial<StyleEntry> = { type: newType };

    // Switching TO variable: auto-add -- prefix and truncate properties to 1
    if (newType === "variable") {
      // --- 2.1: Variable type auto-add -- prefix ---
      if (entry.type === "class") {
        if (entry.name.startsWith("_v-")) {
          patch.name = "--" + entry.name.slice(3);
        } else if (!entry.name.startsWith("--") && entry.name !== "") {
          patch.name = "--" + entry.name;
        }
        // If name already starts with -- or is empty, keep as-is
      } else if (entry.type === "inline") {
        if (entry.name === "") {
          patch.name = "--unnamed";
        } else if (!entry.name.startsWith("--")) {
          patch.name = "--" + entry.name;
        }
        // If name already starts with --, keep as-is
      }

      // --- 2.2: Variable type limit to 1 property ---
      patch.properties = [
        { name: "value", value: entry.properties[0]?.value ?? "" },
      ];
    }

    // Switching FROM variable: strip -- prefix (properties stay as-is, undo system handles restoration)
    if (entry.type === "variable" && newType !== "variable") {
      patch.name = entry.name.startsWith("--") ? entry.name.slice(2) : entry.name;
    }

    onChange(patch);
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
            <span style={{ fontSize: 11, color: "#6B7280", whiteSpace: "nowrap" }}>
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
