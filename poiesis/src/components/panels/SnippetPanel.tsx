/**
 * 代码片段面板 — 按上下文展示可用代码片段。
 *
 * 从 Monaco 编辑器光标位置推断当前上下文（fn_body / view / store_action 等），
 * 展示对应分类的片段。点击片段自动插入到编辑器光标处。
 */

import { useState, useMemo } from "react";
import { BUILTIN_SNIPPETS, CONTEXT_LABELS, CONTEXT_ICONS, type SnippetContext, type OmniSnippet } from "../../snippets/builtin";

interface SnippetPanelProps {
  /** The full source code, used for context detection */
  source: string;
  /** Current cursor position (0-based offset into source) */
  cursorOffset: number;
  /** Callback: insert snippet body at cursor */
  onInsert: (snippetBody: string) => void;
}

export function SnippetPanel({ source, cursorOffset, onInsert }: SnippetPanelProps) {
  const [search, setSearch] = useState("");
  const [expandedContext, setExpandedContext] = useState<SnippetContext | null>(null);

  const currentContext = useMemo(() => detectContext(source, cursorOffset), [source, cursorOffset]);

  // Group snippets by context, filter by search
  const grouped = useMemo(() => {
    const map = new Map<SnippetContext, OmniSnippet[]>();
    for (const s of BUILTIN_SNIPPETS) {
      const list = map.get(s.context) || [];
      if (!search || s.label.toLowerCase().includes(search.toLowerCase()) || s.description.toLowerCase().includes(search.toLowerCase())) {
        list.push(s);
        map.set(s.context, list);
      }
    }
    // Sort: current context first
    const entries = Array.from(map.entries());
    entries.sort((a, b) => {
      if (a[0] === currentContext) return -1;
      if (b[0] === currentContext) return 1;
      return 0;
    });
    return entries;
  }, [search, currentContext]);

  return (
    <div style={{ height: "100%", display: "flex", flexDirection: "column", fontSize: 12 }}>
      {/* Header */}
      <div style={{ padding: "6px 8px", borderBottom: "1px solid #e5e7eb", fontWeight: 600, fontSize: 11, color: "#6B7280", textTransform: "uppercase", display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <span>💡 代码片段</span>
        <span style={{ fontSize: 10, color: "#9CA3AF", textTransform: "none", fontWeight: 400 }}>
          {CONTEXT_LABELS[currentContext]}
        </span>
      </div>

      {/* Search */}
      <div style={{ padding: "4px 8px" }}>
        <input
          type="text"
          placeholder="搜索片段..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          style={{ width: "100%", padding: "3px 6px", fontSize: 11, border: "1px solid #e5e7eb", borderRadius: 4, outline: "none", boxSizing: "border-box" }}
        />
      </div>

      {/* Snippet list by context groups */}
      <div style={{ flex: 1, overflow: "auto", padding: "0 4px 4px" }}>
        {grouped.map(([ctx, snippets]) => {
          const isExpanded = expandedContext === ctx || !search; // auto-expand when not searching
          const isCurrent = ctx === currentContext;
          return (
            <div key={ctx} style={{ marginBottom: 4 }}>
              {/* Context header */}
              <div
                onClick={() => setExpandedContext(isExpanded ? null : ctx)}
                style={{
                  padding: "3px 6px", cursor: "pointer", fontSize: 11, fontWeight: isCurrent ? 600 : 400,
                  color: isCurrent ? "#4F46E5" : "#6B7280", borderRadius: 3,
                  background: isCurrent ? "#EEF2FF" : "transparent",
                  display: "flex", justifyContent: "space-between", alignItems: "center",
                }}
              >
                <span>{CONTEXT_ICONS[ctx]} {CONTEXT_LABELS[ctx]}</span>
                <span style={{ fontSize: 10, color: "#D1D5DB" }}>{snippets.length}</span>
              </div>
              {/* Snippet items */}
              {isExpanded && snippets.map((s) => (
                <div
                  key={s.id}
                  onClick={() => onInsert(s.body)}
                  title={s.description}
                  style={{
                    padding: "4px 8px 4px 18px", cursor: "pointer", fontSize: 11, borderRadius: 3,
                    marginBottom: 1,
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.background = "#F3F4F6";
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.background = "transparent";
                  }}
                >
                  <div style={{ fontWeight: 500, color: "#1F2937" }}>{s.label}</div>
                  <div style={{ color: "#9CA3AF", fontSize: 10 }}>{s.description}</div>
                </div>
              ))}
            </div>
          );
        })}
        {grouped.length === 0 && (
          <div style={{ padding: 12, textAlign: "center", color: "#D1D5DB", fontSize: 11 }}>
            没有匹配的片段
          </div>
        )}
      </div>

      {/* Help text */}
      <div style={{ padding: "4px 8px", borderTop: "1px solid #e5e7eb", fontSize: 10, color: "#D1D5DB" }}>
        点击插入 · Tab 跳到占位符
      </div>
    </div>
  );
}

// ── Context Detection ────────────────────────────

function detectContext(source: string, offset: number): SnippetContext {
  const prefix = source.substring(0, offset);

  // Walk backwards from cursor, tracking brace depth and nearest declaration
  let depth = 0;
  let i = offset - 1;

  // Track what block we're inside
  const foundBlocks: { type: string; depth: number }[] = [];

  for (; i >= 0; i--) {
    const ch = prefix[i];
    if (ch === "}") depth++;
    else if (ch === "{") depth--;
  }

  // Scan forward from the start to find enclosing block types
  depth = 0;
  for (let j = 0; j < offset; j++) {
    const remaining = prefix.substring(j);
    // Detect block openings
    const fnMatch = remaining.match(/^\bfn\b\s+\w+\s*\([^)]*\)\s*\{/);
    const actionMatch = remaining.match(/^\baction\b\s+\w+\s*\([^)]*\)\s*\{/);
    const computedMatch = remaining.match(/^\bcomputed\b\s+\w+\s*\{/);
    const viewMatch = remaining.match(/^\bview\b\s*\{/);
    const storeMatch = remaining.match(/^\bstore\b\s+\w+\s*\{/);

    if (fnMatch && prefix[j] === "f") {
      foundBlocks.push({ type: "fn_body", depth });
    } else if (actionMatch && prefix[j] === "a") {
      foundBlocks.push({ type: "store_action", depth });
    } else if (computedMatch && prefix[j] === "c") {
      foundBlocks.push({ type: "computed", depth });
    } else if (viewMatch && prefix[j] === "v") {
      foundBlocks.push({ type: "view_interpolation", depth });
    } else if (storeMatch && prefix[j] === "s") {
      foundBlocks.push({ type: "state_mgmt", depth });
    }

    if (prefix[j] === "{") depth++;
    else if (prefix[j] === "}") depth--;
  }

  // Return the innermost block type
  if (foundBlocks.length > 0) {
    const innermost = foundBlocks[foundBlocks.length - 1];
    return innermost.type as SnippetContext;
  }

  // Check for if condition context (inside if parens)
  const ifMatch = prefix.match(/if\s*\([^)]*$/);
  if (ifMatch) return "if_condition";

  // Data fetch: inside fn that contains "fetch"
  const lastFn = prefix.lastIndexOf("fn ");
  if (lastFn > 0 && prefix.substring(lastFn, offset).includes("fetch")) {
    return "data_fetch";
  }

  return "top_level";
}
