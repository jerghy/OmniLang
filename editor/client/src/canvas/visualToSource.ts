/** Visual Graph (elements + wires) → .omni source text exporter.
 *  Converts the Zustand store state into a valid .omni source representation
 *  that can be compiled by the Python backend into React TSX / Flutter Dart. */

import type { ElementData, WireData } from "../store/editor";

interface VisualToSourceOptions {
  componentName?: string;
}

/** Generate .omni source from visual graph elements and wires. */
export function visualToSource(
  elements: ElementData[],
  wires: WireData[],
  options: VisualToSourceOptions = {},
): string {
  const name = options.componentName || "App";
  const lines: string[] = [];

  lines.push(`component ${name} {`);

  // Group elements by type
  const signals = elements.filter((e) => e.tag === "signal");
  const computeds = elements.filter((e) => e.tag === "computed");
  const stores = elements.filter((e) => e.tag === "store");
  const actions = elements.filter(
    (e) => e.tag === "action" || e.tag === "setVariable" || e.tag === "runCodeBlock",
  );
  const viewElements = elements.filter((e) => e.nodeType === "view");

  // ── Signals ──
  for (const s of signals) {
    const name = s.props.name || "unnamed";
    const typeAnn = (s.props.typeAnnotation as string) || "i32";
    const defaultValue = typeAnn === "i32" ? "0" : typeAnn === "f64" ? "0.0"
      : typeAnn === "bool" ? "false" : typeAnn === "String" ? '""' : "0";
    lines.push(`  signal ${name}: ${typeAnn} = ${defaultValue}`);
  }

  // ── Computeds ──
  for (const c of computeds) {
    const name = c.props.name || "unnamed";
    const typeAnn = (c.props.typeAnnotation as string) || "i32";
    if (c.source) {
      lines.push(`  ${c.source}`);
    } else {
      lines.push(`  computed ${name}() -> ${typeAnn} { /* TODO */ }`);
    }
  }

  // ── Stores ──
  for (const s of stores) {
    const name = s.props.name || "unnamed";
    lines.push(`  store ${name} { }`);
  }

  // ── Actions / Fns ──
  for (const a of actions) {
    if (a.source) {
      lines.push(`  ${a.source}`);
    } else {
      const name = a.props.name || "handler";
      lines.push(`  action ${name}(event) { /* TODO */ }`);
    }
  }

  // ── Build view tree from visual elements ──
  const rootViews = viewElements.filter((e) => !e.parentId);
  if (rootViews.length > 0) {
    lines.push("");
    lines.push("  view {");
    for (const root of rootViews) {
      lines.push(...buildViewTree(root, viewElements, wires, "    "));
    }
    lines.push("  }");
  }

  // ── Event wirings ──
  for (const w of wires) {
    const fromEl = elements.find((e) => e.id === w.fromId);
    const toEl = elements.find((e) => e.id === w.toId);
    if (fromEl?.nodeType === "view" && toEl?.tag === "action") {
      const btnLabel = fromEl.props.label || fromEl.props.name;
      const handlerName = toEl.props.name || "handler";
      // Find which event: look at fromPort
      const evtName = w.fromPort === "evt-btn-press-001" ? "onPress"
        : w.fromPort === "evt-input-change-001" ? "onChange"
        : w.fromPort;
      // Annotate the view element with the event handler
      // This is a best-effort mapping — the Python compiler needs matching fn names
    }
  }

  lines.push("}");

  return lines.join("\n");
}

/** Recursively build view element XML from the visual tree. */
function buildViewTree(
  el: ElementData,
  allViews: ElementData[],
  wires: WireData[],
  indent: string,
): string[] {
  const lines: string[] = [];
  const tag = el.tag;

  // Build props string
  const props = buildProps(el, wires);
  const children = allViews.filter((e) => e.parentId === el.id);

  if (children.length === 0 && isSelfClosing(tag)) {
    lines.push(`${indent}<${tag}${props} />`);
  } else if (children.length === 0) {
    const text = el.props.label || el.props.text || "";
    lines.push(`${indent}<${tag}${props}>${typeof text === "string" ? text : ""}</${tag}>`);
  } else {
    lines.push(`${indent}<${tag}${props}>`);
    for (const child of children) {
      lines.push(...buildViewTree(child, allViews, wires, indent + "  "));
    }
    lines.push(`${indent}</${tag}>`);
  }

  return lines;
}

function buildProps(el: ElementData, _wires: WireData[]): string {
  const parts: string[] = [];
  const p = el.props as Record<string, unknown>;
  for (const [k, v] of Object.entries(p)) {
    if (k === "children" || k === "parentStore" || k === "typeAnnotation") continue;
    if (v === "" || v === undefined || v === null) continue;
    if (typeof v === "string") {
      parts.push(`${k}="${v}"`);
    } else if (typeof v === "object") {
      parts.push(`${k}={${JSON.stringify(v)}}`);
    }
  }
  return parts.length > 0 ? " " + parts.join(" ") : "";
}

function isSelfClosing(tag: string): boolean {
  return ["Image", "Input", "Spinner", "Progress"].includes(tag);
}

/** Quick export: get .omni source from the current store state. */
export function exportOmniSource(componentName?: string): string {
  // Dynamic import to avoid circular dependency — caller provides the store
  const { useEditorStore } = require("../store/editor");
  const state = useEditorStore.getState();
  return visualToSource(state.elements, state.wires, { componentName });
}
