/** 画布标签栏 — 管理主画布、组件画布和文件画布的标签页切换。
 *  替代原来的 ComponentCanvasHeader 独立返回按钮。 */

import React, { useState } from "react";
import { useEditorStore } from "../store/editor";
import { useComponentRegistry } from "../store/componentRegistry";

interface TabInfo {
  key: string;          // docId: null for main, "comp:Name", "file:path"
  label: string;        // display name
  icon: string;         // emoji icon
  closable: boolean;    // can this tab be closed?
}

export function CanvasTabBar() {
  const activeDocId = useEditorStore((s) => s.activeDocId);
  const openTabs = useEditorStore((s) => s.openTabs);
  const switchTab = useEditorStore((s) => s.switchTab);
  const closeTab = useEditorStore((s) => s.closeTab);
  const enterCanvas = useEditorStore((s) => s.enterComponentCanvas);
  const defineComponent = useComponentRegistry((s) => s.defineComponent);
  const definitions = useComponentRegistry((s) => s.definitions);

  const [showCreateDialog, setShowCreateDialog] = useState(false);

  // Build tab list
  const tabs: TabInfo[] = [
    { key: "main", label: "主画布", icon: "📄", closable: false },
  ];

  // Component tabs (from openTabs)
  for (const tabKey of openTabs) {
    if (tabKey.startsWith("comp:")) {
      const compName = tabKey.replace("comp:", "");
      tabs.push({ key: tabKey, label: compName, icon: "★", closable: true });
    }
  }

  // File tabs (from openTabs)
  for (const tabKey of openTabs) {
    if (tabKey.startsWith("file:")) {
      const fileName = tabKey.replace("file:", "").replace(/^.*[\\/]/, "").replace(".omni.json", "");
      tabs.push({ key: tabKey, label: fileName, icon: "🔧", closable: true });
    }
  }

  // Determine active tab key for highlighting
  const activeKey = activeDocId || "main";

  const handleTabClick = (tab: TabInfo) => {
    if (tab.key === "main") {
      if (activeDocId !== null) {
        // If in component canvas with activeDocId, use exitComponentCanvas
        if (activeDocId?.startsWith("comp:")) {
          useEditorStore.getState().exitComponentCanvas();
        } else {
          switchTab(null);
        }
      }
    } else if (tab.key.startsWith("comp:")) {
      const compName = tab.key.replace("comp:", "");
      if (activeDocId !== tab.key) {
        // If we're already in component mode, switchTab; otherwise enterComponentCanvas
        if (activeDocId?.startsWith("comp:") || activeDocId?.startsWith("file:")) {
          switchTab(tab.key);
        } else {
          enterCanvas(compName);
        }
      }
    } else if (tab.key.startsWith("file:")) {
      if (activeDocId !== tab.key) {
        switchTab(tab.key);
      }
    }
  };

  const handleClose = (e: React.MouseEvent, tab: TabInfo) => {
    e.stopPropagation();
    if (!tab.closable) return;
    // If closing a component tab, also exit if it's the active one
    closeTab(tab.key);
  };

  const handleCreate = (name: string) => {
    if (!name.trim()) return;
    defineComponent(name.trim(), {
      category: "Custom",
      description: "",
      props: {},
      events: {},
      styleProperties: ["padding", "margin", "width", "height"],
      children: true,
      storage: "config",
    });
    setShowCreateDialog(false);
    setTimeout(() => enterCanvas(name.trim()), 50);
  };

  return (
    <>
      <div style={{
        height: 32, minHeight: 32,
        background: "#F3F4F6", borderBottom: "1px solid #E5E7EB",
        display: "flex", alignItems: "stretch", overflow: "hidden",
      }}>
        {/* Scrollable tab area */}
        <div style={{ flex: 1, display: "flex", alignItems: "stretch", overflow: "auto", scrollbarWidth: "none" }}>
          {tabs.map((tab) => {
            const isActive = activeKey === tab.key;
            return (
              <div
                key={tab.key}
                onClick={() => handleTabClick(tab)}
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
                }}
                onMouseEnter={(e) => {
                  if (!isActive) e.currentTarget.style.background = "#E5E7EB";
                }}
                onMouseLeave={(e) => {
                  if (!isActive) e.currentTarget.style.background = "transparent";
                }}
                title={tab.key.startsWith("file:") ? tab.key.replace("file:", "") : tab.label}
              >
                <span style={{ fontSize: 12 }}>{tab.icon}</span>
                <span>{tab.label}</span>
                {tab.closable && (
                  <span
                    onClick={(e) => handleClose(e, tab)}
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
          })}
        </div>

        {/* + button to create new component */}
        <div style={{
          display: "flex", alignItems: "center", padding: "0 6px",
          borderLeft: "1px solid #E5E7EB",
        }}>
          <button
            onClick={() => setShowCreateDialog(true)}
            style={{
              width: 22, height: 22, borderRadius: 4,
              border: "1px dashed #D1D5DB", background: "transparent",
              cursor: "pointer", fontSize: 14, color: "#6B7280",
              display: "flex", alignItems: "center", justifyContent: "center",
              lineHeight: 1, padding: 0,
            }}
            title="新建自定义组件"
            onMouseEnter={(e) => { e.currentTarget.style.borderColor = "#4F46E5"; e.currentTarget.style.color = "#4F46E5"; e.currentTarget.style.background = "#EEF2FF"; }}
            onMouseLeave={(e) => { e.currentTarget.style.borderColor = "#D1D5DB"; e.currentTarget.style.color = "#6B7280"; e.currentTarget.style.background = "transparent"; }}
          >
            +
          </button>
        </div>
      </div>

      {/* Create Dialog */}
      {showCreateDialog && (
        <>
          <div
            onClick={() => setShowCreateDialog(false)}
            style={{ position: "fixed", inset: 0, zIndex: 9998, background: "transparent" }}
          />
          <div style={{
            position: "fixed", top: "50%", left: "50%", transform: "translate(-50%, -50%)",
            zIndex: 9999, background: "#fff", borderRadius: 12, padding: "20px 24px",
            boxShadow: "0 8px 32px rgba(0,0,0,0.2)", minWidth: 300,
          }}>
            <div style={{ fontSize: 14, fontWeight: 600, marginBottom: 12 }}>
              新建自定义组件
            </div>
            <input
              id="tab-new-comp-name"
              placeholder="组件名称 (如 MyCard)"
              autoFocus
              style={{
                width: "100%", padding: "6px 10px", fontSize: 13, border: "1px solid #d1d5db",
                borderRadius: 6, boxSizing: "border-box", marginBottom: 12,
              }}
              onKeyDown={(e) => {
                if (e.key === "Enter") handleCreate((e.target as HTMLInputElement).value);
                if (e.key === "Escape") setShowCreateDialog(false);
              }}
            />
            <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
              <button
                onClick={() => setShowCreateDialog(false)}
                style={{ padding: "5px 14px", fontSize: 12, border: "1px solid #d1d5db", borderRadius: 6, background: "#fff", cursor: "pointer" }}
              >
                取消
              </button>
              <button
                onClick={() => {
                  const input = document.getElementById("tab-new-comp-name") as HTMLInputElement;
                  handleCreate(input?.value || "");
                }}
                style={{ padding: "5px 14px", fontSize: 12, border: "none", borderRadius: 6, background: "#4F46E5", color: "#fff", cursor: "pointer", fontWeight: 600 }}
              >
                创建并编辑
              </button>
            </div>
          </div>
        </>
      )}
    </>
  );
}
