// components/design/IconButtonGroup.tsx
// Generic segmented button group rendered as a radiogroup.

import { useState } from "react";

interface IconButtonGroupProps<T extends string> {
  options: { value: T; label: string; title?: string }[];
  value: T;
  onChange: (value: T) => void;
}

export function IconButtonGroup<T extends string>({
  options,
  value,
  onChange,
}: IconButtonGroupProps<T>) {
  const [hoveredValue, setHoveredValue] = useState<T | null>(null);

  const containerStyle: React.CSSProperties = {
    display: "flex",
    flexDirection: "row",
    gap: 0,
  };

  return (
    <div style={containerStyle} role="radiogroup">
      {options.map((option, idx) => {
        const isActive = option.value === value;
        const isHovered = option.value === hoveredValue;
        const borderRadius = getBorderRadius(idx, options.length);

        const buttonStyle: React.CSSProperties = {
          display: "inline-flex",
          alignItems: "center",
          padding: "2px 6px",
          fontSize: 11,
          lineHeight: "18px",
          borderRadius,
          border: isActive ? "1px solid #6366f1" : "1px solid #e2e8f0",
          background: isActive ? "#6366f1" : isHovered ? "#f1f5f9" : "#fff",
          color: isActive ? "#fff" : "#64748b",
          cursor: "pointer",
          fontFamily: "inherit",
          userSelect: "none",
          outline: "none",
        };

        return (
          <button
            key={option.value}
            role="radio"
            aria-checked={isActive}
            title={option.title ?? option.label}
            style={buttonStyle}
            onClick={() => onChange(option.value)}
            onMouseEnter={() => setHoveredValue(option.value)}
            onMouseLeave={() => setHoveredValue(null)}
          >
            {option.label}
          </button>
        );
      })}
    </div>
  );
}

function getBorderRadius(idx: number, total: number): string {
  if (total <= 1) return "4px";
  if (idx === 0) return "4px 0 0 4px";
  if (idx === total - 1) return "0 4px 4px 0";
  return "0";
}
