import { render, screen, fireEvent, act } from "@testing-library/react";
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

  it("点卡片 → 改值浮层出现（ValueEditor）", () => {
    usePoiesisStore.setState({ designPage: { ...usePoiesisStore.getState().designPage, awake: true, dataNodesVisible: true } });
    render(<SourceNodeLayer />);
    fireEvent.click(screen.getByTestId("source-node-card"));
    expect(screen.getByTestId("value-editor")).toBeTruthy();
    expect(screen.getByTestId("value-editor-input")).toBeTruthy();
  });

  it("门控关闭 → 编辑态清理（浮层不残留）", () => {
    usePoiesisStore.setState({ designPage: { ...usePoiesisStore.getState().designPage, awake: true, dataNodesVisible: true } });
    render(<SourceNodeLayer />);
    fireEvent.click(screen.getByTestId("source-node-card"));
    expect(screen.getByTestId("value-editor")).toBeTruthy();
    // 睡 → 门控关闭（effect 清理编辑态）→ 再醒 → 浮层不残留（act 冲刷中间态，避免批量合并）
    act(() => {
      usePoiesisStore.setState({ designPage: { ...usePoiesisStore.getState().designPage, awake: false } });
    });
    expect(screen.queryByTestId("source-node-layer")).toBeNull();
    act(() => {
      usePoiesisStore.setState({ designPage: { ...usePoiesisStore.getState().designPage, awake: true } });
    });
    expect(screen.queryByTestId("value-editor")).toBeNull();
  });

  it("computed 卡点击不打开编辑器（可覆盖 kind 门控）", () => {
    usePoiesisStore.setState({
      designPage: { ...usePoiesisStore.getState().designPage, awake: true, dataNodesVisible: true },
      componentDefs: {
        Card: {
          name: "Card",
          filePath: "src/components/Card.omni",
          signals: [],
          computeds: [{ pid: "comp-1", name: "fullName", returnType: "string" }],
          events: [], fns: [], lifecycles: [], props: [],
        },
      } as any,
    });
    render(<SourceNodeLayer />);
    expect(screen.getByText("fullName")).toBeTruthy();
    fireEvent.click(screen.getByTestId("source-node-card"));
    expect(screen.queryByTestId("value-editor")).toBeNull();
  });

  it("非组件 tab 不渲染（componentName 门控）", () => {
    usePoiesisStore.setState({
      designPage: {
        ...usePoiesisStore.getState().designPage,
        awake: true,
        dataNodesVisible: true,
        openTabs: [{ id: "t1", filePath: "src/stores/T.omni", componentName: "T", isDirty: false, pidMap: { status: "ok", entries: {}, lineToPid: {} }, tabType: "store" }],
      },
    });
    render(<SourceNodeLayer />);
    expect(screen.queryByTestId("source-node-layer")).toBeNull();
  });
});
