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

/** Singular form for "+ Add {singular}" button. */
const ADD_BTN_SINGULAR: Record<string, string> = {
  colors: "color",
  spacing: "spacing",
  typography: "typography",
  radii: "radius",
};

const CATEGORY_ICONS: Record<string, string> = {
  colors: "🎨",
  spacing: "📏",
  typography: "🔤",
  radii: "⬜",
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

// ── Shared styles ──

const CARD_STYLE: React.CSSProperties = {
  background: "#fff",
  border: "1px solid #e2e8f0",
  borderRadius: 10,
  padding: "16px 20px",
  marginBottom: 16,
};

const SECTION_HEADER: React.CSSProperties = {
  fontSize: 12,
  fontWeight: 600,
  color: "#475569",
  textTransform: "uppercase",
  letterSpacing: "0.5px",
  marginBottom: 12,
  display: "flex",
  alignItems: "center",
  justifyContent: "space-between",
};

const ROW_STYLE: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: 8,
  padding: "4px 0",
};

const INPUT_STYLE: React.CSSProperties = {
  padding: "4px 8px",
  fontSize: 13,
  fontFamily: "'Cascadia Code', 'Fira Code', monospace",
  border: "1px solid #d1d5db",
  borderRadius: 6,
  outline: "none",
  background: "#fff",
  color: "#374151",
  boxSizing: "border-box",
  transition: "border-color 150ms",
};

const REMOVE_BTN: React.CSSProperties = {
  border: "none",
  background: "none",
  cursor: "pointer",
  color: "#ef4444",
  fontSize: 16,
  padding: "2px 6px",
  lineHeight: 1,
  flexShrink: 0,
  borderRadius: 4,
};

const SELECT_STYLE: React.CSSProperties = {
  padding: "4px 6px",
  fontSize: 12,
  border: "1px solid #d1d5db",
  borderRadius: 6,
  outline: "none",
  background: "#fff",
  color: "#374151",
  cursor: "pointer",
  fontFamily: "inherit",
};

