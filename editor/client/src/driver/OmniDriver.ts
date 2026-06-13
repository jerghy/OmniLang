/** OmniDriver — Playwright-like API for programmatic editor control.
 *
 *  Usage from AI / test scripts:
 *  ```js
 *  const omni = window.__OMNI__;
 *
 *  // Create a Button and connect it to a handler
 *  const btn = await omni.component("Button", { label: "Submit" });
 *  const handler = await omni.codeBlock("action", "handleSubmit");
 *  await btn.connectTo(handler, "evt-btn-press-001", "call");
 *
 *  // Query and modify
 *  const text = omni.findByTag("Text")[0];
 *  text.setProp("label", "Hello World");
 *
 *  // Compile and check
 *  const result = await omni.compile();
 *  console.log(result.errors.length ? "FAIL" : "OK");
 *  ```
 */

import { useEditorStore, type ElementData, type WireData } from "../store/editor";
import { useUndoStore } from "../store/undo";
import { COMPONENT_PROTOCOL, type EventMeta } from "../panels/ComponentPalette";
import { api } from "../api/client";
import type {
  PortDescriptor,
  ElementQuery,
  CodeBlockOptions,
  CompileResult,
  EditorSnapshot,
  ComponentInfo,
  EventHandlerOptions,
} from "./types";

// ═══════════════════════════════════════════════════════
// ElementHandle
// ═══════════════════════════════════════════════════════

class ElementHandle {
  constructor(
    public readonly id: string,
    private readonly _driver: OmniDriver,
  ) {}

  /** Get the live element data from the store. */
  get data(): ElementData | undefined {
    return this._driver._store().elements.find((e) => e.id === this.id);
  }

  /** Assert the element still exists; throws if not found. */
  private _require(): ElementData {
    const d = this.data;
    if (!d) throw new Error(`Element ${this.id} not found in store`);
    return d;
  }

  /** Set a prop value (immutable update).
   *  Warns to console if the prop key does not exist in the element's current props. */
  setProp(prop: string, value: unknown): this {
    const d = this._require();
    // Warn if setting a prop that doesn't exist yet (might be intentional or a typo)
    if (!(prop in (d.props as object)) && prop !== "children") {
      console.warn(
        `[OmniDriver] setProp: "${prop}" is not an existing property on <${d.tag}>. ` +
        `Existing props: ${Object.keys(d.props as object).join(", ") || "(none)"}. ` +
        `This may be intentional for new props, or could be a typo.`,
      );
    }
    this._driver._store().setProp(this.id, prop, value);
    return this;
  }

  /** Get a prop value. */
  getProp(prop: string): unknown {
    return this._driver._store().getProp(this.id, prop);
  }

  /** Remove this element from the canvas. */
  remove(): void {
    this._driver._store().removeElement(this.id);
  }

  /** Select this element (shows properties in inspector). */
  select(): this {
    this._driver._store().selectElement(this.id);
    return this;
  }

  /** Get port definitions for this element. */
  getPorts(): PortDescriptor[] {
    const d = this._require();
    return d.ports || [];
  }

  /** Connect this element to a target element. */
  connectTo(
    target: ElementHandle,
    fromPort: string,
    toPort: string,
  ): WireHandle {
    const wireId = `wire-${this.id}-${fromPort}-${target.id}-${toPort}`;
    this._driver._store().addWire({
      id: wireId,
      fromId: this.id,
      fromPort,
      toId: target.id,
      toPort,
    });
    return new WireHandle(wireId, this._driver);
  }

  /** Check if this element still exists in the store. */
  exists(): boolean {
    return this.data !== undefined;
  }

  /** Get the display text of this element (label or text prop). */
  text(): string {
    const d = this.data;
    if (!d) return "";
    return String(
      (d.props as Record<string, unknown>)["label"] ||
      (d.props as Record<string, unknown>)["text"] ||
      (d.props as Record<string, unknown>)["placeholder"] ||
      ""
    );
  }

  /** Get child elements (READ-ONLY — use `parentId` on creation to set parent). */
  children(): ElementHandle[] {
    const d = this._require();
    return d.children.map((cid) => this._driver.element(cid));
  }

