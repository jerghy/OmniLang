/** NodePalette — Unity VS style right-click search popup for adding nodes.
 *  Replaces the old 3-item context menu with a full searchable palette
 *  grouping all registered node types by category. */

import { useState, useCallback, useEffect, useRef, useMemo } from "react";
import { getAllNodeConfigs, CATEGORY_META, type NodeTypeConfig } from "./nodeRegistry";

export interface NodePaletteProps {
  /** Canvas position where the palette should appear (relative to FlowCanvas container). */
  position: { x: number; y: number };
  /** Called when the user selects a node type. */
  onSelect: (config: NodeTypeConfig) => void;
  /** Called to close the palette. */
  onClose: () => void;
}

/** Fuzzy match a query against a config's keywords + label. Returns 0-1 score. */
function fuzzyScore(query: string, config: NodeTypeConfig): number {
  const lower = query.toLowerCase().trim();
  if (!lower) return 0.5; // show all when no query, but sort by category order

  // Exact label match gets highest score
  if (config.label.toLowerCase() === lower) return 2;
  // Label starts with query
  if (config.label.toLowerCase().startsWith(lower)) return 1.5;
  // Label contains query
  if (config.label.toLowerCase().includes(lower)) return 1;

  // Keyword match — each matched keyword adds 1
  let score = 0;
  for (const kw of config.keywords) {
    if (kw.toLowerCase() === lower) score += 1;
    else if (kw.toLowerCase().startsWith(lower)) score += 0.8;
    else if (kw.toLowerCase().includes(lower)) score += 0.5;
  }
  return score;
}

