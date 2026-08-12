// poiesis/src/components/canvas/__tests__/PortHandleLayer.test.tsx
import { render, screen } from "@testing-library/react";
import { describe, it, expect, afterEach } from "vitest";
import { PortHandleLayer, resolveSlots } from "../PortHandleLayer";
import { usePoiesisStore } from "../../../store";

const setAwake = (awake: boolean) =>
  usePoiesisStore.setState((s) => ({ designPage: { ...s.designPage, awake } }));

describe("PortHandleLayer", () => {
  afterEach(() => setAwake(false));

  it("醒态可见：渲染手柄层（ReactFlow 可拖手柄挂载）", () => {
    setAwake(true);
    render(<PortHandleLayer visible />);
    expect(screen.getByTestId("port-handle-layer")).toBeTruthy();
  });

  it("睡态隐藏：返回 null（睡态干净画布，无手柄）", () => {
    setAwake(false);
    const { container } = render(<PortHandleLayer visible />);
    expect(container.querySelector('[data-testid="port-handle-layer"]')).toBeNull();
  });

  it("visible=false 同样返回 null（编译错误态）", () => {
    setAwake(true);
    const { container } = render(<PortHandleLayer visible={false} />);
    expect(container.querySelector('[data-testid="port-handle-layer"]')).toBeNull();
  });
});

describe("resolveSlots（C1 回归：槽解析以测量节点为准）", () => {
  it("节点命中优先：targetHandle 携带对侧（-src）后缀也返回节点槽（不剥成垃圾槽）", () => {
    const nodes = [
      { id: "A", data: { slotKey: "label" } },
      { id: "B", data: { slotKey: "href" } },
    ];
    // C1 场景：端口中心 elementFromPoint 恒命中 source handle → targetHandle 错为 "label-src"
    expect(resolveSlots({ source: "A", target: "B", sourceHandle: "label-src", targetHandle: "label-src" }, nodes)).toEqual({
      sourceSlot: "label",
      targetSlot: "href",
    });
  });

  it("节点未命中：handle-id 后缀剥除兜底（-src/-tgt 双后缀均剥）", () => {
    expect(resolveSlots({ source: "A", target: "B", sourceHandle: "value-src", targetHandle: "img-tgt" }, [])).toEqual({
      sourceSlot: "value",
      targetSlot: "img",
    });
  });

  it("无 handle 且节点未命中 → null（调用方早退，不落垃圾槽）", () => {
    expect(resolveSlots({ source: "A", target: "B", sourceHandle: null, targetHandle: null }, [])).toEqual({
      sourceSlot: null,
      targetSlot: null,
    });
  });
});
