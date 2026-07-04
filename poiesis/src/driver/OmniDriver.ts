/** OmniDriver — Thin facade over 9 focused sub-modules.
 *
 *  The `window.__OMNI__` API surface is IDENTICAL to the original.
 *  All 80+ public methods delegate to specialized drivers (CanvasDriver,
 *  ConnectionDriver, CodeDriver, StateDriver, FileDriver, VisualService,
 *  SnippetService).
 *
 *  Usage from AI / test scripts:
 *  ```js
 *  const omni = window.__OMNI__;
 *  const btn = omni.component("Button", { label: "Submit" });
 *  btn.bindEvent("onPress", "handleSubmit");
 *  const result = await omni.compile();
 *  ```
 */

import { getDesignStore, getActiveStore } from "./store";
import { useEditorStore } from "../store/editor";
import { api } from "../commands";
import { COMPONENT_PROTOCOL } from "../components/panels/ComponentPalette";
import { getNodeConfig } from "../components/canvas/shared/nodeRegistry";
import { ElementHandle } from "./ElementHandle";
import { WireHandle } from "./WireHandle";
import { ConnectionDriver } from "./ConnectionDriver";
import { CanvasDriver } from "./CanvasDriver";
import { parseViewTree, parseCodeBlocks } from "../services/parser/omni-regex-parser";
import { StateDriver } from "./StateDriver";
import { CodeDriver } from "./CodeDriver";
import { FileDriver } from "./FileDriver";
import { VisualService } from "./VisualService";
import { SnippetService } from "./SnippetService";
import type {
  ElementData,
  WireData,
  PortDescriptor,
  ElementQuery,
  CodeBlockOptions,
  CompileResult,
  EditorSnapshot,
  ComponentInfo,
  ValidationReport,
  StateDiff,
  EventBindingRecord,
  FnParam,
  TabInfo,
  FunctionValidationReport,
  FunctionSignature,
} from "./types";

// ═══════════════════════════════════════════════════════
// OmniDriver facade
// ═══════════════════════════════════════════════════════

class OmniDriver {
  // ── Sub-modules ───────────────────────────────────
  private _canvas: CanvasDriver;
  private _connection: ConnectionDriver;
  private _code: CodeDriver;
  private _state: StateDriver;
  private _file: FileDriver;
  private _visual: VisualService;
  private _snippets: SnippetService;

  constructor() {
    this._canvas = new CanvasDriver();
    this._connection = new ConnectionDriver();
    this._code = new CodeDriver();
    this._state = new StateDriver();
    this._file = new FileDriver();
    this._visual = new VisualService();
    this._snippets = new SnippetService(this);
  }

  // ═══════════════════════════════════════════════════
  // Accessor Properties
  // ═══════════════════════════════════════════════════

  /** Alias for getElements() — all elements on the canvas. */
  get elements(): ElementData[] {
    return getActiveStore().elements;
  }

  /** Alias for getWires() — all wires on the canvas. */
  get wires(): WireData[] {
    return getActiveStore().wires;
  }

  /** Get all view elements. */
  get views(): ElementHandle[] {
    return this._canvas.views;
  }

  /** Get all code block elements. */
  get codeBlocks(): ElementHandle[] {
    return this._canvas.codeBlocks;
  }

  // ═══════════════════════════════════════════════════
  // State Queries (StateDriver)
  // ═══════════════════════════════════════════════════

  getState(): EditorSnapshot { return this._state.getState(); }
  getElements(): ElementData[] { return this._state.getElements(); }
  getWires(): WireData[] { return this._state.getWires(); }
  getSelectedElement(): ElementHandle | null { return this._state.getSelectedElement(); }
  getMode(): "design" | "code" { return this._state.getMode(); }
  getPreviewUrl(): string | null { return this._state.getPreviewUrl(); }
  exportState(): string { return this._state.exportState(); }
  importState(json: string): void { return this._state.importState(json); }
  diff(previous: EditorSnapshot): StateDiff { return this._state.diff(previous); }
  undo(): void { return this._state.undo(); }
  redo(): void { return this._state.redo(); }
  saveSnapshot(): string { return this._state.saveSnapshot(); }
  restoreSnapshot(snapshotToken: string): boolean { return this._state.restoreSnapshot(snapshotToken); }

