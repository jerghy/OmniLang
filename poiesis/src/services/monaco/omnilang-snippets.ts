// OmniLang code snippets for Monaco IntelliSense.
// Code blocks use TypeScript language mode — we register on "typescript".

import type { IDisposable, languages as ILanguages } from "monaco-editor";

// ── Guards ──────────────────────────────────────────────────────────
let registered = false;
const disposables: IDisposable[] = [];

// ── Snippet definitions ─────────────────────────────────────────────

interface SnippetDef {
  label: string;
  insertText: string;
  detail: string;
  documentation: string;
}

const snippets: SnippetDef[] = [
  // ── Control flow ──────────────────────────────────────────────
  {
    label: "if",
    insertText: ["if ${1:condition} {", "    ${2}", "}"].join("\n"),
    detail: "OmniLang if statement",
    documentation: "Conditional branch.",
  },
  {
    label: "if else",
    insertText: ["if ${1:condition} {", "    ${2}", "} else {", "    ${3}", "}"].join("\n"),
    detail: "OmniLang if/else",
    documentation: "Conditional with else branch.",
  },
  {
    label: "for",
    insertText: ["for ${1:item} in ${2:iterable} {", "    ${3}", "}"].join("\n"),
    detail: "OmniLang for-in loop",
    documentation: "Iterate over an array.",
  },
  {
    label: "while",
    insertText: ["while ${1:condition} {", "    ${2}", "}"].join("\n"),
    detail: "OmniLang while loop",
    documentation: "Loop while condition holds.",
  },

  // ── Bindings ──────────────────────────────────────────────────
  {
    label: "let",
    insertText: "let ${1:name} = ${2:value};",
    detail: "OmniLang immutable binding",
    documentation: "Declare an immutable variable (type inferred).",
  },
  {
    label: "let mut",
    insertText: "let mut ${1:name} = ${2:value};",
    detail: "OmniLang mutable binding",
    documentation: "Declare a mutable variable.",
  },

  // ── Closures (logic → |pipe| style) ───────────────────────────
  {
    label: "lambda",
    insertText: "|${1:param}| ${2:expression}",
    detail: "OmniLang pipe closure (single param)",
    documentation: "Single-parameter closure: |item| item.id != target.",
  },
  {
    label: "lambda2",
    insertText: "|${1:x}, ${2:y}| ${3:expression}",
    detail: "OmniLang pipe closure (multi param)",
    documentation: "Multi-parameter closure: |a, b| a + b.",
  },

  // ── Closures (view → () => {} style) ──────────────────────────
  {
    label: "arrow",
    insertText: "() => { ${1} }",
    detail: "OmniLang arrow closure (no params)",
    documentation: "Zero-parameter arrow closure: () => { … }.",
  },
  {
    label: "arrow1",
    insertText: "(${1:param}) => { ${2} }",
    detail: "OmniLang arrow closure (single param)",
    documentation: "Single-parameter arrow closure.",
  },

  // ── Other ─────────────────────────────────────────────────────
  {
    label: "return",
    insertText: "return ${1:value};",
    detail: "OmniLang return",
    documentation: "Return a value from a function or action.",
  },
  {
    label: "match",
    insertText: [
      "match ${1:value} {",
      "    ${2:Pattern} => ${3:expression},",
      "    _ => ${4:fallback}",
      "}",
    ].join("\n"),
    detail: "OmniLang match",
    documentation: "Pattern-match on a tagged union.",
  },
  {
    label: "try",
    insertText: ["try {", "    ${1}", "} catch ${2:err} {", "    ${3}", "}"].join("\n"),
    detail: "OmniLang try/catch",
    documentation: "Handle errors with try/catch.",
  },
  {
    label: "import",
    insertText: 'import { ${1:Name} } from "${2:path}";',
    detail: "OmniLang import",
    documentation: "Import from a module or standard library.",
  },
];

// ── Registration ────────────────────────────────────────────────────

export async function initOmniLangSnippets(): Promise<void> {
  if (registered) return;
  registered = true;

  const { languages } = await import("monaco-editor");

  const disposable = languages.registerCompletionItemProvider("typescript", {
    provideCompletionItems: (
      model,
      position,
    ): ILanguages.ProviderResult<ILanguages.CompletionList> => {
      const word = model.getWordUntilPosition(position);
      const range = {
        startLineNumber: position.lineNumber,
        endLineNumber: position.lineNumber,
        startColumn: word.startColumn,
        endColumn: word.endColumn,
      };

      const suggestions: ILanguages.CompletionItem[] = snippets.map(
        (s): ILanguages.CompletionItem => ({
          label: s.label,
          kind: languages.CompletionItemKind.Snippet,
          insertText: s.insertText,
          insertTextRules:
            languages.CompletionItemInsertTextRule.InsertAsSnippet,
          detail: s.detail,
          documentation: s.documentation,
          range,
        }),
      );
      return { suggestions };
    },
  });
  disposables.push(disposable);
}

function disposeSnippets(): void {
  for (const d of disposables) d.dispose();
  disposables.length = 0;
  registered = false;
}
