// poiesis/src/services/codegen/__tests__/graph-parser-midchain.test.ts
// 中链 condition round-trip 回归（E2E 缺陷 2）：
// 1. 中链 condition（前驱 → condition → 分支体 → 后置段）parse 后 wires 完整——
//    线形重建此前跳过 condition 段（无 flow-in wire，重载成孤岛）；
//    末分支 `} else {` 过渡此前挂在分支体段尾被 2c 短路漏检（末分支并入当前分支）。
// 2. compile → parse → compile 幂等（结构不丢、语句 props 不被尾行污染）。
// 3. 尾行结构行（`}` / `else if` / `else {`）不进入 let/assign/call 的 props 解析。

import { describe, it, expect } from "vitest";
import { parseCodeFlowGraph } from "../graph-parser";
import { compileCodeFlowGraph } from "../graph-compiler";
import type { WireData } from "../../../core/types/wire";

/** 中链 condition 源：let → assign → condition(3 分支) → 后置段（flow-out 续接）。 */
const MIDCHAIN_SRC = `fn handleAdd() -> void {
        // @pid:let-1
          let total = 0;
        // @pid:assign-2
          total = total + 1;
        // @pid:cond-3
          if total > 0 {
        // @pid:call-4
            emit onSelect();
          }
          else if total > 10 {
        // @pid:ret-5
            return;
          }
          else {
        // @pid:ret-6
            return;
          }
        // @pid:post-7
          total = total * 2;
    }`;

function wireList(wires: WireData[]): string[] {
  return wires
    .map((w) => `${w.fromId}(${w.fromPort})->${w.toId}(${w.toPort})`)
    .sort();
}

function findEl(g: { elements: Array<{ id: string; tag: string; props: Record<string, unknown>; source: string }> }, id: string) {
  return g.elements.find((e) => e.id === id);
}

describe("graph-parser 中链 condition", () => {
  it("parse 后 wires 完整：前驱→condition(flow-in)、三分支、flow-out→后置段", () => {
    const g = parseCodeFlowGraph(MIDCHAIN_SRC);

    // 入链：前驱 flow-out → condition flow-in（旧实现缺失 → 孤岛）
    expect(wireList(g.wires)).toContain("assign-2(flow-out)->cond-3(flow-in)");
    // 三分支（旧实现末分支并入 elif → br2 wire 缺失）
    expect(wireList(g.wires)).toContain("cond-3(br0)->call-4(flow-in)");
    expect(wireList(g.wires)).toContain("cond-3(br1)->ret-5(flow-in)");
    expect(wireList(g.wires)).toContain("cond-3(br2)->ret-6(flow-in)");
    // 后置段：flow-out 续接（编译器对称语义）
    expect(wireList(g.wires)).toContain("cond-3(flow-out)->post-7(flow-in)");
    expect(wireList(g.wires)).toContain("let-1(flow-out)->assign-2(flow-in)");

    // 分支体正确归位：br1 只含 ret-5（不被 br0 链或 br2 链交叉）
    const br1Wires = g.wires.filter((w) => w.fromId === "cond-3" && w.fromPort === "br1");
    expect(br1Wires.map((w) => w.toId)).toEqual(["ret-5"]);
  });

  it("compile → parse → compile 幂等：结构逐字保持", () => {
    const g1 = parseCodeFlowGraph(MIDCHAIN_SRC);
    const out1 = compileCodeFlowGraph(g1.elements, g1.wires, "let-1");

    const g2 = parseCodeFlowGraph(out1);
    const out2 = compileCodeFlowGraph(g2.elements, g2.wires, "let-1");

    // 结构幂等：二次发射与首次发射一致（分支、条件、语句全部保持）
    expect(out2).toBe(out1);
    // wire 集幂等
    expect(wireList(g2.wires)).toEqual(wireList(g1.wires));
  });

  it("语句 props 不被尾行结构行污染（call 的 emit/fnName 保持）", () => {
    const g = parseCodeFlowGraph(MIDCHAIN_SRC);
    const callEl = findEl(g, "call-4");
    // 旧实现 parseSyntaxProps 收到 "emit onSelect();\n}\nelse if total > 10 {"
    // （发射侧把收尾 `}` 与下一分支头挂在语句段尾）→ 正则失配回退 {fnName:"fn"} → "fn();"
    expect(callEl!.props).toMatchObject({ emit: true, fnName: "onSelect" });
    expect(callEl!.source).toBe("emit onSelect();");

    const assignEl = findEl(g, "assign-2");
    expect(assignEl!.props).toMatchObject({ target: "total", operator: "=", value: "total + 1" });

    // 后置段同样不受污染
    const postEl = findEl(g, "post-7");
    expect(postEl!.props).toMatchObject({ target: "total", value: "total * 2" });
  });

  it("条件首段形态不回归：无前驱时 condition 无入链，flow-out 续接仍成立", () => {
    const src = `fn f() -> void {
        // @pid:cond-0
          if a > 0 {
        // @pid:then-1
          b();
          }
        // @pid:post-2
          c();
    }`;
    const g = parseCodeFlowGraph(src);
    // 首段 condition 无前驱可入链（旧行为保持）
    expect(g.wires.some((w) => w.toId === "cond-0")).toBe(false);
    // flow-out 续接保持
    expect(wireList(g.wires)).toContain("cond-0(flow-out)->post-2(flow-in)");
    expect(wireList(g.wires)).toContain("cond-0(br0)->then-1(flow-in)");
  });
});

// 中链 loop 线形重建（Task 3）：loop 段头压入 branchCtx 栈（body 段 wire 到 br0）+
// 续接循环覆盖 loop（flow-out 续接）——与 condition 对称。
describe("graph-parser 中链 loop", () => {
  const MIDCHAIN_LOOP = `fn handleAdd() -> void {
    // @pid:let-1
      let total = 0;
    // @pid:w-2
      while total < 10 {
    // @pid:assign-3
        total = total + 1;
      }
    // @pid:ret-4
      return total;
  }`;

  it("中链 while：入链 + body(br0) + flow-out 续接完整", () => {
    const g = parseCodeFlowGraph(MIDCHAIN_LOOP);
    const w = wireList(g.wires);
    expect(w).toContain("let-1(flow-out)->w-2(flow-in)");
    expect(w).toContain("w-2(br0)->assign-3(flow-in)");
    expect(w).toContain("w-2(flow-out)->ret-4(flow-in)");
  });

  it("loop 嵌套在 condition 分支体内（branchCtx 栈复用）", () => {
    const g = parseCodeFlowGraph(`fn f() {
  // @pid:cond-1
    if ok {
  // @pid:w-2
      while n > 0 {
  // @pid:a-3
        n = n - 1;
      }
    }
  // @pid:ret-4
    return;
  }`);
    const w = wireList(g.wires);
    expect(w).toContain("cond-1(br0)->w-2(flow-in)");
    expect(w).toContain("w-2(br0)->a-3(flow-in)");
  });

  it("for-in 段元素 props/source 派生（round-trip 用）", () => {
    const g = parseCodeFlowGraph(`fn f() {
  // @pid:fi-1
    for item in items {
  // @pid:a-2
      total = total + item;
    }
  }`);
    const el = g.elements.find((e) => e.id === "fi-1");
    expect(el?.tag).toBe("for-in");
    expect(el?.props.varName).toBe("item");
    expect(el?.props.iterable).toBe("items");
    expect(el?.source).toBe("for item in items {");
  });
});
