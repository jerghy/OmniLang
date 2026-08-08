// @vitest-environment node
/// <reference types="node" />
// poiesis/src/services/codegen/__tests__/event-emission-wasm-validity.test.ts
// 事件意图化 TS→Rust 门禁（Task 4）。
//
// 阶段 1 语义化后，旧 DOM 拼写（@click/@onClick/@tap/@onTap/@mouseenter/
// @onMouseEnter/@mouseleave/@onMouseLeave）已是 Rust 编译期硬错误（E0006），
// 编辑器的 TS 管线必须在「读侧归一化（parser-view）+ 发射侧防御映射
// （source-emitter）」双端把旧拼写收敛为语义名（onPress/onEnter/onLeave）。
//
// 本测试做三件事：
//   1) 纯 TS：toSemanticEventName 映射表单测（含大小写变体、透传名）。
//   2) 纯 TS：parser-view 归一化 + source-emitter 发射往返 → 语义名。
//   3) TS→Rust 门禁：把 componentProtocol.ts 导出的 ALL_EDITOR_EVENT_NAMES
//      （编辑器可见事件全集）逐一绑定到组件 fn，构造 .omni view 源码，
//      过真实 WASM 编译器（compile_multi_to_omniview），断言零 Error——
//      与 Rust event_names.rs 同步，防止 TS/Rust 双端漂移（ADR-005 风险）。

import { describe, it, expect, beforeAll } from "vitest";
import { readFileSync } from "fs";
import path from "path";
import { parseOmniSource } from "../omni-parser";
import { SourceEmitter } from "../source-emitter";
import type { EmitInput, ElementViewNode } from "../types";
import { toSemanticEventName, semanticEventName } from "../event-name-map";
import { ALL_EDITOR_EVENT_NAMES } from "../../../data/componentProtocol";
import type { ViewElementData } from "../../../core/types/element";

/** 取解析结果中指定 tag 的 view 元素（ElementData 联合类型收窄为 ViewElementData）。 */
function viewElements(elements: unknown[] | null | undefined, tag: string): ViewElementData[] {
  return (elements ?? []).filter(
    (el): el is ViewElementData => (el as { nodeType?: string }).nodeType === "view" && (el as { tag?: string }).tag === tag
  );
}

// ── WASM compiler bootstrap ──────────────────────────────────────────────
let wasmModule: typeof import("../../../wasm/omnilang_wasm.js") | null = null;

async function loadWasm(): Promise<typeof import("../../../wasm/omnilang_wasm.js")> {
  if (wasmModule) return wasmModule;
  const mod = await import("../../../wasm/omnilang_wasm.js");
  const wasmPath = path.join(process.cwd(), "src/wasm/omnilang_wasm_bg.wasm");
  await mod.default(readFileSync(wasmPath));
  wasmModule = mod;
  return mod;
}

interface WasmDiag {
  severity?: string;
  code?: string;
  message?: string;
  line?: number;
  file?: string;
}

/** Compile .omni view sources through the REAL WASM compiler. */
async function compileViewSources(sources: Record<string, string>): Promise<WasmDiag[]> {
  const mod = await loadWasm();
  const envelope = mod.compile_multi_to_omniview(sources, false, false, undefined) as {
    result?: { errors?: WasmDiag[] };
    errors?: WasmDiag[];
  };
  return envelope?.result?.errors ?? envelope?.errors ?? [];
}

/** 构造含指定事件绑定的 .omni view 源码（每个事件绑定到组件 fn handleX）。
 *  语法以 test/projects/learn-omniview/src/components/Counter.omni 为基准：
 *  组件段（signal/computed/fn…）→ view → style，无 props 块。 */
function buildViewSource(events: string[]): string {
  const fnDecls = events
    .map((evt) => `    fn handle_${evt.replace(/^on/, "").toLowerCase()}() {}`)
    .join("\n");
  const bindings = events
    .map((evt) => `        <button @${evt}={handle_${evt.replace(/^on/, "").toLowerCase()}}>btn</button>`)
    .join("\n");
  return `import omniview;

component EventGate {
${fnDecls}

    view {
        <div>
${bindings}
        </div>
    }
    style {
    }
}
`;
}

// ── Pure-TS: 映射表单测 ─────────────────────────────────────────────────

