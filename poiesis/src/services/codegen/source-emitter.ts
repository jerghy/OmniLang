// poiesis/src/services/codegen/source-emitter.ts
// CodeWriter helper + SourceEmitter for view and style generation.

import type { EmitInput, AnnotationStrategy, ChainBlock } from "./types";
import type { ViewNode, ElementViewNode } from "./types";
import type { StoreDefinition } from "../../core/types/store";
import { compileCodeFlowGraph } from "./graph-compiler";
import { emitPidAnnotation } from "./annotation-registry";

// ── Value formatting ────────────────────────────────────────────────────────

/**
 * Format a default value for emission into .omni source.
 *
 * OmniLang Strings require quote-delimited literals ("hello" or 'hello').
 * Other types (i32, bool, Array<...>, struct literals) are emitted as-is.
 *
 * This handles values from BOTH sources:
 * - Parser (already quoted for strings: `"all"`)
 * - UI editing via StoreMembersPanel (raw text without quotes: `all`)
 */
export function formatDefaultValue(value: unknown, type: string): string {
  if (value === undefined || value === null) return "";
  const str = String(value);
  if (str === "") return "";

  // Only add quotes for String-typed defaults
  const isStringType = type === "String" || type === "String?";
  if (isStringType) {
    // Don't double-quote values that already have quotes (from parser)
    if ((str.startsWith('"') && str.endsWith('"')) ||
        (str.startsWith("'") && str.endsWith("'"))) {
      return str;
    }
    return `"${str}"`;
  }

  // Numbers, booleans, arrays, struct literals, etc. — emit as-is
  return str;
}

export class CodeWriter {
  private lines: string[] = [];
  private _indent = 0;
  private readonly indentStep = "    "; // 4 spaces

  emit(line: string): void {
    this.lines.push(this.indentStep.repeat(this._indent) + line);
  }

  indent(): void { this._indent++; }
  dedent(): void { this._indent = Math.max(0, this._indent - 1); }
  blank(): void { this.lines.push(""); }
  toString(): string { return this.lines.join("\n") + "\n"; }
}

/**
 * Emit a StoreDefinition as standalone OmniLang store source.
 * Produces a complete `store Name { ... }` block with @pid annotations,
 * signal/getter/action declarations, and compiled action code-flow bodies.
 */
export function emitStoreDefinition(store: StoreDefinition): string {
  const w = new CodeWriter();

  // Stores are view files — always emit import omniview;
  const hasOmniViewImport = store.imports.some((imp) => imp.includes("omniview"));
  if (!hasOmniViewImport) {
    w.emit("import omniview;");
    if (store.imports.length > 0) w.blank();
  }

  // Imports
  for (const imp of store.imports) {
    w.emit(imp);
  }
  if (store.imports.length > 0) w.blank();

  // Store header
  w.emit(emitPidAnnotation(store.pid));
  w.emit(`store ${store.name} {`);
  w.indent();

  // Signals
  for (const sig of store.signals) {
    const defStr = sig.defaultValue !== undefined && sig.defaultValue !== ""
      ? ` = ${formatDefaultValue(sig.defaultValue, sig.type)}`
      : "";
    w.emit(emitPidAnnotation(sig.pid));
    w.emit(`signal ${sig.name}: ${sig.type}${defStr};`);
  }

  // Getters
  if (store.getters.length > 0 && store.signals.length > 0) w.blank();
  for (const g of store.getters) {
    w.emit(emitPidAnnotation(g.pid));
    w.emit(`getter ${g.name}: ${g.returnType} = ${g.expression};`);
  }

  // Actions
  for (const a of store.actions) {
    if (store.signals.length + store.getters.length > 0 || a !== store.actions[0]) w.blank();
    const paramsStr = a.params.map((p) => `${p.name}: ${p.type}`).join(", ");
    const retStr = a.returnType ? `: ${a.returnType}` : "";
    const asyncStr = a.isAsync ? " async" : "";
    w.emit(emitPidAnnotation(a.pid));
    w.emit(`action ${a.name}(${paramsStr})${retStr}${asyncStr} {`);

    // Emit action body via CodeFlow compiler
    if (a.elements.length > 0) {
      w.indent();
      const actionFnPid = a.elements.find((e) => e.tag === "fn")?.id;
      if (actionFnPid) {
        const body = compileCodeFlowGraph(a.elements, a.wires, actionFnPid);
        const lines = body.split("\n");
        // Skip first line (fn signature + @pid) and last line (closing })
        const innerLines = lines.slice(1, -1);
        let hasContent = false;
        for (const line of innerLines) {
          const trimmed = line.trim();
          if (trimmed.length > 0) {
            w.emit(trimmed);
            hasContent = true;
          }
        }
        // Ensure the fn block is properly closed even if compileCodeFlowGraph
        // produced incomplete output (e.g. fn element with no children).
        if (!hasContent) {
          // Emit an empty fn block to keep the syntax valid
          w.emit(`fn ${a.name}() {`);
          w.emit(`}`);
        }
      } else {
        // No fn element found — emit a minimal valid fn block
        w.emit(`fn ${a.name}() {`);
        w.emit(`}`);
      }
      w.dedent();
    }
    w.emit(`}`);
  }

  w.dedent();
  w.emit(`}`);
  return w.toString();
}

