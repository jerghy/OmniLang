/** CanvasDriver — element CRUD, queries, props, and convenience creation methods.
 *  Extracted from OmniDriver god class. */

import { Logger } from "../lib/utils/logger";
import { getDesignStore, isCodeMode, getCodeGraphStore, nextId, ensureHelp } from "./store";
import { COMPONENT_PROTOCOL } from "../components/panels/ComponentPalette";
import { getNodeConfig } from "../components/canvas/shared/nodeRegistry";
import { usePoiesisStore } from "../store";
import { ElementHandle } from "./ElementHandle";
import type { ElementData } from "../core/types/element";
import type { CodeBlockOptions, ElementQuery, PortDescriptor } from "./types";

const log = new Logger({ prefix: "OmniDriver" });

// ═══════════════════════════════════════════════════════
// CanvasDriver
// ═══════════════════════════════════════════════════════

export class CanvasDriver {
  // ── Element Handles (Locator Pattern) ─────────────

  /** Get a handle for a specific element by ID.
   *  Always returns an ElementHandle — use exists() or waitForLive() to check if the element is in the store. */
  element(id: string): ElementHandle {
    return new ElementHandle(id);
  }

  /** Find all elements matching the given tag. */
  findByTag(tag: string): ElementHandle[] {
    return getDesignStore()
      .elements.filter((e) => e.tag === tag)
      .map((e) => new ElementHandle(e.id));
  }

  /** Find elements matching a query object. */
  find(query: ElementQuery): ElementHandle[] {
    let results = getDesignStore().elements;
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
    return results.map((e) => new ElementHandle(e.id));
  }

  /** Find elements by prop key-value. */
  findByProp(prop: string, value: unknown): ElementHandle[] {
    return getDesignStore()
      .elements.filter(
        (e) => (e.props as Record<string, unknown>)[prop] === value,
      )
      .map((e) => new ElementHandle(e.id));
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

  /** Add a view component to the canvas.
   *  If props contains a `children` array of ElementHandles, they are auto-nested. */
  component(
    tag: string,
    props?: Record<string, unknown>,
    parentId?: string | null,
  ): ElementHandle {
    ensureHelp();
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
        if (k === "children" && Array.isArray(v)) {
          const handles = v.filter((c): c is ElementHandle => c instanceof ElementHandle);
          if (handles.length === v.length) {
            inlineChildren.push(...handles);
            continue;
          }
        }
        cleanProps[k] = v;
      }
    }

    const id = nextId();
    const el: ElementData = {
      id,
      nodeType: "view",
      tag,
      props: cleanProps,
      children: [],
      parentId: parentId || null,
    };

    getDesignStore().addElement(el);

    // Auto-nest inline children
    for (const child of inlineChildren) {
      usePoiesisStore.setState({
        elements: usePoiesisStore.getState().elements.map((e) => {
          if (e.id === child.id) return { ...e, parentId: id };
          if (e.id === id) return { ...e, children: [...e.children, child.id] };
          return e;
        }),
      });
    }

