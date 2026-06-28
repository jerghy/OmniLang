// editor/client/src/omni-gen/omni-tokenizer.ts
// Tokenizer for the OmniLang parser — walks source char-by-char and emits a flat Token[].

export type TokenKind =
  | "BRACE_OPEN" | "BRACE_CLOSE"
  | "PAREN_OPEN" | "PAREN_CLOSE"
  | "ANGLE_OPEN" | "ANGLE_CLOSE" | "SELF_CLOSING"
  | "SLASH"
  | "SEMICOLON" | "COLON" | "EQUALS" | "ARROW"
  | "DOT" | "AT" | "PIPE" | "COMMA"
  | "IDENTIFIER" | "STRING" | "NUMBER" | "TEXT"
  | "COMMENT_LINE"
  | "NEWLINE"
  | "EOF";

export interface Token {
  kind: TokenKind;
  text: string;
  line: number;  // 1-based
  col: number;   // 1-based
}

/**
 * Tokenize an OmniLang source string into a flat array of tokens.
 *
 * - Blank lines are skipped entirely (no NEWLINE emitted for them).
 * - A NEWLINE token is emitted between consecutive non-blank lines.
 * - Leading whitespace is consumed silently; the first token on each line
 *   carries its actual column so the parser can derive indent.
 * - Strings (`"..."`) skip all special-character interpretation within.
 * - Comments (`//` to end of line) become a single COMMENT_LINE token.
 * - `/>` becomes SELF_CLOSING, `->` becomes ARROW.
 * - `{` `}` `(` `)` `<` `>` `;` `:` `=` `.` `@` `|` `,` `/` each map to
 *   their own token kinds.
 * - Sequences of [a-zA-Z_][a-zA-Z0-9_]* become IDENTIFIER. Pure digit
 *   sequences become NUMBER.
 */
export function tokenize(source: string): Token[] {
  const rawLines = source.split("\n");
  const tokens: Token[] = [];
  let lastLineHadContent = false;

  for (let i = 0; i < rawLines.length; i++) {
    const line = rawLines[i].replace(/\r$/, "");
    const trimmed = line.trim();

    // Skip completely blank lines
    if (trimmed === "") continue;

    // Emit NEWLINE before this line if the previous line had content
    if (lastLineHadContent) {
      tokens.push({ kind: "NEWLINE", text: "\n", line: i, col: 1 });
    }

    // Tokenize this line
    const indent = line.length - line.trimStart().length; // 0-based
    const lineTokens = scanLine(trimmed, i + 1, indent + 1);
    tokens.push(...lineTokens);
    lastLineHadContent = true;
  }

  tokens.push({ kind: "EOF", text: "", line: rawLines.length, col: 1 });
  return tokens;
}

/**
 * Scan the trimmed content of a single line and produce tokens.
 *
 * @param content  The trimmed line (no leading/trailing whitespace).
 * @param lineNum  1-based line number.
 * @param firstCol 1-based column of the first character.
 */
