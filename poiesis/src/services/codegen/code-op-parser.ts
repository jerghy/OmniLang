// poiesis/src/services/codegen/code-op-parser.ts
// Parse Omni source text into structured CodeOp[] for the CodeCard Editor.
// JS regex-based parser with WASM extension for Phase 2.

import type { CodeOp } from "../../core/types/code-op";
import { uid } from "../../core/utils/id";

/** Detect the CodeOp kind from the first non-empty line of Omni source. */
export function detectOpKind(firstLine: string): CodeOp["kind"] | "raw" {
  const t = firstLine.trim();
  if (/^let\s+mut\s/.test(t) || /^let\s+\w/.test(t)) return "declare_var";
  if (/^if\s+/.test(t) || /^else\s+if\s+/.test(t) || /^else\s*\{/.test(t)) return "if_block";
  if (/^while\s+/.test(t)) return "while";
  if (/^for\s+\w+\s+in\s+/.test(t)) return "for_in";
  if (/^return\b/.test(t)) return "return";
  if (/^break\s*;?\s*$/.test(t)) return "break";
  if (/^continue\s*;?\s*$/.test(t)) return "continue";
  if (/^await\s+/.test(t)) return "await";
  if (/^\w+\s*\(/.test(t)) return "call_fn";
  if (/^\w+\s*(=|\+=|-=|\*=|\/=|%=)/.test(t)) return "set_var";
  if (/^\/\//.test(t)) return "comment";
  return "raw";
}

function braceDepth(line: string): number {
  let d = 0;
  for (const ch of line) {
    if (ch === "{") d++;
    if (ch === "}") d--;
  }
  return d;
}

function extractWhileCondition(line: string): string {
  const m = line.trim().match(/^while\s+(.+?)\s*\{?\s*$/);
  return m ? m[1].trim() : "true";
}

function extractForParts(line: string): { itemName: string; collection: string } {
  const m = line.trim().match(/^for\s+(\w+)\s+in\s+(.+?)\s*\{?\s*$/);
  return { itemName: m?.[1] ?? "item", collection: m?.[2]?.trim() ?? "items" };
}

function extractDeclareVar(line: string) {
  const m = line.trim().match(
    /^let(\s+mut)?\s+(\w+)(?:\s*:\s*(\w+(?:<.+?>)?))?(?:\s*=\s*(.+?))?\s*;?\s*$/
  );
  return {
    mutable: !!m?.[1],
    name: m?.[2] ?? "x",
    varType: m?.[3],
    initValue: m?.[4],
  };
}

function extractSetVar(line: string) {
  const m = line.trim().match(/^(\w+)\s*(=|\+=|-=|\*=|\/=|%=)\s*(.+?)\s*;?\s*$/);
  return { name: m?.[1] ?? "x", operator: (m?.[2] ?? "=") as any, value: m?.[3] ?? "" };
}

function extractReturnExpr(line: string): string | undefined {
  const m = line.trim().match(/^return\s+(.+?)\s*;?\s*$/);
  return m?.[1];
}

function extractCallFn(line: string) {
  const m = line.trim().match(/^(\w+)\s*\(([^)]*)\)\s*;?\s*$/);
  return {
    fnName: m?.[1] ?? "fn",
    args: m?.[2] ? m[2].split(",").map(a => a.trim()).filter(Boolean) : [],
  };
}

function extractAwaitExpr(line: string): string {
  const m = line.trim().match(/^await\s+(.+?)\s*;?\s*$/);
  return m?.[1] ?? "expr";
}

function buildBlockBody(lines: string[], startI: number, startDepth: number): { bodyLines: string[]; nextI: number } {
  const bodyLines: string[] = [];
  let i = startI + 1;
  let depth = startDepth + 1;
  while (i < lines.length) {
    const trimmed = lines[i].trimStart();
    if (trimmed.startsWith("}")) break;
    depth += braceDepth(lines[i]);
    if (depth <= startDepth) break;
    bodyLines.push(lines[i]);
    i++;
  }
  return { bodyLines: bodyLines.map(l => l.replace(/^\s{2,}/, "  ")), nextI: i };
}

function buildIfBranches(lines: string[], startI: number): { branches: { condition: string; body: CodeOp[] }[]; nextI: number } {
  const branches: { condition: string; body: CodeOp[] }[] = [];
  let i = startI;

  while (i < lines.length) {
    const line = lines[i].trim();
    let condition = "";

    if (line.startsWith("if ")) {
      condition = line.slice(3).replace(/\s*\{?\s*$/, "").trim();
    } else if (line.startsWith("else if ")) {
      condition = line.slice(8).replace(/\s*\{?\s*$/, "").trim();
    } else if (line.startsWith("else")) {
      condition = "";
    } else {
      break;
    }

    const startDepth = braceDepth(lines[i]);
    const { bodyLines, nextI } = buildBlockBody(lines, i, startDepth);
    branches.push({ condition, body: parseOps(bodyLines.join("\n")) });
    i = nextI;

    // Check for next branch continuation (} else { or } else if)
    if (i < lines.length && /^\}\s*(else|$)/.test(lines[i].trim()) && lines[i].trim().startsWith("}")) {
      const closeLine = lines[i].trim();
      if (closeLine.includes("else if")) {
        const condPart = closeLine.replace(/^\}\s*else\s+if\s+/, "").replace(/\s*\{?\s*$/, "").trim();
        condition = condPart;
        const sd = braceDepth(lines[i]);
        const { bodyLines: bl, nextI: ni } = buildBlockBody(lines, i, sd);
        branches.push({ condition, body: parseOps(bl.join("\n")) });
        i = ni;
      } else if (closeLine.includes("else")) {
        condition = "";
        const sd = braceDepth(lines[i]);
        const { bodyLines: bl, nextI: ni } = buildBlockBody(lines, i, sd);
        branches.push({ condition, body: parseOps(bl.join("\n")) });
        i = ni;
      } else {
        i++; // just close brace
      }
    }
  }

  return { branches, nextI: i };
}

/** Parse Omni source text into structured CodeOp[]. */
export function parseOps(source: string): CodeOp[] {
  const lines = source.split("\n");
  const ops: CodeOp[] = [];
  let i = 0;

  while (i < lines.length) {
    const line = lines[i];
    const trimmed = line.trim();

    if (trimmed === "" || trimmed === "}" || trimmed.startsWith("} else")) {
      i++;
      continue;
    }

    const kind = detectOpKind(trimmed);

    switch (kind) {
      case "declare_var": {
        const info = extractDeclareVar(trimmed);
        ops.push({
          kind: "declare_var", pid: uid("op"),
          mutable: info.mutable, name: info.name,
          varType: info.varType, initValue: info.initValue,
          scope: "fn",
        });
        i++;
        break;
      }
      case "set_var": {
        const info = extractSetVar(trimmed);
        ops.push({ kind: "set_var", pid: uid("op"), ...info });
        i++;
        break;
      }
      case "if_block": {
        const { branches, nextI } = buildIfBranches(lines, i);
        ops.push({ kind: "if_block", pid: uid("op"), branches });
        i = nextI;
        break;
      }
      case "while": {
        const condition = extractWhileCondition(trimmed);
        const startDepth = braceDepth(line);
        const { bodyLines, nextI } = buildBlockBody(lines, i, startDepth);
        ops.push({ kind: "while", pid: uid("op"), condition, body: parseOps(bodyLines.join("\n")) });
        i = nextI;
        break;
      }
      case "for_in": {
        const { itemName, collection } = extractForParts(trimmed);
        const startDepth = braceDepth(line);
        const { bodyLines, nextI } = buildBlockBody(lines, i, startDepth);
        ops.push({ kind: "for_in", pid: uid("op"), itemName, collection, body: parseOps(bodyLines.join("\n")) });
        i = nextI;
        break;
      }
      case "return":
        ops.push({ kind: "return", pid: uid("op"), value: extractReturnExpr(trimmed) });
        i++;
        break;
      case "call_fn": {
        const info = extractCallFn(trimmed);
        ops.push({ kind: "call_fn", pid: uid("op"), ...info });
        i++;
        break;
      }
      case "await":
        ops.push({ kind: "await", pid: uid("op"), value: extractAwaitExpr(trimmed) });
        i++;
        break;
      case "break":
        ops.push({ kind: "break", pid: uid("op") });
        i++;
        break;
      case "continue":
        ops.push({ kind: "continue", pid: uid("op") });
        i++;
        break;
      case "comment":
        ops.push({ kind: "comment", pid: uid("op"), text: trimmed.replace(/^\/\/\s*/, "") });
        i++;
        break;
      case "raw":
        ops.push({ kind: "raw", pid: uid("op"), source: trimmed });
        i++;
        break;
    }
  }

  return ops;
}

// ── WASM-first parse function ──

export async function parseOpsWasm(source: string): Promise<CodeOp[]> {
  try {
    const wasm = await import("../../wasm/omnilang_wasm.js");
    const result = wasm.parse_code_block(source);
    const parsed: { success: boolean; stmts: any[]; errors: string[] } =
      typeof result === "string" ? JSON.parse(result) : result;

    if (!parsed.success && parsed.stmts.length === 0) {
      // Fall back to JS regex parser
      return parseOps(source);
    }

    return stmtsToCodeOps(parsed.stmts);
  } catch {
    // WASM not available — fall back to JS regex
    return parseOps(source);
  }
}

function stmtsToCodeOps(stmts: any[]): CodeOp[] {
  return stmts.map((stmt: any): CodeOp => {
    // Map Rust AST enum variants to CodeOp
    // The Rust Stmt enum uses tagged variants: { "VarDecl": { mutable, name, ... } }
    if (stmt.VarDecl) {
      const d = stmt.VarDecl;
      return {
        kind: "declare_var",
        pid: uid("op"),
        mutable: d.mutable ?? false,
        name: d.name ?? "x",
        varType: d.var_type ?? d.varType,
        initValue: d.init_value ?? d.initValue ?? d.init,
        scope: "fn",
      };
    }
    if (stmt.Assign) {
      const s = stmt.Assign;
      return {
        kind: "set_var",
        pid: uid("op"),
        name: s.target?.Name ?? s.name ?? "x",
        operator: "=",
        value: s.value ?? "",
      };
    }
    if (stmt.If) {
      const i = stmt.If;
      return {
        kind: "if_block",
        pid: uid("op"),
        branches: [
          {
            condition: i.condition ?? "",
            body: stmtsToCodeOps(i.then_body?.stmts ?? []),
            ...(i.else_body ? { elseBody: stmtsToCodeOps(i.else_body.stmts ?? []) } : {}),
          },
        ],
      };
    }
    if (stmt.While) {
      const w = stmt.While;
      return {
        kind: "while",
        pid: uid("op"),
        condition: w.condition ?? "true",
        body: stmtsToCodeOps(w.body?.stmts ?? []),
      };
    }
    if (stmt.ForIn) {
      const f = stmt.ForIn;
      return {
        kind: "for_in",
        pid: uid("op"),
        itemName: f.var_name ?? f.itemName ?? "item",
        collection: f.iterable ?? f.collection ?? "items",
        body: stmtsToCodeOps(f.body?.stmts ?? []),
      };
    }
    if (stmt.Return) {
      return {
        kind: "return",
        pid: uid("op"),
        value: stmt.Return.value ?? "",
      };
    }
    if (stmt.Break) {
      return { kind: "break", pid: uid("op") };
    }
    if (stmt.Continue) {
      return { kind: "continue", pid: uid("op") };
    }
    if (stmt.ExprStmt) {
      // Try to extract a call expression from ExprStmt
      const expr = stmt.ExprStmt.expr;
      if (expr.FuncCall) {
        return {
          kind: "call_fn",
          pid: uid("op"),
          fnName: expr.FuncCall.func?.VarRef ?? "fn",
          args: (expr.FuncCall.args ?? []).map((a: any) =>
            typeof a === "object" ? a.Literal?.String ?? a.Literal?.Int ?? JSON.stringify(a) : String(a)
          ),
        };
      }
    }
    // Fallback for unmapped stmts
    return {
      kind: "raw",
      pid: uid("op"),
      source: JSON.stringify(stmt),
    };
  });
}