const ADD_BTN_STYLE: React.CSSProperties = {
  border: "1px dashed #cbd5e1",
  borderRadius: 8,
  background: "transparent",
  color: "#6366f1",
  fontSize: 12,
  fontWeight: 500,
  padding: "6px 14px",
  cursor: "pointer",
  width: "100%",
  fontFamily: "inherit",
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
    for (const cat of CATEGORY_ORDER) {
      groups[cat] = [];
    }
    for (const t of filteredTokens) {
      if (!groups[t.category]) groups[t.category] = [];
      groups[t.category].push(t);
    }
    return groups;
  }, [filteredTokens]);

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
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(64px, 1fr))", gap: 8, marginBottom: 8 }}>
            {catTokens.map((t) => (
              <div key={t.id} style={{ textAlign: "center" }}>
                <div
                  onClick={(e) => handleOpenColorPicker(t.id, e)}
                  style={{
                    width: "100%",
                    aspectRatio: "1",
                    borderRadius: 8,
                    background: t.value || "#ccc",
                    border: "2px solid #e2e8f0",
                    cursor: "pointer",
                    boxSizing: "border-box",
                  }}
                  title={t.name}
                />
                <div style={{ fontSize: 11, color: "#64748b", marginTop: 4, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                  {t.name}
                </div>
              </div>
            ))}
          </div>
        );
      }

      return (
        <div style={{ display: "flex", flexDirection: "column", gap: 2, marginBottom: 4 }}>
          {catTokens.map((t) => (
            <div key={t.id} style={ROW_STYLE}>
              {/* Color swatch */}
              <div
                onClick={(e) => handleOpenColorPicker(t.id, e)}
                style={{
                  width: 24,
                  height: 24,
                  borderRadius: 6,
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
                style={{ ...INPUT_STYLE, width: 100, flexShrink: 0 }}
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
                style={{ ...SELECT_STYLE, width: 64 }}
              >
                {TOKEN_TYPE_OPTIONS.map((opt) => (
                  <option key={opt} value={opt}>{opt}</option>
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
        <div style={{ display: "flex", flexDirection: "column", gap: 2, marginBottom: 4 }}>
          {catTokens.map((t) => {
            const { num, unit } = parseNumberUnit(t.value);
            const ratio = isNaN(num) || spacingMax === 0 ? 0 : Math.min(num / spacingMax, 1);
            return (
              <div key={t.id} style={ROW_STYLE}>
                {/* Name */}
                <input
                  value={t.name}
                  onChange={(e) => updateToken(t.id, { name: e.target.value })}
                  style={{ ...INPUT_STYLE, width: 100, flexShrink: 0 }}
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
                  <span style={{ fontSize: 11, color: "#64748b", marginRight: 2 }}>&#8596;</span>
                  <span style={{ fontSize: 13, fontFamily: "'Cascadia Code', 'Fira Code', monospace", color: "#374151" }}>
                    {num}{unit}
                  </span>
                </div>
                {/* Unit selector */}
                <select
                  value={unit}
                  onChange={(e) => updateToken(t.id, { value: `${num}${e.target.value}` })}
                  style={{
                    padding: "4px 4px",
                    fontSize: 11,
                    fontFamily: "'Cascadia Code', 'Fira Code', monospace",
                    border: "1px solid #d1d5db",
                    borderRadius: 6,
                    outline: "none",
                    background: "#fff",
                    color: "#374151",
                    cursor: "pointer",
                    flexShrink: 0,
                    width: 56,
                  }}
                >
                  <option value="px">px</option>
                  <option value="rem">rem</option>
                  <option value="em">em</option>
                </select>
                {/* Ratio bar (clickable) */}
                <div
                  onClick={(e) => handleRatioBarClick(t.id, spacingMax, e)}
                  style={{
                    flexShrink: 0,
                    width: 56,
                    height: 10,
                    borderRadius: 5,
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
                      borderRadius: 5,
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
        <div style={{ display: "flex", flexDirection: "column", gap: 2, marginBottom: 4 }}>
          {catTokens.map((t) => (
            <div key={t.id} style={ROW_STYLE}>
              <input
                value={t.name}
                onChange={(e) => updateToken(t.id, { name: e.target.value })}
                style={{ ...INPUT_STYLE, width: 100, flexShrink: 0 }}
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
        <div style={{ display: "flex", flexDirection: "column", gap: 2, marginBottom: 4 }}>
          {catTokens.map((t) => {
            const { num, unit } = parseNumberUnit(t.value);
            return (
              <div key={t.id} style={ROW_STYLE}>
                <input
                  value={t.name}
                  onChange={(e) => updateToken(t.id, { name: e.target.value })}
                  style={{ ...INPUT_STYLE, width: 100, flexShrink: 0 }}
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
                    <span style={{ fontSize: 11, color: "#64748b", marginRight: 2 }}>&#8596;</span>
                  )}
                  <span style={{ fontSize: 13, fontFamily: "'Cascadia Code', 'Fira Code', monospace", color: "#374151" }}>
                    {num}{unit}
                  </span>
                </div>
                {/* Unit selector for radii */}
                <select
                  value={unit}
                  onChange={(e) => updateToken(t.id, { value: `${num}${e.target.value}` })}
                  style={{
                    padding: "4px 4px",
                    fontSize: 11,
                    fontFamily: "'Cascadia Code', 'Fira Code', monospace",
                    border: "1px solid #d1d5db",
                    borderRadius: 6,
                    outline: "none",
                    background: "#fff",
                    color: "#374151",
                    cursor: "pointer",
                    flexShrink: 0,
                    width: 56,
                  }}
                >
                  <option value="px">px</option>
                  <option value="rem">rem</option>
                  <option value="em">em</option>
                </select>
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

      const icon = CATEGORY_ICONS[category] || "";
      const label = CATEGORY_LABELS[category] || category;

      return (
        <div key={category} style={CARD_STYLE}>
          {/* Category header */}
          <div style={SECTION_HEADER}>
            <span>{icon} {label} ({catTokens.length})</span>
            {/* Palette toggle (colors only) */}
            {category === "colors" && catTokens.length > 0 && (
              <button
                onClick={() => setPaletteView((v) => !v)}
                style={{
                  border: "1px solid #e2e8f0",
                  borderRadius: 6,
                  background: paletteView ? "#6366f1" : "#f8fafc",
                  color: paletteView ? "#fff" : "#64748b",
                  fontSize: 11,
                  fontWeight: 500,
                  padding: "3px 10px",
                  cursor: "pointer",
                  fontFamily: "inherit",
                }}
              >
                {paletteView ? "List" : "Palette"}
              </button>
            )}
          </div>

          {renderFn(catTokens)}

          {/* Add button */}
          <button onClick={() => handleAddToken(category)} style={ADD_BTN_STYLE}>
            + Add {ADD_BTN_SINGULAR[category] || label.toLowerCase().slice(0, -1)}
          </button>
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
    <div style={{ fontSize: 13, color: "#374151", position: "relative" }}>
      {/* Toolbar: search + new button */}
      <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 16 }}>
        <div style={{ position: "relative", flex: 1 }}>
          <input
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search tokens..."
            style={{
              ...INPUT_STYLE,
              width: "100%",
              padding: "8px 32px 8px 12px",
              fontSize: 13,
              borderColor: "#cbd5e1",
            }}
          />
          {searchQuery && (
            <button
              onClick={() => setSearchQuery("")}
              style={{
                position: "absolute",
                right: 8,
                top: "50%",
                transform: "translateY(-50%)",
                border: "none",
                background: "none",
                cursor: "pointer",
                color: "#94a3b8",
                fontSize: 16,
                padding: 0,
                lineHeight: 1,
              }}
            >
              &times;
            </button>
          )}
        </div>
        <div style={{ position: "relative" }}>
          <button
            onClick={() => setShowNewMenu((v) => !v)}
            style={{
              padding: "8px 16px",
              fontSize: 13,
              border: "none",
              borderRadius: 8,
              background: "#6366f1",
              color: "#fff",
              cursor: "pointer",
              fontWeight: 600,
              whiteSpace: "nowrap",
              fontFamily: "inherit",
            }}
          >
            + New token
          </button>
          {showNewMenu && (
            <div
              ref={(el) => {
                (newMenuRef as React.MutableRefObject<HTMLDivElement | null>).current = el;
                if (el) {
                  const btn = el.previousSibling as HTMLElement;
                  if (btn) {
                    const r = btn.getBoundingClientRect();
                    el.style.top = `${r.bottom + 4}px`;
                    el.style.left = `${r.right - 160}px`;
                  }
                }
              }}
              style={{
                position: "fixed",
                background: "#fff",
                border: "1px solid #e2e8f0",
                borderRadius: 10,
                boxShadow: "0 6px 24px rgba(0,0,0,0.14)",
                zIndex: 10000,
                minWidth: 160,
                overflow: "hidden",
              }}
            >{CATEGORY_ORDER.map((cat) => (
                <button
                  key={cat}
                  onClick={() => handleAddToken(cat)}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: 8,
                    width: "100%",
                    padding: "9px 16px",
                    fontSize: 13,
                    border: "none",
                    background: "transparent",
                    color: "#374151",
                    cursor: "pointer",
                    textAlign: "left",
                    fontFamily: "inherit",
                  }}
                  onMouseEnter={(e) => { (e.currentTarget as HTMLElement).style.background = "#f1f5f9"; }}
                  onMouseLeave={(e) => { (e.currentTarget as HTMLElement).style.background = "transparent"; }}
                >
                  <span>{CATEGORY_ICONS[cat]}</span>
                  <span>{CATEGORY_LABELS[cat]}</span>
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
          <div key={cat} style={{ ...CARD_STYLE }}>
            <div style={SECTION_HEADER}>{cat} ({grouped[cat].length})</div>
            <div style={{ display: "flex", flexDirection: "column", gap: 2, marginBottom: 8 }}>
              {grouped[cat].map((t) => (
                <div key={t.id} style={ROW_STYLE}>
                  <input
                    value={t.name}
                    onChange={(e) => updateToken(t.id, { name: e.target.value })}
                    style={{ ...INPUT_STYLE, width: 100, flexShrink: 0 }}
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
        <div style={{ textAlign: "center", padding: 40, color: "#94a3b8", fontSize: 14 }}>
          <div style={{ fontSize: 32, marginBottom: 12 }}>🎨</div>
          <div>No design tokens yet.</div>
          <div style={{ marginTop: 4 }}>Click <strong>+ New token</strong> to add colors, spacing, fonts, or radii.</div>
        </div>
      )}

      {/* No results */}
      {tokens.length > 0 && filteredTokens.length === 0 && (
        <div style={{ textAlign: "center", padding: 40, color: "#94a3b8", fontSize: 14 }}>
          No tokens match "{searchQuery}"
        </div>
      )}
    </div>
  );
}