  // ═══════════════════════════════════════════════════
  // Element Handles (CanvasDriver)
  // ═══════════════════════════════════════════════════

  element(id: string): ElementHandle { return this._canvas.element(id); }
  findByTag(tag: string): ElementHandle[] { return this._canvas.findByTag(tag); }
  find(query: ElementQuery): ElementHandle[] { return this._canvas.find(query); }
  findByProp(prop: string, value: unknown): ElementHandle[] { return this._canvas.findByProp(prop, value); }

  // ── Element Creation ─────────────────────────────

  component(tag: string, props?: Record<string, unknown>, parentId?: string | null): ElementHandle {
    return this._canvas.component(tag, props, parentId);
  }
  codeBlock(blockType: string, name: string, options?: CodeBlockOptions): ElementHandle {
    return this._canvas.codeBlock(blockType, name, options);
  }
  button(label: string, props?: Record<string, unknown>, parentId?: string | null): ElementHandle {
    return this._canvas.button(label, props, parentId);
  }
  text(label: string, props?: Record<string, unknown>, parentId?: string | null): ElementHandle {
    return this._canvas.text(label, props, parentId);
  }
  heading(label: string, level?: string | number, parentId?: string | null): ElementHandle {
    return this._canvas.heading(label, level, parentId);
  }
  container(props?: Record<string, unknown>, parentId?: string | null): ElementHandle {
    return this._canvas.container(props, parentId);
  }
  input(placeholder?: string, props?: Record<string, unknown>, parentId?: string | null): ElementHandle {
    return this._canvas.input(placeholder, props, parentId);
  }
  card(props?: Record<string, unknown>, parentId?: string | null): ElementHandle {
    return this._canvas.card(props, parentId);
  }
  flex(direction?: string, gap?: string, props?: Record<string, unknown>, parentId?: string | null): ElementHandle {
    return this._canvas.flex(direction, gap, props, parentId);
  }
  image(src: string, alt?: string, parentId?: string | null): ElementHandle {
    return this._canvas.image(src, alt, parentId);
  }
  list(parentId?: string | null): ElementHandle {
    return this._canvas.list(parentId);
  }
  alert(type?: string, label?: string, parentId?: string | null): ElementHandle {
    return this._canvas.alert(type, label, parentId);
  }

  // ── Canvas Operations ────────────────────────────

  removeElement(id: string): void { return this._canvas.removeElement(id); }
  clearAll(): void { return this._canvas.clearAll(); }

  // ── Property Operations ──────────────────────────

  setProp(id: string, prop: string, value: unknown): void { return this._canvas.setProp(id, prop, value); }
  getProp(id: string, prop: string): unknown { return this._canvas.getProp(id, prop); }
  setSource(id: string, source: string): void { return this._canvas.setSource(id, source); }
  selectElement(id: string | null): void { return this._canvas.selectElement(id); }

  // ── Mode Switching ───────────────────────────────

  switchMode(mode: "design" | "code"): void { return this._canvas.switchMode(mode); }

  // ═══════════════════════════════════════════════════
  // Wire Operations (ConnectionDriver)
  // ═══════════════════════════════════════════════════

  connect(fromId: string, fromPort: string, toId: string, toPort: string, wireId?: string): WireHandle {
    return this._connection.connect(fromId, fromPort, toId, toPort, wireId);
  }
  disconnect(wireId: string): void { return this._connection.disconnect(wireId); }
  validate(): ValidationReport { return this._connection.validate(); }

