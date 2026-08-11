// poiesis/src/components/canvas/__tests__/BreathingPoint.test.tsx
import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, beforeEach } from "vitest";
import { BreathingPoint } from "../BreathingPoint";
import { usePoiesisStore } from "../../../store";

describe("BreathingPoint", () => {
  beforeEach(() => {
    usePoiesisStore.setState({
      designPage: { ...usePoiesisStore.getState().designPage, awake: false },
    });
  });

  it("点击切换整页醒/睡", () => {
    render(<BreathingPoint />);
    fireEvent.click(screen.getByTestId("breathing-point"));
    expect(usePoiesisStore.getState().designPage.awake).toBe(true);
    fireEvent.click(screen.getByTestId("breathing-point"));
    expect(usePoiesisStore.getState().designPage.awake).toBe(false);
  });

  it("醒态带唤醒样式", () => {
    usePoiesisStore.setState({ designPage: { ...usePoiesisStore.getState().designPage, awake: true } });
    render(<BreathingPoint />);
    expect(screen.getByTestId("breathing-point").className).toContain("awake");
  });

  it("title 随状态变化", () => {
    render(<BreathingPoint />);
    expect(screen.getByTestId("breathing-point").getAttribute("title")).toContain("醒来");
  });
});
