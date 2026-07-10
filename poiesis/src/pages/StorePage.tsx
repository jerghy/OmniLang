// pages/StorePage.tsx
// Top-level component for the Store editor tab.
// Left panel: StoreMembersPanel. Center: FlowCanvas for action code-flow editing.
// Supports inlineMode for component-scoped stores in the drawer.

import { useCallback, useEffect } from "react";
import type { CodeblockElementData } from "../core/types/element";
import type { StoreDefinition } from "../core/types/store";
import { usePoiesisStore } from "../store";
import { StoreMembersPanel } from "../components/store/StoreMembersPanel";
import { FlowCanvas } from "../components/canvas/code/FlowCanvas";
import { ReactFlowProvider } from "@xyflow/react";
import { uid } from "../core/utils/id";

interface Props {
  storeName: string;
  tabId: string;
  /** When true, reads/writes inlineStoreDefs instead of storeDefs. */
  inlineMode?: boolean;
}

function createDefaultStore(storeName: string, filePath: string): StoreDefinition {
  const storePid = uid("s");
  return {
    pid: storePid,
    name: storeName,
    filePath,
    imports: ["import omniview;"],
    signals: [],
    getters: [],
    actions: [],
  };
}

export function StorePage({ storeName, tabId, inlineMode = false }: Props) {
  // Select from correct store slice
  const globalStore = usePoiesisStore((s) => s.storeDefs[storeName]);
  const inlineStore = usePoiesisStore((s) => s.inlineStoreDefs[storeName]);
  const store = inlineMode ? inlineStore : globalStore;

  // Store lifecycle actions
  const loadStoreDef = usePoiesisStore((s) => s.loadStoreDef);
  const loadInlineStoreDef = usePoiesisStore((s) => s.loadInlineStoreDef);
  const registerStorePids = usePoiesisStore((s) => s.registerStorePids);
  const setElementsAndWires = usePoiesisStore((s) => s.setElementsAndWires);
  const tabData = usePoiesisStore((s) => s.tabData);
  const updateStoreAction = usePoiesisStore((s) => s.updateStoreAction);
  const updateInlineStoreAction = usePoiesisStore((s) => s.updateInlineStoreAction);
  const tabs = usePoiesisStore((s) => s.designPage.openTabs);

  // Inline store CRUD actions (passed as callbacks to StoreMembersPanel)
  const addInlineStoreSignal = usePoiesisStore((s) => s.addInlineStoreSignal);
  const updateInlineStoreSignal = usePoiesisStore((s) => s.updateInlineStoreSignal);
  const removeInlineStoreSignal = usePoiesisStore((s) => s.removeInlineStoreSignal);
  const addInlineStoreGetter = usePoiesisStore((s) => s.addInlineStoreGetter);
  const updateInlineStoreGetter = usePoiesisStore((s) => s.updateInlineStoreGetter);
  const removeInlineStoreGetter = usePoiesisStore((s) => s.removeInlineStoreGetter);
  const addInlineStoreAction = usePoiesisStore((s) => s.addInlineStoreAction);
  const removeInlineStoreAction = usePoiesisStore((s) => s.removeInlineStoreAction);

  const selectedActionPid = tabData[tabId]?.selectedActionPid ?? null;

  // Auto-create default StoreDefinition on first open
  useEffect(() => {
    if (!store) {
      const tab = tabs.find((t) => t.id === tabId);
      const filePath = tab?.filePath ?? `src/stores/${storeName}.omni`;
      const def = createDefaultStore(storeName, filePath);
      if (inlineMode) {
        loadInlineStoreDef(def);
      } else {
        loadStoreDef(def);
      }
      registerStorePids(def);
    }
  }, [store, storeName, tabId, tabs, loadStoreDef, loadInlineStoreDef, registerStorePids, inlineMode]);

  // Sync fn node name/params → StoreActionDef in real-time
  useEffect(() => {
    const state = usePoiesisStore.getState();
    const activePid = state.tabData[tabId]?.selectedActionPid;
    if (!activePid || !store) return;
    const action = store.actions.find((a) => a.pid === activePid);
    if (!action) return;

    const codeblockElements = state.elements.filter((e) => e.nodeType === "codeblock");
    const fnEl = codeblockElements.find((e) => e.tag === "fn");
    if (!fnEl) return;

    const fnName = fnEl.props?.name as string | undefined;
    const fnParamsStr = (fnEl.props?.params as string) || "";
    const parsedParams: Array<{ name: string; type: string }> = fnParamsStr
      ? fnParamsStr.split(",").map((p) => {
          const [pName, pType] = p.trim().split(":").map((s) => s.trim());
          return { name: pName || "", type: pType || "String" };
        })
      : [];

    const needsSync =
      (fnName !== undefined && fnName !== action.name) ||
      JSON.stringify(parsedParams) !== JSON.stringify(action.params);

    if (needsSync) {
      const updates: Partial<typeof action> = {};
      if (fnName !== undefined && fnName !== action.name) updates.name = fnName;
      if (JSON.stringify(parsedParams) !== JSON.stringify(action.params)) updates.params = parsedParams;
      const updateAction = inlineMode
        ? state.updateInlineStoreAction
        : state.updateStoreAction;
      updateAction(storeName, activePid, updates);
    }
  });

  const handleSelectAction = useCallback((pid: string) => {
    const state = usePoiesisStore.getState();
    const stores = inlineMode ? state.inlineStoreDefs : state.storeDefs;
    const currentStore = stores[storeName];
    const currentElements = state.elements;
    const currentWires = state.wires;
    const currentSelectedPid = state.tabData[tabId]?.selectedActionPid ?? null;

    if (currentSelectedPid && currentStore) {
      const currentAction = currentStore.actions.find((a) => a.pid === currentSelectedPid);
      if (currentAction) {
        const codeblockElements = currentElements.filter((e) => e.nodeType === "codeblock");
        const fnEl = codeblockElements.find((e) => e.tag === "fn");
        const nameFromFn = (fnEl?.props?.name as string) || currentAction.name;
        const paramsFromFn = (fnEl?.props?.params as string) || "";
        const parsedParams = paramsFromFn
          ? paramsFromFn.split(",").map((p) => {
              const [pName, pType] = p.trim().split(":").map((s) => s.trim());
              return { name: pName || "", type: pType || "String" };
            })
          : currentAction.params;
        const updateAction = inlineMode
          ? state.updateInlineStoreAction
          : state.updateStoreAction;
        updateAction(storeName, currentSelectedPid, {
          name: nameFromFn,
          params: parsedParams,
          elements: codeblockElements as CodeblockElementData[],
          wires: currentWires,
        });
      }
    }

    if (currentStore) {
      const action = currentStore.actions.find((a) => a.pid === pid);
      if (action) {
        setElementsAndWires(action.elements, action.wires);
      }
    }

    usePoiesisStore.setState((s) => ({
      tabData: {
        ...s.tabData,
        [tabId]: { ...s.tabData[tabId], selectedActionPid: pid },
      },
    }));
  }, [storeName, tabId, inlineMode, setElementsAndWires]);

  useEffect(() => {
    if (!selectedActionPid && store && store.actions.length > 0) {
      handleSelectAction(store.actions[0].pid);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  if (!store) {
    return (
      <div style={{
        flex: 1,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        color: "var(--ol-text-muted)",
        fontSize: "var(--ol-font-size-13)",
      }}>
        Loading store…
      </div>
    );
  }

  return (
    <div style={{ display: "flex", height: "100%", width: "100%" }}>
      <div style={{ width: 280, minWidth: 200, height: "100%", flexShrink: 0 }}>
        <StoreMembersPanel
          storeName={storeName}
          selectedActionPid={selectedActionPid}
          onSelectAction={handleSelectAction}
          storeOverride={inlineMode ? store : undefined}
          onAddSignal={inlineMode ? (sig) => addInlineStoreSignal(storeName, sig) : undefined}
          onUpdateSignal={inlineMode ? (pid, updates) => updateInlineStoreSignal(storeName, pid, updates) : undefined}
          onRemoveSignal={inlineMode ? (pid) => removeInlineStoreSignal(storeName, pid) : undefined}
          onAddGetter={inlineMode ? (g) => addInlineStoreGetter(storeName, g) : undefined}
          onUpdateGetter={inlineMode ? (pid, updates) => updateInlineStoreGetter(storeName, pid, updates) : undefined}
          onRemoveGetter={inlineMode ? (pid) => removeInlineStoreGetter(storeName, pid) : undefined}
          onAddAction={inlineMode ? (a) => addInlineStoreAction(storeName, a) : undefined}
          onRemoveAction={inlineMode ? (pid) => removeInlineStoreAction(storeName, pid) : undefined}
        />
      </div>

      <div style={{ flex: 1, height: "100%", borderLeft: "1px solid var(--ol-border)" }}>
        {selectedActionPid ? (
          <ReactFlowProvider>
            <FlowCanvas />
          </ReactFlowProvider>
        ) : (
          <div style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            height: "100%",
            color: "var(--ol-text-muted)",
            fontSize: "var(--ol-font-size-13)",
          }}>
            Select an action to edit
          </div>
        )}
      </div>
    </div>
  );
}
