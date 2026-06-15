/** Code mode tab bar — manages visual programming tabs.
 *  Supports freeform, component, and function tabs.
 *  Function tabs show a parameter editor strip. */

import React, { useState } from "react";
import { useCodeGraphStore } from "../codegraph/store";
import type { CodeTab, FnParam } from "../codegraph/types";
import { newTabId } from "../codegraph/types";

// ── Tab Creation Dialog ──

function NewTabDialog({ onClose }: { onClose: () => void }) {
  const addTab = useCodeGraphStore((s) => s.addTab);
  const [type, setType] = useState<"freeform" | "function">("freeform");
  const [label, setLabel] = useState("");
  const [fnParams, setFnParams] = useState<FnParam[]>([]);
  const [fnReturnType, setFnReturnType] = useState("void");
  const [newParamName, setNewParamName] = useState("");
  const [newParamType, setNewParamType] = useState("String");

  const handleCreate = () => {
    const name = label.trim();
    if (!name) return;
    if (type === "function") {
      addTab("function", name, {
        fnName: name,
        fnParams: fnParams.length > 0 ? fnParams : undefined,
        fnReturnType,
      });
    } else {
      addTab("freeform", name);
    }
    onClose();
  };

  const addParam = () => {
    const pName = newParamName.trim();
    if (!pName) return;
    setFnParams([...fnParams, { name: pName, type: newParamType }]);
    setNewParamName("");
    setNewParamType("String");
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
        boxShadow: "0 8px 32px rgba(0,0,0,0.2)", minWidth: 360,
      }}>
        <div style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>
          新建可视化编程标签页
        </div>

        {/* Tab type */}
        <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
          <button
            onClick={() => setType("freeform")}
            style={{
              flex: 1, padding: "8px 12px", borderRadius: 8, fontSize: 12, fontWeight: 600,
              border: type === "freeform" ? "2px solid #4F46E5" : "1px solid #D1D5DB",
              background: type === "freeform" ? "#EEF2FF" : "#fff",
              color: type === "freeform" ? "#4F46E5" : "#6B7280",
              cursor: "pointer",
            }}
          >
            📄 通用画布
          </button>
          <button
            onClick={() => setType("function")}
            style={{
              flex: 1, padding: "8px 12px", borderRadius: 8, fontSize: 12, fontWeight: 600,
              border: type === "function" ? "2px solid #4F46E5" : "1px solid #D1D5DB",
              background: type === "function" ? "#EEF2FF" : "#fff",
              color: type === "function" ? "#4F46E5" : "#6B7280",
              cursor: "pointer",
            }}
          >
            📦 函数
          </button>
        </div>

        {/* Name */}
        <input
          placeholder={type === "function" ? "函数名 (如 validateForm)" : "标签页名称"}
          value={label}
          onChange={(e) => setLabel(e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter") handleCreate(); if (e.key === "Escape") onClose(); }}
          autoFocus
          style={{
            width: "100%", padding: "6px 10px", fontSize: 13, border: "1px solid #d1d5db",
            borderRadius: 6, boxSizing: "border-box", marginBottom: 12,
          }}
        />

        {/* Function params */}
        {type === "function" && (
          <>
            <div style={{ fontSize: 11, color: "#6B7280", marginBottom: 6, fontWeight: 600 }}>
              参数定义
            </div>
            {fnParams.map((p, i) => (
              <div key={i} style={{
                display: "flex", gap: 4, alignItems: "center", marginBottom: 4,
                fontSize: 11, padding: "3px 8px", background: "#F3F4F6", borderRadius: 4,
              }}>
                <span style={{ fontFamily: "monospace", flex: 1 }}>{p.name}: {p.type}</span>
                <button
                  onClick={() => setFnParams(fnParams.filter((_, j) => j !== i))}
                  style={{ border: "none", background: "transparent", cursor: "pointer", color: "#DC2626", fontSize: 12 }}
                >
                  ✕
                </button>
              </div>
            ))}
            <div style={{ display: "flex", gap: 4, marginBottom: 4 }}>
              <input
                placeholder="参数名"
                value={newParamName}
                onChange={(e) => setNewParamName(e.target.value)}
                onKeyDown={(e) => { if (e.key === "Enter") addParam(); }}
                style={{ flex: 1, padding: "3px 6px", fontSize: 11, border: "1px solid #d1d5db", borderRadius: 4 }}
              />
              <select
                value={newParamType}
                onChange={(e) => setNewParamType(e.target.value)}
                style={{ width: 80, padding: "3px 6px", fontSize: 11, border: "1px solid #d1d5db", borderRadius: 4 }}
              >
                {["String", "i32", "f64", "bool", "any", "MouseEvent"].map((t) => (
                  <option key={t} value={t}>{t}</option>
                ))}
              </select>
              <button
                onClick={addParam}
                style={{
                  padding: "3px 8px", fontSize: 11, border: "1px dashed #D1D5DB",
                  borderRadius: 4, background: "#fff", cursor: "pointer", color: "#6B7280",
                }}
              >
                +
              </button>
            </div>
            <div style={{ display: "flex", gap: 4, alignItems: "center", marginBottom: 12, fontSize: 11 }}>
              <span style={{ color: "#6B7280" }}>返回:</span>
              <select
                value={fnReturnType}
                onChange={(e) => setFnReturnType(e.target.value)}
                style={{ padding: "3px 6px", border: "1px solid #d1d5db", borderRadius: 4, fontSize: 11 }}
              >
                {["void", "String", "i32", "f64", "bool", "any"].map((t) => (
                  <option key={t} value={t}>{t}</option>
                ))}
              </select>
            </div>
          </>
        )}

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

  // Predefined tabs: "main" always comes first
  const mainTab = tabs.find((t) => t.id === "main");
  const customTabs = tabs.filter((t) => t.id !== "main");

  const handleClose = (e: React.MouseEvent, tabId: string) => {
    e.stopPropagation();
    if (tabId === "main") return;
    removeTab(tabId);
  };

  const handleCreateDefault = () => {
    const id = `tab-${Date.now().toString(36)}`;
    addTab("freeform", `画布 ${tabs.length}`);
  };

  const renderTab = (tab: CodeTab) => {
    const isActive = activeTabId === tab.id;
    const label = tab.id === "main" ? "主画布" : tab.label;
    const icon = tab.id === "main" ? "📄" : tab.icon;
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
          height: 32, alignItems: "center",
        }}
        onMouseEnter={(e) => {
          if (!isActive) e.currentTarget.style.background = "#E5E7EB";
        }}
        onMouseLeave={(e) => {
          if (!isActive) e.currentTarget.style.background = "transparent";
        }}
        title={
          tab.type === "function" && tab.fnParams
            ? `${tab.label}(${tab.fnParams.map((p) => `${p.name}: ${p.type}`).join(", ")}) → ${tab.fnReturnType || "void"}`
            : tab.type === "component" && tab.componentId
              ? `组件: ${tab.componentId}`
              : tab.label
        }
      >
        <span style={{ fontSize: 12 }}>{icon}</span>
        <span>{label}</span>
        {tab.type === "function" && tab.fnParams && tab.fnParams.length > 0 && (
          <span style={{
            fontSize: 9, color: "#7C3AED", background: "#F5F3FF",
            padding: "0 3px", borderRadius: 3, marginLeft: 2,
          }}>
            {tab.fnParams.length}p
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
            onClick={handleCreateDefault}
            style={{
              width: 22, height: 22, borderRadius: 4,
              border: "1px dashed #D1D5DB", background: "transparent",
              cursor: "pointer", fontSize: 14, color: "#6B7280",
              display: "flex", alignItems: "center", justifyContent: "center",
              lineHeight: 1, padding: 0,
            }}
            title="新建画布"
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
            title="新建函数标签页..."
          >
            📦
          </button>
        </div>
      </div>

      {showCreateDialog && <NewTabDialog onClose={() => setShowCreateDialog(false)} />}
    </>
  );
}
