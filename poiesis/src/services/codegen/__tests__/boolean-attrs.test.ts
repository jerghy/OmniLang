// poiesis/src/services/codegen/__tests__/boolean-attrs.test.ts
import { describe, it, expect } from "vitest";
import { BOOLEAN_ATTRS } from "../boolean-attrs";

describe("BOOLEAN_ATTRS 共享常量", () => {
  it("包含全部 19 个布尔属性", () => {
    const expected = [
      "disabled", "required", "readonly", "checked", "selected",
      "hidden", "multiple", "autofocus", "novalidate",
      "async", "defer", "ismap", "itemscope", "open",
      "loop", "muted", "controls", "autoplay", "autocomplete",
    ];
    for (const name of expected) expect(BOOLEAN_ATTRS.has(name)).toBe(true);
    expect(BOOLEAN_ATTRS.size).toBe(19);
  });
});
