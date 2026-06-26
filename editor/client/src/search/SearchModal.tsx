/**
 * 统一搜索模态框 (Ctrl+Shift+F)。
 *
 * 实时搜索所有信息源：文件、组件、片段、命令、令牌、连线。
 * 支持键盘导航（↑↓ 选择，Enter 执行，Esc 关闭）。
 */

import React, { useState, useEffect, useRef, useCallback } from "react";
import { unifiedSearch, SOURCE_CONFIG } from "../services/parser/search-index";
import type { SearchResult, SearchContext, SearchSource } from "../services/parser/search-index";

interface SearchModalProps {
  visible: boolean;
  onClose: () => void;
  onSelect: (result: SearchResult) => void;
  context: SearchContext;
}

const SOURCE_ORDER: SearchSource[] = [
  "file", "component", "snippet", "command", "token", "wire", "connector",
];

export function SearchModal({ visible, onClose, onSelect, context }: SearchModalProps) {
  const [query, setQuery] = useState("");
  const [selectedIdx, setSelectedIdx] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const results = query.trim() ? unifiedSearch(query, context) : [];

  // Reset on open
  useEffect(() => {
    if (visible) {
      setQuery("");
      setSelectedIdx(0);
    }
  }, [visible]);

  // Keyboard navigation
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      switch (e.key) {
        case "ArrowDown":
          e.preventDefault();
          setSelectedIdx((i) => Math.min(i + 1, results.length - 1));
          break;
        case "ArrowUp":
          e.preventDefault();
          setSelectedIdx((i) => Math.max(i - 1, 0));
          break;
        case "Enter":
          e.preventDefault();
          if (results[selectedIdx]) {
            onSelect(results[selectedIdx]);
            onClose();
          }
          break;
        case "Escape":
          e.preventDefault();
          onClose();
          break;
      }
    },
    [results, selectedIdx, onSelect, onClose]
  );

  // Auto-scroll selected into view
  useEffect(() => {
    const el = listRef.current?.children[selectedIdx] as HTMLElement | undefined;
    el?.scrollIntoView({ block: "nearest" });
  }, [selectedIdx]);

  if (!visible) return null;

  // Group results by source
  const grouped = new Map<SearchSource, SearchResult[]>();
  for (const r of results) {
    const list = grouped.get(r.source) || [];
    list.push(r);
    grouped.set(r.source, list);
  }

  let globalIdx = 0;

  return (
    <div
      style={{
        position: "fixed",
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        background: "rgba(0,0,0,0.3)",
        display: "flex",
        alignItems: "flex-start",
        justifyContent: "center",
        paddingTop: "15vh",
        zIndex: 10000,
      }}
      onClick={onClose}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        style={{
          width: 560,
          maxHeight: "70vh",
          background: "#FFF",
          borderRadius: 12,
          boxShadow: "0 20px 60px rgba(0,0,0,0.25)",
          display: "flex",
          flexDirection: "column",
          overflow: "hidden",
        }}
      >
        {/* Search input */}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 8,
            padding: "12px 16px",
            borderBottom: "1px solid #E5E7EB",
          }}
        >
          <span style={{ fontSize: 16, color: "#9CA3AF" }}>🔍</span>
          <input
            ref={inputRef}
            autoFocus
            value={query}
            onChange={(e) => { setQuery(e.target.value); setSelectedIdx(0); }}
            onKeyDown={handleKeyDown}
            placeholder="搜索文件、组件、片段、命令…"
            style={{
              flex: 1,
              border: "none",
              outline: "none",
              fontSize: 15,
              background: "transparent",
            }}
            spellCheck={false}
          />
          <span
            style={{
              fontSize: 10,
              color: "#D1D5DB",
              background: "#F3F4F6",
              padding: "2px 6px",
              borderRadius: 3,
              fontFamily: "monospace",
            }}
          >
            ESC
          </span>
        </div>

        {/* Results */}
        <div
          ref={listRef}
          style={{ flex: 1, overflow: "auto", padding: "4px 0" }}
        >
          {results.length === 0 && query.trim() && (
            <div
              style={{
                padding: 24,
                textAlign: "center",
                color: "#9CA3AF",
                fontSize: 13,
              }}
            >
              无匹配结果
            </div>
          )}

          {results.length === 0 && !query.trim() && (
            <div
              style={{
                padding: 24,
                textAlign: "center",
                color: "#9CA3AF",
                fontSize: 13,
              }}
            >
              <div style={{ fontSize: 32, marginBottom: 8 }}>⌨️</div>
              <div>输入关键词开始搜索</div>
              <div style={{ fontSize: 10, marginTop: 4, color: "#D1D5DB" }}>
                跨文件 · 组件 · 片段 · 命令 · 令牌 · 连线 统一搜索
              </div>
            </div>
          )}

          {SOURCE_ORDER.map((source) => {
            const items = grouped.get(source);
            if (!items || items.length === 0) return null;
            const config = SOURCE_CONFIG[source];

            return (
              <div key={source}>
                {/* Group header */}
                <div
                  style={{
                    padding: "6px 16px 2px",
                    fontSize: 10,
                    fontWeight: 700,
                    color: config.color,
                    textTransform: "uppercase",
                    letterSpacing: "0.05em",
                  }}
                >
                  {config.label}
                </div>
                {items.map((r) => {
                  const idx = globalIdx++;
                  const isSelected = idx === selectedIdx;
                  return (
                    <div
                      key={r.id}
                      onClick={() => { onSelect(r); onClose(); }}
                      onMouseEnter={() => setSelectedIdx(idx)}
                      style={{
                        display: "flex",
                        alignItems: "center",
                        gap: 8,
                        padding: "6px 16px",
                        cursor: "pointer",
                        background: isSelected ? "#EEF2FF" : "transparent",
                        fontSize: 13,
                      }}
                    >
                      <span style={{ fontSize: 15 }}>{r.icon}</span>
                      <div style={{ flex: 1, minWidth: 0 }}>
                        <div
                          style={{
                            fontWeight: 600,
                            color: isSelected ? "#4F46E5" : "#111827",
                            overflow: "hidden",
                            textOverflow: "ellipsis",
                            whiteSpace: "nowrap",
                          }}
                        >
                          {r.title}
                        </div>
                        <div
                          style={{
                            fontSize: 11,
                            color: "#9CA3AF",
                            overflow: "hidden",
                            textOverflow: "ellipsis",
                            whiteSpace: "nowrap",
                          }}
                        >
                          {r.subtitle}
                        </div>
                      </div>
                      <span
                        style={{
                          fontSize: 10,
                          color: isSelected ? "#4F46E5" : "#D1D5DB",
                          fontFamily: "monospace",
                        }}
                      >
                        {r.action.label}
                      </span>
                    </div>
                  );
                })}
              </div>
            );
          })}
        </div>

        {/* Footer */}
        <div
          style={{
            padding: "6px 16px",
            borderTop: "1px solid #F3F4F6",
            display: "flex",
            gap: 12,
            fontSize: 10,
            color: "#9CA3AF",
          }}
        >
          <span>↑↓ 导航</span>
          <span>↵ 选择</span>
          <span>Esc 关闭</span>
          <span style={{ flex: 1 }} />
          <span>{results.length} 条结果</span>
        </div>
      </div>
    </div>
  );
}
