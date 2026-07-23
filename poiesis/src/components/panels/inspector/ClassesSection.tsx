// components/panels/inspector/ClassesSection.tsx
// Class binding section: shows bound class pills with remove, search dropdown to add.

import { useState, useCallback } from "react";
import { usePoiesisStore } from "../../../store";
import type { ElementData } from "../../../core/types/element";

interface ClassesSectionProps {
  element: ElementData;
}

export function ClassesSection({ element }: ClassesSectionProps) {
  const activeDesign = usePoiesisStore((s) => s.activeDesign);
  const setElementClassRefs = usePoiesisStore((s) => s.setElementClassRefs);

  const currentRefs: string[] =
    "style" in element ? (element.style?.classRefs ?? []) : [];

  const [search, setSearch] = useState("");
  const [showDropdown, setShowDropdown] = useState(false);

  const filteredClasses = search.trim()
    ? activeDesign.classes.filter((c) =>
        c.name.toLowerCase().includes(search.toLowerCase()),
      )
    : activeDesign.classes;

  const handleAddClass = useCallback(
    (className: string) => {
      const prefixed = `${activeDesign.name}.${className}`;
      if (currentRefs.includes(prefixed)) return;
      setElementClassRefs(element.id, [...currentRefs, prefixed]);
      setSearch("");
      setShowDropdown(false);
    },
    [element.id, currentRefs, setElementClassRefs, activeDesign.name],
  );

  const handleRemoveClass = useCallback(
    (className: string) => {
      setElementClassRefs(
        element.id,
        currentRefs.filter((r) => r !== className),
      );
    },
    [element.id, currentRefs, setElementClassRefs],
  );

  return (
    <div style={{ marginBottom: 12 }}>
      <div
        style={{
          fontSize: 12,
          fontWeight: 600,
          color: "#6b7280",
          textTransform: "uppercase",
          marginBottom: 6,
        }}
      >
        Class 绑定
      </div>

      {/* Bound class pills */}
      <div
        style={{
          display: "flex",
          flexWrap: "wrap",
          gap: 4,
          marginBottom: 6,
        }}
      >
        {currentRefs.map((cls) => (
          <span
            key={cls}
            style={{
              display: "inline-flex",
              alignItems: "center",
              gap: 3,
              padding: "2px 8px",
              fontSize: 12,
              background: "#eef2ff",
              color: "#4338ca",
              borderRadius: 12,
              border: "1px solid #c7d2fe",
            }}
          >
            {cls}
            <button type="button"
              onClick={() => handleRemoveClass(cls)}
              title="移除 class"
              style={{
                border: "none",
                background: "none",
                cursor: "pointer",
                color: "#6366f1",
                fontSize: 12,
                padding: 0,
                lineHeight: 1,
              }}
            >
              ×
            </button>
          </span>
        ))}
        {currentRefs.length === 0 && (
          <span style={{ fontSize: 12, color: "#94a3b8" }}>
            未绑定 class
          </span>
        )}
      </div>

      {/* Search + dropdown */}
      <div style={{ position: "relative" }}>
        <input
          type="text"
          value={search}
          onChange={(e) => {
            setSearch(e.target.value);
            setShowDropdown(true);
          }}
          onFocus={() => setShowDropdown(true)}
          onBlur={() => setTimeout(() => setShowDropdown(false), 150)}
          placeholder="搜索 class..."
          aria-label="搜索 class"
          style={{
            width: "100%",
            padding: "3px 6px",
            fontSize: 12,
            border: "1px solid #e2e8f0",
            borderRadius: 4,
            outline: "2px solid transparent",
            fontFamily: "monospace",
            boxSizing: "border-box",
          }}
        />
        {showDropdown && filteredClasses.length > 0 && (
          <div
            style={{
              position: "absolute",
              top: "100%",
              left: 0,
              right: 0,
              maxHeight: 160,
              overflow: "auto",
              background: "#fff",
              border: "1px solid #e2e8f0",
              borderRadius: 4,
              boxShadow: "0 4px 12px rgba(0,0,0,0.08)",
              zIndex: 10,
            }}
          >
            {filteredClasses.map((c) => (
              <button
                key={c.id}
                type="button"
                tabIndex={0}
                onMouseDown={() => handleAddClass(c.name)}
                style={{
                  padding: "4px 8px",
                  fontSize: 12,
                  cursor: "pointer",
                  color: "#374151",
                  display: "flex",
                  justifyContent: "space-between",
                  alignItems: "center",
                  background: "none",
                  border: "none",
                  width: "100%",
                  textAlign: "left",
                  fontFamily: "inherit",
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.background = "#f1f5f9";
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = "transparent";
                }}
              >
                <span>{c.name}</span>
                <span style={{ color: "#94a3b8", fontSize: 12 }}>
                  {c.properties.length + c.pseudoStates.length} 规则
                </span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
