// editor/client/src/omni-gen/omni-parser.ts
import type { ElementData, WireData, CodeTab } from "../../core/types";
import type { ParseResult } from "./types";
import { parseStylesFromAST } from "./pipeline";
import { useComponentRegistry } from "../../core/state/componentRegistry";

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
  braceDepthStack: number[];
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
  parsedStyleRules: Array<{ class_name: string; properties: Record<string, string> }>;
  /** Whether a style { } block was encountered in source. When false, parsedStyleRules
   *  being empty means no style block existed (backward compat), not an empty block. */
  hasStyleBlock: boolean;
  /** Parsed prop declarations (e.g. `prop title: String = ""`) */
  props: Array<{ name: string; type: string; default?: unknown }>;
  /** Parsed event declarations (e.g. `event onAction`) */
  events: Array<{ name: string }>;
  /** Parsed component metadata from // @component, // @prop, // @event, // @style comments */
  componentMeta: Record<string, unknown> | null;
}

export function parseOmniSource(source: string): ParseResult {
  const lines = source.split("\n");

  const ctx: ParseContext = {
    state: "ROOT",
    stateStack: [],
    braceDepthStack: [],
    elements: [],
    wires: [],
    codeTabs: [{ id: "main", type: "logic", label: "Main", icon: "📄", graph: { elements: [], wires: [] }, variables: [] }],
    imports: [],
    errors: [],
    codeTabType: "logic",
    currentComponent: "",
    currentStore: "",
    parsedStyleRules: [],
    chainName: "",
    chainElements: [],
    chainWires: [],
    viewParentStack: [],
    braceDepth: 0,
    pendingEditFlags: null,
    props: [],
    events: [],
    componentMeta: null,
    hasStyleBlock: false,
  };


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
      case "IN_STORE":
        parseStoreLine(trimmed, ctx);
        break;
      case "IN_STYLE":
        parseStyleBlockLine(trimmed, ctx);
        break;
      case "IN_CHAIN":
        parseChainLine(trimmed, ctx);
        break;
      default:
        ctx.errors.push({ severity: "warning", code: "P099", message: `Unhandled parser state: ${ctx.state}`, line: 0, col: 0, end_line: 0, end_col: 0 });
        break;
    }
  }

  // Flush any remaining chain elements
  flushChain(ctx);

  // Parse styles from the captured AST (style blocks + view tree).
  // Always run when elements exist — handles:
  // - style { } blocks → class/variable StyleClass
  // - inline style: { } attributes → inline ElementStyle
  // - backward compat: no style block → extract style keys from element props
  if (ctx.elements.length > 0) {
    // Build a view tree node from element parent/child relationships
    function buildViewTreeFromElements(el: ElementData): any {
      // Parse style prop string into inline_style object
      let inline_style: Record<string, string> | null = null;
      const styleStr = el.props.style as string | undefined;
      if (styleStr && typeof styleStr === "string") {
        // Remove surrounding braces, split on ";", extract key:value pairs
        const cleaned = styleStr.replace(/^\s*\{\s*/, "").replace(/\s*\}\s*$/, "");
        if (cleaned.length > 0) {
          const pairs: Record<string, string> = {};
          for (const part of cleaned.split(";")) {
            const trimmed = part.trim();
            if (trimmed.length === 0) continue;
            const colonIdx = trimmed.indexOf(":");
            if (colonIdx > 0) {
              pairs[trimmed.slice(0, colonIdx).trim()] = trimmed.slice(colonIdx + 1).trim();
            }
          }
          if (Object.keys(pairs).length > 0) {
            inline_style = pairs;
          }
        }
      }

      return {
        id: el.id,
        props: { ...el.props, class: (el.props.className as string) || "" },
        inline_style,
        children: (el.children || [])
          .map((cid: string) => ctx.elements.find((e: ElementData) => e.id === cid))
          .filter((c: ElementData | undefined): c is ElementData => c !== undefined)
          .map(buildViewTreeFromElements),
      };
    }
    const rootElements = ctx.elements.filter(
      (e: ElementData) => e.nodeType === "view" && !e.parentId,
    );
    if (rootElements.length > 0) {
      const elementMap = new Map(ctx.elements.map((e: ElementData) => [e.id, e]));
      const componentNode = {
        // Pass undefined when no style block existed (backward compat), empty array when it did
        style: ctx.hasStyleBlock ? ctx.parsedStyleRules : undefined,
        view: buildViewTreeFromElements(rootElements[0]),
      };
      parseStylesFromAST(componentNode, elementMap);
    }
  }

  return {
    designElements: ctx.elements.length > 0 ? ctx.elements : null,
    designWires: ctx.wires,
    codeTabs: ctx.codeTabs,
    imports: ctx.imports,
    errors: ctx.errors,
    componentMeta: ctx.componentMeta,
    props: ctx.props.length > 0 ? ctx.props : undefined,
    events: ctx.events.length > 0 ? ctx.events : undefined,
  };
}

