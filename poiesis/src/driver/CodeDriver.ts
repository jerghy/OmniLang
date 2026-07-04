// @ts-nocheck — Legacy driver code to be refactored for unified store
/** CodeDriver — code tab management, custom functions, event bindings.
 *  Extracted from OmniDriver god class. */

import { getDesignStore, getCodeGraphStore, nextId, ensureHelp } from "./store";
import { useCodeGraphStore, type CodeEditorState } from "../services/codegraph/store";
import type { StoreApi } from "zustand";
import { ElementHandle } from "./ElementHandle";
import type { ElementData, WireData } from "../store/editor";
import type { CodeTab, CodeTabVariable } from "../services/codegraph/types";
import type {
  EventBindingRecord,
  FnParam,
  PortDescriptor,
  TabInfo,
  FunctionValidationReport,
  FunctionSignature,
  TabExportData,
} from "./types";

// ═══════════════════════════════════════════════════════
// CodeDriver
// ═══════════════════════════════════════════════════════

export class CodeDriver {
  /** @internal Access the code graph store, trying global singleton first. */
  private _codeStore(): ReturnType<StoreApi<CodeEditorState>["getState"]> | null {
    const global = window.__CODE_GRAPH_STORE__;
    if (global?.getState) return global.getState();
    try { return useCodeGraphStore.getState(); } catch { return null; }
  }

  // ── Event Binding Queries (v2) ─────────────────────

  /** List all event bindings across all view elements. */
  listAllEventBindings(): EventBindingRecord[] {
    const result: EventBindingRecord[] = [];
    for (const el of getDesignStore().elements) {
      if (el.nodeType !== "view") continue;
      const handle = new ElementHandle(el.id);
      const bindings = handle.getEventBindings();
      for (const b of bindings) {
        let tabId = "";
        const codeStore = window.__CODE_GRAPH_STORE__?.getState?.();
        if (codeStore?.tabs) {
          for (const tab of codeStore.tabs) {
            const hasFn = tab.graph.elements.some(
              (e: ElementData) => e.tag === "customFn" && e.config?.fnName === b.fnName,
            );
            if (hasFn) { tabId = tab.id; break; }
          }
        }
        result.push({
          elementId: el.id, tag: el.tag,
          eventName: b.eventName, fnName: b.fnName,
          tabId, paramBindings: b.paramBindings,
        });
      }
    }
    return result;
  }

  /** Find customFn nodes matching a given event. Pass "*" as tabId to search all tabs. */
  findMatchingFunctions(
    tabId: string,
    _eventName: string,
  ): Array<{ fnName: string; params: FnParam[]; returnType: string; tabId: string }> {
    const result: Array<{ fnName: string; params: FnParam[]; returnType: string; tabId: string }> = [];
    const codeStore = window.__CODE_GRAPH_STORE__?.getState?.();
    if (!codeStore?.tabs) return result;
    for (const tab of codeStore.tabs) {
      if (tabId !== "*" && tab.id !== tabId) continue;
      for (const el of tab.graph.elements) {
        if (el.tag === "customFn" && el.config?.fnName) {
          result.push({
            fnName: el.config.fnName as string,
            params: (el.config.fnParams as FnParam[]) || [],
            returnType: (el.config.fnReturnType as string) || "void",
            tabId: tab.id,
          });
        }
      }
    }
    return result;
  }

  // ── Code Tab Management (v2) ───────────────────────

  /** Create a new code tab. */
  createTab(type: "event" | "logic", label?: string, icon?: string): { id: string } {
    ensureHelp();
    const store = this._codeStore();
    if (!store) throw new Error("[OmniDriver] Code graph store not available. Switch to code mode first.");
    const id = store.addTab(type, label || `Tab ${type}`, { icon });
    return { id };
  }

  /** Switch to a code tab by ID. */
  switchTab(tabId: string): void {
    const store = this._codeStore();
    if (!store) throw new Error("[OmniDriver] Code graph store not available.");
    store.switchTab(tabId);
  }

  /** Remove a code tab. */
  removeTab(tabId: string): void {
    const store = this._codeStore();
    if (!store) throw new Error("[OmniDriver] Code graph store not available.");
    store.removeTab(tabId);
  }

  /** List all code tabs with metadata. */
  listTabs(): TabInfo[] {
    const store = this._codeStore();
    if (!store) return [];
    return store.tabs.map((t: CodeTab) => ({
      id: t.id, type: t.type, label: t.label, icon: t.icon,
      variableCount: t.variables.length, elementCount: t.graph.elements.length,
    }));
  }

