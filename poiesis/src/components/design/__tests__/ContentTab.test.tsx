// poiesis/src/components/design/__tests__/ContentTab.test.tsx
// [内容] 面板槽位 badge 回归（M3 T3 复审 + T6 来源印记）：文本槽键对齐落盘键 label——
// props.label 含插值 {count} → 已绑定徽标亮起且显示来源名（来自 count）；纯文本 → 不亮；
// 输入类槽位 bindings 键命中 → 徽标显示信号名。
import { render, screen } from "@testing-library/react";
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
