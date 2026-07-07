// components/design/TokenEditor.tsx
// Token editor with color/spacing/typography/radii support, HUD color picker, drag-scrub, and palette view.

import { useState, useEffect, useRef, useMemo, useCallback } from "react";
import { usePoiesisStore } from "../../store";
import { ColorPicker } from "../panels/inspector/ColorPicker";
import type { DesignToken } from "../../core/types/design";

// ── Constants ──

const CATEGORY_ORDER = ["colors", "spacing", "typography", "radii"] as const;

const CATEGORY_LABELS: Record<string, string> = {
  colors: "Colors",
  spacing: "Spacing",
  typography: "Typography",
  radii: "Radii",
};

const CATEGORY_CONFIG: Record<
  string,
  { tokenType: DesignToken["tokenType"]; defaultName: string; defaultValue: string }
> = {
  colors: { tokenType: "color", defaultName: "color-new", defaultValue: "#6366f1" },
  spacing: { tokenType: "size", defaultName: "space-new", defaultValue: "8px" },
  typography: { tokenType: "font", defaultName: "font-new", defaultValue: "Inter" },
  radii: { tokenType: "size", defaultName: "radius-new", defaultValue: "4px" },
};

const TOKEN_TYPE_OPTIONS: DesignToken["tokenType"][] = ["color", "size", "font", "shadow"];

const SECTION_HEADER: React.CSSProperties = {
  fontSize: 11,
  fontWeight: 600,
  color: "#6b7280",
  textTransform: "uppercase",
  letterSpacing: "0.5px",
  marginBottom: 8,
};

const ROW_STYLE: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: 6,
  padding: "4px 0",
};

const INPUT_STYLE: React.CSSProperties = {
  padding: "3px 6px",
  fontSize: 11,
  fontFamily: "monospace",
  border: "1px solid #e2e8f0",
  borderRadius: 4,
  outline: "none",
  background: "#fff",
  color: "#374151",
  boxSizing: "border-box",
};

const REMOVE_BTN: React.CSSProperties = {
  border: "none",
  background: "none",
  cursor: "pointer",
  color: "#ef4444",
  fontSize: 14,
  padding: "2px 4px",
  lineHeight: 1,
  flexShrink: 0,
};

const SELECT_STYLE: React.CSSProperties = {
  padding: "3px 4px",
  fontSize: 10,
  border: "1px solid #e2e8f0",
  borderRadius: 4,
  outline: "none",
  background: "#fff",
  color: "#374151",
  cursor: "pointer",
};

// ── Drag-scrub helpers ──

function parseNumberUnit(value: string): { num: number; unit: string } {
  const match = value.match(/^(-?\d+(?:\.\d+)?)(.*)$/);
  if (match) {
    return { num: parseFloat(match[1]), unit: match[2] || "" };
  }
  return { num: NaN, unit: "" };
}

interface DragScrubState {
  tokenId: string;
  startX: number;
  startValue: number;
  unit: string;
}

// ── Component ──

