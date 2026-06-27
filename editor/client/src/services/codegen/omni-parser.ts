// editor/client/src/omni-gen/omni-parser.ts
// Recursive-descent parser for OmniLang sources.
// Transforms a token stream into a ParseResult identical to the legacy parser.

import type { ElementData, WireData } from "../store/editor";
import type { CodeTab } from "../codegraph/types";
import type { ParseResult } from "./types";
import { parseStylesFromAST } from "./pipeline";
import { useComponentRegistry } from "../store/componentRegistry";
import { Logger } from "../utils/logger";
import { tokenize } from "./omni-tokenizer";
import { TokenStream } from "./token-stream";
import type { Token, TokenKind } from "./omni-tokenizer";

const log = new Logger({ prefix: "omni-parser" });

// ── Types ──

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
  braceDepth: number;
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
  pendingEditFlags: string | null;
  parsedStyleRules: Array<{ class_name: string; properties: Record<string, string> }>;
  hasStyleBlock: boolean;
  props: Array<{ name: string; type: string; default?: unknown }>;
  events: Array<{ name: string }>;
  componentMeta: Record<string, unknown> | null;
}

// ── Public API ──

export function parseOmniSource(source: string): ParseResult {
  const tokens = tokenize(source);
  const stream = new TokenStream(tokens);
  const parser = new Parser(stream);
  return parser.parse();
}

// ── Parser ──

class Parser {
  private stream: TokenStream;
  private ctx: ParseContext;

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
          icon: "📄",
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
      braceDepth: 0,
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
        case "@":
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
      default: defaultVal || undefined,
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

  // ── View body ──

  private parseView(): void {
    this.stream.consume(); // "view"
    this.stream.expect("BRACE_OPEN");
    this.ctx.viewParentStack = [];
    this.pushState("IN_VIEW");
  }

  private parseViewBody(): void {
    this.stream.skipNewlines();

    while (!this.stream.isEof()) {
      const tok = this.stream.peek();

      // End of view block
      if (tok.kind === "BRACE_CLOSE") {
        this.stream.consume();
        this.popState();
        return;
      }

      // Closing tag
      if (tok.kind === "ANGLE_OPEN" && this.stream.peek(1)?.kind === "SLASH") {
        this.stream.consume(); // ANGLE_OPEN
        this.stream.consume(); // SLASH
        this.stream.consume(); // IDENTIFIER (tag name)
        // Consume optional ANGLE_CLOSE
        if (this.stream.peek().kind === "ANGLE_CLOSE") {
          this.stream.consume();
        }
        if (this.ctx.viewParentStack.length > 0) {
          this.ctx.viewParentStack.pop();
        }
        this.stream.skipNewlines();
        continue;
      }

      // Opening tag
      if (tok.kind === "ANGLE_OPEN") {
        this.parseViewElement();
        this.stream.skipNewlines();
        continue;
      }

      // Comments in view
      if (tok.kind === "COMMENT_LINE") {
        this.parseViewComment();
        this.stream.skipNewlines();
        continue;
      }

      // Text content on its own line
      if (tok.kind === "IDENTIFIER" || tok.kind === "TEXT" || tok.kind === "STRING") {
        this.parseViewTextContent();
        this.stream.skipNewlines();
        continue;
      }

      // Skip any other unexpected tokens
      this.stream.consume();
      this.stream.skipNewlines();
    }
  }

  private parseViewComment(): void {
    const tok = this.stream.consume();
    const text = tok.text.trim();

    // @edit annotation
    if (text.startsWith("// @edit")) {
      this.ctx.pendingEditFlags = text.slice("// @edit".length).trim() || null;
    }
    // Other view comments are ignored (old parser behavior)
  }

