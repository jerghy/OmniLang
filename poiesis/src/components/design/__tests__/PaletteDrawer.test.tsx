// components/design/__tests__/PaletteDrawer.test.tsx
// [组件库] 弹出面板（M2 T6）：薄壳承载 ComponentPalette（拖拽能力原样复用），
// 底栏 [组件库] 按钮开关（DesignPage 受控 paletteOpen），关闭按钮回调 onClose。
import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import { PaletteDrawer } from "../PaletteDrawer";

describe("PaletteDrawer", () => {
  it("渲染标题 + 关闭按钮 + ComponentPalette 内容", () => {
    render(<PaletteDrawer onClose={() => {}} />);
    expect(screen.getByText("组件库")).toBeTruthy();
    expect(screen.getByTestId("palette-drawer-close")).toBeTruthy();
    // ComponentPalette 内容（组件分类标题）
    expect(screen.getByText("🧩 组件面板")).toBeTruthy();
  });

  it("点关闭 → onClose 回调", () => {
    const onClose = vi.fn();
    render(<PaletteDrawer onClose={onClose} />);
    fireEvent.click(screen.getByTestId("palette-drawer-close"));
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
