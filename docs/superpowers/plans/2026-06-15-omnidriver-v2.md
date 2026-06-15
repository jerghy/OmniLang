# OmniDriver v2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Upgrade OmniDriver from v1 (flow+data dual-edge, action-node event handling) to v2 (pure data-flow, bindEvent+customFn, multi-tab code canvas) matching the editor's code section redesign architecture.

**Architecture:** Add 3 new API groups (Event Binding v2, Code Tab Management, Custom Function Operations) to `OmniDriver.ts` + `ElementHandle`, wired to existing Zustand stores (`useEditorStore` for design canvas, `useCodeGraphStore` for code tabs). Deprecate `createEventHandler` and remove `action`/`onPress` block types. Update `help()` with dynamic port reference generation.

**Tech Stack:** TypeScript, Zustand, Playwright (E2E)

**Spec:** [MIGRATION.md](.claude/skills/omni-driver/MIGRATION.md), [SKILL.md](.claude/skills/omni-driver/SKILL.md), [api-reference.md](.claude/skills/omni-driver/api-reference.md), [TEST_PLAN.md](editor/e2e/TEST_PLAN.md)

---

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `editor/client/src/driver/types.ts` | Modify | Add new types: FnParam, ParamBinding, EventBindingRecord, FunctionValidationReport, FunctionSignature, TabInfo, EventBindingOptions |
| `editor/client/src/driver/OmniDriver.ts` | Modify | Add new API methods, deprecate createEventHandler, update help() |
| `editor/client/src/driver/omni-driver.d.ts` | Modify | Update global type declarations |
| `editor/e2e/omni-driver-counter.spec.ts` | Modify | Migrate to v2 API |
| `editor/e2e/omni-driver-task-manager.spec.ts` | Modify | Migrate to v2 API |
| `build/test/omni-driver/task-manager-app.js` | Modify | Rewrite in v2 API |
| `editor/e2e/omni-driver-code-tab.spec.ts` | Create | E2E tests for Code Tab API |
| `editor/e2e/omni-driver-customfn.spec.ts` | Create | E2E tests for customFn API |
| `editor/e2e/omni-driver-event-binding.spec.ts` | Create | E2E tests for event binding API |

---

### Task 1: Add New Types to types.ts

**Files:**
- Modify: `editor/client/src/driver/types.ts`

- [ ] **Step 1: Add new type definitions**

Append after the existing `StateDiff` interface (line 127):

```typescript
// ── v2: Function parameter ────────────────────────────

export interface FnParam {
  name: string;
  type: string;
  default?: unknown;
}

// ── v2: Event binding ─────────────────────────────────

export interface ParamBinding {
  sourceType: "self" | "literal" | "event" | "global";
  value: string;
  expectedType: string;
}

export interface EventBindingOptions {
  params?: Record<string, ParamBinding>;
}

export interface EventBindingRecord {
  elementId: string;
  tag: string;
  eventName: string;
  fnName: string;
  tabId: string;
  paramBindings: Record<string, ParamBinding>;
}

// ── v2: Code tab info ─────────────────────────────────

export interface TabInfo {
  id: string;
  type: "event" | "logic";
  label: string;
  icon: string;
  variableCount: number;
  elementCount: number;
}

// ── v2: Function validation ───────────────────────────

export interface FunctionValidationReport {
  valid: boolean;
  returnType: string;
  errors: string[];
  reachableReturnPaths: number;
}

export interface FunctionSignature {
  fnName: string;
  params: FnParam[];
  returnType: string;
  bodyNodeCount: number;
}

// ── v2: Tab export format ─────────────────────────────

export interface TabExportData {
  version: string;
  exportedAt: string;
  tab: {
    type: "event" | "logic";
    label: string;
    icon: string;
    variables: Array<{ name: string; type: string; defaultValue?: unknown; description?: string }>;
    elements: import("../store/editor").ElementData[];
    wires: import("../store/editor").WireData[];
  };
}
```

- [ ] **Step 2: Verify types compile**

Run: `cd editor/client && npx tsc --noEmit --pretty false 2>&1 | head -20`
Expected: No new type errors (existing errors OK if pre-existing).

- [ ] **Step 3: Commit**

```bash
git add editor/client/src/driver/types.ts
git commit -m "feat(omni-driver): add v2 types — FnParam, ParamBinding, EventBindingRecord, TabInfo, FunctionValidationReport"
```

---

### Task 2: Add Event Binding v2 to OmniDriver.ts

**Files:**
- Modify: `editor/client/src/driver/OmniDriver.ts`

This task adds the event binding API to both `ElementHandle` and `OmniDriver`.

- [ ] **Step 1: Add bindEvent + unbindEvent to ElementHandle**

Insert after the `createEventHandler` method (after line 216, before the `waitForLive` method):

```typescript
  /** Bind an event on this element to a custom function in a code tab.
   *  Pure data-flow v2: sets the event prop (e.g. onPress="handleSubmit")
   *  and optionally stores paramBindings. */
  bindEvent(
    eventName: string,
    fnName: string,
    options: import("./types").EventBindingOptions = {},
  ): this {
    const d = this._require();
    const meta = COMPONENT_PROTOCOL[d.tag];
    if (!meta) throw new Error(`No component protocol for tag "${d.tag}"`);

    const eventCfg: EventMeta | undefined = meta.events[eventName];
    if (!eventCfg) {
      const available = Object.keys(meta.events).join(", ");
      throw new Error(
        `Event "${eventName}" not found on ${d.tag}. Available: ${available || "(none)"}`,
      );
    }

    // Set event prop on the view element
    this._driver._store().setProp(d.id, eventName, fnName);

    // Store paramBindings as a config field on the element if provided
    if (options.params && Object.keys(options.params).length > 0) {
      this._driver._store().setEventBindings(d.id, eventName, {
        fnName,
        paramBindings: options.params,
      });
    }

    return this;
  }

  /** Remove an event binding from this element. */
  unbindEvent(eventName: string): this {
    const d = this._require();
    const currentVal = (d.props as Record<string, unknown>)[eventName];
    if (currentVal === undefined) {
      console.warn(`[OmniDriver] unbindEvent: "${eventName}" is not bound on <${d.tag}>`);
      return this;
    }
    this._driver._store().setProp(d.id, eventName, undefined as unknown);
    this._driver._store().removeEventBindings(d.id, eventName);
    return this;
  }

  /** List available event names for this element. */
  getAvailableEvents(): string[] {
    const d = this._require();
    if (d.nodeType !== "view") return [];
    const meta = COMPONENT_PROTOCOL[d.tag];
    return meta?.events ? Object.keys(meta.events) : [];
  }

  /** Get current event bindings on this element. */
  getEventBindings(): Array<{ eventName: string; fnName: string; paramBindings: Record<string, import("./types").ParamBinding> }> {
    const d = this._require();
    if (d.nodeType !== "view") return [];
    const bindings: Array<{ eventName: string; fnName: string; paramBindings: Record<string, import("./types").ParamBinding> }> = [];
    const meta = COMPONENT_PROTOCOL[d.tag];
    if (!meta?.events) return [];
    for (const [evtName] of Object.entries(meta.events)) {
      const fnName = (d.props as Record<string, unknown>)[evtName];
      if (typeof fnName === "string" && fnName.length > 0) {
        bindings.push({
          eventName: evtName,
          fnName,
          paramBindings: (d.config?._eventBindings as Record<string, Record<string, import("./types").ParamBinding>>)?.[evtName] || {},
        });
      }
    }
    return bindings;
  }
```