  private parseViewTextContent(): void {
    // Collect all textual tokens on this line as text content
    let text = "";
    while (
      !this.stream.isEof() &&
      this.stream.peek().kind !== "NEWLINE" &&
      this.stream.peek().kind !== "BRACE_CLOSE" &&
      this.stream.peek().kind !== "ANGLE_OPEN"
    ) {
      text += this.stream.consume().text;
    }
    text = text.trim();

    if (text && !text.startsWith("//") && !text.startsWith("<")) {
      // Attach as label prop to the current parent element
      const parentId =
        this.ctx.viewParentStack.length > 0
          ? this.ctx.viewParentStack[this.ctx.viewParentStack.length - 1]
          : null;
      if (parentId) {
        const parent = this.ctx.elements.find((e) => e.id === parentId);
        if (parent && !parent.props.label) {
          parent.props.label = text;
        }
      }
    }
  }

  // ── View element ──

  private parseViewElement(): void {
    const indent = this.stream.currentIndent(); // col of first token on this line

    this.stream.consume(); // ANGLE_OPEN (<)
    const tagTok = this.stream.expect("IDENTIFIER");
    const tag = tagTok.text;

    // Parse attributes
    const attrs = this.parseViewAttributes();

    // Generate element ID
    const id = attrs.id !== null
      ? attrs.id
      : `${this.ctx.currentComponent}.${tag}.${this.ctx.elements.length}`;

    // Check for duplicate IDs
    const el = this.createElement(id, tag, attrs, indent);

    // @edit flags
    if (this.ctx.pendingEditFlags) {
      if (this.ctx.pendingEditFlags.includes("folded")) el.isFolded = true;
      if (this.ctx.pendingEditFlags.includes("render=canvas")) el.renderMode = "canvas";
      this.ctx.pendingEditFlags = null;
    }

    this.ctx.elements.push(el);

    // If not self-closing, push to parent stack and check for same-line text content + closing tag
    if (!attrs.isSelfClosing) {
      this.ctx.viewParentStack.push(el.id);

      // Check for text content between > and </ on same line
      let textTokens: string[] = [];
      while (
        !this.stream.isEof() &&
        this.stream.peek().kind !== "NEWLINE" &&
        this.stream.peek().kind !== "BRACE_CLOSE"
      ) {
        if (
          this.stream.peek().kind === "ANGLE_OPEN" &&
          this.stream.peek(1)?.kind === "SLASH"
        ) {
          // Closing tag </tagname> on same line — consume it and pop parent
          this.stream.consume(); // ANGLE_OPEN
          this.stream.consume(); // SLASH
          this.stream.consume(); // IDENTIFIER
          if (this.stream.peek().kind === "ANGLE_CLOSE") {
            this.stream.consume();
          }
          this.ctx.viewParentStack.pop();

          // Set text content if we collected any
          const textContent = textTokens.join("").trim();
          if (textContent && !el.props.label) {
            el.props.label = textContent;
          }
          return;
        }
        // Collect token as text content
        textTokens.push(this.stream.consume().text);
      }

      // If no closing tag on same line, set text content from any collected text
      const textContent = textTokens.join("").trim();
      if (textContent && !el.props.label) {
        el.props.label = textContent;
      }
    }
  }

