// poiesis/src/components/design/__tests__/OmniViewElementSelect.test.tsx
import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import { OmniViewElement } from "../OmniViewElement";

const node = { nid: "n1", tag: "button", text: "按钮", children: [] } as any;

describe("OmniViewElement 选择（醒区输入）", () => {
  it("Shift+点击 → onSelect 带 multi=true", () => {
    const onSelect = vi.fn();
    render(<OmniViewElement node={node} tag="button" isSelected={false} onSelect={onSelect} onDoubleClick={() => {}} />);
    fireEvent.click(screen.getByText("按钮"), { shiftKey: true });
    expect(onSelect).toHaveBeenCalledWith("n1", true);
  });

  it("普通点击 → multi=false（单选替换醒区）", () => {
    const onSelect = vi.fn();
    render(<OmniViewElement node={node} tag="button" isSelected={false} onSelect={onSelect} onDoubleClick={() => {}} />);
    fireEvent.click(screen.getByText("按钮"));
    expect(onSelect).toHaveBeenCalledWith("n1", false);
  });
});
