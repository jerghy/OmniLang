import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import { SourceNodeCard } from "../SourceNodeCard";
import { DndContext } from "@dnd-kit/core";
import type { SourceNode } from "../source-nodes";

const node: SourceNode = {
  id: "sig:abc",
  kind: "signal",
  name: "count",
  type: "i32",
  value: 0,
  ownerKey: "Card",
  auto: false,
};

describe("SourceNodeCard", () => {
  it("渲染图标 + 名 + 当前值", () => {
    render(
      <DndContext>
        <SourceNodeCard node={node} onEdit={() => {}} />
      </DndContext>
    );
    expect(screen.getByText("count")).toBeTruthy();
    expect(screen.getByText("0")).toBeTruthy();
    expect(screen.getByTestId("source-node-card")).toBeTruthy();
  });

  it("五类形象：小灯/算盘/账本 + store-getter 算页签 + inline 本地角标（data-kind 断言）", () => {
    const kinds: SourceNode["kind"][] = ["signal", "computed", "store-signal", "store-getter", "inline-signal"];
    for (const kind of kinds) {
      const { unmount, getByTestId, queryByText } = render(
        <DndContext>
          <SourceNodeCard node={{ ...node, id: `k:${kind}`, kind }} onEdit={() => {}} />
        </DndContext>
      );
      // 图标断言用 data-kind 属性而非 SVG 细节——视觉断言不可脆
      expect(document.querySelector(`[data-kind="${kind}"]`)).toBeTruthy();
      if (kind === "store-getter") {
        expect(getByTestId("kind-badge").textContent).toBe("算");
      } else {
        expect(document.querySelector('[data-testid="kind-badge"]')).toBeNull();
      }
      if (kind === "inline-signal") {
        expect(queryByText("本地")).toBeTruthy();
      } else {
        expect(queryByText("本地")).toBeNull();
      }
      unmount();
    }
  });

  it("点击卡片 → onEdit 回调（含节点）", () => {
    const onEdit = vi.fn();
    render(
      <DndContext>
        <SourceNodeCard node={node} onEdit={onEdit} />
      </DndContext>
    );
    fireEvent.click(screen.getByTestId("source-node-card"));
    expect(onEdit).toHaveBeenCalledWith(node);
  });
});
