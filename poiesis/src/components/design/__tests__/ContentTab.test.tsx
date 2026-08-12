// poiesis/src/components/design/__tests__/ContentTab.test.tsx
// [内容] 面板槽位 badge 回归（M3 T3 复审 + T6 来源印记）：文本槽键对齐落盘键 label——
// props.label 含插值 {count} → 已绑定徽标亮起且显示来源名（来自 count）；纯文本 → 不亮；
// 输入类槽位 bindings 键命中 → 徽标显示信号名。
import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, beforeEach } from "vitest";
import { ContentTab } from "../ContentTab";
import { usePoiesisStore } from "../../../store";
import type { ViewElementData } from "../../../core/types/element";

const elBound: ViewElementData = {
  id: "el-1",
  nodeType: "view",
  tag: "button",
  props: { label: "{count}" },
  children: [],
  parentId: null,
};
const elPlain: ViewElementData = {
  id: "el-2",
  nodeType: "view",
  tag: "div",
  props: { label: "纯文本" },
  children: [],
  parentId: null,
};
const elInputBound: ViewElementData = {
  id: "el-3",
  nodeType: "view",
  tag: "input",
  props: {},
  bindings: { value: "input" },
  children: [],
  parentId: null,
};
// 假亮回归 fixtures（复审 Minor 1/2）：空字符串绑定 / 空插值 { } → 徽标不得亮起
const elInputEmpty: ViewElementData = {
  id: "el-4",
  nodeType: "view",
  tag: "input",
  props: {},
  bindings: { value: "" },
  children: [],
  parentId: null,
};
const elEmptyInterp: ViewElementData = {
  id: "el-5",
  nodeType: "view",
  tag: "button",
  props: { label: "前缀 { } 后缀" },
  children: [],
  parentId: null,
};

describe("ContentTab 槽位 badge", () => {
  beforeEach(() => {
    usePoiesisStore.setState({
      designPage: { ...usePoiesisStore.getState().designPage, awake: false, mainPanelTab: "structure" },
      selectedElementIds: [],
      elements: [elBound, elPlain, elInputBound, elInputEmpty, elEmptyInterp],
    });
  });

  it("文本槽 props.label 插值 {count} → 徽标亮起且显示来源名（来自 count）", () => {
    render(<ContentTab elementId="el-1" />);
    expect(screen.getByTestId("slot-bound-label").textContent).toContain("来自 count");
    expect(screen.getByTestId("slot-bind-label").textContent).toBe("替换");
  });

  it("输入槽 bindings.value 绑定 input 信号 → 徽标显示来源名（来自 input）", () => {
    render(<ContentTab elementId="el-3" />);
    expect(screen.getByTestId("slot-bound-value").textContent).toContain("来自 input");
  });

  it("文本槽纯文本 label → 徽标不亮", () => {
    render(<ContentTab elementId="el-2" />);
    expect(screen.queryByTestId("slot-bound-label")).toBeNull();
    expect(screen.getByTestId("slot-bind-label").textContent).toBe("绑定");
  });

  it("空字符串绑定（bindings.value = \"\"）→ 徽标不亮（假亮回归）", () => {
    render(<ContentTab elementId="el-4" />);
    expect(screen.queryByTestId("slot-bound-value")).toBeNull();
    expect(screen.getByTestId("slot-bind-value").textContent).toBe("绑定");
  });

  it("空插值 { } → 徽标不亮（空来源名假亮回归）", () => {
    render(<ContentTab elementId="el-5" />);
    expect(screen.queryByTestId("slot-bound-label")).toBeNull();
    expect(screen.getByTestId("slot-bind-label").textContent).toBe("绑定");
  });
});

// B1 点名统一路径（Task 4）：点选画布值型控件来源 → connectElementLink 统一落盘
// （auto signal + 目标插值 + wire），不再把元素 label 文本当表达式落盘
describe("ContentTab 点名选值型控件 → connectElementLink 统一路径", () => {
  const elSource: ViewElementData = {
    id: "input-1",
    nodeType: "view",
    tag: "input",
    props: { label: "姓名" },
    children: [],
    parentId: null,
  };
  const elTarget: ViewElementData = {
    id: "title-1",
    nodeType: "view",
    tag: "h1",
    props: { label: "标题" },
    children: [],
    parentId: null,
  };

  beforeEach(() => {
    usePoiesisStore.setState({
      designPage: {
        ...usePoiesisStore.getState().designPage,
        awake: false,
        mainPanelTab: "content",
        activeTabId: "tab-app",
        openTabs: [{ id: "tab-app", filePath: "App.omni", tabType: "component", componentName: "App", isDirty: false, pidMap: { status: "ok", entries: {}, lineToPid: {} } }],
      },
      selectedElementIds: [],
      elements: [elSource, elTarget],
      wires: [],
      componentDefs: {
        App: {
          name: "App",
          filePath: "src/App.omni",
          signals: [],
          props: [],
          computeds: [],
          events: [],
          fns: [],
          lifecycles: [],
        },
      },
      storeDefs: {},
      inlineStoreDefs: {},
      storeReferences: {},
      debugValues: {},
    });
  });

  it("点选画布 input 来源 → auto signal input-1_value 创建 + 目标插值 {input-1_value} + wire", () => {
    render(<ContentTab elementId="title-1" />);

    // 打开文本槽的点名清单 → 点画布元素分组的 input 来源
    fireEvent.click(screen.getByTestId("slot-bind-label"));
    const sourceItem = document.querySelector('[data-source-id="el:input-1"]');
    expect(sourceItem).not.toBeNull(); // 值型控件出现在画布元素分组
    fireEvent.click(sourceItem as HTMLElement);

    // 统一落盘路径：auto signal + 目标插值 + wire（connectElementLink 语义）
    // 命名消毒：元素 id 的 dash → underscore（autoSignalName 既有机制）
    const signals = usePoiesisStore.getState().componentDefs["App"]?.signals ?? [];
    expect(signals.map((s) => s.name)).toEqual(["input_1_value"]);
    const title = usePoiesisStore.getState().elements.find((e) => e.id === "title-1");
    expect(title && "props" in title && title.props?.["label"]).toBe("{input_1_value}");
    const wires = usePoiesisStore.getState().wires;
    expect(wires).toHaveLength(1);
    expect(wires[0]).toMatchObject({ fromId: "input-1", fromPort: "value", toId: "title-1", toPort: "label" });
    // 源端 bind 落盘
    const input = usePoiesisStore.getState().elements.find((e) => e.id === "input-1");
    expect(input && "bindings" in input && input.bindings?.["value"]).toBe("input_1_value");
  });

  it("显示元素（h1）不出现在画布元素来源分组", () => {
    render(<ContentTab elementId="title-1" />);
    fireEvent.click(screen.getByTestId("slot-bind-label"));
    // title-1 自身是 h1——B1 收窄后不作为来源列出（源 = 值型控件）
    expect(document.querySelector('[data-source-id="el:title-1"]')).toBeNull();
    // 非值型来源不存在 → 分组只有 input 一个条目
    expect(document.querySelectorAll("[data-source-id^='el:']")).toHaveLength(1);
  });
});
