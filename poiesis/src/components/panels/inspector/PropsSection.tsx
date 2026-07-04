// components/panels/inspector/PropsSection.tsx
// Editable form for element properties (id, class binding, label, etc.).

import { useState, useCallback } from "react";
import { usePoiesisStore } from "../../../store";
import type { ElementData } from "../../../core/types/element";

interface PropsSectionProps {
  element: ElementData;
}

export function PropsSection({ element }: PropsSectionProps) {
  const setProp = usePoiesisStore((s) => s.setProp);

  // Local editing state to avoid store writes on every keystroke
  const [localProps, setLocalProps] = useState<Record<string, string>>(() => {
    const initial: Record<string, string> = {};
    const known = ["id", "label", "class", "name", "type", "placeholder", "value", "src", "href"];
    for (const key of known) {
      const val = element.props[key];
      if (val !== undefined) {
        initial[key] = String(val);
      }
    }
    return initial;
  });

  const handleChange = useCallback(
    (propName: string, value: string) => {
      setLocalProps((prev) => ({ ...prev, [propName]: value }));
      setProp(element.id, propName, value);
    },
    [element.id, setProp]
  );

  // Compute which props are relevant based on tag
  const relevantProps = getRelevantProps(element.tag);

  return (
    <div style={{ marginBottom: 12 }}>
      <div
        style={{
          fontSize: 11,
          fontWeight: 600,
          color: "#6b7280",
          textTransform: "uppercase",
          marginBottom: 6,
        }}
      >
        属性
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
        {/* Always show id */}
        <PropRow
          label="ID"
          value={localProps["id"] ?? element.id}
          onChange={(v) => handleChange("id", v)}
        />

        {relevantProps.map((prop) => (
          <PropRow
            key={prop}
            label={prop}
            value={localProps[prop] ?? ""}
            onChange={(v) => handleChange(prop, v)}
          />
        ))}
      </div>
    </div>
  );
}

function PropRow({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
}) {
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
      <label
        style={{
          fontSize: 11,
          color: "#64748b",
          minWidth: 50,
          flexShrink: 0,
        }}
      >
        {label}
      </label>
      <input
        type="text"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        style={{
          flex: 1,
          padding: "3px 6px",
          fontSize: 11,
          border: "1px solid #e2e8f0",
          borderRadius: 4,
          outline: "none",
        }}
        onFocus={(e) => {
          e.target.style.borderColor = "#6366F1";
        }}
        onBlur={(e) => {
          e.target.style.borderColor = "#e2e8f0";
        }}
      />
    </div>
  );
}

function getRelevantProps(tag: string): string[] {
  const map: Record<string, string[]> = {
    input: ["type", "placeholder", "value", "name"],
    button: ["label", "type", "name"],
    img: ["src", "alt"],
    a: ["href", "label", "target"],
    textarea: ["placeholder", "value", "name", "rows"],
    select: ["name"],
    label: ["label"],
    h1: ["label"],
    h2: ["label"],
    h3: ["label"],
    p: ["label"],
  };
  return map[tag] ?? ["label"];
}