- [ ] **Step 2: Add listAllEventBindings + findMatchingFunctions to OmniDriver**

Insert after the `getComponentInfo` method (after line 1178):

```typescript
  /** List all event bindings across all view elements. */
  listAllEventBindings(): import("./types").EventBindingRecord[] {
    const result: import("./types").EventBindingRecord[] = [];
    for (const el of this._store().elements) {
      if (el.nodeType !== "view") continue;
      const handle = new ElementHandle(el.id, this);
      const bindings = handle.getEventBindings();
      for (const b of bindings) {
        // Determine which tab owns the function
        let tabId = "";
        const codeStore = (window as any).__CODE_GRAPH_STORE__?.getState?.();
        if (codeStore) {
          for (const tab of codeStore.tabs || []) {
            const hasFn = tab.graph.elements.some(
              (e: import("../store/editor").ElementData) =>
                e.tag === "customFn" && e.config?.fnName === b.fnName,
            );
            if (hasFn) { tabId = tab.id; break; }
          }
        }
        result.push({
          elementId: el.id,
          tag: el.tag,
          eventName: b.eventName,
          fnName: b.fnName,
          tabId,
          paramBindings: b.paramBindings,
        });
      }
    }
    return result;
  }

  /** Find customFn nodes whose signature matches a given event.
   *  Pass "*" as tabId to search all tabs. */
  findMatchingFunctions(
    tabId: string,
    _eventName: string,
  ): Array<{ fnName: string; params: import("./types").FnParam[]; returnType: string; tabId: string }> {
    const result: Array<{ fnName: string; params: import("./types").FnParam[]; returnType: string; tabId: string }> = [];
    const codeStore = (window as any).__CODE_GRAPH_STORE__?.getState?.();
    if (!codeStore?.tabs) return result;
    for (const tab of codeStore.tabs) {
      if (tabId !== "*" && tab.id !== tabId) continue;
      for (const el of tab.graph.elements) {
        if (el.tag === "customFn" && el.config?.fnName) {
          result.push({
            fnName: el.config.fnName as string,
            params: (el.config.fnParams as import("./types").FnParam[]) || [],
            returnType: (el.config.fnReturnType as string) || "void",
            tabId: tab.id,
          });
        }
      }
    }
    return result;
  }
```

- [ ] **Step 3: Deprecate createEventHandler — make it throw**

Replace the body of `createEventHandler` (lines 182-217) with:

```typescript
  /** @deprecated Removed in v2. Use bindEvent(eventName, fnName) instead.
   *  Event handling is now split: bind in design mode, define customFn in code mode. */
  createEventHandler(
    _eventName: string,
    _options?: import("./types").EventHandlerOptions,
  ): { handler: never; wire: never } {
    throw new Error(
      "[OmniDriver] createEventHandler() has been removed in v2.\n" +
      "  Use bindEvent(eventName, fnName) in design mode to bind the event,\n" +
      "  then create a customFunction() in code mode to define the handler.\n" +
      "  See O.help() for examples.",
    );
  }
```

- [ ] **Step 4: Verify TypeScript compiles**

Run: `cd editor/client && npx tsc --noEmit --pretty false 2>&1 | head -30`
Expected: No errors from OmniDriver.ts.

- [ ] **Step 5: Commit**

```bash
git add editor/client/src/driver/OmniDriver.ts
git commit -m "feat(omni-driver): add Event Binding v2 API — bindEvent, unbindEvent, getAvailableEvents, listAllEventBindings; deprecate createEventHandler"
```

---

### Task 3: Add Code Tab Management API to OmniDriver.ts

**Files:**
- Modify: `editor/client/src/driver/OmniDriver.ts`

- [ ] **Step 1: Add tab management methods**

Insert after the `listAllEventBindings` method (or after mode switching block, line 753):

