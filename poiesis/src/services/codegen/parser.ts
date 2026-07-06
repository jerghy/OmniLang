// poiesis/src/omni-gen/parser.ts
// Recursive-descent Parser class for OmniLang sources.

import type { Token } from "./omni-tokenizer";
import { TokenStream } from "./token-stream";
import type { ParseResult } from "./types";
import type { ParserState, ParseContext } from "./parser-types";
import { parseKeyValueAttrs } from "./parser-types";
import * as ParserHelpers from "./parser-helpers";
import * as ParserView from "./parser-view";
import * as ParserFinalize from "./parser-finalize";

export interface ParserInternals {
  stream: TokenStream;
  ctx: ParseContext;
  indentMap: Map<string, number>;
}

export class Parser {
  private stream: TokenStream;
  private ctx: ParseContext;
  private indentMap = new Map<string, number>();

  constructor(stream: TokenStream) {
    this.stream = stream;
    this.ctx = this.createContext();
  }

  private createContext(): ParseContext {
    return {
      state: "ROOT",
      stateStack: [],
      braceDepth: 0,
      braceDepthStack: [],
      elements: [],
      wires: [],
      codeTabs: [
        {
          id: "main",
          type: "logic",
          label: "Main",
          icon: "\u{1F4C4}",
          graph: { elements: [], wires: [] },
          variables: [],
        },
      ],
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
      pendingEditFlags: null,
      props: [],
      events: [],
      componentMeta: null,
      hasStyleBlock: false,
    };
  }

  // ── Public entry ──

  parse(): ParseResult {
    this.parseFile();
    this.finalize();
    return this.buildResult();
  }

  // ── Top-level ──

  private parseFile(): void {
    while (!this.stream.isEof()) {
      this.stream.skipNewlines();

      if (this.stream.isEof()) break;

      const tok = this.stream.peek();

      switch (tok.kind) {
        case "COMMENT_LINE":
          this.parseCommentDirective();
          break;
        case "IDENTIFIER":
          this.parseRootIdentifier();
          break;
        case "EOF":
          return;
        default:
          // Skip unexpected tokens at root
          this.stream.consume();
          break;
      }
    }
  }

  // ── Comment directives (ROOT) ──

