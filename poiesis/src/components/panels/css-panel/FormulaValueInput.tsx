// FormulaValueInput — token-based CSS value editor like Notion formula editor.
// Renders CSS values as styled tokens (chips) with autocomplete for var() references.

import React, { useRef, useState, useCallback, useMemo } from "react";
import type { CssVariable } from "../../../core/types/style";

// ── Token Types ──

export type ValueToken =
  | { type: "text"; value: string }
  | { type: "var-ref"; name: string }
  | { type: "dimension"; value: string }
  | { type: "color"; value: string }
  | { type: "keyword"; value: string };

/** CSS functions — treated as text to avoid over-tokenizing (kept for future use) */
// const RE_FUNC = /\b(calc|hsl|...)\\(/;

// ── CSS keywords (common values that should render as keyword chips) ──

const CSS_KEYWORDS = new Set([
  "none", "auto", "inherit", "initial", "unset",
  "block", "inline", "inline-block", "flex", "grid", "inline-flex",
  "static", "relative", "absolute", "fixed", "sticky",
  "visible", "hidden", "scroll", "clip",
  "row", "row-reverse", "column", "column-reverse",
  "nowrap", "wrap", "wrap-reverse",
  "flex-start", "flex-end", "center", "space-between", "space-around", "space-evenly",
  "stretch", "baseline",
  "left", "right", "top", "bottom",
  "normal", "bold", "bolder", "lighter",
  "pointer", "grab", "grabbing", "crosshair", "move", "not-allowed", "text",
  "solid", "dashed", "dotted", "double", "groove", "ridge", "inset", "outset",
  "uppercase", "lowercase", "capitalize",
  "underline", "line-through", "overline",
  "content-box", "border-box",
  "ease", "ease-in", "ease-out", "ease-in-out", "linear",
  "forwards", "backwards", "both",
  "alternate", "alternate-reverse",
  "infinite",
  "transparent",
  "white", "black", "red", "green", "blue", "yellow", "orange", "purple", "pink", "gray", "grey",
]);

// ── Parsing ──

/** RGB hex color */
const RE_COLOR = /#[0-9a-fA-F]{3,8}\b/;
/** Dimension: number + unit */
const RE_DIMENSION = /\b\d+\.?\d*(px|rem|em|%|vh|vw|vmin|vmax|pt|cm|mm|ms|s|deg|ch|fr|svh|svw|lvh|lvw|dvh|dvw|cqw|cqh|cqi|cqb|cqmin|cqmax|ex|ic|lh|rlh)\b/;
/** var() reference */
const RE_VAR = /var\(\s*(--[\w-]+)\s*(?:,\s*([^)]+))?\)/;
/** Parse a CSS value string into tokens. */
export function parseValueTokens(raw: string, _variables: CssVariable[]): ValueToken[] {
  if (!raw.trim()) return [];
  const tokens: ValueToken[] = [];
  let remaining = raw;

  while (remaining.length > 0) {
    const vMatch = remaining.match(RE_VAR);
    const cMatch = remaining.match(RE_COLOR);
    const dMatch = remaining.match(RE_DIMENSION);

    // Find earliest match
    type Match = { idx: number; len: number; token: ValueToken };
    const candidates: Match[] = [];
    if (vMatch && vMatch.index !== undefined) {
      candidates.push({ idx: vMatch.index, len: vMatch[0].length, token: { type: "var-ref", name: vMatch[1] } });
    }
    if (cMatch && cMatch.index !== undefined) {
      candidates.push({ idx: cMatch.index, len: cMatch[0].length, token: { type: "color", value: cMatch[0] } });
    }
    if (dMatch && dMatch.index !== undefined) {
      candidates.push({ idx: dMatch.index, len: dMatch[0].length, token: { type: "dimension", value: dMatch[0] } });
    }

    candidates.sort((a, b) => a.idx - b.idx);

    if (candidates.length > 0) {
      const m = candidates[0];
      // Leading text
      if (m.idx > 0) {
        const leadText = remaining.slice(0, m.idx);
        pushTextOrKeywords(tokens, leadText);
      }
      tokens.push(m.token);
      remaining = remaining.slice(m.idx + m.len);
    } else {
      // No more special tokens — rest is text
      pushTextOrKeywords(tokens, remaining);
      break;
    }
  }

  // Merge adjacent text tokens
  return mergeTextTokens(tokens);
}

/** Split plain text into text and keyword tokens, then push. */
function pushTextOrKeywords(tokens: ValueToken[], text: string): void {
  // Split by whitespace boundaries to detect standalone keywords
  const parts = text.split(/(\s+)/);
  for (const part of parts) {
    if (part === "") continue;
    if (/^\s+$/.test(part)) {
      tokens.push({ type: "text", value: part });
    } else if (CSS_KEYWORDS.has(part.trim())) {
      // Only tokenize if it's a whole word (not part of a function call etc)
      tokens.push({ type: "keyword", value: part });
    } else {
      tokens.push({ type: "text", value: part });
    }
  }
}

