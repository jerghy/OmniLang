// poiesis/src/services/codegen/__tests__/lib-file-roundtrip.test.ts
// Task 9: lib 文件保存全链路（emitLibFile + ROOT 段 + round-trip，修复实证缺口 G5）。
// 真实 learn-omniview 文件三件套 fixpoint + ROOT 段稳定 + writeback 语义。

import { describe, it, expect } from "vitest";
import * as path from "path";
import * as fs from "fs";
import { parseOmniSource } from "../omni-parser";
import { emitLibFile, emitRootDeclarations } from "../emit-lib-file";

// __tests__ 上溯 5 级到仓库根（poiesis/src/services/codegen/__tests__ → OmniLang 根）。
// 注：brief 原稿用 new URL(rel, import.meta.url)——本仓库 vitest 的 Vite fs 白名单
// 会拦截 poiesis/ 之外的文件（"Denied ID ...?url"），故改用 golden.test.ts 同款
// path.resolve(__dirname, ...) 模式（同为上溯 5 级，语义一致）。
function repoRoot(): string {
  return path.resolve(__dirname, "../../../../..");
}
const FIXTURE = (rel: string) => path.join(repoRoot(), "test/projects/learn-omniview/src", rel);

describe("emitLibFile round-trip（真实 learn-omniview 文件）", () => {
  const fixtures = ["lib/types.omni", "lib/async_demo.omni", "lib/error_handling.omni"];

  for (const rel of fixtures) {
    it(`fixpoint: ${rel}`, () => {
      const src = fs.readFileSync(FIXTURE(rel), "utf-8");
      const r = parseOmniSource(src, rel);
      const def = r.fileDefs![rel];
      expect(def).toBeDefined();
      const out1 = emitLibFile(def, r.codeFlowGraphs ?? {});
      // 纯逻辑文件不发射 import omniview;
      expect(out1).not.toContain("import omniview");
      // 再解析 → 再发射 → 文本不变（pid 注释稳定）
      const r2 = parseOmniSource(out1, rel);
      const out2 = emitLibFile(r2.fileDefs![rel], r2.codeFlowGraphs ?? {});
      expect(out2).toBe(out1);
    });
  }

  it("ROOT 段 round-trip：组件/store 文件头部 struct 保存不丢（FeatureCard/TodoStore 实证）", () => {
    for (const rel of ["components/FeatureCard.omni", "stores/TodoStore.omni"]) {
      const src = fs.readFileSync(FIXTURE(rel), "utf-8");
      const r = parseOmniSource(src, rel);
      const def = r.fileDefs![rel];
      expect(def).toBeDefined();
      if (def.structs.length > 0 || def.types.length > 0) {
        const out1 = emitRootDeclarations(def);
        const r2 = parseOmniSource(out1, rel);
        const out2 = emitRootDeclarations(r2.fileDefs![rel]);
        expect(out2).toBe(out1); // ROOT 段文本稳定
      }
    }
  });

  it("emitRootDeclarations 只含 structs/types（组件/store 头部前置用）", () => {
    const r = parseOmniSource(
      `pub struct Point { x: i32, }\ncomponent A { view { <div id="a"></div> } }\n`,
      "src/components/A.omni",
    );
    const prefix = emitRootDeclarations(r.fileDefs!["src/components/A.omni"]);
    expect(prefix).toContain("pub struct Point {");
    expect(prefix).toContain("x: i32,");
    expect(prefix).not.toContain("import");
  });

  it("画布新建 fn（无磁盘对应）也被发射（writeback 语义）", () => {
    const src = fs.readFileSync(FIXTURE("lib/types.omni"), "utf-8");
    const r = parseOmniSource(src, "src/lib/types.omni");
    const def = r.fileDefs!["src/lib/types.omni"];
    // 手工补一个未在磁盘的 fn 图（模拟画布新建 fn 节点 → writeback 追加 def.fns）
    const extraPid = "fn_extra_el_abc";
    const out = emitLibFile({ ...def, fns: [...def.fns, { pid: extraPid, name: "extra", params: [], returnType: "", body: "", pub: false, isAsync: false, declKind: "fn" }] }, {
      ...(r.codeFlowGraphs ?? {}),
      extra: { elements: [{ id: extraPid, nodeType: "codeblock", tag: "fn", parentId: null, props: { name: "extra", params: "", returnType: "" }, source: "" }], wires: [] },
    });
    expect(out).toContain("fn extra() {");
  });

  it("pub/async 修饰符经 def.fns 落到发射文本（save 全链路保真：async_demo 实证）", () => {
    const src = fs.readFileSync(FIXTURE("lib/async_demo.omni"), "utf-8");
    const r = parseOmniSource(src, "lib/async_demo.omni");
    const def = r.fileDefs!["lib/async_demo.omni"];
    // 解析侧 codeFlowGraphs 的 fn 元素 props 不含 pub/isAsync（synthesizeFnEntry
    // propsExtras 未携带）——emitLibFile 须从 def.fns 补齐，否则保存会静默剥掉 pub/async
    const out1 = emitLibFile(def, r.codeFlowGraphs ?? {});
    expect(out1).toContain("pub async fn task_a() -> String {");
    // 再解析 → 再发射 → pub/async 仍然保留（fixpoint 之外的行为保真）
    const r2 = parseOmniSource(out1, "lib/async_demo.omni");
    const out2 = emitLibFile(r2.fileDefs!["lib/async_demo.omni"], r2.codeFlowGraphs ?? {});
    expect(out2).toContain("pub async fn task_a() -> String {");
  });
});