describe("toSemanticEventName（语义名映射表，镜像 Rust event_names.rs）", () => {
  it("click/press/tap 族（含大小写变体）→ onPress", () => {
    expect(toSemanticEventName("click")).toBe("onPress");
    expect(toSemanticEventName("Click")).toBe("onPress");
    expect(toSemanticEventName("onClick")).toBe("onPress");
    expect(toSemanticEventName("onclick")).toBe("onPress");
    expect(toSemanticEventName("ONCLICK")).toBe("onPress");
    expect(toSemanticEventName("press")).toBe("onPress");
    expect(toSemanticEventName("onPress")).toBe("onPress");
    expect(toSemanticEventName("tap")).toBe("onPress");
    expect(toSemanticEventName("onTap")).toBe("onPress");
  });

  it("enter/mouseenter 族（含大小写变体）→ onEnter", () => {
    expect(toSemanticEventName("enter")).toBe("onEnter");
    expect(toSemanticEventName("onEnter")).toBe("onEnter");
    expect(toSemanticEventName("mouseenter")).toBe("onEnter");
    expect(toSemanticEventName("onMouseEnter")).toBe("onEnter");
    expect(toSemanticEventName("onMOUSEENTER")).toBe("onEnter");
  });

  it("leave/mouseleave 族（含大小写变体）→ onLeave", () => {
    expect(toSemanticEventName("leave")).toBe("onLeave");
    expect(toSemanticEventName("onLeave")).toBe("onLeave");
    expect(toSemanticEventName("mouseleave")).toBe("onLeave");
    expect(toSemanticEventName("onMouseLeave")).toBe("onLeave");
  });

  it("透传名（不在映射表内）返回 null", () => {
    expect(toSemanticEventName("onKeyDown")).toBeNull();
    expect(toSemanticEventName("onMouseOver")).toBeNull();
    expect(toSemanticEventName("onHover")).toBeNull();
    expect(toSemanticEventName("onLongPress")).toBeNull();
    expect(toSemanticEventName("onPointerDown")).toBeNull();
    expect(toSemanticEventName("onChange")).toBeNull();
  });

  it("semanticEventName 恒等语义化：表内收敛、表外原样", () => {
    expect(semanticEventName("onClick")).toBe("onPress");
    expect(semanticEventName("onMouseEnter")).toBe("onEnter");
    expect(semanticEventName("onMouseLeave")).toBe("onLeave");
    expect(semanticEventName("onKeyDown")).toBe("onKeyDown");
  });
});

// ── Pure-TS: parser-view 归一化往返 ─────────────────────────────────────

describe("parser-view 读侧归一化（旧 DOM 拼写 → 语义名）", () => {
  it("@click/@onClick/@tap/@onTap → props.onPress", () => {
    const src = buildViewSource(["click", "onClick", "tap", "onTap"]);
    const result = parseOmniSource(src);
    expect(result.errors).toEqual([]);
    const buttons = viewElements(result.designElements, "button");
    expect(buttons.length).toBe(4);
    const names = buttons.map((b) =>
      Object.keys(b.props).filter((k) => k.startsWith("on"))
    );
    for (const n of names) {
      expect(n).toEqual(["onPress"]);
    }
  });

  it("@mouseenter/@onMouseEnter/@enter → props.onEnter；@mouseleave/@onMouseLeave/@leave → props.onLeave", () => {
    const src = buildViewSource(["mouseenter", "onMouseEnter", "enter", "mouseleave", "onMouseLeave", "leave"]);
    const result = parseOmniSource(src);
    expect(result.errors).toEqual([]);
    const buttons = viewElements(result.designElements, "button");
    expect(buttons.length).toBe(6);
    const names = buttons.map((b) =>
      Object.keys(b.props).filter((k) => k.startsWith("on"))
    );
    for (const n of names.slice(0, 3)) {
      expect(n).toEqual(["onEnter"]);
    }
    for (const n of names.slice(3)) {
      expect(n).toEqual(["onLeave"]);
    }
  });

  it("透传名（onHover/onKeyDown）原样保留，不做语义化", () => {
    const src = buildViewSource(["onHover", "onKeyDown"]);
    const result = parseOmniSource(src);
    expect(result.errors).toEqual([]);
    const buttons = viewElements(result.designElements, "button");
    expect(buttons.length).toBe(2);
    const names = buttons.map((b) =>
      Object.keys(b.props).filter((k) => k.startsWith("on"))
    );
    expect(names[0]).toEqual(["onHover"]);
    expect(names[1]).toEqual(["onKeyDown"]);
  });
});

// ── Pure-TS: source-emitter 发射侧防御映射 ──────────────────────────────