  private parseCommentDirective(): void {
    const tok = this.stream.consume();
    const text = tok.text.trim();

    // @code-tab directive
    if (text.startsWith("// @code-tab type=")) {
      const m = text.match(/type=(logic|event)/);
      if (m) {
        this.ctx.codeTabType = m[1] as "logic" | "event";
        this.ctx.codeTabs[0].type = this.ctx.codeTabType;
      } else {
        this.pushError(
          "warning", "P001",
          `Invalid @code-tab type value in: "${text}"`,
          tok,
        );
      }
      return;
    }

    // @omni-file directive (same semantics as @code-tab)
    if (text.startsWith("// @omni-file type=")) {
      const m = text.match(/type=(logic|event)/);
      if (m) {
        this.ctx.codeTabType = m[1] as "logic" | "event";
        this.ctx.codeTabs[0].type = this.ctx.codeTabType;
      }
      return;
    }

    // Component metadata comments
    if (text.startsWith("// @component ")) {
      const m = text.match(/\/\/\s*@component\s+(.+)/);
      if (m) {
        const meta = parseKeyValueAttrs(m[1]);
        this.ctx.componentMeta = this.ctx.componentMeta || {};
        if (meta.id !== undefined) this.ctx.componentMeta.id = meta.id;
        if (meta.category !== undefined) this.ctx.componentMeta.category = meta.category;
        if (meta.children !== undefined) this.ctx.componentMeta.children = meta.children === "true";
        if (meta.description !== undefined) this.ctx.componentMeta.description = meta.description;
      }
      return;
    }

    if (text.startsWith("// @prop ")) {
      const m = text.match(/\/\/\s*@prop\s+(.+)/);
      if (m) {
        const meta = parseKeyValueAttrs(m[1]);
        if (meta.name) {
          this.ctx.componentMeta = this.ctx.componentMeta || {};
          this.ctx.componentMeta.propSchema = this.ctx.componentMeta.propSchema || ({} as Record<string, unknown>);
          const schema: Record<string, unknown> = {};
          if (meta.editor !== undefined) schema.editor = meta.editor;
          if (meta.values !== undefined) schema.values = String(meta.values).split(",");
          (this.ctx.componentMeta.propSchema as Record<string, unknown>)[meta.name] = schema;
        }
      }
      return;
    }

    if (text.startsWith("// @event ")) {
      const m = text.match(/\/\/\s*@event\s+(.+)/);
      if (m) {
        const meta = parseKeyValueAttrs(m[1]);
        if (meta.name) {
          this.ctx.componentMeta = this.ctx.componentMeta || {};
          this.ctx.componentMeta.eventSchema = this.ctx.componentMeta.eventSchema || ({} as Record<string, unknown>);
          const schema: Record<string, unknown> = {};
          if (meta.payload !== undefined) schema.payload = meta.payload;
          if (meta.description !== undefined) schema.description = meta.description;
          (this.ctx.componentMeta.eventSchema as Record<string, unknown>)[meta.name] = schema;
        }
      }
      return;
    }

    if (text.startsWith("// @style ")) {
      const name = text.slice("// @style ".length).trim();
      if (name) {
        this.ctx.componentMeta = this.ctx.componentMeta || {};
        this.ctx.componentMeta.styleProperties = this.ctx.componentMeta.styleProperties || [];
        (this.ctx.componentMeta.styleProperties as string[]).push(name);
      }
      return;
    }

    // Chain-related comments are handled in IN_CHAIN state or IN_COMPONENT state
    // Ignore other comments at root level
  }

  // ── Root-level identifiers (import, component, store) ──

  private parseRootIdentifier(): void {
    const kw = this.stream.peek().text;

    if (kw === "import") {
      // Consume the rest of this line as an import statement
      this.stream.consume(); // "import"
      let parts = "import";
      while (
        !this.stream.isEof() &&
        this.stream.peek().kind !== "SEMICOLON" &&
        this.stream.peek().kind !== "NEWLINE" &&
        this.stream.peek().kind !== "EOF"
      ) {
        parts += this.stream.consume().text;
      }
      if (this.stream.peek().kind === "SEMICOLON") {
        parts += this.stream.consume().text; // consume ";"
      }
      this.ctx.imports.push(parts);
      return;
    }

    if (kw === "component") {
      this.stream.consume(); // "component"
      const nameTok = this.stream.expect("IDENTIFIER");
      if (nameTok.kind === "EOF") {
        this.pushError("error", "P002", "Invalid component declaration: missing name", nameTok);
        return;
      }
      this.ctx.currentComponent = nameTok.text;
      this.stream.expect("BRACE_OPEN");
      this.pushState("IN_COMPONENT");
      return;
    }

    if (kw === "store") {
      this.stream.consume(); // "store"
      const nameTok = this.stream.expect("IDENTIFIER");
      if (nameTok.kind === "EOF") {
        this.pushError("error", "P003", "Invalid store declaration: missing name", nameTok);
        return;
      }
      this.ctx.currentStore = nameTok.text;
      this.stream.expect("BRACE_OPEN");
      this.pushState("IN_STORE");
      return;
    }

    // Unknown identifier at root — skip
    this.stream.consume();
  }

  // ── IN_COMPONENT ──

