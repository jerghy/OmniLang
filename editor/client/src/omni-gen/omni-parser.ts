// editor/client/src/omni-gen/omni-parser.ts
import type { ElementData, WireData } from "../store/editor";
import type { CodeTab } from "../codegraph/types";
import type { ParseResult } from "./types";

type ParserState =
  | "ROOT"
  | "IN_COMPONENT"
  | "IN_STORE"
  | "IN_VIEW"
  | "IN_CHAIN"
  | "IN_FN"
  | "IN_STYLE";

interface ParseContext {
  state: ParserState;
  stateStack: ParserState[];
  elements: ElementData[];
  wires: WireData[];
  codeTabs: CodeTab[];
  imports: string[];
  errors: ParseResult["errors"];
  codeTabType: "logic" | "event";
  currentComponent: string;
  currentStore: string;
  chainName: string;
  chainElements: ElementData[];
  chainWires: WireData[];
  viewParentStack: string[];
  braceDepth: number;
  pendingEditFlags: string | null;
}

export function parseOmniSource(source: string): ParseResult {
  const ctx: ParseContext = {
    state: "ROOT",
    stateStack: [],
    elements: [],
    wires: [],
    codeTabs: [{ id: "main", type: "logic", label: "Main", icon: "📄", graph: { elements: [], wires: [] }, variables: [] }],
    imports: [],
    errors: [],
    codeTabType: "logic",
    currentComponent: "",
    currentStore: "",
    chainName: "",
    chainElements: [],
    chainWires: [],
    viewParentStack: [],
    braceDepth: 0,
    pendingEditFlags: null,
  };

  const lines = source.split("\n");

  for (const rawLine of lines) {
    const line = rawLine.trimEnd();
    const trimmed = line.trim();

    if (trimmed === "") continue;

    switch (ctx.state) {
      case "ROOT":
        parseRootLine(trimmed, ctx);
        break;
      case "IN_COMPONENT":
        parseComponentLine(trimmed, line, ctx);
        break;
      case "IN_VIEW":
        parseViewLine(trimmed, line, ctx);
        break;
      case "IN_FN":
        parseFnBodyLine(trimmed, ctx);
        break;
      case "IN_STYLE":
        parseStyleBlockLine(trimmed, ctx);
        break;
      case "IN_CHAIN":
        parseChainLine(trimmed, ctx);
        break;
    }
  }

  // Flush any remaining chain elements
  flushChain(ctx);

  return {
    designElements: ctx.elements.length > 0 ? ctx.elements : null,
    designWires: ctx.wires,
    codeTabs: ctx.codeTabs,
    imports: ctx.imports,
    errors: ctx.errors,
  };
}

// ── ROOT state handlers ──

function parseRootLine(trimmed: string, ctx: ParseContext): void {
  if (trimmed.startsWith("// @code-tab type=")) {
    const m = trimmed.match(/type=(logic|event)/);
    if (m) ctx.codeTabType = m[1] as "logic" | "event";
    return;
  }
  if (trimmed.startsWith("import ")) {
    ctx.imports.push(trimmed);
    return;
  }
  if (trimmed.startsWith("component ")) {
    const m = trimmed.match(/^component\s+(\w+)/);
    if (m) {
      ctx.currentComponent = m[1];
      ctx.braceDepth = 1;
      pushState(ctx, "IN_COMPONENT");
    }
    return;
  }
  if (trimmed.startsWith("store ")) {
    const m = trimmed.match(/^store\s+(\w+)/);
    if (m) {
      ctx.currentStore = m[1];
      ctx.braceDepth = 1;
      pushState(ctx, "IN_STORE");
    }
    return;
  }
}

// ── IN_COMPONENT state handlers ──

