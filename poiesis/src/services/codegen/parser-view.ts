// poiesis/src/services/codegen/parser-view.ts
// View parsing functions extracted from the Parser class.

import type { Token } from "./omni-tokenizer";
import type { ElementData, ViewElementData } from "../../core/types/element";
import type { ParserInternals } from "./parser";
import type { ViewElementAttrs } from "./parser-types";
import { isAttrBound } from "./parser-types";
import { createElement } from "./parser-helpers";
import { parsePidAnnotation } from "./annotation-registry";
import { BOOLEAN_ATTRS } from "./boolean-attrs";
import { semanticEventName } from "./event-name-map";

// ── View entry ──

export function parseView(internals: ParserInternals): void {
  internals.stream.consume(); // "view"
  internals.stream.expect("BRACE_OPEN");
  internals.ctx.viewParentStack = [];
  // Inline pushState for IN_VIEW
  internals.ctx.stateStack.push(internals.ctx.state);
  internals.ctx.braceDepthStack.push(internals.ctx.braceDepth);
  internals.ctx.state = "IN_VIEW";
  internals.ctx.braceDepth = 1;
  parseViewBody(internals);
}

// ── View body ──

export function parseViewBody(internals: ParserInternals): void {
  internals.stream.skipNewlines();

  while (!internals.stream.isEof()) {
    const tok = internals.stream.peek();

    // End of view block
    if (tok.kind === "BRACE_CLOSE") {
      internals.stream.consume();
      internals.ctx.state = internals.ctx.stateStack.pop() || "ROOT";
      internals.ctx.braceDepth = internals.ctx.braceDepthStack.pop() || 0;
      return;
    }

    // Closing tag
    if (tok.kind === "ANGLE_OPEN" && internals.stream.peek(1)?.kind === "SLASH") {
      internals.stream.consume(); // ANGLE_OPEN
      internals.stream.consume(); // SLASH
      internals.stream.consume(); // IDENTIFIER (tag name)
      // Consume optional ANGLE_CLOSE
      if (internals.stream.peek().kind === "ANGLE_CLOSE") {
        internals.stream.consume();
      }
      if (internals.ctx.viewParentStack.length > 0) {
        internals.ctx.viewParentStack.pop();
      }
      internals.stream.skipNewlines();
      continue;
    }

    // Opening tag
    if (tok.kind === "ANGLE_OPEN") {
      parseViewElement(internals);
      internals.stream.skipNewlines();
      continue;
    }

    // Comments in view
    if (tok.kind === "COMMENT_LINE") {
      parseViewComment(internals);
      internals.stream.skipNewlines();
      continue;
    }

    // Text content on its own line (including {expr} interpolation)
    if (tok.kind === "IDENTIFIER" || tok.kind === "TEXT" || tok.kind === "STRING" || tok.kind === "BRACE_OPEN") {
      parseViewTextContent(internals);
      internals.stream.skipNewlines();
      continue;
    }

    // Template directives: @if / @for / @slot
    if (tok.kind === "AT") {
      parseViewDirective(internals);
      internals.stream.skipNewlines();
      continue;
    }

    // Skip any other unexpected tokens
    internals.stream.consume();
    internals.stream.skipNewlines();
  }
}

// ── View comment ──

export function parseViewComment(internals: ParserInternals): void {
  const tok = internals.stream.consume();
  const text = tok.text.trim();

  // @pid annotation on view elements — store for later PID index building
  const pidResult = parsePidAnnotation(text);
  if (pidResult) {
    internals.ctx.pendingPid = pidResult.pid;
    return;
  }

  // @edit annotation (unchanged)
  if (text.startsWith("// @edit")) {
    internals.ctx.pendingEditFlags = text.slice("// @edit".length).trim() || null;
  }
  // Other view comments are ignored (old parser behavior)
}

// ── View text content ──

/**
 * Collect consecutive IDENTIFIER / TEXT / STRING / NUMBER tokens and reconstruct
 * text with whitespace between them. The tokenizer discards intra-line whitespace,
 * so we use column gaps to re-insert spaces that were removed during tokenization.
 */
