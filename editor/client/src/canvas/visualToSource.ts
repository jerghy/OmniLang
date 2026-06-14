/** Visual Graph (elements + wires) → .omni source text exporter.
 *
 *  Generates valid .omni source from FlowCanvas elements and wires,
 *  handling ALL 23 registered node types:
 *    - Code blocks: signal, computed, store, action, fn, fn-call
 *    - Visual events: onPress, onChange
 *    - Visual actions: setVariable, runCodeBlock, updateUIProperty
 *    - Visual data: getVariable, add, sub, mul, div, compare, and, or, not, concat, literal
 *    - Flow control: branch, sequence, forEach
 *    - View elements: all UI components from design mode
 *
 *  Architecture:
 *    1. Separate code blocks (have source) from visual nodes and view elements
 *    2. Trace flow chains from event nodes through flow edges
 *    3. Generate fn bodies from flow chains, resolving data expressions inline
 *    4. Assemble complete .omni component source
 */

import type { ElementData, WireData } from "../store/editor";

// ═══════════════════════════════════════════════════
// Types
// ═══════════════════════════════════════════════════

interface VisualToSourceOptions {
  componentName?: string;
}

interface SourceGenContext {
  elements: Map<string, ElementData>;
  wires: WireData[];
  /** fromId → outgoing wires */
  outWires: Map<string, WireData[]>;
  /** toId → incoming wires */
  inWires: Map<string, WireData[]>;
  /** toId:toPort → source wire (for data input resolution) */
  dataInputs: Map<string, WireData>;
  /** Names of generated fns (to avoid duplicates) */
  usedFnNames: Set<string>;
  /** View elements indexed by id */
  viewElements: Map<string, ElementData>;
  /** All code blocks (signal/computed/store/action) indexed by id */
  codeBlocks: Map<string, ElementData>;
}

/** A flow chain: ordered list of nodes from an event through flow edges. */
interface FlowNode {
  elementId: string;
  children?: FlowNode[]; // for branch/sequence/forEach
}

// ═══════════════════════════════════════════════════
// Public API
// ═══════════════════════════════════════════════════

/** Generate .omni source from visual graph elements and wires. */
export function visualToSource(
  elements: ElementData[],
  wires: WireData[],
  options: VisualToSourceOptions = {},
): string {
  const ctx = buildContext(elements, wires);
  const name = options.componentName || "App";
  const lines: string[] = [];

  lines.push(`import omniview;`);
  lines.push("");
  lines.push(`component ${name} {`);

  // ── 1. Code block declarations (signal / computed / store) ──
  emitCodeBlockDeclarations(lines, ctx);

  // ── 2. Generate fns from visual flow chains ──
  const generatedFns = emitVisualFlows(lines, ctx);

  // ── 3. View tree ──
  const rootViews = [...ctx.viewElements.values()].filter((e) => !e.parentId);
  if (rootViews.length > 0) {
    lines.push("");
    lines.push("  view {");
    for (const root of rootViews) {
      lines.push(...buildViewTree(root, ctx, "    "));
    }
    lines.push("  }");
  } else if (generatedFns.length > 0 || ctx.codeBlocks.size > 0) {
    // If we have logic but no view, emit a minimal view
    lines.push("");
    lines.push("  view {");
    lines.push('    <Text label="OmniView App" />');
    lines.push("  }");
  }

  lines.push("}");

  return lines.join("\n") + "\n";
}

/** Quick export: get .omni source from the current store state. */
export function exportOmniSource(componentName?: string): string {
  const { useEditorStore } = require("../store/editor");
  const state = useEditorStore.getState();
  return visualToSource(state.elements, state.wires, { componentName });
}

// ═══════════════════════════════════════════════════
// Context builder
// ═══════════════════════════════════════════════════

