/**
 * Debug Panel — live runtime state inspection for Phase 2 Runtime Bridge.
 *
 * Shows real-time signal/computed/store values from the running app,
 * connection status, state change timeline, and breakpoint controls.
 *
 * Protocol: receives messages via WebSocket relayed by api/client.ts
 *   { type: "state-change", signalId, name, value, timestamp }
 *   { type: "runtime-ready" }
 *   { type: "runtime-disconnected" }
 *   { type: "execution-trace", nodeId, edgeId?, timestamp }
 */

import React, { useState, useEffect, useRef } from "react";

// ── Types ────────────────────────────────────────────────

interface StateChange {
  signalId: string;
  name: string;
  value: unknown;
  timestamp: number;
}

interface DebugPanelProps {
  connected: boolean;
  stateChanges: StateChange[];
  onClear: () => void;
  onToggleBreakpoint: (signalId: string) => void;
  breakpoints: Set<string>;
  paused?: boolean;
  onStep?: () => void;
  onResume?: () => void;
}

// ── Component ────────────────────────────────────────────

export function DebugPanel({
  connected,
  stateChanges,
  onClear,
  onToggleBreakpoint,
  breakpoints,
  paused = false,
  onStep,
  onResume,
}: DebugPanelProps) {
  const [collapsed, setCollapsed] = useState(false);
  const [filter, setFilter] = useState("");
  const [selectedSignal, setSelectedSignal] = useState<string | null>(null);
  const timelineEndRef = useRef<HTMLDivElement>(null);

  // Auto-scroll timeline to latest
  useEffect(() => {
    if (timelineEndRef.current) {
      timelineEndRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [stateChanges.length]);

  // Group state changes by signal name for the variable watch
  const signalValues = new Map<string, StateChange>();
  for (const change of stateChanges) {
    signalValues.set(change.signalId, change);
  }

  // Filter signals
  const filteredSignals = Array.from(signalValues.entries()).filter(
    ([id, sc]) => {
      if (!filter) return true;
      const q = filter.toLowerCase();
      return id.toLowerCase().includes(q) || sc.name.toLowerCase().includes(q);
    }
  );

  // Show last N state changes for timeline
  const recentChanges = stateChanges.slice(-20).reverse();

  if (collapsed) {
    return (
      <div style={collapsedBarStyle}>
        <button
          onClick={() => setCollapsed(false)}
          style={expandButtonStyle}
          title="展开调试面板"
        >
          🐞 调试
          <span
            style={{
              display: "inline-block",
              width: 8,
              height: 8,
              borderRadius: "50%",
              background: connected ? "#10B981" : "#EF4444",
              marginLeft: 6,
            }}
          />
        </button>
      </div>
    );
  }

  return (
    <div style={panelStyle}>
      {/* Header */}
      <div style={headerStyle}>
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <span style={{ fontSize: 16 }}>🐞</span>
          <span style={{ fontWeight: 700, fontSize: 13 }}>调试面板</span>
          <span
            style={{
              fontSize: 10,
              padding: "2px 8px",
              borderRadius: 10,
              background: connected ? "#065F46" : "#7F1D1D",
              color: connected ? "#6EE7B7" : "#FCA5A5",
            }}
          >
            {connected ? "● 已连接" : "○ 未连接"}
          </span>
          {paused && (
            <span
              style={{
                fontSize: 10,
                padding: "2px 8px",
                borderRadius: 10,
                background: "#78350F",
                color: "#FDE68A",
              }}
            >
              ⏸ PAUSED
            </span>
          )}
        </div>
        <div style={{ display: "flex", gap: 4, alignItems: "center" }}>
          {paused && (
            <>
              <button onClick={onStep} style={stepButtonStyle} title="单步执行">
                ⏭ Step
              </button>
              <button onClick={onResume} style={resumeButtonStyle} title="继续执行">
                ▶ Resume
              </button>
            </>
          )}
          <button onClick={onClear} style={smallButtonStyle} title="清除历史">
            🗑
          </button>
          <button
            onClick={() => setCollapsed(true)}
            style={smallButtonStyle}
            title="收起面板"
          >
            ✕
          </button>
        </div>
      </div>

      {/* Toolbar: filter */}
      <div style={{ padding: "4px 8px" }}>
        <input
          type="text"
          placeholder="过滤信号..."
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          style={filterInputStyle}
        />
      </div>

      {/* Variable watch */}
      <div style={sectionStyle}>
        <div style={sectionHeaderStyle}>
          📊 变量监视
          <span style={{ fontSize: 9, color: "#6B7280" }}>
            {filteredSignals.length} 个信号
          </span>
        </div>
        <div style={{ maxHeight: 200, overflowY: "auto" }}>
          {filteredSignals.length > 0 ? (
            filteredSignals.map(([sigId, sc]) => {
              const isSelected = selectedSignal === sigId;
              const hasBreakpoint = breakpoints.has(sigId);
              return (
                <div
                  key={sigId}
                  onClick={() =>
                    setSelectedSignal(isSelected ? null : sigId)
                  }
                  style={{
                    ...signalRowStyle,
                    background: isSelected ? "#374151" : "transparent",
                  }}
                >
                  <div
                    style={{ display: "flex", alignItems: "center", gap: 4 }}
                  >
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        onToggleBreakpoint(sigId);
                      }}
                      style={{
                        ...breakpointDotStyle,
                        background: hasBreakpoint ? "#EF4444" : "#4B5563",
                      }}
                      title={
                        hasBreakpoint
                          ? "移除断点"
                          : "设置断点"
                      }
                    />
                    <span style={signalNameStyle}>{sc.name}</span>
                    <span style={signalTypeStyle}>
                      {sigId}
                    </span>
                  </div>
                  <code style={signalValueStyle}>{formatDebugValue(sc.value)}</code>
                </div>
              );
            })
          ) : (
            <div style={emptyStyle}>
              {connected ? "等待状态变化..." : "未运行 — 点击 ▶ 运行 启动调试"}
            </div>
          )}
        </div>
      </div>

      {/* State timeline */}
      <div style={sectionStyle}>
        <div style={sectionHeaderStyle}>
          ⏱ 状态时间线
          <span style={{ fontSize: 9, color: "#6B7280" }}>
            {stateChanges.length} 条记录
          </span>
        </div>
        <div style={{ maxHeight: 150, overflowY: "auto" }}>
          {recentChanges.length > 0 ? (
            recentChanges.map((sc, i) => (
              <div
                key={`${sc.signalId}-${i}-${sc.timestamp}`}
                style={timelineRowStyle}
                onClick={() => setSelectedSignal(sc.signalId)}
              >
                <span style={{ fontSize: 9, color: "#6B7280", minWidth: 50 }}>
                  {formatTime(sc.timestamp)}
                </span>
                <span style={signalNameStyle}>{sc.name}</span>
                <code style={timelineValueStyle}>
                  {formatDebugValue(sc.value)}
                </code>
              </div>
            ))
          ) : (
            <div style={emptyStyle}>暂无变化</div>
          )}
          <div ref={timelineEndRef} />
        </div>
      </div>

      {/* Selected signal detail */}
      {selectedSignal && signalValues.has(selectedSignal) && (
        <div style={sectionStyle}>
          <div style={sectionHeaderStyle}>🔍 信号详情: {selectedSignal}</div>
          <div style={{ padding: "4px 8px", fontSize: 11 }}>
            <pre style={detailPreStyle}>
              {JSON.stringify(
                signalValues.get(selectedSignal)?.value,
                null,
                2
              )}
            </pre>
          </div>
        </div>
      )}
    </div>
  );
}

