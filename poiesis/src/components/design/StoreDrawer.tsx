import { useState, useCallback, useEffect, useMemo } from "react";
import { usePoiesisStore } from "../../store";
import { StorePage } from "../../pages/StorePage";
import type { StoreDefinition } from "../../core/types/store";
import { uid } from "../../core/utils/id";

export function StoreDrawer() {
  const visible = usePoiesisStore((s) => s.designPage.storePanelVisible);
  const activeStore = usePoiesisStore((s) => s.designPage.storePanelActiveStore);
  const setStorePanelVisible = usePoiesisStore((s) => s.setStorePanelVisible);
  const setStorePanelActiveStore = usePoiesisStore((s) => s.setStorePanelActiveStore);
  const globalStoreDefs = usePoiesisStore((s) => s.storeDefs);
  const inlineStoreDefs = usePoiesisStore((s) => s.inlineStoreDefs);
  const loadInlineStoreDef = usePoiesisStore((s) => s.loadInlineStoreDef);
  const removeInlineStoreDef = usePoiesisStore((s) => s.removeInlineStoreDef);
  const activeTab = usePoiesisStore((s) => {
    const id = s.designPage.activeTabId;
    return id ? s.designPage.openTabs.find(t => t.id === id) : null;
  });
  const openTab = usePoiesisStore((s) => s.openTab);

  const [showStoreSwitcher, setShowStoreSwitcher] = useState(false);
  const [showCreateModal, setShowCreateModal] = useState(false);

  // All stores (global + inline), deduplicated
  const allStores = useMemo(() => [
    ...Object.entries(globalStoreDefs).map(([name, def]) => ({ key: name, name, def, type: "global" as const })),
    ...Object.entries(inlineStoreDefs).map(([key, def]) => ({ key, name: key, def, type: "inline" as const })),
  ], [globalStoreDefs, inlineStoreDefs]);

  // ESC key to close
  useEffect(() => {
    if (!visible) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") setStorePanelVisible(false);
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [visible, setStorePanelVisible]);

  // Auto-select first store on open
  useEffect(() => {
    if (visible && !activeStore && allStores.length > 0) {
      setStorePanelActiveStore(allStores[0].key);
    }
  }, [visible, activeStore, allStores, setStorePanelActiveStore]);

  const handleClose = useCallback(() => setStorePanelVisible(false), [setStorePanelVisible]);

  const handleSelectStore = useCallback((key: string) => {
    setStorePanelActiveStore(key);
    setShowStoreSwitcher(false);
  }, [setStorePanelActiveStore]);

  const handleDeleteStore = useCallback((key: string, type: "global" | "inline") => {
    if (type === "global") {
      usePoiesisStore.getState().removeStoreDef(key);
    } else {
      removeInlineStoreDef(key);
    }
    // If deleting active store, switch to first remaining
    if (key === activeStore) {
      const remaining = Object.keys(type === "global" ? globalStoreDefs : inlineStoreDefs)
        .filter(k => k !== key);
      setStorePanelActiveStore(remaining[0] ?? null);
    }
  }, [activeStore, globalStoreDefs, inlineStoreDefs, removeInlineStoreDef, setStorePanelActiveStore]);

  const handleCreateStore = useCallback((name: string, type: "global" | "inline") => {
    const sanitized = name.trim();
    if (!sanitized) return;
    const pid = uid("s");
    const key = type === "inline"
      ? `${activeTab?.componentName ?? "main"}.${sanitized}`
      : sanitized;
    const def: StoreDefinition = {
      pid,
      name: key, // inline stores use "ComponentName.StoreName" as name
      filePath: type === "global" ? `src/stores/${sanitized}.omni` : (activeTab?.filePath ?? ""),
      imports: ["import omniview;"],
      signals: [],
      getters: [],
      actions: [],
    };
    if (type === "global") {
      usePoiesisStore.getState().loadStoreDef(def);
    } else {
      loadInlineStoreDef(def);
    }
    setStorePanelActiveStore(key);
    setShowCreateModal(false);
  }, [activeTab, loadInlineStoreDef, setStorePanelActiveStore]);

  const handleOpenInTab = useCallback(() => {
    if (!activeStore) return;
    // Determine if this store is global or inline
    const isGlobal = activeStore in globalStoreDefs;
    if (isGlobal) {
      const def = globalStoreDefs[activeStore];
      openTab(def.filePath, activeStore, "store");
    } else {
      // Inline stores must be promoted to global — or open a component tab
      // For now, just open the parent component tab
      const tab = activeTab;
      if (tab) openTab(tab.filePath, tab.componentName, "component");
    }
    setStorePanelVisible(false);
  }, [activeStore, globalStoreDefs, activeTab, openTab, setStorePanelVisible]);

  if (!visible) return null;

  const currentStore = allStores.find(s => s.key === activeStore);

  return (
    <div style={{
      position: "absolute",
      top: 0, right: 0, bottom: 0, left: 0,
      zIndex: 10,
      display: "flex",
      flexDirection: "column",
      background: "var(--ol-surface, #fff)",
      borderLeft: "1px solid #e2e8f0",
      boxShadow: "-4px 0 20px rgba(0,0,0,0.08)",
      animation: "storeDrawerIn 250ms cubic-bezier(0.16, 1, 0.3, 1)",
    }}>
      <style>{`
        @keyframes storeDrawerIn {
          from { transform: translateX(100%); }
          to { transform: translateX(0); }
        }
      `}</style>

      {/* Header */}
      <div style={{
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        padding: "6px 12px",
        borderBottom: "1px solid #e2e8f0",
        background: "#f8fafc",
        minHeight: 36,
      }}>
        {/* Store switcher */}
        <div style={{ position: "relative" }}>
          <button type="button"
            onClick={() => setShowStoreSwitcher(!showStoreSwitcher)}
            style={{
              display: "flex", alignItems: "center", gap: 6,
              padding: "3px 10px",
              fontSize: 12, fontWeight: 600,
              border: "1px solid #e2e8f0", borderRadius: 6,
              background: "#fff", cursor: "pointer",
              color: "#1e293b",
              fontFamily: "inherit",
            }}
          >
            <span style={{ color: "#6366f1", fontSize: 12 }}>●</span>
            {activeStore ?? "选择 Store"}
          </button>

          {showStoreSwitcher && (
            <>
              <button
                type="button"
                tabIndex={0}
                aria-label="Close store switcher"
                onClick={() => setShowStoreSwitcher(false)}
                onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); setShowStoreSwitcher(false); } }}
                style={{ position: "fixed", inset: 0, zIndex: 99, border: "none", cursor: "pointer", background: "transparent" }}
              />
              <div style={{
                position: "absolute", top: "100%", left: 0,
                marginTop: 4,
                background: "#fff",
                border: "1px solid #e2e8f0",
                borderRadius: 8,
                boxShadow: "0 4px 16px rgba(0,0,0,0.12)",
                zIndex: 100,
                minWidth: 220,
                overflow: "hidden",
              }}>
                {allStores.map(s => (
                  <div key={s.key}
                    role="button"
                    tabIndex={0}
                    style={{
                    display: "flex", alignItems: "center",
                    padding: "8px 14px",
                    cursor: "pointer",
                    fontSize: 12,
                    color: s.key === activeStore ? "#6366f1" : "#374151",
                    fontWeight: s.key === activeStore ? 600 : 400,
                    borderBottom: "1px solid #f1f5f9",
                  }}
                    onClick={() => handleSelectStore(s.key)}
                    onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); handleSelectStore(s.key); } }}
                    onMouseEnter={e => { (e.currentTarget as HTMLElement).style.background = "#f8fafc"; }}
                    onMouseLeave={e => { (e.currentTarget as HTMLElement).style.background = "transparent"; }}
                  >
                    <span style={{ marginRight: 8 }}>
                      {s.key === activeStore ? "●" : "○"}
                    </span>
                    <span style={{ flex: 1 }}>{s.key}</span>
                    <button type="button"
                      onClick={e => { e.stopPropagation(); handleDeleteStore(s.key, s.type); }}
                      style={{
                        border: "none", background: "transparent",
                        cursor: "pointer", fontSize: 14,
                        color: "#94a3b8", padding: "0 4px",
                      }}
                    >✕</button>
                  </div>
                ))}
                <div style={{ borderTop: "1px solid #e2e8f0" }}>
                  <button type="button"
                    onClick={() => { setShowStoreSwitcher(false); setShowCreateModal(true); }}
                    style={{
                      width: "100%", padding: "8px 14px",
                      border: "none", background: "transparent",
                      cursor: "pointer", fontSize: 12,
                      color: "#6366f1", fontWeight: 600,
                      textAlign: "left", fontFamily: "inherit",
                    }}
                  >
                    + 新建 Store
                  </button>
                </div>
              </div>
            </>
          )}
        </div>

        {/* Right buttons */}
        <div style={{ display: "flex", gap: 4 }}>
          <button type="button"
            onClick={handleOpenInTab}
            title="在标签页中打开"
            style={{
              padding: "2px 8px", fontSize: 13,
              border: "1px solid #e2e8f0", borderRadius: 4,
              background: "transparent", cursor: "pointer",
            }}
          >📋</button>
          <button type="button"
            onClick={handleClose}
            style={{
              padding: "2px 8px", fontSize: 16,
              border: "none", background: "transparent",
              cursor: "pointer", color: "#94a3b8",
            }}
          >✕</button>
        </div>
      </div>

      {/* Body */}
      <div style={{ flex: 1, overflow: "hidden" }}>
        {currentStore ? (
          <StorePage
            storeName={currentStore.key}
            tabId="__store_panel__"
            inlineMode={currentStore.type === "inline"}
          />
        ) : (
          <EmptyStoreGuide onCreate={() => setShowCreateModal(true)} />
        )}
      </div>

      {/* Create Store Modal */}
      {showCreateModal && (
        <CreateStoreModal
          onClose={() => setShowCreateModal(false)}
          onCreate={handleCreateStore}
        />
      )}
    </div>
  );
}