function buildContext(elements: ElementData[], wires: WireData[]): SourceGenContext {
  const elMap = new Map(elements.map((e) => [e.id, e]));

  const outWires = new Map<string, WireData[]>();
  const inWires = new Map<string, WireData[]>();
  const dataInputs = new Map<string, WireData>();

  for (const w of wires) {
    // outWires: fromId → wires
    const out = outWires.get(w.fromId) || [];
    out.push(w);
    outWires.set(w.fromId, out);

    // inWires: toId → wires
    const inp = inWires.get(w.toId) || [];
    inp.push(w);
    inWires.set(w.toId, inp);

    // dataInputs: toId:toPort → wire (for data port resolution)
    const toEl = elMap.get(w.toId);
    const toPort = toEl?.ports?.find((p) => p.name === w.toPort);
    if (toPort?.portType === "data") {
      dataInputs.set(`${w.toId}:${w.toPort}`, w);
    }
  }

  const viewElements = new Map<string, ElementData>();
  const codeBlocks = new Map<string, ElementData>();

  for (const el of elements) {
    if (el.nodeType === "view") {
      viewElements.set(el.id, el);
    } else if (el.nodeType === "codeblock") {
      codeBlocks.set(el.id, el);
    }
  }

  return {
    elements: elMap,
    wires,
    outWires,
    inWires,
    dataInputs,
    usedFnNames: new Set(),
    viewElements,
    codeBlocks,
  };
}

// ═══════════════════════════════════════════════════
// Code block declarations
// ═══════════════════════════════════════════════════

const CODE_BLOCK_TAGS = new Set(["signal", "computed", "store", "action", "fn"]);

function emitCodeBlockDeclarations(lines: string[], ctx: SourceGenContext): void {
  const emitted = new Set<string>();

  // Signals first
  for (const el of ctx.codeBlocks.values()) {
    if (el.tag === "signal" && !emitted.has(el.id)) {
      emitted.add(el.id);
      const sName = (el.props.name as string) || "unnamed";
      const typeAnn = (el.props.typeAnnotation as string) || "i32";
      const defaultVal = inferDefault(typeAnn);
      lines.push(`  signal ${sName}: ${typeAnn} = ${defaultVal};`);
    }
  }

  // Computeds
  for (const el of ctx.codeBlocks.values()) {
    if (el.tag === "computed" && !emitted.has(el.id)) {
      emitted.add(el.id);
      if (el.source) {
        lines.push(`  ${el.source}`);
      } else {
        const cName = (el.props.name as string) || "unnamed";
        const typeAnn = (el.props.typeAnnotation as string) || "i32";
        lines.push(`  computed ${cName}() -> ${typeAnn} { /* TODO */ }`);
      }
    }
  }

  // Stores
  for (const el of ctx.codeBlocks.values()) {
    if (el.tag === "store" && !emitted.has(el.id)) {
      emitted.add(el.id);
      const name = (el.props.name as string) || "unnamed";
      lines.push(`  store ${name} { }`);
    }
  }
}

function inferDefault(typeAnn: string): string {
  switch (typeAnn) {
    case "i32": return "0";
    case "f64": return "0.0";
    case "bool": return "false";
    case "String": return '""';
    default: return "0";
  }
}

// ═══════════════════════════════════════════════════
// Visual flow → fn generation
// ═══════════════════════════════════════════════════

/** Find event nodes and trace flow chains, generating fn declarations. */
function emitVisualFlows(lines: string[], ctx: SourceGenContext): string[] {
  const generated: string[] = [];

  // Find event nodes (visual script events, not view elements)
  const eventNodes = [...ctx.elements.values()].filter(
    (el) => el.nodeCategory === "event" || el.tag === "onPress" || el.tag === "onChange",
  );

  if (eventNodes.length === 0) {
    // Still emit action sources
    emitActionSources(lines, ctx);
    return generated;
  }

  lines.push("");

  for (const eventNode of eventNodes) {
    // Find what view element this event is connected TO (via data edge)
    const connectedView = findViewForEvent(eventNode, ctx);

    const fnName = makeFnName(eventNode, ctx);
    ctx.usedFnNames.add(fnName);

    // Trace flow chain from this event
    const bodyLines = traceFlowChain(eventNode, ctx, "    ");

    const params = eventNode.tag === "onPress" ? "" : "value: String";
    lines.push(`  fn ${fnName}(${params}) {`);
    if (bodyLines.length === 0) {
      lines.push("    // TODO: connect visual nodes to this event");
    } else {
      lines.push(...bodyLines);
    }
    lines.push("  }");

    // Wire the view element to this fn
    if (connectedView) {
      const viewTag = connectedView.tag;
      const eventName = eventNode.tag === "onPress" ? "onPress" : "onChange";
      connectedView.props = { ...connectedView.props, [`_event_${eventName}`]: fnName };
    }

    generated.push(fnName);
  }

  // Also emit any action sources that are not part of flow chains
  emitActionSources(lines, ctx);

  return generated;
}

