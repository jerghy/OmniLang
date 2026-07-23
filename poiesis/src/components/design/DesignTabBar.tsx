// components/design/DesignTabBar.tsx
// Tab bar showing open .omni files with dirty indicator and close button.
// "+" button opens a hierarchical file tree panel of .omni files.
// Includes "New File" button for creating store or code files.

import { useState, useRef, useCallback, useEffect } from "react";
import { usePoiesisStore } from "../../store";
import { detectTabType } from "../../core/types/design-page";
import type { TabType } from "../../core/types/design-page";
import { FileTreePanel } from "./FileTreePanel";
import { NewFileModal } from "./NewFileModal";
import { ConfirmCloseDialog } from "./ConfirmCloseDialog";
import { api } from "../../commands";
import styles from "./DesignTabBar.module.css";
import { emitDesignSource } from "../../services/codegen/design-emitter";
import { parseOmniSource } from "../../services/codegen/omni-parser";
import { parseStyleValue } from "../../services/codegen/design-value-parser";
import { uid } from "../../core/utils/id";
import { EMPTY_DESIGN, type TokenType, tokenTypeToCategory } from "../../core/types/design";

// ── Constants ──

const MENU_ITEM_STYLE: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: 8,
  width: "100%",
  padding: "8px 14px",
  border: "none",
  background: "transparent",
  cursor: "pointer",
  fontSize: 12,
  color: "#374151",
  textAlign: "left" as const,
  fontFamily: "inherit",
};

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

  // "+" menu
  const [showPlusMenu, setShowPlusMenu] = useState(false);
  const [plusMenuPos, setPlusMenuPos] = useState({ top: 0, left: 0 });
  const plusMenuRef = useRef<HTMLDivElement>(null);
  const plusBtnRef = useRef<HTMLButtonElement>(null);

  // "+" file tree panel
  const [showFilePanel, setShowFilePanel] = useState(false);
  const panelRef = useRef<HTMLDivElement>(null);

  // New file sub-panel
  const [showNewFilePanel, setShowNewFilePanel] = useState(false);



