// ui/panels/StyleValueEditor.tsx
// OmniLang style value editing.
// Supports three segment types: text, signal (@ trigger), var-ref (var( trigger).

import React from "react";
import type { StyleValueSegment } from "../../core/types/style";

interface SignalOption {
  name: string;     // e.g. "ThemeStore.currentBg"
  label: string;    // e.g. "currentBg"
}

interface Props {
  segments: StyleValueSegment[];
  onChange: (segments: StyleValueSegment[]) => void;
  availableSignals?: SignalOption[];
  availableCssVars?: string[];  // e.g. ["--primary", "--radius-sm"]
  placeholder?: string;
}

// Minimal single-line text input with chip rendering for signals/var-refs.
// Phase 2 is intentionally lightweight — can be deepened in a follow-up if needed.

export function StyleValueEditor({
  segments,
  onChange,
  availableSignals = [],
  availableCssVars = [],
  placeholder = "",
}: Props) {
  const [text, setText] = React.useState(segmentsToDisplay(segments));
  const [showSuggestions, setShowSuggestions] = React.useState(false);
  const [suggestionType, setSuggestionType] = React.useState<"signal" | "var-ref" | null>(null);
  const [filterText, setFilterText] = React.useState("");
  const [hoveredIndex, setHoveredIndex] = React.useState<string | null>(null);
  const [highlightedIndex, setHighlightedIndex] = React.useState<number | null>(null);
  const inputRef = React.useRef<HTMLInputElement>(null);

  // Keep text in sync when segments change externally
  React.useEffect(() => {
    setText(segmentsToDisplay(segments));
  }, [segments]);

  const handleInput = (rawValue: string) => {
    setText(rawValue);

    // Detect @ trigger
    const atMatch = rawValue.match(/@(\w*)$/);
    if (atMatch && availableSignals.length > 0) {
      setFilterText(atMatch[1]);
      setSuggestionType("signal");
      setShowSuggestions(true);
      setHighlightedIndex(null);
      return;
    }

    // Detect var( trigger
    const varMatch = rawValue.match(/var\((\-{0,2}\w*)$/);
    if (varMatch && availableCssVars.length > 0) {
      setFilterText(varMatch[1]);
      setSuggestionType("var-ref");
      setShowSuggestions(true);
      setHighlightedIndex(null);
      return;
    }

    setShowSuggestions(false);
    setHighlightedIndex(null);

    // If no special trigger, treat as plain text
    if (!rawValue.includes("@") && !rawValue.includes("var(")) {
      onChange([{ type: "text", value: rawValue }]);
    }
  };

  const insertSignal = (signal: SignalOption) => {
    const beforeText = text.replace(/@\w*$/, "");
    const newSegments: StyleValueSegment[] = [];

    if (beforeText.trim()) {
      newSegments.push({ type: "text", value: beforeText });
    }
    newSegments.push({ type: "signal", name: signal.name, label: signal.label });

    onChange(newSegments);
    setShowSuggestions(false);
    setHighlightedIndex(null);
    inputRef.current?.focus();
  };

  const insertVarRef = (varName: string) => {
    const beforeText = text.replace(/var\([\-]*\w*$/, "");
    const newSegments: StyleValueSegment[] = [];

    if (beforeText.trim()) {
      newSegments.push({ type: "text", value: beforeText });
    }
    newSegments.push({ type: "var-ref", name: varName });

    onChange(newSegments);
    setShowSuggestions(false);
    setHighlightedIndex(null);
    inputRef.current?.focus();
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (!showSuggestions) return;
    if (e.key === "Escape") { setShowSuggestions(false); setHighlightedIndex(null); return; }
    const items = suggestionType === "signal" ? filteredSignals : filteredVars;
    if (items.length === 0) return;
    const currentIdx = highlightedIndex ?? -1;
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setHighlightedIndex(Math.min(currentIdx + 1, items.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setHighlightedIndex(Math.max(currentIdx - 1, 0));
    } else if (e.key === "Enter" && currentIdx >= 0) {
      if (suggestionType === "signal") insertSignal(filteredSignals[currentIdx]);
      else insertVarRef(filteredVars[currentIdx]);
    }
  };

  const filteredSignals = availableSignals.filter((s) =>
    s.label.toLowerCase().includes(filterText.toLowerCase()) ||
    s.name.toLowerCase().includes(filterText.toLowerCase())
  );

  const filteredVars = availableCssVars.filter((v) =>
    v.toLowerCase().includes(filterText.toLowerCase())
  );

  // Render chip-based display when segments include signals/var-refs
  const hasChips = segments.some((s) => s.type !== "text");

  return (
    <div style={{ position: "relative", flex: 1 }}>
      {hasChips ? (
        <div
          onClick={() => setText(segmentsToDisplay(segments))}
          style={{
            display: "flex", alignItems: "center", gap: 2, flexWrap: "wrap",
            minHeight: 24, padding: "2px 6px", border: "1px solid #d1d5db",
            borderRadius: 4, background: "#fff", cursor: "text",
            fontSize: 12,
          }}
        >
          {segments.map((seg, i) => {
            if (seg.type === "text") {
              return <span key={i} style={{ color: "#374151" }}>{seg.value}</span>;
            }
            if (seg.type === "signal") {
              return (
                <span key={i} style={{
                  background: "#DBEAFE", color: "#1E40AF", borderRadius: 3,
                  padding: "1px 5px", fontSize: 11, fontWeight: 500,
                  border: "1px solid #BFDBFE",
                }}>
                  @{seg.label}
                </span>
              );
            }
            if (seg.type === "var-ref") {
              return (
                <span key={i} style={{
                  background: "#D1FAE5", color: "#065F46", borderRadius: 3,
                  padding: "1px 5px", fontSize: 11, fontWeight: 500,
                  border: "1px solid #A7F3D0",
                }}>
                  var({seg.name})
                </span>
              );
            }
            return null;
          })}
        </div>
      ) : (
        <input
          ref={inputRef}
          type="text"
          value={text}
          onChange={(e) => handleInput(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={placeholder}
          style={{
            background: "#fff", color: "#374151", border: "1px solid #d1d5db",
            borderRadius: 4, padding: "4px 8px", fontSize: 12, width: "100%",
            boxSizing: "border-box",
          }}
        />
      )}

      {/* Suggestion dropdown */}
      {showSuggestions && (
        <div style={{
          position: "absolute", zIndex: 20, top: "100%", left: 0,
          background: "#fff", border: "1px solid #d1d5db", borderRadius: 4,
          boxShadow: "0 4px 12px rgba(0,0,0,0.1)", maxHeight: 200,
          overflow: "auto", minWidth: 200,
        }}>
          {suggestionType === "signal" && filteredSignals.map((s, idx) => (
            <div
              key={s.name}
              onClick={() => insertSignal(s)}
              style={{
                padding: "6px 10px", cursor: "pointer", fontSize: 12,
                background: highlightedIndex === idx
                  ? "#E5E7EB"
                  : hoveredIndex === s.name
                    ? "#F3F4F6"
                    : "transparent",
              }}
              onMouseEnter={() => setHoveredIndex(s.name)}
              onMouseLeave={() => setHoveredIndex(null)}
            >
              <span style={{ color: "#1E40AF", fontWeight: 500 }}>@{s.label}</span>
              <span style={{ color: "#9CA3AF", marginLeft: 8, fontSize: 10 }}>{s.name}</span>
            </div>
          ))}
          {suggestionType === "var-ref" && filteredVars.map((v, idx) => (
            <div
              key={v}
              onClick={() => insertVarRef(v)}
              style={{
                padding: "6px 10px", cursor: "pointer", fontSize: 12,
                background: highlightedIndex === idx
                  ? "#E5E7EB"
                  : hoveredIndex === v
                    ? "#F3F4F6"
                    : "transparent",
              }}
              onMouseEnter={() => setHoveredIndex(v)}
              onMouseLeave={() => setHoveredIndex(null)}
            >
              <span style={{ color: "#065F46", fontWeight: 500 }}>{v}</span>
            </div>
          ))}
          {((suggestionType === "signal" && filteredSignals.length === 0) ||
            (suggestionType === "var-ref" && filteredVars.length === 0)) && (
            <div style={{ padding: "6px 10px", color: "#9CA3AF", fontSize: 11 }}>
              无匹配项
            </div>
          )}
        </div>
      )}
    </div>
  );
}

/** Render segments as a display-only text string (for the raw input view). */
function segmentsToDisplay(segments: StyleValueSegment[]): string {
  return segments.map((s) => {
    if (s.type === "text") return s.value;
    if (s.type === "signal") return `@${s.label}`;
    if (s.type === "var-ref") return `var(${s.name})`;
    return "";
  }).join("");
}