export class SourceEmitter {
  constructor(
    private annotationStrategy?: AnnotationStrategy,
    private elementClassMap?: Map<string, string[]>,
    private elementInlineMap?: Map<string, Record<string, string>>,
    private elementBindings?: Map<string, Record<string, string>>,
    private enableAnnotations: boolean = false,
  ) {}

  emitDocument(input: EmitInput, chains: Map<string, ChainBlock>): string {
    const writer = new CodeWriter();

    // @omni-file directive (only emitted for non-default types like "event")
    if (this.enableAnnotations && this.annotationStrategy) {
      const directive = this.annotationStrategy.beforeFile();
      if (directive) writer.emit(directive);
    }

    // imports
    if (input.imports.length > 0) {
      for (const imp of input.imports) writer.emit(imp);
      writer.blank();
    }

    // stores
    for (const store of input.stores) {
      this.emitStore(store, writer);
      writer.blank();
    }

    // component
    this.emitComponent(input, writer, chains);

    // themes
    for (const theme of (input.themes || [])) {
      writer.emit(`theme ${theme.name} {`);
      for (const [cat, entries] of Object.entries(theme.categories)) {
        writer.emit(`  ${cat} {`);
        for (const [k, v] of Object.entries(entries)) writer.emit(`    ${k}: "${v}";`);
        writer.emit(`  }`);
      }
      writer.emit(`}`);
    }

    return writer.toString();
  }

  private emitStore(store: StoreDefinition, w: CodeWriter): void {
    // Imports
    for (const imp of store.imports) {
      w.emit(imp);
    }
    if (store.imports.length > 0) w.blank();

    // Store header
    w.emit(emitPidAnnotation(store.pid));
    w.emit(`store ${store.name} {`);
    w.indent();

    // Signals
    for (const sig of store.signals) {
      const defStr = sig.defaultValue !== undefined && sig.defaultValue !== ""
        ? ` = ${formatDefaultValue(sig.defaultValue, sig.type)}`
        : "";
      w.emit(emitPidAnnotation(sig.pid));
      w.emit(`signal ${sig.name}: ${sig.type}${defStr};`);
    }

    // Getters
    if (store.getters.length > 0 && store.signals.length > 0) w.blank();
    for (const g of store.getters) {
      w.emit(emitPidAnnotation(g.pid));
      w.emit(`getter ${g.name}: ${g.returnType} = ${g.expression};`);
    }

    // Actions
    for (const a of store.actions) {
      if (store.signals.length + store.getters.length > 0 || a !== store.actions[0]) w.blank();
      const paramsStr = a.params.map((p) => `${p.name}: ${p.type}`).join(", ");
      const retStr = a.returnType ? `: ${a.returnType}` : "";
      const asyncStr = a.isAsync ? " async" : "";
      w.emit(emitPidAnnotation(a.pid));
      w.emit(`action ${a.name}(${paramsStr})${retStr}${asyncStr} {`);

      // Emit action body via CodeFlow compiler
      if (a.elements.length > 0) {
        w.indent();
        const actionFnPid = a.elements.find((e) => e.tag === "fn")?.id;
        if (actionFnPid) {
          const body = compileCodeFlowGraph(a.elements, a.wires, actionFnPid);
          const lines = body.split("\n");
          const innerLines = lines.slice(1, -1);
          let hasContent = false;
          for (const line of innerLines) {
            const trimmed = line.trim();
            if (trimmed.length > 0) {
              w.emit(trimmed);
              hasContent = true;
            }
          }
          if (!hasContent) {
            w.emit(`fn ${a.name}() {`);
            w.emit(`}`);
          }
        } else {
          w.emit(`fn ${a.name}() {`);
          w.emit(`}`);
        }
        w.dedent();
      }
      w.emit(`}`);
    }

    w.dedent();
    w.emit(`}`);
  }

