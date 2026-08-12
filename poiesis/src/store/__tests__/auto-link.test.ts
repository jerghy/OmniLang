// poiesis/src/store/__tests__/auto-link.test.ts
// M3 T4 自动中转 signal 生命周期：元素间连线 = 自动 signal。
// 核心契约：创建（源端 bind + 目标端绑定）/ 同槽复用 / 回收（无引用即删）/ 识别。
// 落盘 = 普通组件 signal（componentDefs[compName].signals，零语法变更）。
// 命名 = `<源元素id>_<槽语义>`；id 消毒（dash → underscore）后作前缀。
// 测试断言按摸底调整：signal 存在 componentDefs 而非 elements。

import { describe, it, expect, beforeEach } from "vitest";
import { usePoiesisStore } from "../index";
import { createInitialDesignPageState } from "../operations/tab-lifecycle";
import { autoSignalName, isAutoSignal } from "../slices/auto-link";
import type { ElementData } from "../../core/types/element";

function setupStore(): void {
  usePoiesisStore.setState({
    elements: [
      { id: "input_1", nodeType: "view", tag: "input", props: {}, children: [], parentId: "root" },
      { id: "title_1", nodeType: "view", tag: "h1", props: { label: "标题" }, children: [], parentId: "root" },
    ],
    wires: [],
    selectedElementIds: [],
    rootOrder: ["input_1", "title_1"],
    isDirty: false,
    undoStacks: {},
    componentDefs: {
      App: {
        name: "App",
        filePath: "src/components/App.omni",
        signals: [],
        props: [],
        computeds: [],
        events: [],
        fns: [],
        lifecycles: [],
      },
    },
    designPage: {
      ...createInitialDesignPageState(),
      activeTabId: "tab-1",
      openTabs: [{ id: "tab-1", filePath: "App.omni", tabType: "component", componentName: "App", isDirty: false, pidMap: { status: "ok", entries: {}, lineToPid: {} } }],
    },
  });
}

function getAppSignals() {
  const def = usePoiesisStore.getState().componentDefs["App"];
  return (def?.signals ?? []) as Array<{ pid: string; name: string; type: string; defaultValue?: unknown }>;
}

describe("auto-link: 创建", () => {
  beforeEach(setupStore);

  it("connectElementLink 创建自动 signal：componentDefs.signals + 源端 bind + 目标端绑定 + wire", () => {
    const store = usePoiesisStore.getState();
    store.connectElementLink("input_1", "value", "title_1", "label");

    const signals = getAppSignals();
    expect(signals).toHaveLength(1);
    expect(signals[0].name).toBe("input_1_value");
    // String 大写 + 引号字面量默认值（emitter 空串跳过 + 仅大写 String 加引号，M3 T8 实机二次发现）
    expect(signals[0].type).toBe("String");
    expect(signals[0].defaultValue).toBe('""');

    // 源端：bind:value={input_1_value}
    const input = usePoiesisStore.getState().elements.find((e) => e.id === "input_1");
    expect(input && "bindings" in input && input.bindings?.["value"]).toBe("input_1_value");

    // 目标端：文本槽 → props.label 插值
    const title = usePoiesisStore.getState().elements.find((e) => e.id === "title_1");
    expect(title && "props" in title && title.props?.["label"]).toBe("{input_1_value}");

    // wire 落盘
    const wires = usePoiesisStore.getState().wires;
    expect(wires).toHaveLength(1);
    expect(wires[0]).toMatchObject({ fromId: "input_1", fromPort: "value", toId: "title_1", toPort: "label" });
  });
});