function parseComponentLine(trimmed: string, line: string, ctx: ParseContext): void {
  // Track brace depth
  if (trimmed === "}") {
    ctx.braceDepth--;
    if (ctx.braceDepth <= 0) {
      popState(ctx);
      return;
    }
  }
  if (trimmed.endsWith("{")) ctx.braceDepth++;

  // @store
  if (trimmed.startsWith("@store ")) {
    const m = trimmed.match(/@store\s+(\w+)/);
    if (m) ctx.elements.push(codeblockEl(`${ctx.currentComponent}.store.${m[1]}`, "store", { name: m[1] }));
    return;
  }

  // signal
  if (trimmed.startsWith("signal ")) {
    const m = trimmed.match(/signal\s+(\w+):\s*(\w+)\s*=\s*(.+);/);
    if (m) {
      ctx.elements.push(codeblockEl(`${ctx.currentComponent}.signal.${m[1]}`, "signal", {
        name: m[1], typeAnnotation: m[2],
      }));
    }
    return;
  }

  // computed
  if (trimmed.startsWith("computed ")) {
    const m = trimmed.match(/computed\s+(\w+):\s*(\w+)\s*=\s*(.+);/);
    if (m) {
      ctx.elements.push(codeblockEl(`${ctx.currentComponent}.computed.${m[1]}`, "computed", {
        name: m[1], typeAnnotation: m[2],
      }));
    }
    return;
  }

  // fn
  if (trimmed.startsWith("fn ")) {
    pushState(ctx, "IN_FN");
    return;
  }

  // view
  if (trimmed === "view {") {
    pushState(ctx, "IN_VIEW");
    ctx.viewParentStack = [];
    return;
  }

  // style
  if (trimmed === "style {") {
    pushState(ctx, "IN_STYLE");
    return;
  }

  // @chain
  if (trimmed.startsWith("// @chain ")) {
    const m = trimmed.match(/\/\/\s*@chain\s+(\w+)/);
    if (m) {
      ctx.chainName = m[1];
      ctx.chainElements = [];
      ctx.chainWires = [];
      pushState(ctx, "IN_CHAIN");
    }
    return;
  }
}

// ── IN_VIEW state handlers ──

function parseViewLine(trimmed: string, line: string, ctx: ParseContext): void {
  if (trimmed === "}") {
    popState(ctx);
    return;
  }

  // @edit annotation
  if (trimmed.startsWith("// @edit")) {
    ctx.pendingEditFlags = trimmed.slice("// @edit".length).trim() || null;
    return;
  }

  // Opening tag
  if (trimmed.startsWith("<") && !trimmed.startsWith("</")) {
    const indent = line.length - line.trimStart().length;
    const el = parseElementLine(trimmed, ctx);

    // Parent inference by indent
    while (ctx.viewParentStack.length > 0) {
      const parentId = ctx.viewParentStack[ctx.viewParentStack.length - 1];
      const parent = ctx.elements.find(e => e.id === parentId);
      if (parent && (parent as any)._viewIndent !== undefined && (parent as any)._viewIndent >= indent) {
        ctx.viewParentStack.pop();
      } else {
        break;
      }
    }

    const parentId = ctx.viewParentStack.length > 0 ? ctx.viewParentStack[ctx.viewParentStack.length - 1] : null;
    el.parentId = parentId;
    (el as any)._viewIndent = indent;

    if (parentId) {
      const parent = ctx.elements.find(e => e.id === parentId);
      if (parent) parent.children.push(el.id);
    }

    // @edit flags
    if (ctx.pendingEditFlags) {
      if (ctx.pendingEditFlags.includes("folded")) el.isFolded = true;
      if (ctx.pendingEditFlags.includes("render=canvas")) el.renderMode = "canvas";
      ctx.pendingEditFlags = null;
    }

    ctx.elements.push(el);

    // Not self-closing → push to parent stack
    if (!trimmed.endsWith("/>")) {
      ctx.viewParentStack.push(el.id);
    }
    return;
  }

  // Closing tag
  if (trimmed.startsWith("</")) {
    if (ctx.viewParentStack.length > 0) ctx.viewParentStack.pop();
    return;
  }
}

