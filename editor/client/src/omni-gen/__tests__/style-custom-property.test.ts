// editor/client/src/omni-gen/__tests__/style-custom-property.test.ts
// End-to-end tests for CSS custom property (--name) support in the WASM compiler.
// These tests compile .omni source through the Rust WASM compiler and verify
// that CSS custom properties are handled correctly in both static style blocks
// and dynamic inline styles.

import { describe, it, expect, beforeAll } from "vitest";
import { rustCompiler } from "../../api/rustCompiler";

beforeAll(async () => {
  await rustCompiler.init();
});

describe("WASM -- CSS custom properties", () => {
  it("parses --custom-property in style block", () => {
    const src = [
      "component Test {",
      "    style {",
      '        .foo { --my-color: #333; color: var(--my-color); }',
      "    }",
      '    view { <div class: foo/> }',
      "}",
    ].join("\n");

    const result = rustCompiler.compileToOmniView(src, true);
    expect(result.success).toBe(true);
    expect(result.files!.css).toContain("--my-color");
    expect(result.files!.css).toContain("var(--my-color)");
  });

  it("generates setProperty for dynamic -- inline styles", () => {
    const src = [
      "component Test {",
      '    signal s: String = "#333";',
      '    view { <div style: { --bg: s }/> }',
      "}",
    ].join("\n");

    const result = rustCompiler.compileToOmniView(src, true);
    expect(result.success).toBe(true);
    expect(result.files!.js).toContain('setProperty("--bg"');
  });

  it("returns error for malformed source", () => {
    const result = rustCompiler.compileToOmniView("invalid @@ garbage", true);
    expect(result.success).toBe(false);
    expect(result.errors).toBeDefined();
    expect(result.errors!.length).toBeGreaterThan(0);
  });
});