  /** Get the parent element, if any. */
  parent(): ElementHandle | null {
    const d = this._require();
    return d.parentId ? this._driver.element(d.parentId) : null;
  }

  /** Set the source code for a code block. */
  setSource(source: string): this {
    const d = this._require();
    if (d.nodeType === "codeblock") {
      this._driver._store().setElementSource(this.id, source);
    }
    return this;
  }

  /** Get the source code of a code block. */
  getSource(): string | undefined {
    const d = this._require();
    return d.source;
  }

  /** Create an event handler code block wired to this element's event.
   *  This is the programmatic equivalent of clicking the 🔗 button in the PropertyPanel. */
  createEventHandler(
    eventName: string,
    options: EventHandlerOptions = {},
  ): { handler: ElementHandle; wire: WireHandle } {
    const d = this._require();
    const meta = COMPONENT_PROTOCOL[d.tag];
    if (!meta) throw new Error(`No component protocol for tag "${d.tag}"`);

    const eventCfg: EventMeta | undefined = meta.events[eventName];
    if (!eventCfg) {
      throw new Error(
        `Event "${eventName}" not found on ${d.tag}. Available: ${Object.keys(meta.events).join(", ")}`,
      );
    }

    const handlerName =
      options.handlerName ||
      `on${eventName.replace(/^on/, "").charAt(0).toUpperCase()}${eventName.replace(/^on/, "").slice(1)}`;

    const handlerId = `handler-${d.id}-${eventName}`;
    const wireId = `wire-${d.id}-${eventName}-${Date.now()}`;
    const eventPortId = eventCfg.id || eventName;
    const payloadType = eventCfg.payload || "any";

    // Create handler action node
    const handlerEl: ElementData = {
      id: handlerId,
      nodeType: "codeblock",
      tag: "action",
      props: {
        name: handlerName,
        parentStore: "",
        typeAnnotation: "",
      },
      children: [],
      parentId: null,
      ports: [
        {
          name: "call",
          direction: "in",
          type: payloadType,
          label: eventCfg.payload
            ? `${eventName}(${eventCfg.payload})`
            : eventName,
        },
        { name: "result", direction: "out", type: "any", label: "返回结果" },
      ],
      isFolded: options.folded ?? false,
      source:
        options.source ||
        `// 💻 代码块: ${handlerName}
// 接收事件: ${eventCfg.payload ? `${eventName}(payload: ${eventCfg.payload})` : `${eventName}()`}
// 描述: ${eventCfg.description}
action ${handlerName}(event) {
    // 在此编写事件处理逻辑
    ${eventCfg.payload ? `// 事件数据类型: ${eventCfg.payload}` : "// 无事件数据"}
}`,
    };

    this._driver._store().addElement(handlerEl);
    this._driver._store().addWire({
      id: wireId,
      fromId: d.id,
      fromPort: eventPortId,
      toId: handlerId,
      toPort: "call",
    });

    // Also set the event prop on the source element
    this._driver._store().setProp(d.id, eventName, handlerName);

    return {
      handler: new ElementHandle(handlerId, this._driver),
      wire: new WireHandle(wireId, this._driver),
    };
  }

  /** Wait until this element exists in the store (for async operations). */
  async waitForLive(timeout = 5000): Promise<this> {
    const start = Date.now();
    while (Date.now() - start < timeout) {
      if (this._driver._store().elements.find((e) => e.id === this.id)) {
        return this;
      }
      await new Promise((r) => setTimeout(r, 50));
    }
    throw new Error(`Element ${this.id} did not appear within ${timeout}ms`);
  }
}

// ═══════════════════════════════════════════════════════
// WireHandle
// ═══════════════════════════════════════════════════════

class WireHandle {
  constructor(
    public readonly id: string,
    private readonly _driver: OmniDriver,
  ) {}

  /** Get the live wire data from the store. */
  get data(): WireData | undefined {
    return this._driver._store().wires.find((w) => w.id === this.id);
  }

  /** Disconnect (remove) this wire. */
  disconnect(): void {
    this._driver._store().removeWire(this.id);
  }

  /** Get the source element handle. */
  source(): ElementHandle | null {
    const d = this.data;
    return d ? this._driver.element(d.fromId) : null;
  }

  /** Get the target element handle. */
  target(): ElementHandle | null {
    const d = this.data;
    return d ? this._driver.element(d.toId) : null;
  }
}

// ═══════════════════════════════════════════════════════
// OmniDriver
// ═══════════════════════════════════════════════════════

class OmniDriver {
  /** @internal Access the Zustand store. */
  _store() {
    return useEditorStore.getState();
  }

  /** @internal Access the undo store. */
  _undo() {
    return useUndoStore.getState();
  }

  // ── State Queries ─────────────────────────────────

  /** Get a full snapshot of the editor state. */
  getState(): EditorSnapshot {
    const s = this._store();
    return {
      mode: (window as any).__OMNI_MODE__ || "design",
      elements: s.elements,
      wires: s.wires,
      selectedElementId: s.selectedElementId,
      projectDir: s.projectDir,
      activeFilePath: s.activeFilePath,
      compileErrors: s.compileErrors,
      previewUrl: s.previewUrl,
    };
  }

  /** Get all elements currently on the canvas. */
  getElements(): ElementData[] {
    return this._store().elements;
  }

  /** Alias for getElements() — all elements on the canvas. */
  get elements(): ElementData[] {
    return this._store().elements;
  }

  /** Get all wires currently on the canvas. */
  getWires(): WireData[] {
    return this._store().wires;
  }

  /** Alias for getWires() — all wires on the canvas. */
  get wires(): WireData[] {
    return this._store().wires;
  }

  /** Get the currently selected element, or null. */
  getSelectedElement(): ElementHandle | null {
    const id = this._store().selectedElementId;
    return id ? new ElementHandle(id, this) : null;
  }

  /** Get the current editor mode. */
  getMode(): "design" | "code" {
    return (window as any).__OMNI_MODE__ || "design";
  }

  /** Get the preview URL (after deploy). */
  getPreviewUrl(): string | null {
    return this._store().previewUrl;
  }

  // ── Element Handles (Locator Pattern) ─────────────

  /** Get a handle for a specific element by ID.
   *  Always returns an ElementHandle — use exists() or waitForLive() to check if the element is in the store. */
  element(id: string): ElementHandle {
    return new ElementHandle(id, this);
  }

  /** Find all elements matching the given tag. */
  findByTag(tag: string): ElementHandle[] {
    return this._store()
      .elements.filter((e) => e.tag === tag)
      .map((e) => new ElementHandle(e.id, this));
  }

  /** Find elements matching a query object. */
  find(query: ElementQuery): ElementHandle[] {
    let results = this._store().elements;
    if (query.tag) results = results.filter((e) => e.tag === query.tag);
    if (query.nodeType)
      results = results.filter((e) => e.nodeType === query.nodeType);
    if (query.props) {
      results = results.filter((e) =>
        Object.entries(query.props!).every(
          ([k, v]) => (e.props as Record<string, unknown>)[k] === v,
        ),
      );
    }
    if (query.text) {
      const lower = query.text.toLowerCase();
      results = results.filter((e) => {
        const label = (e.props as Record<string, unknown>)["label"];
        return typeof label === "string" && label.toLowerCase().includes(lower);
      });
    }
    return results.map((e) => new ElementHandle(e.id, this));
  }

  /** Find elements by prop key-value. */
  findByProp(prop: string, value: unknown): ElementHandle[] {
    return this._store()
      .elements.filter(
        (e) => (e.props as Record<string, unknown>)[prop] === value,
      )
      .map((e) => new ElementHandle(e.id, this));
  }

  /** Get all view elements. */
  get views(): ElementHandle[] {
    return this.find({ nodeType: "view" });
  }

  /** Get all code block elements. */
  get codeBlocks(): ElementHandle[] {
    return this.find({ nodeType: "codeblock" });
  }

  // ── Canvas Operations ─────────────────────────────

  private _nextId(): string {
    return `el-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
  }

