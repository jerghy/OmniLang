// DesignToolbar undo/redo 按钮交互测试。
// redo 按钮（task-6）：渲染 + 点击触发 onRedo；undo 按钮同风格回归。
import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { DesignToolbar } from "../DesignToolbar";

function renderToolbar(overrides: { onUndo?: () => void; onRedo?: () => void } = {}) {
  const onUndo = overrides.onUndo ?? vi.fn();
  const onRedo = overrides.onRedo ?? vi.fn();
  render(
    <DesignToolbar
      isDirty={false}
      saving={false}
      onUndo={onUndo}
      onRedo={onRedo}
      onSave={vi.fn()}
      onOpenProject={vi.fn()}
      showCode={false}
      onToggleCode={vi.fn()}
      saveError={null}
    />,
  );
  return { onUndo, onRedo };
}

describe("DesignToolbar undo/redo", () => {
  it("渲染 redo 按钮且点击触发 onRedo", () => {
    const { onRedo } = renderToolbar();
    fireEvent.click(screen.getByLabelText(/redo|重做/i));
    expect(onRedo).toHaveBeenCalledTimes(1);
  });

  it("undo 按钮点击触发 onUndo（回归）", () => {
    const { onUndo } = renderToolbar();
    fireEvent.click(screen.getByLabelText(/undo|撤销/i));
    expect(onUndo).toHaveBeenCalledTimes(1);
  });
});