function emitComponentWithView(view: ElementViewNode): string {
  const emitter = new SourceEmitter();
  const input: EmitInput = {
    codeTabType: "logic",
    imports: ["import omniview;"],
    stores: [],
    component: {
      status: "valid",
      name: "EmitGate",
      signals: {},
      computeds: {},
      storeRefs: [],
      fns: [
        { name: "handlePress", params: [], returnType: "", body: "" },
        { name: "handleEnter", params: [], returnType: "", body: "" },
        { name: "handleLeave", params: [], returnType: "", body: "" },
        { name: "handleHover", params: [], returnType: "", body: "" },
      ],
      style: {},
      view,
      consts: [],
      props: [],
      events: [],
    },
  };
  return emitter.emitDocument(input, new Map());
}

function elementNode(props: Record<string, unknown>, eventBindings?: Record<string, string>): ElementViewNode {
  // eventBindings 未提供时省略 resolvedStyle：source-emitter 的
  // style?.eventBindings 分支对空对象 {} 恒真，会吞掉 props 兜底分支——
  // 只有不存在 resolvedStyle 时 props 分支才会执行。
  const node: ElementViewNode = {
    id: "v_btn",
    path: "EmitGate.v_btn",
    kind: "element",
    tag: "button",
    props,
    children: [],
  };
  if (eventBindings !== undefined) {
    node.resolvedStyle = {
      htmlTag: "button",
      cssClassName: "",
      contentText: "btn",
      htmlAttrs: {},
      eventBindings,
      bindings: {},
      cssRules: {},
      usedClassNames: [],
    };
  }
  return node;
}

describe("source-emitter 发射侧防御映射（旧拼写发射前收敛为语义名）", () => {
  it("eventBindings 分支：onClick→onPress / onMouseEnter→onEnter / onMouseLeave→onLeave", () => {
    const out = emitComponentWithView(
      elementNode(
        {},
        { onClick: "handlePress", onMouseEnter: "handleEnter", onMouseLeave: "handleLeave" }
      )
    );
    expect(out).toContain("@onPress={handlePress}");
    expect(out).toContain("@onEnter={handleEnter}");
    expect(out).toContain("@onLeave={handleLeave}");
    expect(out).not.toContain("@onClick");
    expect(out).not.toContain("@onMouseEnter");
    expect(out).not.toContain("@onMouseLeave");
  });

  it("props 兜底分支：props.onClick/onMouseLeave → 语义名发射", () => {
    const out = emitComponentWithView(
      elementNode({ onClick: "handlePress", onMouseLeave: "handleLeave" })
    );
    expect(out).toContain("@onPress={handlePress}");
    expect(out).toContain("@onLeave={handleLeave}");
    expect(out).not.toContain("@onClick");
    expect(out).not.toContain("@onMouseLeave");
  });

  it("透传名（onHover/onKeyDown）发射时原样保留", () => {
    const out = emitComponentWithView(
      elementNode({}, { onHover: "handleHover", onKeyDown: "handlePress" })
    );
    expect(out).toContain("@onHover={handleHover}");
    expect(out).toContain("@onKeyDown={handlePress}");
  });
});

// ── TS→Rust 门禁：编辑器可见事件全集过真实 WASM 编译器 ─────────────────

describe("事件意图化 TS→Rust 门禁（ALL_EDITOR_EVENT_NAMES 零 Error）", () => {
  beforeAll(async () => {
    await loadWasm();
  });

  it("ALL_EDITOR_EVENT_NAMES（componentProtocol 单一来源）全部编译零 Error", async () => {
    // 单一事实源：编辑器可见事件全集（通用 ∪ 组件特殊）。若新增事件名
    // 未入此表，这里会漏测——componentProtocol.ts 的注释强制补上。
    expect(ALL_EDITOR_EVENT_NAMES.length).toBeGreaterThan(30);

    const src = buildViewSource([...ALL_EDITOR_EVENT_NAMES]);
    const diagnostics = await compileViewSources({ "src/views/EventGate.omni": src });
    const errors = diagnostics.filter((d) => d.severity === "error");
    // 列出失败事件名，方便定位是哪个名字与 Rust event_names.rs 失配
    const failMsgs = errors.map((e) => e.message ?? JSON.stringify(e));
    expect(failMsgs, `WASM 编译零 Error 断言失败：\n${failMsgs.join("\n")}`).toEqual([]);
  });

  it("负向对照：旧 DOM 拼写 @click 在阶段 1 编译器中是硬错误（E0006）", async () => {
    const src = buildViewSource(["click"]);
    const diagnostics = await compileViewSources({ "src/views/Legacy.omni": src });
    const errors = diagnostics.filter((d) => d.severity === "error");
    expect(errors.length).toBeGreaterThan(0);
  });
});
