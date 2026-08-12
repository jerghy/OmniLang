// poiesis/src/components/canvas/__tests__/PortHandleLayer.test.tsx
import { render, screen } from "@testing-library/react";
import { describe, it, expect, afterEach } from "vitest";
import { PortHandleLayer } from "../PortHandleLayer";
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