  /**
   * Parse view element attributes from the token stream.
   * Handles:
   *   id="value"
   *   class: name1 name2
   *   @event={handler}
   *   style: { key: val, ... }
   *   name="value"
   *   bind:name={value}
   * Also detects self-closing tags.
   */
  private parseViewAttributes(): ViewElementAttrs {
    const result: ViewElementAttrs = {
      id: null,
      className: "",
      attrs: {},
      events: {},
      styleStr: "",
      isSelfClosing: false,
    };

    while (!this.stream.isEof()) {
      const tok = this.stream.peek();

      // End of tag
      if (tok.kind === "ANGLE_CLOSE") {
        this.stream.consume();
        break;
      }

      if (tok.kind === "SELF_CLOSING") {
        this.stream.consume();
        result.isSelfClosing = true;
        break;
      }

      // Event binding: @name={handler}
      if (tok.kind === "AT") {
        this.stream.consume();
        const eventName = this.stream.expect("IDENTIFIER").text;
        this.stream.expect("EQUALS");
        this.stream.expect("BRACE_OPEN");
        const handlerName = this.stream.expect("IDENTIFIER").text;
        this.stream.expect("BRACE_CLOSE");
        result.events[eventName] = handlerName;
        continue;
      }

      // Identifier-based attributes
      if (tok.kind === "IDENTIFIER") {
        const attrName = this.stream.consume().text;

        // bind: prefix
        if (attrName === "bind") {
          this.stream.expect("COLON");
          const bindName = this.stream.expect("IDENTIFIER").text;
          this.stream.expect("EQUALS");
          this.stream.expect("BRACE_OPEN");
          const bindValue = this.stream.expect("IDENTIFIER").text;
          this.stream.expect("BRACE_CLOSE");
          result.attrs[`bind:${bindName}`] = bindValue;
          continue;
        }

        // There must be EQUALS or COLON after the attribute name
        const next = this.stream.peek();
        if (next.kind === "EQUALS") {
          this.stream.consume();

          // Check if this is an identifier-value (non-string) or string
          const valTok = this.stream.peek();

          // Stop at end of tag tokens
          if (valTok.kind === "ANGLE_CLOSE" || valTok.kind === "SELF_CLOSING") {
            break;
          }

          if (valTok.kind === "BRACE_OPEN") {
            // {value} binding syntax
            this.stream.consume();
            const v = this.stream.expect("IDENTIFIER").text;
            this.stream.expect("BRACE_CLOSE");
            if (attrName === "id") {
              result.id = v;
            } else if (attrName !== "class") {
              result.attrs[attrName] = v;
            }
            continue;
          }

          if (valTok.kind === "STRING") {
            const strVal = this.stream.consume().text.slice(1, -1); // strip quotes
            if (attrName === "id") {
              result.id = strVal;
            } else if (attrName !== "class") {
              result.attrs[attrName] = strVal;
            }
            continue;
          }

          // Fallback: consume tokens as value
          let value = "";
          while (
            !this.stream.isEof() &&
            this.stream.peek().kind !== "ANGLE_CLOSE" &&
            this.stream.peek().kind !== "SELF_CLOSING" &&
            this.stream.peek().kind !== "NEWLINE"
          ) {
            value += this.stream.consume().text;
          }
          if (attrName === "id") {
            result.id = value;
          } else if (attrName !== "class") {
            result.attrs[attrName] = value;
          }
          continue;
        }

        if (next.kind === "COLON") {
          this.stream.consume();

          // style: { ... }
          if (attrName === "style") {
            result.styleStr = this.parseStyleAttribute();
            continue;
          }

          // class: name1 name2 ...
          if (attrName === "class") {
            result.className = this.parseClassName();
            continue;
          }

          // Other colon-prefixed attributes (e.g. bind: is handled above, but
          // there could be others like `prop: value` syntax)
          let value = "";
          while (
            !this.stream.isEof() &&
            this.stream.peek().kind !== "ANGLE_CLOSE" &&
            this.stream.peek().kind !== "SELF_CLOSING" &&
            this.stream.peek().kind !== "NEWLINE" &&
            this.stream.peek().kind !== "AT" &&
            !(this.stream.peek().kind === "IDENTIFIER" && isAttrBound(this.stream.peek(1)))
          ) {
            value += this.stream.consume().text;
          }
          result.attrs[`${attrName}:`] = value.trim();
          continue;
        }
      }

      // Unknown token — stop
      break;
    }

    return result;
  }

  /**
   * Parse a `style: { key: val, key2: "val2" }` attribute.
   * Returns the reconstructed style string in the format the old parser used:
   * "{ key: val; key2: val2 }" (comma → semicolon, quotes stripped).
   */
  private parseStyleAttribute(): string {
    this.stream.expect("BRACE_OPEN");
    const styleTokens: Token[] = [];
    let depth = 1;
    while (depth > 0 && !this.stream.isEof()) {
      const t = this.stream.consume();
      if (t.kind === "BRACE_OPEN") depth++;
      else if (t.kind === "BRACE_CLOSE") depth--;
      if (depth > 0) styleTokens.push(t);
    }

    // Reconstruct style content from tokens
    const styleContent = styleTokens.map((t) => t.text).join("").trim();

    // Same conversion as the old parser: comma → semicolon, strip quotes from values
    const semiStyle = styleContent
      .split(",")
      .map((part) => part.trim())
      .filter(Boolean)
      .map((part) => part.replace(/:\s*"([^"]*)"/g, ": $1"))
      .join("; ");

    return `{ ${semiStyle} }`;
  }

