// poiesis/src/components/canvas/code/card-editor/row-utils.ts
// EditorRow ↔ CodeOp[] conversion

import type { CodeOp } from "../../../../core/types/code-op";
import { uid } from "../../../../core/utils/id";

export type EditorRow =
  | { kind: "input"; id: string; text: string }
  | { kind: "card"; op: CodeOp; indent: number }
  | { kind: "close-brace"; indent: number }
  | { kind: "empty"; id: string };

/** Convert CodeOp[] → EditorRow[] for display in the line editor. */
export function opsToRows(ops: CodeOp[]): EditorRow[] {
  const rows: EditorRow[] = [];
  for (const op of ops) {
    appendOp(rows, op, 0);
  }
  // trailing input row
  rows.push({ kind: "input", id: uid("in"), text: "" });
  return rows;
}

function appendOp(rows: EditorRow[], op: CodeOp, indent: number): void {
  switch (op.kind) {
    case "if_block": {
      for (let i = 0; i < op.branches.length; i++) {
        const br = op.branches[i];
        const headOp: CodeOp = {
          kind: "if_block",
          pid: i === 0 ? op.pid : uid("op"),
          branches: [{ condition: br.condition, body: [] }],
        };
        rows.push({ kind: "card", op: headOp, indent: indent + (i > 0 ? 0 : 0) });
        // body
        for (const child of br.body) {
          appendOp(rows, child, indent + 1);
        }
      }
      rows.push({ kind: "close-brace", indent });
      break;
    }
    case "while":
      rows.push({ kind: "card", op, indent });
      for (const child of op.body) appendOp(rows, child, indent + 1);
      rows.push({ kind: "close-brace", indent });
      break;
    case "for_in":
      rows.push({ kind: "card", op, indent });
      for (const child of op.body) appendOp(rows, child, indent + 1);
      rows.push({ kind: "close-brace", indent });
      break;
    default:
      rows.push({ kind: "card", op, indent });
      break;
  }
}

/** Convert EditorRow[] → CodeOp[] when saving. */
export function rowsToOps(rows: EditorRow[]): CodeOp[] {
  return collectOps(rows, 0, 0).ops;
}

function collectOps(
  rows: EditorRow[],
  startIndex: number,
  targetIndent: number,
): { ops: CodeOp[]; nextIndex: number } {
  const ops: CodeOp[] = [];
  let i = startIndex;

  while (i < rows.length) {
    const row = rows[i];

    if (row.kind === "close-brace" && row.indent === targetIndent) {
      return { ops, nextIndex: i + 1 };
    }
    if (row.kind === "close-brace") {
      // closing brace at wrong depth — skip
      i++;
      continue;
    }

    if (row.kind === "card") {
      if (row.op.kind === "if_block") {
        // collect elif/else branches
        const branches: { condition: string; body: CodeOp[] }[] = [];
        let cond = row.op.branches[0]?.condition ?? "";
        const { ops: body, nextIndex: afterBody } = collectOps(rows, i + 1, targetIndent + 1);
        branches.push({ condition: cond, body });
        i = afterBody; // past close-brace

        // check for elif/else continuation
        while (i < rows.length && rows[i].kind === "card" && (rows[i] as Extract<EditorRow, { kind: "card" }>).op.kind === "if_block") {
          const elifOp = (rows[i] as Extract<EditorRow, { kind: "card" }>).op as Extract<CodeOp, { kind: "if_block" }>;
          cond = elifOp.branches[0]?.condition ?? "";
          const { ops: elifBody, nextIndex: afterElif } = collectOps(rows, i + 1, targetIndent + 1);
          branches.push({ condition: cond, body: elifBody });
          i = afterElif;
        }

        ops.push({
          kind: "if_block",
          pid: row.op.pid,
          branches,
        });
        continue;
      }

      if (row.op.kind === "while") {
        const { ops: body, nextIndex: afterBody } = collectOps(rows, i + 1, targetIndent + 1);
        ops.push({ ...row.op, body });
        i = afterBody;
        continue;
      }

      if (row.op.kind === "for_in") {
        const { ops: body, nextIndex: afterBody } = collectOps(rows, i + 1, targetIndent + 1);
        ops.push({ ...row.op, body });
        i = afterBody;
        continue;
      }

      // leaf op
      ops.push(row.op);
      i++;
      continue;
    }

    // skip input/empty rows
    i++;
  }

  return { ops, nextIndex: i };
}
