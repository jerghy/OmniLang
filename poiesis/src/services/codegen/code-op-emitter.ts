// poiesis/src/services/codegen/code-op-emitter.ts
import type { CodeOp } from "../../core/types/code-op";

export function emitOp(op: CodeOp, indent: number): string {
  const pad = "  ".repeat(indent);

  switch (op.kind) {
    case "declare_var": {
      const mut = op.mutable ? " mut" : "";
      const type = op.varType ? `: ${op.varType}` : "";
      const init = op.initValue !== undefined ? ` = ${op.initValue}` : "";
      return `${pad}let${mut} ${op.name}${type}${init};\n`;
    }
    case "set_var":
      return `${pad}${op.name} ${op.operator} ${op.value};\n`;
    case "if_block": {
      let out = "";
      for (let i = 0; i < op.branches.length; i++) {
        const br = op.branches[i];
        const prefix = i === 0 ? "" : br.condition ? " else if" : " else";
        // For the first branch there's no prefix; for subsequent ones we
        // combine the closing brace of the previous branch with the next keyword.
        if (i === 0) {
          out += `${pad}if`;
        } else {
          // Remove the trailing newline + indent from the previous closing brace
          // so "} else {" lands on one line.
          out = out.replace(/\n$/, "");
          out += `${prefix}`;
        }
        const cond = br.condition ? ` ${br.condition}` : "";
        out += `${cond} {\n`;
        for (const child of br.body) out += emitOp(child, indent + 1);
        out += `${pad}}\n`;
      }
      return out;
    }
    case "while": {
      let out = `${pad}while ${op.condition} {\n`;
      for (const child of op.body) out += emitOp(child, indent + 1);
      out += `${pad}}\n`;
      return out;
    }
    case "for_in": {
      let out = `${pad}for ${op.itemName} in ${op.collection} {\n`;
      for (const child of op.body) out += emitOp(child, indent + 1);
      out += `${pad}}\n`;
      return out;
    }
    case "return":
      return `${pad}return${op.value ? " " + op.value : ""};\n`;
    case "call_fn":
      return `${pad}${op.fnName}(${op.args.join(", ")});\n`;
    case "await":
      return `${pad}await ${op.value};\n`;
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
  return ops.map(op => emitOp(op, 0)).join("");
}
