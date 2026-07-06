/** ElementHandle — locator-style handle for programmatic element control.
 *  Extracted from OmniDriver god class.
 *  Constructor takes only the element ID (imports store directly). */

import { Logger } from "../lib/utils/logger";
import { getDesignStore, getAllPorts } from "./store";
import { COMPONENT_PROTOCOL } from "../components/panels/ComponentPalette";
import type { ElementData } from "../core/types/element";
import type {
  PortDescriptor,
  EventBindingOptions,
  EventHandlerOptions,
  FnParam,
  ParamBinding,
} from "./types";
import type { WireHandle } from "./WireHandle";
import { connect } from "./ConnectionDriver";

const log = new Logger({ prefix: "OmniDriver" });

// ═══════════════════════════════════════════════════════
// ElementHandle
// ═══════════════════════════════════════════════════════

export class ElementHandle {
  constructor(public readonly id: string) {}

  /** Get the live element data from the store. */
  get data(): ElementData | undefined {
    return getDesignStore().elements.find((e) => e.id === this.id);
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
      log.warn(
        `setProp: "${prop}" is not an existing property on <${d.tag}>. ` +
        `Existing props: ${Object.keys(d.props as object).join(", ") || "(none)"}. ` +
        `This may be intentional for new props, or could be a typo.`,
      );
    }
    getDesignStore().setProp(this.id, prop, value);
    return this;
  }

  /** Get a prop value. */
  getProp(prop: string): unknown {
    return getDesignStore().getProp(this.id, prop);
  }

  /** Remove this element from the canvas.
   *  Cascade-deletes all wires connected to this element. */
  remove(): void {
    const store = getDesignStore();
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
    getDesignStore().selectElement(this.id);
    return this;
  }

  /** Get port definitions for this element.
   *  For view elements, event ports from COMPONENT_PROTOCOL are included automatically. */
  getPorts(): PortDescriptor[] {
    return getAllPorts(this._require()) as PortDescriptor[];
  }

  /** Get event metadata for this element from COMPONENT_PROTOCOL.
   *  Returns a map of event name -> { id (UUID), description, payload }.
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
   *  Delegates to standalone connect() which runs full shared validation
   *  (portType mismatch, type compatibility, duplicate wires, cycles, fan-out). */
  connectTo(
    target: ElementHandle,
    fromPort: string,
    toPort: string,
  ): WireHandle {
    return connect(this.id, fromPort, target.id, toPort);
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
    return d.children.map((cid) => new ElementHandle(cid));
  }

  /** Get the parent element, if any. */
  parent(): ElementHandle | null {
    const d = this._require();
    return d.parentId ? new ElementHandle(d.parentId) : null;
  }

  /** Set the source code for a code block. */
  setSource(source: string): this {
    const d = this._require();
    if (d.nodeType === "codeblock") {
      getDesignStore().setElementSource(this.id, source);
    }
    return this;
  }

  /** Get the source code of a code block. */
  getSource(): string | undefined {
    const d = this._require();
    return d.source;
  }

  /** @deprecated Removed in v2. Use bindEvent(eventName, fnName) instead. */
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

    const eventCfg = meta.events[eventName];
    if (!eventCfg) {
      const available = Object.keys(meta.events).join(", ");
      throw new Error(
        `Event "${eventName}" not found on ${d.tag}. Available: ${available || "(none)"}`,
      );
    }

    // Set event prop on the view element
    getDesignStore().setProp(d.id, eventName, fnName);

    // Store paramBindings as _eventBindings on element config if provided
    if (options.params && Object.keys(options.params).length > 0) {
      getDesignStore().setEventBindings(d.id, eventName, {
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
      log.warn(`unbindEvent: "${eventName}" is not bound on <${d.tag}>`);
      return this;
    }
    getDesignStore().setProp(d.id, eventName, undefined as unknown);
    getDesignStore().removeEventBindings(d.id, eventName);
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
  getEventBindings(): Array<{ eventName: string; fnName: string; paramBindings: Record<string, ParamBinding> }> {
    const d = this._require();
    if (d.nodeType !== "view") return [];
    const bindings: Array<{ eventName: string; fnName: string; paramBindings: Record<string, ParamBinding> }> = [];
    const meta = COMPONENT_PROTOCOL[d.tag];
    if (!meta?.events) return [];
    for (const [evtName] of Object.entries(meta.events)) {
      const boundFn = (d.props as Record<string, unknown>)[evtName];
      if (typeof boundFn === "string" && boundFn.length > 0) {
        bindings.push({
          eventName: evtName,
          fnName: boundFn,
          paramBindings: ((d.config?._eventBindings as Record<string, Record<string, ParamBinding>>)?.[evtName]) || {},
        });
      }
    }
    return bindings;
  }

  /** Add a parameter to a customFn (dynamically adds input port). */
  addParam(name: string, type: string, defaultValue?: unknown): this {
    const d = this._require();
    if (d.tag !== "customFn") throw new Error("addParam is only available on customFn nodes");
    const config = (d.config || {}) as Record<string, unknown>;
    const params = [...((config.fnParams as FnParam[]) || []), { name, type, default: defaultValue }];
    const newConfig = { ...config, fnParams: params } as Record<string, unknown>;
    getDesignStore().setProp(this.id, "config", newConfig);
    const inPorts = params.map((p) => ({
      name: p.name, direction: "in" as const, portType: "data" as const,
      type: p.type, label: `${p.name}: ${p.type}`,
    }));
    const outPort = {
      name: "return", direction: "out" as const, portType: "data" as const,
      type: (newConfig.fnReturnType as string) || "void", label: `返回: ${newConfig.fnReturnType || "void"}`,
    };
    getDesignStore().setElementPorts(this.id, [...inPorts, outPort]);
    return this;
  }

  /** Set the return type of a customFn. */
  setReturnType(type: string): this {
    const d = this._require();
    if (d.tag !== "customFn") throw new Error("setReturnType is only available on customFn nodes");
    const config = { ...d.config, fnReturnType: type };
    getDesignStore().setProp(this.id, "config", config);
    const ports = d.ports || [];
    const updated = ports.map((p) =>
      p.name === "return" ? { ...p, type, label: `返回: ${type}` } : p,
    );
    getDesignStore().setElementPorts(this.id, updated);
    return this;
  }

  /** Set the description of a customFn. */
  setDescription(desc: string): this {
    const d = this._require();
    if (d.tag !== "customFn") throw new Error("setDescription is only available on customFn nodes");
    const config = { ...d.config, description: desc };
    getDesignStore().setProp(this.id, "config", config);
    return this;
  }

  /** Wait until this element exists in the store (for async operations). */
  async waitForLive(timeout = 5000): Promise<this> {
    const start = Date.now();
    while (Date.now() - start < timeout) {
      if (getDesignStore().elements.find((e) => e.id === this.id)) {
        return this;
      }
      await new Promise((r) => setTimeout(r, 50));
    }
    throw new Error(`Element ${this.id} did not appear within ${timeout}ms`);
  }
}