/** Find the view element that an event node connects TO (via outgoing data wire for event data). */
function findViewForEvent(
  eventNode: ElementData,
  ctx: SourceGenContext,
): ElementData | null {
  const outW = ctx.outWires.get(eventNode.id) || [];
  for (const w of outW) {
    const toEl = ctx.elements.get(w.toId);
    if (toEl?.nodeType === "view") return toEl;
  }
  // Also check: does any wire go FROM a view element to this event?
  const inW = ctx.inWires.get(eventNode.id) || [];
  for (const w of inW) {
    const fromEl = ctx.elements.get(w.fromId);
    if (fromEl?.nodeType === "view") return fromEl;
  }
  return null;
}

/** Generate a unique fn name for an event node. */
function makeFnName(eventNode: ElementData, ctx: SourceGenContext): string {
  const base = eventNode.tag === "onPress" ? "handlePress" : "handleChange";
  if (!ctx.usedFnNames.has(base)) return base;
  let i = 2;
  while (ctx.usedFnNames.has(`${base}${i}`)) i++;
  return `${base}${i}`;
}

/** Emit action code blocks that have source code but aren't part of any flow chain. */
function emitActionSources(lines: string[], ctx: SourceGenContext): void {
  const emitted = new Set<string>();
  // Only emit actions that haven't been consumed by flow chain generation
  for (const el of ctx.codeBlocks.values()) {
    if ((el.tag === "action" || el.tag === "fn") && !emitted.has(el.id)) {
      emitted.add(el.id);
      if (el.source) {
        lines.push(`  ${el.source}`);
      }
    }
  }
  // Also emit fn-call as comments for reference
  for (const el of ctx.elements.values()) {
    if (el.tag === "fn-call" && !emitted.has(el.id)) {
      emitted.add(el.id);
      const fnName = (el.props.name as string) || "unnamed";
      lines.push(`  // fn-call: ${fnName}() — wire to an event to generate code`);
    }
  }
}

// ═══════════════════════════════════════════════════
// Flow chain tracing (BFS through flow edges)
// ═══════════════════════════════════════════════════

/** Trace the flow chain from a starting node, generating .omni code lines. */
function traceFlowChain(
  startNode: ElementData,
  ctx: SourceGenContext,
  indent: string,
  visited: Set<string> = new Set(),
): string[] {
  const lines: string[] = [];

  // Find flow-out edges from this node
  const flowOuts = findFlowOutEdges(startNode, ctx);

  // If this is a branch node, handle specially
  if (startNode.tag === "branch") {
    return generateBranch(startNode, ctx, indent, visited);
  }

  // If this is a sequence node
  if (startNode.tag === "sequence") {
    return generateSequence(startNode, ctx, indent, visited);
  }

  // If this is a forEach
  if (startNode.tag === "forEach") {
    return generateForEach(startNode, ctx, indent, visited);
  }

  // For normal nodes: generate a statement and follow flow edges
  if (startNode.nodeCategory === "action") {
    const stmt = generateStatement(startNode, ctx);
    if (stmt) lines.push(`${indent}${stmt}`);
  }

  // Follow flow-out edges
  for (const w of flowOuts) {
    const toId = w.toId;
    if (visited.has(toId)) continue;
    const toEl = ctx.elements.get(toId);
    if (!toEl) continue;

    // Only follow if the target port is a flow-in port
    const toPort = toEl.ports?.find((p) => p.name === w.toPort);
    if (toPort?.portType === "flow") {
      visited.add(toId);
      lines.push(...traceFlowChain(toEl, ctx, indent, visited));
    }
  }

  return lines;
}