  /** Get the active tab. */
  getActiveTab(): TabInfo | null {
    const store = this._codeStore();
    if (!store) return null;
    const tab = store.getActiveTab();
    if (!tab) return null;
    return { id: tab.id, type: tab.type, label: tab.label, icon: tab.icon,
      variableCount: tab.variables.length, elementCount: tab.graph.elements.length };
  }

  /** Get a tab by ID. */
  getTab(tabId: string): TabInfo | null {
    const store = this._codeStore();
    if (!store) return null;
    const tab = store.tabs.find((t: CodeTab) => t.id === tabId);
    if (!tab) return null;
    return { id: tab.id, type: tab.type, label: tab.label, icon: tab.icon,
      variableCount: tab.variables.length, elementCount: tab.graph.elements.length };
  }

  /** Deep-clone a code tab. */
  cloneTab(tabId: string, newLabel?: string): { id: string } | null {
    const store = this._codeStore();
    if (!store) return null;
    const srcTab = store.tabs.find((t: CodeTab) => t.id === tabId);
    if (!srcTab) return null;
    const idMap = new Map<string, string>();
    const newId = () => `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
    const cloneElements = srcTab.graph.elements.map((el: ElementData) => {
      const newElId = newId(); idMap.set(el.id, newElId);
      return { ...el, id: newElId, children: [] as string[], parentId: null as string | null };
    });
    for (let i = 0; i < cloneElements.length; i++) {
      const orig = srcTab.graph.elements[i];
      if (orig.parentId && idMap.has(orig.parentId)) cloneElements[i].parentId = idMap.get(orig.parentId)!;
      if (orig.children.length > 0) {
        cloneElements[i].children = orig.children.map((cid: string) => idMap.get(cid) || cid).filter(Boolean);
      }
    }
    const cloneWires = srcTab.graph.wires.map((w: WireData) => ({
      ...w, id: newId(),
      fromId: idMap.get(w.fromId) || w.fromId,
      toId: idMap.get(w.toId) || w.toId,
    }));
    const cloneVars = srcTab.variables.map((v: CodeTabVariable) => ({ ...v }));
    const id = store.addTab(srcTab.type, newLabel || `${srcTab.label} (Copy)`, {
      icon: srcTab.icon, initialGraph: { elements: cloneElements, wires: cloneWires },
    });
    const tabState = store.tabs.find((t: CodeTab) => t.id === id);
    if (tabState) { for (const v of cloneVars) store.defineVariable(v); }
    return { id };
  }

  /** Export a tab to JSON. */
  exportTab(tabId: string): string | null {
    const store = this._codeStore();
    if (!store) return null;
    const tab = store.tabs.find((t: CodeTab) => t.id === tabId);
    if (!tab) return null;
    return JSON.stringify({
      version: "2.0", exportedAt: new Date().toISOString(),
      tab: { type: tab.type, label: tab.label, icon: tab.icon,
        variables: tab.variables, elements: tab.graph.elements, wires: tab.graph.wires },
    });
  }

  /** Import a tab from JSON. */
  importTab(json: string): { id: string } | null {
    const store = this._codeStore();
    if (!store) return null;
    try {
      const data: TabExportData = JSON.parse(json);
      if (!data.tab?.elements || !data.tab?.wires) return null;
      const id = store.addTab(data.tab.type, data.tab.label, {
        icon: data.tab.icon, initialGraph: { elements: data.tab.elements, wires: data.tab.wires },
      });
      for (const v of (data.tab.variables || [])) store.defineVariable(v);
      return { id };
    } catch { return null; }
  }

  /** Add a variable to a tab's scope. */
  addVariable(tabId: string, name: string, type: string, defaultValue?: unknown, description?: string): void {
    const store = this._codeStore();
    if (!store) throw new Error("[OmniDriver] Code graph store not available.");
    const prevTab = store.activeTabId;
    store.switchTab(tabId);
    store.defineVariable({ name, type, defaultValue, description });
    store.switchTab(prevTab);
  }

  /** Remove a variable from a tab's scope. */
  removeVariable(tabId: string, name: string): void {
    const store = this._codeStore();
    if (!store) throw new Error("[OmniDriver] Code graph store not available.");
    const prevTab = store.activeTabId;
    store.switchTab(tabId);
    store.removeVariable(name);
    store.switchTab(prevTab);
  }

  /** Get variables in a tab's scope. */
  getVariables(tabId: string): Array<{ name: string; type: string; defaultValue?: unknown; description?: string }> {
    const store = this._codeStore();
    if (!store) return [];
    const tab = store.tabs.find((t: CodeTab) => t.id === tabId);
    return tab?.variables.map((v: CodeTabVariable) => ({
      name: v.name, type: v.type, defaultValue: v.defaultValue,
      description: (v as { description?: string }).description,
    })) || [];
  }

  /** Get elements in a specific tab. */
  getTabElements(tabId: string): ElementData[] {
    const store = this._codeStore();
    if (!store) return [];
    const tab = store.tabs.find((t: CodeTab) => t.id === tabId);
    return tab?.graph.elements || [];
  }

  /** Get wires in a specific tab. */
  getTabWires(tabId: string): WireData[] {
    const store = this._codeStore();
    if (!store) return [];
    const tab = store.tabs.find((t: CodeTab) => t.id === tabId);
    return tab?.graph.wires || [];
  }

  // ── Custom Function Operations (v2) ─────────────────

  /** Create a customFn node. */
  customFunction(fnName: string, fnParams: FnParam[], fnReturnType: string): ElementHandle {
    ensureHelp();
    const inPorts: PortDescriptor[] = fnParams.map((p) => ({
      name: p.name, direction: "in", portType: "data", type: p.type, label: `${p.name}: ${p.type}`,
    }));
    const outPort: PortDescriptor = { name: "return", direction: "out", portType: "data", type: fnReturnType, label: `返回: ${fnReturnType}` };
    const id = nextId();
    const el: ElementData = {
      id, nodeType: "codeblock", tag: "customFn", props: { name: fnName }, children: [], parentId: null,
      ports: [...inPorts, outPort], nodeCategory: "function" as ElementData["nodeCategory"],
      config: { fnName, fnParams, fnReturnType, description: "" },
    };
    // customFn always goes to code graph store
    getCodeGraphStore().addElement(el);
    return new ElementHandle(id);
  }

  /** Create a return node associated with a customFn. */
  returnNode(parentFnId: string): ElementHandle {
    const cgStore = getCodeGraphStore();
    const fnEl = cgStore.getActiveGraph().elements.find((e: ElementData) => e.id === parentFnId);
    if (!fnEl || fnEl.tag !== "customFn") {
      throw new Error(`[OmniDriver] returnNode: parentFnId "${parentFnId}" is not a customFn node`);
    }
    const returnType = (fnEl.config?.fnReturnType as string) || "void";
    const id = nextId();
    const el: ElementData = {
      id, nodeType: "codeblock", tag: "return", props: { name: `return ${parentFnId.slice(-6)}` },
      children: [], parentId: parentFnId,
      ports: [{ name: "value", direction: "in", portType: "data", type: returnType, label: "返回值" }],
      nodeCategory: "function" as ElementData["nodeCategory"],
      config: { parentFnId, returnType },
    };
    getCodeGraphStore().addElement(el);
    return new ElementHandle(id);
  }

  /** Validate a customFn — scan for return nodes and check type consistency. */
  validateFunction(fnId: string): FunctionValidationReport {
    const graph = getCodeGraphStore().getActiveGraph();
    const el = graph.elements.find((e: ElementData) => e.id === fnId);
    if (!el || el.tag !== "customFn") {
      return { valid: false, returnType: "unknown", errors: [`Element "${fnId}" is not a customFn node`], reachableReturnPaths: 0 };
    }
    const expectedType = (el.config?.fnReturnType as string) || "void";
    const wires = graph.wires;

    // BFS from customFn to find all reachable return nodes
    const visited = new Set<string>();
    const queue = [fnId];
    const reachableReturnNodes: ElementData[] = [];
    while (queue.length > 0) {
      const current = queue.shift()!;
      if (visited.has(current)) continue;
      visited.add(current);
      for (const w of wires.filter((wr: WireData) => wr.fromId === current)) {
        const target = graph.elements.find((e: ElementData) => e.id === w.toId);
        if (!target) continue;
        if (target.tag === "return") { reachableReturnNodes.push(target); }
        else { queue.push(target.id); }
      }
    }

    // Also find return nodes that reference this fn as parent (fallback for disconnected tests)
    const parentRefReturns = graph.elements.filter(
      (e: ElementData) => e.tag === "return" && e.config?.parentFnId === fnId,
    );
    const allReturnNodes = [...new Map(
      [...reachableReturnNodes, ...parentRefReturns].map((n) => [n.id, n]),
    ).values()];

    const errors: string[] = [];
    if (allReturnNodes.length === 0) {
      errors.push("No return node reachable from customFn. A function must have at least one return path.");
    }
    for (const rn of allReturnNodes) {
      const port = rn.ports?.find((p) => p.name === "value");
      const actualType = port?.type || "any";
      if (actualType !== expectedType && actualType !== "any" && expectedType !== "any") {
        errors.push(`Return type mismatch at node ${rn.id}: value port type "${actualType}" but function expects "${expectedType}"`);
      }
    }

    return {
      valid: errors.length === 0,
      returnType: expectedType,
      errors,
      reachableReturnPaths: allReturnNodes.length,
    };
  }

  /** List all customFn nodes across all tabs AND the editor store. */
  listCustomFunctions(): Array<{ fnName: string; params: FnParam[]; returnType: string; tabId: string; description: string }> {
    const result: Array<{ fnName: string; params: FnParam[]; returnType: string; tabId: string; description: string }> = [];
    // Search editor store (design)
    for (const el of getDesignStore().elements) {
      if (el.tag === "customFn" && el.config?.fnName) {
        result.push({
          fnName: el.config.fnName as string,
          params: (el.config.fnParams as FnParam[]) || [],
          returnType: (el.config.fnReturnType as string) || "void",
          tabId: "design", description: (el.config.description as string) || "",
        });
      }
    }
    // Search code graph store (code mode tabs)
    const codeStore = window.__CODE_GRAPH_STORE__?.getState?.();
    if (codeStore?.tabs) {
      for (const tab of codeStore.tabs) {
        for (const el of tab.graph.elements) {
          if (el.tag === "customFn" && el.config?.fnName) {
            result.push({
              fnName: el.config.fnName as string,
              params: (el.config.fnParams as FnParam[]) || [],
              returnType: (el.config.fnReturnType as string) || "void",
              tabId: tab.id, description: (el.config.description as string) || "",
            });
          }
        }
      }
    }
    return result;
  }

  /** Find a customFn by name (searches editor store + code tabs). */
  findFunctionByName(fnName: string): { fnName: string; params: FnParam[]; returnType: string; tabId: string; description: string; nodeId: string } | null {
    // Search editor store first
    for (const el of getDesignStore().elements) {
      if (el.tag === "customFn" && el.config?.fnName === fnName) {
        return {
          fnName: el.config.fnName as string,
          params: (el.config.fnParams as FnParam[]) || [],
          returnType: (el.config.fnReturnType as string) || "void",
          tabId: "design", description: (el.config.description as string) || "",
          nodeId: el.id,
        };
      }
    }
    // Search code graph store
    const codeStore = window.__CODE_GRAPH_STORE__?.getState?.();
    if (codeStore?.tabs) {
      for (const tab of codeStore.tabs) {
        for (const el of tab.graph.elements) {
          if (el.tag === "customFn" && el.config?.fnName === fnName) {
            return {
              fnName: el.config.fnName as string,
              params: (el.config.fnParams as FnParam[]) || [],
              returnType: (el.config.fnReturnType as string) || "void",
              tabId: tab.id, description: (el.config.description as string) || "",
              nodeId: el.id,
            };
          }
        }
      }
    }
    return null;
  }

  /** Check for function name conflicts. */
  checkFunctionNameConflict(fnName: string): Array<{ fnName: string; tabId: string }> {
    const result: Array<{ fnName: string; tabId: string }> = [];
    const fns = this.listCustomFunctions();
    for (const f of fns) { if (f.fnName === fnName) result.push({ fnName: f.fnName, tabId: f.tabId }); }
    return result;
  }

  /** Get function signature summary. */
  getFunctionSignature(fnId: string): FunctionSignature | null {
    const el = getDesignStore().elements.find((e) => e.id === fnId);
    if (!el || el.tag !== "customFn") return null;
    const wires = getDesignStore().wires;
    const visited = new Set<string>([fnId]);
    const queue = [fnId];
    while (queue.length > 0) {
      const current = queue.shift()!;
      for (const w of wires.filter((wr) => wr.fromId === current)) {
        if (!visited.has(w.toId)) { visited.add(w.toId); queue.push(w.toId); }
      }
    }
    return {
      fnName: (el.config?.fnName as string) || "",
      params: (el.config?.fnParams as FnParam[]) || [],
      returnType: (el.config?.fnReturnType as string) || "void",
      bodyNodeCount: visited.size - 1,
    };
  }
}