  /** Add a view component to the canvas.
   *  @returns ElementHandle for the created component. */
  component(
    tag: string,
    props?: Record<string, unknown>,
    parentId?: string | null,
  ): ElementHandle {
    const meta = COMPONENT_PROTOCOL[tag];
    if (!meta) {
      throw new Error(
        `Unknown component tag "${tag}". Available: ${Object.keys(COMPONENT_PROTOCOL).join(", ")}`,
      );
    }

    const id = this._nextId();
    const el: ElementData = {
      id,
      nodeType: "view",
      tag,
      props: props || {},
      children: [],
      parentId: parentId || null,
    };

    this._store().addElement(el);

    // If parent specified, add this to parent's children array
    if (parentId) {
      const parent = this._store().elements.find((e) => e.id === parentId);
      if (parent) {
        useEditorStore.setState({
          elements: useEditorStore.getState().elements.map((e) =>
            e.id === parentId
              ? { ...e, children: [...e.children, id] }
              : e,
          ),
        });
      }
    }

    return new ElementHandle(id, this);
  }

  /** Add a code block to the canvas. */
  codeBlock(
    blockType: string,
    name: string,
    options: CodeBlockOptions = {},
  ): ElementHandle {
    const portDefs: Record<
      string,
      Array<{ name: string; direction: "in" | "out"; type: string }>
    > = {
      store: [{ name: "items", direction: "out", type: "any" }],
      signal: [
        { name: "value", direction: "out", type: "i32" },
        { name: "set", direction: "in", type: "i32" },
      ],
      computed: [
        { name: "value", direction: "out", type: "i32" },
        { name: "deps", direction: "in", type: "any" },
      ],
      action: [
        { name: "call", direction: "in", type: "any" },
        { name: "result", direction: "out", type: "any" },
      ],
    };

    const id = this._nextId();
    const parentStore = options.parentStore || "";
    const parentId = parentStore ? `cb-store-${parentStore}` : null;

    const el: ElementData = {
      id,
      nodeType: "codeblock",
      tag: blockType,
      props: {
        name,
        parentStore,
        typeAnnotation: options.typeAnnotation || "",
      },
      children: [],
      parentId,
      ports: options.ports || portDefs[blockType] || [],
      isFolded: options.isFolded ?? false,
      source: options.source || "",
    };

    this._store().addElement(el);

    return new ElementHandle(id, this);
  }