  // ═══════════════════════════════════════════════════
  // Code & Event Operations (CodeDriver)
  // ═══════════════════════════════════════════════════

  listAllEventBindings(): EventBindingRecord[] { return this._code.listAllEventBindings(); }
  findMatchingFunctions(tabId: string, eventName: string): Array<{ fnName: string; params: FnParam[]; returnType: string; tabId: string }> {
    return this._code.findMatchingFunctions(tabId, eventName);
  }

  // ── Code Tab Management ──────────────────────────

  createTab(type: "event" | "logic", label?: string, icon?: string): { id: string } {
    return this._code.createTab(type, label, icon);
  }
  switchTab(tabId: string): void { return this._code.switchTab(tabId); }
  removeTab(tabId: string): void { return this._code.removeTab(tabId); }
  listTabs(): TabInfo[] { return this._code.listTabs(); }
  getActiveTab(): TabInfo | null { return this._code.getActiveTab(); }
  getTab(tabId: string): TabInfo | null { return this._code.getTab(tabId); }
  cloneTab(tabId: string, newLabel?: string): { id: string } | null { return this._code.cloneTab(tabId, newLabel); }
  exportTab(tabId: string): string | null { return this._code.exportTab(tabId); }
  importTab(json: string): { id: string } | null { return this._code.importTab(json); }
  addVariable(tabId: string, name: string, type: string, defaultValue?: unknown, description?: string): void {
    return this._code.addVariable(tabId, name, type, defaultValue, description);
  }
  removeVariable(tabId: string, name: string): void { return this._code.removeVariable(tabId, name); }
  getVariables(tabId: string): Array<{ name: string; type: string; defaultValue?: unknown; description?: string }> {
    return this._code.getVariables(tabId);
  }
  getTabElements(tabId: string): ElementData[] { return this._code.getTabElements(tabId); }
  getTabWires(tabId: string): WireData[] { return this._code.getTabWires(tabId); }

  // ── Custom Function Operations ───────────────────

  customFunction(fnName: string, fnParams: FnParam[], fnReturnType: string): ElementHandle {
    return this._code.customFunction(fnName, fnParams, fnReturnType);
  }
  returnNode(parentFnId: string): ElementHandle { return this._code.returnNode(parentFnId); }
  validateFunction(fnId: string): FunctionValidationReport { return this._code.validateFunction(fnId); }
  listCustomFunctions(): Array<{ fnName: string; params: FnParam[]; returnType: string; tabId: string; description: string }> {
    return this._code.listCustomFunctions();
  }
  findFunctionByName(fnName: string): { fnName: string; params: FnParam[]; returnType: string; tabId: string; description: string; nodeId: string } | null {
    return this._code.findFunctionByName(fnName);
  }
  checkFunctionNameConflict(fnName: string): Array<{ fnName: string; tabId: string }> {
    return this._code.checkFunctionNameConflict(fnName);
  }
  getFunctionSignature(fnId: string): FunctionSignature | null { return this._code.getFunctionSignature(fnId); }

  // ═══════════════════════════════════════════════════
  // File Operations (FileDriver)
  // ═══════════════════════════════════════════════════

  getSourceCode(): string | null { return this._file.getSourceCode(); }
  setSourceCode(content: string): void { return this._file.setSourceCode(content); }
  getActiveFilePath(): string | null { return this._file.getActiveFilePath(); }
  async save(): Promise<boolean> { return this._file.save(); }
  async readFile(filePath: string): Promise<string | null> { return this._file.readFile(filePath); }
  async writeFile(filePath: string, content: string): Promise<boolean> { return this._file.writeFile(filePath, content); }
  async openFile(filePath: string): Promise<void> { return this._file.openFile(filePath); }

  // ═══════════════════════════════════════════════════
  // Visual & Wait (VisualService)
  // ═══════════════════════════════════════════════════