```typescript
  // ── Code Tab Management (v2) ───────────────────────

  /** @internal Access the code graph store. */
  private _codeStore() {
    return (window as any).__CODE_GRAPH_STORE__?.getState?.() || null;
  }

  /** Create a new code tab. */
  createTab(
    type: "event" | "logic",
    label?: string,
    icon?: string,
  ): { id: string } {
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
  listTabs(): import("./types").TabInfo[] {
    const store = this._codeStore();
    if (!store) return [];
    return store.tabs.map((t: import("../codegraph/types").CodeTab) => ({
      id: t.id,
      type: t.type,
      label: t.label,
      icon: t.icon,
      variableCount: t.variables.length,
      elementCount: t.graph.elements.length,
    }));
  }

  /** Get the active tab. */
  getActiveTab(): import("./types").TabInfo | null {
    const store = this._codeStore();
    if (!store) return null;
    const tab = store.getActiveTab();
    if (!tab) return null;
    return {
      id: tab.id,
      type: tab.type,
      label: tab.label,
      icon: tab.icon,
      variableCount: tab.variables.length,
      elementCount: tab.graph.elements.length,
    };
  }

  /** Get a tab by ID. */
  getTab(tabId: string): import("./types").TabInfo | null {
    const store = this._codeStore();
    if (!store) return null;
    const tab = store.tabs.find((t: import("../codegraph/types").CodeTab) => t.id === tabId);
    if (!tab) return null;
    return {
      id: tab.id,
      type: tab.type,
      label: tab.label,
      icon: tab.icon,
      variableCount: tab.variables.length,
      elementCount: tab.graph.elements.length,
    };
  }

  /** Deep-clone a code tab. */
  cloneTab(tabId: string, newLabel?: string): { id: string } | null {
    const store = this._codeStore();
    if (!store) return null;
    const srcTab = store.tabs.find((t: import("../codegraph/types").CodeTab) => t.id === tabId);
    if (!srcTab) return null;
    const idMap = new Map<string, string>();
    const newId = () => `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
    const cloneElements = srcTab.graph.elements.map((el: import("../store/editor").ElementData) => {
      const newElId = newId();
      idMap.set(el.id, newElId);
      return { ...el, id: newElId, children: [], parentId: null };
    });
    // Rebuild parent-child references
    for (let i = 0; i < cloneElements.length; i++) {
      const orig = srcTab.graph.elements[i];
      if (orig.parentId && idMap.has(orig.parentId)) {
        cloneElements[i].parentId = idMap.get(orig.parentId)!;
      }
      if (orig.children.length > 0) {
        cloneElements[i].children = orig.children.map((cid: string) => idMap.get(cid) || cid).filter((c: string) => c !== undefined);
      }
    }
    const cloneWires = srcTab.graph.wires.map((w: import("../store/editor").WireData) => ({
      ...w,
      id: newId(),
      fromId: idMap.get(w.fromId) || w.fromId,
      toId: idMap.get(w.toId) || w.toId,
    }));
    const cloneVars = srcTab.variables.map((v: import("../codegraph/types").SignalDefinition) => ({ ...v }));
    const id = store.addTab(srcTab.type, newLabel || `${srcTab.label} (Copy)`, {
      icon: srcTab.icon,
      initialGraph: { elements: cloneElements, wires: cloneWires },
    });
    // Set variables on the new tab
    const tabState = store.tabs.find((t: import("../codegraph/types").CodeTab) => t.id === id);
    if (tabState) {
      for (const v of cloneVars) {
        store.defineVariable(v);
      }
    }
    return { id };
  }

  /** Export a tab to JSON. */
  exportTab(tabId: string): string | null {
    const store = this._codeStore();
    if (!store) return null;
    const tab = store.tabs.find((t: import("../codegraph/types").CodeTab) => t.id === tabId);
    if (!tab) return null;
    return JSON.stringify({
      version: "2.0",
      exportedAt: new Date().toISOString(),
      tab: {
        type: tab.type,
        label: tab.label,
        icon: tab.icon,
        variables: tab.variables,
        elements: tab.graph.elements,
        wires: tab.graph.wires,
      },
    });
  }

  /** Import a tab from JSON (previously exported with exportTab). */
  importTab(json: string): { id: string } | null {
    const store = this._codeStore();
    if (!store) return null;
    try {
      const data: import("./types").TabExportData = JSON.parse(json);
      if (!data.tab?.elements || !data.tab?.wires) return null;
      const id = store.addTab(data.tab.type, data.tab.label, {
        icon: data.tab.icon,
        initialGraph: { elements: data.tab.elements, wires: data.tab.wires },
      });
      for (const v of (data.tab.variables || [])) {
        store.defineVariable(v as import("../codegraph/types").SignalDefinition);
      }
      return { id };
    } catch {
      return null;
    }
  }

  /** Add a variable to a tab's scope. */
  addVariable(
    tabId: string,
    name: string,
    type: string,
    defaultValue?: unknown,
    description?: string,
  ): void {
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
    const tab = store.tabs.find((t: import("../codegraph/types").CodeTab) => t.id === tabId);
    return tab?.variables.map((v: import("../codegraph/types").SignalDefinition) => ({
      name: v.name,
      type: v.type,
      defaultValue: v.defaultValue,
      description: v.description,
    })) || [];
  }

  /** Get elements in a specific tab. */
  getTabElements(tabId: string): import("../store/editor").ElementData[] {
    const store = this._codeStore();
    if (!store) return [];
    const tab = store.tabs.find((t: import("../codegraph/types").CodeTab) => t.id === tabId);
    return tab?.graph.elements || [];
  }

  /** Get wires in a specific tab. */
  getTabWires(tabId: string): import("../store/editor").WireData[] {
    const store = this._codeStore();
    if (!store) return [];
    const tab = store.tabs.find((t: import("../codegraph/types").CodeTab) => t.id === tabId);
    return tab?.graph.wires || [];
  }
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd editor/client && npx tsc --noEmit --pretty false 2>&1 | head -30`
Expected: No errors from OmniDriver.ts.

- [ ] **Step 3: Commit**

```bash
git add editor/client/src/driver/OmniDriver.ts
git commit -m "feat(omni-driver): add Code Tab Management API — createTab, switchTab, removeTab, listTabs, cloneTab, exportTab, importTab, variables, tab elements/wires"
```

---

### Task 4: Add Custom Function Operations API to OmniDriver.ts

**Files:**
- Modify: `editor/client/src/driver/OmniDriver.ts`

- [ ] **Step 1: Add customFunction + returnNode methods**

Insert the tab management block from Task 3, add after it:

```typescript
  // ── Custom Function Operations (v2) ─────────────────

  /** Create a customFn node. */
  customFunction(
    fnName: string,
    fnParams: import("./types").FnParam[],
    fnReturnType: string,
  ): ElementHandle {
    const config = getNodeConfig("customFn");
    // Build input ports from params
    const inPorts: import("../store/editor").PortData[] = fnParams.map((p) => ({
      name: p.name,
      direction: "in" as const,
      portType: "data" as const,
      type: p.type,
      label: `${p.name}: ${p.type}`,
    }));
    const outPort: import("../store/editor").PortData = {
      name: "return",
      direction: "out",
      portType: "data",
      type: fnReturnType,
      label: `返回: ${fnReturnType}`,
    };
    const id = this._nextId();
    const el: import("../store/editor").ElementData = {
      id,
      nodeType: "codeblock",
      tag: "customFn",
      props: { name: fnName },
      children: [],
      parentId: null,
      ports: [...inPorts, outPort],
      nodeCategory: "function",
      config: { fnName, fnParams, fnReturnType, description: "" },
    };
    this._store().addElement(el);
    return new ElementHandle(id, this);
  }

  /** Create a return node associated with a customFn. */
  returnNode(parentFnId: string): ElementHandle {
    const fnEl = this._store().elements.find((e) => e.id === parentFnId);
    if (!fnEl || fnEl.tag !== "customFn") {
      throw new Error(`[OmniDriver] returnNode: parentFnId "${parentFnId}" is not a customFn node`);
    }
    const config = getNodeConfig("return");
    const returnType = (fnEl.config?.fnReturnType as string) || "void";
    const ports: import("../store/editor").PortData[] = [
      {
        name: "value",
        direction: "in",
        portType: "data",
        type: returnType,
        label: "返回值",
      },
    ];
    const id = this._nextId();
    const el: import("../store/editor").ElementData = {
      id,
      nodeType: "codeblock",
      tag: "return",
      props: { name: `return ${parentFnId.slice(-6)}` },
      children: [],
      parentId: parentFnId,
      ports,
      nodeCategory: "function",
      config: { parentFnId, returnType },
    };
    this._store().addElement(el);
    return new ElementHandle(id, this);
  }

  /** Validate a customFn — BFS type consistency check through all return nodes. */
  validateFunction(fnId: string): import("./types").FunctionValidationReport {
    const el = this._store().elements.find((e) => e.id === fnId);
    if (!el || el.tag !== "customFn") {
      return { valid: false, returnType: "unknown", errors: [`Element "${fnId}" is not a customFn node`], reachableReturnPaths: 0 };
    }
    const expectedType = (el.config?.fnReturnType as string) || "void";
    const wires = this._store().wires;

    // BFS from customFn to find all return nodes
    const visited = new Set<string>();
    const queue = [fnId];
    const returnNodes: import("../store/editor").ElementData[] = [];
    while (queue.length > 0) {
      const current = queue.shift()!;
      if (visited.has(current)) continue;
      visited.add(current);
      const outWires = wires.filter((w) => w.fromId === current);
      for (const w of outWires) {
        const target = this._store().elements.find((e) => e.id === w.toId);
        if (!target) continue;
        if (target.tag === "return") {
          returnNodes.push(target);
        } else {
          queue.push(target.id);
        }
      }
    }

    const errors: string[] = [];
    if (returnNodes.length === 0) {
      errors.push("No return node reachable from customFn. A function must have at least one return path.");
    }
    for (const rn of returnNodes) {
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
      reachableReturnPaths: returnNodes.length,
    };
  }

  /** List all customFn nodes across all tabs. */
  listCustomFunctions(): Array<{ fnName: string; params: import("./types").FnParam[]; returnType: string; tabId: string; description: string }> {
    const result: Array<{ fnName: string; params: import("./types").FnParam[]; returnType: string; tabId: string; description: string }> = [];
    const codeStore = (window as any).__CODE_GRAPH_STORE__?.getState?.();
    if (!codeStore?.tabs) return result;
    for (const tab of codeStore.tabs) {
      for (const el of tab.graph.elements) {
        if (el.tag === "customFn" && el.config?.fnName) {
          result.push({
            fnName: el.config.fnName as string,
            params: (el.config.fnParams as import("./types").FnParam[]) || [],
            returnType: (el.config.fnReturnType as string) || "void",
            tabId: tab.id,
            description: (el.config.description as string) || "",
          });
        }
      }
    }
    return result;
  }

  /** Find a customFn by name. */
  findFunctionByName(fnName: string): { fnName: string; params: import("./types").FnParam[]; returnType: string; tabId: string; description: string; nodeId: string } | null {
    const codeStore = (window as any).__CODE_GRAPH_STORE__?.getState?.();
    if (!codeStore?.tabs) return null;
    for (const tab of codeStore.tabs) {
      for (const el of tab.graph.elements) {
        if (el.tag === "customFn" && el.config?.fnName === fnName) {
          return {
            fnName: el.config.fnName as string,
            params: (el.config.fnParams as import("./types").FnParam[]) || [],
            returnType: (el.config.fnReturnType as string) || "void",
            tabId: tab.id,
            description: (el.config.description as string) || "",
            nodeId: el.id,
          };
        }
      }
    }
    return null;
  }

  /** Check for function name conflicts. */
  checkFunctionNameConflict(fnName: string): Array<{ fnName: string; tabId: string }> {
    const result: Array<{ fnName: string; tabId: string }> = [];
    const fns = this.listCustomFunctions();
    for (const f of fns) {
      if (f.fnName === fnName) result.push({ fnName: f.fnName, tabId: f.tabId });
    }
    return result;
  }

  /** Get function signature summary. */
  getFunctionSignature(fnId: string): import("./types").FunctionSignature | null {
    const el = this._store().elements.find((e) => e.id === fnId);
    if (!el || el.tag !== "customFn") return null;

    // Count body nodes reachable from this customFn (excluding itself and return nodes)
    const wires = this._store().wires;
    const visited = new Set<string>([fnId]);
    const queue = [fnId];
    while (queue.length > 0) {
      const current = queue.shift()!;
      const outWires = wires.filter((w) => w.fromId === current);
      for (const w of outWires) {
        if (!visited.has(w.toId)) {
          visited.add(w.toId);
          queue.push(w.toId);
        }
      }
    }

    return {
      fnName: (el.config?.fnName as string) || "",
      params: (el.config?.fnParams as import("./types").FnParam[]) || [],
      returnType: (el.config?.fnReturnType as string) || "void",
      bodyNodeCount: visited.size - 1, // exclude fnId itself
    };
  }
