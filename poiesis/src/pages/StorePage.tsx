// pages/StorePage.tsx
// Top-level component for the Store editor tab.
// Left panel: StoreMembersPanel with Signals/Getters/Actions.
// Center: FlowCanvas for action body code-flow editing.

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

/** Create a default empty StoreDefinition for a previously-unseen store. */
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

  // Read selectedActionPid from tabData
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
    // Save current action's elements/wires back to storeDefs
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

    // Load selected action's elements/wires into store
    if (currentStore) {
      const action = currentStore.actions.find((a) => a.pid === pid);
      if (action) {
        setElementsAndWires(action.elements, action.wires);
      }
    }

    // Update tabData with new selection
    usePoiesisStore.setState((s) => ({
      tabData: {
        ...s.tabData,
        [tabId]: { ...s.tabData[tabId], selectedActionPid: pid },
      },
    }));
  }, [storeName, tabId, updateStoreAction, setElementsAndWires]);

  // Load first action on mount if none selected
  useEffect(() => {
    if (!selectedActionPid && store && store.actions.length > 0) {
      handleSelectAction(store.actions[0].pid);
    }
    // Only run on mount
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  if (!store) {
    return <div style={{ padding: 32, textAlign: "center", color: "#999" }}>Store not loaded</div>;
  }

  return (
    <div style={{ display: "flex", height: "100%", width: "100%" }}>
      {/* Left: Members Panel */}
      <div style={{ width: 300, minWidth: 200, height: "100%" }}>
        <StoreMembersPanel
          storeName={storeName}
          selectedActionPid={selectedActionPid}
          onSelectAction={handleSelectAction}
        />
      </div>

      {/* Center: CodeFlow Canvas */}
      <div style={{ flex: 1, height: "100%" }}>
        {selectedActionPid ? (
          <ReactFlowProvider>
            <FlowCanvas />
          </ReactFlowProvider>
        ) : (
          <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100%", color: "#999" }}>
            Select an action to edit
          </div>
        )}
      </div>
    </div>
  );
}
