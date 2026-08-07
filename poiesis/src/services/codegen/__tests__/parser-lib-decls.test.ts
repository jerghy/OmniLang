// poiesis/src/services/codegen/__tests__/parser-lib-decls.test.ts
// Task 1: ROOT 层声明解析（fileDefs 模型 + parseRootIdentifier 扩展，修复 G2/G3）
// — struct/type/pub async fn 顶层声明进入 fileDefs（按文件路径键）；
//   未注解 body 兜底为 code-block（保存不丢正文）。

import { describe, it, expect } from "vitest";
import { parseOmniSource } from "../omni-parser";

describe("parseOmniSource ROOT 层声明（fileDefs）", () => {
  const src = `import { sleep } from "std/async";

pub struct Point {
  x: i32,
  y: i32,
}
struct nominal UserId { value: i32, }
pub type Percentage = f64;
type ValidationResult = Valid { message: String } | Invalid { errors: Array<String> } | Pending

pub async fn fetch_user(id: u64) -> User {
  return fetch_user(id);
}
fn parse_config(file_path) {
  return 0;
}
`;

  it("struct/type/pub async fn 进入 fileDefs（按路径键）", () => {
    const r = parseOmniSource(src, "src/lib/types.omni");
    const def = r.fileDefs?.["src/lib/types.omni"];
    expect(def).toBeDefined();
    expect(def!.structs.map((s) => s.name)).toEqual(["Point", "UserId"]);
    expect(def!.structs[0]).toMatchObject({ pub: true, nominal: false,
      fields: [{ name: "x", type: "i32" }, { name: "y", type: "i32" }] });
    expect(def!.structs[1]).toMatchObject({ pub: false, nominal: true,
      fields: [{ name: "value", type: "i32" }] });
    expect(def!.types.map((t) => t.name)).toEqual(["Percentage", "ValidationResult"]);
    expect(def!.types[0]).toMatchObject({ pub: true, value: "f64" });
    // union 值：token 无空格拼接（OmniLang 空白不敏感；括号拼接保真）
    expect(def!.types[1].value).toBe("Valid{message:String}|Invalid{errors:Array<String>}|Pending");
    expect(def!.imports).toEqual([`import{sleep}from"std/async";`]);
    expect(def!.fns.map((f) => f.name)).toEqual(["fetch_user", "parse_config"]);
    expect(def!.fns[0]).toMatchObject({ pub: true, isAsync: true, returnType: "User", declKind: "fn" });
    // 无 -> 无类型参数：returnType "" / type ""（error_handling round-trip 关键）
    expect(def!.fns[1]).toMatchObject({ pub: false, isAsync: false, returnType: "" });
    expect(def!.fns[1].params).toEqual([{ name: "file_path", type: "" }]);
    expect(r.codeFlowGraphs?.["fetch_user"]).toBeDefined();
    expect(r.codeFlowGraphs?.["parse_config"]).toBeDefined();
  });

  it("无 filePath 调用（现有调用形态）输出不变——fileDefs 不产出", () => {
    const r = parseOmniSource(src);
    expect(r.fileDefs).toBeUndefined();
    expect(r.codeFlowGraphs?.["fetch_user"]).toBeDefined();
  });

  it("未注解 body 兜底为 code-block（手写源码保真，保存不丢正文）", () => {
    const r = parseOmniSource(`fn parse_config(file_path) {
  let data = fs_read(file_path);
  return data;
}
`, "src/lib/error_handling.omni");
    const g = r.codeFlowGraphs?.["parse_config"];
    expect(g).toBeDefined();
    // 入口 fn 元素 + 兜底 code-block（body 文本）
    expect(g!.elements.some((e) => e.tag === "fn")).toBe(true);
    expect(g!.elements.some((e) => e.tag === "code-block" && (e.source ?? "").includes("fs_read"))).toBe(true);
  });
});