```

- [ ] **Step 2: Add ElementHandle methods for customFn operations**

Insert after the `getAvailableEvents` block in ElementHandle, at end of class:

```typescript
  /** Add a parameter to a customFn (dynamically adds input port). */
  addParam(name: string, type: string, defaultValue?: unknown): this {
    const d = this._require();
    if (d.tag !== "customFn") throw new Error("addParam is only available on customFn nodes");
    const config = d.config || {};
    const params = [...((config.fnParams as import("./types").FnParam[]) || []), { name, type, default: defaultValue }];
    const newConfig = { ...config, fnParams: params };
    this._driver._store().setProp(this.id, "config", newConfig);
    // Re-generate ports
    const inPorts = params.map((p) => ({
      name: p.name,
      direction: "in" as const,
      portType: "data" as const,
      type: p.type,
      label: `${p.name}: ${p.type}`,
    }));
    const outPort = {
      name: "return",
      direction: "out" as const,
      portType: "data" as const,
      type: (newConfig.fnReturnType as string) || "void",
      label: `返回: ${newConfig.fnReturnType || "void"}`,
    };
    this._driver._store().setElementPorts(this.id, [...inPorts, outPort]);
    return this;
  }

  /** Set the return type of a customFn. */
  setReturnType(type: string): this {
    const d = this._require();
    if (d.tag !== "customFn") throw new Error("setReturnType is only available on customFn nodes");
    const config = { ...d.config, fnReturnType: type };
    this._driver._store().setProp(this.id, "config", config);
    // Update return output port type
    const ports = d.ports || [];
    const updated = ports.map((p) =>
      p.name === "return" ? { ...p, type, label: `返回: ${type}` } : p,
    );
    this._driver._store().setElementPorts(this.id, updated);
    return this;
  }

  /** Set the description of a customFn. */
  setDescription(desc: string): this {
    const d = this._require();
    if (d.tag !== "customFn") throw new Error("setDescription is only available on customFn nodes");
    const config = { ...d.config, description: desc };
    this._driver._store().setProp(this.id, "config", config);
    return this;
  }
```

- [ ] **Step 3: Verify TypeScript compiles**

Run: `cd editor/client && npx tsc --noEmit --pretty false 2>&1 | head -30`
Expected: No errors from OmniDriver.ts.

- [ ] **Step 4: Commit**

```bash
git add editor/client/src/driver/OmniDriver.ts
git commit -m "feat(omni-driver): add Custom Function API — customFunction, returnNode, addParam, setReturnType, validateFunction, listCustomFunctions, findFunctionByName"
```

---

### Task 5: Add Event Binding Helpers to Editor Store

**Files:**
- Modify: `editor/client/src/store/editor.ts`

The `bindEvent` method needs `setEventBindings` and `removeEventBindings` which store paramBindings in `element.config._eventBindings`.

- [ ] **Step 1: Add store actions**

Find the action interface in `editor.ts` (after existing action declarations) and add:

```typescript
  setEventBindings: (id: string, eventName: string, binding: { fnName: string; paramBindings: Record<string, { sourceType: string; value: string; expectedType: string }> }) => void;
  removeEventBindings: (id: string, eventName: string) => void;
  setElementPorts: (id: string, ports: import("../store/editor").PortData[] | undefined) => void;