function parseElementLine(trimmed: string, ctx: ParseContext): ElementData {
  const tagMatch = trimmed.match(/^<(\w+)/);
  const tag = tagMatch ? tagMatch[1] : "div";

  const idMatch = trimmed.match(/id="([^"]*)"/);
  const id = idMatch ? idMatch[1] : `${ctx.currentComponent}.${tag}.${ctx.elements.length}`;

  const classMatch = trimmed.match(/class:\s*(\w+)/);
  const className = classMatch ? classMatch[1] : "";

  const events: Record<string, string> = {};
  const eventRegex = /@(\w+)={(\w+)}/g;
  let evtMatch;
  while ((evtMatch = eventRegex.exec(trimmed)) !== null) {
    events[evtMatch[1]] = evtMatch[2];
  }

  const attrs: Record<string, string> = {};
  const attrRegex = /\b(\w+)="([^"]*)"/g;
  let attrMatch;
  while ((attrMatch = attrRegex.exec(trimmed)) !== null) {
    if (attrMatch[1] !== "id" && attrMatch[1] !== "class") {
      attrs[attrMatch[1]] = attrMatch[2];
    }
  }

  const textMatch = trimmed.match(/>([^<]*)<\//);
  const textContent = textMatch ? textMatch[1].trim() : "";

  return {
    id,
    nodeType: "view",
    tag,
    props: { ...attrs, ...events, className, ...(textContent ? { label: textContent } : {}) },
    children: [],
    parentId: null,
  };
}

// ── IN_FN — skip function body ──

function parseFnBodyLine(trimmed: string, ctx: ParseContext): void {
  if (trimmed === "{" || trimmed.endsWith("{")) ctx.braceDepth++;
  if (trimmed === "}") {
    ctx.braceDepth--;
    if (ctx.braceDepth <= 0) popState(ctx);
  }
}

// ── IN_STYLE — skip style block ──

function parseStyleBlockLine(trimmed: string, ctx: ParseContext): void {
  if (trimmed === "{") ctx.braceDepth++;
  if (trimmed === "}") {
    ctx.braceDepth--;
    if (ctx.braceDepth <= 0) popState(ctx);
  }
}

// ── IN_CHAIN — parse @node / @wire ──

function parseChainLine(trimmed: string, ctx: ParseContext): void {
  if (trimmed.startsWith("// ═")) {
    flushChain(ctx);
    popState(ctx);
    return;
  }

  if (trimmed.startsWith("//   @node ")) {
    const parts = trimmed.slice(3).trim().split(/\s+/); // skip "//"
    const id = parts[1];
    const tag = parts[2];
    const attrs: Record<string, string> = {};
    let posX = 0, posY = 0;
    for (let i = 3; i < parts.length; i++) {
      if (parts[i].startsWith("pos=")) {
        const coords = parts[i].slice(4).split(",");
        posX = parseInt(coords[0]) || 0;
        posY = parseInt(coords[1]) || 0;
      } else if (parts[i].includes("=")) {
        const [k, v] = parts[i].split("=");
        attrs[k] = v;
      }
    }
    ctx.chainElements.push({
      id, nodeType: "codeblock", tag,
      props: attrs,
      position: { x: posX, y: posY },
      children: [], parentId: null,
    });
    return;
  }

  if (trimmed.startsWith("//   @wire ")) {
    const wireMatch = trimmed.match(/@wire\s+(\S+)\s+(\S+)\s*->\s*(\S+)/);
    if (wireMatch) {
      const [, wireId, from, to] = wireMatch;
      const [fromId, fromPort = ""] = from.includes(".") ? from.split(".") : [from];
      const [toId, toPort = ""] = to.includes(".") ? to.split(".") : [to];
      ctx.chainWires.push({ id: wireId, fromId, fromPort, toId, toPort });
    }
  }
}

function flushChain(ctx: ParseContext): void {
  if (ctx.chainElements.length === 0) return;
  const tab = ctx.codeTabs.find(t => t.id === "main") || ctx.codeTabs[0];
  for (const el of ctx.chainElements) tab.graph.elements.push(el);
  for (const w of ctx.chainWires) tab.graph.wires.push(w);
  ctx.chainElements = [];
  ctx.chainWires = [];
}

// ── Helpers ──

function pushState(ctx: ParseContext, state: ParserState): void {
  ctx.stateStack.push(ctx.state);
  ctx.state = state;
  ctx.braceDepth = 1;
}

function popState(ctx: ParseContext): void {
  ctx.state = ctx.stateStack.pop() || "ROOT";
}

function codeblockEl(id: string, tag: string, props: Record<string, unknown>): ElementData {
  return {
    id, nodeType: "codeblock", tag, props: props as Record<string, unknown>,
    children: [], parentId: null,
  };
}
