// ui/panels/ClassCard.tsx
// Expandable card for editing a single StyleClass.

import React, { useState } from "react";
import type { StyleClass, StyleProperty } from "../../core/types/style";
import { ClassPreview } from "./ClassPreview";
import { StylePropertyEditor } from "./StylePropertyEditor";
import { uid } from "../../core/utils/id";

interface SignalOption {
  name: string;
  label: string;
}

interface Props {
  styleClass: StyleClass;
  onChange: (updated: StyleClass) => void;
  onDelete: () => void;
  availableSignals?: SignalOption[];
  availableCssVars?: string[];
}

export function ClassCard({ styleClass, onChange, onDelete, availableSignals, availableCssVars }: Props) {
  const [expanded, setExpanded] = useState(false);
  const [previewTag, setPreviewTag] = useState("div");

  const handleNameChange = (name: string) => {
    onChange({ ...styleClass, name });
  };

  const handlePropertyChange = (index: number, prop: StyleProperty) => {
    const newProps = [...styleClass.properties];
    newProps[index] = prop;
    onChange({ ...styleClass, properties: newProps });
  };

  const handlePropertyDelete = (index: number) => {
    onChange({
      ...styleClass,
      properties: styleClass.properties.filter((_, i) => i !== index),
    });
  };

  const handleAddProperty = () => {
    onChange({
      ...styleClass,
      properties: [
        ...styleClass.properties,
        { id: uid("p"), name: "padding", segments: [{ type: "text", value: "" }] },
      ],
    });
  };

  if (!expanded) {
    return (
      <div
        onClick={() => setExpanded(true)}
        style={{
          display: "flex", alignItems: "center", gap: 8,
          padding: "6px 10px", cursor: "pointer", borderRadius: 4,
          fontSize: 13, color: "#374151",
        }}
        onMouseEnter={(e) => { e.currentTarget.style.background = "#F3F4F6"; }}
        onMouseLeave={(e) => { e.currentTarget.style.background = "transparent"; }}
      >
        <span style={{ fontSize: 10, color: "#9CA3AF" }}>▶</span>
        <span style={{ fontWeight: 500 }}>.{styleClass.name || "unnamed"}</span>
      </div>
    );
  }

  return (
    <div style={{
      background: "#fff", border: "1px solid #e5e7eb", borderRadius: 6,
      padding: 10, marginBottom: 6,
    }}>
      {/* Header */}
      <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8 }}>
        <span
          onClick={() => setExpanded(false)}
          style={{ fontSize: 10, color: "#9CA3AF", cursor: "pointer" }}
        >
          ▼
        </span>
        <span style={{ fontSize: 11, color: "#6B7280" }}>.</span>
        <input
          type="text"
          value={styleClass.name}
          onChange={(e) => handleNameChange(e.target.value)}
          placeholder="class-name"
          style={{
            border: "1px solid #d1d5db", borderRadius: 4, padding: "2px 6px",
            fontSize: 12, flex: 1, maxWidth: 200,
          }}
        />
        <button
          onClick={(e) => { e.stopPropagation(); onDelete(); }}
          style={{
            marginLeft: "auto", background: "none", border: "none",
            color: "#DC2626", cursor: "pointer", fontSize: 14, padding: "0 4px",
          }}
        >
          ×
        </button>
      </div>

      {/* Body: preview left + properties right */}
      <div style={{ display: "flex", gap: 12 }}>
        <ClassPreview
          styleClass={styleClass}
          previewTag={previewTag}
          onTagChange={setPreviewTag}
        />
        <div style={{ flex: 1 }}>
          {styleClass.properties.map((prop, i) => (
            <StylePropertyEditor
              key={prop.id}
              property={prop}
              onChange={(p) => handlePropertyChange(i, p)}
              onDelete={() => handlePropertyDelete(i)}
              availableSignals={availableSignals}
              availableCssVars={availableCssVars}
            />
          ))}
          <button
            onClick={handleAddProperty}
            style={{
              background: "none", border: "1px dashed #C7D2FE", borderRadius: 4,
              color: "#4F46E5", cursor: "pointer", fontSize: 11,
              padding: "4px 8px", width: "100%", marginTop: 4,
            }}
          >
            + 添加属性
          </button>
        </div>
      </div>
    </div>
  );
}
