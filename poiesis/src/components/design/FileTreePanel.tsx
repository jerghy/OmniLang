// components/design/FileTreePanel.tsx
// Hierarchical file tree panel showing .omni files for quick navigation.
// Includes "New File" button to open the creation dialog.

import { useState, useCallback, useEffect, useRef } from "react";
import { api } from "../../commands";
import { createLogger } from "../../lib/utils/logger";
import styles from "./FileTreePanel.module.css";

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
        <button type="button"
          onClick={() => onToggleDir(node.path)}
          className={styles.dirBtn}
          style={{ padding: `4px 12px 4px ${12 + indent}px` }}
        >
          <span className={styles.dirArrow}>
            {isExpanded ? "▼" : "▶"}
          </span>
          <span className={styles.dirIcon}>{isExpanded ? "📂" : "📁"}</span>
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
    <button type="button"
      onClick={() => !isOpen && onOpenFile(node.path, node.name)}
      disabled={isOpen}
      className={styles.fileBtn}
      style={{
        padding: `4px 12px 4px ${12 + indent + 14}px`,
        cursor: isOpen ? "default" : "pointer",
        color: isOpen ? "var(--ol-text-muted)" : "var(--ol-gray-700)",
        opacity: isOpen ? 0.5 : 1,
      }}
    >
      <span className={styles.fileIcon}>{isOpen ? "📌" : "📄"}</span>
      <span className={styles.fileName}>{node.name}</span>
      {isOpen && (
        <span className={styles.openBadge}>
          已打开
        </span>
      )}
    </button>
  );
}

export function FileTreePanel({ tabs, onOpenFile, onNewFile, onClose, panelRef }: FileTreePanelProps) {
  const [fileTree, setFileTree] = useState<FileTreeNode[]>([]);
  const [expandedDirs, setExpandedDirs] = useState<Set<string>>(new Set());
  const onCloseRef = useRef(onClose);
  useEffect(() => { onCloseRef.current = onClose; });

  // Close panel on outside click
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (panelRef.current && !panelRef.current.contains(e.target as Node)) {
        onCloseRef.current();
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [panelRef]);

  // Build file tree when panel opens — recursively scan for .omni files
  useEffect(() => {
    (async () => {
      const scanDir = async (dirPath: string): Promise<FileTreeNode[]> => {
        const nodes: FileTreeNode[] = [];
        try {
          const result = await api.project.listDir(dirPath);
          if (!result.entries) return nodes;
          const entries = await Promise.all(
            result.entries.map(async (entry) => {
              const entryPath = dirPath === "." ? entry.name : `${dirPath}/${entry.name}`;
              if (entry.type === "directory") {
                const children = await scanDir(entryPath);
                if (children.length > 0) {
                  return { name: entry.name, path: entryPath, type: "directory" as const, children };
                }
              } else if (entry.ext === "omni") {
                return { name: entry.name, path: entryPath, type: "file" as const, children: [] };
              }
              return null;
            })
          );
          for (const n of entries) {
            if (n) nodes.push(n);
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
      <button
        type="button"
        tabIndex={0}
        aria-label="Close file tree"
        onClick={onClose}
        onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onClose(); } }}
        className={styles.backdrop}
      />
      <div ref={panelRef} className={styles.panel}>
        <div className={styles.header}>
          <span>📁 项目文件</span>
          <span className={styles.headerStatus}>
            ({fileTree.length > 0 ? "已加载" : "扫描中..."})
          </span>
        </div>

        {/* Scrollable tree area */}
        <div className={styles.scrollArea}>
          {fileTree.length === 0 && (
            <div className={styles.emptyMsg}>
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
        <div className={styles.footer}>
          <button type="button" onClick={onNewFile} className={styles.newFileBtn}>
            <span style={{ fontSize: 14 }}>+</span>
            <span>新建文件</span>
          </button>
        </div>
      </div>
    </>
  );
}
