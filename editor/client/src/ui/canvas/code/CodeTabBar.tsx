/** Code mode tab bar — manages visual programming tabs.
 *  Tab types: "event" (functions appear in design event binding dropdown)
 *             "logic" (general purpose logic code) */

import React, { useState } from "react";
import { useCodeGraphStore } from "../../../codegraph/store";
import type { CodeTab, CodeTabType } from "../../../core/types";

// ── Tab Creation Dialog ──

function NewTabDialog({ onClose }: { onClose: () => void }) {
  const addTab = useCodeGraphStore((s) => s.addTab);
  const [type, setType] = useState<CodeTabType>("logic");
  const [label, setLabel] = useState("");

  const handleCreate = () => {
    const name = label.trim();
    if (!name) return;
    const icon = type === "event" ? "⚡" : "📄";
    addTab(type, name, { icon });
    onClose();
  };

  return (
    <>
      <div
        onClick={onClose}
        style={{ position: "fixed", inset: 0, zIndex: 9998, background: "transparent" }}
      />
      <div style={{
        position: "fixed", top: "50%", left: "50%", transform: "translate(-50%, -50%)",
        zIndex: 9999, background: "#fff", borderRadius: 12, padding: "20px 24px",
        boxShadow: "0 8px 32px rgba(0,0,0,0.2)", minWidth: 320,
      }}>
        <div style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>
          新建可视化编程标签页
        </div>

        {/* Tab type */}
        <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
          <button
            onClick={() => setType("event")}
            style={{
              flex: 1, padding: "8px 12px", borderRadius: 8, fontSize: 12, fontWeight: 600,
              border: type === "event" ? "2px solid #7C3AED" : "1px solid #D1D5DB",
              background: type === "event" ? "#F5F3FF" : "#fff",
              color: type === "event" ? "#7C3AED" : "#6B7280",
              cursor: "pointer",
            }}
          >
            ⚡ 事件代码
          </button>
          <button
            onClick={() => setType("logic")}
            style={{
              flex: 1, padding: "8px 12px", borderRadius: 8, fontSize: 12, fontWeight: 600,
              border: type === "logic" ? "2px solid #4F46E5" : "1px solid #D1D5DB",
              background: type === "logic" ? "#EEF2FF" : "#fff",
              color: type === "logic" ? "#4F46E5" : "#6B7280",
              cursor: "pointer",
            }}
          >
            📄 逻辑代码
          </button>
        </div>

        {/* Tab type description */}
        <div style={{
          fontSize: 11, color: "#6B7280", marginBottom: 12, lineHeight: 1.4,
          padding: "6px 8px", background: "#F9FAFB", borderRadius: 6,
        }}>
          {type === "event"
            ? "事件代码标签页中的自定义函数会出现在设计界面的事件绑定下拉中，用于处理组件的交互事件。"
            : "逻辑代码标签页用于编写通用的数据处理逻辑、工具函数等，不会出现在事件绑定下拉中。"}
        </div>

        {/* Name */}
        <input
          placeholder="标签页名称"
          value={label}
          onChange={(e) => setLabel(e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter") handleCreate(); if (e.key === "Escape") onClose(); }}
          autoFocus
          style={{
            width: "100%", padding: "6px 10px", fontSize: 13, border: "1px solid #d1d5db",
            borderRadius: 6, boxSizing: "border-box", marginBottom: 12,
          }}
        />

        {/* Actions */}
        <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
          <button
            onClick={onClose}
            style={{ padding: "5px 14px", fontSize: 12, border: "1px solid #d1d5db", borderRadius: 6, background: "#fff", cursor: "pointer" }}
          >
            取消
          </button>
          <button
            onClick={handleCreate}
            disabled={!label.trim()}
            style={{
              padding: "5px 14px", fontSize: 12, border: "none", borderRadius: 6,
              background: label.trim() ? "#4F46E5" : "#D1D5DB", color: "#fff",
              cursor: label.trim() ? "pointer" : "not-allowed", fontWeight: 600,
            }}
          >
            创建
          </button>
        </div>
      </div>
    </>
  );
}

// ── Main CodeTabBar ──

export function CodeTabBar() {
  const tabs = useCodeGraphStore((s) => s.tabs);
  const activeTabId = useCodeGraphStore((s) => s.activeTabId);
  const switchTab = useCodeGraphStore((s) => s.switchTab);
  const removeTab = useCodeGraphStore((s) => s.removeTab);
  const renameTab = useCodeGraphStore((s) => s.renameTab);
  const addTab = useCodeGraphStore((s) => s.addTab);
  const [showCreateDialog, setShowCreateDialog] = useState(false);

  const handleClose = (e: React.MouseEvent, tabId: string) => {
    e.stopPropagation();
    if (tabId === "main") return;
    removeTab(tabId);
  };

  const handleQuickCreate = () => {
    addTab("logic", `画布 ${tabs.length}`, { icon: "📄" });
  };

  const renderTab = (tab: CodeTab) => {
    const isActive = activeTabId === tab.id;
    const label = tab.id === "main" ? "主画布" : tab.label;
    const closable = tab.id !== "main";

    return (
      <div
        key={tab.id}
        onClick={() => switchTab(tab.id)}
        onDoubleClick={() => {
          if (tab.id !== "main") {
            const newLabel = prompt("重命名标签页:", tab.label);
            if (newLabel && newLabel.trim()) {
              renameTab(tab.id, newLabel.trim());
            }
          }
        }}
        style={{
          display: "flex", alignItems: "center", gap: 4,
          padding: "0 10px", cursor: "pointer", fontSize: 11,
          whiteSpace: "nowrap", userSelect: "none",
          background: isActive ? "#fff" : "transparent",
          borderBottom: isActive ? "2px solid #4F46E5" : "2px solid transparent",
          borderRight: "1px solid #E5E7EB",
          color: isActive ? "#1F2937" : "#6B7280",
          fontWeight: isActive ? 600 : 400,
          transition: "background 0.1s",
          height: 32,
        }}
        onMouseEnter={(e) => {
          if (!isActive) e.currentTarget.style.background = "#E5E7EB";
        }}
        onMouseLeave={(e) => {
          if (!isActive) e.currentTarget.style.background = "transparent";
        }}
        title={
          tab.type === "event"
            ? "⚡ 事件代码 — 函数会出现在设计界面事件绑定下拉中"
            : "📄 逻辑代码"
        }
      >
        <span style={{ fontSize: 12 }}>{tab.type === "event" ? "⚡" : "📄"}</span>
        <span>{label}</span>
        {tab.type === "event" && (
          <span style={{
            fontSize: 9, color: "#7C3AED", background: "#F5F3FF",
            padding: "0 3px", borderRadius: 3, marginLeft: 2,
          }}>
            事件
          </span>
        )}
        {closable && (
          <span
            onClick={(e) => handleClose(e, tab.id)}
            style={{
              marginLeft: 2, fontSize: 13, lineHeight: 1,
              padding: "0 2px", borderRadius: 3, color: "#9CA3AF",
            }}
            onMouseEnter={(e) => { e.currentTarget.style.background = "#FEE2E2"; e.currentTarget.style.color = "#DC2626"; }}
            onMouseLeave={(e) => { e.currentTarget.style.background = "transparent"; e.currentTarget.style.color = "#9CA3AF"; }}
            title="关闭标签页"
          >
            ×
          </span>
        )}
      </div>
    );
  };

  return (
    <>
      <div style={{
        height: 32, minHeight: 32,
        background: "#F3F4F6", borderBottom: "1px solid #E5E7EB",
        display: "flex", alignItems: "stretch", overflow: "hidden",
      }}>
        {/* Scrollable tab area */}
        <div style={{
          flex: 1, display: "flex", alignItems: "stretch",
          overflow: "auto", scrollbarWidth: "none",
        }}>
          {tabs.map(renderTab)}
        </div>

        {/* + buttons */}
        <div style={{
          display: "flex", alignItems: "center", padding: "0 4px",
          borderLeft: "1px solid #E5E7EB", gap: 2,
        }}>
          <button
            onClick={handleQuickCreate}
            style={{
              width: 22, height: 22, borderRadius: 4,
              border: "1px dashed #D1D5DB", background: "transparent",
              cursor: "pointer", fontSize: 14, color: "#6B7280",
              display: "flex", alignItems: "center", justifyContent: "center",
              lineHeight: 1, padding: 0,
            }}
            title="新建逻辑画布"
          >
            +
          </button>
          <button
            onClick={() => setShowCreateDialog(true)}
            style={{
              width: 22, height: 22, borderRadius: 4,
              border: "1px dashed #D1D5DB", background: "transparent",
              cursor: "pointer", fontSize: 12, color: "#7C3AED",
              display: "flex", alignItems: "center", justifyContent: "center",
              lineHeight: 1, padding: 0,
            }}
            title="新建标签页（支持事件/逻辑）"
          >
            ⚡
          </button>
        </div>
      </div>

      {showCreateDialog && <NewTabDialog onClose={() => setShowCreateDialog(false)} />}
    </>
  );
}