// ── ROOT state handlers ──

const _COMPONENT_META_RE = /^\/\/\s*@component\s+(.+)/;
const _PROP_META_RE = /^\/\/\s*@prop\s+(.+)/;
const _EVENT_META_RE = /^\/\/\s*@event\s+(.+)/;
const _STYLE_META_RE = /^\/\/\s*@style\s+(.+)/;

function parseRootLine(trimmed: string, ctx: ParseContext): void {
  // Component metadata comments (replaces old *.meta.json)
  // @component id=... category=... children=... description=...
  if (trimmed.startsWith("// @component ")) {
    const m = trimmed.match(/^\/\/\s*@component\s+(.+)/);
    if (m) {
      const meta = parseKeyValueAttrs(m[1]);
      ctx.componentMeta = ctx.componentMeta || {};
      if (meta.id !== undefined) ctx.componentMeta.id = meta.id;
      if (meta.category !== undefined) ctx.componentMeta.category = meta.category;
      if (meta.children !== undefined) ctx.componentMeta.children = meta.children === "true";
      if (meta.description !== undefined) ctx.componentMeta.description = meta.description;
    }
    return;
  }
  // @prop name=... editor=... values=...
  if (trimmed.startsWith("// @prop ")) {
    const m = trimmed.match(/^\/\/\s*@prop\s+(.+)/);
    if (m) {
      const meta = parseKeyValueAttrs(m[1]);
      if (meta.name) {
        ctx.componentMeta = ctx.componentMeta || {};
        ctx.componentMeta.propSchema = ctx.componentMeta.propSchema || ({} as Record<string, unknown>);
        const schema: Record<string, unknown> = {};
        if (meta.editor !== undefined) schema.editor = meta.editor;
        if (meta.values !== undefined) schema.values = String(meta.values).split(",");
        (ctx.componentMeta.propSchema as Record<string, unknown>)[meta.name] = schema;
      }
    }
    return;
  }
  // @event name=... payload=... description=...
  if (trimmed.startsWith("// @event ")) {
    const m = trimmed.match(/^\/\/\s*@event\s+(.+)/);
    if (m) {
      const meta = parseKeyValueAttrs(m[1]);
      if (meta.name) {
        ctx.componentMeta = ctx.componentMeta || {};
        ctx.componentMeta.eventSchema = ctx.componentMeta.eventSchema || ({} as Record<string, unknown>);
        const schema: Record<string, unknown> = {};
        if (meta.payload !== undefined) schema.payload = meta.payload;
        if (meta.description !== undefined) schema.description = meta.description;
        (ctx.componentMeta.eventSchema as Record<string, unknown>)[meta.name] = schema;
      }
    }
    return;
  }
  // @style <name>
  if (trimmed.startsWith("// @style ")) {
    const name = trimmed.slice("// @style ".length).trim();
    if (name) {
      ctx.componentMeta = ctx.componentMeta || {};
      ctx.componentMeta.styleProperties = ctx.componentMeta.styleProperties || [];
      (ctx.componentMeta.styleProperties as string[]).push(name);
    }
    return;
  }

  if (trimmed.startsWith("// @omni-file type=") || trimmed.startsWith("// @code-tab type=")) {
    const m = trimmed.match(/@(?:omni-file|code-tab)\s+type=(logic|event)/);
    if (m) {
      ctx.codeTabType = m[1] as "logic" | "event";
      ctx.codeTabs[0].type = ctx.codeTabType;
    } else {
      ctx.errors.push({ severity: "warning", code: "P001", message: `Invalid @omni-file type value in: "${trimmed}"`, line: 0, col: 0, end_line: 0, end_col: 0 });
    }
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
    } else {
      ctx.errors.push({ severity: "error", code: "P002", message: `Invalid component declaration: "${trimmed}"`, line: 0, col: 0, end_line: 0, end_col: 0 });
    }
    return;
  }
  if (trimmed.startsWith("store ")) {
    const m = trimmed.match(/^store\s+(\w+)/);
    if (m) {
      ctx.currentStore = m[1];
      ctx.braceDepth = 1;
      pushState(ctx, "IN_STORE");
    } else {
      ctx.errors.push({ severity: "error", code: "P003", message: `Invalid store declaration: "${trimmed}"`, line: 0, col: 0, end_line: 0, end_col: 0 });
    }
    return;
  }
}

