/**
 * Monarch tokenizer for OmniLang (.omni) — provides syntax highlighting in Monaco.
 *
 * OmniLang keywords are drawn from the Lark grammar. We define a custom language
 * registration so Monaco highlights .omni files correctly without a full language server.
 */

import type { languages } from "monaco-editor";

export const OMNILANG_LANGUAGE_ID = "omnilang";

export const conf: languages.LanguageConfiguration = {
  comments: { lineComment: "//", blockComment: ["/*", "*/"] },
  brackets: [
    ["{", "}"],
    ["[", "]"],
    ["(", ")"],
  ],
  autoClosingPairs: [
    { open: "{", close: "}" },
    { open: "[", close: "]" },
    { open: "(", close: ")" },
    { open: '"', close: '"' },
    { open: "'", close: "'" },
    { open: "`", close: "`" },
  ],
  surroundingPairs: [
    { open: "{", close: "}" },
    { open: "[", close: "]" },
    { open: "(", close: ")" },
    { open: '"', close: '"' },
    { open: "'", close: "'" },
    { open: "`", close: "`" },
  ],
};

/**
 * Monarch tokenizer rules for OmniLang.
 *
 * Matches the .omni grammar tokens used by the Lark parser:
 *   component | store | signal | computed | fn | view | import | export |
 *   if / else / for / while / match | let | var | const |
 *   Container | Button | Text | … (OmniView built-in tags via convention)
 */
export const language: languages.IMonarchLanguage = {
  defaultToken: "",
  tokenPostfix: ".omni",

  keywords: [
    "component", "store", "signal", "computed", "fn", "view",
    "import", "export", "as", "from",
    "if", "else", "for", "while", "match", "case", "default",
    "return", "yield", "await", "async",
    "let", "var", "const", "final", "late",
    "class", "enum", "struct", "trait", "impl", "extends", "super",
    "this", "self", "new", "throw", "try", "catch", "finally",
    "break", "continue", "in", "is",
    "pub", "static", "override", "required", "mixin", "with", "on",
    "use", "namespace", "module",
    "true", "false", "null", "undefined",
  ],

  typeKeywords: [
    "String", "int", "double", "bool", "void", "num", "dynamic",
    "List", "Map", "Set", "Future", "Stream", "Option", "Result",
    "Widget", "BuildContext", "Element",
  ],

  operators: [
    "=", ">", "<", "!", "~", "?", ":",
    "==", "!=", ">=", "<=", "&&", "||",
    "+", "-", "*", "/", "%", "**", "~/",
    "+=", "-=", "*=", "/=", "%=",
    "=>", "->", "??", "?.", "..",
    "|", "&", "^", "<<", ">>",
  ],

  symbols: /[=><!~?:&|+\-*/%^]+/,

  escapes: /\\(?:[abfnrtv\\"']|x[0-9A-Fa-f]{1,4}|u[0-9A-Fa-f]{4}|U[0-9A-Fa-f]{8})/,

  tokenizer: {
    root: [
      // Whitespace
      [/[ \t\r\n]+/, "white"],

      // Comments
      [/\/\/.*$/, "comment"],
      [/\/\*/, "comment", "@blockComment"],

      // String interpolation (OmniView) — highlight {{...}} in strings
      [/{{/, "delimiter.curly.omni"],

      // Strings
      [/'/, "string", "@singleString"],
      [/"/, "string", "@doubleString"],
      [/`/, "string", "@templateString"],

      // Numbers
      [/\d*\.\d+([eE][-+]?\d+)?/, "number.float"],
      [/0[xX][0-9a-fA-F]+/, "number.hex"],
      [/\d+/, "number"],

      // Annotations / directives
      [/@[\w.]+/, "annotation"],

      // Identifiers and keywords
      [
        /[a-zA-Z_$][\w$]*/,
        {
          cases: {
            "@typeKeywords": "type",
            "@keywords": "keyword",
            "@default": "identifier",
          },
        },
      ],

      // Delimiters
      [/[{}()[\]]/, "@brackets"],
      [/[;,.]/, "delimiter"],

      // Operators
      [/@symbols/, {
        cases: {
          "@operators": "operator",
          "@default": "",
        },
      }],
    ],

    blockComment: [
      [/[^*]+/, "comment"],
      [/\*\//, "comment", "@pop"],
      [/\*/, "comment"],
    ],

    doubleString: [
      [/[^\\"]+/, "string"],
      [/@escapes/, "string.escape"],
      [/\\./, "string.escape.invalid"],
      [/"/, "string", "@pop"],
    ],

    singleString: [
      [/[^\\']+/, "string"],
      [/@escapes/, "string.escape"],
      [/\\./, "string.escape.invalid"],
      [/[']/, "string", "@pop"],
    ],

    templateString: [
      [/[^\\`$]+/, "string"],
      [/@escapes/, "string.escape"],
      [/\\./, "string.escape.invalid"],
      [/\$\{/, { token: "delimiter.bracket", next: "@interpolation" }],
      [/`/, "string", "@pop"],
    ],

    interpolation: [
      [/[{}]/, "delimiter.bracket"],
      [/'[^']*'/, "string"],
      [/"[^"]*"/, "string"],
      [/\d+/, "number"],
      [/[a-zA-Z_]\w*/, "identifier"],
      [/[ \t\r\n]+/, "white"],
      [/\}/, { token: "delimiter.bracket", next: "@pop" }],
    ],
  },
};