  /** Convenience: add a Button component. */
  button(label: string, props?: Record<string, unknown>, parentId?: string | null): ElementHandle {
    return this.component("Button", { label, ...props }, parentId);
  }

  /** Convenience: add a Text component. */
  text(label: string, props?: Record<string, unknown>, parentId?: string | null): ElementHandle {
    return this.component("Text", { label, ...props }, parentId);
  }

  /** Convenience: add a Heading component. */
  heading(label: string, level = "1", parentId?: string | null): ElementHandle {
    return this.component("Heading", { label, level }, parentId);
  }

  /** Convenience: add a Container component. */
  container(props?: Record<string, unknown>, parentId?: string | null): ElementHandle {
    return this.component("Container", props, parentId);
  }

  /** Convenience: add an Input component. */
  input(placeholder?: string, props?: Record<string, unknown>, parentId?: string | null): ElementHandle {
    return this.component("Input", { placeholder: placeholder || "", ...props }, parentId);
  }

  /** Convenience: add a Card component. */
  card(props?: Record<string, unknown>, parentId?: string | null): ElementHandle {
    return this.component("Card", props, parentId);
  }

  /** Convenience: add a Flex component. */
  flex(direction = "row", gap = "8px", props?: Record<string, unknown>, parentId?: string | null): ElementHandle {
    return this.component("Flex", { direction, gap, ...props }, parentId);
  }

  /** Convenience: add an Image component. */
  image(src: string, alt = "", parentId?: string | null): ElementHandle {
    return this.component("Image", { src, alt }, parentId);
  }

  /** Convenience: add a List component. */
  list(parentId?: string | null): ElementHandle {
    return this.component("List", undefined, parentId);
  }

  /** Convenience: add an Alert component. */
  alert(type = "info", label = "", parentId?: string | null): ElementHandle {
    return this.component("Alert", { type, label }, parentId);
  }