/**
 * Parse key=value pairs from a string.
 * Supports: id=abc123, category=Custom, children=true, description="quoted string"
 */
function parseKeyValueAttrs(input: string): Record<string, string> {
  const result: Record<string, string> = {};
  const regex = /(\w+)=(?:"([^"]*)"|(\S+))/g;
  let match;
  while ((match = regex.exec(input)) !== null) {
    result[match[1]] = match[2] !== undefined ? match[2] : match[3];
  }
  return result;
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

  // prop (e.g. `prop title: String = ""`)
  if (trimmed.startsWith("prop ")) {
    const m = trimmed.match(/prop\s+(\w+):\s*(\w+)(?:\s*=\s*(.+?))?\s*;/);
    if (m) {
      ctx.props.push({ name: m[1], type: m[2], default: m[3]?.replace(/^"|"$/g, "") || undefined });
    }
    return;
  }

  // event (e.g. `event onAction`)
  if (trimmed.startsWith("event ")) {
    const m = trimmed.match(/event\s+(\w+);/);
    if (m) {
      ctx.events.push({ name: m[1] });
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
    ctx.hasStyleBlock = true;
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

    // Auto-rename duplicate IDs to avoid parent-child tree corruption
    if (ctx.elements.some((e) => e.id === el.id)) {
      console.warn(`[omni-parser] Duplicate element ID "${el.id}" at line — "${trimmed}"; assigning unique ID`);
      let counter = 1;
      let newId = `${el.id}__dup${counter}`;
      while (ctx.elements.some((e) => e.id === newId)) {
        counter++;
        newId = `${el.id}__dup${counter}`;
      }
      el.id = newId;
    }

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

  // Text content between tags (e.g., "Hello" between <p> and </p>)
  if (trimmed && !trimmed.startsWith("//")) {
    const text = trimmed.trim();
    if (text && !text.startsWith("<")) {
      // Attach as label prop to the current parent element
      const parentId = ctx.viewParentStack.length > 0
        ? ctx.viewParentStack[ctx.viewParentStack.length - 1]
        : null;
      if (parentId) {
        const parent = ctx.elements.find(e => e.id === parentId);
        if (parent && !parent.props.label) {
          parent.props.label = text;
        }
      }
    }
  }
}

function parseElementLine(trimmed: string, ctx: ParseContext): ElementData {
  const tagMatch = trimmed.match(/^<(\w+)/);
  const tag = tagMatch ? tagMatch[1] : "div";

  const idMatch = trimmed.match(/id="([^"]*)"/);
  const id = idMatch ? idMatch[1] : `${ctx.currentComponent}.${tag}.${ctx.elements.length}`;

  // Captures one or more [\w-] class names after "class:", separated by whitespace.
  // Lookahead terminates on:
  //   - whitespace + known attribute prefix (key=, @, bind:, style:, >, //)
  //   - end-of-string
  //   - /> or > immediately (no trailing space, e.g. <div class: foo>)
  const classMatch = trimmed.match(/class:\s*([\w-]+(?:\s+[\w-]+)*?)(?=\s+(?:[\w-]+\s*=|@|bind:|style:|>|\/{2})|$|\/>|>)/);
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

  // Parse style: { key: val, ... } attribute — store as "style" prop so
  // buildViewTreeFromElements can reconstruct inline_style for parseStylesFromAST.
  const styleAttrMatch = trimmed.match(/style:\s*\{([^}]*)\}/);
  if (styleAttrMatch) {
    const styleContent = styleAttrMatch[1].trim();
    // Rebuild as brace-wrapped, semicolon-separated string that buildViewTreeFromElements expects.
    // Strip surrounding double-quotes from values (source-emitter wraps in "")
    // e.g. 'padding: "16px", color: "#333"' → '{ padding: 16px; color: #333 }'
    const semiStyle = styleContent
      .split(",")
      .map((part) => part.trim())
      .filter(Boolean)
      .map((part) => part.replace(/:\s*"([^"]*)"/g, ': $1'))
      .join("; ");
    attrs["style"] = `{ ${semiStyle} }`;
  }

  const textMatch = trimmed.match(/>([^<]*)<\//);
  const textContent = textMatch ? textMatch[1].trim() : "";

  // Detect custom component references: PascalCase tags that match a registry
  // definition get componentRef set so the canvas can expand them and the
  // generator can emit proper import statements.
  const isPascal = /^[A-Z]/.test(tag);
  const isCustom = isPascal && !!useComponentRegistry.getState().definitions[tag];

  return {
    id,
    nodeType: "view",
    tag,
    props: { ...attrs, ...events, className, ...(textContent ? { label: textContent } : {}) },
    children: [],
    parentId: null,
    ...(isCustom ? { componentRef: tag } : {}),
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

// ── IN_STORE — skip store body (tracks braces only) ──

function parseStoreLine(trimmed: string, ctx: ParseContext): void {
  if (trimmed === "{" || trimmed.endsWith("{")) ctx.braceDepth++;
  if (trimmed === "}") {
    ctx.braceDepth--;
    if (ctx.braceDepth <= 0) popState(ctx);
  }
}

// ── IN_STYLE — parse style block ──

function parseStyleBlockLine(trimmed: string, ctx: ParseContext): void {
  // Track brace depth for the style block itself
  if (trimmed === "{" || trimmed.endsWith("{")) {
    ctx.braceDepth++;
    return;
  }
  if (trimmed === "}") {
    ctx.braceDepth--;
    if (ctx.braceDepth <= 0) {
      popState(ctx);
    }
    return;
  }

  // Parse CSS rule: ".class-name { prop: value; prop2: value2; }"
  const ruleMatch = trimmed.match(/^\.([\w-]+)\s*\{\s*(.*?)\s*\}$/);
  if (ruleMatch) {
    const className = ruleMatch[1];
    const propsStr = ruleMatch[2];
    const properties: Record<string, string> = {};
    for (const pair of propsStr.split(";").filter(s => s.trim().length > 0)) {
      const colonIdx = pair.indexOf(":");
      if (colonIdx > 0) {
        properties[pair.slice(0, colonIdx).trim()] = pair.slice(colonIdx + 1).trim();
      }
    }
    ctx.parsedStyleRules.push({ class_name: className, properties });
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
  ctx.braceDepthStack.push(ctx.braceDepth);
  ctx.state = state;
  ctx.braceDepth = 1;
}

function popState(ctx: ParseContext): void {
  ctx.state = ctx.stateStack.pop() || "ROOT";
  ctx.braceDepth = ctx.braceDepthStack.pop() || 0;
}

function codeblockEl(id: string, tag: string, props: Record<string, unknown>): ElementData {
  return {
    id, nodeType: "codeblock", tag, props: props as Record<string, unknown>,
    children: [], parentId: null,
  };
}
