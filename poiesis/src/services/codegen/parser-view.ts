// poiesis/src/services/codegen/parser-view.ts
// View parsing functions extracted from the Parser class.

import type { Token } from "./omni-tokenizer";
import type { ElementData, ViewElementData } from "../../core/types/element";
import type { ParserInternals } from "./parser";
import type { ViewElementAttrs } from "./parser-types";
import { isAttrBound } from "./parser-types";
import { createElement } from "./parser-helpers";

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

    // Text content on its own line
    if (tok.kind === "IDENTIFIER" || tok.kind === "TEXT" || tok.kind === "STRING") {
      parseViewTextContent(internals);
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

  // @edit annotation
  if (text.startsWith("// @edit")) {
    internals.ctx.pendingEditFlags = text.slice("// @edit".length).trim() || null;
  }
  // Other view comments are ignored (old parser behavior)
}

// ── View text content ──

export function parseViewTextContent(internals: ParserInternals): void {
  // Collect all textual tokens on this line as text content
  let text = "";
  while (
    !internals.stream.isEof() &&
    internals.stream.peek().kind !== "NEWLINE" &&
    internals.stream.peek().kind !== "BRACE_CLOSE" &&
    internals.stream.peek().kind !== "ANGLE_OPEN"
  ) {
    text += internals.stream.consume().text;
  }
  text = text.trim();

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

    // Check for text content between > and </ on same line
    const textTokens: string[] = [];
    while (
      !internals.stream.isEof() &&
      internals.stream.peek().kind !== "NEWLINE" &&
      internals.stream.peek().kind !== "BRACE_CLOSE"
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

        // Set text content if we collected any
        const textContent = textTokens.join("").trim();
        if (textContent && "props" in el) {
          const elIdx = internals.ctx.elements.findIndex(e => e.id === el.id);
          if (elIdx !== -1 && "props" in internals.ctx.elements[elIdx] && !internals.ctx.elements[elIdx].props.label) {
            internals.ctx.elements[elIdx] = { ...el, props: { ...el.props, label: textContent } } as ElementData;
          }
        }
        return;
      }
      // Collect token as text content
      textTokens.push(internals.stream.consume().text);
    }

    // If no closing tag on same line, set text content from any collected text
    const textContent = textTokens.join("").trim();
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

    // Event binding: @name={handler}
    if (tok.kind === "AT") {
      internals.stream.consume();
      const eventName = internals.stream.expect("IDENTIFIER").text;
      internals.stream.expect("EQUALS");
      internals.stream.expect("BRACE_OPEN");
      const handlerName = internals.stream.expect("IDENTIFIER").text;
      internals.stream.expect("BRACE_CLOSE");
      result.events[eventName] = handlerName;
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
        result.attrs[`bind:${bindName}`] = bindValue;
        continue;
      }

      // There must be EQUALS or COLON after the attribute name
      const next = internals.stream.peek();
      if (next.kind === "EQUALS") {
        internals.stream.consume();

        // Check if this is an identifier-value (non-string) or string
        const valTok = internals.stream.peek();

        // Stop at end of tag tokens
        if (valTok.kind === "ANGLE_CLOSE" || valTok.kind === "SELF_CLOSING") {
          break;
        }

        if (valTok.kind === "BRACE_OPEN") {
          // {value} binding syntax
          internals.stream.consume();
          const v = internals.stream.expect("IDENTIFIER").text;
          internals.stream.expect("BRACE_CLOSE");
          if (attrName === "id") {
            result.id = v;
          } else if (attrName !== "class") {
            result.attrs[attrName] = v;
          }
          continue;
        }

        if (valTok.kind === "STRING") {
          const strVal = internals.stream.consume().text.slice(1, -1); // strip quotes
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

        // Other colon-prefixed attributes (e.g. bind: is handled above, but
        // there could be others like `prop: value` syntax)
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

      classes.push(name);
    } else if (t.kind === "TEXT" && t.text === "-") {
      // Stray hyphen — skip
      internals.stream.consume();
    } else {
      // End of class list — stop at end-of-tag tokens
      break;
    }
  }

  return classes.join(" ");
}
