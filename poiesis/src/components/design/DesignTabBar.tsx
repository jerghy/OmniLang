// components/design/DesignTabBar.tsx
// Tab bar showing open .omni files with dirty indicator and close button.
// "+" button opens a hierarchical file tree panel of .omni files.
// Includes "New File" button for creating store or code files.

import { useState, useRef, useCallback, useEffect } from "react";
import { usePoiesisStore } from "../../store";
import { api } from "../../commands";
import type { TabType } from "../../core/types/design-page";
import { detectTabType } from "../../core/types/design-page";

interface OmniFile {
  path: string;
  name: string;
}

interface FileTreeNode {
  name: string;
  path: string;
  type: "directory" | "file";
  children: FileTreeNode[];
}

type NewFileType = "store" | "code";

// ── Recursive file tree node renderer ──

interface FileNodeRendererProps {
  node: FileTreeNode;
  tabs: { filePath: string }[];
  expandedDirs: Set<string>;
  onToggleDir: (path: string) => void;
  onOpenFile: (path: string, name: string) => void;
  depth?: number;
}

function FileNodeRenderer({
  node,
  tabs,
  expandedDirs,
  onToggleDir,
  onOpenFile,
  depth = 0,
}: FileNodeRendererProps) {
  const isExpanded = expandedDirs.has(node.path);
  const indent = depth * 16;

  if (node.type === "directory") {
    return (
      <div>
        <button
          onClick={() => onToggleDir(node.path)}
          style={{
            display: "flex",
            alignItems: "center",
            gap: 4,
            width: "100%",
            padding: `4px 12px 4px ${12 + indent}px`,
            border: "none",
            background: "transparent",
            cursor: "pointer",
            fontSize: 12,
            color: "#374151",
            fontWeight: 500,
            textAlign: "left" as const,
            fontFamily: "inherit",
          }}
        >
          <span style={{ fontSize: 10, width: 14, color: "#94a3b8" }}>
            {isExpanded ? "▼" : "▶"}
          </span>
          <span style={{ fontSize: 12 }}>{isExpanded ? "📂" : "📁"}</span>
          <span>{node.name}</span>
        </button>
        {isExpanded &&
          node.children.map((child) => (
            <FileNodeRenderer
              key={child.path}
              node={child}
              tabs={tabs}
              expandedDirs={expandedDirs}
              onToggleDir={onToggleDir}
              onOpenFile={onOpenFile}
              depth={depth + 1}
            />
          ))}
      </div>
    );
  }

  // File node
  const isOpen = tabs.some((t) => t.filePath === node.path);
  return (
    <button
      onClick={() => !isOpen && onOpenFile(node.path, node.name)}
      disabled={isOpen}
      style={{
        display: "flex",
        alignItems: "center",
        gap: 6,
        width: "100%",
        padding: `4px 12px 4px ${12 + indent + 14}px`,
        border: "none",
        background: "transparent",
        cursor: isOpen ? "default" : "pointer",
        fontSize: 12,
        color: isOpen ? "#94a3b8" : "#374151",
        textAlign: "left" as const,
        fontFamily: "inherit",
        opacity: isOpen ? 0.5 : 1,
        whiteSpace: "nowrap" as const,
        overflow: "hidden",
        textOverflow: "ellipsis",
      }}
    >
      <span style={{ fontSize: 11 }}>{isOpen ? "📌" : "📄"}</span>
      <span style={{ overflow: "hidden", textOverflow: "ellipsis" }}>{node.name}</span>
      {isOpen && (
        <span style={{ fontSize: 10, color: "#94a3b8", marginLeft: "auto", flexShrink: 0 }}>
          已打开
        </span>
      )}
    </button>
  );
}

// ── Main tab bar component ──