  private emitComponent(input: EmitInput, w: CodeWriter, chains: Map<string, ChainBlock>): void {
    const comp = input.component;

    // Emit component metadata comments (replaces old *.meta.json)
    this.emitComponentMeta(input.componentMeta, w);

    // PascalCase: ensure first character is uppercase
    const pascalName = comp.name.charAt(0).toUpperCase() + comp.name.slice(1);
    w.emit(`component ${pascalName} {`);
    w.indent();

    for (const sr of comp.storeRefs) w.emit(`@store ${sr};`);

    // ── Inline stores ──
    for (const store of input.inlineStores ?? []) {
      this.emitInlineStore(store, w);
      w.blank();
    }

    // Props (e.g. `prop title: String = ""`)
    for (const prop of comp.props || []) {
      const def = prop.defaultValue !== undefined ? ` = ${formatDefaultValue(prop.defaultValue, prop.type)}` : "";
      w.emit(`prop ${prop.name}: ${prop.type}${def};`);
    }
    // Events (e.g. `event onAction`)
    for (const evt of comp.events || []) {
      w.emit(`event ${evt.name};`);
    }

    for (const [name, sig] of Object.entries(comp.signals)) {
      const def = sig.defaultValue !== undefined && sig.defaultValue !== ""
        ? ` = ${formatDefaultValue(sig.defaultValue, sig.type)}`
        : "";
      w.emit(`signal ${name}: ${sig.type}${def};`);
    }
    for (const [name, c] of Object.entries(comp.computeds || {})) {
      w.emit(`computed ${name}: ${c.type} = ${c.body};`);
    }

    // Constants (compile-time, e.g. `const MAX_COUNT = 100`)
    if (comp.consts) {
      for (const c of comp.consts) {
        const val = typeof c.value === "string"
          ? `"${c.value}"`
          : String(c.value ?? "");
        w.emit(`const ${c.name} = ${val};`);
      }
    }

    // Functions with chains
    for (const fn of comp.fns || []) {
      // Event handler boundary marker (for round-trip tab reconstruction)
      if (input.codeTabType === "event") {
        w.emit(`// @omni-file type=event label=${fn.name}`);
      }
      const chain = chains.get(fn.name);
      if (chain) w.emit(formatChainBlock(chain));
      const params = fn.params.map(p => `${p.name}: ${p.type}`).join(", ");
      w.emit(`fn ${fn.name}(${params}) -> ${fn.returnType} {`);
      w.indent();
      for (const line of fn.body.split("\n")) w.emit(line);
      w.dedent();
      w.emit(`}`);
      w.blank();
    }

    // View
    if (comp.view) {
      w.emit(`view {`);
      w.indent();
      this.emitViewNode(comp.view, w, 0);
      w.dedent();
      w.emit(`}`);
    }

    // Style — one property per line, matching hand-written .omni conventions
    const style = comp.style || {};
    if (Object.keys(style).length > 0) {
      w.emit(`style {`);
      w.indent();
      for (const [cls, rules] of Object.entries(style)) {
        if (Object.keys(rules).length === 0) continue;
        w.emit(`.${cls} {`);
        w.indent();
        for (const [prop, val] of Object.entries(rules)) {
          w.emit(`${prop}: ${val};`);
        }
        w.dedent();
        w.emit(`}`);
        w.blank();
      }
      w.dedent();
      w.emit(`}`);
    }

    w.dedent();
    w.emit(`}`);
  }