/** Find all flow-type outgoing wires from a node. */
function findFlowOutEdges(node: ElementData, ctx: SourceGenContext): WireData[] {
  const outs = ctx.outWires.get(node.id) || [];
  return outs.filter((w) => {
    const fromPort = node.ports?.find((p) => p.name === w.fromPort);
    return fromPort?.portType === "flow";
  });
}

// ═══════════════════════════════════════════════════
// Statement generation (action nodes)
// ═══════════════════════════════════════════════════

/** Generate a single .omni statement from an action node. */
function generateStatement(el: ElementData, ctx: SourceGenContext): string {
  switch (el.tag) {
    case "setVariable": {
      // Find the signal name and the value expression
      const varName = (el.props.name as string) || resolveVariableName(el, ctx);
      const valueExpr = resolveDataInput(el, "value", ctx);
      const rhs = valueExpr || "/* TODO: wire a value */";
      return `${varName} = ${rhs};`;
    }

    case "runCodeBlock": {
      // Look up the fn this references
      const fnName = (el.props.name as string) || resolveFnCallName(el, ctx);
      return `${fnName}();`;
    }

    case "updateUIProperty": {
      const targetId = el.config?.targetId as string || "";
      const property = el.config?.property as string || "label";
      const valueExpr = resolveDataInput(el, "value", ctx);
      const rhs = valueExpr || '""';
      // Find the target component name
      const targetEl = ctx.viewElements.get(targetId);
      const targetTag = targetEl?.tag || "Text";
      return `// updateUIProperty: ${targetTag}.${property} = ${rhs};`;
    }

    case "action": {
      const fnName = (el.props.name as string) || "handleAction";
      return `${fnName}();`;
    }

    default:
      return "";
  }
}

/** Resolve the variable name that setVariable targets. */
function resolveVariableName(el: ElementData, ctx: SourceGenContext): string {
  // 1. Check config
  if (el.config?.variableName) return el.config.variableName as string;
  if (el.props.name) return el.props.name as string;

  // 2. Look at data input "value" — if it comes from a getVariable, use its name
  const valueWire = ctx.dataInputs.get(`${el.id}:value`);
  if (valueWire) {
    const fromEl = ctx.elements.get(valueWire.fromId);
    if (fromEl?.tag === "getVariable") {
      return (fromEl.props.name as string) || "x";
    }
  }

  return "x";
}

/** Resolve the fn name that runCodeBlock calls. */
function resolveFnCallName(el: ElementData, ctx: SourceGenContext): string {
  if (el.props.name) return el.props.name as string;
  if (el.config?.fnName) return el.config.fnName as string;

  // Find fn-call or action node connected via flow or data
  const outs = ctx.outWires.get(el.id) || [];
  for (const w of outs) {
    const toEl = ctx.elements.get(w.toId);
    if (toEl?.tag === "fn-call" || toEl?.tag === "action") {
      return (toEl.props.name as string) || "handler";
    }
  }

  return "handler";
}

// ═══════════════════════════════════════════════════
// Expression resolution (data nodes)
// ═══════════════════════════════════════════════════

/**
 * Resolve the value connected to a data input port of a node.
 * Recursively evaluates data nodes to produce a .omni expression string.
 */
function resolveDataInput(
  el: ElementData,
  portName: string,
  ctx: SourceGenContext,
): string | null {
  const wire = ctx.dataInputs.get(`${el.id}:${portName}`);
  if (!wire) return null;

  const fromEl = ctx.elements.get(wire.fromId);
  if (!fromEl) return null;

  return resolveExpression(fromEl, ctx);
}