  /** Return a self-describing API reference for AI agent discovery.
   *  Call this when you need to know what methods are available. */
  help(): Record<string, unknown> {
    return {
      version: "1.0.0",
      entryPoint: "window.__OMNI__",
      description: "Playwright-like API for OmniLang editor automation",
      discovery: "Use Object.getOwnPropertyNames(Object.getPrototypeOf(window.__OMNI__)) to list methods",
      state: ["getState()", "getElements() / .elements", "getWires() / .wires", "getMode()", "getSelectedElement()", "getPreviewUrl()", ".views", ".codeBlocks"],
      locator: ["element(id)", "findByTag(tag)", "find(query)", "findByProp(prop,value)"],
      creation: ["component(tag,props?,parentId?)", "button(label,props?,parentId?)", "text(label,props?,parentId?)", "heading(label,level?,parentId?)", "container(props?,parentId?)", "input(placeholder?,props?,parentId?)", "card(props?,parentId?)", "flex(direction?,gap?,props?,parentId?)", "image(src,alt?,parentId?)", "list(parentId?)", "alert(type?,label?,parentId?)"],
      codeBlocks: ["codeBlock(type,name,options) — types: signal, computed, action, store"],
      elementHandle: ["exists()", "text()", "setProp(k,v)", "getProp(k)", "select()", "remove()", "connectTo(target,fromPort,toPort)", "createEventHandler(event,options)", "getPorts()", "children()", "parent()", "getSource()", "setSource(src)", "waitForLive(ms)", "data"],
      wireHandle: ["disconnect()", "source()", "target()", "data"],
      events: ["createEventHandler(eventName,options)"],
      compile: ["compile() — rebuilds canvas from .omni source, discarding programmatic elements"],
      wait: ["waitForElement(tag,ms)", "waitForWire(port,ms)", "waitFor(predicate,ms,msg)"],
      undo: ["undo()", "redo()"],
      file: ["getSourceCode()", "setSourceCode(content)", "getActiveFilePath()", "save()", "readFile(path)", "writeFile(path,content)", "openFile(path)"],
      discovery_api: ["listComponents()", "getComponentInfo(tag)"],
      other: ["clearAll()", "batch(fn)", "removeElement(id)", "switchMode('design'|'code')"],
      componentProtocol: "Use listComponents() to see all 19 view component types and their props/events/styleProperties",
    };
  }

  /** Remove an element by ID. */
  removeElement(id: string): void {
    this._store().removeElement(id);
  }

  /** Clear all elements and wires from the canvas. */
  clearAll(): void {
    useEditorStore.setState({ elements: [], wires: [], selectedElementId: null });
  }

  // ── Wire Operations ───────────────────────────────

  /** Create a wire between two elements. */
  connect(
    fromId: string,
    fromPort: string,
    toId: string,
    toPort: string,
    wireId?: string,
  ): WireHandle {
    const id = wireId || `wire-${fromId}-${fromPort}-${toId}-${toPort}`;
    const fromEl = this._store().elements.find((e) => e.id === fromId);
    const toEl = this._store().elements.find((e) => e.id === toId);

    if (!fromEl) throw new Error(`Source element ${fromId} not found`);
    if (!toEl) throw new Error(`Target element ${toId} not found`);

    // Validate that ports exist on the respective elements
    const fromPortExists = (fromEl.ports || []).some((p) => p.name === fromPort);
    const toPortExists = (toEl.ports || []).some((p) => p.name === toPort);

    if (!fromPortExists && (fromEl.ports || []).length > 0) {
      console.warn(
        `[OmniDriver] connect: port "${fromPort}" not found on <${fromEl.tag}>. ` +
        `Available ports: ${(fromEl.ports || []).map((p) => `${p.name}(${p.direction})`).join(", ") || "(none)"}. ` +
        `If this is an event port, use the UUID (e.g. "evt-btn-press-001"), not the event name.`,
      );
    }
    if (!toPortExists && (toEl.ports || []).length > 0) {
      console.warn(
        `[OmniDriver] connect: port "${toPort}" not found on <${toEl.tag}>. ` +
        `Available ports: ${(toEl.ports || []).map((p) => `${p.name}(${p.direction})`).join(", ") || "(none)"}.`,
      );
    }

    this._store().addWire({
      id,
      fromId,
      fromPort,
      toId,
      toPort,
    });

    return new WireHandle(id, this);
  }

  /** Disconnect a wire by ID. */
  disconnect(wireId: string): void {
    this._store().removeWire(wireId);
  }

