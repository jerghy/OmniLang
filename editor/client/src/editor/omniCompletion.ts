/**
 * OmniLang completion provider for Monaco.
 *
 * Provides context-aware suggestions for:
 *  - top-level declarations (component, store, fn, signal, computed)
 *  - OmniView built-in components (Container, Button, Text, …)
 *  - store / signal / computed block bodies
 *  - fn block body expressions
 */

import type { languages } from "monaco-editor";

// ── Snippets ──────────────────────────────────────

interface OmniSnippet {
  label: string;
  detail: string;
  insert: string;
  kind: languages.CompletionItemKind;
}

const TOP_LEVEL_SNIPPETS: OmniSnippet[] = [
  {
    label: "component",
    detail: "OmniView UI component",
    insert: "component ${1:Name} {\n    view {\n        Container {\n            Text { label: \"${2:Hello}\" }\n        }\n    }\n}",
    kind: 9, // Class
  },
  {
    label: "store",
    detail: "State management store",
    insert: "store ${1:StoreName} {\n    signal ${2:count}: int = 0;\n\n    action ${3:increment}() {\n        this.${2:count} += 1;\n    }\n}",
    kind: 9,
  },
  {
    label: "signal",
    detail: "Reactive signal (inside store)",
    insert: "signal ${1:name}: ${2:int} = ${3:0};",
    kind: 22, // Field
  },
  {
    label: "computed",
    detail: "Derived computed value (inside store)",
    insert: "computed ${1:name}: ${2:String} {\n    return \"${3:derived}\";\n}",
    kind: 22,
  },
  {
    label: "fn",
    detail: "Top-level or method function",
    insert: "fn ${1:name}(${2:params}) {\n    ${3}\n}",
    kind: 1, // Function
  },
  {
    label: "view",
    detail: "View block inside component",
    insert: "view {\n    ${1}\n}",
    kind: 10, // Module
  },
  {
    label: "if",
    detail: "If statement or directive",
    insert: "if (${1:condition}) {\n    ${2}\n}",
    kind: 15, // Keyword
  },
  {
    label: "for",
    detail: "For loop / directive",
    insert: "for (${1:item} in ${2:items}) {\n    ${3}\n}",
    kind: 15,
  },
  {
    label: "match",
    detail: "Pattern match",
    insert: "match ${1:value} {\n    case ${2:pattern} => ${3:result};\n    default => ${4:fallback};\n}",
    kind: 15,
  },
  {
    label: "import",
    detail: "Import statement",
    insert: "import \"${1:./module.omni}\";",
    kind: 18, // File
  },
  {
    label: "export",
    detail: "Export statement",
    insert: "export ${1:Name};",
    kind: 18,
  },
];

// OmniView component tags
const OMNIVIEW_TAGS: string[] = [
  "Container", "Flex", "Grid", "Stack", "Spacer",
  "Text", "Heading", "Image", "Link", "Divider",
  "Button", "Input", "TextArea", "Checkbox", "Select",
  "List", "ListItem", "Spinner", "Progress", "Alert",
  "Card", "Form", "Table", "TableRow", "TableCell",
  "Tabs", "TabPanel", "Modal", "Drawer", "Tooltip",
  "Icon", "Avatar", "Badge", "Chip", "Switch",
  "Slider", "Radio", "RadioGroup",
];

// Store pattern keywords
const STORE_KEYWORDS: string[] = [
  "signal", "computed", "action", "state",
  "constructor", "initState", "dispose",
];

// Common style properties
const STYLE_PROPS: string[] = [
  "direction", "gap", "padding", "margin",
  "align", "justify", "wrap",
  "width", "height", "minWidth", "minHeight", "maxWidth", "maxHeight",
  "background_color", "color", "opacity",
  "font_size", "font_weight", "font_family", "text_align",
  "border_radius", "border", "border_color", "border_width",
  "shadow", "overflow", "position",
];

// ── Provider ─────────────────────────────────────