    return new ElementHandle(id);
  }

  /** Add a code block to the canvas.
   *  Uses getNodeConfig() to derive proper portType, nodeCategory, and default ports. */
  codeBlock(
    blockType: string,
    name: string,
    options: CodeBlockOptions = {},
  ): ElementHandle {
    ensureHelp();
    // Determine the value type from typeAnnotation, falling back to "i32"
    const typeAnnotation = options.typeAnnotation?.trim();
    const valueType = typeAnnotation && typeAnnotation.length > 0
      ? typeAnnotation
      : (blockType === "store" ? "any" : "i32");

    // Look up node config from registry for proper portType + nodeCategory
    const config = getNodeConfig(blockType);

    // Clone default ports, mapping value-typed ports to the caller's valueType.
    // Keep deps as "any" (it accepts dependencies of any type).
    const defaultPorts: PortDescriptor[] = config.defaultPorts.map((p: PortDescriptor) => {
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

    const id = nextId();
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
    if (isCodeMode()) {
      getCodeGraphStore().addElement(el);
    } else {
      getDesignStore().addElement(el);
    }

    return new ElementHandle(id);
  }

  /** Convenience: add a button component. */
  button(label: string, props?: Record<string, unknown>, parentId?: string | null): ElementHandle {
    return this.component("button", { label, ...props }, parentId);
  }

  /** Convenience: add a p (paragraph/text) component.
   *  Auto-detects when parentId is mistakenly passed as props (common agent mistake). */
  text(label: string, props?: Record<string, unknown>, parentId?: string | null): ElementHandle {
    if (typeof props === "string") {
      log.warn(
        `text() 第2个参数应为 props 对象，收到了字符串 "${props}"。` +
        `你可能把 parentId 传到了 props 位置。已自动修复。正确调用: O.text("${label}", {}, "${props}")`,
      );
      // Auto-recover: treat the string as parentId
      return this.component("p", { label }, props);
    }
    return this.component("p", { label, ...props }, parentId);
  }

  /** Convenience: add a heading (h1-h6) component. */
  heading(label: string, level: string | number = "1", parentId?: string | null): ElementHandle {
    const tag = `h${level}`;
    return this.component(tag, { label }, parentId);
  }

  /** Convenience: add a div container. */
  container(props?: Record<string, unknown>, parentId?: string | null): ElementHandle {
    return this.component("div", props, parentId);
  }

  /** Convenience: add an input component. */
  input(placeholder?: string, props?: Record<string, unknown>, parentId?: string | null): ElementHandle {
    return this.component("input", { placeholder: placeholder || "", ...props }, parentId);
  }

  /** Convenience: add a div (card container). */
  card(props?: Record<string, unknown>, parentId?: string | null): ElementHandle {
    return this.component("div", props, parentId);
  }

  /** Convenience: add a div flex container. */
  flex(direction = "row", gap = "8px", props?: Record<string, unknown>, parentId?: string | null): ElementHandle {
    return this.component("div", { direction, gap, ...props }, parentId);
  }

  /** Convenience: add an img component.
   *  Auto-detects when parentId is mistakenly passed as alt (common agent mistake). */
  image(src: string, alt = "", parentId?: string | null): ElementHandle {
    if (alt && alt.startsWith("el-") && !parentId) {
      log.warn(
        `image() 第2个参数 alt="${alt}" 看起来像元素 ID。` +
        `你可能把 parentId 传到了 alt 位置。正确调用: O.image("${src}", "", "${alt}")`,
      );
    }
    return this.component("img", { src, alt }, parentId);
  }

  /** Convenience: add a ul (unordered list) component. */
  list(parentId?: string | null): ElementHandle {
    return this.component("ul", undefined, parentId);
  }

  /** Convenience: add a div (alert container) component. */
  alert(type = "info", label = "", parentId?: string | null): ElementHandle {
    return this.component("div", { type, label }, parentId);
  }

  /** Remove an element by ID. Cascade-deletes all connected wires. */
  removeElement(id: string): void {
    const store = getDesignStore();
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
    usePoiesisStore.setState({ elements: [], wires: [], selectedElementIds: [] });
  }

  // ── Property Operations ───────────────────────────

  /** Set a prop value on an element. */
  setProp(id: string, prop: string, value: unknown): void {
    getDesignStore().setProp(id, prop, value);
  }

  /** Get a prop value from an element. */
  getProp(id: string, prop: string): unknown {
    return getDesignStore().getProp(id, prop);
  }

  /** Set source code on a code block. */
  setSource(id: string, source: string): void {
    getDesignStore().setElementSource(id, source);
  }

  // ── Selection ─────────────────────────────────────

  /** Select an element by ID. Pass null to deselect. */
  selectElement(id: string | null): void {
    getDesignStore().selectElement(id);
  }

  // ── Mode Switching ────────────────────────────────

  /** Switch between design and code mode. */
  switchMode(mode: "design" | "code"): void {
    usePoiesisStore.getState().switchMode(mode);
    // Dispatch a custom event for App.tsx to pick up
    window.dispatchEvent(
      new CustomEvent("omni-mode-change", { detail: { mode } }),
    );
  }
}
