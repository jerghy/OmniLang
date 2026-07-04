// components/design/DesignTabBar.tsx
// Tab bar showing open .omni files with dirty indicator and close button.

import { useState, useRef, useCallback } from "react";
import { usePoiesisStore } from "../../store";

export function DesignTabBar() {
  const tabs = usePoiesisStore((s) => s.designPage.openTabs);
  const activeTabId = usePoiesisStore((s) => s.designPage.activeTabId);
  const tabOrder = usePoiesisStore((s) => s.designPage.tabOrder);
  const switchTab = usePoiesisStore((s) => s.switchTab);
  const closeTab = usePoiesisStore((s) => s.closeTab);
  const reorderTabs = usePoiesisStore((s) => s.reorderTabs);

  const [dragOverTabId, setDragOverTabId] = useState<string | null>(null);
  const dragIdRef = useRef<string | null>(null);

  // Confirm dialog for dirty tabs
  const [confirmClose, setConfirmClose] = useState<string | null>(null);

  const handleClose = useCallback(
    (e: React.MouseEvent, tabId: string) => {
      e.stopPropagation();
      const tab = tabs.find((t) => t.id === tabId);
      if (tab?.isDirty) {
        setConfirmClose(tabId);
      } else {
        closeTab(tabId);
      }
    },
    [tabs, closeTab]
  );

  const confirmAndClose = useCallback(() => {
    if (confirmClose) {
      closeTab(confirmClose);
      setConfirmClose(null);
    }
  }, [confirmClose, closeTab]);

  // Drag reorder
  const handleDragStart = useCallback((tabId: string) => {
    dragIdRef.current = tabId;
  }, []);

  const handleDragOver = useCallback(
    (e: React.DragEvent, tabId: string) => {
      e.preventDefault();
      setDragOverTabId(tabId);
    },
    []
  );

  const handleDrop = useCallback(
    (targetId: string) => {
      const draggedId = dragIdRef.current;
      if (!draggedId || draggedId === targetId) return;
      const newOrder = tabOrder.filter((id) => id !== draggedId);
      const targetIdx = newOrder.indexOf(targetId);
      newOrder.splice(targetIdx, 0, draggedId);
      reorderTabs(newOrder);
      dragIdRef.current = null;
      setDragOverTabId(null);
    },
    [tabOrder, reorderTabs]
  );

  const handleDragEnd = useCallback(() => {
    dragIdRef.current = null;
    setDragOverTabId(null);
  }, []);

  const orderedTabs = tabOrder
    .map((id) => tabs.find((t) => t.id === id))
    .filter((t): t is NonNullable<typeof t> => t !== undefined);

  return (
    <>
      <div
        style={{
          display: "flex",
          height: 36,
          background: "#f1f5f9",
          borderBottom: "1px solid #e2e8f0",
          overflow: "auto",
        }}
      >
        {orderedTabs.map((tab) => {
          const isActive = tab.id === activeTabId;
          const isDragOver = tab.id === dragOverTabId;
          return (
            <div
              key={tab.id}
              draggable
              onDragStart={() => handleDragStart(tab.id)}
              onDragOver={(e) => handleDragOver(e, tab.id)}
              onDrop={() => handleDrop(tab.id)}
              onDragEnd={handleDragEnd}
              onClick={() => switchTab(tab.id)}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 4,
                padding: "0 12px",
                cursor: "pointer",
                fontSize: 12,
                borderRight: "1px solid #e2e8f0",
                borderTop: isActive ? "2px solid #6366F1" : "2px solid transparent",
                background: isActive ? "#fff" : "transparent",
                color: isActive ? "#1e293b" : "#64748b",
                opacity: isDragOver ? 0.7 : 1,
                transition: "opacity 150ms, background 150ms",
                whiteSpace: "nowrap",
                userSelect: "none",
              }}
            >
              <span>
                {tab.isDirty && <span style={{ color: "#f59e0b" }}>● </span>}
                {tab.componentName}.omni
              </span>
              <button
                onClick={(e) => handleClose(e, tab.id)}
                style={{
                  border: "none",
                  background: "none",
                  cursor: "pointer",
                  fontSize: 14,
                  color: "#94a3b8",
                  padding: "0 2px",
                  lineHeight: 1,
                }}
                title="关闭"
              >
                ×
              </button>
            </div>
          );
        })}
        {orderedTabs.length === 0 && (
          <div
            style={{
              display: "flex",
              alignItems: "center",
              padding: "0 12px",
              fontSize: 12,
              color: "#94a3b8",
            }}
          >
            没有打开的 .omni 文件
          </div>
        )}
      </div>

      {/* Confirm dialog for dirty tab close */}
      {confirmClose && (
        <>
          <div
            onClick={() => setConfirmClose(null)}
            style={{
              position: "fixed",
              inset: 0,
              background: "rgba(0,0,0,0.3)",
              zIndex: 10000,
            }}
          />
          <div
            style={{
              position: "fixed",
              top: "50%",
              left: "50%",
              transform: "translate(-50%, -50%)",
              zIndex: 10001,
              background: "#fff",
              borderRadius: 12,
              padding: "20px 24px",
              boxShadow: "0 8px 32px rgba(0,0,0,0.2)",
              minWidth: 320,
            }}
          >
            <div style={{ fontSize: 14, fontWeight: 600, marginBottom: 8 }}>
              未保存的更改
            </div>
            <div style={{ fontSize: 12, color: "#64748b", marginBottom: 16 }}>
              此标签页有未保存的更改。确定要关闭吗？
            </div>
            <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
              <button
                onClick={() => setConfirmClose(null)}
                style={{
                  padding: "5px 14px",
                  fontSize: 12,
                  border: "1px solid #d1d5db",
                  borderRadius: 6,
                  background: "#fff",
                  cursor: "pointer",
                }}
              >
                取消
              </button>
              <button
                onClick={confirmAndClose}
                style={{
                  padding: "5px 14px",
                  fontSize: 12,
                  border: "none",
                  borderRadius: 6,
                  background: "#ef4444",
                  color: "#fff",
                  cursor: "pointer",
                  fontWeight: 600,
                }}
              >
                关闭
              </button>
            </div>
          </div>
        </>
      )}
    </>
  );
}
