// poiesis/src/services/codegen/code-op-emitter.ts
// Converts CodeOp[] back into OmniLang source text.
// Used by BlockEditor to produce .omni source from Blockly blocks.

import type { CodeOp } from "../../core/types/code-op";
import type { SlotValue } from "../../core/types/slot-value";
import { slotToText } from "../../core/types/slot-value";

/** Extract text from a SlotValue, returning empty string for empty/undefined. */
function sv(s: SlotValue | undefined): string {
  if (!s) return "";
  return slotToText(s);
}

export function emitOp(op: CodeOp, indent: number): string {
  const pad = "  ".repeat(indent);

  switch (op.kind) {
    case "declare_var": {
      const mut = op.mutable ? " mut" : "";
      const type = sv(op.varType);
      const typeStr = type ? `: ${type}` : "";
      const init = sv(op.initValue);
      const initStr = init ? ` = ${init}` : "";
      return `${pad}let${mut} ${sv(op.name)}${typeStr}${initStr};\n`;
    }
    case "set_var":
      return `${pad}${sv(op.name)} ${op.operator} ${sv(op.value)};\n`;
    case "if_block": {
      let out = "";
      for (let i = 0; i < op.branches.length; i++) {
        const br = op.branches[i];
        const cond = sv(br.condition);
        const prefix = i === 0 ? "" : cond ? " else if" : " else";
        if (i === 0) {
          out += `${pad}if`;
        } else {
          out = out.replace(/\n$/, "");
          out += `${prefix}`;
        }
        out += `${cond ? " " + cond : ""} {\n`;
        for (const child of br.body) out += emitOp(child, indent + 1);
        out += `${pad}}\n`;
      }
      return out;
    }
    case "while": {
      let out = `${pad}while ${sv(op.condition)} {\n`;
      for (const child of op.body) out += emitOp(child, indent + 1);
      out += `${pad}}\n`;
      return out;
    }
    case "for_in": {
      let out = `${pad}for ${sv(op.itemName)} in ${sv(op.collection)} {\n`;
      for (const child of op.body) out += emitOp(child, indent + 1);
      out += `${pad}}\n`;
      return out;
    }
    case "return":
      return `${pad}return${sv(op.value) ? " " + sv(op.value) : ""};\n`;
    case "call_fn": {
      const args = op.args.map((a) => sv(a)).filter(Boolean);
      return `${pad}${sv(op.fnName)}(${args.join(", ")});\n`;
    }
    case "await":
      return `${pad}await ${sv(op.value)};\n`;
    case "break":
      return `${pad}break;\n`;
    case "continue":
      return `${pad}continue;\n`;
    case "comment":
      return `${pad}// ${op.text}\n`;
    case "raw":
      return op.source;
  }
}

export function emitOps(ops: CodeOp[]): string {
  return ops.map((op) => emitOp(op, 0)).join("");
}