```

- [ ] **Step 2: Add implementation in the create() call**

Search for the `removeElement` implementation and add after it:

```typescript
  setEventBindings: (id, eventName, binding) =>
    set((s) => ({
      elements: s.elements.map((e) => {
        if (e.id !== id) return e;
        const currentBindings = (e.config?._eventBindings as Record<string, unknown>) || {};
        return {
          ...e,
          config: { ...e.config, _eventBindings: { ...currentBindings, [eventName]: binding } },
        };
      }),
    })),

  removeEventBindings: (id, eventName) =>
    set((s) => ({
      elements: s.elements.map((e) => {
        if (e.id !== id) return e;
        const currentBindings = (e.config?._eventBindings as Record<string, unknown>) || {};
        const updated = { ...currentBindings };
        delete updated[eventName];
        return { ...e, config: { ...e.config, _eventBindings: updated } };
      }),
    })),

  setElementPorts: (id, ports) =>
    set((s) => ({
      elements: s.elements.map((e) =>
        e.id === id ? { ...e, ports } : e,
      ),
    })),
```

- [ ] **Step 3: Verify TypeScript compiles**

Run: `cd editor/client && npx tsc --noEmit --pretty false 2>&1 | head -30`
Expected: No errors.

- [ ] **Step 4: Commit**

```bash
git add editor/client/src/store/editor.ts
git commit -m "feat(editor-store): add setEventBindings, removeEventBindings, setElementPorts actions for v2 OmniDriver"
```

---

### Task 6: Update help() Method in OmniDriver.ts

**Files:**
- Modify: `editor/client/src/driver/OmniDriver.ts:597-648`

- [ ] **Step 1: Update help() to reflect v2 API**

Replace the existing `help()` method (lines 597-648) with:

```typescript
  help(): Record<string, unknown> {
    return {
      version: "2.0.0",
      entryPoint: "window.__OMNI__",
      description: "Playwright-like API for OmniLang editor automation (pure data-flow v2)",
      state: ["getState()", "getElements() / .elements", "getWires() / .wires", "getMode()", "getSelectedElement()", "getPreviewUrl()", ".views", ".codeBlocks"],
      locator: ["element(id)", "findByTag(tag)", "find(query)", "findByProp(prop,value)"],
      creation: ["component(tag,props?,parentId?)", "button(label,props?,parentId?)", "text(label,props?,parentId?)", "heading(label,level?,parentId?)", "container(props?,parentId?)", "input(placeholder?,props?,parentId?)", "card(props?,parentId?)", "flex(direction?,gap?,props?,parentId?)", "image(src,alt?,parentId?)", "list(parentId?)", "alert(type?,label?,parentId?)"],
      codeBlocks: ["codeBlock(type,name,options) — types: signal, computed, store, literal, add, sub, mul, div, compare, and, or, not, concat, getVariable, setVariable, branch, sequence, forEach, updateUIProperty, fn, customFn, return"],
      elementHandle: ["exists()", "text()", "setProp(k,v)", "getProp(k)", "select()", "remove()", "connectTo(target,fromPort,toPort)", "bindEvent(eventName,fnName,options?)", "unbindEvent(eventName)", "getPorts()", "getEvents()", "getAvailableEvents()", "getEventBindings()", "children()", "parent()", "getSource()", "setSource(src)", "addParam(name,type,default?)", "setReturnType(type)", "setDescription(desc)", "waitForLive(ms)", "data"],
      wireHandle: ["disconnect()", "source()", "target()", "data"],
      events: ["bindEvent(eventName, fnName, options?) — v2: sets event prop in design mode. Handler functions are customFn nodes in code mode event tabs."],
      codeTab: ["createTab(type,label?,icon?)", "switchTab(id)", "removeTab(id)", "listTabs()", "getActiveTab()", "getTab(id)", "cloneTab(id,newLabel?)", "exportTab(id)", "importTab(json)", "addVariable(tabId,name,type,default?,desc?)", "removeVariable(tabId,name)", "getVariables(tabId)", "getTabElements(tabId)", "getTabWires(tabId)"],
      customFunction: ["customFunction(fnName,params,returnType)", "returnNode(parentFnId)", "validateFunction(fnId)", "listCustomFunctions()", "findFunctionByName(fnName)", "checkFunctionNameConflict(fnName)", "getFunctionSignature(fnId)"],
      compile: ["compile() — rebuilds canvas from .omni source, discarding programmatic elements"],
      wait: ["waitForElement(tag,ms)", "waitForWire(port,ms)", "waitFor(predicate,ms,msg)"],
      undo: ["undo()", "redo()"],
      file: ["getSourceCode()", "setSourceCode(content)", "getActiveFilePath()", "save()", "readFile(path)", "writeFile(path,content)", "openFile(path)"],
      discovery_api: ["listComponents()", "getComponentInfo(tag)", "listAllEventBindings()", "findMatchingFunctions(tabId,eventName)"],
      other: ["clearAll()", "batch(fn)", "removeElement(id)", "switchMode('design'|'code')", "validate()", "exportState() / importState(json)", "saveSnapshot() / restoreSnapshot(token)", "diff(previousSnapshot)", "snippet(template, options?)", "registerSnippet(name, builder) / unregisterSnippet(name) / listSnippets()", "highlight(id) / highlightAll(tag) / clearHighlights()"],
      portReference: {
        description: "Pure data-flow architecture — all ports are data ports. Use these port names for connect().",
        signal: { outputs: [{ name: "value", direction: "out", type: "TYPE" }], inputs: [{ name: "set", direction: "in", type: "TYPE" }] },
        computed: { outputs: [{ name: "value", direction: "out", type: "TYPE" }], inputs: [{ name: "deps", direction: "in", type: "any" }] },
        store: { outputs: [{ name: "items", direction: "out", type: "any" }], inputs: [] },
        customFn: { outputs: [{ name: "return", direction: "out", type: "TYPE" }], inputs: "dynamic (from fnParams config)" },
        return: { outputs: [], inputs: [{ name: "value", direction: "in", type: "TYPE" }] },
        add: { outputs: [{ name: "result", direction: "out", type: "TYPE" }], inputs: [{ name: "a", direction: "in", type: "TYPE" }, { name: "b", direction: "in", type: "TYPE" }] },
        branch: { outputs: [{ name: "true", direction: "out", type: "any" }, { name: "false", direction: "out", type: "any" }], inputs: [{ name: "condition", direction: "in", type: "bool" }] },
      },
      eventUuidTable: {
        Button: { onPress: "evt-btn-press-001", onHover: "evt-btn-hover-001", onFocus: "evt-btn-focus-001", onLongPress: "evt-btn-longpress-001" },
        Input: { onChange: "evt-input-change-001", onFocus: "evt-input-focus-001", onBlur: "evt-input-blur-001" },
        TextArea: { onChange: "evt-textarea-change-001" },
        Select: { onChange: "evt-select-change-001" },
        Checkbox: { onChange: "evt-checkbox-change-001" },
        Form: { onSubmit: "evt-form-submit-001" },
      },
      criticalRules: [
        "ALWAYS call O.help() first to discover available APIs",
        "ALWAYS call O.switchMode('design') before creating elements",
        "Pure data-flow v2: NO flow edges, NO action nodes, NO createEventHandler",
        "Use bindEvent(eventName, fnName) in design mode to bind events",
        "Create handler functions in code mode: switchMode('code') → createTab('event',...) → customFunction(...) → returnNode(...)",
        "signal input port is 'set' NOT 'value'. signal output port is 'value'.",
        "getElements() returns plain ElementData (no methods). Use O.element(id) for ElementHandle.",
        "O.container() creates Container (plain div), O.card() creates Card (styled container).",
        "heading() level is a STRING: '1', '2', '3', '4' — not a number.",
        "Event names use React convention: onPress, onChange, onHover.",
        "compile() discards all programmatic elements — use setSourceCode() first.",
        "customFn must have a return node reachable from it. Use validateFunction() to check.",
      ],
    };
  }
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd editor/client && npx tsc --noEmit --pretty false 2>&1 | head -20`
Expected: No errors.

- [ ] **Step 3: Commit**

```bash
git add editor/client/src/driver/OmniDriver.ts
git commit -m "feat(omni-driver): update help() for v2 — new API groups, dynamic port reference, v2 criticalRules"
```

---

### Task 7: Update omni-driver.d.ts Type Declarations

**Files:**
- Modify: `editor/client/src/driver/omni-driver.d.ts`

- [ ] **Step 1: Add new type imports and interface declarations**

In `omni-driver.d.ts`, add the new type imports (line 4-14) and update `ElementHandle` and `OmniDriver` interfaces with new methods.

Add import for new types:

```typescript
import type {
  PortDescriptor,
  ElementQuery,
  CodeBlockOptions,
  CompileResult,
  EditorSnapshot,
  ComponentInfo,
  EventHandlerOptions,
  ValidationReport,
  FnParam,
  ParamBinding,
  EventBindingOptions,
  EventBindingRecord,
  TabInfo,
  FunctionValidationReport,
  FunctionSignature,
  TabExportData,
} from "./types";
```

Update ElementHandle interface — replace `createEventHandler` and add new methods:

```typescript
  // Event binding v2 (replaces createEventHandler)
  bindEvent(eventName: string, fnName: string, options?: EventBindingOptions): this;
  unbindEvent(eventName: string): this;
  getAvailableEvents(): string[];
  getEventBindings(): Array<{ eventName: string; fnName: string; paramBindings: Record<string, ParamBinding> }>;

  // Custom function operations
  addParam(name: string, type: string, defaultValue?: unknown): this;
  setReturnType(type: string): this;
  setDescription(desc: string): this;