  private emitInlineStore(store: StoreDefinition, w: CodeWriter): void {
    w.emit(emitPidAnnotation(store.pid));
    // Inline store names are qualified as "ComponentName.StoreName" from the parser
    // but inside the component body we emit just the unqualified "StoreName".
    const unqualifiedName = store.name.includes(".")
      ? store.name.slice(store.name.lastIndexOf(".") + 1)
      : store.name;
    w.emit(`store ${unqualifiedName} {`);
    w.indent();

    // Signals
    for (const sig of store.signals) {
      const defStr = sig.defaultValue !== undefined && sig.defaultValue !== ""
        ? ` = ${formatDefaultValue(sig.defaultValue, sig.type)}`
        : "";
      w.emit(emitPidAnnotation(sig.pid));
      w.emit(`signal ${sig.name}: ${sig.type}${defStr};`);
    }

    // Getters
    if (store.getters.length > 0 && store.signals.length > 0) w.blank();
    for (const g of store.getters) {
      w.emit(emitPidAnnotation(g.pid));
      w.emit(`getter ${g.name}: ${g.returnType} = ${g.expression};`);
    }

    // Actions
    for (const a of store.actions) {
      if (store.signals.length + store.getters.length > 0 || a !== store.actions[0]) w.blank();
      const paramsStr = a.params.map((p) => `${p.name}: ${p.type}`).join(", ");
      const retStr = a.returnType ? `: ${a.returnType}` : "";
      const asyncStr = a.isAsync ? " async" : "";
      w.emit(emitPidAnnotation(a.pid));
      w.emit(`action ${a.name}(${paramsStr})${retStr}${asyncStr} {`);

      if (a.elements.length > 0) {
        w.indent();
        const actionFnPid = a.elements.find((e) => e.tag === "fn")?.id;
        if (actionFnPid) {
          const body = compileCodeFlowGraph(a.elements, a.wires, actionFnPid);
          const lines = body.split("\n");
          const innerLines = lines.slice(1, -1);
          let hasContent = false;
          for (const line of innerLines) {
            const trimmed = line.trim();
            if (trimmed.length > 0) {
              w.emit(trimmed);
              hasContent = true;
            }
          }
          if (!hasContent) {
            w.emit(`fn ${a.name}() {`);
            w.emit(`}`);
          }
        } else {
          w.emit(`fn ${a.name}() {`);
          w.emit(`}`);
        }
        w.dedent();
      }
      w.emit(`}`);
    }

    w.dedent();
    w.emit(`}`);
  }

  // ── Component metadata comments ──

  private emitComponentMeta(meta: EmitInput["componentMeta"], w: CodeWriter): void {
    if (!meta) return;

    // @component: id, category, children, description
    const parts: string[] = [];
    if (meta.id) parts.push(`id=${meta.id}`);
    if (meta.category) parts.push(`category=${meta.category}`);
    if (meta.children !== undefined) parts.push(`children=${meta.children}`);
    if (meta.description) parts.push(`description="${meta.description}"`);
    if (parts.length > 0) w.emit(`// @component ${parts.join(" ")}`);

    // @prop: per-prop editor schema (extends `prop <name>: <type>;` in DSL)
    if (meta.propSchema) {
      for (const [name, p] of Object.entries(meta.propSchema)) {
        const propParts: string[] = [`name=${name}`];
        if (p.editor) propParts.push(`editor=${p.editor}`);
        if (p.values && p.values.length > 0) propParts.push(`values=${p.values.join(",")}`);
        w.emit(`// @prop ${propParts.join(" ")}`);
      }
    }

    // @event: per-event editor metadata (extends `event <name>;` in DSL)
    if (meta.eventSchema) {
      for (const [name, e] of Object.entries(meta.eventSchema)) {
        const evtParts: string[] = [`name=${name}`];
        if (e.payload) evtParts.push(`payload=${e.payload}`);
        if (e.description) evtParts.push(`description="${e.description}"`);
        w.emit(`// @event ${evtParts.join(" ")}`);
      }
    }

    // @style: each style property
    if (meta.styleProperties) {
      for (const sp of meta.styleProperties) {
        w.emit(`// @style ${sp}`);
      }
    }
  }

  /** HTML void elements that must self-close. */
  private static readonly VOID_ELEMENTS = new Set([
    "area", "base", "br", "col", "embed", "hr", "img",
    "input", "link", "meta", "param", "source", "track", "wbr",
  ]);

  /** HTML boolean attributes — emitted as bare attribute name. */
  private static readonly BOOLEAN_ATTRS = new Set([
    "disabled", "required", "readonly", "checked", "selected",
    "hidden", "multiple", "autofocus", "novalidate",
    "async", "defer", "ismap", "itemscope", "open",
    "loop", "muted", "controls", "autoplay",
  ]);

