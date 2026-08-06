import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { NodeFormulaField, type FormulaCandidateSource } from "../NodeFormulaField";

/** 模拟 buildFormulaSource 的 filter 语义（空 query 全量、其余 startsWith 匹配）——
 *  逃生口测试依赖"无匹配 → 空候选"。 */
const source: FormulaCandidateSource = {
  filter: (q) => (q === "" || q.startsWith("count") ? [{ value: "count", label: "count" }] : []),
  storeSignals: () => [],
};

const el = (w: HTMLElement) => w.querySelector("textarea")!;

describe("NodeFormulaField", () => {
  it("渲染：胶囊块 + 结构字符着色 + 透明文本层", () => {
    const { container } = render(<NodeFormulaField value="count + 1" onCommit={() => {}} candidates={source} />);
    expect(container.querySelectorAll("[data-seg-kind='capsule']")).toHaveLength(2);
    expect(container.querySelectorAll("[data-seg-kind='struct']")).toHaveLength(0);
    expect(el(container).value).toBe("\uE000count\uE001 \uE000+\uE001 1");
  });

  it("斜杠边界键入 → 弹全量候选；Enter 选中 → 胶囊入框", async () => {
    const onCommit = vi.fn();
    const { container } = render(<NodeFormulaField value="" onCommit={onCommit} candidates={source} />);
    const ta = el(container);
    ta.value = "/";
    ta.setSelectionRange(1, 1); // 必须在 fireEvent 前设（onInput 同步读 selectionStart）
    fireEvent.input(ta, { target: { value: "/" } });
    expect(await screen.findByRole("listbox")).toBeTruthy();
    fireEvent.keyDown(ta, { key: "Enter" });
    expect(ta.value).toBe("\uE000count\uE001");
    fireEvent.keyDown(ta, { key: "Enter" });
    expect(onCommit).toHaveBeenCalledWith("count");
  });

  it("运算符边界键入 → 弹运算符候选；Enter 选中", async () => {
    const { container } = render(<NodeFormulaField value="count" onCommit={() => {}} candidates={source} />);
    const ta = el(container);
    ta.value = "count +";
    ta.setSelectionRange(7, 7);
    fireEvent.input(ta, { target: { value: "count +" } });
    const box = await screen.findByRole("listbox");
    expect(box).toBeTruthy();
    fireEvent.keyDown(ta, { key: "Enter" });
    expect(ta.value).toBe("count \uE000+\uE001");
  });

  it("词中间运算符不弹框（字符串内容）", async () => {
    const { container } = render(<NodeFormulaField value="" onCommit={() => {}} candidates={source} />);
    const ta = el(container);
    ta.value = "user-s";
    ta.setSelectionRange(6, 6);
    fireEvent.input(ta, { target: { value: "user-s" } });
    expect(screen.queryByRole("listbox")).toBeNull();
  });

  it("斜杠逃生口：无匹配 Enter → 引用胶囊", async () => {
    const { container } = render(<NodeFormulaField value="" onCommit={() => {}} candidates={source} />);
    const ta = el(container);
    ta.value = "/";
    ta.setSelectionRange(1, 1);
    fireEvent.input(ta, { target: { value: "/" } });
    expect(await screen.findByRole("listbox")).toBeTruthy();
    ta.value = "/s";
    ta.setSelectionRange(2, 2);
    fireEvent.input(ta, { target: { value: "/s" } }); // 菜单重过滤：候选空 → 仅运算符组
    fireEvent.keyDown(ta, { key: "Enter" });
    expect(ta.value).toBe("\uE000s\uE001");
  });

  it("点击胶囊 → 重新弹候选；Enter 替换", async () => {
    const { container } = render(<NodeFormulaField value="count + 1" onCommit={() => {}} candidates={source} />);
    const cap = container.querySelector("[data-seg-kind='capsule']") as HTMLElement;
    fireEvent.mouseDown(cap);
    expect(await screen.findByRole("listbox")).toBeTruthy();
    const ta = el(container);
    fireEvent.keyDown(ta, { key: "Enter" });
    expect(ta.value).toBe("\uE000count\uE001 \uE000+\uE001 1");
  });

  it("Backspace 在胶囊标记起点 → 整块删除", () => {
    const { container } = render(<NodeFormulaField value="count + 1" onCommit={() => {}} candidates={source} />);
    const ta = el(container);
    ta.setSelectionRange(1, 1); // 光标在胶囊内
    fireEvent.keyDown(ta, { key: "Backspace" });
    expect(ta.value).toBe(" \uE000+\uE001 1");
  });

  it("Enter 提交（无候选框）；Esc 取消恢复 value", () => {
    const onCommit = vi.fn();
    const { container } = render(<NodeFormulaField value="count" onCommit={onCommit} candidates={source} />);
    const ta = el(container);
    ta.value = "\uE000count\uE001 \uE000+\uE001 1";
    ta.setSelectionRange(11, 11);
    fireEvent.input(ta, { target: { value: "\uE000count\uE001 \uE000+\uE001 1" } });
    fireEvent.keyDown(ta, { key: "Enter" });
    expect(onCommit).toHaveBeenCalledWith("count + 1");
    // Esc 取消 → 恢复外部 value 的反向结构化形态
    ta.value = "\uE000count\uE001 \uE000+\uE001 2";
    ta.setSelectionRange(11, 11);
    fireEvent.input(ta, { target: { value: "count + 2" } });
    fireEvent.keyDown(ta, { key: "Escape" });
    expect(ta.value).toBe("\uE000count\uE001");
  });
});