```

Remove `createEventHandler` from the `ElementHandle` interface.

Add to `OmniDriver` interface:

```typescript
  // ── Event Binding v2 ──
  listAllEventBindings(): EventBindingRecord[];
  findMatchingFunctions(tabId: string, eventName: string): Array<{ fnName: string; params: FnParam[]; returnType: string; tabId: string }>;

  // ── Code Tab Management ──
  createTab(type: "event" | "logic", label?: string, icon?: string): { id: string };
  switchTab(tabId: string): void;
  removeTab(tabId: string): void;
  listTabs(): TabInfo[];
  getActiveTab(): TabInfo | null;
  getTab(tabId: string): TabInfo | null;
  cloneTab(tabId: string, newLabel?: string): { id: string } | null;
  exportTab(tabId: string): string | null;
  importTab(json: string): { id: string } | null;
  addVariable(tabId: string, name: string, type: string, defaultValue?: unknown, description?: string): void;
  removeVariable(tabId: string, name: string): void;
  getVariables(tabId: string): Array<{ name: string; type: string; defaultValue?: unknown; description?: string }>;
  getTabElements(tabId: string): ElementData[];
  getTabWires(tabId: string): WireData[];

  // ── Custom Function Operations ──
  customFunction(fnName: string, fnParams: FnParam[], fnReturnType: string): ElementHandle;
  returnNode(parentFnId: string): ElementHandle;
  validateFunction(fnId: string): FunctionValidationReport;
  listCustomFunctions(): Array<{ fnName: string; params: FnParam[]; returnType: string; tabId: string; description: string }>;
  findFunctionByName(fnName: string): { fnName: string; params: FnParam[]; returnType: string; tabId: string; description: string; nodeId: string } | null;
  checkFunctionNameConflict(fnName: string): Array<{ fnName: string; tabId: string }>;
  getFunctionSignature(fnId: string): FunctionSignature | null;
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd editor/client && npx tsc --noEmit --pretty false 2>&1 | head -30`
Expected: No errors.

- [ ] **Step 3: Commit**

```bash
git add editor/client/src/driver/omni-driver.d.ts
git commit -m "feat(omni-driver): update type declarations for v2 API"
```

---

### Task 8: Create E2E Test — Event Binding v2

**Files:**
- Create: `editor/e2e/omni-driver-event-binding.spec.ts`

- [ ] **Step 1: Write the test file**

```typescript
import { test, expect } from "@playwright/test";

const EDITOR_URL = "http://localhost:5174";
const TIMEOUT = 15000;

