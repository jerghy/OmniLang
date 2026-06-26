import React, { useState, useEffect, useRef } from "react";
import type { StyleProperty } from "../constants/style-properties";
import {
  isColorProperty,
  isCustomProperty,
  getUnitGroup,
  getDropdownOptions,
} from "../constants/style-properties";

interface Props {
  property: StyleProperty;
  onChange: (prop: StyleProperty) => void;
  onDelete: () => void;
}

// Categorized property name list for the dropdown
const PROPERTY_CATEGORIES: { label: string; props: string[] }[] = [
  {
    label: "Layout", props: [
      "width", "height", "min-width", "max-width", "min-height", "max-height",
      "margin", "margin-top", "margin-right", "margin-bottom", "margin-left",
      "padding", "padding-top", "padding-right", "padding-bottom", "padding-left",
      "display", "gap", "overflow", "overflow-x", "overflow-y",
    ],
  },
  {
    label: "Typography", props: [
      "font-size", "font-weight", "font-family", "color",
      "text-align", "line-height", "letter-spacing",
      "text-decoration", "text-transform", "white-space", "word-break",
    ],
  },
  {
    label: "Background & Border", props: [
      "background-color", "background-image", "border", "border-color",
      "border-radius", "box-shadow", "opacity",
    ],
  },
  {
    label: "Position", props: [
      "position", "top", "right", "bottom", "left", "z-index",
    ],
  },
  {
    label: "Flex & Grid", props: [
      "flex-direction", "flex-wrap", "flex-grow", "flex-shrink", "flex-basis",
      "justify-content", "align-items", "align-self",
      "grid-template-columns", "grid-template-rows", "grid-column", "grid-row",
    ],
  },
  {
    label: "Transform & Transition", props: [
      "transform", "transition", "transition-duration", "transition-property",
      "transition-timing-function", "animation", "animation-name",
      "animation-duration", "animation-iteration-count",
      "animation-direction", "animation-fill-mode", "animation-timing-function",
      "cursor",
    ],
  },
];

function parseValueAndUnit(raw: string): { num: string; unit: string } {
  const match = raw.match(/^(-?[\d.]+)\s*(.*)$/);
  if (match) return { num: match[1], unit: match[2] || "" };
  return { num: raw, unit: "" };
}

const styles = {
  row: { display: "flex", gap: 6, alignItems: "center", marginBottom: 4 } as React.CSSProperties,
  input: {
    background: "#fff", color: "#374151", border: "1px solid #d1d5db",
    borderRadius: 4, padding: "4px 8px", fontSize: 12, flex: 1,
  } as React.CSSProperties,
  select: { background: "#fff", color: "#374151", border: "1px solid #d1d5db",
    borderRadius: 4, padding: "4px 8px", fontSize: 12, width: 110, flex: "none" } as React.CSSProperties,
  deleteBtn: {
    background: "none", border: "none", color: "#DC2626", cursor: "pointer",
    fontSize: 14, padding: "0 4px",
  } as React.CSSProperties,
};

