import { describe, it, expect } from "vitest";
import { tokenize } from "../omni-tokenizer";
import { TokenStream } from "../token-stream";
import { Parser } from "../parser";
import type { ConditionalBlockData } from "../../../core/types/element";

function parse(source: string) {
  const tokens = tokenize(source);
  const stream = new TokenStream(tokens);
  const parser = new Parser(stream);
  return parser.parse();
}

describe("conditional rendering parser", () => {
  it("parses @if with @else into ConditionalBlockData", () => {
    const source = `
import omniview;
component Test {
  view {
    @if isActive {
      <div id="a">Active</div>
    } @else {
      <div id="b">Inactive</div>
    }
  }
}`;
    const result = parse(source);
    expect(result.errors).toHaveLength(0);
    const condBlock = result.designElements?.find(
      (e) => e.nodeType === "conditional"
    ) as ConditionalBlockData | undefined;
    expect(condBlock).toBeDefined();
    expect(condBlock!.branches).toHaveLength(2);
    expect(condBlock!.branches[0].type).toBe("if");
    expect(condBlock!.branches[0].condition).toBe("isActive");
    expect(condBlock!.branches[1].type).toBe("else");
    expect(condBlock!.branches[1].condition).toBe("");
    // Each branch should have 1 child
    expect(condBlock!.branches[0].children).toHaveLength(1);
    expect(condBlock!.branches[1].children).toHaveLength(1);
    // Flattened children
    expect(condBlock!.children).toHaveLength(2);
  });

  it("parses @if / @else if / @else chain", () => {
    const source = `
import omniview;
component Test {
  view {
    @if status == "loading" {
      <div id="a">Loading</div>
    } @else if status == "error" {
      <div id="b">Error</div>
    } @else {
      <div id="c">OK</div>
    }
  }
}`;
    const result = parse(source);
    expect(result.errors).toHaveLength(0);
    const condBlock = result.designElements?.find(
      (e) => e.nodeType === "conditional"
    ) as ConditionalBlockData | undefined;
    expect(condBlock).toBeDefined();
    expect(condBlock!.branches).toHaveLength(3);
    expect(condBlock!.branches[0].type).toBe("if");
    expect(condBlock!.branches[0].condition).toBe('status == "loading"');
    expect(condBlock!.branches[1].type).toBe("elif");
    expect(condBlock!.branches[1].condition).toBe('status == "error"');
    expect(condBlock!.branches[2].type).toBe("else");
  });
});