/** Merge consecutive text tokens. */
function mergeTextTokens(tokens: ValueToken[]): ValueToken[] {
  const result: ValueToken[] = [];
  for (const t of tokens) {
    const last = result[result.length - 1];
    if (last && last.type === "text" && t.type === "text") {
      last.value += t.value;
    } else {
      result.push(t);
    }
  }
  return result;
}

/** Serialize tokens back to a CSS value string. */
export function tokensToCss(tokens: ValueToken[]): string {
  return tokens.map((t) => {
    switch (t.type) {
      case "var-ref": return `var(${t.name})`;
      case "text": return t.value;
      case "color": return t.value;
      case "dimension": return t.value;
      case "keyword": return t.value;
    }
  }).join("");
}

// ── Token Colors ──

const TOKEN_STYLES: Record<string, React.CSSProperties> = {
  "var-ref": {
    background: "#EDE9FE", color: "#6D28D9", border: "1px solid #C4B5FD",
    borderRadius: 3, padding: "1px 4px", fontSize: 11, fontWeight: 600,
    fontFamily: "monospace",
  },
  "color": {
    background: "#FCE7F3", color: "#BE185D", border: "1px solid #F9A8D4",
    borderRadius: 3, padding: "1px 4px", fontSize: 11, fontWeight: 600,
    fontFamily: "monospace",
  },
  "dimension": {
    background: "#DBEAFE", color: "#1D4ED8", border: "1px solid #93C5FD",
    borderRadius: 3, padding: "1px 4px", fontSize: 11, fontWeight: 600,
    fontFamily: "monospace",
  },
  "keyword": {
    background: "#F3F4F6", color: "#6B7280", border: "1px solid #D1D5DB",
    borderRadius: 3, padding: "1px 4px", fontSize: 11, fontWeight: 500,
    fontFamily: "monospace",
  },
};

// ── Component Props ──

export interface FormulaValueInputProps {
  value: string;
  onChange: (value: string) => void;
  onBlur?: () => void;
  variables: CssVariable[];
  placeholder?: string;
  style?: React.CSSProperties;
}

// ── Component ──

