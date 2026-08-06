// 代码样本提取（spec D10）：从 fn 赋值字面量提取 struct 数组的真实样本。
import { describe, it, expect } from "vitest";
import { extractAssignSamples } from "../sample-extractor";

const INIT_ROWS = `
fn initRows() {
    rows = [
        RowItem { id: 1, name: "Alpha", status: "Active", score: 95 },
        RowItem { id: 2, name: "Bravo", status: "Active", score: 87 },
        RowItem { id: 3, name: "Charlie", status: "Inactive", score: 72 },
    ];
}
`;

describe("extractAssignSamples", () => {
  it("提取 name = [ ... ] 数组字面量（平衡括号）", () => {
    const samples = extractAssignSamples([INIT_ROWS], "rows");
    expect(samples.length).toBe(1);
    expect(samples[0].label).toContain("3 项");
    expect(samples[0].literal).toContain("RowItem { id: 1");
  });

  it("无该信号的赋值 → 空数组", () => {
    expect(extractAssignSamples([INIT_ROWS], "other")).toEqual([]);
  });

  it("赋值含函数调用/展开 → 跳过", () => {
    const src = "fn add() {\n  rows = [...rows, RowItem { id: 9 }];\n}\n";
    expect(extractAssignSamples([src], "rows")).toEqual([]);
  });

  it("多 fn 各自提取（最多 3 个样本）", () => {
    const srcs = [
      "fn a() {\n  rows = [RowItem { id: 1 }];\n}\n",
      "fn b() {\n  rows = [RowItem { id: 2 }, RowItem { id: 3 }];\n}\n",
    ];
    const samples = extractAssignSamples(srcs, "rows");
    expect(samples.length).toBe(2);
    expect(samples[0].label).toContain("1 项");
    expect(samples[1].label).toContain("2 项");
  });

  it("样本字面量去重（相同内容只留一个）", () => {
    const srcs = [
      "fn a() {\n  rows = [RowItem { id: 1 }];\n}\n",
      "fn b() {\n  rows = [RowItem { id: 1 }];\n}\n",
    ];
    expect(extractAssignSamples(srcs, "rows").length).toBe(1);
  });
});