function EmptyStoreGuide({ onCreate }: { onCreate: () => void }) {
  return (
    <div style={{
      display: "flex", flexDirection: "column",
      alignItems: "center", justifyContent: "center",
      height: "100%", color: "#94a3b8", gap: 12,
    }}>
      <div style={{ fontSize: 36 }}>📦</div>
      <div style={{ fontSize: 14, fontWeight: 600 }}>还没有 Store</div>
      <div style={{ fontSize: 12 }}>使用 Store 管理状态、信号和操作</div>
      <button type="button"
        onClick={onCreate}
        style={{
          marginTop: 8,
          padding: "6px 16px", fontSize: 12, fontWeight: 600,
          border: "none", borderRadius: 6,
          background: "#4f46e5", color: "#fff",
          cursor: "pointer", fontFamily: "inherit",
        }}
      >+ 新建 Store</button>
    </div>
  );
}

function CreateStoreModal({
  onClose, onCreate,
}: {
  onClose: () => void;
  onCreate: (name: string, type: "global" | "inline") => void;
}) {
  const [storeType, setStoreType] = useState<"inline" | "global">("inline");
  const [name, setName] = useState("");

  return (
    <>
      <button type="button" tabIndex={0} aria-label="Close new store modal" onClick={onClose} onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onClose(); } }} style={{ position: "fixed", inset: 0, background: "rgba(0,0,0,0.3)", zIndex: 1000, border: "none", cursor: "pointer" }} />
      <div style={{
        position: "fixed", top: "50%", left: "50%",
        transform: "translate(-50%, -50%)",
        zIndex: 1001,
        background: "#fff", borderRadius: 12,
        padding: "24px 28px",
        boxShadow: "0 8px 32px rgba(0,0,0,0.2)",
        minWidth: 320,
      }}>
        <div style={{ fontSize: 16, fontWeight: 700, marginBottom: 16, color: "#1e293b" }}>
          新建 Store
        </div>

        <div style={{ display: "flex", flexDirection: "column", gap: 8, marginBottom: 16 }}>
          <label style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 12, cursor: "pointer" }}>
            <input type="radio" name="storeType" checked={storeType === "inline"}
              onChange={() => setStoreType("inline")} />
            <div>
              <div style={{ fontWeight: 600 }}>内联 Store</div>
              <div style={{ color: "#94a3b8", fontSize: 12 }}>保存在当前组件 .omni 文件中</div>
            </div>
          </label>
          <label style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 12, cursor: "pointer" }}>
            <input type="radio" name="storeType" checked={storeType === "global"}
              onChange={() => setStoreType("global")} />
            <div>
              <div style={{ fontWeight: 600 }}>全局 Store</div>
              <div style={{ color: "#94a3b8", fontSize: 12 }}>独立文件 src/stores/xxx.omni</div>
            </div>
          </label>
        </div>

        <input
          placeholder="Store 名称"
          aria-label="Store name"
          value={name}
          onChange={e => setName(e.target.value)}
          onKeyDown={e => {
            if (e.key === "Enter" && name.trim()) onCreate(name, storeType);
            if (e.key === "Escape") onClose();
          }}
          style={{
            width: "100%", padding: "6px 10px", fontSize: 13,
            border: "1px solid #d1d5db", borderRadius: 6,
            boxSizing: "border-box", marginBottom: 16,
            fontFamily: "inherit",
          }}
        />

        <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
          <button type="button" onClick={onClose} style={{
            padding: "5px 14px", fontSize: 12,
            border: "1px solid #d1d5db", borderRadius: 6,
            background: "#fff", cursor: "pointer", fontFamily: "inherit",
          }}>取消</button>
          <button type="button" onClick={() => { if (name.trim()) onCreate(name, storeType); }} style={{
            padding: "5px 14px", fontSize: 12, fontWeight: 600,
            border: "none", borderRadius: 6,
            background: "#4f46e5", color: "#fff",
            cursor: "pointer", fontFamily: "inherit",
          }}>创建</button>
        </div>
      </div>
    </>
  );
}