// ── Helpers ──────────────────────────────────────────────

function formatDebugValue(value: unknown): string {
  if (value === null || value === undefined) return "null";
  if (typeof value === "boolean") return value ? "true" : "false";
  if (typeof value === "string") {
    return value.length > 30 ? `"${value.slice(0, 30)}…"` : `"${value}"`;
  }
  if (typeof value === "number") return String(value);
  if (typeof value === "object") {
    try {
      const s = JSON.stringify(value);
      return s.length > 50 ? s.slice(0, 50) + "…" : s;
    } catch {
      return "[object]";
    }
  }
  return String(value);
}

function formatTime(ts: number): string {
  const d = new Date(ts);
  return `${pad(d.getMinutes())}:${pad(d.getSeconds())}.${String(d.getMilliseconds()).slice(0, 2)}`;
}

function pad(n: number): string {
  return String(n).padStart(2, "0");
}

// ── Styles ───────────────────────────────────────────────

const panelStyle: React.CSSProperties = {
  position: "absolute",
  bottom: 8,
  right: 8,
  zIndex: 100,
  width: 340,
  maxHeight: "60vh",
  background: "#1F2937",
  color: "#F9FAFB",
  borderRadius: 10,
  boxShadow: "0 8px 24px rgba(0,0,0,0.5)",
  fontSize: 11,
  overflow: "hidden",
  display: "flex",
  flexDirection: "column",
  pointerEvents: "auto",
  animation: "omnilang-debug-slidein 200ms ease-out",
};

