import { describe, it, expect } from "vitest";
import { collectLetVarsPerFn, findOwnerFnId } from "../fn-topology";
import type { CodeblockElementData } from "../../../core/types/element";
import type { WireData } from "../../../core/types/wire";

const el = (id: string, tag: string, name?: string): CodeblockElementData => ({
  id, nodeType: "codeblock", tag, parentId: null, source: "", props: name ? { name } : {},
});
const wire = (id: string, fromId: string, toId: string): WireData => ({ id, fromId, fromPort: "flow-out", toId, toPort: "flow-in" });

describe("collectLetVarsPerFn", () => {
  it("单 fn：收集其 let 节点，忽略 assign/其它", () => {
    const elements = [el("fn1", "fn"), el("let1", "let", "x"), el("let2", "let", "y"), el("a1", "assign")];
    const wires = [wire("w1", "fn1", "let1"), wire("w2", "let1", "a1"), wire("w3", "a1", "let2")];
    const map = collectLetVarsPerFn(elements, wires);
    expect(map.get("fn1")?.map((v) => v.name)).toEqual(["x", "y"]);
  });
  it("多 fn 隔离：BFS 不跨越 fn 边界", () => {
    const elements = [el("fn1", "fn"), el("let1", "let", "x"), el("fn2", "fn"), el("let2", "let", "z")];
    const wires = [wire("w1", "fn1", "let1"), wire("w2", "fn2", "let2")];
    const map = collectLetVarsPerFn(elements, wires);
    expect(map.get("fn1")?.map((v) => v.name)).toEqual(["x"]);
    expect(map.get("fn2")?.map((v) => v.name)).toEqual(["z"]);
  });
  it("经过 condition 分支口（fromPort=br0）仍可达", () => {
    const elements = [el("fn1", "fn"), el("c1", "condition"), el("let1", "let", "x")];
    const wires = [wire("w1", "fn1", "c1"), { id: "w2", fromId: "c1", fromPort: "br0", toId: "let1", toPort: "flow-in" }];
    expect(collectLetVarsPerFn(elements, wires).get("fn1")?.map((v) => v.name)).toEqual(["x"]);
  });
  it("孤岛节点（无 fn 可达）不归属任何 fn", () => {
    const elements = [el("fn1", "fn"), el("let1", "let", "x"), el("orphan", "let", "o")];
    const wires = [wire("w1", "fn1", "let1")];
    const map = collectLetVarsPerFn(elements, wires);
    expect(map.get("fn1")?.map((v) => v.name)).toEqual(["x"]);
    expect(map.get("orphan")).toBeUndefined();
  });
});

describe("findOwnerFnId", () => {
  it("沿 flow-in 链反推所属 fn", () => {
    const elements = [el("fn1", "fn"), el("c1", "condition"), el("let1", "let", "x")];
    const wires = [wire("w1", "fn1", "c1"), wire("w2", "c1", "let1")];
    expect(findOwnerFnId("let1", elements, wires)).toBe("fn1");
  });
  it("fn 自身 → 自身", () => {
    const elements = [el("fn1", "fn")];
    expect(findOwnerFnId("fn1", elements, [])).toBe("fn1");
  });
  it("无 fn 可达 → null", () => {
    const elements = [el("c1", "condition"), el("let1", "let", "x")];
    const wires = [wire("w1", "c1", "let1")];
    expect(findOwnerFnId("let1", elements, wires)).toBeNull();
  });
});
