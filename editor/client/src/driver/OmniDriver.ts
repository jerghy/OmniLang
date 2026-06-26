/** OmniDriver — Playwright-like API for programmatic editor control.
 *
 *  Usage from AI / test scripts:
 *  ```js
 *  const omni = window.__OMNI__;
 *
 *  // Create a Button and bind an event
 *  const btn = omni.component("Button", { label: "Submit" });
 *  btn.bindEvent("onPress", "handleSubmit");
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

import { useEditorStore } from "../store/editor";
import type { ElementData, WireData } from "../core/types";
import { useCodeGraphStore } from "../codegraph/store";
import { useUndoStore } from "../store/undo";
import { COMPONENT_PROTOCOL, type EventMeta } from "../panels/ComponentPalette";
import { getDefaultProps } from "../core/constants";
import { api } from "../services/api";
import { getNodeConfig } from "../ui/canvas/shared/nodeRegistry";
import { getAllPorts, validateConnection } from "../ui/canvas/shared/connectionValidators";
import { uid } from "../core/utils";
import { AddElement } from "../core/commands/AddElement";
import { AddWire } from "../core/commands/AddWire";
import type { CommandContext } from "../core/commands/types";
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
  EventBindingOptions,
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

  /** @deprecated Removed in v2. Use bindEvent(eventName, fnName) instead.
   *  Event handling is now split: bind in design mode, define customFn in code mode. */
  createEventHandler(
    _eventName: string,
    _options?: EventHandlerOptions,
  ): { handler: never; wire: never } {
    throw new Error(
      "[OmniDriver] createEventHandler() has been removed in v2.\n" +
      "  Use bindEvent(eventName, fnName) in design mode to bind the event,\n" +
      "  then create a customFunction() in code mode to define the handler.\n" +
      "  See O.help() for examples.",
    );
  }

  /** Bind an event on this element to a custom function in a code tab.
   *  Pure data-flow v2: sets the event prop (e.g. onPress="handleSubmit")
   *  and optionally stores paramBindings. */
  bindEvent(
    eventName: string,
    fnName: string,
    options: EventBindingOptions = {},
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

    // Store paramBindings as _eventBindings on element config if provided
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
      const boundFn = (d.props as Record<string, unknown>)[evtName];
      if (typeof boundFn === "string" && boundFn.length > 0) {
        bindings.push({
          eventName: evtName,
          fnName: boundFn,
          paramBindings: (d.config?._eventBindings as Record<string, Record<string, import("./types").ParamBinding>>)?.[evtName] || {},
        });
      }
    }
    return bindings;
  }

  /** Add a parameter to a customFn (dynamically adds input port). */
  addParam(name: string, type: string, defaultValue?: unknown): this {
    const d = this._require();
    if (d.tag !== "customFn") throw new Error("addParam is only available on customFn nodes");
    const config = d.config || {};
    const params = [...((config.fnParams as import("./types").FnParam[]) || []), { name, type, default: defaultValue }];
    const newConfig = { ...config, fnParams: params };
    this._driver._store().setProp(this.id, "config", newConfig);
    const inPorts = params.map((p) => ({
      name: p.name, direction: "in" as const, portType: "data" as const,
      type: p.type, label: `${p.name}: ${p.type}`,
    }));
    const outPort = {
      name: "return", direction: "out" as const, portType: "data" as const,
      type: (newConfig.fnReturnType as string) || "void", label: `返回: ${newConfig.fnReturnType || "void"}`,
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
  /** @internal Access the design-mode Zustand store (always, for view elements, file ops, etc.). */
  _store() {
    return useEditorStore.getState();
  }

  /** @internal Check if we are in code mode. */
  private _isCodeMode(): boolean {
    return ((window as any).__OMNI_MODE__ || "design") === "code";
  }

  /** @internal Access the code graph store for tab/graph operations. */
  private _cgStore() {
    return useCodeGraphStore.getState();
  }

  /** @internal Access the active store (design or code graph) for element/wire CRUD.
   *  Returns an object with .elements, .wires, .addElement(), .addWire(), .removeWire(). */
  private _activeStore(): any {
    if (this._isCodeMode()) {
      const cg = useCodeGraphStore.getState();
      const graph = cg.getActiveGraph();
      return {
        get elements() { return graph.elements; },
        get wires() { return graph.wires; },
        addElement: (el: ElementData) => cg.addElement(el),
        removeElement: (id: string) => cg.removeElement(id),
        setProp: (id: string, prop: string, value: unknown) => cg.setProp(id, prop, value),
        getProp: (id: string, prop: string) => cg.getProp(id, prop),
        addWire: (w: WireData) => cg.addWire(w),
        removeWire: (id: string) => cg.removeWire(id),
        setElementSource: (id: string, source: string) => cg.setElementSource(id, source),
        setElementPorts: (id: string, ports: any) => cg.setElementPorts(id, ports),
        selectElement: (_id: string | null, _multi?: boolean) => {},
        get selectedElementIds() { return []; },
        get previewUrl() { return null; },
        get activeFileContent() { return ""; },
        get activeFilePath() { return ""; },
        get projectDir() { return ""; },
      };
    }
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
    return this._activeStore().elements;
  }

  /** Get all wires currently on the canvas. */
  getWires(): WireData[] {
    return this._activeStore().wires;
  }

  /** Alias for getWires() — all wires on the canvas. */
  get wires(): WireData[] {
    return this._activeStore().wires;
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
    return uid("el");
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
    const defaultProps = getDefaultProps(tag);
    const cleanProps: Record<string, unknown> = { ...defaultProps };
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

    const cmdCtx: CommandContext = {
      get elements() { return useEditorStore.getState().elements; },
      get wires() { return useEditorStore.getState().wires; },
      getElement: (id) => useEditorStore.getState().elements.find((e) => e.id === id),
      addElement: (el) => useEditorStore.getState().addElement(el),
      removeElement: (id) => useEditorStore.getState().removeElement(id),
      setProp: (id, p, v) => useEditorStore.getState().setProp(id, p, v),
      getProp: (id, p) => useEditorStore.getState().getProp(id, p) ?? null,
      addWire: (w) => useEditorStore.getState().addWire(w),
      removeWire: (id) => useEditorStore.getState().removeWire(id),
      setElementsAndWires: (els, ws) => useEditorStore.getState().setElementsAndWires(els, ws),
      reorderElement: (...args: any[]) => (useEditorStore.getState() as any).reorderElement(...args),
      moveElement: (...args: any[]) => (useEditorStore.getState() as any).moveElement(...args),
      getRootOrder: () => (useEditorStore.getState() as any).rootOrder,
    };
    new AddElement(el).execute(cmdCtx);
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

    // Route to code graph store when in code mode
    if (this._isCodeMode()) {
      this._cgStore().addElement(el);
    } else {
      this._store().addElement(el);
    }

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
    const store = this._activeStore();
    const id = wireId || `wire-${fromId}-${fromPort}-${toId}-${toPort}`;
    const fromEl = store.elements.find((e: ElementData) => e.id === fromId);
    const toEl = store.elements.find((e: ElementData) => e.id === toId);

    if (!fromEl) throw new Error(`Source element ${fromId} not found`);
    if (!toEl) throw new Error(`Target element ${toId} not found`);

    // Use shared getAllPorts for consistent port discovery
    const fromPorts = this._getAllPorts(fromEl);
    const toPorts = this._getAllPorts(toEl);

    // Run shared validation (same as FlowCanvas.handleConnect)
    const issues = validateConnection(
      fromEl, fromPort, toEl, toPort, store.wires, fromPorts, toPorts,
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

    const wire: WireData = { id, fromId, fromPort, toId, toPort };
    const wireCtx: CommandContext = {
      get elements() { return store.elements; },
      get wires() { return store.wires; },
      getElement: (id) => store.elements.find((e: ElementData) => e.id === id),
      addElement: (el) => store.addElement(el),
      removeElement: (id) => store.removeElement(id),
      setProp: (id, p, v) => store.setProp(id, p, v),
      getProp: (id, p) => store.getProp(id, p) ?? null,
      addWire: (w) => store.addWire(w),
      removeWire: (id) => store.removeWire(id),
      setElementsAndWires: (els, ws) => store.setElementsAndWires?.(els, ws) ?? undefined,
      reorderElement: (...args: any[]) => {},
      moveElement: (...args: any[]) => {},
    };
    new AddWire(wire).execute(wireCtx);
    return new WireHandle(id, this);
  }

  /** Disconnect a wire by ID. */
  disconnect(wireId: string): void {
    this._activeStore().removeWire(wireId);
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
    const result = await api.compile.compile(content, target);

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
   *    btn.bindEvent("onPress", "handlePurchase");
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
        // Bind event (handler function must be created in code-mode event-code tab)
        submitBtn.bindEvent("onPress", "handleSignIn");
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
          // Bind event (handler function must be created in code-mode event-code tab)
          actionBtn.bindEvent("onPress", "handleAction");
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
        // Bind event (handler function must be created in code-mode event-code tab)
        addBtn.bindEvent("onPress", "handleAdd");
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

  // ── Event Binding Queries (v2) ─────────────────────

  /** List all event bindings across all view elements. */
  listAllEventBindings(): import("./types").EventBindingRecord[] {
    const result: import("./types").EventBindingRecord[] = [];
    for (const el of this._store().elements) {
      if (el.nodeType !== "view") continue;
      const handle = new ElementHandle(el.id, this);
      const bindings = handle.getEventBindings();
      for (const b of bindings) {
        let tabId = "";
        const codeStore = (window as any).__CODE_GRAPH_STORE__?.getState?.();
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

  // ── Code Tab Management (v2) ───────────────────────

  /** @internal Access the code graph store. */
  private _codeStore() {
    // Try the global singleton first, fall back to direct import
    const global = (window as any).__CODE_GRAPH_STORE__;
    if (global?.getState) return global.getState();
    try { return useCodeGraphStore.getState(); } catch { return null; }
  }

  /** Create a new code tab. */
  createTab(type: "event" | "logic", label?: string, icon?: string): { id: string } {
    this._ensureHelp();
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
    return store.tabs.map((t: any) => ({
      id: t.id, type: t.type, label: t.label, icon: t.icon,
      variableCount: t.variables.length, elementCount: t.graph.elements.length,
    }));
  }

  /** Get the active tab. */
  getActiveTab(): import("./types").TabInfo | null {
    const store = this._codeStore();
    if (!store) return null;
    const tab = store.getActiveTab();
    if (!tab) return null;
    return { id: tab.id, type: tab.type, label: tab.label, icon: tab.icon,
      variableCount: tab.variables.length, elementCount: tab.graph.elements.length };
  }

  /** Get a tab by ID. */
  getTab(tabId: string): import("./types").TabInfo | null {
    const store = this._codeStore();
    if (!store) return null;
    const tab = store.tabs.find((t: any) => t.id === tabId);
    if (!tab) return null;
    return { id: tab.id, type: tab.type, label: tab.label, icon: tab.icon,
      variableCount: tab.variables.length, elementCount: tab.graph.elements.length };
  }

  /** Deep-clone a code tab. */
  cloneTab(tabId: string, newLabel?: string): { id: string } | null {
    const store = this._codeStore();
    if (!store) return null;
    const srcTab = store.tabs.find((t: any) => t.id === tabId);
    if (!srcTab) return null;
    const idMap = new Map<string, string>();
    const cloneElements = srcTab.graph.elements.map((el: ElementData) => {
      const newElId = uid(); idMap.set(el.id, newElId);
      return { ...el, id: newElId, children: [], parentId: null };
    });
    for (let i = 0; i < cloneElements.length; i++) {
      const orig = srcTab.graph.elements[i];
      if (orig.parentId && idMap.has(orig.parentId)) cloneElements[i].parentId = idMap.get(orig.parentId)!;
      if (orig.children.length > 0) {
        cloneElements[i].children = orig.children.map((cid: string) => idMap.get(cid) || cid).filter(Boolean);
      }
    }
    const cloneWires = srcTab.graph.wires.map((w: WireData) => ({
      ...w, id: uid(),
      fromId: idMap.get(w.fromId) || w.fromId,
      toId: idMap.get(w.toId) || w.toId,
    }));
    const cloneVars = srcTab.variables.map((v: any) => ({ ...v }));
    const id = store.addTab(srcTab.type, newLabel || `${srcTab.label} (Copy)`, {
      icon: srcTab.icon, initialGraph: { elements: cloneElements, wires: cloneWires },
    });
    const tabState = store.tabs.find((t: any) => t.id === id);
    if (tabState) { for (const v of cloneVars) store.defineVariable(v); }
    return { id };
  }

  /** Export a tab to JSON. */
  exportTab(tabId: string): string | null {
    const store = this._codeStore();
    if (!store) return null;
    const tab = store.tabs.find((t: any) => t.id === tabId);
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
      const data: import("./types").TabExportData = JSON.parse(json);
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
    const tab = store.tabs.find((t: any) => t.id === tabId);
    return tab?.variables.map((v: any) => ({
      name: v.name, type: v.type, defaultValue: v.defaultValue, description: v.description,
    })) || [];
  }

  /** Get elements in a specific tab. */
  getTabElements(tabId: string): ElementData[] {
    const store = this._codeStore();
    if (!store) return [];
    const tab = store.tabs.find((t: any) => t.id === tabId);
    return tab?.graph.elements || [];
  }

  /** Get wires in a specific tab. */
  getTabWires(tabId: string): WireData[] {
    const store = this._codeStore();
    if (!store) return [];
    const tab = store.tabs.find((t: any) => t.id === tabId);
    return tab?.graph.wires || [];
  }

  // ── Custom Function Operations (v2) ─────────────────

  /** Create a customFn node. */
  customFunction(fnName: string, fnParams: import("./types").FnParam[], fnReturnType: string): ElementHandle {
    this._ensureHelp();
    const inPorts: import("./types").PortDescriptor[] = fnParams.map((p) => ({
      name: p.name, direction: "in", portType: "data", type: p.type, label: `${p.name}: ${p.type}`,
    }));
    const outPort: import("./types").PortDescriptor = { name: "return", direction: "out", portType: "data", type: fnReturnType, label: `返回: ${fnReturnType}` };
    const id = this._nextId();
    const el: ElementData = {
      id, nodeType: "codeblock", tag: "customFn", props: { name: fnName }, children: [], parentId: null,
      ports: [...inPorts, outPort], nodeCategory: "function" as ElementData["nodeCategory"],
      config: { fnName, fnParams, fnReturnType, description: "" },
    };
    // customFn always goes to code graph store
    this._cgStore().addElement(el);
    return new ElementHandle(id, this);
  }

  /** Create a return node associated with a customFn. */
  returnNode(parentFnId: string): ElementHandle {
    const cgStore = this._cgStore();
    const fnEl = cgStore.getActiveGraph().elements.find((e: ElementData) => e.id === parentFnId);
    if (!fnEl || fnEl.tag !== "customFn") {
      throw new Error(`[OmniDriver] returnNode: parentFnId "${parentFnId}" is not a customFn node`);
    }
    const returnType = (fnEl.config?.fnReturnType as string) || "void";
    const id = this._nextId();
    const el: ElementData = {
      id, nodeType: "codeblock", tag: "return", props: { name: `return ${parentFnId.slice(-6)}` },
      children: [], parentId: parentFnId,
      ports: [{ name: "value", direction: "in", portType: "data", type: returnType, label: "返回值" }],
      nodeCategory: "function" as ElementData["nodeCategory"],
      config: { parentFnId, returnType },
    };
    this._cgStore().addElement(el);
    return new ElementHandle(id, this);
  }

  /** Validate a customFn — scan for return nodes and check type consistency. */
  validateFunction(fnId: string): import("./types").FunctionValidationReport {
    const graph = this._cgStore().getActiveGraph();
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
  listCustomFunctions(): Array<{ fnName: string; params: import("./types").FnParam[]; returnType: string; tabId: string; description: string }> {
    const result: Array<{ fnName: string; params: import("./types").FnParam[]; returnType: string; tabId: string; description: string }> = [];
    // Search editor store (design)
    for (const el of this._store().elements) {
      if (el.tag === "customFn" && el.config?.fnName) {
        result.push({
          fnName: el.config.fnName as string,
          params: (el.config.fnParams as import("./types").FnParam[]) || [],
          returnType: (el.config.fnReturnType as string) || "void",
          tabId: "design", description: (el.config.description as string) || "",
        });
      }
    }
    // Search code graph store (code mode tabs)
    const codeStore = (window as any).__CODE_GRAPH_STORE__?.getState?.();
    if (codeStore?.tabs) {
      for (const tab of codeStore.tabs) {
        for (const el of tab.graph.elements) {
          if (el.tag === "customFn" && el.config?.fnName) {
            result.push({
              fnName: el.config.fnName as string,
              params: (el.config.fnParams as import("./types").FnParam[]) || [],
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
  findFunctionByName(fnName: string): { fnName: string; params: import("./types").FnParam[]; returnType: string; tabId: string; description: string; nodeId: string } | null {
    // Search editor store first
    for (const el of this._store().elements) {
      if (el.tag === "customFn" && el.config?.fnName === fnName) {
        return {
          fnName: el.config.fnName as string,
          params: (el.config.fnParams as import("./types").FnParam[]) || [],
          returnType: (el.config.fnReturnType as string) || "void",
          tabId: "design", description: (el.config.description as string) || "",
          nodeId: el.id,
        };
      }
    }
    // Search code graph store
    const codeStore = (window as any).__CODE_GRAPH_STORE__?.getState?.();
    if (codeStore?.tabs) {
      for (const tab of codeStore.tabs) {
        for (const el of tab.graph.elements) {
          if (el.tag === "customFn" && el.config?.fnName === fnName) {
            return {
              fnName: el.config.fnName as string,
              params: (el.config.fnParams as import("./types").FnParam[]) || [],
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
  getFunctionSignature(fnId: string): import("./types").FunctionSignature | null {
    const el = this._store().elements.find((e) => e.id === fnId);
    if (!el || el.tag !== "customFn") return null;
    const wires = this._store().wires;
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
      params: (el.config?.fnParams as import("./types").FnParam[]) || [],
      returnType: (el.config?.fnReturnType as string) || "void",
      bodyNodeCount: visited.size - 1,
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
    const result = await api.project.writeFile(path, content);
    return result.ok;
  }

  // ── File Operations ──────────────────────────────

  /** Read a file from the project. */
  async readFile(filePath: string): Promise<string | null> {
    const result = await api.project.readFile(filePath);
    return result.content || null;
  }

  /** Write content to a file in the project. */
  async writeFile(filePath: string, content: string): Promise<boolean> {
    const result = await api.project.writeFile(filePath, content);
    return result.ok;
  }

  /** Open a file in the editor. */
  async openFile(filePath: string): Promise<void> {
    const result = await api.project.readFile(filePath);
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