export function TokenEditor() {
  const tokens = usePoiesisStore((s) => s.activeDesign.tokens);
  const addToken = usePoiesisStore((s) => s.addToken);
  const updateToken = usePoiesisStore((s) => s.updateToken);
  const removeToken = usePoiesisStore((s) => s.removeToken);

  const [searchQuery, setSearchQuery] = useState("");
  const [paletteView, setPaletteView] = useState(false);
  const [showNewMenu, setShowNewMenu] = useState(false);

  // Color picker popover
  const [colorPickerState, setColorPickerState] = useState<{
    tokenId: string;
    anchorRect: DOMRect;
  } | null>(null);

  // Refs
  const newMenuRef = useRef<HTMLDivElement>(null);
  const scrubRef = useRef<DragScrubState | null>(null);

  // ── Derived data ──

  const filteredTokens = useMemo(() => {
    if (!searchQuery.trim()) return tokens;
    const q = searchQuery.toLowerCase();
    return tokens.filter((t) => t.name.toLowerCase().includes(q));
  }, [tokens, searchQuery]);

  const grouped = useMemo(() => {
    const groups: Record<string, DesignToken[]> = {};
    // Ensure all known categories appear even when empty
    for (const cat of CATEGORY_ORDER) {
      groups[cat] = [];
    }
    for (const t of filteredTokens) {
      if (!groups[t.category]) groups[t.category] = [];
      groups[t.category].push(t);
    }
    return groups;
  }, [filteredTokens]);

  // Ratio bar max (computed from all spacing tokens, not just filtered)
  const spacingMax = useMemo(() => {
    const nums = tokens
      .filter((t) => t.category === "spacing")
      .map((t) => parseFloat(t.value))
      .filter((n) => !isNaN(n) && n > 0);
    return nums.length > 0 ? Math.max(...nums) : 1;
  }, [tokens]);

  // ── Global drag-scrub event handlers ──

  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      if (!scrubRef.current) return;
      const { tokenId, startX, startValue, unit } = scrubRef.current;
      const dx = e.clientX - startX;
      const modifier = e.shiftKey ? 10 : 1;
      const newVal = Math.max(0, Math.round((startValue + dx * modifier) * 100) / 100);
      usePoiesisStore.getState().updateToken(tokenId, { value: `${newVal}${unit}` });
    };
    const handleMouseUp = () => {
      scrubRef.current = null;
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
  }, []);

  // ── Close new-token menu on outside click ──

  useEffect(() => {
    if (!showNewMenu) return;
    const handleClick = (e: MouseEvent) => {
      if (newMenuRef.current && !newMenuRef.current.contains(e.target as Node)) {
        setShowNewMenu(false);
      }
    };
    const timer = setTimeout(() => document.addEventListener("mousedown", handleClick), 0);
    return () => {
      clearTimeout(timer);
      document.removeEventListener("mousedown", handleClick);
    };
  }, [showNewMenu]);

  // ── Handlers ──

  const handleAddToken = useCallback(
    (category: string) => {
      const cfg = CATEGORY_CONFIG[category];
      if (!cfg) return;
      addToken(cfg.defaultName, category, cfg.tokenType, cfg.defaultValue);
      setShowNewMenu(false);
    },
    [addToken],
  );

  const handleOpenColorPicker = useCallback(
    (tokenId: string, e: React.MouseEvent) => {
      const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
      setColorPickerState({ tokenId, anchorRect: rect });
    },
    [],
  );

  const handleCloseColorPicker = useCallback(() => {
    setColorPickerState(null);
  }, []);

  const handleScrubStart = useCallback(
    (tokenId: string, currentValue: string, e: React.MouseEvent) => {
      const parsed = parseNumberUnit(currentValue);
      if (isNaN(parsed.num)) return;
      e.preventDefault();
      scrubRef.current = {
        tokenId,
        startX: e.clientX,
        startValue: parsed.num,
        unit: parsed.unit,
      };
      document.body.style.cursor = "ew-resize";
      document.body.style.userSelect = "none";
    },
    [],
  );

  const handleRatioBarClick = useCallback(
    (tokenId: string, max: number, e: React.MouseEvent) => {
      const bar = e.currentTarget as HTMLElement;
      const rect = bar.getBoundingClientRect();
      const fraction = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
      const newVal = Math.round(fraction * max);
      // Determine unit from existing value
      const token = tokens.find((t) => t.id === tokenId);
      const { unit } = parseNumberUnit(token?.value ?? "");
      updateToken(tokenId, { value: `${newVal}${unit}` });
    },
    [tokens, updateToken],
  );

  // ── Render helpers ──

  const renderColorTokens = useCallback(
    (catTokens: DesignToken[]) => {
      if (paletteView) {
        return (
          <div style={{ display: "grid", gridTemplateColumns: "repeat(4, 1fr)", gap: 6, marginBottom: 8 }}>
            {catTokens.map((t) => (
              <div key={t.id} style={{ textAlign: "center" }}>
                <div
                  onClick={(e) => handleOpenColorPicker(t.id, e)}
                  style={{
                    width: "100%",
                    aspectRatio: "1",
                    borderRadius: 6,
                    background: t.value || "#ccc",
                    border: "2px solid #e2e8f0",
                    cursor: "pointer",
                    boxSizing: "border-box",
                  }}
                  title={t.name}
                />
                <div
                  style={{
                    fontSize: 9,
                    color: "#64748b",
                    marginTop: 2,
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                  }}
                >
                  {t.name}
                </div>
              </div>
            ))}
          </div>
        );
      }

      return (
        <div style={{ display: "flex", flexDirection: "column", gap: 2, marginBottom: 8 }}>
          {catTokens.map((t) => (
            <div key={t.id} style={ROW_STYLE}>
              {/* Color swatch */}
              <div
                onClick={(e) => handleOpenColorPicker(t.id, e)}
                style={{
                  width: 20,
                  height: 20,
                  borderRadius: 4,
                  background: t.value || "#ccc",
                  border: "1px solid #cbd5e1",
                  cursor: "pointer",
                  flexShrink: 0,
                }}
              />
              {/* Name */}
              <input
                value={t.name}
                onChange={(e) => updateToken(t.id, { name: e.target.value })}
                style={{ ...INPUT_STYLE, width: 80, flexShrink: 0 }}
                placeholder="name"
              />
              {/* Value (hex) */}
              <input
                value={t.value}
                onChange={(e) => updateToken(t.id, { value: e.target.value })}
                style={{ ...INPUT_STYLE, flex: 1, minWidth: 0 }}
                placeholder="#000000"
              />
              {/* Token type dropdown */}
              <select
                value={t.tokenType}
                onChange={(e) => updateToken(t.id, { tokenType: e.target.value as DesignToken["tokenType"] })}
                style={{ ...SELECT_STYLE, width: 56 }}
              >
                {TOKEN_TYPE_OPTIONS.map((opt) => (
                  <option key={opt} value={opt}>
                    {opt}
                  </option>
                ))}
              </select>
              {/* Remove */}
              <button onClick={() => removeToken(t.id)} style={REMOVE_BTN} title="Remove token">
                &times;
              </button>
            </div>
          ))}
        </div>
      );
    },
    [paletteView, handleOpenColorPicker, updateToken, removeToken],
  );

  const renderSpacingTokens = useCallback(
    (catTokens: DesignToken[]) => {
      return (
        <div style={{ display: "flex", flexDirection: "column", gap: 2, marginBottom: 8 }}>
          {catTokens.map((t) => {
            const { num, unit } = parseNumberUnit(t.value);
            const ratio = isNaN(num) || spacingMax === 0 ? 0 : Math.min(num / spacingMax, 1);
            return (
              <div key={t.id} style={ROW_STYLE}>
                {/* Name */}
                <input
                  value={t.name}
                  onChange={(e) => updateToken(t.id, { name: e.target.value })}
                  style={{ ...INPUT_STYLE, width: 80, flexShrink: 0 }}
                  placeholder="name"
                />
                {/* Value with drag-scrub */}
                <div
                  onMouseDown={(e) => handleScrubStart(t.id, t.value, e)}
                  style={{
                    ...INPUT_STYLE,
                    flex: 1,
                    minWidth: 0,
                    cursor: "ew-resize",
                    userSelect: "none",
                    background: "#f0fdf4",
                    display: "flex",
                    alignItems: "center",
                    gap: 4,
                  }}
                  title="Drag to scrub value"
                >
                  <span style={{ fontSize: 10, color: "#64748b", marginRight: 2 }}>&#8596;</span>
                  <span style={{ fontSize: 11, fontFamily: "monospace", color: "#374151" }}>
                    {t.value}
                  </span>
                </div>
                {/* Ratio bar (clickable) */}
                <div
                  onClick={(e) => handleRatioBarClick(t.id, spacingMax, e)}
                  style={{
                    flexShrink: 0,
                    width: 48,
                    height: 8,
                    borderRadius: 4,
                    background: "#e2e8f0",
                    position: "relative",
                    cursor: "pointer",
                    overflow: "hidden",
                  }}
                  title={`${Math.round(ratio * 100)}% of max (${spacingMax}${unit})`}
                >
                  <div
                    style={{
                      width: `${ratio * 100}%`,
                      height: "100%",
                      background: "#6366f1",
                      borderRadius: 4,
                      transition: "width 0.05s",
                    }}
                  />
                </div>
                {/* Remove */}
                <button onClick={() => removeToken(t.id)} style={REMOVE_BTN} title="Remove token">
                  &times;
                </button>
              </div>
            );
          })}
        </div>
      );
    },
    [spacingMax, handleScrubStart, handleRatioBarClick, updateToken, removeToken],
  );

  const renderTypographyTokens = useCallback(
    (catTokens: DesignToken[]) => {
      return (
        <div style={{ display: "flex", flexDirection: "column", gap: 2, marginBottom: 8 }}>
          {catTokens.map((t) => (
            <div key={t.id} style={ROW_STYLE}>
              <input
                value={t.name}
                onChange={(e) => updateToken(t.id, { name: e.target.value })}
                style={{ ...INPUT_STYLE, width: 80, flexShrink: 0 }}
                placeholder="name"
              />
              <input
                value={t.value}
                onChange={(e) => updateToken(t.id, { value: e.target.value })}
                style={{ ...INPUT_STYLE, flex: 1, minWidth: 0 }}
                placeholder="value"
              />
              <button onClick={() => removeToken(t.id)} style={REMOVE_BTN} title="Remove token">
                &times;
              </button>
            </div>
          ))}
        </div>
      );
    },
    [updateToken, removeToken],
  );

  const renderRadiiTokens = useCallback(
    (catTokens: DesignToken[]) => {
      return (
        <div style={{ display: "flex", flexDirection: "column", gap: 2, marginBottom: 8 }}>
          {catTokens.map((t) => {
            const { num } = parseNumberUnit(t.value);
            return (
              <div key={t.id} style={ROW_STYLE}>
                <input
                  value={t.name}
                  onChange={(e) => updateToken(t.id, { name: e.target.value })}
                  style={{ ...INPUT_STYLE, width: 80, flexShrink: 0 }}
                  placeholder="name"
                />
                <div
                  onMouseDown={(e) => handleScrubStart(t.id, t.value, e)}
                  style={{
                    ...INPUT_STYLE,
                    flex: 1,
                    minWidth: 0,
                    cursor: isNaN(num) ? "text" : "ew-resize",
                    userSelect: "none",
                    background: isNaN(num) ? "#fff" : "#f0fdf4",
                    display: "flex",
                    alignItems: "center",
                    gap: 4,
                  }}
                  title="Drag to scrub value"
                >
                  {!isNaN(num) && (
                    <span style={{ fontSize: 10, color: "#64748b", marginRight: 2 }}>&#8596;</span>
                  )}
                  <span style={{ fontSize: 11, fontFamily: "monospace", color: "#374151" }}>
                    {t.value}
                  </span>
                </div>
                <button onClick={() => removeToken(t.id)} style={REMOVE_BTN} title="Remove token">
                  &times;
                </button>
              </div>
            );
          })}
        </div>
      );
    },
    [handleScrubStart, updateToken, removeToken],
  );

  // ── Section rendering dispatch ──

  const renderSection = useCallback(
    (category: string, catTokens: DesignToken[]) => {
      const renderMap: Record<string, (t: DesignToken[]) => ReturnType<typeof renderColorTokens>> = {
        colors: renderColorTokens,
        spacing: renderSpacingTokens,
        typography: renderTypographyTokens,
        radii: renderRadiiTokens,
      };
      const renderFn = renderMap[category];
      if (!renderFn) return null;

      return (
        <div key={category} style={{ marginBottom: 16 }}>
          {/* Category header */}
          <div
            style={{
              ...SECTION_HEADER,
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
            }}
          >
            <span>
              {CATEGORY_LABELS[category] || category} ({catTokens.length})
            </span>
            {/* Palette toggle (colors only) */}
            {category === "colors" && catTokens.length > 0 && (
              <button
                onClick={() => setPaletteView((v) => !v)}
                style={{
                  border: "1px solid #e2e8f0",
                  borderRadius: 4,
                  background: paletteView ? "#6366f1" : "#f8fafc",
                  color: paletteView ? "#fff" : "#64748b",
                  fontSize: 9,
                  padding: "2px 6px",
                  cursor: "pointer",
                }}
              >
                {paletteView ? "List" : "Palette"}
              </button>
            )}
          </div>

          {renderFn(catTokens)}

          {/* Add button for this category */}
          {category === "colors" && (
            <button
              onClick={() => handleAddToken(category)}
              style={{
                border: "1px dashed #cbd5e1",
                borderRadius: 4,
                background: "transparent",
                color: "#64748b",
                fontSize: 10,
                padding: "4px 10px",
                cursor: "pointer",
                width: "100%",
              }}
            >
              + Add color
            </button>
          )}
          {category === "spacing" && (
            <button
              onClick={() => handleAddToken(category)}
              style={{
                border: "1px dashed #cbd5e1",
                borderRadius: 4,
                background: "transparent",
                color: "#64748b",
                fontSize: 10,
                padding: "4px 10px",
                cursor: "pointer",
                width: "100%",
              }}
            >
              + Add spacing
            </button>
          )}
          {category === "typography" && (
            <button
              onClick={() => handleAddToken(category)}
              style={{
                border: "1px dashed #cbd5e1",
                borderRadius: 4,
                background: "transparent",
                color: "#64748b",
                fontSize: 10,
                padding: "4px 10px",
                cursor: "pointer",
                width: "100%",
              }}
            >
              + Add font
            </button>
          )}
          {category === "radii" && (
            <button
              onClick={() => handleAddToken(category)}
              style={{
                border: "1px dashed #cbd5e1",
                borderRadius: 4,
                background: "transparent",
                color: "#64748b",
                fontSize: 10,
                padding: "4px 10px",
                cursor: "pointer",
                width: "100%",
              }}
            >
              + Add radius
            </button>
          )}
        </div>
      );
    },
    [
      renderColorTokens,
      renderSpacingTokens,
      renderTypographyTokens,
      renderRadiiTokens,
      paletteView,
      handleAddToken,
    ],
  );

  // ── Main render ──

  return (
    <div style={{ padding: 12, fontSize: 12, color: "#374151", position: "relative" }}>
      {/* Toolbar: search + new button */}
      <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 12 }}>
        <input
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          placeholder="Search tokens..."
          style={{
            ...INPUT_STYLE,
            flex: 1,
            minWidth: 0,
            padding: "5px 8px",
            fontSize: 12,
          }}
        />
        <div style={{ position: "relative" }}>
          <button
            onClick={() => setShowNewMenu((v) => !v)}
            style={{
              padding: "5px 10px",
              fontSize: 11,
              border: "none",
              borderRadius: 6,
              background: "#6366f1",
              color: "#fff",
              cursor: "pointer",
              fontWeight: 600,
              whiteSpace: "nowrap",
            }}
          >
            + New
          </button>
          {showNewMenu && (
            <div
              ref={newMenuRef}
              style={{
                position: "absolute",
                top: "100%",
                right: 0,
                marginTop: 4,
                background: "#fff",
                border: "1px solid #e2e8f0",
                borderRadius: 8,
                boxShadow: "0 4px 16px rgba(0,0,0,0.12)",
                zIndex: 1000,
                minWidth: 140,
                overflow: "hidden",
              }}
            >
              {CATEGORY_ORDER.map((cat) => (
                <button
                  key={cat}
                  onClick={() => handleAddToken(cat)}
                  style={{
                    display: "block",
                    width: "100%",
                    padding: "8px 14px",
                    fontSize: 12,
                    border: "none",
                    background: "transparent",
                    color: "#374151",
                    cursor: "pointer",
                    textAlign: "left",
                  }}
                  onMouseEnter={(e) => {
                    (e.currentTarget as HTMLElement).style.background = "#f1f5f9";
                  }}
                  onMouseLeave={(e) => {
                    (e.currentTarget as HTMLElement).style.background = "transparent";
                  }}
                >
                  {CATEGORY_LABELS[cat] || cat}
                </button>
              ))}
            </div>
          )}
        </div>
      </div>

      {/* Category sections */}
      {CATEGORY_ORDER.map((cat) => renderSection(cat, grouped[cat] || []))}

      {/* Unknown categories */}
      {Object.keys(grouped)
        .filter((cat) => !CATEGORY_ORDER.includes(cat as (typeof CATEGORY_ORDER)[number]))
        .map((cat) => (
          <div key={cat} style={{ marginBottom: 16 }}>
            <div style={SECTION_HEADER}>
              {cat} ({grouped[cat].length})
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: 2, marginBottom: 8 }}>
              {grouped[cat].map((t) => (
                <div key={t.id} style={ROW_STYLE}>
                  <input
                    value={t.name}
                    onChange={(e) => updateToken(t.id, { name: e.target.value })}
                    style={{ ...INPUT_STYLE, width: 80, flexShrink: 0 }}
                    placeholder="name"
                  />
                  <input
                    value={t.value}
                    onChange={(e) => updateToken(t.id, { value: e.target.value })}
                    style={{ ...INPUT_STYLE, flex: 1, minWidth: 0 }}
                    placeholder="value"
                  />
                  <button onClick={() => removeToken(t.id)} style={REMOVE_BTN} title="Remove token">
                    &times;
                  </button>
                </div>
              ))}
            </div>
          </div>
        ))}

      {/* Color picker overlay */}
      {colorPickerState &&
        (() => {
          const token = tokens.find((t) => t.id === colorPickerState.tokenId);
          if (!token) return null;
          return (
            <ColorPicker
              color={token.value || "#6366f1"}
              onChange={(newColor) => updateToken(colorPickerState.tokenId, { value: newColor })}
              onClose={handleCloseColorPicker}
              anchorRect={colorPickerState.anchorRect}
            />
          );
        })()}

      {/* Empty state */}
      {tokens.length === 0 && !searchQuery && (
        <div style={{ textAlign: "center", padding: 24, color: "#94a3b8", fontSize: 12 }}>
          No design tokens yet. Click <strong>+ New</strong> to add your first token.
        </div>
      )}

      {/* No results */}
      {tokens.length > 0 && filteredTokens.length === 0 && (
        <div style={{ textAlign: "center", padding: 24, color: "#94a3b8", fontSize: 12 }}>
          No tokens match "{searchQuery}"
        </div>
      )}
    </div>
  );
}
