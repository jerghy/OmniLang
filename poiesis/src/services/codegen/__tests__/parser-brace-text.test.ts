import { describe, it, expect } from "vitest";
import { parseOmniSource } from "../omni-parser";

describe("parser text content with braces", () => {
  it("preserves {expr} in element content text", () => {
    const source = `import omniview;

component Foo {
    view {
        <div>
            <p>Hello {name}</p>
        </div>
    }
}`;
    const result = parseOmniSource(source);
    const pEl = result.designElements?.find(e => "tag" in e && e.tag === "p");
    expect(pEl).toBeDefined();
    expect("props" in pEl! && pEl.props.label).toBe("Hello {name}");
  });

  it("preserves plain text content", () => {
    const source = `import omniview;

component Foo {
    view {
        <div>
            <p>Hello World</p>
        </div>
    }
}`;
    const result = parseOmniSource(source);
    const pEl = result.designElements?.find(e => "tag" in e && e.tag === "p");
    expect("props" in pEl! && pEl.props.label).toBe("Hello World");
  });

  it("preserves {expr} only as content", () => {
    const source = `import omniview;

component Foo {
    view {
        <div>
            <p>{name}</p>
        </div>
    }
}`;
    const result = parseOmniSource(source);
    const pEl = result.designElements?.find(e => "tag" in e && e.tag === "p");
    expect("props" in pEl! && pEl.props.label).toBe("{name}");
  });

  it("preserves multiple {expr} in mixed text", () => {
    const source = `import omniview;

component Foo {
    view {
        <p>{count} items, {done} done</p>
    }
}`;
    const result = parseOmniSource(source);
    const pEl = result.designElements?.find(e => "tag" in e && e.tag === "p");
    expect("props" in pEl! && pEl.props.label).toBe("{count} items, {done} done");
  });
});