function scanLine(
  content: string,
  lineNum: number,
  firstCol: number,
): Token[] {
  const tokens: Token[] = [];
  let pos = 0;
  let col = firstCol;

  while (pos < content.length) {
    const ch = content[pos];

    // Skip whitespace BETWEEN tokens (spaces, tabs inside the line)
    if (ch === " " || ch === "\t") {
      pos++;
      col++;
      continue;
    }

    switch (ch) {
      case "{":
        tokens.push({ kind: "BRACE_OPEN", text: "{", line: lineNum, col });
        pos++;
        col++;
        break;

      case "}":
        tokens.push({ kind: "BRACE_CLOSE", text: "}", line: lineNum, col });
        pos++;
        col++;
        break;

      case "(":
        tokens.push({ kind: "PAREN_OPEN", text: "(", line: lineNum, col });
        pos++;
        col++;
        break;

      case ")":
        tokens.push({ kind: "PAREN_CLOSE", text: ")", line: lineNum, col });
        pos++;
        col++;
        break;

      case "<":
        if (content[pos + 1] === "/") {
          // Closing tag </tagname>
          tokens.push({ kind: "ANGLE_OPEN", text: "<", line: lineNum, col });
          tokens.push({ kind: "SLASH", text: "/", line: lineNum, col: col + 1 });
          pos += 2;
          col += 2;
        } else {
          tokens.push({ kind: "ANGLE_OPEN", text: "<", line: lineNum, col });
          pos++;
          col++;
        }
        break;

      case ">":
        tokens.push({ kind: "ANGLE_CLOSE", text: ">", line: lineNum, col });
        pos++;
        col++;
        break;

      case ";":
        tokens.push({ kind: "SEMICOLON", text: ";", line: lineNum, col });
        pos++;
        col++;
        break;

      case ":":
        tokens.push({ kind: "COLON", text: ":", line: lineNum, col });
        pos++;
        col++;
        break;

      case "=":
        tokens.push({ kind: "EQUALS", text: "=", line: lineNum, col });
        pos++;
        col++;
        break;

      case ".":
        tokens.push({ kind: "DOT", text: ".", line: lineNum, col });
        pos++;
        col++;
        break;

      case "@":
        tokens.push({ kind: "AT", text: "@", line: lineNum, col });
        pos++;
        col++;
        break;

      case "|":
        tokens.push({ kind: "PIPE", text: "|", line: lineNum, col });
        pos++;
        col++;
        break;

      case ",":
        tokens.push({ kind: "COMMA", text: ",", line: lineNum, col });
        pos++;
        col++;
        break;

      case "/":
        if (content[pos + 1] === "/") {
          // Comment to end of line
          tokens.push({
            kind: "COMMENT_LINE",
            text: content.substring(pos),
            line: lineNum,
            col,
          });
          pos = content.length; // consume rest of line
        } else if (content[pos + 1] === ">") {
          // Self-closing tag />
          tokens.push({
            kind: "SELF_CLOSING",
            text: "/>",
            line: lineNum,
            col,
          });
          pos += 2;
          col += 2;
        } else {
          // Standalone slash (e.g. in a URL path inside a string — shouldn't reach here, but handle)
          tokens.push({ kind: "SLASH", text: "/", line: lineNum, col });
          pos++;
          col++;
        }
        break;

      case "-":
        if (content[pos + 1] === ">") {
          tokens.push({ kind: "ARROW", text: "->", line: lineNum, col });
          pos += 2;
          col += 2;
        } else {
          // Standalone hyphen — TEXT token
          tokens.push({ kind: "TEXT", text: "-", line: lineNum, col });
          pos++;
          col++;
        }
        break;

      case '"': {
        // String literal — read until closing quote
        const startPos = pos;
        pos++;
        col++;
        while (pos < content.length) {
          if (content[pos] === "\\") {
            // Escaped character
            pos += 2;
            col += 2;
            continue;
          }
          if (content[pos] === '"') {
            pos++;
            col++;
            break;
          }
          pos++;
          col++;
        }
        tokens.push({
          kind: "STRING",
          text: content.substring(startPos, pos),
          line: lineNum,
          col: firstCol + startPos,
        });
        break;
      }

      default:
        if (/[a-zA-Z_]/.test(ch)) {
          // Identifier
          const startPos = pos;
          while (
            pos < content.length &&
            /[a-zA-Z_0-9]/.test(content[pos])
          ) {
            pos++;
            col++;
          }
          tokens.push({
            kind: "IDENTIFIER",
            text: content.substring(startPos, pos),
            line: lineNum,
            col: firstCol + startPos,
          });
        } else if (/[0-9]/.test(ch)) {
          // Number
          const startPos = pos;
          while (
            pos < content.length &&
            /[0-9]/.test(content[pos])
          ) {
            pos++;
            col++;
          }
          tokens.push({
            kind: "NUMBER",
            text: content.substring(startPos, pos),
            line: lineNum,
            col: firstCol + startPos,
          });
        } else {
          // Collect consecutive non-pattern characters as TEXT
          const textStartPos = pos;
          const textStartCol = col;
          while (pos < content.length) {
            const c = content[pos];
            // Stop at any character that starts a known token pattern
            if ("{}()<>;:=.@|,/".includes(c) || c === '"' || c === '-' ||
                c === ' ' || c === '\t' ||
                /[a-zA-Z_0-9]/.test(c)) {
              break;
            }
            // Also stop at // (comment) or /> (self-closing)
            if (c === '/' && (content[pos + 1] === '/' || content[pos + 1] === '>')) break;
            pos++;
            col++;
          }
          if (pos > textStartPos) {
            tokens.push({
              kind: "TEXT",
              text: content.substring(textStartPos, pos),
              line: lineNum,
              col: textStartCol,
            });
          }
        }
        break;
    }
  }

  return tokens;
}
