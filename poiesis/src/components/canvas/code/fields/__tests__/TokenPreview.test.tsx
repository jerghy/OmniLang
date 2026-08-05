// poiesis/src/components/canvas/code/fields/__tests__/TokenPreview.test.tsx
// TokenPreview 只读 token 预览：表达式结构胶囊渲染 + 纯文本退化。
import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { TokenPreview } from "../TokenPreview";

describe("TokenPreview", () => {
  it("表达式 token 化渲染（胶囊与符号区分）", () => {
    const { container } = render(<TokenPreview value="count + 1" />);
    expect(container.querySelectorAll('[data-token-kind="identifier"]')).toHaveLength(1);
    expect(container.querySelector('[data-token-kind="identifier"]')?.textContent).toBe("count");
    expect(container.querySelectorAll('[data-token-kind="operator"]')).toHaveLength(1);
    expect(container.querySelectorAll('[data-token-kind="number"]')).toHaveLength(1);
  });
  it("member 整体胶囊", () => {
    const { container } = render(<TokenPreview value="TodoStore.total" />);
    expect(container.querySelector('[data-token-kind="member"]')?.textContent).toBe("TodoStore.total");
  });
  it("空值/纯文本退化不炸", () => {
    render(<TokenPreview value="" />);
    expect(screen.getByTestId("token-preview").textContent).toBe("");
  });
  it("string token 渲染原文含引号（Task 7 契约：text 不含引号，统一 slice(start,end) 取原文）", () => {
    const { container } = render(<TokenPreview value={'a == "b c"'} />);
    const str = container.querySelector('[data-token-kind="string"]');
    expect(str?.textContent).toBe('"b c"');
  });
});
