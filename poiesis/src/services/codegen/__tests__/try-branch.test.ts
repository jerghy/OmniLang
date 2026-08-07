import { describe, it, expect } from "vitest";
import { parseCodeFlowGraph } from "../graph-parser";
import { compileCodeFlowGraph } from "../graph-compiler";
import type { WireData } from "../../../core/types/wire";

/** try 中链源：镜像 graph-parser-midchain.test.ts 的段约定——
 *  fn wrapper 首行（无 pid，被 splitByPid 丢弃）+ 每 statement 一个 `// @pid:xxx` 注释
 *  + 延续行（`} catch err {` / `} finally {`）裸挂在上一 statement 段尾（无 pid）：
 *  结构行不生成元素，由 2d pendingTransition + parseSyntaxProps stmtEnd 截断处理。 */
const TRY_SRC = `fn f() {
// @pid:try-1
try {
// @pid:let-2
let raw = fs_read(file_path);
} catch err {
// @pid:call-4
log_error(err);
} finally {
// @pid:call-6
cleanup();
}
}
`;

function wireList(wires: WireData[]): string[] {
  return wires
    .map((w) => `${w.fromId}(${w.fromPort})->${w.toId}(${w.toPort})`)
    .sort();
}

describe("graph-parser try 块", () => {
  it("解析：try/catch/finally 三分支 + catchVar 提取 + 分支 wire", () => {
    const graph = parseCodeFlowGraph(TRY_SRC);
    const tryEl = graph.elements.find((e) => e.tag === "try");
    expect(tryEl).toBeDefined();
    const branches = tryEl!.props?.branches as Array<{ id: string; type: string; catchVar?: string }>;
    expect(branches.map((b) => b.type)).toEqual(["try", "catch", "finally"]);
    expect(branches.map((b) => b.id)).toEqual(["br0", "br1", "br2"]);
    expect(branches[1].catchVar).toBe("err");
    // 分支体归位：br0→let-2、br1→call-4、br2→call-6
    expect(wireList(graph.wires)).toContain("try-1(br0)->let-2(flow-in)");
    expect(wireList(graph.wires)).toContain("try-1(br1)->call-4(flow-in)");
    expect(wireList(graph.wires)).toContain("try-1(br2)->call-6(flow-in)");
  });

  it("编译 → 再解析 → 再编译幂等（新行风格发射 + fixpoint）", () => {
    const g1 = parseCodeFlowGraph(TRY_SRC);
    const out1 = compileCodeFlowGraph(g1.elements, g1.wires, "try-1")!;
    // 发射镜像 condition：每分支 `${keyword}${catchVarStr} {\n`——catch 行无前导 `}`
    expect(out1).toContain("try {");
    expect(out1).toContain("catch err {");
    expect(out1).toContain("finally {");
    expect(out1).not.toContain("} catch err {");
    expect(out1).toContain("log_error(err);");
    expect(out1).toContain("cleanup();");

    const g2 = parseCodeFlowGraph(out1);
    const out2 = compileCodeFlowGraph(g2.elements, g2.wires, "try-1")!;
    expect(out2).toBe(out1);
    expect(wireList(g2.wires)).toEqual(wireList(g1.wires));
  });

  it("catch 无变量形式（catch {）", () => {
    const graph = parseCodeFlowGraph(`fn f() {
// @pid:t-1
try {
// @pid:a-2
do_something();
} catch {
// @pid:b-3
recover();
}
}
`);
    const branches = graph.elements.find((e) => e.tag === "try")!.props?.branches as Array<{ type: string; catchVar?: string }>;
    expect(branches[1]).toMatchObject({ type: "catch", catchVar: undefined });
    expect(wireList(graph.wires)).toContain("t-1(br1)->b-3(flow-in)");
  });
});