export function DesignTabBar() {
  const tabs = usePoiesisStore((s) => s.designPage.openTabs);
  const activeTabId = usePoiesisStore((s) => s.designPage.activeTabId);
  const tabOrder = usePoiesisStore((s) => s.designPage.tabOrder);
  const switchTab = usePoiesisStore((s) => s.switchTab);
  const closeTab = usePoiesisStore((s) => s.closeTab);
  const reorderTabs = usePoiesisStore((s) => s.reorderTabs);
  const openTab = usePoiesisStore((s) => s.openTab);

  const [dragOverTabId, setDragOverTabId] = useState<string | null>(null);
  const dragIdRef = useRef<string | null>(null);

  // Confirm dialog for dirty tabs
  const [confirmClose, setConfirmClose] = useState<string | null>(null);

  // "+" file tree panel
  const [showFilePanel, setShowFilePanel] = useState(false);
  const [fileTree, setFileTree] = useState<FileTreeNode[]>([]);
  const [expandedDirs, setExpandedDirs] = useState<Set<string>>(new Set());
  const panelRef = useRef<HTMLDivElement>(null);

  // New file sub-panel
  const [showNewFilePanel, setShowNewFilePanel] = useState(false);
  const [newFileType, setNewFileType] = useState<NewFileType>("code");
  const [newFileName, setNewFileName] = useState("");

  // Build file tree when panel opens — recursively scan for .omni files
  useEffect(() => {
    if (!showFilePanel) return;
    (async () => {
      const scanDir = async (dirPath: string): Promise<FileTreeNode[]> => {
        const nodes: FileTreeNode[] = [];
        try {
          const result = await api.project.listDir(dirPath);
          if (!result.entries) return nodes;
          for (const entry of result.entries) {
            const entryPath = dirPath === "." ? entry.name : `${dirPath}/${entry.name}`;
            if (entry.type === "directory") {
              const children = await scanDir(entryPath);
              // Only include directories that contain .omni files (recursively)
              if (children.length > 0) {
                nodes.push({
                  name: entry.name,
                  path: entryPath,
                  type: "directory",
                  children,
                });
              }
            } else if (entry.ext === "omni") {
              nodes.push({
                name: entry.name,
                path: entryPath,
                type: "file",
                children: [],
              });
            }
          }
        } catch { /* dir may not exist */ }
        return nodes;
      };

      const tree = await scanDir(".");
      setFileTree(tree);
      // Auto-expand src/ and its standard subdirs
      const autoExpand = new Set<string>();
      const addPath = (nodes: FileTreeNode[], prefix: string) => {
        for (const n of nodes) {
          const p = prefix ? `${prefix}/${n.name}` : n.name;
          if (n.type === "directory") {
            autoExpand.add(p);
            addPath(n.children, p);
          }
        }
      };
      // Expand the first 2 levels for visibility
      for (const node of tree) {
        if (node.type === "directory") {
          autoExpand.add(node.name);
          for (const child of node.children) {
            if (child.type === "directory") {
              autoExpand.add(`${node.name}/${child.name}`);
            }
          }
        }
      }
      setExpandedDirs(autoExpand);
    })();
  }, [showFilePanel]);

  // Close panel on outside click
  useEffect(() => {
    if (!showFilePanel) return;
    const handler = (e: MouseEvent) => {
      if (panelRef.current && !panelRef.current.contains(e.target as Node)) {
        setShowFilePanel(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [showFilePanel]);

  const handleOpenFile = useCallback(
    (filePath: string, fileName: string) => {
      const componentName = fileName.replace(/\.omni$/, "");
      const tabType = detectTabType(filePath);
      openTab(filePath, componentName, tabType);
      setShowFilePanel(false);
    },
    [openTab]
  );

  const handleCreateNewFile = useCallback(() => {
    if (!newFileName.trim()) return;
    const sanitized = newFileName.trim().replace(/\.omni$/i, "");
    const fullName = `${sanitized}.omni`;
    const dir = newFileType === "store" ? "src/stores" : "src/lib";
    const filePath = `${dir}/${fullName}`;
    const tabType: TabType = newFileType; // "store" → "store", "code" → "code"
    openTab(filePath, sanitized, tabType);
    setShowNewFilePanel(false);
    setShowFilePanel(false);
    setNewFileName("");
  }, [newFileName, newFileType, openTab]);

  const toggleDir = useCallback((dirPath: string) => {
    setExpandedDirs((prev) => {
      const next = new Set(prev);
      if (next.has(dirPath)) {
        next.delete(dirPath);
      } else {
        next.add(dirPath);
      }
      return next;
    });
  }, []);

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

        {/* "+" button to open file picker */}
        <div style={{ display: "flex", alignItems: "center", padding: "0 2px" }}>
          <button
            onClick={(e) => {
              e.stopPropagation();
              setShowFilePanel(!showFilePanel);
            }}
            title="打开 .omni 文件"
            style={{
              border: "none",
              background: showFilePanel ? "#e2e8f0" : "transparent",
              cursor: "pointer",
              fontSize: 16,
              color: "#6366F1",
              fontWeight: 700,
              padding: "0 8px",
              height: 28,
              borderRadius: 4,
              lineHeight: 1,
            }}
          >
            +
          </button>
        </div>
      </div>

      {/* File tree panel */}
      {showFilePanel && !showNewFilePanel && (
        <>
          <div
            onClick={() => setShowFilePanel(false)}
            style={{
              position: "fixed",
              inset: 0,
              zIndex: 9998,
              background: "transparent",
            }}
          />
          <div
            ref={panelRef}
            style={{
              position: "fixed",
              top: 44,
              left: 8,
              zIndex: 9999,
              background: "#fff",
              borderRadius: 8,
              boxShadow: "0 4px 20px rgba(0,0,0,0.15)",
              minWidth: 260,
              maxWidth: 360,
              maxHeight: 420,
              display: "flex",
              flexDirection: "column",
            }}
          >
            <div
              style={{
                fontSize: 11,
                fontWeight: 600,
                color: "#6b7280",
                padding: "6px 12px",
                borderBottom: "1px solid #f1f5f9",
                display: "flex",
                alignItems: "center",
                gap: 6,
              }}
            >
              <span>📁 项目文件</span>
              <span style={{ fontSize: 10, color: "#94a3b8", fontWeight: 400 }}>
                ({fileTree.length > 0 ? "已加载" : "扫描中..."})
              </span>
            </div>

            {/* Scrollable tree area */}
            <div
              style={{
                flex: 1,
                overflow: "auto",
                padding: "4px 0",
                minHeight: 0,
              }}
            >
              {fileTree.length === 0 && (
                <div style={{ padding: "16px 12px", fontSize: 12, color: "#94a3b8", textAlign: "center" }}>
                  扫描中...
                </div>
              )}
              {fileTree.map((node) => (
                <FileNodeRenderer
                  key={node.path}
                  node={node}
                  tabs={tabs}
                  expandedDirs={expandedDirs}
                  onToggleDir={toggleDir}
                  onOpenFile={handleOpenFile}
                />
              ))}
            </div>

            {/* New File button */}
            <div
              style={{
                borderTop: "1px solid #f1f5f9",
                padding: "4px",
              }}
            >
              <button
                onClick={() => setShowNewFilePanel(true)}
                style={{
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  gap: 4,
                  width: "100%",
                  padding: "7px 12px",
                  border: "1px dashed #d1d5db",
                  borderRadius: 6,
                  background: "transparent",
                  cursor: "pointer",
                  fontSize: 12,
                  color: "#6366F1",
                  fontWeight: 500,
                  fontFamily: "inherit",
                }}
              >
                <span style={{ fontSize: 14 }}>+</span>
                <span>新建文件</span>
              </button>
            </div>
          </div>
        </>
      )}

      {/* New file creation sub-panel */}
      {showFilePanel && showNewFilePanel && (
        <>
          <div
            onClick={() => {
              setShowNewFilePanel(false);
              setNewFileName("");
            }}
            style={{
              position: "fixed",
              inset: 0,
              zIndex: 9998,
              background: "transparent",
            }}
          />
          <div
            style={{
              position: "fixed",
              top: 44,
              left: 8,
              zIndex: 9999,
              background: "#fff",
              borderRadius: 8,
              boxShadow: "0 4px 20px rgba(0,0,0,0.15)",
              minWidth: 280,
              padding: "16px",
            }}
          >
            <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 12, color: "#1e293b" }}>
              新建文件
            </div>

            {/* File type selector */}
            <div style={{ marginBottom: 12 }}>
              <div style={{ fontSize: 11, color: "#6b7280", marginBottom: 6 }}>文件类型</div>
              <div style={{ display: "flex", gap: 8 }}>
                <button
                  onClick={() => setNewFileType("store")}
                  style={{
                    flex: 1,
                    padding: "6px 10px",
                    border: newFileType === "store" ? "2px solid #6366F1" : "1px solid #e2e8f0",
                    borderRadius: 6,
                    background: newFileType === "store" ? "#EEF2FF" : "#fff",
                    cursor: "pointer",
                    fontSize: 12,
                    fontWeight: newFileType === "store" ? 600 : 400,
                    color: newFileType === "store" ? "#4338CA" : "#64748b",
                    fontFamily: "inherit",
                  }}
                >
                  🗄 Store
                </button>
                <button
                  onClick={() => setNewFileType("code")}
                  style={{
                    flex: 1,
                    padding: "6px 10px",
                    border: newFileType === "code" ? "2px solid #6366F1" : "1px solid #e2e8f0",
                    borderRadius: 6,
                    background: newFileType === "code" ? "#EEF2FF" : "#fff",
                    cursor: "pointer",
                    fontSize: 12,
                    fontWeight: newFileType === "code" ? 600 : 400,
                    color: newFileType === "code" ? "#4338CA" : "#64748b",
                    fontFamily: "inherit",
                  }}
                >
                  {"</>"} Code
                </button>
              </div>
            </div>

            {/* File name input */}
            <div style={{ marginBottom: 4 }}>
              <div style={{ fontSize: 11, color: "#6b7280", marginBottom: 6 }}>文件名</div>
              <div style={{ display: "flex", alignItems: "center", gap: 2 }}>
                <input
                  type="text"
                  value={newFileName}
                  onChange={(e) => setNewFileName(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") handleCreateNewFile();
                    if (e.key === "Escape") {
                      setShowNewFilePanel(false);
                      setNewFileName("");
                    }
                  }}
                  placeholder="my-file"
                  autoFocus
                  style={{
                    flex: 1,
                    padding: "5px 8px",
                    border: "1px solid #d1d5db",
                    borderRadius: 4,
                    fontSize: 12,
                    fontFamily: "inherit",
                    outline: "none",
                  }}
                />
                <span style={{ fontSize: 12, color: "#94a3b8" }}>.omni</span>
              </div>
            </div>

            {/* Target path preview */}
            <div
              style={{
                fontSize: 10,
                color: "#94a3b8",
                marginBottom: 12,
                padding: "2px 0",
              }}
            >
              存储路径: {newFileType === "store" ? "src/stores" : "src/lib"}/
              {newFileName.trim() ? `${newFileName.trim().replace(/\.omni$/i, "")}.omni` : "..."}
            </div>

            {/* Action buttons */}
            <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
              <button
                onClick={() => {
                  setShowNewFilePanel(false);
                  setNewFileName("");
                }}
                style={{
                  padding: "5px 14px",
                  fontSize: 12,
                  border: "1px solid #d1d5db",
                  borderRadius: 6,
                  background: "#fff",
                  cursor: "pointer",
                  fontFamily: "inherit",
                }}
              >
                返回
              </button>
              <button
                onClick={handleCreateNewFile}
                disabled={!newFileName.trim()}
                style={{
                  padding: "5px 14px",
                  fontSize: 12,
                  border: "none",
                  borderRadius: 6,
                  background: newFileName.trim() ? "#6366F1" : "#d1d5db",
                  color: "#fff",
                  cursor: newFileName.trim() ? "pointer" : "default",
                  fontWeight: 600,
                  fontFamily: "inherit",
                }}
              >
                创建
              </button>
            </div>
          </div>
        </>
      )}

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