  // ── Property Operations ───────────────────────────

  /** Set a prop value on an element. */
  setProp(id: string, prop: string, value: unknown): void {
    this._store().setProp(id, prop, value);
  }

  /** Get a prop value from an element. */
  getProp(id: string, prop: string): unknown {
    return this._store().getProp(id, prop);
  }

  /** Set source code on a code block. */
  setSource(id: string, source: string): void {
    this._store().setElementSource(id, source);
  }

  // ── Selection ─────────────────────────────────────

  /** Select an element by ID. Pass null to deselect. */
  selectElement(id: string | null): void {
    this._store().selectElement(id);
  }

  // ── Mode Switching ────────────────────────────────

  /** Switch between design and code mode. */
  switchMode(mode: "design" | "code"): void {
    (window as any).__OMNI_MODE__ = mode;
    // Dispatch a custom event for App.tsx to pick up
    window.dispatchEvent(
      new CustomEvent("omni-mode-change", { detail: { mode } }),
    );
  }

  // ── Compile ───────────────────────────────────────

  /** Compile the current source and populate the canvas.
   *  This is the programmatic equivalent of clicking the "编译" button. */
  async compile(): Promise<CompileResult> {
    const content = this._store().activeFileContent;
    if (!content) {
      return {
        success: false,
        errors: [{ code: "NO_CONTENT", message: "No file content to compile", line: 0, severity: "error" }],
      };
    }

    const target = (window as any).__OMNI_TARGET__ || "react";
    const result = await api.compile(content, target);

    // Parse view tree and code blocks
    try {
      const [vRes, cbRes] = await Promise.all([
        fetch("/api/parse-view", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ source: content }),
        }),
        fetch("/api/parse-codeblocks", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ source: content }),
        }),
      ]);

      const vData = await vRes.json();
      const cbData = await cbRes.json();

      const elements: ElementData[] = [];

      // Parse view elements
      if (vData.success && vData.nodes) {
        const flattenTree = (
          nodes: any[],
          parentId: string | null,
        ): ElementData[] => {
          const result: ElementData[] = [];
          for (const n of nodes) {
            const el: ElementData = {
              id: n.id,
              nodeType: "view",
              tag: n.tag,
              props: n.props || {},
              children: [],
              parentId,
            };
            if (n.children?.length) {
              el.children = n.children.map((c: any) => c.id);
              result.push(...flattenTree(n.children, n.id));
            }
            result.push(el);
          }
          return result;
        };
        elements.push(...flattenTree(vData.nodes, null));
      }

      // Parse code blocks
      if (cbData.success && cbData.blocks) {
        const seenIds = new Set<string>();
        const portDefs: Record<string, PortDescriptor[]> = {
          store: [{ name: "items", direction: "out", type: "any" }],
          signal: [
            { name: "value", direction: "out", type: "i32" },
            { name: "set", direction: "in", type: "i32" },
          ],
          computed: [
            { name: "value", direction: "out", type: "i32" },
            { name: "deps", direction: "in", type: "any" },
          ],
          action: [
            { name: "call", direction: "in", type: "any" },
            { name: "result", direction: "out", type: "any" },
          ],
        };
        for (const b of cbData.blocks) {
          if (seenIds.has(b.id)) continue;
          seenIds.add(b.id);
          elements.push({
            id: b.id,
            nodeType: "codeblock",
            tag: b.blockType,
            props: {
              name: b.name,
              parentStore: b.parentStore || "",
              typeAnnotation: b.typeAnnotation,
            },
            children: [],
            parentId: b.parentStore
              ? `cb-store-${b.parentStore}`
              : null,
            ports: portDefs[b.blockType] || [],
            isFolded: false,
          });
        }
      }

      useEditorStore.setState({ elements });
    } catch {
      // Canvas population is best-effort
    }

    return {
      success: !result.errors?.some((e: any) => e.severity === "error"),
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

  // ── Wait Mechanisms ───────────────────────────────

  /** Wait for an element with the given tag to appear on the canvas. */
  async waitForElement(
    tag: string,
    timeout = 5000,
  ): Promise<ElementHandle> {
    const start = Date.now();
    while (Date.now() - start < timeout) {
      const found = this._store().elements.find((e) => e.tag === tag);
      if (found) return new ElementHandle(found.id, this);
      await new Promise((r) => setTimeout(r, 100));
    }
    throw new Error(
      `Element with tag "${tag}" did not appear within ${timeout}ms`,
    );
  }

  /** Wait for a wire matching the given port to appear. */
  async waitForWire(fromPort?: string, timeout = 5000): Promise<WireHandle> {
    const start = Date.now();
    while (Date.now() - start < timeout) {
      const found = fromPort
        ? this._store().wires.find((w) => w.fromPort === fromPort)
        : this._store().wires[0];
      if (found) return new WireHandle(found.id, this);
      await new Promise((r) => setTimeout(r, 100));
    }
    throw new Error(
      `Wire matching fromPort="${fromPort}" did not appear within ${timeout}ms`,
    );
  }

  /** Wait for a specific condition to be true. */
  async waitFor(
    predicate: () => boolean,
    timeout = 5000,
    message = "Condition not met",
  ): Promise<void> {
    const start = Date.now();
    while (Date.now() - start < timeout) {
      if (predicate()) return;
      await new Promise((r) => setTimeout(r, 100));
    }
    throw new Error(`${message} within ${timeout}ms`);
  }

  // ── Undo / Redo ──────────────────────────────────

  /** Undo the last operation. */
  undo(): void {
    const store = this._store();
    this._undo().undo({
      getElement: (id) => store.elements.find((e) => e.id === id),
      get elements() { return useEditorStore.getState().elements; },
      get wires() { return useEditorStore.getState().wires; },
      addElement: (el) => store.addElement(el),
      removeElement: (id) => store.removeElement(id),
      setProp: (id, p, v) => store.setProp(id, p, v),
      getProp: (id, p) => store.getProp(id, p) ?? null,
      addWire: (w) => store.addWire(w),
      removeWire: (id) => store.removeWire(id),
    });
  }

  /** Redo the last undone operation. */
  redo(): void {
    const store = this._store();
    this._undo().redo({
      getElement: (id) => store.elements.find((e) => e.id === id),
      get elements() { return useEditorStore.getState().elements; },
      get wires() { return useEditorStore.getState().wires; },
      addElement: (el) => store.addElement(el),
      removeElement: (id) => store.removeElement(id),
      setProp: (id, p, v) => store.setProp(id, p, v),
      getProp: (id, p) => store.getProp(id, p) ?? null,
      addWire: (w) => store.addWire(w),
      removeWire: (id) => store.removeWire(id),
    });
  }

  // ── Batch Operations ─────────────────────────────

  /** Execute a batch of operations (state is updated once). */
  batch(fn: (driver: OmniDriver) => void): void {
    fn(this);
  }

  // ── Source Code ──────────────────────────────────

  /** Get the active file content (the .omni source). */
  getSourceCode(): string | null {
    return this._store().activeFileContent;
  }

  /** Set the active file content. */
  setSourceCode(content: string): void {
    useEditorStore.setState({ activeFileContent: content });
  }

  /** Get the active file path. */
  getActiveFilePath(): string | null {
    return this._store().activeFilePath;
  }

  /** Write the current source to disk. */
  async save(): Promise<boolean> {
    const path = this._store().activeFilePath;
    const content = this._store().activeFileContent;
    if (!path || !content) return false;
    const result = await api.writeFile(path, content);
    return result.ok;
  }

  // ── File Operations ──────────────────────────────

  /** Read a file from the project. */
  async readFile(filePath: string): Promise<string | null> {
    const result = await api.readFile(filePath);
    return result.content || null;
  }

  /** Write content to a file in the project. */
  async writeFile(filePath: string, content: string): Promise<boolean> {
    const result = await api.writeFile(filePath, content);
    return result.ok;
  }

  /** Open a file in the editor. */
  async openFile(filePath: string): Promise<void> {
    const result = await api.readFile(filePath);
    if (result.content) {
      useEditorStore.getState().openFile(filePath, result.content);
    }
  }
}

// ═══════════════════════════════════════════════════════
// Export
// ═══════════════════════════════════════════════════════

export { OmniDriver, ElementHandle, WireHandle };
export default OmniDriver;