// Close "+" menu on outside click
  useEffect(() => {
    if (!showPlusMenu) return;
    const handler = (e: MouseEvent) => {
      const target = e.target as Node;
      if (plusMenuRef.current && !plusMenuRef.current.contains(target) &&
          plusBtnRef.current && !plusBtnRef.current.contains(target)) {
        setShowPlusMenu(false);
      }
    };
    // Delay to avoid immediately closing on the same click that opened it
    const timer = setTimeout(() => document.addEventListener("mousedown", handler), 0);
    return () => {
      clearTimeout(timer);
      document.removeEventListener("mousedown", handler);
    };
  }, [showPlusMenu]);

  const handleOpenFile = useCallback(
    (filePath: string, fileName: string) => {
      const componentName = fileName.replace(/\.omni$/, "");
      const tabType = detectTabType(filePath);
      // ── Deduplicate design.omni: if any design tab is open, redirect to canonical ──
      if (tabType === "design") {
        const existingDesignTab = tabs.find((t) => t.tabType === "design");
        if (existingDesignTab) {
          switchTab(existingDesignTab.id);
          // Load content from the requested file into the design store
          api.project.readFile(filePath).then((res) => {
            if (res.content) {
              const result = parseOmniSource(res.content);
              if ((result.designTokens?.length ?? 0) > 0 || (result.designClasses?.length ?? 0) > 0 || (result.designVariants?.length ?? 0) > 0) {
                const { loadDesign } = usePoiesisStore.getState();
                loadDesign({
                  name: result.designName || "AppDesign",
                  tokens: (result.designTokens ?? []).map((t) => ({
                    id: t.id, name: t.name, category: tokenTypeToCategory(t.tokenType), tokenType: t.tokenType as TokenType, value: t.value,
                  })),
                  classes: (result.designClasses ?? []).map((c) => ({
                    id: c.id,
                    name: c.name,
                    properties: (c.properties ?? []).map((p) => ({
                      id: uid("sp"),
                      name: p.name,
                      segments: parseStyleValue(p.value),
                    })),
                    pseudoStates: (c.pseudoStates ?? []).map((ps) => ({
                      id: uid("ps"),
                      pseudo: ps.pseudo,
                      properties: ps.properties.map((p) => ({
                        id: uid("sp"),
                        name: p.name,
                        segments: parseStyleValue(p.value),
                      })),
                    })),
                  })),
                  variants: (result.designVariants ?? []).map((v) => ({
                    id: v.id,
                    name: v.name,
                    dimensions: (v.dimensions ?? []).map((d) => ({
                      name: d.name,
                      values: d.values.map((dv) => dv.name),
                    })),
                    defaults: v.defaults ?? {},
                    baseProperties: (v.baseProperties ?? []).map((p) => ({
                      id: uid("sp"),
                      name: p.name,
                      segments: parseStyleValue(p.value),
                    })),
                    branches: (v.branches ?? []).map((b) => {
                      const dotIdx = b.name.indexOf(".");
                      const dim = dotIdx >= 0 ? b.name.slice(0, dotIdx) : "";
                      const val = dotIdx >= 0 ? b.name.slice(dotIdx + 1) : b.name;
                      const props = b.properties.map((p) => ({
                        id: uid("sp"),
                        name: p.name,
                        segments: parseStyleValue(p.value),
                      }));
                      return { id: uid("vb"), dimension: dim, value: val, properties: props };
                    }),
                  })),
                });
              }
            }
          }).catch((e) => { console.warn("Tab operation failed", e); });
          setShowFilePanel(false);
          return;
        }
      }
      openTab(filePath, componentName, tabType);
      setShowFilePanel(false);
      // If opening a design.omni file, load its content into the design store
      if (tabType === "design") {
        api.project.readFile(filePath).then((res) => {
          if (res.content) {
            const result = parseOmniSource(res.content);
            if ((result.designTokens?.length ?? 0) > 0 || (result.designClasses?.length ?? 0) > 0 || (result.designVariants?.length ?? 0) > 0) {
              const { loadDesign } = usePoiesisStore.getState();
              loadDesign({
                name: result.designName || "AppDesign",
                tokens: (result.designTokens ?? []).map((t) => ({
                  id: t.id, name: t.name, category: tokenTypeToCategory(t.tokenType), tokenType: t.tokenType as TokenType, value: t.value,
                })),
                classes: (result.designClasses ?? []).map((c) => ({
                  id: c.id,
                  name: c.name,
                  properties: (c.properties ?? []).map((p) => ({
                    id: uid("sp"),
                    name: p.name,
                    segments: parseStyleValue(p.value),
                  })),
                  pseudoStates: (c.pseudoStates ?? []).map((ps) => ({
                    id: uid("ps"),
                    pseudo: ps.pseudo,
                    properties: ps.properties.map((p) => ({
                      id: uid("sp"),
                      name: p.name,
                      segments: parseStyleValue(p.value),
                    })),
                  })),
                })),
                variants: (result.designVariants ?? []).map((v) => ({
                  id: v.id,
                  name: v.name,
                  dimensions: (v.dimensions ?? []).map((d) => ({
                    name: d.name,
                    values: d.values.map((dv) => dv.name),
                  })),
                  defaults: v.defaults ?? {},
                  baseProperties: (v.baseProperties ?? []).map((p) => ({
                    id: uid("sp"),
                    name: p.name,
                    segments: parseStyleValue(p.value),
                  })),
                  branches: (v.branches ?? []).map((b) => {
                    const dotIdx = b.name.indexOf(".");
                    const dim = dotIdx >= 0 ? b.name.slice(0, dotIdx) : "";
                    const val = dotIdx >= 0 ? b.name.slice(dotIdx + 1) : b.name;
                    const props = b.properties.map((p) => ({
                      id: uid("sp"),
                      name: p.name,
                      segments: parseStyleValue(p.value),
                    }));
                    return { id: uid("vb"), dimension: dim, value: val, properties: props };
                  }),
                })),
              });
            }
          }
        }).catch((e) => { console.warn("Tab operation failed", e); });
      }

      // Load inline store definitions for component files
      if (tabType !== "design" && tabType !== "store") {
        api.project.readFile(filePath).then((res) => {
          if (res.content) {
            const result = parseOmniSource(res.content);
            if (result.inlineStoreDefs) {
              const { loadInlineStoreDef } = usePoiesisStore.getState();
              for (const def of Object.values(result.inlineStoreDefs)) {
                // Re-key: parser uses PascalCase from source (e.g. "Main.TestStore"),
                // but tab's componentName may differ in case (e.g. "main" from main.omni).
                const dotIdx = def.name.indexOf(".");
                const storeName = dotIdx >= 0 ? def.name.slice(dotIdx + 1) : def.name;
                const rekeyed = { ...def, name: `${componentName}.${storeName}` };
                loadInlineStoreDef(rekeyed);
              }
            }
          }
        }).catch((e) => { console.warn("Failed to load inline stores", e); });
      }
    },
    [openTab, tabs, switchTab]
  );

  const handleCreateNewFile = useCallback(
    (name: string, type: "store" | "code") => {
      const sanitized = name.trim().replace(/\.omni$/i, "");
      const fullName = `${sanitized}.omni`;
      const dir = type === "store" ? "src/stores" : "src/lib";
      const filePath = `${dir}/${fullName}`;
      const tabType: TabType = type;
      openTab(filePath, sanitized, tabType);
      setShowNewFilePanel(false);
      setShowFilePanel(false);
      // Write stub file to disk so it survives reload
      const stubContent = type === "store"
        ? `store ${sanitized} {\n}\n`
        : `// ${sanitized}.omni\n`;
      api.project.writeFile(filePath, stubContent).catch((e) => {
        console.warn("Failed to write stub file:", e);
      });
    },
    [openTab]
  );

  const handleDesignOmni = useCallback(async () => {
    setShowPlusMenu(false);
    const canonicalPath = "src/design.omni";

    // ── Check if any design tab already exists ──
    const existingDesignTab = tabs.find((t) => t.tabType === "design");
    if (existingDesignTab) {
      switchTab(existingDesignTab.id);
      // If the existing tab has a non-canonical path (e.g. src/design.omni),
      // read from canonical path
      const readPath = existingDesignTab.filePath !== canonicalPath
        ? canonicalPath
        : existingDesignTab.filePath;
      const res = await api.project.readFile(readPath);
      if (res.content) {
        const result = parseOmniSource(res.content);
        if ((result.designTokens?.length ?? 0) > 0
            || (result.designClasses?.length ?? 0) > 0
            || (result.designVariants?.length ?? 0) > 0) {
          const { loadDesign } = usePoiesisStore.getState();
          loadDesign({
            name: result.designName || "AppDesign",
            tokens: (result.designTokens ?? []).map((t) => ({
              id: t.id, name: t.name, category: tokenTypeToCategory(t.tokenType), tokenType: t.tokenType as TokenType, value: t.value,
            })),
            classes: (result.designClasses ?? []).map((c) => ({
              id: c.id,
              name: c.name,
              properties: (c.properties ?? []).map((p) => ({
                id: uid("sp"),
                name: p.name,
                segments: parseStyleValue(p.value),
              })),
              pseudoStates: (c.pseudoStates ?? []).map((ps) => ({
                id: uid("ps"),
                pseudo: ps.pseudo,
                properties: ps.properties.map((p) => ({
                  id: uid("sp"),
                  name: p.name,
                  segments: parseStyleValue(p.value),
                })),
              })),
            })),
            variants: (result.designVariants ?? []).map((v) => ({
              id: v.id,
              name: v.name,
              dimensions: (v.dimensions ?? []).map((d) => ({
                name: d.name,
                values: d.values.map((dv) => dv.name),
              })),
              defaults: v.defaults ?? {},
              baseProperties: (v.baseProperties ?? []).map((p) => ({
                id: uid("sp"),
                name: p.name,
                segments: parseStyleValue(p.value),
              })),
              branches: (v.branches ?? []).map((b) => {
                const dotIdx = b.name.indexOf(".");
                const dim = dotIdx >= 0 ? b.name.slice(0, dotIdx) : "";
                const val = dotIdx >= 0 ? b.name.slice(dotIdx + 1) : b.name;
                const props = b.properties.map((p) => ({
                  id: uid("sp"),
                  name: p.name,
                  segments: parseStyleValue(p.value),
                }));
                return { id: uid("vb"), dimension: dim, value: val, properties: props };
              }),
            })),
          });
        }
      }
      return;
    }

    // ── Read existing canonical design.omni or create if missing ──
    const existing = await api.project.readFile(canonicalPath);
    if (existing.content && !existing.error) {
      // File exists with content — parse it
      openTab(canonicalPath, "design", "design");
      const result = parseOmniSource(existing.content);
      if ((result.designTokens?.length ?? 0) > 0
          || (result.designClasses?.length ?? 0) > 0
          || (result.designVariants?.length ?? 0) > 0) {
        const { loadDesign } = usePoiesisStore.getState();
        loadDesign({
          name: result.designName || "AppDesign",
          tokens: (result.designTokens ?? []).map((t) => ({
            id: t.id, name: t.name, category: tokenTypeToCategory(t.tokenType), tokenType: t.tokenType as TokenType, value: t.value,
          })),
          classes: (result.designClasses ?? []).map((c) => ({
            id: c.id,
            name: c.name,
            properties: (c.properties ?? []).map((p) => ({
              id: uid("sp"),
              name: p.name,
              segments: parseStyleValue(p.value),
            })),
            pseudoStates: (c.pseudoStates ?? []).map((ps) => ({
              id: uid("ps"),
              pseudo: ps.pseudo,
              properties: ps.properties.map((p) => ({
                id: uid("sp"),
                name: p.name,
                segments: parseStyleValue(p.value),
              })),
            })),
          })),
          variants: (result.designVariants ?? []).map((v) => ({
            id: v.id,
            name: v.name,
            dimensions: (v.dimensions ?? []).map((d) => ({
              name: d.name,
              values: d.values.map((dv) => dv.name),
            })),
            defaults: v.defaults ?? {},
            baseProperties: (v.baseProperties ?? []).map((p) => ({
              id: uid("sp"),
              name: p.name,
              segments: parseStyleValue(p.value),
            })),
            branches: (v.branches ?? []).map((b) => {
              const dotIdx = b.name.indexOf(".");
              const dim = dotIdx >= 0 ? b.name.slice(0, dotIdx) : "";
              const val = dotIdx >= 0 ? b.name.slice(dotIdx + 1) : b.name;
              const props = b.properties.map((p) => ({
                id: uid("sp"),
                name: p.name,
                segments: parseStyleValue(p.value),
              }));
              return { id: uid("vb"), dimension: dim, value: val, properties: props };
            }),
          })),
        });
      }
    } else {
      // File doesn't exist — create empty and open
      const source = emitDesignSource({ ...EMPTY_DESIGN, name: "AppDesign" });
      await api.project.writeFile(canonicalPath, source);
      openTab(canonicalPath, "design", "design");
    }
  }, [tabs, switchTab, openTab]);

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
      <div className={styles.tabBar}>
        {orderedTabs.map((tab) => {
          const isActive = tab.id === activeTabId;
          const isDragOver = tab.id === dragOverTabId;
          return (
            <div
              key={tab.id}
              role="tab"
              tabIndex={0}
              draggable
              onDragStart={() => handleDragStart(tab.id)}
              onDragOver={(e) => handleDragOver(e, tab.id)}
              onDrop={() => handleDrop(tab.id)}
              onDragEnd={handleDragEnd}
              onClick={(e) => { if ((e.target as HTMLElement).closest('[data-action="close-tab"]')) { handleClose(e as unknown as React.MouseEvent, tab.id); } switchTab(tab.id); }}
              onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); switchTab(tab.id); } }}
              className={styles.tabItem}
              style={{
                borderTop: isActive ? "2px solid #6366F1" : "2px solid transparent",
                background: isActive ? "#fff" : "transparent",
                color: isActive ? "#1e293b" : "#64748b",
                opacity: isDragOver ? 0.7 : 1,
              }}
            >
              <span>
                {tab.isDirty && <span style={{ color: "#f59e0b" }}>● </span>}
                {tab.componentName}.omni
              </span>
              <span data-action="close-tab" className={styles.tabCloseBtn} title="关闭">
                ×
              </span>
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
              color: "#64748b",
            }}
          >
            没有打开的 .omni 文件
          </div>
        )}

        {/* "+" button with dropdown menu */}
        <div style={{ display: "flex", alignItems: "center", padding: "0 2px", position: "relative" }}>
          <button type="button"
            ref={plusBtnRef}
            onClick={(e) => {
              e.stopPropagation();
              const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
              setPlusMenuPos({ top: rect.bottom + 2, left: rect.left });
              setShowPlusMenu(!showPlusMenu);
            }}
            title="添加文件"
            className={styles.addTabBtn}
            style={{ background: showPlusMenu ? "#e2e8f0" : "transparent" }}
          >
            +
          </button>

          {/* "+" dropdown menu */}
          {showPlusMenu && (
              <div
                ref={plusMenuRef}
                className={styles.plusMenuDropdown}
                style={{ top: plusMenuPos.top, left: plusMenuPos.left }}
              >
                <button type="button"
                  onClick={() => { setShowPlusMenu(false); setShowFilePanel(true); }}
                  style={MENU_ITEM_STYLE}
                  onMouseEnter={(e) => { (e.currentTarget as HTMLElement).style.background = "#f1f5f9"; }}
                  onMouseLeave={(e) => { (e.currentTarget as HTMLElement).style.background = "transparent"; }}
                >
                  <span>📁 打开 .omni 文件...</span>
                </button>
                <div style={{ borderTop: "1px solid #f1f5f9" }} />
                <button type="button"
                  onClick={handleDesignOmni}
                  style={MENU_ITEM_STYLE}
                  onMouseEnter={(e) => { (e.currentTarget as HTMLElement).style.background = "#f1f5f9"; }}
                  onMouseLeave={(e) => { (e.currentTarget as HTMLElement).style.background = "transparent"; }}
                >
                  <span>🎨 打开 design.omni</span>
                </button>
              </div>
          )}
        </div>
      </div>

      {/* File tree panel */}
      {showFilePanel && !showNewFilePanel && (
        <FileTreePanel
          tabs={tabs}
          panelRef={panelRef as React.RefObject<HTMLDivElement | null>}
          onOpenFile={handleOpenFile}
          onNewFile={() => setShowNewFilePanel(true)}
          onClose={() => setShowFilePanel(false)}
        />
      )}

      {/* New file creation sub-panel */}
      {showFilePanel && showNewFilePanel && (
        <NewFileModal
          onCreate={handleCreateNewFile}
          onClose={() => {
            setShowNewFilePanel(false);
            setShowFilePanel(false);
          }}
        />
      )}

      {/* Confirm dialog for dirty tab close */}
      {confirmClose && (
        <ConfirmCloseDialog
          onConfirm={confirmAndClose}
          onCancel={() => setConfirmClose(null)}
        />
      )}
    </>
  );
}