  /**
   * Parse class names after `class:` — collects identifier fragments
   * joined by hyphens and spaces.
   */
  private parseClassName(): string {
    const classes: string[] = [];

    while (!this.stream.isEof()) {
      // Skip any stray whitespace text
      while (this.stream.peek()?.kind === "TEXT" && /^\s+$/.test(this.stream.peek()!.text)) {
        this.stream.consume();
      }

      const t = this.stream.peek();

      if (t.kind === "IDENTIFIER") {
        // Stop if this identifier is the start of another attribute
        if (isAttrBound(this.stream.peek(1))) {
          break;
        }

        // Start a class name
        let name = this.stream.consume().text;

        // Handle hyphenated continuation: my-class
        while (
          this.stream.peek()?.kind === "TEXT" &&
          this.stream.peek()?.text === "-"
        ) {
          this.stream.consume(); // consume "-"
          if (this.stream.peek()?.kind === "IDENTIFIER") {
            name += "-" + this.stream.consume().text;
          } else {
            break;
          }
        }

        classes.push(name);
      } else if (t.kind === "TEXT" && t.text === "-") {
        // Stray hyphen — skip
        this.stream.consume();
      } else {
        // End of class list — stop at end-of-tag tokens
        break;
      }
    }

    return classes.join(" ");
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

  // ── Helpers ──

  private skipUntilSemicolon(): void {
    while (
      !this.stream.isEof() &&
      this.stream.peek().kind !== "SEMICOLON" &&
      this.stream.peek().kind !== "NEWLINE"
    ) {
      this.stream.consume();
    }
  }

  private pushError(
    severity: string,
    code: string,
    message: string,
    tok: Token,
  ): void {
    this.ctx.errors.push({
      severity,
      code,
      message,
      line: tok.line,
      col: tok.col,
      end_line: tok.line,
      end_col: tok.col + tok.text.length,
    });
  }

  /**
   * Create an ElementData for a view element with parent inference by indent.
   */
  private createElement(
    id: string,
    tag: string,
    attrs: ViewElementAttrs,
    indent: number,
  ): ElementData {
    // Auto-rename duplicate IDs
    let finalId = id;
    if (this.ctx.elements.some((e) => e.id === finalId)) {
      log.warn(`Duplicate element ID "${finalId}"; assigning unique ID`);
      let counter = 1;
      let newId = `${finalId}__dup${counter}`;
      while (this.ctx.elements.some((e) => e.id === newId)) {
        counter++;
        newId = `${finalId}__dup${counter}`;
      }
      finalId = newId;
    }

    // Parent inference by indent (same as old parser)
    while (this.ctx.viewParentStack.length > 0) {
      const parentId =
        this.ctx.viewParentStack[this.ctx.viewParentStack.length - 1];
      const parent = this.ctx.elements.find((e) => e.id === parentId);
      if (
        parent &&
        this.getElementIndent(parent) !== undefined &&
        this.getElementIndent(parent)! >= indent
      ) {
        this.ctx.viewParentStack.pop();
      } else {
        break;
      }
    }

    const parentId =
      this.ctx.viewParentStack.length > 0
        ? this.ctx.viewParentStack[this.ctx.viewParentStack.length - 1]
        : null;

    // Detect custom component references
    const isPascal = /^[A-Z]/.test(tag);
    const isCustom =
      isPascal && !!useComponentRegistry.getState().definitions[tag];

    const el: ElementData = {
      id: finalId,
      nodeType: "view",
      tag,
      props: {
        ...attrs.attrs,
        ...attrs.events,
        ...(attrs.className ? { className: attrs.className } : {}),
      },
      children: [],
      parentId,
      ...(isCustom ? { componentRef: tag } : {}),
    };

    // Set parent/child relationships
    if (parentId) {
      const parent = this.ctx.elements.find((e) => e.id === parentId);
      if (parent) parent.children.push(finalId);
    }

    // Store indent on the element (for parent inference)
    this.setElementIndent(el, indent);

    // Handle inline style if present
    if (attrs.styleStr) {
      el.props.style = attrs.styleStr;
    }

    return el;
  }

  // ── Element indent tracking (replaces old WeakMap) ──

  private indentMap = new Map<string, number>();

  private getElementIndent(el: ElementData): number | undefined {
    return this.indentMap.get(el.id);
  }

  private setElementIndent(el: ElementData, indent: number): void {
    this.indentMap.set(el.id, indent);
  }

  // ── Chain flush ──

  private flushChain(): void {
    if (this.ctx.chainElements.length === 0) return;
    const tab =
      this.ctx.codeTabs.find((t) => t.id === "main") || this.ctx.codeTabs[0];
    for (const el of this.ctx.chainElements) tab.graph.elements.push(el);
    for (const w of this.ctx.chainWires) tab.graph.wires.push(w);
    this.ctx.chainElements = [];
    this.ctx.chainWires = [];
  }

  // ── Codeblock element factory ──

  private codeblockEl(
    id: string,
    tag: string,
    props: Record<string, unknown>,
  ): ElementData {
    return {
      id,
      nodeType: "codeblock",
      tag,
      props: props as Record<string, unknown>,
      children: [],
      parentId: null,
    };
  }

  // ── Finalization (post-processing) ──

  private finalize(): void {
    this.flushChain();

    // Parse styles from captured AST (style blocks + view tree).
    // Same logic as the old parser.
    if (this.ctx.elements.length > 0) {
      function buildViewTreeFromElements(
        el: ElementData,
        elements: ElementData[],
      ): any {
        let inline_style: Record<string, string> | null = null;
        const styleStr = el.props.style as string | undefined;
        if (styleStr && typeof styleStr === "string") {
          const cleaned = styleStr
            .replace(/^\s*\{\s*/, "")
            .replace(/\s*\}\s*$/, "");
          if (cleaned.length > 0) {
            const pairs: Record<string, string> = {};
            for (const part of cleaned.split(";")) {
              const t = part.trim();
              if (t.length === 0) continue;
              const colonIdx = t.indexOf(":");
              if (colonIdx > 0) {
                pairs[t.slice(0, colonIdx).trim()] = t.slice(colonIdx + 1).trim();
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
            .map((cid: string) =>
              elements.find((e: ElementData) => e.id === cid),
            )
            .filter((c: ElementData | undefined): c is ElementData => c !== undefined)
            .map((c: ElementData) => buildViewTreeFromElements(c, elements)),
        };
      }

      const rootElements = this.ctx.elements.filter(
        (e: ElementData) => e.nodeType === "view" && !e.parentId,
      );
      if (rootElements.length > 0) {
        const elementMap = new Map(
          this.ctx.elements.map((e: ElementData) => [e.id, e]),
        );
        const componentNode = {
          style: this.ctx.hasStyleBlock
            ? this.ctx.parsedStyleRules
            : undefined,
          view: buildViewTreeFromElements(rootElements[0], this.ctx.elements),
        };
        parseStylesFromAST(componentNode, elementMap);
      }
    }
  }

  // ── Build result ──

  private buildResult(): ParseResult {
    return {
      designElements:
        this.ctx.elements.length > 0 ? this.ctx.elements : null,
      designWires: this.ctx.wires,
      codeTabs: this.ctx.codeTabs,
      imports: this.ctx.imports,
      errors: this.ctx.errors,
      componentMeta: this.ctx.componentMeta,
      props: this.ctx.props.length > 0 ? this.ctx.props : undefined,
      events: this.ctx.events.length > 0 ? this.ctx.events : undefined,
    };
  }
}

// ── Parsed attribute shape ──

interface ViewElementAttrs {
  id: string | null;
  className: string;
  attrs: Record<string, string>;
  events: Record<string, string>;
  styleStr: string;
  isSelfClosing: boolean;
}

// ── Utility functions ──

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

/**
 * Check if a token looks like the start of a next attribute binding
 * (attribute + EQUALS or COLON on the very next token).
 */
function isAttrBound(next: Token): boolean {
  return next?.kind === "EQUALS" || next?.kind === "COLON";
}
