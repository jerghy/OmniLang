// poiesis/src/components/design/__tests__/ContentTab.test.tsx
// [内容] 面板槽位 badge 回归（M3 T3 复审）：文本槽键对齐落盘键 label——
// props.label 含插值 {count} → 已绑定徽标亮起；纯文本 → 不亮。
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

describe("ContentTab 槽位 badge", () => {
  beforeEach(() => {
    usePoiesisStore.setState({
      designPage: { ...usePoiesisStore.getState().designPage, awake: false, mainPanelTab: "structure" },
      selectedElementIds: [],
      elements: [elBound, elPlain],
    });
  });

  it("文本槽 props.label 插值 {count} → 已绑定徽标亮起（键对齐 label）", () => {
    render(<ContentTab elementId="el-1" />);
    expect(screen.getByTestId("slot-bound-label")).toBeTruthy();
    expect(screen.getByTestId("slot-bind-label").textContent).toBe("替换");
  });

  it("文本槽纯文本 label → 徽标不亮", () => {
    render(<ContentTab elementId="el-2" />);
    expect(screen.queryByTestId("slot-bound-label")).toBeNull();
    expect(screen.getByTestId("slot-bind-label").textContent).toBe("绑定");
  });
});