/** Recursively resolve an expression from a node's output. */
function resolveExpression(el: ElementData, ctx: SourceGenContext): string {
  switch (el.tag) {
    // ── Literals ──
    case "literal": {
      const val = el.config?.value;
      if (val === undefined || val === null) return "0";
      if (typeof val === "string") {
        if (el.config?.type === "String") return `"${val}"`;
        if (el.config?.type === "bool") return val;
        return val;
      }
      return String(val);
    }

    // ── Variable access ──
    case "getVariable": {
      const name = (el.props.name as string) || "x";
      return name;
    }

    case "signal": {
      // Signal block: resolve its output value
      const name = (el.props.name as string) || el.id;
      return name;
    }

    case "computed": {
      const name = (el.props.name as string) || "total";
      return `${name}()`;
    }

    // ── Arithmetic ──
    case "add": return `(${resolveDataInput(el, "a", ctx) || "0"} + ${resolveDataInput(el, "b", ctx) || "0"})`;
    case "sub": return `(${resolveDataInput(el, "a", ctx) || "0"} - ${resolveDataInput(el, "b", ctx) || "0"})`;
    case "mul": return `(${resolveDataInput(el, "a", ctx) || "1"} * ${resolveDataInput(el, "b", ctx) || "1"})`;
    case "div": return `(${resolveDataInput(el, "a", ctx) || "1"} / ${resolveDataInput(el, "b", ctx) || "1"})`;

    // ── Comparison ──
    case "compare": {
      const op = (el.config?.op as string) || "==";
      return `(${resolveDataInput(el, "a", ctx) || "0"} ${op} ${resolveDataInput(el, "b", ctx) || "0"})`;
    }

    // ── Logic ──
    case "and": return `(${resolveDataInput(el, "a", ctx) || "false"} && ${resolveDataInput(el, "b", ctx) || "false"})`;
    case "or": return `(${resolveDataInput(el, "a", ctx) || "false"} || ${resolveDataInput(el, "b", ctx) || "false"})`;
    case "not": return `(!${resolveDataInput(el, "a", ctx) || "false"})`;

    // ── String ──
    case "concat": return `(${resolveDataInput(el, "a", ctx) || '""'} + ${resolveDataInput(el, "b", ctx) || '""'})`;

    // ── Fallback for code blocks ──
    default: {
      // If it's a code block with a name, use that name
      const pName = (el.props as Record<string, unknown>)?.name;
      if (typeof pName === "string") return pName;
      return `/* ${el.tag}:${el.id.slice(0, 8)} */`;
    }
  }
}

// ═══════════════════════════════════════════════════
// Flow control: Branch
// ═══════════════════════════════════════════════════

function generateBranch(
  el: ElementData,
  ctx: SourceGenContext,
  indent: string,
  visited: Set<string>,
): string[] {
  const lines: string[] = [];
  const condition = resolveDataInput(el, "condition", ctx) || "false";

  lines.push(`${indent}if (${condition}) {`);

  // Follow "true" flow-out port
  const outs = ctx.outWires.get(el.id) || [];
  const trueWire = outs.find((w) => w.fromPort === "true");
  const falseWire = outs.find((w) => w.fromPort === "false");

  if (trueWire) {
    const toEl = ctx.elements.get(trueWire.toId);
    if (toEl && !visited.has(toEl.id)) {
      visited.add(toEl.id);
      lines.push(...traceFlowChain(toEl, ctx, indent + "  ", visited));
    }
  } else {
    lines.push(`${indent}  // (true branch: connect nodes here)`);
  }

  if (falseWire) {
    lines.push(`${indent}} else {`);
    const toEl = ctx.elements.get(falseWire.toId);
    if (toEl && !visited.has(toEl.id)) {
      visited.add(toEl.id);
      lines.push(...traceFlowChain(toEl, ctx, indent + "  ", visited));
    } else {
      lines.push(`${indent}  // (false branch: connect nodes here)`);
    }
  }

  lines.push(`${indent}}`);

  // Continue after branch (flow-out not connected to true/false)
  for (const w of outs) {
    if (w.fromPort === "true" || w.fromPort === "false") continue;
    const toEl = ctx.elements.get(w.toId);
    if (toEl && !visited.has(toEl.id)) {
      const toPort = toEl.ports?.find((p) => p.name === w.toPort);
      if (toPort?.portType === "flow") {
        visited.add(toEl.id);
        lines.push(...traceFlowChain(toEl, ctx, indent, visited));
      }
    }
  }

  return lines;
}

