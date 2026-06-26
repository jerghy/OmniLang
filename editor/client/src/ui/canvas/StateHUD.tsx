/**
 * 状态检查 HUD — 选中元素时显示绑定的 signal/store 实时值。
 *
 * 通信协议：
 *   编辑器 → 预览 iframe: postMessage({ type: "omni:get-state", elementId })
 *   预览 iframe → 编辑器: postMessage({ type: "omni:state", values: {...} })
 */

import React, { useState, useEffect, useRef } from "react";
import { useEditorStore } from "../../store/editor";

interface StateHUDProps {
  elementId: string;
  visible: boolean;
  onClose: () => void;
}

interface SignalEntry {
  name: string;
  value: unknown;
  type: string;
}

/** A single point in signal history. */
interface HistoryPoint {
  time: number;    // Date.now()
  value: unknown;
}

export function StateHUD({ elementId, visible, onClose }: StateHUDProps) {
  const [signals, setSignals] = useState<SignalEntry[]>([]);
  const [connected, setConnected] = useState(false);
  const [showTimeline, setShowTimeline] = useState(false);
  const intervalRef = useRef<number | null>(null);
  const historyRef = useRef<Map<string, HistoryPoint[]>>(new Map());
  const elements = useEditorStore((s) => s.elements);

  const el = elements.find((e) => e.id === elementId);

  useEffect(() => {
    if (!visible) {
      if (intervalRef.current !== null) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
      return;
    }

    // 收集该元素（及 codeblock 关联）的 signal 名称
    const elSignals = collectElementSignals(elementId, elements);

    const iframe = document.querySelector("iframe") as HTMLIFrameElement | null;
    if (!iframe?.contentWindow) {
      // 无预览 iframe → 显示静态绑定信息
      setSignals(
        elSignals.map((s) => ({ name: s, value: "(未运行)", type: "unknown" }))
      );
      return;
    }

    // 获取 iframe 的 origin 用于安全通信
    const previewOrigin = new URL(iframe.src || window.location.href).origin;

    // 定期轮询预览 iframe 中的运行时状态
    const poll = () => {
      iframe.contentWindow?.postMessage(
        { type: "omni:get-state", elementId, signals: elSignals },
        previewOrigin,  // 限制为预览 iframe 的 origin
      );
    };

    poll();
    intervalRef.current = window.setInterval(poll, 1000);

    const handler = (e: MessageEvent) => {
      // 安全: 验证消息来源
      if (e.origin !== previewOrigin) return;

      if (e.data?.type === "omni:state" && e.data?.elementId === elementId) {
        setConnected(true);
        const values = e.data.values as Record<string, unknown>;
        if (values && typeof values === "object") {
          const now = Date.now();
          const history = historyRef.current;
          const newSignals = Object.entries(values).map(([name, value]) => {
            // Track history (max 50 entries per signal)
            const points = history.get(name) || [];
            const last = points[points.length - 1];
            if (!last || JSON.stringify(last.value) !== JSON.stringify(value)) {
              points.push({ time: now, value });
              if (points.length > 50) points.shift();
            }
            history.set(name, points);
            return { name, value, type: typeof value };
          });
          setSignals(newSignals);
        }
      }
      // 心跳确认
      if (e.data?.type === "omni:ready") {
        setConnected(true);
      }
    };

    window.addEventListener("message", handler);
    poll(); // 立即再请求一次

    return () => {
      window.removeEventListener("message", handler);
      if (intervalRef.current !== null) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
    };
  }, [elementId, visible, elements]);

  if (!visible) return null;

  return (
    <div
      style={{
        position: "absolute",
        top: 8,
        right: 8,
        zIndex: 100,
        minWidth: 220,
        maxWidth: 320,
        padding: 12,
        borderRadius: 10,
        background: "#1F2937",
        color: "#F9FAFB",
        fontSize: 12,
        boxShadow: "0 8px 24px rgba(0,0,0,0.4)",
        pointerEvents: "auto",
        animation: "omnilang-hud-fadein 200ms ease-out",
      }}
    >
      {/* Header */}
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          marginBottom: 8,
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
          <span style={{ fontSize: 14 }}>🔍</span>
          <span style={{ fontWeight: 700, fontSize: 13 }}>状态检查</span>
          {el && (
            <span
              style={{
                fontSize: 10,
                color: "#9CA3AF",
                background: "#374151",
                padding: "1px 6px",
                borderRadius: 3,
              }}
            >
              {el.tag}
              {el.props["name"] ? ` ${el.props["name"]}` : ""}
            </span>
          )}
        </div>
        <button
          onClick={onClose}
          style={{
            background: "none",
            border: "none",
            color: "#9CA3AF",
            cursor: "pointer",
            fontSize: 14,
            padding: "2px 4px",
            lineHeight: 1,
          }}
        >
          ✕
        </button>
      </div>

      {/* Connection status */}
      {!connected && (
        <div
          style={{
            fontSize: 10,
            color: "#F59E0B",
            marginBottom: 6,
            display: "flex",
            alignItems: "center",
            gap: 4,
          }}
        >
          <span>🟡</span> 等待运行时连接...
        </div>
      )}

      {/* Signal values */}
      {/* Timeline toggle */}
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <span style={{ fontSize: 9, color: "#9CA3AF" }}>实时值</span>
        <button
          onClick={() => setShowTimeline(!showTimeline)}
          style={{
            background: showTimeline ? "#4F46E5" : "transparent",
            color: showTimeline ? "#FFF" : "#9CA3AF",
            border: "1px solid", borderColor: showTimeline ? "#4F46E5" : "#4B5563",
            borderRadius: 4, fontSize: 9, padding: "1px 6px", cursor: "pointer",
          }}
        >
          {showTimeline ? "⏱ 隐藏" : "⏱ 历史"}
        </button>
      </div>
      {/* Timeline view */}
      {showTimeline && (
        <div style={{ maxHeight: 100, overflowY: "auto", borderTop: "1px solid #374151", paddingTop: 4, paddingBottom: 4 }}>
          {signals.map((sig) => {
            const points = historyRef.current.get(sig.name) || [];
            if (points.length <= 1) return null;
            return (
              <div key={'hist-' + sig.name} style={{ marginBottom: 2 }}>
                <div style={{ fontSize: 8, color: "#6B7280" }}>{sig.name}: {points.length} changes</div>
                <div style={{ display: "flex", gap: 2, flexWrap: "wrap" }}>
                  {points.slice(-8).map((p, i) => {
                    const prev = points.slice(-9)[i];
                    const changed = !prev || String(prev?.value) !== String(p.value);
                    const ago = Math.round((Date.now() - p.time) / 1000);
                    return (
                      <span key={i} title={ago + 's ago'}
                        style={{ fontSize: 7, padding: '1px 3px', borderRadius: 2,
                          background: changed ? '#4F46E5' : '#374151',
                          color: changed ? '#FFF' : '#9CA3AF' }}>
                        {String(p.value).slice(0, 8)}
                      </span>
                    );
                  })}
                </div>
              </div>
            );
          })}
        </div>
      )}
      {signals.length > 0 ? (
        <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
          {signals.map((sig) => (
            <div
              key={sig.name}
              style={{
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
                padding: "4px 8px",
                background: "#374151",
                borderRadius: 6,
              }}
            >
              <span style={{ fontWeight: 600, color: "#A5B4FC" }}>
                {sig.name}
              </span>
              <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                <span
                  style={{ fontSize: 10, color: "#6B7280", fontFamily: "monospace" }}
                >
                  {sig.type}
                </span>
                <code
                  style={{
                    fontSize: 12,
                    color: "#10B981",
                    fontFamily: "'Cascadia Code', monospace",
                    background: "#1F2937",
                    padding: "2px 6px",
                    borderRadius: 3,
                    maxWidth: 140,
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                  }}
                >
                  {formatValue(sig.value)}
                </code>
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div style={{ color: "#6B7280", fontSize: 11, textAlign: "center", padding: "8px 0" }}>
          该元素无绑定 signal
        </div>
      )}

      {/* Help */}
      <div
        style={{
          marginTop: 8,
          fontSize: 10,
          color: "#4B5563",
          borderTop: "1px solid #374151",
          paddingTop: 6,
        }}
      >
        提示：在 .omni 文件中用 <code style={{ color: "#A5B4FC" }}>{"{signalName}"}</code> 绑定状态
      </div>
    </div>
  );
}

// ── Helpers ────────────────────────────────────────

function collectElementSignals(
  elementId: string,
  elements: import("../store/editor").ElementData[]
): string[] {
  const el = elements.find((e) => e.id === elementId);
  if (!el) return [];

  const signalNames: string[] = [];

  // 检查 props 中是否有 {} 插值引用 signal
  for (const value of Object.values(el.props)) {
    if (typeof value === "string") {
      const matches = value.match(/\{([\w.]+)\}/g);
      if (matches) {
        for (const m of matches) {
          const name = m.slice(1, -1);
          if (!signalNames.includes(name)) signalNames.push(name);
        }
      }
    }
  }

  // 检查关联的 codeblock signals
  const codeBlocks = elements.filter(
    (e) =>
      e.nodeType === "codeblock" &&
      (e.props["name"] === el.tag || e.parentId === el.id)
  );
  for (const cb of codeBlocks) {
    const name = cb.props["name"] as string;
    if (name && !signalNames.includes(name)) signalNames.push(name);
  }

  return signalNames;
}

function formatValue(value: unknown): string {
  if (value === null || value === undefined) return "null";
  if (typeof value === "string") return `"${value}"`;
  if (typeof value === "boolean") return value ? "true" : "false";
  if (typeof value === "object") {
    try {
      return JSON.stringify(value);
    } catch {
      return "[object]";
    }
  }
  return String(value);
}
