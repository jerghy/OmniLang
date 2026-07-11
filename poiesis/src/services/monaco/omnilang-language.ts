// Monarch tokenizer + language registration for OmniLang (.omni) in Monaco.
// Import this file as a side-effect to register the "omnilang" language.

import { languages } from "monaco-editor";

// ── Guards ──────────────────────────────────────────────────────────
let registered = false;

// ── Keywords ────────────────────────────────────────────────────────
const keywords = [
  // Top-level
  "component", "store", "design", "struct", "type",
  "import", "export", "pub",
  // Component-level
  "signal", "computed", "prop", "event", "fn", "action", "getter",
  "view", "style", "ref",
  // Store-level (overlap handled by context-free Monarch)
  // Design-level
  "token", "class", "variant", "base",
  // Statements
  "if", "else", "for", "while", "return", "break", "continue",
  "match", "try", "catch", "finally", "throw", "emit",
  // Declarations
  "let", "mut", "async", "await",
  // Output
  "print", "println",
  // Literals
  "true", "false", "null",
  // Modifiers
  "nominal", "extends", "as", "from",
];

const primitiveTypes = [
  "i8", "i16", "i32", "i64", "isize",
  "u8", "u16", "u32", "u64", "usize",
  "f32", "f64", "bool", "String", "Void",
  "Array", "Map", "HtmlElement",
];

const tokenTypes = [
  "color", "size", "radius", "fontFamily", "fontSize", "fontWeight",
  "lineHeight", "letterSpacing", "shadow", "duration", "easing",
  "opacity", "zIndex",
];

const stdLibFunctions = [
  "console_log", "console_warn", "console_error",
  "http_get", "http_post",
  "json_parse", "json_stringify",
  "array_filter", "array_map", "array_find", "array_contains",
  "array_sort", "array_length", "array_push", "array_remove",
  "string_length", "string_contains", "string_starts_with",
  "string_ends_with", "string_trim", "string_concat", "string_split",
  "string_join", "string_replace", "string_substring",
  "string_to_upper", "string_to_lower",
  "math_abs", "math_ceil", "math_floor", "math_round",
  "math_sqrt", "math_pow", "math_sin", "math_cos", "math_tan",
  "math_log", "math_log2", "math_log10",
  "math_min", "math_max", "math_clamp",
  "math_random", "math_random_int", "math_pi", "math_e",
  "storage_get", "storage_set",
  "datetime_now", "datetime_format",
  "async_sleep", "async_spawn", "async_all", "async_race", "async_timeout",
  "encode_base64_encode", "encode_base64_decode",
  "encode_hex_encode", "encode_hex_decode",
  "encode_url_encode", "encode_url_decode",
  "regex_match", "regex_search", "regex_replace",
];

// ── Combined tokenizer rules ────────────────────────────────────────
const monarchLanguage: languages.IMonarchLanguage = {
  defaultToken: "invalid",

  keywords,
  typeKeywords: [...primitiveTypes, ...tokenTypes],
  functions: stdLibFunctions,

  tokenizer: {
    root: [
      // ── Whitespace ───────────────────────────────────────────
      [/[ \t\r\n]+/, "white"],

      // ── Comments ──────────────────────────────────────────────
      [/\/\/.*$/, "comment"],
      [/\/\*/, "comment", "@blockComment"],

      // ── Strings ───────────────────────────────────────────────
      [/`/, "string", "@templateString"],
      [/"([^"\\]|\\.)*$/, "string.invalid"],   // non-terminated
      [/"/, "string", "@stringDouble"],
      [/'([^'\\]|\\.)*$/, "string.invalid"],
      [/'/, "string", "@stringSingle"],

      // ── Annotations ───────────────────────────────────────────
      [/@[a-zA-Z_][a-zA-Z0-9_-]*/, "annotation"],

      // ── Numbers ────────────────────────────────────────────────
      [/0[xX][0-9a-fA-F]+/, "number.hex"],
      [/0[oO][0-7]+/, "number.octal"],
      [/0[bB][01]+/, "number.binary"],
      [/\d*\.\d+([eE][\-+]?\d+)?/, "number.float"],
      [/\d+[eE][\-+]?\d+/, "number.float"],
      [/\d+/, "number"],

      // ── Operators ──────────────────────────────────────────────
      [/->/, "operator"],
      [/=>/, "operator"],
      [/::/, "operator"],
      [/==|!=|<=|>=/, "operator"],
      [/&&|\|\||\?\?/, "operator"],
      [/[+\-*/%=]=/, "operator"],
      [/\.\.\./, "operator"],
      [/\.\./, "operator"],
      [/[+\-*/%]/, "operator"],
      [/[!]=?/, "operator"],

      // ── Punctuation / delimiters ───────────────────────────────
      [/[{}]/, "delimiter.curly"],
      [/[()]/, "delimiter.parenthesis"],
      [/[<>]/, "delimiter.angle"],
      [/[\[\]]/, "delimiter.square"],
      [/;/, "delimiter"],
      [/[:,]/, "delimiter"],
      [/[.|]/, "delimiter"],

      // ── Identifiers ────────────────────────────────────────────
      [
        /[a-zA-Z_][a-zA-Z0-9_-]*/,
        {
          cases: {
            "@keywords": "keyword",
            "@typeKeywords": "type",
            "@functions": "function",
            "@default": "identifier",
          },
        },
      ],
    ],

    // ── Sub-states ────────────────────────────────────────────────
    blockComment: [
      [/\*\//, "comment", "@pop"],
      [/./, "comment"],
    ],

    stringDouble: [
      [/[^\\"]+/, "string"],
      [/\\./, "string.escape"],
      [/"/, "string", "@pop"],
    ],

    stringSingle: [
      [/[^\\']+/, "string"],
      [/\\./, "string.escape"],
      [/'/, "string", "@pop"],
    ],

    templateString: [
      [/\$\{/, "delimiter", "@templateExpr"],
      [/[^\\`$]+/, "string"],
      [/\\[$`\\]/, "string.escape"],
      [/`/, "string", "@pop"],
    ],

    templateExpr: [
      [/[{}]/, "delimiter.curly"],
      [/[a-zA-Z_]\w*/, "identifier"],
      [/[0-9]+/, "number"],
      [/["'`]/, "string", "@pop"],  // nested strings pop back to template
      [/\}/, "delimiter.curly", "@pop"],
      [/./, "identifier"],
    ],
  },
};

// ── Language configuration (brackets, auto-closing, comments) ──────

const langConfig: languages.LanguageConfiguration = {
  comments: {
    lineComment: "//",
    blockComment: ["/*", "*/"],
  },
  brackets: [
    ["{", "}"],
    ["(", ")"],
    ["<", ">"],
    ["[", "]"],
  ],
  autoClosingPairs: [
    { open: "{", close: "}" },
    { open: "(", close: ")" },
    { open: "<", close: ">" },
    { open: "[", close: "]" },
    { open: '"', close: '"' },
    { open: "'", close: "'" },
    { open: "`", close: "`" },
  ],
  surroundingPairs: [
    { open: "{", close: "}" },
    { open: "(", close: ")" },
    { open: "<", close: ">" },
    { open: "[", close: "]" },
    { open: '"', close: '"' },
    { open: "'", close: "'" },
    { open: "`", close: "`" },
  ],
};

// ── Register ───────────────────────────────────────────────────────

export function initOmniLangLanguage(): void {
  if (registered) return;
  registered = true;

  languages.register({ id: "omnilang" });
  languages.setMonarchTokensProvider("omnilang", monarchLanguage);
  languages.setLanguageConfiguration("omnilang", langConfig);
}
