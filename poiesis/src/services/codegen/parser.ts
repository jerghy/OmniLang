// poiesis/src/services/codegen/parser.ts
// Recursive-descent Parser class for OmniLang sources.

import type { Token } from "./omni-tokenizer";
import { TokenStream } from "./token-stream";
import type { ParseResult } from "./types";
import type { CodeblockElementData } from "../../core/types/element";
import type { WireData } from "../../core/types";
import { parseCodeFlowGraph } from "./graph-parser";
import type {
  ParserState,
  ParseContext,
  ParsedClassProperty,
  ParsedPseudoState,
  ParsedDimension,
  ParsedDimensionValue,
  ParsedBranch,
} from "./parser-types";
import { parseKeyValueAttrs } from "./parser-types";
import { parseDesignAnnotation } from "../../core/types/design";
import { uid } from "../../core/utils/id";
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
  private parsedFnBodies = new Map<string, { elements: CodeblockElementData[]; wires: WireData[] }>();

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
      currentStorePid: "",
      storeDef: null,
      inlineStoreDefs: {} as Record<string, import("../../core/types/store").StoreDefinition>,
      _parsingInlineStore: false,
      storeSignalCounter: 0,
      storeActionCounter: 0,
      storeGetterCounter: 0,
      parsedStorePid: null,
      designName: "",
      designTokens: [],
      designClasses: [],
      designVariants: [],
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
      this.ctx.currentStorePid = ""; // will be filled by @pid comment or auto-generated
      this.ctx.storeSignalCounter = 0;
      this.ctx.storeActionCounter = 0;
      this.ctx.storeGetterCounter = 0;
      this.ctx.parsedStorePid = null;
      this.stream.expect("BRACE_OPEN");
      this.pushState("IN_STORE");
      return;
    }

    if (kw === "design") {
      this.stream.consume(); // "design"
      const nameTok = this.stream.expect("IDENTIFIER");
      if (nameTok.kind === "EOF") {
        this.pushError("error", "P004", "Invalid design declaration: missing name", nameTok);
        return;
      }
      this.ctx.designName = nameTok.text;
      this.stream.expect("BRACE_OPEN");
      this.pushState("IN_DESIGN");
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
      case "store":
        this.stream.consume(); // "store"
        const storeNameTok = this.stream.expect("IDENTIFIER");
        if (storeNameTok.kind === "EOF") {
          this.pushError("error", "P005", "Invalid inline store declaration: missing name", storeNameTok);
          break;
        }
        this.ctx.currentStore = storeNameTok.text;
        this.ctx.currentStorePid = "";
        this.ctx.storeSignalCounter = 0;
        this.ctx.storeActionCounter = 0;
        this.ctx.storeGetterCounter = 0;
        this.ctx.parsedStorePid = null;
        // Mark BEFORE pushState — pushState synchronously calls parseStoreBody
        this.ctx._parsingInlineStore = true;
        this.stream.expect("BRACE_OPEN");
        this.pushState("IN_STORE");
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

  // ── Fn body (parse into code flow graph) ──

  private parseFn(): void {
    this.stream.consume(); // "fn"

    // Collect fn signature tokens until {
    let fnSignature = "";
    while (!this.stream.isEof()) {
      const tok = this.stream.peek();
      if (tok.kind === "BRACE_OPEN") {
        this.stream.consume();
        break;
      }
      fnSignature += tok.text + " ";
      this.stream.consume();
    }

    // Extract fn name from signature
    const sigMatch = fnSignature.match(/(\w+)\s*\(([^)]*)\)/);
    const fnName = sigMatch?.[1] || "unnamed";

    // Collect fn body with brace counting, preserving // @pid:xxx comments
    let fnBody = "";
    let depth = 1;
    while (depth > 0 && !this.stream.isEof()) {
      const tok = this.stream.peek();
      if (tok.kind === "BRACE_OPEN") { depth++; fnBody += "{\n"; }
      else if (tok.kind === "BRACE_CLOSE") { depth--; if (depth > 0) fnBody += "}\n"; }
      else { fnBody += tok.text + " "; }
      this.stream.consume();
    }

    // Parse fn body into code flow graph
    const { elements, wires } = parseCodeFlowGraph(fnBody);

    this.parsedFnBodies.set(fnName, { elements, wires });
  }

  // ── Store body (parse signal/getter/action members into StoreDefinition) ──

  private parseStoreBody(): void {
    const storeName = this.ctx._parsingInlineStore
      ? `${this.ctx.currentComponent}.${this.ctx.currentStore}`
      : this.ctx.currentStore;
    const storeDef: import("../../core/types/store").StoreDefinition = {
      pid: this.ctx.currentStorePid || `s${this.ctx.storeDef ? Object.keys(this.ctx.storeDef).length + 1 : 1}`,
      name: storeName,
      filePath: "",
      imports: [],
      signals: [],
      getters: [],
      actions: [],
    };
    let pendingPid: string | null = this.ctx.parsedStorePid;

    while (!this.stream.isEof()) {
      this.stream.skipNewlines();
      if (this.stream.isEof()) break;

      const tok = this.stream.peek();

      // Close store
      if (tok.kind === "BRACE_CLOSE") {
        this.stream.consume();
        if (this.ctx._parsingInlineStore) {
          // Inline store inside a component
          // storeDef.name is already qualified as "ComponentName.StoreName"
          this.ctx.inlineStoreDefs = { ...this.ctx.inlineStoreDefs, [storeDef.name]: storeDef };
          this.ctx._parsingInlineStore = false;
        } else {
          this.ctx.storeDef = storeDef;
        }
        this.popState();
        return;
      }

      // Parse @pid comment
      if (tok.kind === "COMMENT_LINE" && tok.text.trim().startsWith("// @pid:")) {
        pendingPid = tok.text.trim().slice(8).trim();
        this.stream.consume();
        continue;
      }

      // Parse import
      if (tok.kind === "IDENTIFIER" && tok.text === "import") {
        const importLine = this.collectUntilNewline();
        storeDef.imports.push("import" + importLine);
        continue;
      }

      // Parse signal
      if (tok.kind === "IDENTIFIER" && tok.text === "signal") {
        this.stream.consume(); // "signal"
        const nameTok = this.stream.expect("IDENTIFIER");
        this.stream.expect("COLON");
        const typeStr = this.collectType();
        let defaultValue: unknown = undefined;
        const next = this.stream.peek();
        if (next.kind === "EQUALS") {
          this.stream.consume(); // "="
          defaultValue = this.collectDefaultValue();
        }
        this.expectSemicolon();
        const pid = pendingPid || `${storeDef.pid}_n${++this.ctx.storeSignalCounter}`;
        pendingPid = null;
        storeDef.signals.push({ pid, name: nameTok.text, type: typeStr, defaultValue });
        continue;
      }

      // Parse getter
      if (tok.kind === "IDENTIFIER" && tok.text === "getter") {
        this.stream.consume(); // "getter"
        const nameTok = this.stream.expect("IDENTIFIER");
        this.stream.expect("COLON");
        const returnType = this.collectType();
        this.stream.expect("EQUALS");
        const expression = this.collectExpressionUntilSemicolon();
        this.expectSemicolon();
        const pid = pendingPid || `${storeDef.pid}_n${++this.ctx.storeGetterCounter}`;
        pendingPid = null;
        storeDef.getters.push({ pid, name: nameTok.text, returnType, expression });
        continue;
      }

      // Parse action
      if (tok.kind === "IDENTIFIER" && tok.text === "action") {
        this.stream.consume(); // "action"
        const nameTok = this.stream.expect("IDENTIFIER");
        this.stream.expect("PAREN_OPEN");
        const params = this.collectActionParams();
        this.stream.expect("PAREN_CLOSE");
        let returnType: string | undefined;
        let isAsync = false;
        // Optional ": ReturnType"
        if (this.stream.peek().kind === "COLON") {
          this.stream.consume(); // ":"
          returnType = this.collectType();
        }
        // Optional "async"
        if (this.stream.peek().kind === "IDENTIFIER" && this.stream.peek().text === "async") {
          this.stream.consume();
          isAsync = true;
        }
        this.stream.expect("BRACE_OPEN");
        // Collect action body (until matching close brace)
        const actionBody = this.collectActionBody();
        const { elements, wires } = parseCodeFlowGraph(actionBody);
        const pid = pendingPid || `${storeDef.pid}_n${++this.ctx.storeActionCounter}`;
        pendingPid = null;
        storeDef.actions.push({ pid, name: nameTok.text, params, returnType, isAsync, elements, wires });
        continue;
      }

      // Skip unknown tokens
      this.stream.consume();
    }
  }

  // ── Store body helpers (Task 4) ──

  /** Collect tokens until NEWLINE — used for import lines. */
  private collectUntilNewline(): string {
    let result = "";
    while (!this.stream.isEof() && this.stream.peek().kind !== "NEWLINE") {
      result += this.stream.consume().text;
    }
    return result;
  }

  /** Collect a type annotation string (e.g. "String", "Todo[]", "Array<String>"). */
  private collectType(): string {
    let typeStr = "";
    let depth = 0;
    while (!this.stream.isEof()) {
      const t = this.stream.peek();
      if (
        depth === 0 && (
          t.kind === "SEMICOLON" ||
          t.kind === "EQUALS" ||
          t.kind === "COLON" ||
          t.kind === "NEWLINE" ||
          t.kind === "BRACE_CLOSE" ||
          t.kind === "BRACE_OPEN" ||
          t.kind === "COMMA" ||
          t.kind === "PAREN_CLOSE" ||
          t.kind === "PAREN_OPEN" ||
          (t.kind === "IDENTIFIER" && t.text === "async")
        )
      ) break;
      if (t.kind === "ANGLE_OPEN") depth++;
      if (t.kind === "ANGLE_CLOSE") depth--;
      typeStr += this.stream.consume().text;
    }
    return typeStr.trim();
  }

  /** Collect a default value expression after "=", stopping at ";". */
  private collectDefaultValue(): unknown {
    let val = "";
    while (!this.stream.isEof() && this.stream.peek().kind !== "SEMICOLON") {
      val += this.stream.consume().text;
    }
    const trimmed = val.trim();
    // Return as string (caller can JSON.parse if needed)
    return trimmed;
  }

  /** Collect an expression until ";" (possibly multi-line). */
  private collectExpressionUntilSemicolon(): string {
    let expr = "";
    while (!this.stream.isEof() && this.stream.peek().kind !== "SEMICOLON") {
      expr += this.stream.consume().text;
    }
    return expr.trim();
  }

  /** Expect and consume a semicolon. */
  private expectSemicolon(): void {
    if (this.stream.peek().kind === "SEMICOLON") {
      this.stream.consume();
    }
  }

  /** Collect action params inside "(...)", comma-separated. */
  private collectActionParams(): Array<{ name: string; type: string }> {
    const params: Array<{ name: string; type: string }> = [];
    while (!this.stream.isEof() && this.stream.peek().kind !== "PAREN_CLOSE") {
      const name = this.stream.expect("IDENTIFIER").text;
      this.stream.expect("COLON");
      const pType = this.collectType();
      params.push({ name, type: pType });
      if (this.stream.peek().kind === "COMMA") {
        this.stream.consume(); // ","
      }
    }
    return params;
  }

  /** Collect action body between braces, preserving @pid comments for parseCodeFlowGraph. */
  private collectActionBody(): string {
    let body = "";
    let depth = 1;
    while (depth > 0 && !this.stream.isEof()) {
      const tok = this.stream.consume();
      if (tok.kind === "BRACE_OPEN") depth++;
      else if (tok.kind === "BRACE_CLOSE") { depth--; if (depth === 0) break; }
      if (tok.kind === "NEWLINE") body += "\n";
      else body += tok.text + (tok.kind === "COMMENT_LINE" ? "" : " ");
    }
    return body.trim();
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
            source: "",
            position: { x: posX, y: posY },
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

  // ── Design block ──

  private parseDesignBody(): void {
    let lastAnnotation: { type: "token" | "class" | "variant"; id: string } | null = null;

    while (!this.stream.isEof()) {
      this.stream.skipNewlines();

      if (this.stream.isEof()) break;

      const tok = this.stream.peek();

      // Close design block
      if (tok.kind === "BRACE_CLOSE") {
        this.stream.consume();
        this.popState();
        return;
      }

      // Annotation comments giving stable IDs
      if (tok.kind === "COMMENT_LINE") {
        this.stream.consume();
        const text = tok.text.trim();
        const ann = parseDesignAnnotation(text);
        if (ann) {
          lastAnnotation = ann;
        }
        continue;
      }

      if (tok.kind === "IDENTIFIER") {
        const kw = tok.text;

        if (kw === "token") {
          this.parseDesignToken(lastAnnotation);
          lastAnnotation = null;
        } else if (kw === "class") {
          this.parseDesignClass(lastAnnotation);
          lastAnnotation = null;
        } else if (kw === "variant") {
          this.parseDesignVariant(lastAnnotation);
          lastAnnotation = null;
        } else {
          // Unknown keyword inside design block — skip
          this.stream.consume();
        }
      } else {
        // Skip unexpected tokens
        this.stream.consume();
      }
    }
  }

  private parseDesignToken(
    annotation: { type: "token" | "class" | "variant"; id: string } | null,
  ): void {
    this.stream.consume(); // "token"
    const nameTok = this.stream.expect("IDENTIFIER");
    if (nameTok.kind === "EOF") return;
    this.stream.expect("COLON");
    const typeTok = this.stream.expect("IDENTIFIER");
    if (typeTok.kind === "EOF") return;
    // Optional "=" value
    let value = "";
    if (this.stream.peek().kind === "EQUALS") {
      this.stream.consume();
      const valueTok = this.stream.consume();
      value = valueTok.kind === "STRING" ? valueTok.text.slice(1, -1) : valueTok.text;
    }
    // Expect semicolon
    if (this.stream.peek().kind === "SEMICOLON") {
      this.stream.consume();
    }

    const id = annotation?.id || uid("tk");
    this.ctx.designTokens.push({
      id,
      name: nameTok.text,
      tokenType: typeTok.text,
      value,
    });
  }

  private parseDesignClass(
    annotation: { type: "token" | "class" | "variant"; id: string } | null,
  ): void {
    this.stream.consume(); // "class"
    const nameTok = this.stream.expect("IDENTIFIER");
    if (nameTok.kind === "EOF") return;
    this.stream.expect("BRACE_OPEN");

    const properties: ParsedClassProperty[] = [];
    const pseudoStates: ParsedPseudoState[] = [];

    this.parseClassBody(properties, pseudoStates);

    const id = annotation?.id || uid("cls");
    this.ctx.designClasses.push({
      id,
      name: nameTok.text,
      properties,
      pseudoStates,
    });
  }

  private parseClassBody(
    props: ParsedClassProperty[],
    pseudos: ParsedPseudoState[],
  ): void {
    while (!this.stream.isEof()) {
      this.stream.skipNewlines();
      if (this.stream.peek().kind === "BRACE_CLOSE") {
        this.stream.consume();
        return;
      }
      // Detect pseudo-state: either ":hover" or "&:hover"
      if (
        this.stream.peek().kind === "COLON" ||
        (this.stream.peek().kind === "TEXT" && this.stream.peek().text === "&")
      ) {
        pseudos.push(this.parsePseudoState());
      } else if (this.stream.peek().kind === "IDENTIFIER") {
        const prop = this.parseProperty();
        if (prop) props.push(prop);
      } else {
        // COMMENT_LINE or other unexpected tokens — skip
        this.stream.consume();
      }
    }
  }

  private parsePseudoState(): ParsedPseudoState {
    // Handle optional "&" prefix (for "&:hover" syntax)
    if (this.stream.peek().kind === "TEXT" && this.stream.peek().text === "&") {
      this.stream.consume();
    }
    this.stream.consume(); // consume ':'
    const name = this.stream.expect("IDENTIFIER").text;
    this.stream.skipNewlines();
    this.stream.expect("BRACE_OPEN");

    const props: ParsedClassProperty[] = [];
    while (!this.stream.isEof()) {
      this.stream.skipNewlines();
      if (this.stream.peek().kind === "BRACE_CLOSE") {
        this.stream.consume();
        return { pseudo: name, properties: props };
      }
      const prop = this.parseProperty();
      if (prop) props.push(prop);
    }
    return { pseudo: name, properties: props };
  }

  private parseProperty(): ParsedClassProperty | null {
    if (this.stream.peek().kind !== "IDENTIFIER") {
      this.stream.consume();
      return null;
    }
    const name = this.stream.consume().text;
    if (this.stream.peek().kind !== "COLON") {
      this.skipLine();
      return null;
    }
    this.stream.consume(); // ':'
    const value = this.readLineValue();
    return { name, value };
  }

  private readLineValue(): string {
    let result = "";
    let lastToken: import("./omni-tokenizer").Token | null = null;
    while (!this.stream.isEof()) {
      const tok = this.stream.peek();
      if (tok.kind === "NEWLINE" || tok.kind === "BRACE_CLOSE") break;
      if (tok.kind === "SEMICOLON") {
        this.stream.consume();
        break;
      }
      this.stream.consume();
      if (lastToken) {
        const gap = tok.col - (lastToken.col + lastToken.text.length);
        if (gap > 0) result += " ".repeat(gap);
      }
      result += tok.text;
      lastToken = tok;
    }
    return result;
  }

  private skipLine(): void {
    while (!this.stream.isEof()) {
      const tok = this.stream.peek();
      if (
        tok.kind === "NEWLINE" ||
        tok.kind === "BRACE_CLOSE" ||
        tok.kind === "EOF"
      ) {
        return;
      }
      this.stream.consume();
    }
  }

  private parseDesignVariant(
    annotation: { type: "token" | "class" | "variant"; id: string } | null,
  ): void {
    this.stream.consume(); // "variant"
    const nameTok = this.stream.expect("IDENTIFIER");
    if (nameTok.kind === "EOF") return;
    this.stream.expect("BRACE_OPEN");

    const dimensions: ParsedDimension[] = [];
    const baseProperties: ParsedClassProperty[] = [];
    const branches: ParsedBranch[] = [];

    this.parseVariantBody(dimensions, baseProperties, branches);

    // Build defaults map from dimension values marked (default)
    const defaults: Record<string, string> = {};
    for (const dim of dimensions) {
      for (const val of dim.values) {
        if (val.isDefault) defaults[dim.name] = val.name;
      }
    }

    const id = annotation?.id || uid("var");
    this.ctx.designVariants.push({
      id,
      name: nameTok.text,
      dimensions,
      baseProperties,
      branches,
      defaults,
    });
  }

  private parseVariantBody(
    dims: ParsedDimension[],
    baseProps: ParsedClassProperty[],
    branches: ParsedBranch[],
  ): void {
    while (!this.stream.isEof()) {
      this.stream.skipNewlines();
      if (this.stream.peek().kind === "BRACE_CLOSE") {
        this.stream.consume();
        return;
      }

      const tok = this.stream.peek();

      // Handle @dim and @default annotations
      if (tok.kind === "AT") {
        this.stream.consume(); // '@'
        const kwTok = this.stream.peek();
        if (kwTok.kind === "IDENTIFIER") {
          const kw = kwTok.text;
          this.stream.consume(); // keyword after @
          if (kw === "dim") {
            dims.push(this.parseAtDimBlock());
          } else if (kw === "default") {
            this.parseDefaultBlock(baseProps, dims);
          } else {
            this.skipLine();
          }
        } else {
          this.skipLine();
        }
        continue;
      }

      // Handle bare identifiers: "base", or dimension.value branch notation
      if (tok.kind === "IDENTIFIER") {
        const kw = tok.text;
        this.stream.consume();

        if (kw === "base") {
          this.parsePropertyBlock(baseProps);
        } else if (
          this.stream.peek().kind === "DOT"
        ) {
          // dimension.value { ... } branch notation
          this.stream.consume(); // '.'
          const valTok = this.stream.expect("IDENTIFIER");
          if (valTok.kind === "EOF") continue;
          const branch = this.parseBranchProperties();
          branch.name = `${kw}.${valTok.text}`;
          branches.push(branch);
        } else {
          this.skipLine();
        }
        continue;
      }

      // Skip unexpected tokens
      this.stream.consume();
    }
  }

  /**
   * Parse `@dim name: value1 | value2 | ... ;` annotation.
   * Values are pipe-separated; the first value before `|` is the default.
   */
  private parseAtDimBlock(): ParsedDimension {
    const nameTok = this.stream.expect("IDENTIFIER");
    const name = nameTok.text;
    // Expect colon
    if (this.stream.peek().kind === "COLON") {
      this.stream.consume();
    }
    const values: ParsedDimensionValue[] = [];
    let isFirst = true;
    while (!this.stream.isEof()) {
      this.stream.skipNewlines();
      const t = this.stream.peek();
      if (t.kind === "SEMICOLON" || t.kind === "NEWLINE" || t.kind === "BRACE_CLOSE") {
        if (t.kind === "SEMICOLON") this.stream.consume();
        break;
      }
      // Skip pipe separators
      if (t.kind === "PIPE") {
        this.stream.consume();
        continue;
      }
      if (t.kind === "IDENTIFIER") {
        const valName = this.stream.consume().text;
        values.push({ name: valName, isDefault: isFirst });
        isFirst = false;
      } else {
        this.stream.consume();
      }
    }
    return { name, values };
  }

  /**
   * Parse `@default { key: value; ... }` annotation.
   * Stores resolved default map directly on the last dimension's values.
   */
  private parseDefaultBlock(
    baseProps: ParsedClassProperty[],
    dims: ParsedDimension[],
  ): void {
    if (this.stream.peek().kind === "BRACE_OPEN") {
      this.stream.consume(); // '{'
      while (!this.stream.isEof()) {
        this.stream.skipNewlines();
        if (this.stream.peek().kind === "BRACE_CLOSE") {
          this.stream.consume();
          return;
        }
        const prop = this.parseProperty();
        if (prop) {
          // Mark the matching dimension value as default
          for (const dim of dims) {
            for (const val of dim.values) {
              if (val.name === prop.value) {
                val.isDefault = true;
              }
            }
          }
        }
      }
    } else {
      this.skipLine();
    }
  }

  /** Parse properties inside `{ ... }` and return as a ParsedBranch. */
  private parseBranchProperties(): ParsedBranch {
    const props: ParsedClassProperty[] = [];
    if (this.stream.peek().kind === "BRACE_OPEN") {
      this.stream.consume();
      while (!this.stream.isEof()) {
        this.stream.skipNewlines();
        if (this.stream.peek().kind === "BRACE_CLOSE") {
          this.stream.consume();
          break;
        }
        const prop = this.parseProperty();
        if (prop) props.push(prop);
      }
    }
    return { name: "", properties: props };
  }

  private parsePropertyBlock(props: ParsedClassProperty[]): void {
    this.stream.expect("BRACE_OPEN");
    while (!this.stream.isEof()) {
      this.stream.skipNewlines();
      if (this.stream.peek().kind === "BRACE_CLOSE") {
        this.stream.consume();
        return;
      }
      const prop = this.parseProperty();
      if (prop) props.push(prop);
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
      case "IN_STORE":
        this.parseStoreBody();
        break;
      case "IN_STYLE":
        this.parseStyleBlockBody();
        break;
      case "IN_CHAIN":
        this.parseChainBody();
        break;
      case "IN_DESIGN":
        this.parseDesignBody();
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

  /** Get parsed code flow graphs keyed by fn name. */
  getParsedFnBodies(): Map<string, { elements: CodeblockElementData[]; wires: WireData[] }> {
    return this.parsedFnBodies;
  }

  // ── Delegation wrappers for extracted methods ──

  private skipUntilSemicolon(): void {
    ParserHelpers.skipUntilSemicolon(this.stream);
  }

  private pushError(
    severity: "error" | "warning" | "info",
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
    const result = ParserFinalize.buildParseResult(this.ctx);
    if (this.parsedFnBodies.size > 0) {
      const entries: Record<string, { elements: CodeblockElementData[]; wires: WireData[] }> = {};
      for (const [key, value] of this.parsedFnBodies) {
        entries[key] = value;
      }
      result.codeFlowGraphs = entries;
    }
    return result;
  }
}
