// components/design/TokenEditor.tsx
// Token editor with color/spacing/typography/radii support, HUD color picker, drag-scrub, and palette view.

import { useState, useEffect, useRef, useMemo, useCallback } from "react";
import { usePoiesisStore } from "../../store";
import { ColorPicker } from "../panels/inspector/ColorPicker";
import type { DesignToken } from "../../core/types/design";
import styles from "./TokenEditor.module.css";
import { NewTokenMenu } from "./NewTokenMenu";

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
  outline: "2px solid transparent",
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
  outline: "2px solid transparent",
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
    const nums: number[] = [];
    for (const t of tokens) {
      if (t.category !== "spacing") continue;
      const n = parseFloat(t.value);
      if (!isNaN(n) && n > 0) nums.push(n);
    }
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
                <button
                  type="button"
                  tabIndex={0}
                  onClick={(e) => handleOpenColorPicker(t.id, e)}
                  onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); handleOpenColorPicker(t.id, e as unknown as React.MouseEvent); } }}
                  style={{
                    width: "100%",
                    aspectRatio: "1",
                    borderRadius: 8,
                    background: t.value || "#ccc",
                    border: "2px solid #e2e8f0",
                    cursor: "pointer",
                    boxSizing: "border-box",
                    padding: 0,
                  }}
                  title={t.name}
                />
                <div style={{ fontSize: 12, color: "#64748b", marginTop: 4, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
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
              <button
                type="button"
                tabIndex={0}
                aria-label={`Color swatch for ${t.name}`}
                onClick={(e) => handleOpenColorPicker(t.id, e)}
                onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); handleOpenColorPicker(t.id, e as unknown as React.MouseEvent); } }}
                style={{
                  width: 24,
                  height: 24,
                  borderRadius: 6,
                  background: t.value || "#ccc",
                  border: "1px solid #cbd5e1",
                  cursor: "pointer",
                  flexShrink: 0,
                  padding: 0,
                }}
              />
              {/* Name */}
              <input
                value={t.name}
                onChange={(e) => updateToken(t.id, { name: e.target.value })}
                style={{ ...INPUT_STYLE, width: 100, flexShrink: 0 }}
                placeholder="name"
              aria-label="Color token name"
              />
              {/* Value (hex) */}
              <input
                value={t.value}
                onChange={(e) => updateToken(t.id, { value: e.target.value })}
                style={{ ...INPUT_STYLE, flex: 1, minWidth: 0 }}
                placeholder="#000000"
                aria-label="Color value"
              />
              {/* Token type dropdown */}
              <select
                aria-label="Token type"
                value={t.tokenType}
                onChange={(e) => updateToken(t.id, { tokenType: e.target.value as DesignToken["tokenType"] })}
                style={{ ...SELECT_STYLE, width: 64 }}
              >
                {TOKEN_TYPE_OPTIONS.map((opt) => (
                  <option key={opt} value={opt}>{opt}</option>
                ))}
              </select>
              {/* Remove */}
              <button type="button" onClick={() => removeToken(t.id)} style={REMOVE_BTN} title="Remove token">
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
                  aria-label="Spacing token name"
                />
                {/* Value with drag-scrub */}
                <button
                  type="button"
                  tabIndex={0}
                  aria-label={`Scrub value: ${t.value}`}
                  onMouseDown={(e) => handleScrubStart(t.id, t.value, e)}
                  onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); handleScrubStart(t.id, t.value, e as unknown as React.MouseEvent); } }}
                  className={styles.spacingScrubBtn}
                  title="Drag to scrub value"
                >
                  <span style={{ fontSize: 12, color: "#64748b", marginRight: 2 }}>&#8596;</span>
                  <span style={{ fontSize: 13, fontFamily: "'Cascadia Code', 'Fira Code', monospace", color: "#374151" }}>
                    {num}{unit}
                  </span>
                </button>
                {/* Unit selector */}
                <select
                  aria-label="Spacing unit"
                  value={unit}
                  onChange={(e) => updateToken(t.id, { value: `${num}${e.target.value}` })}
                  className={styles.unitSelect}
                >
                  <option value="px">px</option>
                  <option value="rem">rem</option>
                  <option value="em">em</option>
                </select>
                {/* Ratio bar (clickable) */}
                <div
                  role="button"
                  tabIndex={0}
                  onClick={(e) => handleRatioBarClick(t.id, spacingMax, e)}
                  onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); handleRatioBarClick(t.id, spacingMax, e as unknown as React.MouseEvent); } }}
                  className={styles.ratioBar}
                  title={`${Math.round(ratio * 100)}% of max (${spacingMax}${unit})`}
                >
                  <div
                    className={styles.ratioFill}
                    style={{ width: `${ratio * 100}%` }}
                  />
                </div>
                {/* Remove */}
                <button type="button" onClick={() => removeToken(t.id)} style={REMOVE_BTN} title="Remove token">
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
                aria-label="Typography token name"
              />
              <input
                value={t.value}
                onChange={(e) => updateToken(t.id, { value: e.target.value })}
                style={{ ...INPUT_STYLE, flex: 1, minWidth: 0 }}
                placeholder="value"
                aria-label="Typography value"
              />
              <button type="button" onClick={() => removeToken(t.id)} style={REMOVE_BTN} title="Remove token">
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
                  aria-label="Radius token name"
                />
                <button
                  type="button"
                  tabIndex={0}
                  aria-label={`Scrub value: ${t.value}`}
                  onMouseDown={(e) => handleScrubStart(t.id, t.value, e)}
                  onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); handleScrubStart(t.id, t.value, e as unknown as React.MouseEvent); } }}
                  className={styles.spacingScrubBtn}
                  style={{
                    ...INPUT_STYLE,
                    cursor: isNaN(num) ? "text" : "ew-resize",
                    background: isNaN(num) ? "#fff" : "#f0fdf4",
                    fontFamily: "inherit",
                    fontSize: "inherit",
                  }}
                  title="Drag to scrub value"
                >
                  {!isNaN(num) && (
                    <span style={{ fontSize: 12, color: "#64748b", marginRight: 2 }}>&#8596;</span>
                  )}
                  <span style={{ fontSize: 13, fontFamily: "'Cascadia Code', 'Fira Code', monospace", color: "#374151" }}>
                    {num}{unit}
                  </span>
                </button>
                {/* Unit selector for radii */}
                <select
                  aria-label="Radius unit"
                  value={unit}
                  onChange={(e) => updateToken(t.id, { value: `${num}${e.target.value}` })}
                  className={styles.unitSelect}
                >
                  <option value="px">px</option>
                  <option value="rem">rem</option>
                  <option value="em">em</option>
                </select>
                <button type="button" onClick={() => removeToken(t.id)} style={REMOVE_BTN} title="Remove token">
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
              <button type="button"
                onClick={() => setPaletteView((v) => !v)}
                className={styles.paletteToggleBtn}
                style={{ background: paletteView ? "#6366f1" : "#f8fafc", color: paletteView ? "#fff" : "#64748b" }}
              >
                {paletteView ? "List" : "Palette"}
              </button>
            )}
          </div>

          {renderFn(catTokens)}

          {/* Add button */}
          <button type="button" onClick={() => handleAddToken(category)} style={ADD_BTN_STYLE}>
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
            aria-label="Search tokens"
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
            <button type="button"
              onClick={() => setSearchQuery("")}
              className={styles.clearSearchBtn}
            >
              &times;
            </button>
          )}
        </div>
        <div style={{ position: "relative" }}>
          <button type="button"
            onClick={() => setShowNewMenu((v) => !v)}
            className={styles.newTokenBtn}
          >
            + New token
          </button>
          {showNewMenu && (
            <NewTokenMenu
              ref={newMenuRef}
              onSelect={handleAddToken}
            />
          )}
        </div>
      </div>

      {/* Category sections */}
      {CATEGORY_ORDER.map((cat) => renderSection(cat, grouped[cat] || []))}

      {/* Unknown categories */}
      {(() => {
        const unknownCats: JSX.Element[] = [];
        for (const cat of Object.keys(grouped)) {
          if (!CATEGORY_ORDER.includes(cat as (typeof CATEGORY_ORDER)[number])) {
            unknownCats.push(
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
                        aria-label="Token name"
                      />
                      <input
                        value={t.value}
                        onChange={(e) => updateToken(t.id, { value: e.target.value })}
                        style={{ ...INPUT_STYLE, flex: 1, minWidth: 0 }}
                        placeholder="value"
                        aria-label="Token value"
                      />
                      <button type="button" onClick={() => removeToken(t.id)} style={REMOVE_BTN} title="Remove token">
                        &times;
                      </button>
                    </div>
                  ))}
                </div>
              </div>
            );
          }
        }
        return unknownCats;
      })()}

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
