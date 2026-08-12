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

  it("kind 图标映射：signal→⚡ store-signal→📦 computed→🧮 inline→🔌", () => {
    const kinds: Array<[SourceNode["kind"], string]> = [
      ["signal", "⚡"],
      ["store-signal", "📦"],
      ["computed", "🧮"],
      ["inline-signal", "🔌"],
      ["store-getter", "📦"],
    ];
    for (const [kind, icon] of kinds) {
      const { unmount } = render(
        <DndContext>
          <SourceNodeCard node={{ ...node, id: `k:${kind}`, kind }} onEdit={() => {}} />
        </DndContext>
      );
      expect(screen.getByText(icon)).toBeTruthy();
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