  private emitViewNode(node: ViewNode, w: CodeWriter, depth: number): void {
    // PID annotation (skip for text/interpolation which are inline)
    if (this.enableAnnotations && this.annotationStrategy && node.kind !== "text" && node.kind !== "interpolation") {
      const ann = this.annotationStrategy.beforeViewNode(node, depth);
      if (ann) w.emit(emitPidAnnotation(ann.pid));
    }

    // ── Non-element node kinds (don't need resolvedStyle) ──

    if (node.kind === "text") {
      w.emit(node.text);
      return;
    }
    if (node.kind === "interpolation") {
      w.emit(`{${node.expression}}`);
      return;
    }

    // ── @if / @else if / @else ──
    if (node.kind === "if") {
      if (node.branches && node.branches.length > 0) {
        const first = node.branches[0];
        w.emit(`@if ${first.condition} {`);
        w.indent();
        for (const child of first.body) this.emitViewNode(child, w, depth + 1);
        w.dedent();
        w.emit(`}`);
        for (let i = 1; i < node.branches.length; i++) {
          const b = node.branches[i];
          w.emit(`@else if ${b.condition} {`);
          w.indent();
          for (const child of b.body) this.emitViewNode(child, w, depth + 1);
          w.dedent();
          w.emit(`}`);
        }
      } else if (node.condition) {
        w.emit(`@if ${node.condition} {`);
        w.indent();
        for (const child of node.children) this.emitViewNode(child, w, depth + 1);
        w.dedent();
        w.emit(`}`);
      } else {
        return;
      }
      if (node.fallback && node.fallback.length > 0) {
        w.emit(`@else {`);
        w.indent();
        for (const child of node.fallback) this.emitViewNode(child, w, depth + 1);
        w.dedent();
        w.emit(`}`);
      }
      return;
    }

    // ── @for / @empty ──
    if (node.kind === "for") {
      const iter = node.iterable ?? "";
      const item = node.itemBinding ?? "item";
      w.emit(`@for ${item} in ${iter} {`);
      w.indent();
      for (const child of node.children) this.emitViewNode(child, w, depth + 1);
      w.dedent();
      w.emit(`}`);
      if (node.emptyBody && node.emptyBody.length > 0) {
        w.emit(`@empty {`);
        w.indent();
        for (const child of node.emptyBody) this.emitViewNode(child, w, depth + 1);
        w.dedent();
        w.emit(`}`);
      }
      return;
    }

    // ── @slot ──
    if (node.kind === "slot") {
      const name = node.slotName ?? "";
      w.emit(name ? `@slot ${name} {` : `@slot {`);
      w.indent();
      for (const child of node.children) this.emitViewNode(child, w, depth + 1);
      w.dedent();
      w.emit(`}`);
      return;
    }

    // ── Element nodes ──
    // Elements may or may not have resolvedStyle (WASM handles CSS resolution).
    // When resolvedStyle is missing, derive info from the node's own fields.
    const elNode = node as ElementViewNode;
    const tag = elNode.tag;
    const style = elNode.resolvedStyle;

    const parts: string[] = [];
    parts.push(`<${tag}`);
    parts.push(` id="${elNode.id}"`);

    // ── class: attribute ──
    // Design class refs from elementClassMap (e.g. "AppDesign.Card")
    // Local class from resolvedStyle.cssClassName (e.g. "app_view" for root wrapper)
    const designClasses = this.elementClassMap?.get(elNode.id);
    const localClass = style?.cssClassName;
    const classParts: string[] = [];
    if (localClass) classParts.push(localClass);
    if (designClasses && designClasses.length > 0) {
      for (const cls of designClasses) {
        if (!classParts.includes(cls)) classParts.push(cls);
      }
    }
    // Only emit class: when there are meaningful class names
    const validClassParts = classParts.filter(c => c && c.length > 0);
    if (validClassParts.length > 0) {
      parts.push(` class: ${validClassParts.join(" ")}`);
    }

    // ── variant: attribute ──
    const variantRef = elNode.variantRef;
    if (variantRef && variantRef.designName && variantRef.variantName) {
      const paramsStr = Object.entries(variantRef.params)
        .map(([k, val]) => `${k}=${val}`)
        .join(", ");
      const variantStr = `${variantRef.designName}.${variantRef.variantName}`;
      parts.push(` variant: ${variantStr}${paramsStr ? `(${paramsStr})` : ""}`);
    }

    // ── HTML attributes ──
    if (style?.htmlAttrs) {
      for (const [attr, val] of Object.entries(style.htmlAttrs)) {
        if (attr === "className" || attr === "style") continue;
        if (SourceEmitter.BOOLEAN_ATTRS.has(attr)) {
          parts.push(` ${attr}`);
        } else {
          parts.push(` ${attr}="${escapeAttr(String(val))}"`);
        }
      }
    } else {
      // Derive HTML attributes from node.props (used when applyStyles is no-op)
      for (const [key, val] of Object.entries(elNode.props)) {
        if (key === "className" || key === "style" || key === "key" || key === "label") continue;
        if (key.startsWith("on")) continue; // event handlers handled below
        if (typeof val === "boolean") {
          if (val) parts.push(` ${key}`);
        } else if (typeof val === "string" || typeof val === "number") {
          parts.push(` ${key}="${escapeAttr(String(val))}"`);
        }
      }
    }

    // ── Inline styles ──
    const inlineStyles = this.elementInlineMap?.get(elNode.id);
    if (inlineStyles && Object.keys(inlineStyles).length > 0) {
      const styleStr = Object.entries(inlineStyles)
        .filter(([, v]) => v !== "" && v !== undefined && v !== null)
        .map(([k, v]) => `${k}: ${v}`)
        .join(", ");
      parts.push(` style: { ${styleStr} }`);
    }

    // ── Event bindings ──
    if (style?.eventBindings) {
      for (const [evt, handler] of Object.entries(style.eventBindings)) {
        parts.push(` @${evt}={${handler}}`);
      }
    } else {
      for (const [key, val] of Object.entries(elNode.props)) {
        if (key.startsWith("on") && typeof val === "string") {
          const evtName = key[2].toLowerCase() + key.slice(3); // onClick → click
          parts.push(` @${evtName}={${val}}`);
        }
      }
    }

    // ── Signal bindings ──
    // Prefer elementBindings map (newer path) over resolvedStyle.bindings
    const emittedBindings = new Set<string>();
    if (this.elementBindings) {
      const elBindings = this.elementBindings.get(elNode.id);
      if (elBindings) {
        for (const [prop, sig] of Object.entries(elBindings)) {
          parts.push(` bind:${prop}={${sig}}`);
          emittedBindings.add(prop);
        }
      }
    }
    if (style?.bindings) {
      for (const [prop, sig] of Object.entries(style.bindings)) {
        if (!emittedBindings.has(prop)) {
          parts.push(` bind:${prop}={${sig}}`);
        }
      }
    }

    const openTag = parts.join("");
    const isVoid = SourceEmitter.VOID_ELEMENTS.has(tag);
    // Determine content text: prefer resolvedStyle, fall back to props.label
    let contentText = style?.contentText;
    if (!contentText && elNode.props.label && typeof elNode.props.label === "string") {
      contentText = elNode.props.label;
    }
    const hasContent = contentText || elNode.children.length > 0;

    // Custom components: always self-close — their internal template
    // is defined in their own .omni file. Expanded children are a
    // canvas-rendering artifact, not part of the component's source API.
    const isComponentTag = /^[A-Z]/.test(tag);
    if (isComponentTag) {
      w.emit(`${openTag} />`);
      return;
    }

    if (isVoid || !hasContent) {
      w.emit(isVoid ? `${openTag} />` : `${openTag}></${tag}>`);
      return;
    }

    w.emit(`${openTag}>`);
    w.indent();

    // Emit content text only when no binding covers the same prop.
    if (contentText) {
      const bindingKeys = new Set([
        ...(this.elementBindings?.get(elNode.id) ? Object.keys(this.elementBindings.get(elNode.id)!) : []),
        ...Object.keys(style?.bindings ?? {}),
      ]);
      const contentIsBound = bindingKeys.has("label") || bindingKeys.has("value");
      if (!contentIsBound) {
        w.emit(contentText);
      }
    }

    for (const child of elNode.children) {
      if (child.kind === "text" || child.kind === "interpolation") {
        this.emitViewNode(child, w, depth + 1);
      } else {
        this.emitViewNode(child, w, depth + 1);
      }
    }

    w.dedent();
    w.emit(`</${tag}>`);
  }
}

function formatChainBlock(chain: ChainBlock): string {
  const lines: string[] = [];
  lines.push(`// ═══════════════════════════════════════════════════════`);
  lines.push(`// @chain ${chain.functionName}`);
  for (const nl of chain.nodeLines) lines.push(nl);
  for (const wl of chain.wireLines) lines.push(wl);
  lines.push(`// ═══════════════════════════════════════════════════════`);
  return lines.join("\n");
}

function escapeAttr(val: string): string {
  return val.replace(/"/g, '\\"');
}