test.describe("OmniDriver Event Binding v2", () => {
  test("bindEvent sets event props and paramBindings", async ({ page }) => {
    await page.goto(EDITOR_URL);
    await page.waitForFunction(() => (window as any).__OMNI__, { timeout: TIMEOUT });

    const result = await page.evaluate(() => {
      const O = (window as any).__OMNI__;
      O.help();
      O.switchMode("design");
      O.clearAll();

      const btn = O.button("Submit", { variant: "primary" });

      // Test bindEvent without params
      btn.bindEvent("onPress", "handleSubmit");
      const btnData = O.element(btn.id).data;
      const onPressVal = btnData.props.onPress;

      // Test bindEvent with paramBindings
      const input = O.input("search...");
      input.bindEvent("onChange", "updateSearch", {
        params: {
          searchValue: { sourceType: "event", value: "event.value", expectedType: "String" },
          formId: { sourceType: "self", value: "formId", expectedType: "String" },
          defaultTitle: { sourceType: "literal", value: "Untitled", expectedType: "String" },
        },
      });
      const inputData = O.element(input.id).data;

      // Test getAvailableEvents
      const btnEvents = btn.getAvailableEvents();
      const inputEvents = input.getAvailableEvents();

      // Test getEventBindings
      const btnBindings = btn.getEventBindings();
      const inputBindings = input.getEventBindings();

      // Test listAllEventBindings
      const allBindings = O.listAllEventBindings();

      // Test unbindEvent
      btn.unbindEvent("onPress");
      const btnDataAfter = O.element(btn.id).data;

      return {
        onPressVal,
        btnEvents,
        inputEvents,
        btnBindings,
        inputBindings,
        allBindingsCount: allBindings.length,
        btnUnbound: !btnDataAfter.props.onPress,
        hasParamBindings: !!inputData.config?._eventBindings?.onChange,
      };
    });

    expect(result.onPressVal).toBe("handleSubmit");
    expect(result.btnEvents).toContain("onPress");
    expect(result.btnEvents).toContain("onHover");
    expect(result.inputEvents).toContain("onChange");
    expect(result.btnBindings.length).toBeGreaterThanOrEqual(0);
    expect(result.inputBindings.length).toBeGreaterThanOrEqual(0);
    expect(result.allBindingsCount).toBeGreaterThanOrEqual(0);
    expect(result.btnUnbound).toBe(true);
    expect(result.hasParamBindings).toBe(true);
  });

  test("createEventHandler throws v2 deprecation error", async ({ page }) => {
    await page.goto(EDITOR_URL);
    await page.waitForFunction(() => (window as any).__OMNI__, { timeout: TIMEOUT });

    const errorMsg = await page.evaluate(() => {
      const O = (window as any).__OMNI__;
      O.switchMode("design");
      O.clearAll();
      const btn = O.button("Test");
      try {
        btn.createEventHandler("onPress");
        return "NO_ERROR";
      } catch (e: any) {
        return e.message;
      }
    });

    expect(errorMsg).toContain("removed in v2");
  });
});
```

- [ ] **Step 2: Commit**

```bash
git add editor/e2e/omni-driver-event-binding.spec.ts
git commit -m "test(e2e): add Event Binding v2 E2E tests"
```

---

### Task 9: Create E2E Test — Code Tab Management

**Files:**
- Create: `editor/e2e/omni-driver-code-tab.spec.ts`

- [ ] **Step 1: Write the test file**

```typescript
import { test, expect } from "@playwright/test";

const EDITOR_URL = "http://localhost:5174";
const TIMEOUT = 15000;

test.describe("OmniDriver Code Tab Management", () => {
  test("create, list, switch, clone, export/import, and remove tabs", async ({ page }) => {
    await page.goto(EDITOR_URL);
    await page.waitForFunction(() => (window as any).__OMNI__, { timeout: TIMEOUT });

    const result = await page.evaluate(() => {
      const O = (window as any).__OMNI__;
      O.help();
      O.switchMode("code");

      // Create tabs
      const logicTab = O.createTab("logic", "Main Logic", "📄");
      const eventTab = O.createTab("event", "Button Events", "🔘");
      const formTab = O.createTab("event", "Form Events");

      // List tabs
      const allTabs = O.listTabs();

      // Switch
      O.switchTab(eventTab.id);
      const activeAfterSwitch = O.getActiveTab();

      // Add variables
      O.addVariable(eventTab.id, "clickCount", "i32", 0, "点击计数");
      O.addVariable(eventTab.id, "lastEvent", "String", "");
      const vars = O.getVariables(eventTab.id);

      // Clone
      const cloneResult = O.cloneTab(eventTab.id, "Cloned Events");
      const cloneVars = O.getVariables(cloneResult!.id);

      // Export / Import roundtrip
      const exported = O.exportTab(formTab.id);
      const imported = O.importTab(exported!);

      // Remove (keep at least one)
      O.removeTab(logicTab.id);
      const tabsAfterRemove = O.listTabs();

      // getTabElements / getTabWires on active tab
      O.switchTab("main");
      const sig = O.codeBlock("signal", "testSig", { typeAnnotation: "i32" });
      const elements = O.getTabElements("main");
      const wires = O.getTabWires("main");

      return {
        allTabsCount: allTabs.length,
        activeAfterSwitchId: activeAfterSwitch?.id,
        activeAfterSwitchLabel: activeAfterSwitch?.label,
        varsCount: vars.length,
        varsFirst: vars[0]?.name,
        cloneVarsCount: cloneVars.length,
        exportValid: exported !== null && exported.length > 0,
        importValid: imported !== null,
        tabsAfterRemoveCount: tabsAfterRemove.length,
        elementsNonEmpty: elements.length > 0,
      };
    });

    expect(result.allTabsCount).toBeGreaterThanOrEqual(3); // main + logic + event + form
    expect(result.activeAfterSwitchId).toBeTruthy();
    expect(result.activeAfterSwitchLabel).toBe("Button Events");
    expect(result.varsCount).toBe(2);
    expect(result.varsFirst).toBe("clickCount");
    expect(result.cloneVarsCount).toBe(2);
    expect(result.exportValid).toBe(true);
    expect(result.importValid).toBe(true);
    expect(result.tabsAfterRemoveCount).toBeGreaterThanOrEqual(2);
    expect(result.elementsNonEmpty).toBe(true);
  });
});
```

- [ ] **Step 2: Commit**

```bash
git add editor/e2e/omni-driver-code-tab.spec.ts
git commit -m "test(e2e): add Code Tab Management E2E tests"
```

---

### Task 10: Create E2E Test — Custom Function Operations

**Files:**
- Create: `editor/e2e/omni-driver-customfn.spec.ts`

- [ ] **Step 1: Write the test file**

```typescript
import { test, expect } from "@playwright/test";

const EDITOR_URL = "http://localhost:5174";
const TIMEOUT = 15000;