  private parseComponentBody(): void {
    while (!this.stream.isEof()) {
      this.stream.skipNewlines();

      if (this.stream.isEof()) break;

      const tok = this.stream.peek();

      // Close component
      if (tok.kind === "BRACE_CLOSE") {
        this.stream.consume();
        this.popState();
        return;
      }

      // Handle various component-level constructs
      switch (tok.kind) {
        case "COMMENT_LINE":
          this.parseComponentComment();
          break;
        case "IDENTIFIER":
          this.parseComponentKeyword();
          break;
        case "AT":
          this.parseAtStore();
          break;
        default:
          // Skip unknown tokens
          this.stream.consume();
          break;
      }
    }
  }

  private parseComponentComment(): void {
    const tok = this.stream.consume();
    const text = tok.text.trim();

    // @chain
    if (text.startsWith("// @chain ")) {
      const m = text.match(/\/\/\s*@chain\s+(\w+)/);
      if (m) {
        this.flushChain();
        this.ctx.chainName = m[1];
        this.ctx.chainElements = [];
        this.ctx.chainWires = [];
        this.pushState("IN_CHAIN");
      }
      return;
    }

    // Other comments in component body are ignored (same as old parser)
  }

  private parseComponentKeyword(): void {
    const kw = this.stream.peek().text;

    switch (kw) {
      case "signal":
        this.parseSignal();
        break;
      case "computed":
        this.parseComputed();
        break;
      case "prop":
        this.parsePropDeclaration();
        break;
      case "event":
        this.parseEventDeclaration();
        break;
      case "fn":
        this.parseFn();
        break;
      case "view":
        this.parseView();
        break;
      case "style":
        this.parseStyleBlock();
        break;
      default:
        // Unknown keyword — skip token
        this.stream.consume();
        break;
    }
  }

  private parseAtStore(): void {
    this.stream.consume(); // @
    const kw = this.stream.peek()?.text;
    if (kw === "store") {
      this.stream.consume(); // "store"
      const nameTok = this.stream.expect("IDENTIFIER");
      this.stream.expect("SEMICOLON");
      this.ctx.elements.push(
        this.codeblockEl(
          `${this.ctx.currentComponent}.store.${nameTok.text}`,
          "store",
          { name: nameTok.text },
        ),
      );
    }
  }

  // ── Signal ──

  private parseSignal(): void {
    this.stream.consume(); // "signal"
    const nameTok = this.stream.expect("IDENTIFIER");
    this.stream.expect("COLON");
    const typeTok = this.stream.expect("IDENTIFIER");
    // Optional = default
    if (this.stream.peek().kind === "EQUALS") {
      // Skip default value expression
      this.stream.consume();
      this.skipUntilSemicolon();
    }
    this.stream.expect("SEMICOLON");

    this.ctx.elements.push(
      this.codeblockEl(
        `${this.ctx.currentComponent}.signal.${nameTok.text}`,
        "signal",
        { name: nameTok.text, typeAnnotation: typeTok.text },
      ),
    );
  }

  // ── Computed ──

  private parseComputed(): void {
    this.stream.consume(); // "computed"
    const nameTok = this.stream.expect("IDENTIFIER");
    this.stream.expect("COLON");
    const typeTok = this.stream.expect("IDENTIFIER");
    this.stream.expect("EQUALS");
    // Skip the body expression until ;
    this.skipUntilSemicolon();
    this.stream.expect("SEMICOLON");

    this.ctx.elements.push(
      this.codeblockEl(
        `${this.ctx.currentComponent}.computed.${nameTok.text}`,
        "computed",
        { name: nameTok.text, typeAnnotation: typeTok.text },
      ),
    );
  }

  // ── Prop declaration ──

  private parsePropDeclaration(): void {
    this.stream.consume(); // "prop"
    const nameTok = this.stream.expect("IDENTIFIER");
    this.stream.expect("COLON");
    const typeTok = this.stream.expect("IDENTIFIER");
    let defaultVal: string | undefined;
    if (this.stream.peek().kind === "EQUALS") {
      this.stream.consume();
      // Read the default value (could be a string or identifier)
      const defTok = this.stream.peek();
      if (defTok.kind === "STRING") {
        defaultVal = this.stream.consume().text.slice(1, -1);
      } else {
        defaultVal = this.stream.consume().text;
      }
    }
    this.stream.expect("SEMICOLON");

    this.ctx.props.push({
      name: nameTok.text,
      type: typeTok.text,
      defaultValue: defaultVal || undefined,
    });
  }