const collapsedBarStyle: React.CSSProperties = {
  position: "absolute",
  bottom: 8,
  right: 8,
  zIndex: 100,
  pointerEvents: "auto",
};

const headerStyle: React.CSSProperties = {
  display: "flex",
  justifyContent: "space-between",
  alignItems: "center",
  padding: "8px 10px",
  borderBottom: "1px solid #374151",
  background: "#111827",
};

const sectionStyle: React.CSSProperties = {
  borderTop: "1px solid #374151",
};

const sectionHeaderStyle: React.CSSProperties = {
  padding: "6px 10px",
  fontSize: 10,
  fontWeight: 600,
  color: "#9CA3AF",
  textTransform: "uppercase",
  letterSpacing: "0.5px",
  display: "flex",
  justifyContent: "space-between",
  alignItems: "center",
};

const signalRowStyle: React.CSSProperties = {
  display: "flex",
  justifyContent: "space-between",
  alignItems: "center",
  padding: "4px 10px",
  cursor: "pointer",
  borderBottom: "1px solid #374151",
};

const signalNameStyle: React.CSSProperties = {
  fontWeight: 600,
  color: "#A5B4FC",
  fontSize: 11,
};

const signalTypeStyle: React.CSSProperties = {
  fontSize: 9,
  color: "#6B7280",
  fontFamily: "monospace",
  marginLeft: 4,
};

const signalValueStyle: React.CSSProperties = {
  fontSize: 11,
  color: "#10B981",
  fontFamily: "'Cascadia Code', monospace",
  background: "#111827",
  padding: "1px 6px",
  borderRadius: 3,
  maxWidth: 130,
  overflow: "hidden",
  textOverflow: "ellipsis",
  whiteSpace: "nowrap",
};

const timelineRowStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: 6,
  padding: "2px 10px",
  cursor: "pointer",
  borderBottom: "1px solid #1F2937",
};

const timelineValueStyle: React.CSSProperties = {
  fontSize: 10,
  color: "#6EE7B7",
  fontFamily: "'Cascadia Code', monospace",
  maxWidth: 120,
  overflow: "hidden",
  textOverflow: "ellipsis",
  whiteSpace: "nowrap",
};

const emptyStyle: React.CSSProperties = {
  padding: "12px 10px",
  color: "#6B7280",
  fontSize: 11,
  textAlign: "center",
};

const filterInputStyle: React.CSSProperties = {
  width: "100%",
  padding: "4px 8px",
  borderRadius: 4,
  border: "1px solid #374151",
  background: "#111827",
  color: "#F9FAFB",
  fontSize: 11,
  outline: "none",
  boxSizing: "border-box",
};

const stepButtonStyle: React.CSSProperties = {
  padding: "2px 8px",
  fontSize: 10,
  border: "1px solid #F59E0B",
  borderRadius: 4,
  background: "#78350F",
  color: "#FDE68A",
  cursor: "pointer",
  fontWeight: 600,
};

const resumeButtonStyle: React.CSSProperties = {
  padding: "2px 8px",
  fontSize: 10,
  border: "1px solid #10B981",
  borderRadius: 4,
  background: "#064E3B",
  color: "#6EE7B7",
  cursor: "pointer",
  fontWeight: 600,
};

const smallButtonStyle: React.CSSProperties = {
  background: "none",
  border: "none",
  color: "#9CA3AF",
  cursor: "pointer",
  fontSize: 12,
  padding: "2px 4px",
  lineHeight: 1,
};

const expandButtonStyle: React.CSSProperties = {
  background: "#1F2937",
  color: "#F9FAFB",
  border: "1px solid #374151",
  borderRadius: 6,
  padding: "6px 12px",
  fontSize: 12,
  cursor: "pointer",
  display: "flex",
  alignItems: "center",
  gap: 4,
  boxShadow: "0 4px 12px rgba(0,0,0,0.3)",
};

const breakpointDotStyle: React.CSSProperties = {
  width: 8,
  height: 8,
  borderRadius: "50%",
  border: "none",
  cursor: "pointer",
  padding: 0,
  flexShrink: 0,
};

const detailPreStyle: React.CSSProperties = {
  margin: 0,
  fontSize: 10,
  color: "#D1D5DB",
  fontFamily: "'Cascadia Code', monospace",
  whiteSpace: "pre-wrap",
  wordBreak: "break-all",
  maxHeight: 150,
  overflow: "auto",
};