// ═══════════════════════════════════════════════════
// Flow control: Sequence
// ═══════════════════════════════════════════════════

function generateSequence(
  el: ElementData,
  ctx: SourceGenContext,
  indent: string,
  visited: Set<string>,
): string[] {
  const lines: string[] = [];
  const outs = ctx.outWires.get(el.id) || [];

  // Follow numbered flow-out ports in order: flowOut0, flowOut1, ...
  const sortedOuts = [...outs]
    .filter((w) => w.fromPort.startsWith("flowOut"))
    .sort((a, b) => a.fromPort.localeCompare(b.fromPort));

  for (const w of sortedOuts) {
    const toEl = ctx.elements.get(w.toId);
    if (!toEl || visited.has(toEl.id)) continue;
    visited.add(toEl.id);
    lines.push(...traceFlowChain(toEl, ctx, indent, visited));
  }

  return lines;
}

// ═══════════════════════════════════════════════════
// Flow control: ForEach
// ═══════════════════════════════════════════════════

function generateForEach(
  el: ElementData,
  ctx: SourceGenContext,
  indent: string,
  visited: Set<string>,
): string[] {
  const lines: string[] = [];
  const collection = resolveDataInput(el, "collection", ctx) || "items";
  const itemName = "item";

  lines.push(`${indent}for (${itemName} in ${collection}) {`);

  // Follow "loopBody" flow-out port
  const outs = ctx.outWires.get(el.id) || [];
  const loopWire = outs.find((w) => w.fromPort === "loopBody");
  const doneWire = outs.find((w) => w.fromPort === "completed");

  if (loopWire) {
    const toEl = ctx.elements.get(loopWire.toId);
    if (toEl && !visited.has(toEl.id)) {
      visited.add(toEl.id);
      lines.push(...traceFlowChain(toEl, ctx, indent + "  ", visited));
    }
  } else {
    lines.push(`${indent}  // (loop body: connect nodes here)`);
  }

  lines.push(`${indent}}`);

  // Follow "completed" flow-out
  if (doneWire) {
    const toEl = ctx.elements.get(doneWire.toId);
    if (toEl && !visited.has(toEl.id)) {
      visited.add(toEl.id);
      lines.push(...traceFlowChain(toEl, ctx, indent, visited));
    }
  }

  return lines;
}

// ═══════════════════════════════════════════════════
// View tree generation (preserved from original)
// ═══════════════════════════════════════════════════

/** Recursively build view element XML from the visual tree. */
function buildViewTree(
  el: ElementData,
  ctx: SourceGenContext,
  indent: string,
): string[] {
  const lines: string[] = [];
  const tag = el.tag;

  const props = buildProps(el);
  const children = [...ctx.viewElements.values()].filter((e) => e.parentId === el.id);

  if (children.length === 0 && isSelfClosing(tag)) {
    lines.push(`${indent}<${tag}${props} />`);
  } else if (children.length === 0) {
    const text = el.props.label || el.props.text || "";
    lines.push(`${indent}<${tag}${props}>${typeof text === "string" ? text : ""}</${tag}>`);
  } else {
    lines.push(`${indent}<${tag}${props}>`);
    for (const child of children) {
      lines.push(...buildViewTree(child, ctx, indent + "  "));
    }
    lines.push(`${indent}</${tag}>`);
  }

  return lines;
}

function buildProps(el: ElementData): string {
  const parts: string[] = [];
  const p = el.props as Record<string, unknown>;
  for (const [k, v] of Object.entries(p)) {
    // Skip internal keys
    if (k === "children" || k === "parentStore" || k === "typeAnnotation") continue;
    if (k.startsWith("_event_")) {
      // Event binding: _event_onPress → onPress={fnName}
      const eventName = k.replace("_event_", "");
      parts.push(`${eventName}={${v}}`);
      continue;
    }
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