  // ── Event declaration ──

  private parseEventDeclaration(): void {
    this.stream.consume(); // "event"
    const nameTok = this.stream.expect("IDENTIFIER");
    this.stream.expect("SEMICOLON");
    this.ctx.events.push({ name: nameTok.text });
  }

  // ── Fn body (skip by brace counting) ──

  private parseFn(): void {
    this.stream.consume(); // "fn"
    // Skip fn signature tokens until the opening BRACE_OPEN that starts the body
    while (!this.stream.isEof()) {
      const tok = this.stream.peek();
      if (tok.kind === "BRACE_OPEN") {
        this.stream.consume();
        break;
      }
      this.stream.consume();
    }
    this.pushState("IN_FN");
  }

  private parseFnBody(): void {
    let depth = 1;
    while (depth > 0 && !this.stream.isEof()) {
      const tok = this.stream.consume();
      if (tok.kind === "BRACE_OPEN") depth++;
      else if (tok.kind === "BRACE_CLOSE") depth--;
    }
    this.popState();
  }

  // ── Store body (skip by brace counting) ──

  private parseStoreBody(): void {
    let depth = 1;
    while (depth > 0 && !this.stream.isEof()) {
      const tok = this.stream.consume();
      if (tok.kind === "BRACE_OPEN") depth++;
      else if (tok.kind === "BRACE_CLOSE") depth--;
    }
    this.popState();
  }

  // ── Style block ──

  private parseStyleBlock(): void {
    this.stream.consume(); // "style"
    this.stream.expect("BRACE_OPEN");
    this.ctx.hasStyleBlock = true;
    this.pushState("IN_STYLE");
  }

  private parseStyleBlockBody(): void {
    this.stream.skipNewlines();

    while (!this.stream.isEof()) {
      // Collect ALL tokens on this line until NEWLINE or EOF.
      // CSS rules like `.card { ... }` have their own BRACE_CLOSE on the same line.
      // The style block's closing `}` will appear on its own line (after a NEWLINE).
      const lineTokens: Token[] = [];
      while (
        !this.stream.isEof() &&
        this.stream.peek().kind !== "NEWLINE"
      ) {
        lineTokens.push(this.stream.consume());
      }

      // If the line is just a BRACE_CLOSE, it's the end of the style block
      if (
        lineTokens.length === 1 &&
        lineTokens[0].kind === "BRACE_CLOSE"
      ) {
        this.popState();
        return;
      }

      // Reconstruct the line from tokens and try to match a CSS rule
      const line = lineTokens.map((t) => t.text).join("");

      // Use the same regex as the old parser to extract CSS rules
      const ruleMatch = line.match(/^\.([\w-]+)\s*\{\s*(.*?)\s*\}$/);
      if (ruleMatch) {
        const className = ruleMatch[1];
        const propsStr = ruleMatch[2];
        const properties: Record<string, string> = {};
        for (const pair of propsStr.split(";").filter((s) => s.trim().length > 0)) {
          const colonIdx = pair.indexOf(":");
          if (colonIdx > 0) {
            properties[pair.slice(0, colonIdx).trim()] = pair.slice(colonIdx + 1).trim();
          }
        }
        this.ctx.parsedStyleRules.push({ class_name: className, properties });
      }

      this.stream.skipNewlines();
    }
  }

  // ── Chain block ──