export function createCompletionProvider(): languages.CompletionItemProvider {
  return {
    triggerCharacters: [" ", ".", "@", "{", "("],

    provideCompletionItems(model, position): languages.ProviderResult<languages.CompletionList> {
      const word = model.getWordUntilPosition(position);
      const range = {
        startLineNumber: position.lineNumber,
        endLineNumber: position.lineNumber,
        startColumn: word.startColumn,
        endColumn: word.endColumn,
      };

      const line = model.getLineContent(position.lineNumber);
      const textBefore = line.substring(0, position.column - 1).trim();
      const fullText = model.getValue();
      const offset = model.getOffsetAt(position);

      const suggestions: languages.CompletionItem[] = [];

      // ── Top-level suggestions ──
      if (isTopLevel(fullText, offset)) {
        for (const s of TOP_LEVEL_SNIPPETS) {
          suggestions.push({
            label: s.label,
            kind: s.kind,
            detail: s.detail,
            insertText: s.insert,
            insertTextRules: 4, // InsertAsSnippet
            range,
          });
        }
      }

      // ── OmniView tag suggestions (inside view block) ──
      if (isInsideViewBlock(fullText, offset)) {
        for (const tag of OMNIVIEW_TAGS) {
          suggestions.push({
            label: tag,
            kind: 7, // Class
            detail: "OmniView component",
            insertText: `${tag} {\n    \${0}\n}`,
            insertTextRules: 4,
            range,
          });
        }
      }

      // ── Store keywords ──
      if (isInsideStoreBlock(fullText, offset)) {
        for (const kw of STORE_KEYWORDS) {
          suggestions.push({
            label: kw,
            kind: 15,
            detail: `Store: ${kw}`,
            insertText: kw === "signal"
              ? "signal ${1:name}: ${2:type} = ${3:init};"
              : kw === "computed"
              ? "computed ${1:name}: ${2:type} {\n    return ${3:expr};\n}"
              : kw === "action"
              ? "action ${1:name}(${2:params}) {\n    ${3}\n}"
              : kw,
            insertTextRules: 4,
            range,
          });
        }
      }

      // ── Style property suggestions (inside component block) ──
      if (isInsideComponentBlock(fullText, offset)) {
        for (const prop of STYLE_PROPS) {
          suggestions.push({
            label: prop,
            kind: 21, // Property
            detail: "Style property",
            insertText: `${prop}: \${1:value};`,
            insertTextRules: 4,
            range,
          });
        }
      }

      // ── Common type suggestions ──
      if (textBefore.endsWith(":")) {
        const types = ["String", "int", "double", "bool", "void", "num", "dynamic", "List", "Map", "Set"];
        for (const t of types) {
          suggestions.push({
            label: t,
            kind: 17, // TypeParameter
            detail: "Type",
            insertText: t,
            insertTextRules: 0,
            range,
          });
        }
      }

      return { suggestions, incomplete: true };
    },
  };
}

// ── Context Detection Heuristics ──────────────────

function isTopLevel(text: string, offset: number): boolean {
  const prefix = text.substring(0, offset);
  // Count braces to determine nesting
  let depth = 0;
  for (const ch of prefix) {
    if (ch === "{" || ch === "(") depth++;
    else if (ch === "}" || ch === ")") depth--;
  }
  return depth <= 0;
}

function isInsideViewBlock(text: string, offset: number): boolean {
  const prefix = text.substring(0, offset);
  // Look for the most recent "view {" not yet closed
  const lines = prefix.split("\n");
  let depth = 0;
  let inView = false;
  for (const line of lines) {
    if (/\bview\s*\{/.test(line)) { inView = true; }
    for (const ch of line) {
      if (ch === "{") depth++;
      else if (ch === "}") { depth--; if (depth === 1) inView = false; }
    }
  }
  return inView;
}

function isInsideStoreBlock(text: string, offset: number): boolean {
  const prefix = text.substring(0, offset);
  // Match "store Name {" — track depth
  const storeMatch = prefix.match(/\bstore\s+\w+\s*\{/g);
  if (!storeMatch) return false;

  let depth = 0;
  let storeDepth = -1;
  let i = 0;
  for (const ch of prefix) {
    if (ch === "{") {
      depth++;
      // Check if this opening brace corresponds to a store declaration
      const before = prefix.substring(Math.max(0, i - 50), i);
      if (/\bstore\s+\w+\s*$/.test(before.trimEnd())) {
        storeDepth = depth;
      }
    } else if (ch === "}") {
      if (depth === storeDepth) storeDepth = -1;
      depth--;
    }
    i++;
  }
  return storeDepth > 0 && depth >= storeDepth;
}

function isInsideComponentBlock(text: string, offset: number): boolean {
  const prefix = text.substring(0, offset);
  // Inside a component tag block: we're inside a view block and inside {...}
  let depth = 0;
  void (false);
  let componentDepth = -1;

  const lines = prefix.split("\n");
  for (const line of lines) {
    const tagMatch = line.match(/^\s*(Container|Flex|Grid|Button|Text|Heading|Input|Card|List|ListItem|Alert|Form|Select|TextArea|Checkbox|Spinner|Progress|Image|Link)\s*\{/);
    if (tagMatch) {
      componentDepth = depth + 1; // the brace opens on this line
    }
    for (const ch of line) {
      if (ch === "{") {
        depth++;
      } else if (ch === "}") {
        if (depth === componentDepth) { componentDepth = -1; }
        depth--;
      }
    }
  }

  // We're inside a component if componentDepth > 0 and the last char before cursor isn't closing a brace
  return componentDepth > 0 && depth >= componentDepth;
}
