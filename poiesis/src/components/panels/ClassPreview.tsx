// ui/panels/ClassPreview.tsx
// Renders a single HTML element with the given class styles applied for live preview.

import React, { useMemo } from "react";
import type { StyleClass } from "../../core/types/style";

const PREVIEW_TAGS = ["div", "button", "h1", "h2", "h3", "input", "span", "p"];

interface Props {
  styleClass: StyleClass;
  previewTag: string;
  onTagChange: (tag: string) => void;
}

export function ClassPreview({ styleClass, previewTag, onTagChange }: Props) {
  const cssText = useMemo(() => {
    return styleClass.properties
      .map((p) => {
        const val = segmentsToCssValue(p.segments);
        return `${p.name}: ${val};`;
      })
      .join(" ");
  }, [styleClass]);

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
      {/* Tag selector */}
      <select
        value={previewTag}
        onChange={(e) => onTagChange(e.target.value)}
        style={{
          padding: "2px 6px", fontSize: 11, border: "1px solid #d1d5db",
          borderRadius: 4, background: "#fff", cursor: "pointer",
        }}
      >
        {PREVIEW_TAGS.map((t) => (
          <option key={t} value={t}>&lt;{t}&gt;</option>
        ))}
      </select>

      {/* Preview area */}
      <div style={{
        border: "1px dashed #d1d5db", borderRadius: 6,
        padding: 12, minWidth: 120, minHeight: 40,
        background: "#fafafa",
        display: "flex", alignItems: "center", justifyContent: "center",
      }}>
        {previewTag === "button" && (
          <button style={cssText ? parseInlineStyle(cssText) : undefined}>
            {cssText ? "Click" : "Button"}
          </button>
        )}
        {previewTag === "input" && (
          <input placeholder="Input" style={cssText ? parseInlineStyle(cssText) : undefined} />
        )}
        {(previewTag === "div" || previewTag === "span" || previewTag === "p") && (
          React.createElement(previewTag, {
            style: cssText ? parseInlineStyle(cssText) : undefined,
            children: cssText ? "Preview" : previewTag,
          })
        )}
        {(previewTag === "h1" || previewTag === "h2" || previewTag === "h3") && (
          React.createElement(previewTag, {
            style: cssText ? parseInlineStyle(cssText) : undefined,
            children: cssText ? "Heading" : previewTag,
          })
        )}
      </div>
    </div>
  );
}

function segmentsToCssValue(segments: Array<{ type: string; value?: string; name?: string; label?: string }>): string {
  return segments.map((s) => {
    if (s.type === "text") return s.value ?? "";
    if (s.type === "var-ref") return `var(${s.name})`;
    if (s.type === "signal") return `@${s.label}`;
    return "";
  }).join("");
}

function parseInlineStyle(cssText: string): React.CSSProperties {
  const style: Record<string, string> = {};
  cssText.split(";").forEach((decl) => {
    const colonIdx = decl.indexOf(":");
    if (colonIdx > 0) {
      const prop = decl.slice(0, colonIdx).trim();
      const val = decl.slice(colonIdx + 1).trim();
      if (prop && val) {
        const camelProp = prop.replace(/-([a-z])/g, (_, c) => c.toUpperCase());
        style[camelProp] = val;
      }
    }
  });
  return style as React.CSSProperties;
}
