// poiesis/src/store/__tests__/behavior.test.ts
// M3 T2 行为卡落盘：applyBehaviorCard → 自动 fn 生成 + 语义事件指向 + emitter 往返闭环。
import { describe, it, expect } from "vitest";
import { usePoiesisStore } from "../index";
import { generateOmniSource } from "../../services/codegen/pipeline";
import { parseOmniSource } from "../../services/codegen/omni-parser";
import type { ElementData, CodeblockElementData } from "../../core/types/element";
import type { WireData } from "../../core/types/wire";

const basicCodeTabs = [
  { id: "main", type: "logic", label: "Main", icon: "code", graph: { elements: [], wires: [] }, variables: [] },
] as const;

function setupButton() {
  usePoiesisStore.setState({
    elements: [
      { id: "btn-1", nodeType: "view", tag: "button", props: { label: "Click me" }, children: [], parentId: null } as ElementData,
    ],
    wires: [],
    rootOrder: ["btn-1"],
    isDirty: false,
  });
  return usePoiesisStore.getState();
}

describe("applyBehaviorCard", () => {
  it("提交行为卡 → 生成 fn 元素 + 语义事件指向 + dirty", () => {
    const state = setupButton();
    state.applyBehaviorCard("btn-1", { elementId: "btn-1", event: "click", action: "add", params: { target: "count", amount: "1" } });

    const el = usePoiesisStore.getState().elements;
    const fns = el.filter((e) => "tag" in e && e.tag === "fn");
    expect(fns).toHaveLength(1);
    const fn = fns[0] as CodeblockElementData;
    expect(fn.props.name).toMatch(/^on[A-Z]/);
    expect(fn.props.name).toBe("onPressAuto"); // click → 语义 onPress → fn 名
    expect(fn.parentId).toBeNull();

    const btn = el.find((e) => e.id === "btn-1");
    expect(btn && "props" in btn ? (btn.props as Record<string, unknown>).onPress : undefined).toBe("onPressAuto");
    expect(usePoiesisStore.getState().isDirty).toBe(true);
  });

  it("fn 体来自动作模板：code-block 元素 + flow 线", () => {
    const state = setupButton();
    state.applyBehaviorCard("btn-1", { elementId: "btn-1", event: "click", action: "add", params: { target: "count", amount: "1" } });

    const el = usePoiesisStore.getState().elements;
    const fn = el.find((e) => "tag" in e && e.tag === "fn") as CodeblockElementData | undefined;
    const body = el.find((e) => "tag" in e && e.tag === "code-block") as CodeblockElementData | undefined;
    expect(body).toBeTruthy();
    expect(body!.source).toContain("count = count + (1);");

    const wires = usePoiesisStore.getState().wires;
    const wire = wires.find((w: WireData) => w.fromId === fn!.id);
    expect(wire).toBeTruthy();
    expect(wire!.fromPort).toBe("flow-out");
    expect(wire!.toId).toBe(body!.id);
    expect(wire!.toPort).toBe("flow-in");
  });

  it("emitter 往返：@onPress={fn} 落盘 + 解析回 fn/事件引用", () => {
    const state = setupButton();
    state.applyBehaviorCard("btn-1", { elementId: "btn-1", event: "click", action: "add", params: { target: "count", amount: "1" } });

    const result = generateOmniSource({
      designElements: usePoiesisStore.getState().elements as ElementData[],
      designWires: usePoiesisStore.getState().wires,
      codeTabs: basicCodeTabs as unknown as Parameters<typeof generateOmniSource>[0]["codeTabs"],
      componentName: "App",
    });
    expect(result.errors.filter((e) => e.severity === "fatal")).toHaveLength(0);
    expect(result.value).toContain("fn onPressAuto()");
    expect(result.value).toContain("@onPress={onPressAuto}");

    const r = parseOmniSource(result.value ?? "");
    expect(r.codeFlowGraphs?.["onPressAuto"]?.elements[0]?.props?.name).toBe("onPressAuto");
    // 事件引用解析回视图元素 props（parser-helpers 将 attrs.events 折入 ElementData.props）
    const btn = r.designElements?.find((e) => e.id === "btn-1");
    expect(btn && "props" in btn ? (btn.props as Record<string, unknown>).onPress : undefined).toBe("onPressAuto");
  });
});