describe("auto-link: 同槽复用", () => {
  beforeEach(setupStore);

  it("同一 (源,槽) 第二条连线复用同一 signal", () => {
    const store = usePoiesisStore.getState();
    store.connectElementLink("input_1", "value", "title_1", "label");

    // 再往第二个元素连（同源同槽）
    usePoiesisStore.setState({
      elements: [
        ...usePoiesisStore.getState().elements,
        { id: "subtitle_1", nodeType: "view", tag: "h2", props: { label: "副标题" }, children: [], parentId: "root" },
      ],
      rootOrder: ["input_1", "title_1", "subtitle_1"],
    });
    usePoiesisStore.getState().connectElementLink("input_1", "value", "subtitle_1", "label");

    expect(getAppSignals()).toHaveLength(1);
    expect(getAppSignals()[0].name).toBe("input_1_value");

    // 两条 wire 共享同一 signal 名
    const wires = usePoiesisStore.getState().wires;
    expect(wires).toHaveLength(2);
    const subtitle = usePoiesisStore.getState().elements.find((e) => e.id === "subtitle_1");
    expect(subtitle && "props" in subtitle && subtitle.props?.["label"]).toBe("{input_1_value}");
  });
});

describe("auto-link: 回收", () => {
  beforeEach(setupStore);

  it("disconnect 后无其他引用 → 删除 signal", () => {
    const store = usePoiesisStore.getState();
    store.connectElementLink("input_1", "value", "title_1", "label");
    const wireId = usePoiesisStore.getState().wires[0].id;

    store.disconnectElementLink(wireId);

    // wire 删除
    expect(usePoiesisStore.getState().wires).toHaveLength(0);
    // signal 回收
    expect(getAppSignals()).toHaveLength(0);
    // 源端 bind 解除
    const input = usePoiesisStore.getState().elements.find((e) => e.id === "input_1");
    expect(input && "bindings" in input && input.bindings?.["value"]).toBeUndefined();
    // 目标端绑定值复位（clearTargetBinding 对文本槽写 ""）
    const title = usePoiesisStore.getState().elements.find((e) => e.id === "title_1");
    expect(title && "props" in title && title.props?.["label"]).toBe("");
  });

  it("disconnect 后仍有其他引用 → 保留 signal", () => {
    const store = usePoiesisStore.getState();
    store.connectElementLink("input_1", "value", "title_1", "label");
    usePoiesisStore.setState({
      elements: [
        ...usePoiesisStore.getState().elements,
        { id: "subtitle_1", nodeType: "view", tag: "h2", props: { label: "副标题" }, children: [], parentId: "root" },
      ],
      rootOrder: ["input_1", "title_1", "subtitle_1"],
    });
    store.connectElementLink("input_1", "value", "subtitle_1", "label");
    const wireA = usePoiesisStore.getState().wires[0].id;

    store.disconnectElementLink(wireA);

    expect(usePoiesisStore.getState().wires).toHaveLength(1);
    expect(getAppSignals()).toHaveLength(1);
    expect(getAppSignals()[0].name).toBe("input_1_value");
  });
});

describe("auto-link: 识别与命名", () => {
  beforeEach(setupStore);

  it("isAutoSignal 识别 `<源元素id>_<槽语义>` 命名（含 dash id 消毒）", () => {
    const elements: ElementData[] = [
      { id: "input_1", nodeType: "view", tag: "input", props: {}, children: [], parentId: null },
      { id: "input-m0abc-xyz-1", nodeType: "view", tag: "input", props: {}, children: [], parentId: null },
    ];

    expect(isAutoSignal("input_1_value", elements)).toBe(true);
    expect(isAutoSignal("input_1_label", elements)).toBe(true);
    // dash id 消毒后前缀匹配
    expect(isAutoSignal("input_m0abc_xyz_1_value", elements)).toBe(true);
    // 手工信号（非自动前缀）不识别
    expect(isAutoSignal("userName_value", elements)).toBe(false);
    expect(isAutoSignal("count", elements)).toBe(false);
  });

  it("autoSignalName 命名与消毒：dash → underscore；slot 语义后缀", () => {
    expect(autoSignalName("input_1", "value")).toBe("input_1_value");
    expect(autoSignalName("input-m0abc-xyz-1", "value")).toBe("input_m0abc_xyz_1_value");
  });
});