function collectTextWithSpacing(internals: ParserInternals): string {
  let result = "";
  let lastEndCol = -1;
  let braceDepth = 0;

  while (
    !internals.stream.isEof() &&
    internals.stream.peek().kind !== "NEWLINE" &&
    internals.stream.peek().kind !== "ANGLE_OPEN"
  ) {
    const tok = internals.stream.peek();

    // 结构性 }（括号未配平，非 {expr} 插值的收尾）不属于文本内容——
    // 停在原地交给上层（parseViewBody 视为 view 结束）处理。
    if (tok.kind === "BRACE_CLOSE" && braceDepth === 0) break;

    if (tok.kind === "BRACE_OPEN") braceDepth++;
    else if (tok.kind === "BRACE_CLOSE") braceDepth--;

    // Insert spaces based on column gap between consecutive tokens
    if (lastEndCol >= 0 && tok.col > lastEndCol) {
      result += " ".repeat(tok.col - lastEndCol);
    }

    internals.stream.consume();
    result += tok.text;
    lastEndCol = tok.col + tok.text.length;
  }

  return result.trim();
}

export function parseViewTextContent(internals: ParserInternals): void {
  const text = collectTextWithSpacing(internals);

  if (text && !text.startsWith("//") && !text.startsWith("<")) {
    // Attach as label prop to the current parent element
    const parentId =
      internals.ctx.viewParentStack.length > 0
        ? internals.ctx.viewParentStack[internals.ctx.viewParentStack.length - 1]
        : null;
    if (parentId) {
      const idx = internals.ctx.elements.findIndex(e => e.id === parentId);
      if (idx !== -1 && "props" in internals.ctx.elements[idx] && !internals.ctx.elements[idx].props.label) {
        const parent = internals.ctx.elements[idx];
        internals.ctx.elements[idx] = { ...parent, props: { ...parent.props, label: text } } as ElementData;
      }
    }
  }
}

// ── View element ──

export function parseViewElement(internals: ParserInternals): void {
  const indent = internals.stream.currentIndent(); // col of first token on this line

  internals.stream.consume(); // ANGLE_OPEN (<)
  const tagTok = internals.stream.expect("IDENTIFIER");
  const tag = tagTok.text;

  // Parse attributes
  const attrs = parseViewAttributes(internals);

  // Generate element ID
  const id = attrs.id !== null
    ? attrs.id
    : `${internals.ctx.currentComponent}.${tag}.${internals.ctx.elements.length}`;

  // Check for duplicate IDs
  const el = createElement(internals, id, tag, attrs, indent);

  // @edit flags
  if (internals.ctx.pendingEditFlags) {
    const viewEl = el as ViewElementData;
    if (internals.ctx.pendingEditFlags.includes("folded")) viewEl.isFolded = true;
    if (internals.ctx.pendingEditFlags.includes("render=canvas")) viewEl.renderMode = "canvas";
    internals.ctx.pendingEditFlags = null;
  }

  internals.ctx.elements.push(el);

  // If not self-closing, push to parent stack and check for same-line text content + closing tag
  if (!attrs.isSelfClosing) {
    internals.ctx.viewParentStack.push(el.id);

    // Check for text content between > and </ on same line.
    // The tokenizer discards intra-line whitespace, so we track column gaps
    // to re-insert spaces between tokens.
    const textParts: Array<{ text: string; col: number }> = [];
    while (
      !internals.stream.isEof() &&
      internals.stream.peek().kind !== "NEWLINE"
    ) {
      if (
        internals.stream.peek().kind === "ANGLE_OPEN" &&
        internals.stream.peek(1)?.kind === "SLASH"
      ) {
        // Closing tag </tagname> on same line — consume it and pop parent
        internals.stream.consume(); // ANGLE_OPEN
        internals.stream.consume(); // SLASH
        internals.stream.consume(); // IDENTIFIER
        if (internals.stream.peek().kind === "ANGLE_CLOSE") {
          internals.stream.consume();
        }
        internals.ctx.viewParentStack.pop();

        // Set text content if we collected any, with column-gap spacing
        let textContent = "";
        let lastEndCol = -1;
        for (const p of textParts) {
          if (lastEndCol >= 0 && p.col > lastEndCol) {
            textContent += " ".repeat(p.col - lastEndCol);
          }
          textContent += p.text;
          lastEndCol = p.col + p.text.length;
        }
        textContent = textContent.trim();
        if (textContent && "props" in el) {
          const elIdx = internals.ctx.elements.findIndex(e => e.id === el.id);
          if (elIdx !== -1 && "props" in internals.ctx.elements[elIdx] && !internals.ctx.elements[elIdx].props.label) {
            internals.ctx.elements[elIdx] = { ...el, props: { ...el.props, label: textContent } } as ElementData;
          }
        }
        return;
      }
      // Collect token with its column position
      const tok = internals.stream.consume();
      textParts.push({ text: tok.text, col: tok.col });
    }

    // If no closing tag on same line, set text content from any collected text
    let textContent = "";
    let lastEndCol = -1;
    for (const p of textParts) {
      if (lastEndCol >= 0 && p.col > lastEndCol) {
        textContent += " ".repeat(p.col - lastEndCol);
      }
      textContent += p.text;
      lastEndCol = p.col + p.text.length;
    }
    textContent = textContent.trim();
    if (textContent && "props" in el) {
      const elIdx = internals.ctx.elements.findIndex(e => e.id === el.id);
      if (elIdx !== -1 && "props" in internals.ctx.elements[elIdx] && !internals.ctx.elements[elIdx].props.label) {
        internals.ctx.elements[elIdx] = { ...el, props: { ...el.props, label: textContent } } as ElementData;
      }
    }
  }
}