  highlight(elementId: string): void { return this._visual.highlight(elementId); }
  highlightAll(tag: string): void { return this._visual.highlightAll(tag); }
  clearHighlights(): void { return this._visual.clearHighlights(); }
  async waitForElement(tag: string, timeout?: number): Promise<ElementHandle> {
    return this._visual.waitForElement(tag, timeout);
  }
  async waitForWire(fromPort?: string, timeout?: number): Promise<WireHandle> {
    return this._visual.waitForWire(fromPort, timeout);
  }
  async waitFor(predicate: () => boolean, timeout?: number, message?: string): Promise<void> {
    return this._visual.waitFor(predicate, timeout, message);
  }

  // ═══════════════════════════════════════════════════
  // Facade-Only Methods
  // ═══════════════════════════════════════════════════

  // ── Help ──────────────────────────────────────────

  /** Return a self-describing API reference for AI agent discovery.
   *  Call this when you need to know what methods are available. */
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
      other: ["clearAll()", "batch(fn)", "removeElement(id)", "switchMode('design'|'code')", "validate() — canvas health check", "exportState() / importState(json)", "saveSnapshot() / restoreSnapshot(token)", "diff(previousSnapshot)", "snippet(template, options?)", "registerSnippet(name, builder) / unregisterSnippet(name) / listSnippets()", "highlight(id) / highlightAll(tag) / clearHighlights()"],
      portReference: {
        description: "Pure data-flow architecture — all ports are data ports. Use these for connect().",
        signal: { outputs: [{ name: "value", direction: "out", type: "TYPE" }], inputs: [{ name: "set", direction: "in", type: "TYPE" }] },
        computed: { outputs: [{ name: "value", direction: "out", type: "TYPE" }], inputs: [{ name: "deps", direction: "in", type: "any" }] },
        store: { outputs: [{ name: "items", direction: "out", type: "any" }], inputs: [] },
        customFn: { outputs: [{ name: "return", direction: "out", type: "TYPE" }], inputs: "dynamic (from fnParams config)" },
        return: { outputs: [], inputs: [{ name: "value", direction: "in", type: "TYPE" }] },
        add: { outputs: [{ name: "result", direction: "out", type: "TYPE" }], inputs: [{ name: "a", direction: "in", type: "TYPE" }, { name: "b", direction: "in", type: "TYPE" }] },
        branch: { outputs: [{ name: "true", direction: "out", type: "any" }, { name: "false", direction: "out", type: "any" }], inputs: [{ name: "condition", direction: "in", type: "bool" }] },
      },
      eventUuidTable: {
        description: "Event port UUIDs (for manual connect only; bindEvent uses event NAME).",
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
        "Handler functions: switchMode('code') → createTab('event',...) → customFunction(...) → returnNode(...)",
        "signal input port is 'set' NOT 'value'. signal output port is 'value'.",
        "getElements() returns plain ElementData (no methods). Use O.element(id) for ElementHandle.",
        "heading() level is a STRING: '1', '2', '3', '4' — not a number.",
        "Event names use React convention: onPress, onChange, onHover.",
        "compile() discards all programmatic elements — use setSourceCode() first.",
        "customFn must have a return node reachable. Use validateFunction() to check.",
      ],
    };
  }

  // ── Compile ──────────────────────────────────────

  /** Compile the current source and populate the canvas.
   *  This is the programmatic equivalent of clicking the "编译" button. */
  async compile(): Promise<CompileResult> {
    const content = getDesignStore().activeFileContent;
    if (!content) {
      return {
        success: false,
        errors: [{ code: "NO_CONTENT", message: "No file content to compile", line: 0, severity: "error" }],
      };
    }

    const target = window.__OMNI_TARGET__ || "react";
    const result = await api.compile.compile(content, target);

    // Parse view tree and code blocks via local TS parser (replaces Python /api/parse-*)
    try {
      const viewNodes = parseViewTree(content);
      const codeBlocks = parseCodeBlocks(content);

      const elements: ElementData[] = [];

      // Parse view elements
      for (const n of viewNodes) {
        elements.push({
          id: n.id,
          nodeType: "view",
          tag: n.tag,
          props: n.props || {},
          children: n.children?.map((c: { id: string }) => c.id) || [],
          parentId: null,
        });
      }

      // Parse code blocks
      if (codeBlocks.length > 0) {
        const seenIds = new Set<string>();
        // Factory: derive port types from typeAnnotation, using nodeRegistry defaults
        const buildPorts = (blockType: string, typeAnnotation?: string): PortDescriptor[] => {
          const t = typeAnnotation?.trim() || "";
          const vt = t.length > 0 ? t : "i32";
          const cfg = getNodeConfig(blockType);
          return cfg.defaultPorts.map((p) => {
            const shouldMapType =
              p.type === "any" &&
              p.name !== "deps";
            return {
              name: p.name,
              direction: p.direction,
              portType: p.portType,
              type: shouldMapType ? vt : p.type,
              label: p.label,
            };
          });
        };
        for (const b of codeBlocks) {
          if (seenIds.has(b.id)) continue;
          seenIds.add(b.id);
          elements.push({
            id: b.id,
            nodeType: "codeblock",
            tag: b.blockType,
            props: {
              name: b.name,
              parentStore: b.parentStore || "",
              typeAnnotation: b.typeAnnotation || "",
            },
            children: [],
            parentId: b.parentStore
              ? `cb-store-${b.parentStore}`
              : null,
            ports: buildPorts(b.blockType, b.typeAnnotation),
            isFolded: false,
          });
        }
      }

      useEditorStore.setState({ elements });
    } catch {
      // Canvas population is best-effort
    }

    return {
      success: !result.errors?.some((e) => e.severity === "error"),
      errors: result.errors || [],
    };
  }

  // ── Component Discovery ───────────────────────────

  /** List all available component types from the Component Protocol.
   *  Useful for AI agents to discover what components can be created. */
  listComponents(): ComponentInfo[] {
    return Object.values(COMPONENT_PROTOCOL).map((m) => ({
      tag: m.component,
      category: m.category,
      description: m.description,
      props: m.props,
      events: m.events,
      styleProperties: m.styleProperties,
      hasChildren: m.children,
    }));
  }

  /** Get the Component Protocol entry for a specific tag. */
  getComponentInfo(tag: string): ComponentInfo | null {
    const m = COMPONENT_PROTOCOL[tag];
    if (!m) return null;
    return {
      tag: m.component,
      category: m.category,
      description: m.description,
      props: m.props,
      events: m.events,
      styleProperties: m.styleProperties,
      hasChildren: m.children,
    };
  }

  // ── Batch Operations ─────────────────────────────

  /** Execute a batch of operations (state is updated once). */
  batch(fn: (driver: OmniDriver) => void): void {
    fn(this);
  }

  // ═══════════════════════════════════════════════════
  // Snippet Templates (SnippetService)
  // ═══════════════════════════════════════════════════

  /** Register a custom snippet template. */
  registerSnippet(
    name: string,
    builder: (options?: Record<string, unknown>) => ElementHandle,
  ): void {
    this._snippets.registerSnippet(name, builder);
  }

  /** Remove a registered snippet template. */
  unregisterSnippet(name: string): boolean {
    return this._snippets.unregisterSnippet(name);
  }

  /** List all registered snippet names (built-in + custom). */
  listSnippets(): { builtin: string[]; custom: string[] } {
    return this._snippets.listSnippets();
  }

  /** Create a pre-built UI pattern. */
  snippet(template: string, options?: Record<string, unknown>): ElementHandle {
    return this._snippets.snippet(template, options);
  }
}

// ═══════════════════════════════════════════════════════
// Export
// ═══════════════════════════════════════════════════════

export { OmniDriver, ElementHandle, WireHandle };
export default OmniDriver;
