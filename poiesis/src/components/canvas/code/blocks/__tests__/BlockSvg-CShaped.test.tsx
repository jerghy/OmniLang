// poiesis/src/components/canvas/code/blocks/__tests__/BlockSvg-CShaped.test.tsx
import { describe, it, expect } from "vitest";
import { render } from "@testing-library/react";
import { IfBlock, WhileBlock, ForInBlock } from "../BlockSvg/index";
import type { IfBlockOp, WhileLoopOp, ForLoopOp, CodeOp } from "../../../../../core/types/code-op";

// ── Helpers ──

function renderBody(ops: CodeOp[]): React.ReactNode {
  return ops.map((op) => <g key={op.pid} data-testid={`body-block-${op.pid}`} />);
}

// ── IfBlock ──

describe("IfBlock", () => {
  const baseIf: IfBlockOp = {
    pid: "if-1",
    kind: "if_block",
    bodyId: "body-1",
    branches: [
      {
        condition: { kind: "text", value: "x > 0" },
        body: [{ pid: "body-a", kind: "set_var", name: { kind: "text", value: "y" }, operator: "=", value: { kind: "text", value: "1" } } as CodeOp as CodeOp],
      },
    ],
  };

  it("renders g with data-block-id", () => {
    const { container } = render(
      <svg>
        <IfBlock op={baseIf} renderBody={renderBody} />
      </svg>,
    );
    const g = container.querySelector("[data-block-id='if-1']");
    expect(g).toBeTruthy();
  });

  it("renders 'if' keyword for the first branch", () => {
    const { container } = render(
      <svg>
        <IfBlock op={baseIf} renderBody={renderBody} />
      </svg>,
    );
    const text = container.textContent || "";
    expect(text).toContain("if");
  });

  it("renders condition text from slot", () => {
    const { container } = render(
      <svg>
        <IfBlock op={baseIf} renderBody={renderBody} />
      </svg>,
    );
    expect(container.textContent).toContain("x > 0");
  });

  it("renders 'elif' for middle non-empty branches", () => {
    const op: IfBlockOp = {
      ...baseIf,
      branches: [
        { condition: { kind: "text", value: "a" }, body: [] },
        { condition: { kind: "text", value: "b" }, body: [] },
        { condition: { kind: "empty" }, body: [] },
      ],
    };
    const { container } = render(
      <svg>
        <IfBlock op={op} renderBody={renderBody} />
      </svg>,
    );
    expect(container.textContent).toContain("elif");
  });

  it("renders 'else' for empty-condition branch after first", () => {
    const op: IfBlockOp = {
      ...baseIf,
      branches: [
        { condition: { kind: "text", value: "a" }, body: [] },
        { condition: { kind: "empty" }, body: [] },
      ],
    };
    const { container } = render(
      <svg>
        <IfBlock op={op} renderBody={renderBody} />
      </svg>,
    );
    expect(container.textContent).toContain("else");
  });

  it("does not render condition slot for else branch", () => {
    const op: IfBlockOp = {
      ...baseIf,
      branches: [
        { condition: { kind: "text", value: "a" }, body: [] },
        { condition: { kind: "empty" }, body: [] },
      ],
    };
    const { container } = render(
      <svg>
        <IfBlock op={op} renderBody={renderBody} />
      </svg>,
    );
    expect(container.textContent).toContain("if");
    expect(container.textContent).toContain("else");
  });

  it("renders body blocks via renderBody callback", () => {
    const { container } = render(
      <svg>
        <IfBlock op={baseIf} renderBody={renderBody} />
      </svg>,
    );
    expect(container.querySelector("[data-testid='body-block-body-a']")).toBeTruthy();
  });

  it("has displayName", () => {
    expect(IfBlock.displayName).toBe("IfBlock");
  });

  it("positions bottom connector after body blocks across all branches", () => {
    // Branch 0: 0 body blocks, Branch 1: 2 body blocks
    const op: IfBlockOp = {
      ...baseIf,
      branches: [
        { condition: { kind: "text", value: "a" }, body: [] },
        {
          condition: { kind: "text", value: "b" },
          body: [
            { pid: "b0", kind: "set_var", name: { kind: "text", value: "x" }, operator: "=", value: { kind: "text", value: "1" } } as CodeOp,
            { pid: "b1", kind: "set_var", name: { kind: "text", value: "y" }, operator: "=", value: { kind: "text", value: "2" } } as CodeOp,
          ],
        },
      ],
    };
    // Branch 0: 36 + 0*48 = 36, Branch 1: 36 + 2*48 = 132, total = 168, bottomY = 168 + 8 = 176
    const { container } = render(
      <svg>
        <IfBlock op={op} renderBody={renderBody} />
      </svg>,
    );
    const connectorRect = container.querySelector("rect[height='8']");
    expect(connectorRect?.getAttribute("y")).toBe("176");
  });
});

