// editor/client/src/omni-gen/source-emitter.ts
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
  constructor(private annotationStrategy?: AnnotationStrategy) {}

  emitDocument(input: EmitInput, chains: Map<string, ChainBlock>): string {
    const writer = new CodeWriter();

    // @code-tab
    if (this.annotationStrategy) {
      writer.emit(this.annotationStrategy.beforeFile());
    }

    // imports
    if (input.imports.length > 0) {
      for (const imp of input.imports) writer.emit(imp);
      writer.blank();
    }

    // stores
    for (const store of input.stores) {
      this.emitStore(store, writer, chains);
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

  private emitStore(store: EmitInput["stores"][0], w: CodeWriter, chains: Map<string, ChainBlock>): void {
    w.emit(`store ${store.name} {`);
    w.indent();
    for (const [name, sig] of Object.entries(store.signals)) {
      const def = sig.default !== undefined ? ` = ${JSON.stringify(sig.default)}` : "";
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
    w.emit(`component ${comp.name} {`);
    w.indent();

    for (const sr of comp.storeRefs) w.emit(`@store ${sr};`);
    for (const [name, sig] of Object.entries(comp.signals)) {
      const def = sig.default !== undefined ? ` = ${JSON.stringify(sig.default)}` : "";
      w.emit(`signal ${name}: ${sig.type}${def};`);
    }
    for (const [name, c] of Object.entries(comp.computeds)) {
      w.emit(`computed ${name}: ${c.type} = ${c.body};`);
    }

    // Functions with chains
    for (const fn of comp.fns) {
      // Event handler boundary marker (for round-trip tab reconstruction)
      if (input.codeTabType === "event") {
        w.emit(`// @code-tab type=event label=${fn.name}`);
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

    // Style
    if (Object.keys(comp.style).length > 0) {
      w.emit(`style {`);
      w.indent();
      for (const [cls, rules] of Object.entries(comp.style)) {
        const ruleStr = Object.entries(rules).map(([k, v]) => `${k}: ${v};`).join(" ");
        w.emit(`.${cls} { ${ruleStr} }`);
      }
      w.dedent();
      w.emit(`}`);
    }

    w.dedent();
    w.emit(`}`);
  }

  private emitViewNode(node: ViewNode, w: CodeWriter, depth: number): void {
    // Annotation
    if (this.annotationStrategy) {
      const ann = this.annotationStrategy.beforeViewNode(node, depth);
      if (ann) w.emit(ann);
    }

    const style = node.resolvedStyle;
    if (!style) return;

    const parts: string[] = [];
    parts.push(`<${style.htmlTag}`);
    parts.push(` id="${node.id}"`);
    parts.push(` class: ${style.cssClassName}`);

    for (const [attr, val] of Object.entries(style.htmlAttrs)) {
      parts.push(` ${attr}="${escapeAttr(val)}"`);
    }
    for (const [evt, handler] of Object.entries(style.eventBindings)) {
      parts.push(` @${evt}={${handler}}`);
    }
    for (const [prop, sig] of Object.entries(style.bindings)) {
      parts.push(` bind:${prop}={${sig}}`);
    }

    const openTag = parts.join("");
    const hasContent = style.contentText || node.children.length > 0;

    if (!hasContent) {
      w.emit(`${openTag}></${style.htmlTag}>`);
      return;
    }

    w.emit(`${openTag}>`);
    w.indent();

    if (style.contentText) w.emit(style.contentText);

    for (const child of node.children) this.emitViewNode(child, w, depth + 1);

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
