/** OmniDriver — Playwright-like API for programmatic editor control.
 *
 *  Usage from AI / test scripts:
 *  ```js
 *  const omni = window.__OMNI__;
 *
 *  // Create a Button and bind an event handler
 *  const btn = await omni.component("Button", { label: "Submit" });
 *  btn.createEventHandler("onPress", { handlerName: "handleSubmit" });
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
import { getNodeConfig } from "../canvas/nodeRegistry";
import { getAllPorts, validateConnection } from "../canvas/connectionValidators";
import type {
  PortDescriptor,
  ElementQuery,
  CodeBlockOptions,
  CompileResult,
  EditorSnapshot,
  ComponentInfo,
  EventHandlerOptions,
  ValidationReport,
  StateDiff,
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

  /** Remove this element from the canvas.
   *  Cascade-deletes all wires connected to this element. */
  remove(): void {
    const store = this._driver._store();
    // Cascade-delete connected wires before removing the element
    const connectedWires = store.wires.filter(
      (w) => w.fromId === this.id || w.toId === this.id,
    );
    for (const w of connectedWires) {
      store.removeWire(w.id);
    }
    store.removeElement(this.id);
  }

  /** Select this element (shows properties in inspector). */
  select(): this {
    this._driver._store().selectElement(this.id);
    return this;
  }

  /** Get port definitions for this element.
   *  For view elements, event ports from COMPONENT_PROTOCOL are included automatically. */
  getPorts(): PortDescriptor[] {
    return this._driver._getAllPorts(this._require());
  }

  /** Get event metadata for this element from COMPONENT_PROTOCOL.
   *  Returns a map of event name → { id (UUID), description, payload }.
   *  Returns empty object for elements without events. */
  getEvents(): Record<string, { id: string; description: string; payload: string | null }> {
    const d = this._require();
    if (d.nodeType !== "view") return {};
    const meta = COMPONENT_PROTOCOL[d.tag];
    if (!meta?.events) return {};
    // Return a shallow copy to prevent mutation
    return { ...meta.events };
  }

  /** Connect this element to a target element.
   *  Delegates to OmniDriver.connect() which runs full shared validation
   *  (portType mismatch, type compatibility, duplicate wires, cycles, fan-out). */
  connectTo(
    target: ElementHandle,
    fromPort: string,
    toPort: string,
  ): WireHandle {
    return this._driver.connect(this.id, fromPort, target.id, toPort);
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

  /** Bind an event on this element to a handler function.
   *  In the pure data-flow architecture, events are bound in design mode
   *  by setting the event prop on the view element. The actual handler
   *  function should be created as a customFn node in an event-code tab.
   *
   *  This sets the event prop (e.g. onPress="handleSubmit") and returns
   *  this element as the handler handle for backward compatibility. */
  createEventHandler(
    eventName: string,
    options: EventHandlerOptions = {},
  ): { handler: ElementHandle; wire: null } {
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

    const payloadType = eventCfg.payload || "any";

    // Set the event prop on the view element — this is how events are bound in design mode
    this._driver._store().setProp(d.id, eventName, handlerName);

    console.info(
      `[OmniDriver] ✅ Event bound: <${d.tag}>.${eventName} = "${handlerName}"\n` +
      `  💡 提示：处理函数 "${handlerName}" 需要在代码模式的事件标签页中创建（使用 customFn 节点）。\n` +
      `  参数类型：${payloadType}`,
    );

    // Return this element as handler for backward compatibility
    return {
      handler: this,
      wire: null,
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

  /** @internal Remind AI agents to call help() on first API use. */
  private _helpShown = false;

  private _ensureHelp(): void {
    if (!this._helpShown) {
      console.log(
        "[OmniDriver] 💡 首次使用？调用 O.help() 查看完整 API 参考、端口结构和事件 UUID 表",
      );
      this._helpShown = true;
    }
  }

  /** @internal Get all ports for an element, including event ports from COMPONENT_PROTOCOL.
   *  Delegates to shared getAllPorts() which is also used by FlowCanvas. */
  _getAllPorts(el: ElementData): PortDescriptor[] {
    return getAllPorts(el) as PortDescriptor[];
  }

  // ── State Queries ─────────────────────────────────

  /** Get a full snapshot of the editor state. */
  getState(): EditorSnapshot {
    const s = this._store();
    return {
      mode: (window as any).__OMNI_MODE__ || "design",
      elements: s.elements,
      wires: s.wires,
      selectedElementIds: s.selectedElementIds,
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
    const id = this._store().selectedElementIds[0] ?? null;
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
   *  If props contains a `children` array of ElementHandles, they are auto-nested. */
  component(
    tag: string,
    props?: Record<string, unknown>,
    parentId?: string | null,
  ): ElementHandle {
    this._ensureHelp();
    const meta = COMPONENT_PROTOCOL[tag];
    if (!meta) {
      throw new Error(
        `Unknown component tag "${tag}". Available: ${Object.keys(COMPONENT_PROTOCOL).join(", ")}`,
      );
    }

    // Extract inline children from props (Playwright-style: card({ children: [btn, txt] }))
    const inlineChildren: ElementHandle[] = [];
    const cleanProps: Record<string, unknown> = {};
    if (props) {
      for (const [k, v] of Object.entries(props)) {
        if (k === "children" && Array.isArray(v) && v.every((c: unknown) => c instanceof ElementHandle)) {
          inlineChildren.push(...(v as unknown as ElementHandle[]));
        } else {
          cleanProps[k] = v;
        }
      }
    }

    const id = this._nextId();
    const el: ElementData = {
      id,
      nodeType: "view",
      tag,
      props: cleanProps,
      children: [],
      parentId: parentId || null,
    };

    this._store().addElement(el);
    // Note: parentId sync is now handled by store.addElement()

    // Auto-nest inline children
    for (const child of inlineChildren) {
      useEditorStore.setState({
        elements: useEditorStore.getState().elements.map((e) => {
          if (e.id === child.id) return { ...e, parentId: id };
          if (e.id === id) return { ...e, children: [...e.children, child.id] };
          return e;
        }),
      });
    }

    return new ElementHandle(id, this);
  }

  /** Add a code block to the canvas.
   *  Uses getNodeConfig() to derive proper portType, nodeCategory, and default ports. */
  codeBlock(
    blockType: string,
    name: string,
    options: CodeBlockOptions = {},
  ): ElementHandle {
    this._ensureHelp();
    // Determine the value type from typeAnnotation, falling back to "i32"
    const typeAnnotation = options.typeAnnotation?.trim();
    const valueType = typeAnnotation && typeAnnotation.length > 0
      ? typeAnnotation
      : (blockType === "store" ? "any" : "i32");

    // Look up node config from registry for proper portType + nodeCategory
    const config = getNodeConfig(blockType);

    // Clone default ports, mapping value-typed ports to the caller's valueType.
    // Keep deps as "any" (it accepts dependencies of any type).
    const defaultPorts: PortDescriptor[] = config.defaultPorts.map((p) => {
      const shouldMapType =
        p.type === "any" &&
        p.name !== "deps";       // computed deps always accepts "any"
      return {
        name: p.name,
        direction: p.direction,
        portType: p.portType,
        type: shouldMapType ? valueType : p.type,
        label: p.label,
      };
    });

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
      ports: options.ports || defaultPorts,
      isFolded: options.isFolded ?? false,
      source: options.source || "",
      nodeCategory: config.category,
    };

    this._store().addElement(el);

    return new ElementHandle(id, this);
  }

  /** Convenience: add a Button component. */
  button(label: string, props?: Record<string, unknown>, parentId?: string | null): ElementHandle {
    return this.component("Button", { label, ...props }, parentId);
  }

  /** Convenience: add a Text component.
   *  Auto-detects when parentId is mistakenly passed as props (common agent mistake). */
  text(label: string, props?: Record<string, unknown>, parentId?: string | null): ElementHandle {
    if (typeof props === "string") {
      console.warn(
        `[OmniDriver] text() 第2个参数应为 props 对象，收到了字符串 "${props}"。` +
        `你可能把 parentId 传到了 props 位置。已自动修复。正确调用: O.text("${label}", {}, "${props}")`,
      );
      // Auto-recover: treat the string as parentId
      return this.component("Text", { label }, props);
    }
    return this.component("Text", { label, ...props }, parentId);
  }

  /** Convenience: add a Heading component.
   *  Level auto-converts number to string (e.g. 2 → "2"). */
  heading(label: string, level: string | number = "1", parentId?: string | null): ElementHandle {
    const levelStr = String(level);
    return this.component("Heading", { label, level: levelStr }, parentId);
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

  /** Convenience: add an Image component.
   *  Auto-detects when parentId is mistakenly passed as alt (common agent mistake). */
  image(src: string, alt = "", parentId?: string | null): ElementHandle {
    if (alt && alt.startsWith("el-") && !parentId) {
      console.warn(
        `[OmniDriver] image() 第2个参数 alt="${alt}" 看起来像元素 ID。` +
        `你可能把 parentId 传到了 alt 位置。正确调用: O.image("${src}", "", "${alt}")`,
      );
    }
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
      codeBlocks: ["codeBlock(type,name,options) — types: signal, computed, store, literal, add, sub, mul, div, compare, and, or, not, concat, getVariable, setVariable, branch, sequence, forEach, updateUIProperty, fn, customFn, return"],
      elementHandle: ["exists()", "text()", "setProp(k,v)", "getProp(k)", "select()", "remove()", "connectTo(target,fromPort,toPort)", "createEventHandler(event,options)", "getPorts()", "getEvents()", "children()", "parent()", "getSource()", "setSource(src)", "waitForLive(ms)", "data"],
      wireHandle: ["disconnect()", "source()", "target()", "data"],
      events: ["createEventHandler(eventName,options) — pass event NAME (e.g. \"onPress\"), NOT UUID"],
      compile: ["compile() — rebuilds canvas from .omni source, discarding programmatic elements"],
      wait: ["waitForElement(tag,ms)", "waitForWire(port,ms)", "waitFor(predicate,ms,msg)"],
      undo: ["undo()", "redo()"],
      file: ["getSourceCode()", "setSourceCode(content)", "getActiveFilePath()", "save()", "readFile(path)", "writeFile(path,content)", "openFile(path)"],
      discovery_api: ["listComponents()", "getComponentInfo(tag)"],
      other: ["clearAll()", "batch(fn)", "removeElement(id)", "switchMode('design'|'code')", "validate() — canvas health check", "exportState() / importState(json)", "saveSnapshot() / restoreSnapshot(token) — safe rollback", "diff(previousSnapshot)", "snippet(template, options?)", "registerSnippet(name, builder) / unregisterSnippet(name) / listSnippets()", "highlight(id) / highlightAll(tag) / clearHighlights() — visual indicators"],
      componentProtocol: "Use listComponents() to see all 19 view component types and their props/events/styleProperties",
      portReference: {
        description: "Code block port structure — use these port names when calling connect()",
        signal: { outputs: [{ name: "value", direction: "out", type: "TYPE" }], inputs: [{ name: "set", direction: "in", type: "TYPE" }] },
        computed: { outputs: [{ name: "value", direction: "out", type: "TYPE" }], inputs: [{ name: "deps", direction: "in", type: "any" }] },
        store: { outputs: [{ name: "items", direction: "out", type: "any" }], inputs: [] },
        customFn: { outputs: [{ name: "return", direction: "out", type: "TYPE" }], inputs: "dynamic (from fnParams config)" },
        add: { outputs: [{ name: "result", direction: "out", type: "TYPE" }], inputs: [{ name: "a", direction: "in", type: "TYPE" }, { name: "b", direction: "in", type: "TYPE" }] },
        branch: { outputs: [{ name: "true", direction: "out", type: "any" }, { name: "false", direction: "out", type: "any" }], inputs: [{ name: "condition", direction: "in", type: "bool" }] },
      },
      eventUuidTable: {
        description: "Event port UUIDs for manual connect(). createEventHandler() uses event NAME, not UUID.",
        Button: { onPress: "evt-btn-press-001", onHover: "evt-btn-hover-001", onFocus: "evt-btn-focus-001", onLongPress: "evt-btn-longpress-001" },
        Input: { onChange: "evt-input-change-001", onFocus: "evt-input-focus-001", onBlur: "evt-input-blur-001" },
        TextArea: { onChange: "evt-textarea-change-001" },
        Select: { onChange: "evt-select-change-001" },
        Checkbox: { onChange: "evt-checkbox-change-001" },
        Form: { onSubmit: "evt-form-submit-001" },
      },
      criticalRules: [
        "ALWAYS call O.switchMode('design') before creating elements",
        "ALWAYS call O.help() first to discover available APIs",
        "createEventHandler sets event prop on view element (e.g. onPress='handleSubmit'). Handler functions must be created in code mode event-code tabs using customFn nodes.",
        "signal input port is 'set' NOT 'value' or 'input'. signal output port is 'value'.",
        "getElements() returns plain ElementData (no methods). Use O.element(id) for ElementHandle.",
        "O.container() creates Container (plain div), O.card() creates Card (styled container with border/shadow).",
        "heading() level is a STRING: '1', '2', '3', '4' — not a number.",
        "Event names use React convention: onPress, onChange, onHover — NOT press, change, hover.",
        "compile() discards all programmatic elements — use setSourceCode() first for source-driven mode.",
        "Pure data-flow architecture: no flow edges, no event nodes. All connections are data dependencies.",
      ],
    };
  }

  /** Remove an element by ID.  Cascade-deletes all connected wires. */
  removeElement(id: string): void {
    const store = this._store();
    const connectedWires = store.wires.filter(
      (w) => w.fromId === id || w.toId === id,
    );
    for (const w of connectedWires) {
      store.removeWire(w.id);
    }
    store.removeElement(id);
  }

  /** Clear all elements and wires from the canvas. */
  clearAll(): void {
    useEditorStore.setState({ elements: [], wires: [], selectedElementIds: [] });
  }

  // ── Wire Operations ───────────────────────────────

  /** Create a wire between two elements.
   *  Runs full shared validation: portType mismatch, type compatibility,
   *  duplicate wires, cycle detection, fan-out warnings. */
  connect(
    fromId: string,
    fromPort: string,
    toId: string,
    toPort: string,
    wireId?: string,
  ): WireHandle {
    this._ensureHelp();
    const id = wireId || `wire-${fromId}-${fromPort}-${toId}-${toPort}`;
    const fromEl = this._store().elements.find((e) => e.id === fromId);
    const toEl = this._store().elements.find((e) => e.id === toId);

    if (!fromEl) throw new Error(`Source element ${fromId} not found`);
    if (!toEl) throw new Error(`Target element ${toId} not found`);

    // Use shared getAllPorts for consistent port discovery
    const fromPorts = this._getAllPorts(fromEl);
    const toPorts = this._getAllPorts(toEl);

    // Run shared validation (same as FlowCanvas.handleConnect)
    const issues = validateConnection(
      fromEl, fromPort, toEl, toPort, this._store().wires, fromPorts, toPorts,
    );

    // Log all issues
    for (const issue of issues) {
      const level = issue.severity === "error" ? "error" : "warn";
      console[level](`[OmniDriver] connect: ${issue.code} — ${issue.message}`);
    }

    // Block connection on errors (portType mismatch, type incompat, duplicate)
    const errors = issues.filter((i) => i.severity === "error");
    if (errors.length > 0) {
      console.error(
        `[OmniDriver] connect: Connection blocked (${errors.length} error(s)). ` +
        `Wire NOT created: ${fromId}.${fromPort} → ${toId}.${toPort}`,
      );
      return new WireHandle(id, this);
    }

    this._store().addWire({ id, fromId, fromPort, toId, toPort });
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
      success: !result.errors?.some((e: any) => e.severity === "error"),
      errors: result.errors || [],
    };
  }

  // ── Validation (Canvas Health Check) ──────────────

  /** Run a canvas health check — find orphan elements, unwired events,
   *  type mismatches, and unreachable code blocks.
   *  Use this to diagnose issues in a programmatically-built canvas. */
  validate(): ValidationReport {
    const state = this.getState();
    const elements = state.elements;
    const wires = state.wires;

    // 1. Orphan elements: view elements without parent (when multiple roots exist)
    const rootElements = elements.filter((e) => !e.parentId);
    const orphanElements = rootElements
      .filter((e) => e.nodeType === "view" && rootElements.length > 1)
      .map((e) => e.id);

    // 2. Unwired events: view elements with defined events but no connecting wire
    const unwiredEvents: ValidationReport["unwiredEvents"] = [];
    for (const el of elements) {
      if (el.nodeType !== "view") continue;
      const meta = COMPONENT_PROTOCOL[el.tag];
      if (!meta?.events) continue;
      for (const [evtName, evtMeta] of Object.entries(meta.events)) {
        const hasWire = wires.some(
          (w) => w.fromId === el.id && w.fromPort === evtMeta.id,
        );
        if (!hasWire) {
          unwiredEvents.push({
            elementId: el.id,
            tag: el.tag,
            eventName: evtName,
            eventPortId: evtMeta.id,
          });
        }
      }
    }

    // 3. Type mismatches: wires connecting incompatible port types
    const typeMismatches: ValidationReport["typeMismatches"] = [];
    for (const w of wires) {
      const fromEl = elements.find((e) => e.id === w.fromId);
      const toEl = elements.find((e) => e.id === w.toId);
      if (!fromEl || !toEl) continue;
      const fromPorts = this._getAllPorts(fromEl);
      const toPorts = this._getAllPorts(toEl);
      const fromPort = fromPorts.find((p) => p.name === w.fromPort);
      const toPort = toPorts.find((p) => p.name === w.toPort);
      if (
        fromPort && toPort &&
        fromPort.type !== toPort.type &&
        fromPort.type !== "any" && toPort.type !== "any" &&
        fromPort.type !== "event" && toPort.type !== "event"
      ) {
        typeMismatches.push({
          wireId: w.id,
          fromPort: w.fromPort,
          fromType: fromPort.type,
          toPort: w.toPort,
          toType: toPort.type,
        });
      }
    }

    // 4. Unreachable blocks: code blocks with no inbound data wire
    const reachableIds = new Set(wires.map((w) => w.toId));
    const unreachableBlocks = elements
      .filter((e) => e.nodeType === "codeblock" && !reachableIds.has(e.id))
      .map((e) => e.id);

    return {
      orphanElements,
      unwiredEvents,
      typeMismatches,
      unreachableBlocks,
      isValid: orphanElements.length === 0 && typeMismatches.length === 0,
    };
  }

  // ── State Snapshots ───────────────────────────────

  /** Export the full canvas state as a JSON string.
   *  Use with importState() to save/restore canvas snapshots. */
  exportState(): string {
    const state = this.getState();
    return JSON.stringify({
      version: "1.0",
      exportedAt: new Date().toISOString(),
      elements: state.elements,
      wires: state.wires,
    });
  }

  /** Import a canvas state previously exported with exportState().
   *  Replaces all current elements and wires. */
  importState(json: string): void {
    const data = JSON.parse(json);
    if (!data.elements || !data.wires) {
      console.warn("[OmniDriver] importState: invalid state JSON — expected { elements, wires }");
      return;
    }
    useEditorStore.setState({
      elements: data.elements,
      wires: data.wires,
    });
  }

  // ── State Diff ────────────────────────────────────

  /** Compare the current canvas against a previous snapshot.
   *  Returns added/removed/modified elements and wires. */
  diff(previous: EditorSnapshot): StateDiff {
    const current = this.getState();

    const prevElIds = new Set(previous.elements.map((e) => e.id));
    const currElIds = new Set(current.elements.map((e) => e.id));

    // Added elements
    const added = current.elements.filter((e) => !prevElIds.has(e.id));

    // Removed element IDs
    const removed = previous.elements
      .filter((e) => !currElIds.has(e.id))
      .map((e) => e.id);

    // Modified elements
    const prevElMap = new Map(previous.elements.map((e) => [e.id, e]));
    const modified: StateDiff["modified"] = [];
    for (const currEl of current.elements) {
      const prevEl = prevElMap.get(currEl.id);
      if (!prevEl) continue;
      const changes: Record<string, { from: unknown; to: unknown }> = {};
      // Check props
      if (currEl.props) {
        for (const [k, v] of Object.entries(currEl.props)) {
          const prevV = (prevEl.props as Record<string, unknown>)?.[k];
          if (JSON.stringify(v) !== JSON.stringify(prevV)) {
            changes[k] = { from: prevV, to: v };
          }
        }
      }
      // Check source
      if (currEl.source !== prevEl.source) {
        changes["source"] = { from: prevEl.source, to: currEl.source };
      }
      // Check ports
      if (JSON.stringify(currEl.ports) !== JSON.stringify(prevEl.ports)) {
        changes["ports"] = { from: prevEl.ports, to: currEl.ports };
      }
      if (Object.keys(changes).length > 0) {
        modified.push({ id: currEl.id, changes });
      }
    }

    // Wire changes
    const prevWireIds = new Set(previous.wires.map((w) => w.id));
    const currWireIds = new Set(current.wires.map((w) => w.id));
    const addedWires = current.wires.filter((w) => !prevWireIds.has(w.id));
    const removedWires = previous.wires
      .filter((w) => !currWireIds.has(w.id))
      .map((w) => w.id);

    return { added, removed, modified, addedWires, removedWires };
  }

  // ── Snippet Templates ─────────────────────────────

  /** Registry for user-registered snippet builders. */
  private _snippetRegistry: Map<string, (options?: Record<string, unknown>) => ElementHandle> = new Map();

  /** Register a custom snippet template.
   *  The builder function receives an optional options object and must return the root ElementHandle.
   *
   *  @example
   *  O.registerSnippet("product-card", (opts) => {
   *    const card = O.card({ padding: "16px" });
   *    O.image(opts.image, "Product", card.id);
   *    O.heading(opts.title, "3", card.id);
   *    O.text(`$${opts.price}`, {}, card.id);
   *    const btn = O.button("Buy", { variant: "primary" }, card.id);
   *    btn.createEventHandler("onPress", {
   *      handlerName: "handlePurchase",
   *      source: `action handlePurchase(event) {\n  // TODO: add to cart\n}`,
   *    });
   *    return card;
   *  });
   *  O.snippet("product-card", { image: "url", title: "Widget", price: "9.99" });
   */
  registerSnippet(
    name: string,
    builder: (options?: Record<string, unknown>) => ElementHandle,
  ): void {
    this._snippetRegistry.set(name, builder);
  }

  /** Remove a registered snippet template. */
  unregisterSnippet(name: string): boolean {
    return this._snippetRegistry.delete(name);
  }

  /** List all registered snippet names (built-in + custom). */
  listSnippets(): { builtin: string[]; custom: string[] } {
    return {
      builtin: ["login-form", "data-card", "crud-table"],
      custom: Array.from(this._snippetRegistry.keys()),
    };
  }

  /** Create a pre-built UI pattern.
   *  Built-in templates: "login-form", "data-card", "crud-table".
   *  Use registerSnippet() to add custom templates. */
  snippet(
    template: string,
    options?: Record<string, unknown>,
  ): ElementHandle {
    // Check custom registry first (allows overriding built-ins)
    const customBuilder = this._snippetRegistry.get(template);
    if (customBuilder) {
      return customBuilder(options);
    }

    switch (template) {
      case "login-form": {
        const title = (options?.title as string) || "Sign In";
        const card = this.card({ padding: "32px" });
        this.heading(title, "2", card.id);
        this.input("Email", { type: "email" }, card.id);
        this.input("Password", { type: "password" }, card.id);
        const submitBtn = this.button("Sign In", { variant: "primary" }, card.id);
        // Bind event handler (function must be created in code-mode event-code tab)
        submitBtn.createEventHandler("onPress", {
          handlerName: "handleSignIn",
        });
        return card;
      }
      case "data-card": {
        const title = (options?.title as string) || "Card Title";
        const desc = (options?.description as string) || "Card description here.";
        const card = this.card({ padding: "24px" });
        this.heading(title, "3", card.id);
        this.text(desc, {}, card.id);
        if (options?.actionLabel) {
          const actionBtn = this.button(options.actionLabel as string, { variant: "primary" }, card.id);
          // Bind event handler (function must be created in code-mode event-code tab)
          actionBtn.createEventHandler("onPress", {
            handlerName: "handleAction",
          });
        }
        return card;
      }
      case "crud-table": {
        const title = (options?.title as string) || "Data";
        const card = this.card({ padding: "24px" });
        this.heading(title, "2", card.id);
        const toolbar = this.flex("row", "8px", {}, card.id);
        this.input("Search...", {}, toolbar.id);
        const addBtn = this.button("Add", { variant: "primary" }, toolbar.id);
        // Bind event handler (function must be created in code-mode event-code tab)
        addBtn.createEventHandler("onPress", {
          handlerName: "handleAdd",
        });
        // Create table header
        const columns = (options?.columns as string[]) || ["Name", "Status", "Actions"];
        const headerRow = this.flex("row", "12px", {}, card.id);
        for (const col of columns) {
          this.heading(col, "4", headerRow.id);
        }
        return card;
      }
      default:
        throw new Error(
          `Unknown snippet "${template}". Built-in: "login-form", "data-card", "crud-table". ` +
          `Custom: ${Array.from(this._snippetRegistry.keys()).join(", ") || "(none)"}. ` +
          `Use O.registerSnippet(name, builder) to add custom templates.`,
        );
    }
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

  // ── Visual Indicators ─────────────────────────────

  /** Highlight an element on the canvas with a temporary visual indicator.
   *  Dispatches a custom DOM event that the FlowCanvas can listen to.
   *  Use clearHighlights() to remove all indicators.
   *
   *  @example
   *  O.highlight("el-123");          // highlight one element
   *  O.highlightAll("Button");       // highlight all Buttons
   *  O.clearHighlights();            // remove all highlights
   */
  highlight(elementId: string): void {
    const el = this._store().elements.find((e) => e.id === elementId);
    if (!el) {
      console.warn(`[OmniDriver] highlight: element "${elementId}" not found`);
      return;
    }
    window.dispatchEvent(
      new CustomEvent("omni-highlight", {
        detail: { elementIds: [elementId], action: "add" },
      }),
    );
  }

  /** Highlight all elements with the given tag on the canvas. */
  highlightAll(tag: string): void {
    const ids = this._store()
      .elements.filter((e) => e.tag === tag)
      .map((e) => e.id);
    if (ids.length === 0) {
      console.warn(`[OmniDriver] highlightAll: no elements with tag "${tag}" found`);
      return;
    }
    window.dispatchEvent(
      new CustomEvent("omni-highlight", {
        detail: { elementIds: ids, action: "add" },
      }),
    );
  }

  /** Remove all visual highlights from the canvas. */
  clearHighlights(): void {
    window.dispatchEvent(
      new CustomEvent("omni-highlight", {
        detail: { elementIds: [], action: "clear" },
      }),
    );
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

  /** Save a snapshot token that can be restored later with restoreSnapshot().
   *  Use this before risky canvas operations to enable safe rollback.
   *
   *  @example
   *  const token = O.saveSnapshot();
   *  try {
   *    // ... risky canvas operations ...
   *  } catch (e) {
   *    O.restoreSnapshot(token);  // rollback
   *  }
   */
  saveSnapshot(): string {
    return this.exportState();
  }

  /** Restore the canvas to a previously saved snapshot.
   *  @returns true if restore succeeded, false if the snapshot was invalid. */
  restoreSnapshot(snapshotToken: string): boolean {
    try {
      this.importState(snapshotToken);
      return true;
    } catch (e) {
      console.warn("[OmniDriver] restoreSnapshot: failed to restore — invalid snapshot token", e);
      return false;
    }
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
