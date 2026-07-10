// pages/StorePage.tsx
// Top-level component for the Store editor tab.
// Left panel: StoreMembersPanel. Center: FlowCanvas for action code-flow editing.

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

export function StorePage({ storeName, tabId }: Props) {
  const store = usePoiesisStore((s) => s.storeDefs[storeName]);
  const loadStoreDef = usePoiesisStore((s) => s.loadStoreDef);
  const registerStorePids = usePoiesisStore((s) => s.registerStorePids);
  const setElementsAndWires = usePoiesisStore((s) => s.setElementsAndWires);
  const tabData = usePoiesisStore((s) => s.tabData);
  const updateStoreAction = usePoiesisStore((s) => s.updateStoreAction);
  const tabs = usePoiesisStore((s) => s.designPage.openTabs);

  const selectedActionPid = tabData[tabId]?.selectedActionPid ?? null;

  // Auto-create default StoreDefinition on first open
  useEffect(() => {
    if (!store) {
      const tab = tabs.find((t) => t.id === tabId);
      const filePath = tab?.filePath ?? `src/stores/${storeName}.omni`;
      const def = createDefaultStore(storeName, filePath);
      loadStoreDef(def);
      registerStorePids(def);
    }
  }, [store, storeName, tabId, tabs, loadStoreDef, registerStorePids]);

  const handleSelectAction = useCallback((pid: string) => {
    const state = usePoiesisStore.getState();
    const currentStore = state.storeDefs[storeName];
    const currentElements = state.elements;
    const currentWires = state.wires;
    const currentSelectedPid = state.tabData[tabId]?.selectedActionPid ?? null;

    if (currentSelectedPid && currentStore) {
      const currentAction = currentStore.actions.find((a) => a.pid === currentSelectedPid);
      if (currentAction) {
        const codeblockElements = currentElements.filter((e) => e.nodeType === "codeblock");
        updateStoreAction(storeName, currentSelectedPid, {
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
  }, [storeName, tabId, updateStoreAction, setElementsAndWires]);

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
