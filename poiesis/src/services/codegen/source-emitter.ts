// poiesis/src/services/codegen/source-emitter.ts
// CodeWriter helper + SourceEmitter for view and style generation.

import type { EmitInput, AnnotationStrategy, ChainBlock } from "./types";
import type { ViewNode } from "./types";

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

export class SourceEmitter {
  constructor(
    private annotationStrategy?: AnnotationStrategy,
    private elementClassMap?: Map<string, string[]>,
    private elementInlineMap?: Map<string, Record<string, string>>,
  ) {}

  emitDocument(input: EmitInput, chains: Map<string, ChainBlock>): string {
    const writer = new CodeWriter();

    // @omni-file directive (only emitted for non-default types like "event")
    if (this.annotationStrategy) {
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

  private emitStore(store: EmitInput["stores"][0], w: CodeWriter): void {
    w.emit(`store ${store.name} {`);
    w.indent();
    for (const [name, sig] of Object.entries(store.signals)) {
      const def = sig.defaultValue !== undefined ? ` = ${JSON.stringify(sig.defaultValue)}` : "";
      w.emit(`signal ${name}: ${sig.type}${def};`);
    }
    for (const [name, g] of Object.entries(store.getters)) {
      w.emit(`getter ${name}: ${g.returnType} = ${g.body};`);
    }
    for (const a of store.actions) {
      const params = a.params.map(p => `${p.name}: ${p.type}`).join(", ");
      w.emit(`action ${a.name}(${params}) {`);
      w.indent();
      for (const line of a.body.split("\n")) w.emit(line);
      w.dedent();
      w.emit(`}`);
    }
    w.dedent();
    w.emit(`}`);
  }

  private emitComponent(input: EmitInput, w: CodeWriter, chains: Map<string, ChainBlock>): void {
    const comp = input.component;

    // Emit component metadata comments (replaces old *.meta.json)
    this.emitComponentMeta(input.componentMeta, w);

    w.emit(`component ${comp.name} {`);
    w.indent();

    for (const sr of comp.storeRefs) w.emit(`@store ${sr};`);

    // Props (e.g. `prop title: String = ""`)
    for (const prop of comp.props || []) {
      const def = prop.defaultValue !== undefined ? ` = ${JSON.stringify(prop.defaultValue)}` : "";
      w.emit(`prop ${prop.name}: ${prop.type}${def};`);
    }
    // Events (e.g. `event onAction`)
    for (const evt of comp.events || []) {
      w.emit(`event ${evt.name};`);
    }

    for (const [name, sig] of Object.entries(comp.signals)) {
      const def = sig.defaultValue !== undefined ? ` = ${JSON.stringify(sig.defaultValue)}` : "";
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
    if (this.annotationStrategy && node.kind !== "text" && node.kind !== "interpolation") {
      const ann = this.annotationStrategy.beforeViewNode(node, depth);
      if (ann) w.emit(`// @pid:${ann.pid}`);
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

    // ── Element nodes (require resolvedStyle) ──
    // If resolvedStyle is missing, skip silently — this can happen when a node
    // was added to the ViewNode tree but applyStyles() hasn't been called yet
    // (e.g. during incremental compilation where only part of the tree is rebuilt).
    const style = node.resolvedStyle;
    if (!style) {
      // Skipping element node without resolvedStyle — caller should ensure
      // applyStyles() is called before emitting.
      return;
    }

    const parts: string[] = [];
    parts.push(`<${style.htmlTag}`);
    parts.push(` id="${node.id}"`);

    // Base class name from resolved style + extra classes from element classRefs
    const extraClasses = this.elementClassMap?.get(node.id);
    const fullClassName = extraClasses?.length
      ? `${style.cssClassName} ${extraClasses.join(" ")}`
      : style.cssClassName;
    parts.push(` class: ${fullClassName}`);

    for (const [attr, val] of Object.entries(style.htmlAttrs)) {
      // Skip className — already emitted via class: above
      if (attr === "className" || attr === "style") continue;
      // Boolean attributes: emit bare name
      if (SourceEmitter.BOOLEAN_ATTRS.has(attr)) {
        parts.push(` ${attr}`);
      } else {
        parts.push(` ${attr}="${escapeAttr(val)}"`);
      }
    }

    // Inline styles — values are already compiled segment strings from
    // collectStylesFromEntries (text segments quoted, signals bare, var-refs as var()).
    const inlineStyles = this.elementInlineMap?.get(node.id);
    if (inlineStyles && Object.keys(inlineStyles).length > 0) {
      const styleStr = Object.entries(inlineStyles)
        .filter(([, v]) => v !== "" && v !== undefined && v !== null)
        .map(([k, v]) => `${k}: ${v}`)
        .join(", ");
      parts.push(` style: { ${styleStr} }`);
    }

    for (const [evt, handler] of Object.entries(style.eventBindings)) {
      parts.push(` @${evt}={${handler}}`);
    }
    for (const [prop, sig] of Object.entries(style.bindings)) {
      parts.push(` bind:${prop}={${sig}}`);
    }

    const openTag = parts.join("");
    const isVoid = SourceEmitter.VOID_ELEMENTS.has(style.htmlTag);
    const hasContent = style.contentText || node.children.length > 0;

    if (isVoid || !hasContent) {
      w.emit(isVoid ? `${openTag} />` : `${openTag}></${style.htmlTag}>`);
      return;
    }

    w.emit(`${openTag}>`);
    w.indent();

    // Emit content text only when no binding covers the same prop.
    // When a signal is wired to the content prop (e.g. label on Button),
    // the binding takes priority; contentText becomes a stale literal.
    if (style.contentText) {
      const bindingKeys = new Set(Object.keys(style.bindings));
      const contentIsBound = style.contentText.length > 0 &&
        Array.from(bindingKeys).some((k) => k === "label" || k === "value");
      if (!contentIsBound) {
        w.emit(style.contentText);
      }
    }

    for (const child of node.children) {
      if (child.kind === "text" || child.kind === "interpolation") {
        this.emitViewNode(child, w, depth + 1);
      } else {
        this.emitViewNode(child, w, depth + 1);
      }
    }

    w.dedent();
    w.emit(`</${style.htmlTag}>`);
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