export function FormulaValueInput({ value, onChange, onBlur, variables, placeholder, style }: FormulaValueInputProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [isFocused, setIsFocused] = useState(false);
  const [autocomplete, setAutocomplete] = useState<{
    show: boolean;
    query: string;
    items: CssVariable[];
    selectedIdx: number;
  }>({ show: false, query: "", items: [], selectedIdx: 0 });

  // Parse current value into tokens
  const tokens = useMemo(() => parseValueTokens(value, variables), [value, variables]);

  // Handle clicks on token chips to allow editing
  const handleTokenClick = useCallback((token: ValueToken, idx: number) => {
    if (token.type === "var-ref") {
      // Click on var token — replace with text for editing
      const newTokens = [...tokens];
      newTokens[idx] = { type: "text", value: `var(${token.name})` };
      onChange(tokensToCss(newTokens));
    }
  }, [tokens, onChange]);

  // Handle input changes
  const handleInput = useCallback((e: React.FormEvent<HTMLInputElement>) => {
    const raw = (e.target as HTMLInputElement).value;
    onChange(raw);

    // Check for var( autocomplete trigger
    const cursorPos = (e.target as HTMLInputElement).selectionStart || 0;
    const textBefore = raw.slice(0, cursorPos);
    const varMatch = textBefore.match(/var\(\s*([\w-]*)$/);
    if (varMatch) {
      const query = varMatch[1].toLowerCase();
      const matching = variables.filter((v) => v.name.toLowerCase().includes(query));
      if (matching.length > 0) {
        setAutocomplete({ show: true, query, items: matching, selectedIdx: 0 });
        return;
      }
    }
    setAutocomplete((s) => ({ ...s, show: false }));
  }, [onChange, variables]);

  // Handle key navigation in autocomplete
  const handleKeyDown = useCallback((e: React.KeyboardEvent<HTMLInputElement>) => {
    if (!autocomplete.show) return;

    if (e.key === "ArrowDown") {
      e.preventDefault();
      setAutocomplete((s) => ({ ...s, selectedIdx: Math.min(s.selectedIdx + 1, s.items.length - 1) }));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setAutocomplete((s) => ({ ...s, selectedIdx: Math.max(s.selectedIdx - 1, 0) }));
    } else if (e.key === "Enter" || e.key === "Tab") {
      e.preventDefault();
      const selected = autocomplete.items[autocomplete.selectedIdx];
      if (selected) {
        const input = e.target as HTMLInputElement;
        const cursorPos = input.selectionStart || 0;
        const textBefore = value.slice(0, cursorPos);
        const textAfter = value.slice(cursorPos);
        // Replace `var(--partial` with `var(--fullname)`
        const replaced = textBefore.replace(/var\(\s*[\w-]*$/, `var(${selected.name})`);
        onChange(replaced + textAfter);
      }
      setAutocomplete({ show: false, query: "", items: [], selectedIdx: 0 });
    } else if (e.key === "Escape") {
      setAutocomplete({ show: false, query: "", items: [], selectedIdx: 0 });
    }
  }, [autocomplete, value, onChange]);

  // Select autocomplete item on click
  const selectVariable = useCallback((varName: string) => {
    // Replace `var(` prefix before cursor with full var()
    const input = containerRef.current?.querySelector("input") as HTMLInputElement | null;
    if (input) {
      const cursorPos = input.selectionStart || 0;
      const textBefore = value.slice(0, cursorPos);
      const textAfter = value.slice(cursorPos);
      const replaced = textBefore.replace(/var\(\s*[\w-]*$/, `var(${varName})`);
      onChange(replaced + textAfter);
    } else {
      onChange(`var(${varName})`);
    }
    setAutocomplete({ show: false, query: "", items: [], selectedIdx: 0 });
  }, [value, onChange]);

  // Close autocomplete on blur
  const handleBlur = useCallback(() => {
    // Delay to allow click on autocomplete item
    setTimeout(() => {
      setIsFocused(false);
      setAutocomplete({ show: false, query: "", items: [], selectedIdx: 0 });
      onBlur?.();
    }, 150);
  }, [onBlur]);

  // Build display text with tokenized highlights (shown as styled spans below the input)
  const tokenSpans = useMemo(() => {
    if (tokens.length === 0 || tokens.every((t) => t.type === "text")) return null;
    return tokens.map((t, i) => {
      const display = t.type === "var-ref" ? `var(${t.name})`
        : t.type === "text" ? t.value
        : t.type === "color" ? t.value
        : t.type === "dimension" ? t.value
        : t.value;
      const ts = TOKEN_STYLES[t.type];
      return (
        <span
          key={i}
          style={{
            ...ts,
            cursor: t.type === "var-ref" ? "pointer" : "default",
            userSelect: "none",
          }}
          title={t.type === "var-ref" ? `Click to edit` : undefined}
          onClick={() => t.type === "var-ref" && handleTokenClick(t, i)}
        >
          {t.type === "color" && (
            <span style={{
              display: "inline-block", width: 10, height: 10, borderRadius: 2,
              background: t.value, marginRight: 3, verticalAlign: "middle",
              border: "1px solid rgba(0,0,0,0.15)",
            }} />
          )}
          {display}
        </span>
      );
    });
  }, [tokens, handleTokenClick]);

  const hasTokens = tokenSpans !== null;

  return (
    <div ref={containerRef} style={{ position: "relative", ...style }}>
      {/* Input row */}
      <div style={{
        display: "flex", alignItems: "center",
        border: `1px solid ${isFocused ? "#6366F1" : "#D1D5DB"}`,
        borderRadius: 4,
        background: isFocused ? "#fff" : "#FAFAFA",
        minHeight: 28,
        transition: "border-color 0.15s",
      }}>
        <input
          type="text"
          value={value}
          onChange={handleInput}
          onKeyDown={handleKeyDown}
          onFocus={() => setIsFocused(true)}
          onBlur={handleBlur}
          placeholder={placeholder ?? "输入CSS值，如 16px, var(--primary), #6366f1"}
          style={{
            flex: 1,
            border: "none",
            outline: "none",
            background: "transparent",
            padding: "3px 6px",
            fontSize: 11,
            fontFamily: "monospace",
            color: "#374151",
            minWidth: 0,
          }}
          spellCheck={false}
        />
        {/* Formula badge */}
        <span style={{
          fontSize: 9, color: "#9CA3AF", paddingRight: 6,
          fontFamily: "monospace", userSelect: "none",
          opacity: hasTokens ? 1 : 0,
        }}>
          {hasTokens ? "ƒ" : ""}
        </span>
      </div>

      {/* Token preview strip */}
      {hasTokens && (
        <div style={{
          display: "flex", flexWrap: "wrap", gap: 2, alignItems: "center",
          marginTop: 3, minHeight: 20, padding: "2px 2px",
          background: "#FAFAFA", borderRadius: 3, border: "1px solid #F3F4F6",
        }}>
          {tokenSpans}
        </div>
      )}

      {/* Autocomplete dropdown */}
      {autocomplete.show && (
        <div style={{
          position: "absolute", top: "100%", left: 0, right: 0,
          zIndex: 9999, background: "#fff",
          border: "1px solid #E2E8F0", borderRadius: 4,
          boxShadow: "0 4px 12px rgba(0,0,0,0.1)",
          maxHeight: 160, overflow: "auto",
          marginTop: 2,
        }}>
          {autocomplete.items.map((v, i) => (
            <div
              key={v.id}
              onMouseDown={(e) => { e.preventDefault(); selectVariable(v.name); }}
              style={{
                padding: "4px 8px", fontSize: 11, fontFamily: "monospace",
                cursor: "pointer",
                background: i === autocomplete.selectedIdx ? "#EEF2FF" : "transparent",
                color: i === autocomplete.selectedIdx ? "#4F46E5" : "#374151",
                display: "flex", alignItems: "center", gap: 6,
              }}
            >
              <span style={{ color: "#7C3AED", fontWeight: 600 }}>{v.name}</span>
              <span style={{ color: "#9CA3AF", fontSize: 10, flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                {v.segments[0]?.type === "text" ? v.segments[0].value : ""}
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