// ── View attributes ──

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
export function parseViewAttributes(internals: ParserInternals): ViewElementAttrs {
  const result: ViewElementAttrs = {
    id: null,
    className: "",
    attrs: {},
    events: {},
    bindings: {},
    styleStr: "",
    isSelfClosing: false,
  };

  while (!internals.stream.isEof()) {
    const tok = internals.stream.peek();

    // End of tag
    if (tok.kind === "ANGLE_CLOSE") {
      internals.stream.consume();
      break;
    }

    if (tok.kind === "SELF_CLOSING") {
      internals.stream.consume();
      result.isSelfClosing = true;
      break;
    }

    // Event binding: @name={expression}
    if (tok.kind === "AT") {
      internals.stream.consume(); // AT (@)
      const rawEventName = internals.stream.expect("IDENTIFIER").text;
      internals.stream.expect("EQUALS");
      internals.stream.expect("BRACE_OPEN");

      // Collect all tokens between { and } as a raw expression string,
      // using brace-depth counting to handle nested braces (closures, objects, etc.)
      let depth = 1;
      let handlerExpr = "";
      while (depth > 0 && !internals.stream.isEof()) {
        const t = internals.stream.peek();
        if (t.kind === "BRACE_OPEN") depth++;
        else if (t.kind === "BRACE_CLOSE") depth--;
        if (depth === 0) {
          internals.stream.consume(); // consume closing }
          break;
        }
        internals.stream.consume();
        handlerExpr += t.text;
      }
      handlerExpr = handlerExpr.trim();

      // Normalize event name: ensure it has "on" prefix
      // @click => onClick, @onClick => onClick
      const normalizedEventName = rawEventName.startsWith("on")
        ? rawEventName
        : "on" + rawEventName.charAt(0).toUpperCase() + rawEventName.slice(1);

      // 语义化归一化（Task 4）：旧 DOM 拼写（@click/@onClick/@tap/@onTap/
      // @mouseenter/@onMouseEnter/@mouseleave/@onMouseLeave）收敛为语义名
      // （onPress/onEnter/onLeave）——与 Rust event_names.rs 同步，门禁测试防漂移。
      // 读侧归一化保证编辑器内存态只出现语义名；semanticEventName 是
      // toSemanticEventName + 恒等回退的封装，两端共用同一张表。
      const semanticName = semanticEventName(normalizedEventName);

      result.events[semanticName] = handlerExpr;
      continue;
    }

    // Identifier-based attributes
    if (tok.kind === "IDENTIFIER") {
      const attrName = internals.stream.consume().text;

      // bind: prefix
      if (attrName === "bind") {
        internals.stream.expect("COLON");
        const bindName = internals.stream.expect("IDENTIFIER").text;
        internals.stream.expect("EQUALS");
        internals.stream.expect("BRACE_OPEN");
        const bindValue = internals.stream.expect("IDENTIFIER").text;
        internals.stream.expect("BRACE_CLOSE");
        result.bindings[bindName] = bindValue;
        continue;
      }

      // There must be EQUALS or COLON after the attribute name
      const next = internals.stream.peek();

      // 裸布尔属性（D3）：<input disabled> → attrs.disabled = true。
      // 必须 continue——现状 break 会终止整个属性循环，后续属性全部丢失。
      if (next.kind !== "EQUALS" && next.kind !== "COLON") {
        if (BOOLEAN_ATTRS.has(attrName)) {
          result.attrs[attrName] = true;
          continue;
        }
        // 未知裸属性：维持旧容错（break）
        break;
      }

      if (next.kind === "EQUALS") {
        internals.stream.consume();

        // Check if this is an identifier-value (non-string) or string
        const valTok = internals.stream.peek();

        // Stop at end of tag tokens
        if (valTok.kind === "ANGLE_CLOSE" || valTok.kind === "SELF_CLOSING") {
          break;
        }

        if (valTok.kind === "BRACE_OPEN") {
          // {expr} 动态属性值（D2）：配平花括号收集任意表达式，存 "{expr}"。
          // 与事件 handler（315-329）同一收集模式；id 特例保持旧行为（D7）。
          internals.stream.consume(); // {
          let depth = 1;
          let expr = "";
          let lastEndCol = -1;
          while (depth > 0 && !internals.stream.isEof()) {
            const t = internals.stream.peek();
            if (t.kind === "BRACE_OPEN") depth++;
            else if (t.kind === "BRACE_CLOSE") depth--;
            if (depth === 0) {
              internals.stream.consume(); // }
              break;
            }
            // 行内空格被 tokenizer 丢弃——按列距插回（同 @if condition 收集模式）
            if (lastEndCol >= 0 && t.col > lastEndCol) {
              expr += " ".repeat(t.col - lastEndCol);
            }
            internals.stream.consume();
            expr += t.text;
            lastEndCol = t.col + t.text.length;
          }
          expr = expr.trim();
          if (attrName === "id") {
            result.id = expr;
          } else if (attrName === "class") {
            // 动态 class={expr} 不在本次范围（class 走 className 管线）——跳过
          } else {
            result.attrs[attrName] = expr.length > 0 ? `{${expr}}` : "";
          }
          continue;
        }

        if (valTok.kind === "STRING") {
          const strVal = internals.stream.consume().text.slice(1, -1); // strip quotes
          if (attrName === "id") {
            result.id = strVal;
          } else if (attrName === "class") {
            // D5：class="x" 不再静默丢弃——与 class: 形式统一走 className 管线
            result.className = strVal.trim();
          } else {
            result.attrs[attrName] = strVal;
          }
          continue;
        }

        // Fallback: consume tokens as value
        let value = "";
        while (
          !internals.stream.isEof() &&
          internals.stream.peek().kind !== "ANGLE_CLOSE" &&
          internals.stream.peek().kind !== "SELF_CLOSING" &&
          internals.stream.peek().kind !== "NEWLINE"
        ) {
          value += internals.stream.consume().text;
        }
        if (attrName === "id") {
          result.id = value;
        } else if (attrName !== "class") {
          result.attrs[attrName] = value;
        }
        continue;
      }

      if (next.kind === "COLON") {
        internals.stream.consume();

        // style: { ... }
        if (attrName === "style") {
          result.styleStr = parseStyleAttribute(internals);
          continue;
        }

        // class: name1 name2 ...
        if (attrName === "class") {
          result.className = parseClassName(internals);
          continue;
        }

        // variant: DesignName.VariantName(param=val, ...)
        if (attrName === "variant") {
          result.attrs["variant:"] = parseVariantAttribute(internals);
          continue;
        }

        // Other colon-prefixed attributes
        let value = "";
        while (
          !internals.stream.isEof() &&
          internals.stream.peek().kind !== "ANGLE_CLOSE" &&
          internals.stream.peek().kind !== "SELF_CLOSING" &&
          internals.stream.peek().kind !== "NEWLINE" &&
          internals.stream.peek().kind !== "AT" &&
          !(internals.stream.peek().kind === "IDENTIFIER" && isAttrBound(internals.stream.peek(1)))
        ) {
          value += internals.stream.consume().text;
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
 * Split a string on commas that appear at the top level (outside of quotes).
 * Preserves commas inside double-quoted strings so that values like
 * `rgba(0,0,0,0.1)` or `"0, 0, 0"` are not truncated.
 */
function splitTopLevelCommas(content: string): string[] {
  const parts: string[] = [];
  let current = "";
  let inQuotes = false;

  for (let i = 0; i < content.length; i++) {
    const ch = content[i];
    if (ch === '"') {
      inQuotes = !inQuotes;
      current += ch;
    } else if (ch === "," && !inQuotes) {
      parts.push(current);
      current = "";
    } else {
      current += ch;
    }
  }
  if (current.length > 0) {
    parts.push(current);
  }
  return parts;
}

/**
 * Parse a `style: { key: val, key2: "val2" }` attribute.
 * Returns the reconstructed style string in the format the old parser used:
 * "{ key: val; key2: val2 }" (comma → semicolon, quotes stripped).
 */
export function parseStyleAttribute(internals: ParserInternals): string {
  internals.stream.expect("BRACE_OPEN");
  const styleTokens: Token[] = [];
  let depth = 1;
  while (depth > 0 && !internals.stream.isEof()) {
    const t = internals.stream.consume();
    if (t.kind === "BRACE_OPEN") depth++;
    else if (t.kind === "BRACE_CLOSE") depth--;
    if (depth > 0) styleTokens.push(t);
  }

  // Reconstruct style content from tokens
  const styleContent = styleTokens.map((t) => t.text).join("").trim();

  // Split on top-level commas only (not inside quoted strings),
  // so that values like rgba(0,0,0,0.1) or "0, 0, 0" survive intact.
  const semiStyle = splitTopLevelCommas(styleContent)
    .reduce<string[]>((acc, part) => {
      const p = part.trim();
      if (p) acc.push(p.replace(/:\s*"([^"]*)"/g, ": $1"));
      return acc;
    }, [])
    .join("; ");

  return `{ ${semiStyle} }`;
}

/**
 * Parse variant attribute value: `DesignName.VariantName(param=val, ...)`
 */
export function parseVariantAttribute(internals: ParserInternals): string {
  let result = "";
  // Read DesignName.VariantName
  result += internals.stream.expect("IDENTIFIER").text; // DesignName
  if (internals.stream.peek().kind === "DOT") {
    result += internals.stream.consume().text; // "."
    result += internals.stream.expect("IDENTIFIER").text; // VariantName
  }
  // Optionally read (param=val, ...)
  if (internals.stream.peek().kind === "PAREN_OPEN") {
    let depth = 1;
    result += internals.stream.consume().text; // "("
    while (depth > 0 && !internals.stream.isEof()) {
      const t = internals.stream.peek();
      if (t.kind === "PAREN_OPEN") depth++;
      else if (t.kind === "PAREN_CLOSE") depth--;
      if (t.kind === "ANGLE_CLOSE" || t.kind === "SELF_CLOSING" || t.kind === "NEWLINE") break;
      result += internals.stream.consume().text;
    }
  }
  return result;
}

/**
 * Parse class names after `class:` — collects identifier fragments
 * joined by hyphens and spaces.
 */
export function parseClassName(internals: ParserInternals): string {
  const classes: string[] = [];

  while (!internals.stream.isEof()) {
    // Skip any stray whitespace text
    while (internals.stream.peek()?.kind === "TEXT" && /^\s+$/.test(internals.stream.peek()!.text)) {
      internals.stream.consume();
    }

    const t = internals.stream.peek();

    if (t.kind === "IDENTIFIER") {
      // Stop if this identifier is the start of another attribute
      if (isAttrBound(internals.stream.peek(1))) {
        break;
      }

      // Start a class name
      let name = internals.stream.consume().text;

      // Handle hyphenated continuation: my-class
      while (
        internals.stream.peek()?.kind === "TEXT" &&
        internals.stream.peek()?.text === "-"
      ) {
        internals.stream.consume(); // consume "-"
        if (internals.stream.peek()?.kind === "IDENTIFIER") {
          name += "-" + internals.stream.consume().text;
        } else {
          break;
        }
      }

      // Handle dot-separated continuation: Design.Class
      // The DOT token is emitted as kind "DOT", not "TEXT"
      while (internals.stream.peek()?.kind === "DOT") {
        internals.stream.consume(); // consume "."
        if (internals.stream.peek()?.kind === "IDENTIFIER") {
          name += "." + internals.stream.consume().text;
        } else {
          break;
        }
      }

      classes.push(name);
    } else if (t.kind === "TEXT" && t.text === "-") {
      // Stray hyphen — skip
      internals.stream.consume();
    } else if (t.kind === "STRING") {
      // D5：class: "a b" 字符串形式——拆空白并入类名列表
      internals.stream.consume();
      for (const cls of t.text.slice(1, -1).split(/\s+/)) {
        if (cls) classes.push(cls);
      }
    } else {
      // End of class list — stop at end-of-tag tokens
      break;
    }
  }

  return classes.join(" ");
}

// ── View directive (@if only — @for/@slot to follow) ──

/**
 * Parse @if / @else if / @else directives in view body.
 * Produces a ConditionalBlockData element with branch children.
 */
export function parseViewDirective(internals: ParserInternals): void {
  internals.stream.consume(); // AT (@)

  const directiveTok = internals.stream.expect("IDENTIFIER");
  if (directiveTok.kind === "EOF") return;

  // @for 解析（2026-08-06：view 级 @for 支持，闭环调试列表卡）
  if (directiveTok.text === "for") {
    parseForDirective(internals);
    return;
  }

  // Currently only @if is handled; @for and @slot will follow
  if (directiveTok.text !== "if") {
    // Skip unknown directives by consuming until end of block
    let depth = 0;
    while (!internals.stream.isEof()) {
      const t = internals.stream.peek();
      if (t.kind === "BRACE_OPEN") depth++;
      else if (t.kind === "BRACE_CLOSE") { depth--; if (depth < 0) break; }
      internals.stream.consume();
    }
    return;
  }

  const condBlockId = `${internals.ctx.currentComponent}.@if.${internals.ctx.elements.length}`;
  const branches: Array<{
    type: "if" | "elif" | "else";
    condition: string;
    children: string[];
  }> = [];

  // Push condition block ID to parent stack so children get correct parentId
  // （无条件：顶层 @if 的子元素 parentId 也必须是条件块——2026-08-06 与
  // @for 统一；此前顶层指令子元素 parentId 恒 null，删除/undo 连带丢失）
  internals.ctx.viewParentStack.push(condBlockId);

  let currentBranchType: "if" | "elif" | "else" = "if";

  while (!internals.stream.isEof()) {
    // Collect condition expression until BRACE_OPEN
    let condition = "";
    if (currentBranchType !== "else") {
      let lastEndCol = -1;
      while (
        !internals.stream.isEof() &&
        internals.stream.peek().kind !== "BRACE_OPEN" &&
        internals.stream.peek().kind !== "NEWLINE"
      ) {
        const t = internals.stream.peek();
        // Insert spaces based on column gap between consecutive tokens
        if (lastEndCol >= 0 && t.col > lastEndCol) {
          condition += " ".repeat(t.col - lastEndCol);
        }
        internals.stream.consume();
        condition += t.text;
        lastEndCol = t.col + t.text.length;
      }
      condition = condition.trim();
    }

    // Expect opening brace
    if (internals.stream.peek().kind === "NEWLINE") {
      internals.stream.skipNewlines();
    }
    internals.stream.expect("BRACE_OPEN");

    // Track elements added inside this branch
    const beforeCount = internals.ctx.elements.length;

    // Parse branch body: recurse into view elements until matching close brace
    let braceDepth = 1;
    while (braceDepth > 0 && !internals.stream.isEof()) {
      const tok = internals.stream.peek();

      if (tok.kind === "BRACE_CLOSE") {
        braceDepth--;
        if (braceDepth === 0) {
          internals.stream.consume();
          break;
        }
        internals.stream.consume();
        continue;
      }

      if (tok.kind === "BRACE_OPEN") {
        braceDepth++;
        internals.stream.consume();
        continue;
      }

      // Recurse into view elements
      if (tok.kind === "ANGLE_OPEN") {
        // Handle closing tags (</tagname>) — pop from stack, don't create element
        if (internals.stream.peek(1)?.kind === "SLASH") {
          internals.stream.consume(); // ANGLE_OPEN
          internals.stream.consume(); // SLASH
          internals.stream.consume(); // IDENTIFIER (tag name)
          if (internals.stream.peek().kind === "ANGLE_CLOSE") {
            internals.stream.consume();
          }
          if (internals.ctx.viewParentStack.length > 0) {
            internals.ctx.viewParentStack.pop();
          }
        } else {
          parseViewElement(internals);
        }
      } else if (tok.kind === "COMMENT_LINE") {
        parseViewComment(internals);
      } else if (
        tok.kind === "IDENTIFIER" ||
        tok.kind === "TEXT" ||
        tok.kind === "STRING"
      ) {
        parseViewTextContent(internals);
      } else if (tok.kind === "AT") {
        // @if 分支体内可嵌套指令（@for/@if）——2026-08-06 补全
        parseViewDirective(internals);
      } else {
        internals.stream.consume();
      }

      internals.stream.skipNewlines();
    }

    // Collect child element IDs added during this branch
    const afterCount = internals.ctx.elements.length;
    const branchChildren: string[] = [];
    for (let i = beforeCount; i < afterCount; i++) {
      branchChildren.push(internals.ctx.elements[i].id);
    }

    // Save branch
    branches.push({
      type: currentBranchType,
      condition,
      children: branchChildren,
    });

    // Check for @else / @else if continuation
    internals.stream.skipNewlines();
    const nextTok = internals.stream.peek();
    if (
      nextTok.kind === "AT" &&
      internals.stream.peek(1)?.kind === "IDENTIFIER" &&
      internals.stream.peek(1)?.text === "else"
    ) {
      internals.stream.consume(); // AT
      internals.stream.consume(); // "else"
      // Check if it's "else if" or just "else"
      const afterElse = internals.stream.peek();
      if (afterElse.kind === "IDENTIFIER" && afterElse.text === "if") {
        internals.stream.consume(); // "if"
        currentBranchType = "elif";
        continue;
      }
      currentBranchType = "else";
      continue;
    }

    // No more branches
    break;
  }

  // Pop condition block from parent stack FIRST so parentId resolves
  // to the actual parent element, not the condBlock itself or any bogus
  // elements that may have been pushed during branch body parsing.
  internals.ctx.viewParentStack.pop();

  // Now compute parentId from the actual parent (top of remaining stack)
  const parentId =
    internals.ctx.viewParentStack.length > 0
      ? internals.ctx.viewParentStack[internals.ctx.viewParentStack.length - 1]
      : null;

  // Build ConditionalBlockData
  const allChildren = branches.flatMap((b) => b.children);
  const condBlock: import("../../core/types/element").ConditionalBlockData = {
    id: condBlockId,
    nodeType: "conditional",
    parentId,
    branches,
    children: allChildren,
  };

  internals.ctx.elements.push(condBlock);

  // Register conditional block in parent's children array
  if (parentId) {
    const parent = internals.ctx.elements.find((e) => e.id === parentId);
    if (parent && "children" in parent && Array.isArray(parent.children)) {
      (parent as { children: string[] }).children.push(condBlockId);
    }
  }
}

/**
 * Parse @for directive: `@for <item> in <collection> key={expr}? { body } @empty? { body }`
 * Produces a ForBlockData element (2026-08-06 —— 与 @if 同构：设计树节点 + 发射已支持）。
 */
function parseForDirective(internals: ParserInternals): void {
  const forId = `${internals.ctx.currentComponent}.@for.${internals.ctx.elements.length}`;

  // itemName: @for <ident> in ...
  const itemTok = internals.stream.expect("IDENTIFIER");
  const itemName = itemTok.kind === "EOF" ? "item" : itemTok.text;
  const inTok = internals.stream.peek();
  if (inTok.kind === "IDENTIFIER" && inTok.text === "in") {
    internals.stream.consume();
  }

  // collection: 收集 token 直到 key= 或块花括号（列距空格插入，同 @if condition 处理）
  let collection = "";
  let lastEndCol = -1;
  while (!internals.stream.isEof()) {
    const t = internals.stream.peek();
    if (t.kind === "BRACE_OPEN") break;
    if (t.kind === "IDENTIFIER" && t.text === "key" && internals.stream.peek(1)?.kind === "EQUALS") break;
    if (t.kind === "NEWLINE") break;
    if (lastEndCol >= 0 && t.col > lastEndCol) {
      collection += " ".repeat(t.col - lastEndCol);
    }
    internals.stream.consume();
    collection += t.text;
    lastEndCol = t.col + t.text.length;
  }
  collection = collection.trim();

  // 可选 key={expr}（表达式花括号，配平收集）
  let keyExpr: string | undefined;
  const afterCol = internals.stream.peek();
  if (afterCol.kind === "IDENTIFIER" && afterCol.text === "key") {
    internals.stream.consume(); // key
    const eq = internals.stream.peek();
    if (eq.kind === "EQUALS") internals.stream.consume();
    const open = internals.stream.peek();
    if (open.kind === "BRACE_OPEN") {
      internals.stream.consume();
      keyExpr = "";
      let depth = 1;
      while (depth > 0 && !internals.stream.isEof()) {
        const t = internals.stream.peek();
        if (t.kind === "BRACE_OPEN") depth++;
        else if (t.kind === "BRACE_CLOSE") { depth--; if (depth === 0) { internals.stream.consume(); break; } }
        internals.stream.consume();
        keyExpr += t.text;
      }
      keyExpr = keyExpr.trim();
    }
  }

  // 推入父栈（children 的 parentId 解析）——无条件：顶层 @for（栈空）时
  // 子元素 parentId 也必须是 for 块（2026-08-06 实测：conditional 的
  // `if (length > 0)` 保护导致顶层指令子元素 parentId null，@for 不沿用）
  internals.ctx.viewParentStack.push(forId);

  // 循环体：braceDepth 扫描 + 递归 view 元素（同 @if 分支体逻辑）
  internals.stream.skipNewlines();
  internals.stream.expect("BRACE_OPEN");
  const bodyBefore = internals.ctx.elements.length;
  let braceDepth = 1;
  while (braceDepth > 0 && !internals.stream.isEof()) {
    const tok = internals.stream.peek();
    if (tok.kind === "BRACE_CLOSE") {
      braceDepth--;
      if (braceDepth === 0) { internals.stream.consume(); break; }
      internals.stream.consume();
      continue;
    }
    if (tok.kind === "BRACE_OPEN") { braceDepth++; internals.stream.consume(); continue; }
    if (tok.kind === "ANGLE_OPEN") {
      if (internals.stream.peek(1)?.kind === "SLASH") {
        internals.stream.consume(); // ANGLE_OPEN
        internals.stream.consume(); // SLASH
        internals.stream.consume(); // IDENTIFIER
        if (internals.stream.peek().kind === "ANGLE_CLOSE") internals.stream.consume();
        if (internals.ctx.viewParentStack.length > 0) internals.ctx.viewParentStack.pop();
      } else {
        parseViewElement(internals);
      }
    } else if (tok.kind === "COMMENT_LINE") {
      parseViewComment(internals);
    } else if (tok.kind === "IDENTIFIER" || tok.kind === "TEXT" || tok.kind === "STRING") {
      parseViewTextContent(internals);
    } else {
      internals.stream.consume();
    }
    internals.stream.skipNewlines();
  }
  const bodyAfter = internals.ctx.elements.length;
  const children: string[] = [];
  for (let i = bodyBefore; i < bodyAfter; i++) children.push(internals.ctx.elements[i].id);

  // 可选 @empty { body }
  let emptyChildren: string[] = [];
  internals.stream.skipNewlines();
  const nextTok = internals.stream.peek();
  if (nextTok.kind === "AT" && internals.stream.peek(1)?.kind === "IDENTIFIER" && internals.stream.peek(1)?.text === "empty") {
    internals.stream.consume(); // AT
    internals.stream.consume(); // empty
    internals.stream.skipNewlines();
    internals.stream.expect("BRACE_OPEN");
    const emptyBefore = internals.ctx.elements.length;
    let emptyDepth = 1;
    while (emptyDepth > 0 && !internals.stream.isEof()) {
      const tok = internals.stream.peek();
      if (tok.kind === "BRACE_CLOSE") {
        emptyDepth--;
        if (emptyDepth === 0) { internals.stream.consume(); break; }
        internals.stream.consume();
        continue;
      }
      if (tok.kind === "BRACE_OPEN") { emptyDepth++; internals.stream.consume(); continue; }
      if (tok.kind === "ANGLE_OPEN") {
        if (internals.stream.peek(1)?.kind === "SLASH") {
          internals.stream.consume(); // ANGLE_OPEN
          internals.stream.consume(); // SLASH
          internals.stream.consume(); // IDENTIFIER
          if (internals.stream.peek().kind === "ANGLE_CLOSE") internals.stream.consume();
          if (internals.ctx.viewParentStack.length > 0) internals.ctx.viewParentStack.pop();
        } else {
          parseViewElement(internals);
        }
      } else if (tok.kind === "COMMENT_LINE") {
        parseViewComment(internals);
      } else if (tok.kind === "IDENTIFIER" || tok.kind === "TEXT" || tok.kind === "STRING") {
        parseViewTextContent(internals);
      } else {
        internals.stream.consume();
      }
      internals.stream.skipNewlines();
    }
    const emptyAfter = internals.ctx.elements.length;
    for (let i = emptyBefore; i < emptyAfter; i++) emptyChildren.push(internals.ctx.elements[i].id);
  }

  // 弹栈后计算真实 parentId
  internals.ctx.viewParentStack.pop();
  const parentId =
    internals.ctx.viewParentStack.length > 0
      ? internals.ctx.viewParentStack[internals.ctx.viewParentStack.length - 1]
      : null;

  const forBlock: import("../../core/types/element").ForBlockData = {
    id: forId,
    nodeType: "for",
    parentId,
    itemName,
    collection,
    ...(keyExpr ? { keyExpr } : {}),
    children,
    emptyChildren,
  };
  internals.ctx.elements.push(forBlock);

  if (parentId) {
    const parent = internals.ctx.elements.find((e) => e.id === parentId);
    if (parent && "children" in parent && Array.isArray(parent.children)) {
      (parent as { children: string[] }).children.push(forId);
    }
  }
}