  private parseChainBody(): void {
    this.stream.skipNewlines();

    while (!this.stream.isEof()) {
      const tok = this.stream.peek();

      // Chain separator or end of file
      if (tok.kind === "EOF") break;

      if (tok.kind === "COMMENT_LINE") {
        const text = tok.text.trimStart();

        // Chain separator
        if (text.startsWith("// ═")) {
          this.flushChain();
          this.stream.consume();
          this.popState();
          return;
        }

        // @node
        if (text.startsWith("//   @node ") || text.startsWith("// @node ")) {
          this.stream.consume();
          const parts = text.replace(/^\/\/\s*/, "").trim().split(/\s+/);
          const nodeId = parts[1];
          const nodeTag = parts[2];
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
          this.ctx.chainElements.push({
            id: nodeId,
            nodeType: "codeblock",
            tag: nodeTag,
            props: attrs,
            position: { x: posX, y: posY },
            children: [],
            parentId: null,
          });
          this.stream.skipNewlines();
          continue;
        }

        // @wire
        if (text.startsWith("//   @wire ") || text.startsWith("// @wire ")) {
          this.stream.consume();
          const wireMatch = text.match(/@wire\s+(\S+)\s+(\S+)\s*->\s*(\S+)/);
          if (wireMatch) {
            const [, wireId, from, to] = wireMatch;
            const [fromId, fromPort = ""] = from.includes(".") ? from.split(".") : [from, ""];
            const [toId, toPort = ""] = to.includes(".") ? to.split(".") : [to, ""];
            this.ctx.chainWires.push({ id: wireId, fromId, fromPort, toId, toPort });
          }
          this.stream.skipNewlines();
          continue;
        }

        // Other comment in chain — skip (old parser ignores)
        this.stream.consume();
        this.stream.skipNewlines();
        continue;
      }

      // Non-comment tokens in chain — skip
      this.stream.consume();
      this.stream.skipNewlines();
    }
  }

  // ── State machine helpers ──

  private pushState(state: ParserState): void {
    this.ctx.stateStack.push(this.ctx.state);
    this.ctx.braceDepthStack.push(this.ctx.braceDepth);
    this.ctx.state = state;
    // Entering a new block: initial brace depth is 1 (the opening { was just consumed)
    this.ctx.braceDepth = 1;

    // Immediately start parsing the new state's body
    switch (state) {
      case "IN_COMPONENT":
        this.parseComponentBody();
        break;
      case "IN_VIEW":
        this.parseViewBody();
        break;
      case "IN_FN":
        this.parseFnBody();
        break;
      case "IN_STORE":
        this.parseStoreBody();
        break;
      case "IN_STYLE":
        this.parseStyleBlockBody();
        break;
      case "IN_CHAIN":
        this.parseChainBody();
        break;
    }
  }

  private popState(): void {
    this.ctx.state = this.ctx.stateStack.pop() || "ROOT";
    this.ctx.braceDepth = this.ctx.braceDepthStack.pop() || 0;
  }

  // ── Internals getter ──

  private get internals(): ParserInternals {
    return { stream: this.stream, ctx: this.ctx, indentMap: this.indentMap };
  }

  // ── Delegation wrappers for extracted methods ──

  private skipUntilSemicolon(): void {
    ParserHelpers.skipUntilSemicolon(this.stream);
  }

  private pushError(
    severity: string,
    code: string,
    message: string,
    tok: Token,
  ): void {
    ParserHelpers.pushError(this.ctx, severity, code, message, tok);
  }

  private codeblockEl(
    id: string,
    tag: string,
    props: Record<string, unknown>,
  ) {
    return ParserHelpers.codeblockEl(id, tag, props);
  }

  private flushChain(): void {
    ParserHelpers.flushChain(this.ctx);
  }

  private parseView(): void {
    ParserView.parseView(this.internals);
  }

  private parseViewBody(): void {
    ParserView.parseViewBody(this.internals);
  }

  private finalize(): void {
    ParserFinalize.finalizeParse(this.internals);
  }

  private buildResult(): ParseResult {
    return ParserFinalize.buildParseResult(this.ctx);
  }
}
