import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, beforeEach } from "vitest";
import { SourceNodeLayer } from "../SourceNodeLayer";
import { usePoiesisStore } from "../../../store";

const compDefs = {
  Card: {
    name: "Card",
    filePath: "src/components/Card.omni",
    signals: [{ pid: "sig-1", name: "count", type: "i32", defaultValue: 0 }],
    computeds: [], events: [], fns: [], lifecycles: [], props: [],
  },
};

describe("SourceNodeLayer", () => {
  beforeEach(() => {
    usePoiesisStore.setState({
      designPage: {
        ...usePoiesisStore.getState().designPage,
        awake: false,
        dataNodesVisible: false,
        activeTabId: "t1",
        openTabs: [{ id: "t1", filePath: "src/components/Card.omni", componentName: "Card", isDirty: false, pidMap: { status: "ok", entries: {}, lineToPid: {} }, tabType: "component" }],
      },
      componentDefs: compDefs as any,
      storeDefs: {},
      inlineStoreDefs: {},
      storeReferences: {},
      debugValues: {},
    });
  });

  it("睡态不渲染（门控 awake）", () => {
    usePoiesisStore.setState({ designPage: { ...usePoiesisStore.getState().designPage, dataNodesVisible: true } });
    render(<SourceNodeLayer />);
    expect(screen.queryByTestId("source-node-layer")).toBeNull();
  });

  it("开关关不渲染（门控 dataNodesVisible）", () => {
    usePoiesisStore.setState({ designPage: { ...usePoiesisStore.getState().designPage, awake: true } });
    render(<SourceNodeLayer />);
    expect(screen.queryByTestId("source-node-layer")).toBeNull();
  });

  it("醒 + 开关开 → 渲染源节点卡片（数据来自 collectSourceNodes）", () => {
    usePoiesisStore.setState({ designPage: { ...usePoiesisStore.getState().designPage, awake: true, dataNodesVisible: true } });
    render(<SourceNodeLayer />);
    expect(screen.getByTestId("source-node-layer")).toBeTruthy();
    expect(screen.getByText("count")).toBeTruthy();
    expect(screen.getByText("0")).toBeTruthy(); // defaultValue
  });

  it("点卡片 → 编辑指示出现（T5 换 ValueEditor）", () => {
    usePoiesisStore.setState({ designPage: { ...usePoiesisStore.getState().designPage, awake: true, dataNodesVisible: true } });
    render(<SourceNodeLayer />);
    fireEvent.click(screen.getByTestId("source-node-card"));
    expect(screen.getByTestId("editing-indicator")).toBeTruthy();
  });
});