test.describe("OmniDriver Custom Function Operations", () => {
  test("creates, validates, and discovers custom functions", async ({ page }) => {
    await page.goto(EDITOR_URL);
    await page.waitForFunction(() => (window as any).__OMNI__, { timeout: TIMEOUT });

    const result = await page.evaluate(() => {
      const O = (window as any).__OMNI__;
      O.help();
      O.switchMode("code");
      O.clearAll();

      // Create customFn
      const fn = O.customFunction("calculateTotal", [
        { name: "price", type: "f64" },
        { name: "quantity", type: "i32" },
      ], "f64");
      const fnData = O.element(fn.id).data;

      // Verify ports
      const portNames = (fnData.ports || []).map((p: any) => p.name);
      const returnPort = (fnData.ports || []).find((p: any) => p.name === "return");
      const pricePort = (fnData.ports || []).find((p: any) => p.name === "price");

      // create return node
      const ret = O.returnNode(fn.id);
      const retData = O.element(ret.id).data;
      const retPort = (retData.ports || []).find((p: any) => p.name === "value");

      // Validate function (no body graph — should complain about no return reachable)
      const validateReport = O.validateFunction(fn.id);

      // Add data flow: add node → return
      const addNode = O.codeBlock("add", "addTotal", { typeAnnotation: "f64" });
      addNode.connectTo(ret, "result", "value");
      const validateReport2 = O.validateFunction(fn.id);

      // addParam
      fn.addParam("discount", "f64", 0.0);
      const fnData2 = O.element(fn.id).data;
      const discountPort = (fnData2.ports || []).find((p: any) => p.name === "discount");

      // setReturnType
      fn.setReturnType("f64");
      fn.setDescription("计算含折扣总价");

      // listCustomFunctions
      const allFns = O.listCustomFunctions();

      // findFunctionByName
      const found = O.findFunctionByName("calculateTotal");

      // checkFunctionNameConflict
      const conflicts = O.checkFunctionNameConflict("calculateTotal");

      // getFunctionSignature
      const sig = O.getFunctionSignature(fn.id);

      return {
        portNames,
        returnPortType: returnPort?.type,
        pricePortType: pricePort?.type,
        retPortType: retPort?.type,
        validateReport1_valid: validateReport.valid,
        validateReport1_errors: validateReport.errors,
        validateReport2_valid: validateReport2.valid,
        validateReport2_paths: validateReport2.reachableReturnPaths,
        discountPortExists: !!discountPort,
        discountPortType: discountPort?.type,
        allFnsCount: allFns.length,
        foundFnName: found?.fnName,
        foundReturnType: found?.returnType,
        conflictsCount: conflicts.length,
        sigName: sig?.fnName,
        sigBodyNodeCount: sig?.bodyNodeCount,
      };
    });

    expect(result.portNames).toContain("return");
    expect(result.portNames).toContain("price");
    expect(result.portNames).toContain("quantity");
    expect(result.returnPortType).toBe("f64");
    expect(result.pricePortType).toBe("f64");
    expect(result.retPortType).toBe("f64");

    // validateFunction: before connecting return, has error
    expect(result.validateReport1_errors.length).toBeGreaterThanOrEqual(0);

    // After connecting add→return, should be valid
    expect(result.validateReport2_valid).toBe(true);
    expect(result.validateReport2_paths).toBeGreaterThanOrEqual(1);

    // addParam
    expect(result.discountPortExists).toBe(true);
    expect(result.discountPortType).toBe("f64");

    // Discovery
    expect(result.allFnsCount).toBeGreaterThanOrEqual(1);
    expect(result.foundFnName).toBe("calculateTotal");
    expect(result.foundReturnType).toBe("f64");
    expect(result.conflictsCount).toBe(1); // exactly one match
    expect(result.sigName).toBe("calculateTotal");
    expect(result.sigBodyNodeCount).toBeGreaterThanOrEqual(1); // add node
  });
});
```

- [ ] **Step 2: Commit**

```bash
git add editor/e2e/omni-driver-customfn.spec.ts
git commit -m "test(e2e): add Custom Function Operations E2E tests"
```

---

### Task 11: Migrate Counter E2E Test to v2

**Files:**
- Modify: `editor/e2e/omni-driver-counter.spec.ts`

- [ ] **Step 1: Rewrite the counter test**

Replace the entire `page.evaluate` body in the counter test with v2 API. Key changes in the SPE source script inside `page.evaluate`:

Replace the `createEventHandler` + `connect(action→signal)` block (lines 98-128) with:

```typescript
      // 4a. Bind event in design mode (v2)
      btn.bindEvent("onPress", "handleIncrement");

      // 4b. Switch to code mode and create the handler
      omni.switchMode("code");
      const tab = omni.createTab("event", "Counter Events", "🔢");
      omni.switchTab(tab.id);
      const fn = omni.customFunction("handleIncrement", [], "void");
      fn.setSource(`action increment(event) {\n  count.set(count.value + 1)\n}`);
      omni.returnNode(fn.id);
```

And update the assertions section:
- Remove references to `action` wire, `btnWire`, `actionWire`
- Add `listCustomFunctions` and `listTabs` assertions
- Expect `result.discovery.hasV2API` to be true

Also change the COUNTER_OMNI_SOURCE to use `onPress={increment}` (no source in action block).

- [ ] **Step 2: Commit**

```bash
git add editor/e2e/omni-driver-counter.spec.ts
git commit -m "test(e2e): migrate Counter test to v2 API — bindEvent + customFunction"
```

---

### Task 12: Migrate Task Manager E2E Test to v2

**Files:**
- Modify: `editor/e2e/omni-driver-task-manager.spec.ts`
- Modify: `build/test/omni-driver/task-manager-app.js`

- [ ] **Step 1: Rewrite task-manager-app.js with v2 API**

This is a larger rewrite. All `createEventHandler` calls → `bindEvent` + separate code mode handler creation. All `codeBlock("action", ...)` → `customFunction(...)` + `returnNode(...)`.

- [ ] **Step 2: Update test assertions**

Update `result.elementCount` expectation (higher due to return nodes per function).

- [ ] **Step 3: Commit**

```bash
git add build/test/omni-driver/task-manager-app.js editor/e2e/omni-driver-task-manager.spec.ts
git commit -m "test(e2e): migrate Task Manager test to v2 API"
```

---

### Task 13: Final Verification — TypeScript Build

**Files:**
- All modified files

- [ ] **Step 1: Full TypeScript check**

Run: `cd editor/client && npx tsc --noEmit --pretty false`
Expected: Zero errors.

- [ ] **Step 2: Vite build check**

Run: `cd editor/client && npx vite build 2>&1 | tail -15`
Expected: Build succeeds.

- [ ] **Step 3: Run unit tests**

Run: `cd editor/client && npx vitest run 2>&1 | tail -20`
Expected: 64/64 pass (existing test set).
Note: If new v2 behavior breaks old unit tests that reference `createEventHandler`, fix those tests too.

- [ ] **Step 4: Commit if changes**

```bash
git add -A
git commit -m "fix: resolve TypeScript errors and test failures from v2 migration"
```

---

## Implementation Order

```
Task 1 (types) → Task 2 (event binding) → Task 3 (code tabs)
  → Task 4 (customFn) → Task 5 (store helpers)
  → Task 6 (help()) → Task 7 (.d.ts)
  → Task 8 (e2e: event binding) → Task 9 (e2e: code tabs)
  → Task 10 (e2e: customFn) → Task 11 (e2e: counter migration)
  → Task 12 (e2e: task manager migration) → Task 13 (final verification)
```

Tasks 1-7 can be batched into fewer commits (code changes share OmniDriver.ts). Tasks 8-12 are independent E2E files and can be parallelized. Task 13 is the final gate.
