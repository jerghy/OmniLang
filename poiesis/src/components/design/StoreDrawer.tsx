import { useState, useCallback, useEffect, useMemo } from "react";
import { usePoiesisStore } from "../../store";
import { StorePage } from "../../pages/StorePage";
import type { StoreDefinition } from "../../core/types/store";
import { uid } from "../../core/utils/id";
import styles from "./StoreDrawer.module.css";

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
    <div className={styles.drawer}>
      {/* Header */}
      <div className={styles.header}>
        {/* Store switcher */}
        <div className={styles.switcherWrapper}>
          <button type="button"
            onClick={() => setShowStoreSwitcher(!showStoreSwitcher)}
            className={styles.switcherBtn}
          >
            <span className={styles.switcherDot}>●</span>
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
                className={styles.switcherBackdrop}
              />
              <div className={styles.switcherDropdown}>
                {allStores.map(s => (
                  <button key={s.key}
                    type="button"
                    className={styles.storeOption}
                    style={{
                      color: s.key === activeStore ? "#6366f1" : "#374151",
                      fontWeight: s.key === activeStore ? 600 : 400,
                    }}
                    onClick={(e) => { if ((e.target as HTMLElement).getAttribute('data-action') === 'delete-store') { handleDeleteStore(s.key, s.type); } else { handleSelectStore(s.key); } }}
                    onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); handleSelectStore(s.key); } }}
                    onMouseEnter={e => { (e.currentTarget as HTMLElement).style.background = "#f8fafc"; }}
                    onMouseLeave={e => { (e.currentTarget as HTMLElement).style.background = "transparent"; }}
                  >
                    <span className={styles.storeOptionIcon}>
                      {s.key === activeStore ? "●" : "○"}
                    </span>
                    <span className={styles.storeOptionName}>{s.key}</span>
                    <span data-action="delete-store" className={styles.storeDeleteBtn}>✕</span>
                  </button>
                ))}
                <div className={styles.storeDivider}>
                  <button type="button"
                    onClick={() => { setShowStoreSwitcher(false); setShowCreateModal(true); }}
                    className={styles.newStoreBtn}
                  >
                    + 新建 Store
                  </button>
                </div>
              </div>
            </>
          )}
        </div>

        {/* Right buttons */}
        <div className={styles.rightBtns}>
          <button type="button"
            onClick={handleOpenInTab}
            title="在标签页中打开"
            className={styles.iconBtn}
          >📋</button>
          <button type="button"
            onClick={handleClose}
            className={styles.closeBtn}
          >✕</button>
        </div>
      </div>

      {/* Body */}
      <div className={styles.bodyArea}>
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
    <div className={styles.emptyGuide}>
      <div className={styles.emptyIcon}>📦</div>
      <div className={styles.emptyTitle}>还没有 Store</div>
      <div className={styles.emptyDesc}>使用 Store 管理状态、信号和操作</div>
      <button type="button" onClick={onCreate} className={styles.emptyCreateBtn}>
        + 新建 Store
      </button>
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
      <button type="button" tabIndex={0} aria-label="Close new store modal" onClick={onClose} onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onClose(); } }} className={styles.modalBackdrop} />
      <div className={styles.modal}>
        <div className={styles.modalTitle}>
          新建 Store
        </div>

        <div className={styles.radioGroup}>
          <label className={styles.radioLabel}>
            <input type="radio" name="storeType" checked={storeType === "inline"}
              onChange={() => setStoreType("inline")} />
            <div>
              <div className={styles.radioLabelTitle}>内联 Store</div>
              <div className={styles.radioLabelDesc}>保存在当前组件 .omni 文件中</div>
            </div>
          </label>
          <label className={styles.radioLabel}>
            <input type="radio" name="storeType" checked={storeType === "global"}
              onChange={() => setStoreType("global")} />
            <div>
              <div className={styles.radioLabelTitle}>全局 Store</div>
              <div className={styles.radioLabelDesc}>独立文件 src/stores/xxx.omni</div>
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
          className={styles.inputField}
        />

        <div className={styles.btnRow}>
          <button type="button" onClick={onClose} className={styles.cancelBtn}>取消</button>
          <button type="button" onClick={() => { if (name.trim()) onCreate(name, storeType); }} className={styles.createBtn}>创建</button>
        </div>
      </div>
    </>
  );
}
