// poiesis/src/services/codegen/__tests__/tokenizer-compound-assign.test.ts
// 复合赋值运算符（+= -= *= /= %=）tokenizer 单 token 保真 + 解析闭环。
//
// 回归背景（2026-08-10 真机）：main.omni 里 `count += 1;` 经 TS tokenizer
// 被拆成 TEXT "+" + EQUALS "="，fn 体重建变 `count + = 1 ;`，graph-parser 的
// assign 正则失配 → 降级 code-block 原始文本 → 画布重编译时 WASM 报
// "Unexpected token in expression: Eq"。修复：tokenizer 把复合赋值作为
// 单 TEXT token（text 即运算符原文），fn 体重建保持 `count += 1;`。

import { describe, it, expect, beforeAll } from "vitest";
import { readFileSync } from "fs";
import path from "path";
import { tokenize } from "../omni-tokenizer";
import { parseOmniSource } from "../omni-parser";
import { compileCodeFlowGraph } from "../graph-compiler";
import type { CodeblockElementData } from "../../../core/types/element";
import type { WireData } from "../../../core/types/wire";

const OPS = ["+=", "-=", "*=", "/=", "%="];

describe("tokenize 复合赋值单 token", () => {
  it.each(OPS)("`count %s 1;` 产出单 TEXT token（text 保真）", (op) => {
    const toks = tokenize(`count ${op} 1;`);
    const opTok = toks.find((t) => t.text === op);
    expect(opTok).toBeDefined();
    expect(opTok!.kind).toBe("TEXT");
    // 必须没有任何 `+` `=` 之类的拆分残留（如 `+` 与 `=` 分离）
    expect(toks.filter((t) => t.text === op).length).toBe(1);
    // 重建（tok.text + " " 语义）后运算符仍在原位置
    const rebuilt = toks.map((t) => t.text).join(" ");
    expect(rebuilt).toContain(`count ${op} 1`);
  });

  it("普通加法 `a + b` 不受影响（仍是独立 TEXT '+'）", () => {
    const toks = tokenize("a + b;");
    const plus = toks.find((t) => t.text === "+");
    expect(plus).toBeDefined();
    expect(plus!.kind).toBe("TEXT");
    expect(toks.filter((t) => t.text === "=").length).toBe(0);
  });

  it("自增 `i++` 不受影响（`++` 不是复合赋值，保持两个 TEXT）", () => {
    const toks = tokenize("i++;");
    expect(toks.filter((t) => t.text === "+").length).toBe(2);
  });
});

describe("fn 体解析闭环：复合赋值 → assign 节点", () => {
  it.each(OPS)("fn body 含 `total %s 1;` → codeFlowGraphs assign 节点 operator 保真", (op) => {
    const src = `import omniview;

component C {
    fn bump() -> void {
        total ${op} 1;
    }
}
`;
    const r = parseOmniSource(src, "src/components/C.omni");
    const graph = r?.codeFlowGraphs?.["bump"];
    expect(graph).toBeDefined();
    const assignEl = graph!.elements.find((e) => e.tag === "assign");
    expect(assignEl).toBeDefined();
    const props = (assignEl as CodeblockElementData).props as { target?: string; operator?: string; value?: string };
    expect(props.target).toBe("total");
    expect(props.operator).toBe(op);
    expect(props.value).toBe("1");
  });

  it("graph-compiler 发射 `count += 1;` 保持运算符", () => {
    const src = `import omniview;

component C {
    fn bump() -> void {
        count += 1;
    }
}
`;
    const r = parseOmniSource(src, "src/components/C.omni");
    const graph = r!.codeFlowGraphs!["bump"];
    const fnEl = graph.elements.find((e) => e.tag === "fn") as CodeblockElementData;
    const compiled = compileCodeFlowGraph(graph.elements as CodeblockElementData[], graph.wires as WireData[], fnEl.id);
    expect(compiled).toContain("count += 1;");
    expect(compiled).not.toContain("count + = 1");
  });
});

// ── TS→WASM 门禁：复合赋值 fn 体过真实 WASM 编译器零 Error ──
let wasmModule: typeof import("../../../wasm/omnilang_wasm.js") | null = null;

async function loadWasm(): Promise<typeof import("../../../wasm/omnilang_wasm.js")> {
  if (wasmModule) return wasmModule;
  const mod = await import("../../../wasm/omnilang_wasm.js");
  const wasmPath = path.join(process.cwd(), "src/wasm/omnilang_wasm_bg.wasm");
  await mod.default(readFileSync(wasmPath));
  wasmModule = mod;
  return mod;
}

describe("WASM 门禁：复合赋值 fn 体可编译", () => {
  beforeAll(async () => {
    await loadWasm();
  });

  it.each(OPS)("`total %s 1;` 的组件源码过 WASM 零 error", async (op) => {
    const src = `import omniview;

component C {
    fn bump() -> void {
        let total = 0;
        total ${op} 1;
    }
    view {
        <button @onPress={bump}>go</button>
    }
}
`;
    const mod = await loadWasm();
    const envelope = mod.compile_multi_to_omniview({ "src/components/C.omni": src }, false, false, undefined) as any;
    const errors = envelope?.result?.errors ?? envelope?.errors ?? [];
    expect(errors.filter((e: any) => e.severity === "error")).toEqual([]);
  });
});
