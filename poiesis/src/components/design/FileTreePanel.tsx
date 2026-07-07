// components/design/FileTreePanel.tsx
// Hierarchical file tree panel showing .omni files for quick navigation.
// Includes "New File" button to open the creation dialog.

import { useState, useCallback, useEffect } from "react";
import { api } from "../../commands";
import { createLogger } from "../../lib/utils/logger";

const log = createLogger("FileTreePanel");

interface FileTreeNode {
  name: string;
  path: string;
  type: "directory" | "file";
  children: FileTreeNode[];
}

interface FileTreePanelProps {
  tabs: { filePath: string }[];
  onOpenFile: (filePath: string, name: string) => void;
  onNewFile: () => void;
  onClose: () => void;
  panelRef: React.RefObject<HTMLDivElement | null>;
}

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

export function FileTreePanel({ tabs, onOpenFile, onNewFile, onClose, panelRef }: FileTreePanelProps) {
  const [fileTree, setFileTree] = useState<FileTreeNode[]>([]);
  const [expandedDirs, setExpandedDirs] = useState<Set<string>>(new Set());

  // Close panel on outside click
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (panelRef.current && !panelRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [onClose, panelRef]);

  // Build file tree when panel opens — recursively scan for .omni files
  useEffect(() => {
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
        } catch (e) { log.debug("Could not scan directory:", e); }
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
  }, []);

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

  return (
    <>
      <div
        onClick={onClose}
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
              onOpenFile={onOpenFile}
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
            onClick={onNewFile}
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
  );
}
