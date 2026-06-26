// editor/client/src/omni-gen/__tests__/omni-parser.test.ts
import { describe, it, expect } from "vitest";
import { parseOmniSource } from "../omni-parser";

const SIMPLE_OMNI = `// @code-tab type=logic
import omniview;

component Counter {
    signal count: i32 = 0;

    fn inc() -> void {
        count = count + 1;
    }

    view {
        // @edit
        <div id="Counter.root" class: box>
            // @edit
            <button id="Counter.root.button.0" class: btn @onPress={inc}>+</button>
        </div>
    }
    style {
        .box { background: white; padding: 20px; }
        .btn { width: 40px; border-radius: 8px; }
    }
}
`;

describe("parseOmniSource", () => {
  it("parses @code-tab directive", () => {
    const result = parseOmniSource(SIMPLE_OMNI);
    expect(result.errors).toEqual([]);
    expect(result.codeTabs).toHaveLength(1);
    expect(result.codeTabs[0].type).toBe("logic");
  });

  it("parses @code-tab type=event produces event type", () => {
    const result = parseOmniSource(`// @code-tab type=event
component Test { view { <div/> } }`);
    expect(result.codeTabs).toHaveLength(1);
    expect(result.codeTabs[0].type).toBe("event");
  });

  it("parses component signal", () => {
    const result = parseOmniSource(SIMPLE_OMNI);
    const signals = result.designElements!.filter(e => e.tag === "signal");
    expect(signals).toHaveLength(1);
    expect(signals[0].props.name).toBe("count");
    expect(signals[0].props.typeAnnotation).toBe("i32");
  });

  it("parses view elements with @edit annotations", () => {
    const result = parseOmniSource(SIMPLE_OMNI);
    const views = result.designElements!.filter(e => e.nodeType === "view");
    expect(views).toHaveLength(2);
    expect(views[0].id).toBe("Counter.root");
    expect(views[0].tag).toBe("div");
    expect(views[1].id).toBe("Counter.root.button.0");
    expect(views[1].tag).toBe("button");
    expect(views[1].props.onPress).toBe("inc");
  });

  it("builds parent/child relationships", () => {
    const result = parseOmniSource(SIMPLE_OMNI);
    const root = result.designElements!.find(e => e.id === "Counter.root")!;
    const btn = result.designElements!.find(e => e.id === "Counter.root.button.0")!;
    expect(root.children).toContain(btn.id);
    expect(btn.parentId).toBe(root.id);
  });
});
