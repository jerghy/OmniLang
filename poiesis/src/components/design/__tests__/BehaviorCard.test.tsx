// poiesis/src/components/design/__tests__/BehaviorCard.test.tsx
// M3 T1 行为卡：当[事件]→[动作][参数] 表单——默认渲染 / 选动作出参数 / 提交回传。
import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import { BehaviorCard } from "../BehaviorCard";

describe("BehaviorCard", () => {
  it("默认渲染 当 + 事件/动作 下拉", () => {
    render(<BehaviorCard elementId="el-1" onCommit={vi.fn()} />);
    expect(screen.getByText("当")).toBeTruthy();
    expect(screen.getByTestId("behavior-event")).toBeTruthy();
    expect(screen.getByTestId("behavior-action")).toBeTruthy();
  });

  it("选动作「加」→ 出现目标信号参数槽", () => {
    render(<BehaviorCard elementId="el-1" onCommit={vi.fn()} />);
    fireEvent.change(screen.getByTestId("behavior-action"), { target: { value: "add" } });
    expect(screen.getByTestId("behavior-target")).toBeTruthy();
  });

  it("提交 → onCommit 收到 { event, action }", () => {
    const onCommit = vi.fn();
    render(<BehaviorCard elementId="el-1" onCommit={onCommit} />);
    fireEvent.change(screen.getByTestId("behavior-event"), { target: { value: "click" } });
    fireEvent.change(screen.getByTestId("behavior-action"), { target: { value: "navigate" } });
    fireEvent.click(screen.getByTestId("behavior-commit"));
    expect(onCommit).toHaveBeenCalledWith(
      expect.objectContaining({ event: "click", action: "navigate" })
    );
  });
});