export function NodePalette({ position, onSelect, onClose }: NodePaletteProps) {
  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  // Auto-focus the search input
  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  // Close on Escape
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        onClose();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [onClose]);

  // Close on click outside
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      if (!target.closest("[data-node-palette]")) {
        onClose();
      }
    };
    // Delay to avoid the right-click event itself triggering the close
    const timer = setTimeout(() => window.addEventListener("click", handler), 0);
    return () => {
      clearTimeout(timer);
      window.removeEventListener("click", handler);
    };
  }, [onClose]);

  // Filter and group results
  const groupedResults = useMemo(() => {
    const all = getAllNodeConfigs();
    const scored = all
      .map((c) => ({ config: c, score: fuzzyScore(query, c) }))
      .filter((s) => s.score > 0)
      .sort((a, b) => b.score - a.score);

    // Group by category, preserving category order
    const groups = new Map<string, NodeTypeConfig[]>();
    for (const { config } of scored) {
      const cat = config.category;
      if (!groups.has(cat)) groups.set(cat, []);
      groups.get(cat)!.push(config);
    }

    // Sort groups by category order
    const sorted = Array.from(groups.entries()).sort(
      (a, b) => (CATEGORY_META[a[0]]?.order ?? 99) - (CATEGORY_META[b[0]]?.order ?? 99),
    );

    return sorted;
  }, [query]);

  // Flatten results for keyboard nav
  const flatResults = useMemo(
    () => groupedResults.flatMap(([, configs]) => configs),
    [groupedResults],
  );

  // Clamp selected index
  const safeIndex = Math.max(0, Math.min(selectedIndex, flatResults.length - 1));

  // Scroll selected item into view
  useEffect(() => {
    const el = listRef.current?.querySelector(`[data-palette-index="${safeIndex}"]`);
    el?.scrollIntoView({ block: "nearest" });
  }, [safeIndex]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      switch (e.key) {
        case "ArrowDown":
          e.preventDefault();
          setSelectedIndex((i) => Math.min(i + 1, flatResults.length - 1));
          break;
        case "ArrowUp":
          e.preventDefault();
          setSelectedIndex((i) => Math.max(i - 1, 0));
          break;
        case "Enter":
          e.preventDefault();
          if (flatResults[safeIndex]) {
            onSelect(flatResults[safeIndex]);
          }
          break;
      }
    },
    [flatResults, safeIndex, onSelect],
  );

  return (
    <div
      data-node-palette
      style={{
        position: "absolute",
        left: position.x,
        top: position.y,
        zIndex: 2000,
        width: 320,
        maxHeight: 420,
        background: "#FFFFFF",
        borderRadius: 12,
        boxShadow: "0 8px 32px rgba(0,0,0,0.18), 0 0 0 1px rgba(0,0,0,0.06)",
        display: "flex",
        flexDirection: "column",
        overflow: "hidden",
        fontFamily: "system-ui, sans-serif",
        fontSize: 13,
      }}
    >
      {/* Search input */}
      <div style={{ padding: "10px 12px", borderBottom: "1px solid #F3F4F6" }}>
        <input
          ref={inputRef}
          type="text"
          placeholder="🔍 搜索节点..."
          value={query}
          onChange={(e) => {
            setQuery(e.target.value);
            setSelectedIndex(0);
          }}
          onKeyDown={handleKeyDown}
          style={{
            width: "100%",
            border: "none",
            outline: "none",
            fontSize: 14,
            padding: "6px 8px",
            background: "#F9FAFB",
            borderRadius: 8,
            boxSizing: "border-box",
          }}
        />
      </div>

      {/* Results list */}
      <div
        ref={listRef}
        style={{
          flex: 1,
          overflowY: "auto",
          padding: "6px 0",
        }}
      >
        {groupedResults.length === 0 && (
          <div
            style={{
              padding: "16px",
              textAlign: "center",
              color: "#9CA3AF",
              fontSize: 12,
            }}
          >
            未找到匹配的节点
          </div>
        )}

        {groupedResults.map(([cat, configs]) => {
          const meta = CATEGORY_META[cat];
          let globalIdx = 0;
          // Calculate starting index for this group
          for (const [, prevConfigs] of groupedResults) {
            if (prevConfigs === configs) break;
            globalIdx += prevConfigs.length;
          }

          return (
            <div key={cat}>
              {/* Category header */}
              <div
                style={{
                  padding: "4px 12px",
                  color: "#9CA3AF",
                  fontSize: 10,
                  fontWeight: 600,
                  textTransform: "uppercase",
                  letterSpacing: "0.5px",
                  marginTop: globalIdx > 0 ? 4 : 0,
                }}
              >
                {meta?.icon} {meta?.label || cat}
              </div>

              {configs.map((config, i) => {
                const idx = globalIdx + i;
                const isSelected = idx === safeIndex;

                return (
                  <div
                    key={config.tag}
                    data-palette-index={idx}
                    onClick={() => onSelect(config)}
                    onMouseEnter={() => setSelectedIndex(idx)}
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: 10,
                      padding: "8px 12px",
                      cursor: "pointer",
                      background: isSelected ? "#F3F4F6" : "transparent",
                      transition: "background 0.08s",
                    }}
                  >
                    {/* Icon */}
                    <span
                      style={{
                        fontSize: 16,
                        width: 28,
                        height: 28,
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "center",
                        background: config.color.bg,
                        borderRadius: 6,
                        border: `1.5px solid ${config.color.border}`,
                        flexShrink: 0,
                      }}
                    >
                      {config.icon}
                    </span>

                    {/* Label + description */}
                    <div style={{ minWidth: 0 }}>
                      <div style={{ fontWeight: 600, color: "#1F2937", fontSize: 13, lineHeight: 1.3 }}>
                        {config.label}
                      </div>
                      {config.description && (
                        <div style={{ fontSize: 11, color: "#9CA3AF", marginTop: 1 }}>
                          {config.description}
                        </div>
                      )}
                    </div>

                    {/* Port preview (abbreviated) */}
                    <div style={{ marginLeft: "auto", display: "flex", gap: 3, flexShrink: 0 }}>
                      {config.defaultPorts.filter((p) => p.direction === "in").slice(0, 3).map((p) => (
                        <span
                          key={p.name}
                          title={`in: ${p.name} (${p.portType})`}
                          style={{
                            width: 6,
                            height: 6,
                            borderRadius: p.portType === "flow" ? 1 : "50%",
                            background: p.portType === "flow" ? "#9CA3AF" : "#2563EB",
                            transform: p.portType === "flow" ? "rotate(45deg)" : "none",
                          }}
                        />
                      ))}
                      {config.defaultPorts.filter((p) => p.direction === "out").slice(0, 3).map((p) => (
                        <span
                          key={p.name}
                          title={`out: ${p.name} (${p.portType})`}
                          style={{
                            width: 6,
                            height: 6,
                            borderRadius: p.portType === "flow" ? 1 : "50%",
                            background: p.portType === "flow" ? "#6366F1" : "#059669",
                            transform: p.portType === "flow" ? "rotate(45deg)" : "none",
                            marginLeft: 4,
                          }}
                        />
                      ))}
                    </div>
                  </div>
                );
              })}
            </div>
          );
        })}
      </div>

      {/* Footer hint */}
      <div
        style={{
          padding: "6px 12px",
          borderTop: "1px solid #F3F4F6",
          color: "#D1D5DB",
          fontSize: 10,
          display: "flex",
          gap: 12,
        }}
      >
        <span>↑↓ 导航</span>
        <span>↵ 选择</span>
        <span>Esc 关闭</span>
      </div>
    </div>
  );
}
