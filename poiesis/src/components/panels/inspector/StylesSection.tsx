// components/panels/inspector/StylesSection.tsx
// Inline style editor with simple key-value pair rows.

import { useState, useCallback, useRef, useEffect } from "react";
import { usePoiesisStore } from "../../../store";
import type { ElementData } from "../../../core/types/element";
import type { StyleProperty } from "../../../core/types/style";

interface StylesSectionProps {
  element: ElementData;
}

const COMMON_STYLES = [
  "width",
  "height",
  "padding",
  "margin",
  "background",
  "color",
  "fontSize",
  "fontWeight",
  "border",
  "borderRadius",
  "display",
  "flex",
  "gap",
  "alignItems",
  "justifyContent",
  "opacity",
];

function buildLocalStyles(element: ElementData): Record<string, string> {
  const existingProperties = "style" in element ? (element.style?.inlineProperties ?? []) : [];
  const initial: Record<string, string> = {};
  for (const p of existingProperties) {
    if (
      p.segments.length > 0 &&
      p.segments[0].type === "text"
    ) {
      initial[p.name] = p.segments[0].value;
    }
  }
  return initial;
}

export function StylesSection({ element }: StylesSectionProps) {
  const setElementInlineStyles = usePoiesisStore(
    (s) => s.setElementInlineStyles
  );

  const [localStyles, setLocalStyles] = useState<
    Record<string, string>
  >(() => buildLocalStyles(element));

  const prevElementIdRef = useRef(element.id);
  if (element.id !== prevElementIdRef.current) {
    prevElementIdRef.current = element.id;
    setLocalStyles(buildLocalStyles(element));
  }

  const commitStyles = useCallback(
    (updated: Record<string, string>) => {
      const properties: StyleProperty[] = Object.entries(updated)
        .filter(([, val]) => val.trim().length > 0)
        .map(([name, value]) => ({
          id: `style_${name}`,
          name,
          segments: [{ type: "text" as const, value }],
        }));
      setElementInlineStyles(element.id, properties);
    },
    [element.id, setElementInlineStyles]
  );

  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const latestStylesRef = useRef(localStyles);

  const debouncedCommitStyles = useCallback(() => {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => {
      commitStyles(latestStylesRef.current);
    }, 200);
  }, [commitStyles]);

  useEffect(() => {
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, []);

  const [addingStyle, setAddingStyle] = useState<string>("");

  const handleChange = useCallback(
    (styleName: string, value: string) => {
      setLocalStyles((prev) => {
        const updated = { ...prev, [styleName]: value };
        latestStylesRef.current = updated;
        return updated;
      });
      debouncedCommitStyles();
    },
    [debouncedCommitStyles]
  );

  const handleRemove = useCallback(
    (styleName: string) => {
      setLocalStyles((prev) => {
        const { [styleName]: _, ...rest } = prev;
        latestStylesRef.current = rest;
        commitStyles(rest);
        return rest;
      });
    },
    [commitStyles]
  );

  const handleAddStyle = useCallback(() => {
    if (!addingStyle.trim()) return;
    handleChange(addingStyle.trim(), "");
    setAddingStyle("");
  }, [addingStyle, handleChange]);

  const activeStyleNames = new Set(Object.keys(localStyles));
  const availableStyles = COMMON_STYLES.filter(
    (s) => !activeStyleNames.has(s)
  );

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
        内联样式
      </div>

      {/* Existing styles */}
      <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
        {Object.entries(localStyles).map(([name, value]) => (
          <div
            key={name}
            style={{ display: "flex", alignItems: "center", gap: 4 }}
          >
            <label
              style={{
                fontSize: 11,
                color: "#64748b",
                minWidth: 70,
                flexShrink: 0,
              }}
            >
              {name}
            </label>
            <input
              type="text"
              value={value}
              onChange={(e) => handleChange(name, e.target.value)}
              placeholder="value"
              style={{
                flex: 1,
                padding: "3px 6px",
                fontSize: 11,
                border: "1px solid #e2e8f0",
                borderRadius: 4,
                outline: "none",
                fontFamily: "monospace",
              }}
              onFocus={(e) => {
                e.target.style.borderColor = "#6366F1";
              }}
              onBlur={(e) => {
                e.target.style.borderColor = "#e2e8f0";
              }}
            />
            <button
              onClick={() => handleRemove(name)}
              title="移除样式"
              style={{
                border: "none",
                background: "none",
                cursor: "pointer",
                color: "#ef4444",
                fontSize: 12,
                padding: "2px 4px",
              }}
            >
              ×
            </button>
          </div>
        ))}
      </div>

      {/* Add style controls */}
      <div style={{ marginTop: 6, display: "flex", gap: 4 }}>
        <select
          value={addingStyle}
          onChange={(e) => setAddingStyle(e.target.value)}
          style={{
            flex: 1,
            padding: "3px 6px",
            fontSize: 11,
            border: "1px solid #e2e8f0",
            borderRadius: 4,
            outline: "none",
            background: "#fff",
            color: availableStyles.length === 0 ? "#cbd5e1" : "#374151",
          }}
        >
          <option value="">
            {availableStyles.length === 0
              ? "无可添加样式"
              : "选择样式..."}
          </option>
          {availableStyles.map((s) => (
            <option key={s} value={s}>
              {s}
            </option>
          ))}
        </select>
        <button
          onClick={handleAddStyle}
          disabled={!addingStyle.trim()}
          style={{
            padding: "3px 10px",
            fontSize: 11,
            border: "none",
            borderRadius: 4,
            background: addingStyle.trim() ? "#6366F1" : "#e2e8f0",
            color: addingStyle.trim() ? "#fff" : "#94a3b8",
            cursor: addingStyle.trim() ? "pointer" : "default",
            fontWeight: 600,
          }}
        >
          +
        </button>
      </div>

      {/* Quick-add common styles as chips */}
      {availableStyles.length > 0 && (
        <div
          style={{
            marginTop: 6,
            display: "flex",
            flexWrap: "wrap",
            gap: 3,
          }}
        >
          {availableStyles.slice(0, 8).map((s) => (
            <button
              key={s}
              onClick={() => handleChange(s, "")}
              style={{
                padding: "1px 6px",
                fontSize: 9,
                border: "1px solid #e2e8f0",
                borderRadius: 3,
                background: "#f8fafc",
                color: "#64748b",
                cursor: "pointer",
              }}
            >
              +{s}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