const PropertyNameSearchSelect: React.FC<{
  value: string;
  onChange: (name: string) => void;
  includeCustom?: boolean;
}> = ({ value, onChange, includeCustom }) => {
  const [isOpen, setIsOpen] = useState(false);
  const [searchText, setSearchText] = useState("");
  const [hoveredValue, setHoveredValue] = useState<string | null>(null);
  const ref = useRef<HTMLDivElement>(null);

  // Close on outside click and Escape key
  useEffect(() => {
    if (!isOpen) return;
    const close = () => { setIsOpen(false); setSearchText(""); };
    const mouseHandler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) close();
    };
    const keyHandler = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    document.addEventListener("mousedown", mouseHandler);
    document.addEventListener("keydown", keyHandler);
    return () => {
      document.removeEventListener("mousedown", mouseHandler);
      document.removeEventListener("keydown", keyHandler);
    };
  }, [isOpen]);

  const filtered = PROPERTY_CATEGORIES
    .map(cat => ({
      ...cat,
      props: cat.props.filter(p =>
        p.toLowerCase().includes(searchText.toLowerCase())
      )
    }))
    .filter(cat => cat.props.length > 0);

  // Custom property support: if includeCustom and value starts with --,
  // show it as an option when it matches the search
  const showCustom = includeCustom && value.startsWith("--") &&
    value.toLowerCase().includes(searchText.toLowerCase());

  return (
    <div ref={ref} style={{ position: "relative" }}>
      <div
        onClick={() => setIsOpen(!isOpen)}
        style={{ ...styles.select, cursor: "pointer" }}
      >
        {value || "select property..."}
      </div>
      {isOpen && (
        <div style={{
          position: "absolute", zIndex: 10,
          background: "#fff", border: "1px solid #d1d5db",
          borderRadius: 4, marginTop: 2,
          maxHeight: 250, overflow: "auto",
          width: "100%", boxShadow: "0 4px 12px rgba(0,0,0,0.1)",
        }}>
          <input
            autoFocus
            placeholder="🔍 搜索属性..."
            value={searchText}
            onChange={e => setSearchText(e.target.value)}
            style={{ width: "100%", boxSizing: "border-box", padding: "6px 8px", border: "none", borderBottom: "1px solid #e5e7eb", outline: "none", fontSize: 12 }}
          />
          {filtered.length === 0 && !showCustom && searchText && (
            <div style={{ padding: "8px", color: "#9CA3AF", fontSize: 11 }}>无匹配属性</div>
          )}
          {filtered.map(cat => (
            <div key={cat.label}>
              <div style={{ padding: "4px 8px", fontSize: 10, color: "#6B7280", fontWeight: 600 }}>{cat.label}</div>
              {cat.props.map(p => (
                <div key={p}
                  onClick={() => { onChange(p); setIsOpen(false); setSearchText(""); }}
                  onMouseEnter={() => setHoveredValue(p)}
                  onMouseLeave={() => setHoveredValue(null)}
                  style={{
                    padding: "4px 8px", cursor: "pointer", fontSize: 12,
                    background: p === value ? "#EEF2FF" : hoveredValue === p ? "#F3F4F6" : "transparent",
                    color: p === value ? "#4F46E5" : "#374151",
                  }}
                >
                  {p}
                </div>
              ))}
            </div>
          ))}
          {showCustom && (
            <div key="custom-section">
              <div style={{ padding: "4px 8px", fontSize: 10, color: "#6B7280", fontWeight: 600 }}>Custom</div>
              <div
                onClick={() => { onChange(value); setIsOpen(false); setSearchText(""); }}
                onMouseEnter={() => setHoveredValue(value)}
                onMouseLeave={() => setHoveredValue(null)}
                style={{
                  padding: "4px 8px", cursor: "pointer", fontSize: 12,
                  background: hoveredValue === value ? "#F3F4F6" : "#EEF2FF",
                  color: "#4F46E5",
                }}
              >
                {value}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
};

const DeleteButton: React.FC<{ onClick: () => void }> = ({ onClick }) => (
  <button style={styles.deleteBtn} onClick={onClick} title="Delete property">×</button>
);

export function StylePropertyEditor({ property, onChange, onDelete }: Props) {
  const unitGroup = getUnitGroup(property.name);
  const dropdownOpts = getDropdownOptions(property.name);
  const color = isColorProperty(property.name);
  const isCustom = isCustomProperty(property.name);

  // Color properties — color picker + text
  if (color) {
    return (
      <div style={styles.row}>
        <PropertyNameSearchSelect
          value={property.name}
          onChange={(name) => onChange({ ...property, name })}
          includeCustom={isCustom}
        />
        <input
          type="color"
          value={property.value.startsWith("#") ? property.value : "#000000"}
          onChange={(e) => onChange({ ...property, value: e.target.value })}
          style={{ width: 28, height: 24, border: "none", cursor: "pointer", flex: "none" }}
        />
        <input
          type="text"
          value={property.value}
          onChange={(e) => onChange({ ...property, value: e.target.value })}
          style={styles.input}
        />
        <DeleteButton onClick={onDelete} />
      </div>
    );
  }

  // Dropdown properties (has options list)
  if (dropdownOpts !== null) {
    // If dropdownOpts is a string array, show dropdown. If null, show text input.
    if (Array.isArray(dropdownOpts)) {
      return (
        <div style={styles.row}>
          <PropertyNameSearchSelect
            value={property.name}
            onChange={(name) => onChange({ ...property, name })}
            includeCustom={isCustom}
          />
          <select
            style={styles.input}
            value={property.value}
            onChange={(e) => onChange({ ...property, value: e.target.value })}
          >
            {dropdownOpts.map((opt) => (
              <option key={opt} value={opt}>{opt}</option>
            ))}
          </select>
          <DeleteButton onClick={onDelete} />
        </div>
      );
    }
    // dropdownOpts is null — free text for properties like transform, font-family
    return (
      <div style={styles.row}>
        <PropertyNameSearchSelect
          value={property.name}
          onChange={(name) => onChange({ ...property, name })}
          includeCustom={isCustom}
        />
        <input
          type="text"
          value={property.value}
          onChange={(e) => onChange({ ...property, value: e.target.value })}
          style={styles.input}
          placeholder="e.g. rotate(45deg)"
        />
        <DeleteButton onClick={onDelete} />
      </div>
    );
  }

  // Opacity — slider + number
  if (property.name === "opacity") {
    return (
      <div style={styles.row}>
        <PropertyNameSearchSelect
          value={property.name}
          onChange={(name) => onChange({ ...property, name })}
          includeCustom={isCustom}
        />
        <input
          type="range"
          min="0" max="1" step="0.01"
          value={isNaN(parseFloat(property.value)) ? 1 : parseFloat(property.value)}
          onChange={(e) => onChange({ ...property, value: e.target.value })}
          style={{ flex: 1 }}
        />
        <input
          type="number"
          min="0" max="1" step="0.01"
          value={property.value}
          onChange={(e) => onChange({ ...property, value: e.target.value })}
          style={{ ...styles.input, width: 56, flex: "none" }}
        />
        <DeleteButton onClick={onDelete} />
      </div>
    );
  }

  // Numeric + unit properties
  if (unitGroup) {
    const { num, unit } = parseValueAndUnit(property.value);
    const effectiveUnit = unit || unitGroup.default;
    const units = [...unitGroup.common, ...(unitGroup.advanced || [])];

    return (
      <div style={styles.row}>
        <PropertyNameSearchSelect
          value={property.name}
          onChange={(name) => onChange({ ...property, name })}
          includeCustom={isCustom}
        />
        <input
          type="text"
          value={num}
          onChange={(e) => {
            const newVal = e.target.value + effectiveUnit;
            onChange({ ...property, value: newVal });
          }}
          style={{ ...styles.input, width: 60, flex: "none" }}
          placeholder={unitGroup.default}
        />
        <select
          value={effectiveUnit}
          onChange={(e) => {
            const newUnit = e.target.value;
            const newVal = newUnit === "" ? num : num + newUnit;
            onChange({ ...property, value: newVal });
          }}
          style={{ ...styles.select, width: 64 }}
        >
          {units.map((u) => (
            <option key={u} value={u}>{u || "(none)"}</option>
          ))}
        </select>
        <DeleteButton onClick={onDelete} />
      </div>
    );
  }

  // Fallback: generic text input
  return (
    <div style={styles.row}>
      <PropertyNameSearchSelect
        value={property.name}
        onChange={(name) => onChange({ ...property, name })}
        includeCustom={isCustom}
      />
      <input
        type="text"
        value={property.value}
        onChange={(e) => onChange({ ...property, value: e.target.value })}
        style={styles.input}
        placeholder="e.g. 16px"
      />
      <DeleteButton onClick={onDelete} />
    </div>
  );
}