// ── WhileBlock ──

describe("WhileBlock", () => {
  const baseWhile: WhileLoopOp = {
    pid: "while-1",
    kind: "while",
    bodyId: "body-w1",
    condition: { kind: "text", value: "running" },
    body: [
      { pid: "wb-a", kind: "set_var", name: { kind: "text", value: "x" }, operator: "+=", value: { kind: "text", value: "1" } } as CodeOp,
    ],
  };

  it("renders g with data-block-id", () => {
    const { container } = render(
      <svg>
        <WhileBlock op={baseWhile} renderBody={renderBody} />
      </svg>,
    );
    const g = container.querySelector("[data-block-id='while-1']");
    expect(g).toBeTruthy();
  });

  it("renders 'while' keyword", () => {
    const { container } = render(
      <svg>
        <WhileBlock op={baseWhile} renderBody={renderBody} />
      </svg>,
    );
    expect(container.textContent).toContain("while");
  });

  it("renders condition text", () => {
    const { container } = render(
      <svg>
        <WhileBlock op={baseWhile} renderBody={renderBody} />
      </svg>,
    );
    expect(container.textContent).toContain("running");
  });

  it("renders body blocks via renderBody", () => {
    const { container } = render(
      <svg>
        <WhileBlock op={baseWhile} renderBody={renderBody} />
      </svg>,
    );
    expect(container.querySelector("[data-testid='body-block-wb-a']")).toBeTruthy();
  });

  it("has displayName", () => {
    expect(WhileBlock.displayName).toBe("WhileBlock");
  });

  it("positions bottom connector dynamically based on body block count", () => {
    const op: WhileLoopOp = {
      ...baseWhile,
      body: [
        { pid: "wb-x", kind: "set_var", name: { kind: "text", value: "a" }, operator: "=", value: { kind: "text", value: "1" } } as CodeOp,
        { pid: "wb-y", kind: "set_var", name: { kind: "text", value: "b" }, operator: "=", value: { kind: "text", value: "2" } } as CodeOp,
        { pid: "wb-z", kind: "set_var", name: { kind: "text", value: "c" }, operator: "=", value: { kind: "text", value: "3" } } as CodeOp,
      ],
    };
    // bottomY = 36 + 3*48 + 8 = 188
    const { container } = render(
      <svg>
        <WhileBlock op={op} renderBody={renderBody} />
      </svg>,
    );
    const connectorRect = container.querySelector("rect[height='8']");
    expect(connectorRect?.getAttribute("y")).toBe("188");
  });
});

// ── ForInBlock ──

describe("ForInBlock", () => {
  const baseFor: ForLoopOp = {
    pid: "for-1",
    kind: "for_in",
    bodyId: "body-f1",
    itemName: { kind: "text", value: "item" },
    collection: { kind: "text", value: "items" },
    body: [
      { pid: "fb-a", kind: "set_var", name: { kind: "text", value: "acc" }, operator: "+=", value: { kind: "text", value: "item" } } as CodeOp,
    ],
  };

  it("renders g with data-block-id", () => {
    const { container } = render(
      <svg>
        <ForInBlock op={baseFor} renderBody={renderBody} />
      </svg>,
    );
    const g = container.querySelector("[data-block-id='for-1']");
    expect(g).toBeTruthy();
  });

  it("renders 'for' and 'in' keywords", () => {
    const { container } = render(
      <svg>
        <ForInBlock op={baseFor} renderBody={renderBody} />
      </svg>,
    );
    const text = container.textContent || "";
    expect(text).toContain("for");
    expect(text).toContain("in");
  });

  it("renders item name and collection text", () => {
    const { container } = render(
      <svg>
        <ForInBlock op={baseFor} renderBody={renderBody} />
      </svg>,
    );
    const text = container.textContent || "";
    expect(text).toContain("item");
    expect(text).toContain("items");
  });

  it("renders body blocks via renderBody", () => {
    const { container } = render(
      <svg>
        <ForInBlock op={baseFor} renderBody={renderBody} />
      </svg>,
    );
    expect(container.querySelector("[data-testid='body-block-fb-a']")).toBeTruthy();
  });

  it("has displayName", () => {
    expect(ForInBlock.displayName).toBe("ForInBlock");
  });

  it("positions bottom connector dynamically when body is empty", () => {
    const op: ForLoopOp = {
      ...baseFor,
      body: [],
    };
    // bottomY = 36 + 0*48 + 8 = 44
    const { container } = render(
      <svg>
        <ForInBlock op={op} renderBody={renderBody} />
      </svg>,
    );
    const connectorRect = container.querySelector("rect[height='8']");
    expect(connectorRect?.getAttribute("y")).toBe("44");
  });
});
