# Editor Style Panel Redesign — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Upgrade editor style editing from 12 flat CSS properties with text inputs to a class-based three-tier style system (variable → class → inline) with smart property editors and full CSS custom property support.

**Architecture:** Three-phase approach. Phase 1 patches the Rust compiler (parser + codegen) and rebuilds WASM — this is the unblocking dependency. Phase 2 adds `StyleEntry[]` to the data model, new Commands for undo/redo, and save/parse roundtrip through `.omni` source. Phase 3 builds the UI: `StylePropertyEditor` (smart controls), `StyleEntryEditor` (class/variable/inline modes), and integrates into `PropertyPanel` as an accordion list mirroring the event list pattern.

**Tech Stack:** TypeScript (React + Zustand), Rust (omni-compiler, wasm-pack), Vitest (unit tests), existing Command pattern

## Global Constraints

- `.omni` source is the single source of truth — no editor-only style annotations
- All style mutations go through Command pattern for undo/redo
- CSS property names use kebab-case in `StyleProperty.name` and all lookup tables
- Variable names use `--` prefix; generated class names use `_v-` prefix
- 30 CSS units supported via smart filtering (not all-at-once dropdowns)
- Backward compatible: old `.omni` without `style { }` blocks must parse (props→inline)
- Parser failures in real-time editing: keep last valid state; strict on save

---

## File Map

| File | Role | Phase |
|------|------|-------|
| `omni-compiler/src/parser/view.rs` | Parser: `--` custom property support in `parse_style_rules()` | 1 |
| `omni-compiler/src/codegen/omniview.rs` | Codegen: `setProperty()` for dynamic `--` props | 1 |
| `editor/public/wasm/*` | Rebuilt WASM binary | 1 |
| `editor/client/src/store/editor.ts` | Data model: `styles: StyleEntry[]` on `ElementData`, store methods | 2 |
| `editor/client/src/commands/AddStyleEntry.ts` | **New** — Command: add style entry | 2 |
| `editor/client/src/commands/RemoveStyleEntry.ts` | **New** — Command: remove style entry | 2 |
| `editor/client/src/commands/ReorderStyleEntry.ts` | **New** — Command: reorder style entries | 2 |
| `editor/client/src/commands/UpdateStyleProperty.ts` | **New** — Command: add/update/delete property in entry | 2 |
| `editor/client/src/commands/SetStyleEntryType.ts` | **New** — Command: switch class/variable/inline | 2 |
| `editor/client/src/commands/RenameStyleEntry.ts` | **New** — Command: rename class/variable | 2 |
| `editor/client/src/constants/style-properties.ts` | **New** — `PROPERTY_UNIT_MAP`, `DROPDOWN_PROPERTIES`, `STYLE_PROPERTY_NAMES` | 2 |
| `editor/client/src/omni-gen/style-resolver.ts` | Parse: `--` custom property passthrough | 1/2 |
| `editor/client/src/omni-gen/pipeline.ts` | Save/parse: collect style entries from AST, emit style blocks | 2 |
| `editor/client/src/omni-gen/source-emitter.ts` | Save: `style { }` block generation | 2 |
| `editor/client/src/panels/StylePropertyEditor.tsx` | **New** — Single property row (name dropdown + value + unit) | 3 |
| `editor/client/src/panels/StyleEntryEditor.tsx` | **New** — Single StyleEntry editor (class/variable/inline modes) | 3 |
| `editor/client/src/panels/PropertyPanel.tsx` | UI: style list integration (accordion, drag-sort) | 3 |

---

## Phase 1: Compiler Layer (Unblocking Dependency)

### Task 1.1: Parser — CSS custom property name support

**Files:**
- Modify: `omni-compiler/src/parser/view.rs` (~line 1171, `parse_style_rules`)

**Interfaces:**
- Consumes: `TokenKind::Minus`, `TokenKind::Ident`, `expect_ident()`, `peek_kind()`
- Produces: CSS custom property name string `"--primary-color"` parsed as valid property key

**Context:** The lexer tokenizes `--primary-color` as `Minus Minus Ident("primary") Minus Ident("color")`. `expect_ident()` fails on the first `Minus`. We need to detect the `--` prefix and collect subsequent `Ident`/`Minus` tokens into a single property name string.

- [ ] **Step 1: Add unit test in Rust**

Create `omni-compiler/tests/css_custom_property_test.rs`:

```rust
use omni_compiler::parser::Parser;

#[test]
fn test_parse_css_custom_property_in_style_block() {
    let src = r#"
component Test {
    style {
        .foo {
            --primary-color: #333;
            --spacing-md: 16px;
            color: var(--primary-color);
            padding: var(--spacing-md);
        }
    }
    view { <div class: foo/> }
}
"#;
    let result = Parser::parse(src);
    assert!(result.is_ok(), "Should parse CSS custom properties: {:?}", result.err());
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd omni-compiler && cargo test test_parse_css_custom_property_in_style_block
```

Expected: FAIL — `expect_ident` errors on `Minus` token at `--primary-color`.

- [ ] **Step 3: Implement `--` prefix detection in `parse_style_rules`**

In `omni-compiler/src/parser/view.rs`, replace the property name parsing (current line `let mut key = self.expect_ident("expected property name")?;`):

```rust
// Parse property name — supports CSS custom properties (--name)
let mut key = if self.peek_kind() == TokenKind::Minus {
    let saved = self.save_position();
    self.advance(); // eat first Minus
    if self.peek_kind() == TokenKind::Minus {
        self.advance(); // eat second Minus
        let mut name = String::from("--");
        loop {
            match self.peek_kind() {
                TokenKind::Ident(_) => {
                    name.push_str(&self.token_text());
                    self.advance();
                }
                TokenKind::Minus => {
                    name.push('-');
                    self.advance();
                }
                _ => break,
            }
        }
        name
    } else {
        self.restore_position(saved);
        self.expect_ident("expected property name")?
    }
} else {
    self.expect_ident("expected property name")?
};
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd omni-compiler && cargo test test_parse_css_custom_property_in_style_block
```

Expected: PASS.

- [ ] **Step 5: Run full test suite**

```bash
cd omni-compiler && cargo test
```

Expected: All existing tests still pass (no regressions).

- [ ] **Step 6: Commit**

```bash
cd D:/space/OmniLang && git add omni-compiler/src/parser/view.rs omni-compiler/tests/css_custom_property_test.rs
git commit -m "feat(parser): add CSS custom property (--name) support in style blocks"
```

---

### Task 1.2: OmniView Codegen — setProperty for dynamic inline styles

**Files:**
- Modify: `omni-compiler/src/codegen/omniview.rs` (lines ~1902, ~1930, ~1965)

**Interfaces:**
- Produces: Generated JS uses `el.style.setProperty("--prop", value)` when prop starts with `--`, otherwise `el.style.prop = value`

**Context:** Three code paths in `element_to_html()` use `el.style.{js_prop} = value` for dynamic inline styles. This is invalid JS for `--` prefixed properties. Must use `el.style.setProperty()` instead.

- [ ] **Step 1: Add inline style test case**

Append to `omni-compiler/tests/css_custom_property_test.rs`:

```rust
#[test]
fn test_inline_style_custom_property_dynamic() {
    // Test that dynamic inline styles with -- properties use setProperty
    let src = r#"
component InlineTest {
    signal color: String = "#ff0000";
    view {
        <div style: { --custom-color: color, padding: "16px" }>Hello</div>
    }
}
"#;
    let result = omni_compiler::compile_to_omniview(src).unwrap();
    // The JS output should contain setProperty for --custom-color
    assert!(result.js.contains("setProperty(\"--custom-color\"")
        || result.js.contains("setProperty(\"--custom-color\""),
        "Dynamic -- prop must use setProperty. JS: {}", result.js);
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd omni-compiler && cargo test test_inline_style_custom_property_dynamic
```

Expected: FAIL — generated JS contains `el.style.--custom-color` which is invalid.

- [ ] **Step 3: Implement setProperty fallback**

In `omni-compiler/src/codegen/omniview.rs`, add a helper function near the top of the style emission section:

```rust
fn emit_style_assignment(&mut self, prop: &str, value_expr: &str) {
    if prop.starts_with("--") {
        self.js.push_str(&format!(
            "el.style.setProperty(\"{}\",{});", prop, value_expr
        ));
    } else {
        self.js.push_str(&format!(
            "el.style.{}={};", to_js_prop(prop), value_expr
        ));
    }
}
```

Then replace the three `el.style.{js_prop} = ...` lines:

**Line ~1902 ($signal dynamic style):**
```rust
// Before: self.js.push_str(&format!("el.style.{}={};", js_prop, val));
// After:
self.emit_style_assignment(&prop, &val);
```

**Line ~1930 ($concat dynamic style):** Same replacement pattern.

**Line ~1965 ($call dynamic style):** Same replacement pattern.

- [ ] **Step 4: Also verify static inline style path**

Check that the static inline style path (~line 1972) correctly outputs `--primary-color: #333` in the `style=""` HTML attribute. This should work because `to_css_property("--primary-color")` returns `"--primary-color"` unchanged (no uppercase letters to convert).

Add to test:
```rust
#[test]
fn test_static_inline_style_custom_property() {
    let src = r#"
component StaticTest {
    view {
        <div style: { --bg: "#eee" }>Hello</div>
    }
}
"#;
    let result = omni_compiler::compile_to_omniview(src).unwrap();
    assert!(result.html.contains("--bg:#eee") || result.html.contains("--bg: #eee"),
        "Static -- prop must appear in style attribute. HTML: {}", result.html);
}
```

- [ ] **Step 5: Run all tests**

```bash
cd omni-compiler && cargo test
```

Expected: All tests PASS, including new custom property tests.

- [ ] **Step 6: Commit**

```bash
git add omni-compiler/src/codegen/omniview.rs omni-compiler/tests/css_custom_property_test.rs
git commit -m "fix(codegen): use setProperty for CSS custom properties in dynamic inline styles"
```

---

### Task 1.3: Rebuild WASM and verify

**Files:**
- Regenerate: `editor/public/wasm/omni_compiler_bg.wasm`, `editor/public/wasm/omni_compiler.js`, `editor/public/wasm/omni_compiler.d.ts`

**Interfaces:**
- Consumes: Updated Rust compiler from Tasks 1.1-1.2
- Produces: WASM binary that supports `--` custom properties in parse and codegen

- [ ] **Step 1: Build WASM**

```bash
cd D:/space/OmniLang/omni-compiler && wasm-pack build --target web --out-dir ../editor/public/wasm --features wasm
```

Expected: Build succeeds, files written to `editor/public/wasm/`.

- [ ] **Step 2: Verify WASM parses custom properties**

Create `editor/client/src/omni-gen/__tests__/style-custom-property.test.ts`:

```typescript
import { describe, it, expect } from "vitest";

// Dynamic import of the WASM compiler
async function getCompiler() {
  const wasmModule = await import("../../../public/wasm/omni_compiler.js");
  await wasmModule.default();
  return wasmModule;
}

describe("WASM — CSS custom properties", () => {
  it("parses --custom-property in style block", async () => {
    const compiler = await getCompiler();
    const src = `
component Test {
    style {
        .foo { --my-color: #333; color: var(--my-color); }
    }
    view { <div class: foo/> }
}`;
    const result = compiler.compile_to_omniview(src);
    expect(result).toBeDefined();
    expect(result.css).toContain("--my-color");
    expect(result.css).toContain("var(--my-color)");
  });

  it("generates setProperty for dynamic -- inline styles", async () => {
    const compiler = await getCompiler();
    const src = `
component Test {
    signal s: String = "#333";
    view { <div style: { --bg: s }/> }
}`;
    const result = compiler.compile_to_omniview(src);
    expect(result.js).toContain("setProperty");
  });
});
```

- [ ] **Step 3: Run WASM tests**

```bash
cd D:/space/OmniLang/editor/client && npx vitest run src/omni-gen/__tests__/style-custom-property.test.ts
```

Expected: Both tests PASS.

- [ ] **Step 4: Commit**

```bash
git add editor/public/wasm/omni_compiler_bg.wasm editor/public/wasm/omni_compiler.js editor/public/wasm/omni_compiler.d.ts editor/client/src/omni-gen/__tests__/style-custom-property.test.ts
git commit -m "chore: rebuild WASM with CSS custom property support"
```

---

### Task 1.4: Editor style-resolver — passthrough `--` properties

**Files:**
- Modify: `editor/client/src/omni-gen/style-resolver.ts`

**Interfaces:**
- Consumes: `resolveStyle(node)` function, `STYLE_PROP_CSS_MAP`
- Produces: `--` prefixed keys written to `cssRules` instead of `htmlAttrs`

- [ ] **Step 1: Write test**

Add to `editor/client/src/omni-gen/__tests__/style-custom-property.test.ts`:

```typescript
import { resolveStyle } from "../style-resolver";
import type { ViewNode } from "../types";

describe("style-resolver — CSS custom properties", () => {
  it("routes --prefixed keys to cssRules not htmlAttrs", () => {
    const node: ViewNode = {
      tag: "Container",
      path: "root.container",
      props: { "--primary-color": "#333", padding: "16px" },
      children: [],
    };
    const result = resolveStyle(node);
    expect(result.cssRules["--primary-color"]).toBe("#333");
    expect(result.htmlAttrs["--primary-color"]).toBeUndefined();
  });

  it("backward compatible: non-prefixed keys still use STYLE_PROP_CSS_MAP", () => {
    const node: ViewNode = {
      tag: "Container",
      path: "root.container",
      props: { background_color: "#eee" },
      children: [],
    };
    const result = resolveStyle(node);
    expect(result.cssRules["background"]).toBe("#eee");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd D:/space/OmniLang/editor/client && npx vitest run src/omni-gen/__tests__/style-custom-property.test.ts -t "style-resolver"
```

Expected: FAIL — `--primary-color` currently goes to `htmlAttrs`.

- [ ] **Step 3: Implement passthrough**

In `editor/client/src/omni-gen/style-resolver.ts`, in `resolveStyle()`, after the STYLE_PROP_CSS_MAP lookup, add:

```typescript
// Before the htmlAttrs fallback (current line ~93):
if (key.startsWith("--")) {
  cssRules[key] = String(val);  // CSS custom property → cssRules
} else if (htmlPropNames.has(key)) {
  htmlAttrs[key] = String(val); // known HTML attr
} else {
  htmlAttrs[key] = String(val); // unknown → HTML attr
}
```

Replace the existing fallback:
```typescript
// Current: htmlAttrs[key] = String(val);
// New: conditional check as above
```

- [ ] **Step 4: Run test to verify it passes + run full suite**

```bash
cd D:/space/OmniLang/editor/client && npx vitest run src/omni-gen/__tests__/style-custom-property.test.ts -t "style-resolver"
npx vitest run
```

Expected: All tests PASS.

- [ ] **Step 5: Commit**

```bash
git add editor/client/src/omni-gen/style-resolver.ts editor/client/src/omni-gen/__tests__/style-custom-property.test.ts
git commit -m "fix(style-resolver): route -- CSS custom properties to cssRules"
```

---

## Phase 2: Data Model, Commands, Save/Parse

### Task 2.1: Create style constants file

**Files:**
- Create: `editor/client/src/constants/style-properties.ts`

**Interfaces:**
- Produces: `PROPERTY_UNIT_MAP`, `DROPDOWN_PROPERTIES`, `STYLE_PROPERTY_NAMES`, `StyleProperty`, `StyleEntry` types

- [ ] **Step 1: Create the constants file**

```typescript
// editor/client/src/constants/style-properties.ts

// ─── Types ───────────────────────────────────────────
export interface StyleProperty {
  name: string;   // kebab-case CSS property name, e.g. "font-size", "--primary-color"
  value: string;  // CSS value, e.g. "16px", "#333", "var(--primary-color)"
}

export interface StyleEntry {
  id: string;
  type: "class" | "variable" | "inline";
  name: string;  // class name | variable name (--xxx) | "" (inline)
  properties: StyleProperty[];
}

// ─── Style property name set (for migration + parsing) ─
export const STYLE_PROPERTY_NAMES = new Set([
  // From STYLE_PROP_CSS_MAP legacy
  "background_color", "border_radius", "font_size", "font_weight",
  "text_align", "shadow", "padding", "margin", "gap",
  "width", "height", "color",
  // Additional CSS properties (kebab-case + camelCase aliases)
  "display", "position", "opacity", "overflow", "z-index", "z_index", "zIndex",
  "box-shadow", "box_shadow", "boxShadow",
  "border", "border-color", "border_color", "borderColor",
  "font-family", "font_family", "fontFamily",
  "line-height", "line_height", "lineHeight",
  "letter-spacing", "letter_spacing", "letterSpacing",
  "text-decoration", "text_decoration", "textDecoration",
  "text-transform", "text_transform", "textTransform",
  "white-space", "white_space", "whiteSpace",
  "word-break", "word_break", "wordBreak",
  "min-width", "min_width", "minWidth",
  "max-width", "max_width", "maxWidth",
  "min-height", "min_height", "minHeight",
  "max-height", "max_height", "maxHeight",
  "transform", "transition", "animation", "cursor",
  "box-sizing", "box_sizing", "boxSizing",
  "flex-direction", "flex_direction", "flexDirection",
  "flex-wrap", "flex_wrap", "flexWrap",
  "flex-grow", "flex_grow", "flexGrow",
  "flex-shrink", "flex_shrink", "flexShrink",
  "flex-basis", "flex_basis", "flexBasis",
  "justify-content", "justify_content", "justifyContent",
  "align-items", "align_items", "alignItems",
  "align-self", "align_self", "alignSelf",
  "grid-template-columns", "grid_template_columns", "gridTemplateColumns",
  "grid-template-rows", "grid_template_rows", "gridTemplateRows",
  "grid-column", "grid_column", "gridColumn",
  "grid-row", "grid_row", "gridRow",
  "background-image", "background_image", "backgroundImage",
  "transition-property", "transition_property", "transitionProperty",
  "transition-duration", "transition_duration", "transitionDuration",
  "transition-timing-function", "transition_timing_function", "transitionTimingFunction",
  "animation-name", "animation_name", "animationName",
  "animation-duration", "animation_duration", "animationDuration",
  "animation-iteration-count", "animation_iteration_count", "animationIterationCount",
  "animation-direction", "animation_direction", "animationDirection",
  "animation-fill-mode", "animation_fill_mode", "animationFillMode",
  "animation-timing-function", "animation_timing_function", "animationTimingFunction",
]);

// ─── Property → Unit mapping (kebab-case keys) ────────
export interface UnitGroup {
  default: string;        // default unit ("" = unitless)
  common: string[];       // always-visible units
  advanced?: string[];    // hidden behind "more" toggle
}

export const PROPERTY_UNIT_MAP: Record<string, UnitGroup> = {
  // Length
  width:        { default: "px", common: ["px", "rem", "em", "%"], advanced: ["vw", "vh", "vmin", "vmax", "auto"] },
  height:       { default: "px", common: ["px", "rem", "em", "%"], advanced: ["vw", "vh", "vmin", "vmax", "auto"] },
  "min-width":  { default: "px", common: ["px", "rem", "em", "%"], advanced: ["vw", "vh"] },
  "max-width":  { default: "px", common: ["px", "rem", "em", "%"], advanced: ["vw", "vh", "none"] },
  "min-height": { default: "px", common: ["px", "rem", "em", "%"], advanced: ["vw", "vh"] },
  "max-height": { default: "px", common: ["px", "rem", "em", "%"], advanced: ["vw", "vh", "none"] },
  padding:      { default: "px", common: ["px", "rem", "em", "%"] },
  "padding-top":    { default: "px", common: ["px", "rem", "em", "%"] },
  "padding-right":  { default: "px", common: ["px", "rem", "em", "%"] },
  "padding-bottom": { default: "px", common: ["px", "rem", "em", "%"] },
  "padding-left":   { default: "px", common: ["px", "rem", "em", "%"] },
  margin:       { default: "px", common: ["px", "rem", "em", "%"], advanced: ["auto"] },
  "margin-top":    { default: "px", common: ["px", "rem", "em", "%"], advanced: ["auto"] },
  "margin-right":  { default: "px", common: ["px", "rem", "em", "%"], advanced: ["auto"] },
  "margin-bottom": { default: "px", common: ["px", "rem", "em", "%"], advanced: ["auto"] },
  "margin-left":   { default: "px", common: ["px", "rem", "em", "%"], advanced: ["auto"] },
  gap:          { default: "px", common: ["px", "rem", "em", "%"] },

  // Typography
  "font-size":      { default: "px", common: ["px", "rem", "em", "%"], advanced: ["pt", "ch"] },
  "line-height":    { default: "", common: ["", "px", "rem", "em", "%"], advanced: ["ch"] },
  "letter-spacing": { default: "px", common: ["px", "rem", "em"] },

  // Borders & effects
  "border-radius":  { default: "px", common: ["px", "rem", "em", "%"] },
  "box-shadow":     { default: "px", common: ["px"] },

  // Time
  "transition-duration": { default: "ms", common: ["ms", "s"] },
  "animation-duration":  { default: "ms", common: ["ms", "s"] },
};

// ─── Dropdown-only properties (kebab-case keys) ───────
// null = free text input
export const DROPDOWN_PROPERTIES: Record<string, string[] | null> = {
  display:            ["block", "flex", "grid", "inline", "inline-block", "inline-flex", "none"],
  position:           ["static", "relative", "absolute", "fixed", "sticky"],
  "font-weight":      ["100", "200", "300", "400", "500", "600", "700", "800", "900", "normal", "bold", "lighter", "bolder"],
  "text-align":       ["left", "center", "right", "justify", "start", "end"],
  overflow:           ["visible", "hidden", "scroll", "auto"],
  "overflow-x":       ["visible", "hidden", "scroll", "auto"],
  "overflow-y":       ["visible", "hidden", "scroll", "auto"],
  cursor:             ["auto", "default", "pointer", "text", "move", "not-allowed", "grab", "grabbing", "crosshair", "zoom-in", "zoom-out"],
  "box-sizing":       ["content-box", "border-box"],
  "text-transform":   ["none", "uppercase", "lowercase", "capitalize"],
  "text-decoration":  ["none", "underline", "line-through", "overline"],
  "white-space":      ["normal", "nowrap", "pre", "pre-wrap", "pre-line"],
  "word-break":       ["normal", "break-all", "break-word", "keep-all"],
  "flex-direction":   ["row", "row-reverse", "column", "column-reverse"],
  "flex-wrap":        ["nowrap", "wrap", "wrap-reverse"],
  "justify-content":  ["flex-start", "flex-end", "center", "space-between", "space-around", "space-evenly"],
  "align-items":      ["stretch", "flex-start", "flex-end", "center", "baseline"],
  "align-self":       ["auto", "stretch", "flex-start", "flex-end", "center", "baseline"],
  "transition-timing-function": ["ease", "ease-in", "ease-out", "ease-in-out", "linear", "step-start", "step-end"],
  "animation-iteration-count":  ["infinite", "1", "2", "3"],
  "animation-direction":        ["normal", "reverse", "alternate", "alternate-reverse"],
  "animation-fill-mode":        ["none", "forwards", "backwards", "both"],
  "animation-timing-function":  ["ease", "ease-in", "ease-out", "ease-in-out", "linear"],
  // Free-text properties (null = text input)
  "grid-template-columns": null,
  "grid-template-rows": null,
  "grid-column": null,
  "grid-row": null,
  "transition-property": null,
  "animation-name": null,
  "flex-grow": null,
  "flex-shrink": null,
  "flex-basis": null,
  "font-family": null,
  "background-image": null,
  "box-shadow": null,
  transform: null,
};

// ─── Helpers ──────────────────────────────────────────
/** Detect if a property name is a CSS custom property */
export function isCustomProperty(name: string): boolean {
  return name.startsWith("--");
}

/** Detect if a property is color-like (needs color picker) */
export function isColorProperty(name: string): boolean {
  return name.includes("color") && !isCustomProperty(name);
}

/** Get the unit group for a property, or null if unitless */
export function getUnitGroup(propName: string): UnitGroup | null {
  return PROPERTY_UNIT_MAP[propName] ?? null;
}

/** Get dropdown options for a property, or null if text input */
export function getDropdownOptions(propName: string): string[] | null {
  const opts = DROPDOWN_PROPERTIES[propName];
  if (opts === undefined) return null;   // not in dropdown list
  return opts;                            // string[] or null (free text)
}

/** Variable name "--primary-color" → class name "_v-primary-color" */
export function variableToClassName(varName: string): string {
  return "_v-" + varName.slice(2); // strip --, prefix _v-
}

/** Class name "_v-primary-color" → variable name "--primary-color" */
export function classNameToVariable(className: string): string | null {
  if (className.startsWith("_v-")) {
    return "--" + className.slice(3);
  }
  return null;
}
```

- [ ] **Step 2: Verify no import errors**

```bash
cd D:/space/OmniLang/editor/client && npx tsc --noEmit src/constants/style-properties.ts
```

Expected: No type errors.

- [ ] **Step 3: Commit**

```bash
git add editor/client/src/constants/style-properties.ts
git commit -m "feat: add style-properties constants (PROPERTY_UNIT_MAP, DROPDOWN_PROPERTIES, STYLE_PROPERTY_NAMES)"
```

---

### Task 2.2: Add styles to ElementData + store methods

**Files:**
- Modify: `editor/client/src/store/editor.ts`

**Interfaces:**
- Consumes: `StyleEntry`, `StyleProperty` from `constants/style-properties`
- Produces: `ElementData.styles: StyleEntry[]`, store methods `getStyles`, `setStyles`, `updateStyleEntry`, `removeStyleEntry`

- [ ] **Step 1: Write tests for new store methods**

Create `editor/client/src/store/__tests__/style-store.test.ts`:

```typescript
import { describe, it, expect, beforeEach } from "vitest";
import { useEditorStore } from "../editor";
import type { StyleEntry } from "../../constants/style-properties";

function makeEntry(overrides: Partial<StyleEntry> = {}): StyleEntry {
  return {
    id: "s1",
    type: "class",
    name: "card-style",
    properties: [{ name: "padding", value: "16px" }],
    ...overrides,
  };
}

describe("Style store methods", () => {
  beforeEach(() => {
    useEditorStore.setState({ elements: [] });
  });

  it("addStyleEntry adds entry to element.styles", () => {
    const el = { id: "a", tag: "Button", props: {}, children: [], parentId: null, nodeType: "view" as const };
    useEditorStore.setState({ elements: [el] });

    const entry = makeEntry();
    useEditorStore.getState().addStyleEntry("a", entry);

    const updated = useEditorStore.getState().elements.find(e => e.id === "a");
    expect(updated?.styles).toHaveLength(1);
    expect(updated?.styles?.[0].name).toBe("card-style");
  });

  it("removeStyleEntry removes entry by id", () => {
    const entry = makeEntry();
    const el = { id: "a", tag: "Button", props: {}, children: [], parentId: null, nodeType: "view" as const, styles: [entry] };
    useEditorStore.setState({ elements: [el] });

    useEditorStore.getState().removeStyleEntry("a", "s1");

    const updated = useEditorStore.getState().elements.find(e => e.id === "a");
    expect(updated?.styles).toHaveLength(0);
  });

  it("updateStyleEntry updates a specific entry", () => {
    const entry = makeEntry();
    const el = { id: "a", tag: "Button", props: {}, children: [], parentId: null, nodeType: "view" as const, styles: [entry] };
    useEditorStore.setState({ elements: [el] });

    useEditorStore.getState().updateStyleEntry("a", "s1", { name: "new-name" });

    const updated = useEditorStore.getState().elements.find(e => e.id === "a");
    expect(updated?.styles?.[0].name).toBe("new-name");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd D:/space/OmniLang/editor/client && npx vitest run src/store/__tests__/style-store.test.ts
```

Expected: FAIL — `styles` field doesn't exist on `ElementData`, `addStyleEntry` etc. not defined.

- [ ] **Step 3: Add `styles` field to ElementData and store methods**

In `editor/client/src/store/editor.ts`:

Add to `ElementData` interface (after `props`):
```typescript
styles?: StyleEntry[];  // ordered style list, later entries override earlier ones
```

Add to `EditorStore` interface:
```typescript
addStyleEntry: (elementId: string, entry: StyleEntry) => void;
removeStyleEntry: (elementId: string, entryId: string) => void;
updateStyleEntry: (elementId: string, entryId: string, patch: Partial<StyleEntry>) => void;
reorderStyleEntries: (elementId: string, entryIds: string[]) => void;
setStyleEntryType: (elementId: string, entryId: string, newType: StyleEntry["type"]) => void;
renameStyleEntry: (elementId: string, entryId: string, newName: string) => void;
getStyles: (elementId: string) => StyleEntry[];
```

Add implementations in `create`:
```typescript
addStyleEntry: (elementId, entry) =>
  set((s) => ({
    elements: s.elements.map((e) =>
      e.id === elementId
        ? { ...e, styles: [...(e.styles || []), entry] }
        : e
    ),
    isDirty: true,
  })),

removeStyleEntry: (elementId, entryId) =>
  set((s) => ({
    elements: s.elements.map((e) =>
      e.id === elementId
        ? { ...e, styles: (e.styles || []).filter((s) => s.id !== entryId) }
        : e
    ),
    isDirty: true,
  })),

updateStyleEntry: (elementId, entryId, patch) =>
  set((s) => ({
    elements: s.elements.map((e) =>
      e.id === elementId
        ? { ...e, styles: (e.styles || []).map((s) =>
            s.id === entryId ? { ...s, ...patch } : s
          )}
        : e
    ),
    isDirty: true,
  })),

reorderStyleEntries: (elementId, entryIds) =>
  set((s) => {
    const el = s.elements.find((e) => e.id === elementId);
    if (!el?.styles) return s;
    const byId = new Map(el.styles.map((s) => [s.id, s]));
    const reordered = entryIds.map((id) => byId.get(id)!).filter(Boolean);
    return {
      elements: s.elements.map((e) =>
        e.id === elementId ? { ...e, styles: reordered } : e
      ),
      isDirty: true,
    };
  }),

getStyles: (elementId) => {
  return get().elements.find((e) => e.id === elementId)?.styles || [];
},
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd D:/space/OmniLang/editor/client && npx vitest run src/store/__tests__/style-store.test.ts
```

Expected: PASS.

- [ ] **Step 5: Run full test suite**

```bash
cd D:/space/OmniLang/editor/client && npx vitest run
```

Expected: All existing tests still pass.

- [ ] **Step 6: Commit**

```bash
git add editor/client/src/store/editor.ts editor/client/src/store/__tests__/style-store.test.ts
git commit -m "feat: add styles: StyleEntry[] to ElementData with store methods"
```

---

### Task 2.3: Create Style Commands

**Files:**
- Create: `editor/client/src/commands/AddStyleEntry.ts`
- Create: `editor/client/src/commands/RemoveStyleEntry.ts`
- Create: `editor/client/src/commands/ReorderStyleEntry.ts`
- Create: `editor/client/src/commands/UpdateStyleProperty.ts`
- Create: `editor/client/src/commands/SetStyleEntryType.ts`
- Create: `editor/client/src/commands/RenameStyleEntry.ts`

**Interfaces:**
- Each command implements the existing `Command` interface (`execute(ctx)`, `undo(ctx)`)
- Consumes `ctx` with new store methods from Task 2.2

- [ ] **Step 1: Write tests for all 6 commands**

Create `editor/client/src/commands/__tests__/style-commands.test.ts`:

```typescript
import { describe, it, expect, beforeEach } from "vitest";
import { AddStyleEntry } from "../AddStyleEntry";
import { RemoveStyleEntry } from "../RemoveStyleEntry";
import { ReorderStyleEntry } from "../ReorderStyleEntry";
import { UpdateStyleProperty } from "../UpdateStyleProperty";
import { SetStyleEntryType } from "../SetStyleEntryType";
import { RenameStyleEntry } from "../RenameStyleEntry";
import type { CommandContext } from "../types";
import type { StyleEntry } from "../../constants/style-properties";

function makeCtx(elements: any[] = []): CommandContext {
  return {
    elements: JSON.parse(JSON.stringify(elements)),
    getProp: (id: string, prop: string) =>
      elements.find((e: any) => e.id === id)?.props?.[prop],
    setProp: (id: string, prop: string, value: unknown) => {
      const el = elements.find((e: any) => e.id === id);
      if (el) { el.props ||= {}; el.props[prop] = value; }
    },
    getElement: (id: string) => elements.find((e: any) => e.id === id) ?? null,
    addElement: () => {},
    removeElement: () => {},
    addWire: () => {},
    removeWire: () => {},
    setElementsAndWires: (els: any[]) => { elements.length = 0; elements.push(...els); },
    reorderElement: () => {},
    moveElement: () => {},
  } as unknown as CommandContext;
}

const baseEl = {
  id: "a", tag: "Button", props: {}, children: [], parentId: null, nodeType: "view" as const, styles: [] as StyleEntry[],
};

describe("AddStyleEntry", () => {
  it("execute adds entry", () => {
    const el = { ...baseEl, styles: [] as StyleEntry[] };
    const ctx = makeCtx([el]);
    const entry: StyleEntry = { id: "s1", type: "class", name: "card", properties: [{ name: "padding", value: "16px" }] };
    new AddStyleEntry("a", entry).execute(ctx);
    expect((ctx.elements[0] as any).styles).toHaveLength(1);
  });

  it("undo removes the entry", () => {
    const el = { ...baseEl, styles: [] as StyleEntry[] };
    const ctx = makeCtx([el]);
    const entry: StyleEntry = { id: "s1", type: "class", name: "card", properties: [] };
    const cmd = new AddStyleEntry("a", entry);
    cmd.execute(ctx);
    cmd.undo(ctx);
    expect((ctx.elements[0] as any).styles).toHaveLength(0);
  });
});

describe("RemoveStyleEntry", () => {
  it("execute removes entry at index", () => {
    const entry: StyleEntry = { id: "s1", type: "class", name: "card", properties: [] };
    const el = { ...baseEl, styles: [entry] };
    const ctx = makeCtx([el]);
    new RemoveStyleEntry("a", entry, 0).execute(ctx);
    expect((ctx.elements[0] as any).styles).toHaveLength(0);
  });

  it("undo restores entry at original index", () => {
    const entry: StyleEntry = { id: "s1", type: "class", name: "card", properties: [] };
    const el = { ...baseEl, styles: [entry] };
    const ctx = makeCtx([el]);
    const cmd = new RemoveStyleEntry("a", entry, 0);
    cmd.execute(ctx);
    cmd.undo(ctx);
    expect((ctx.elements[0] as any).styles).toHaveLength(1);
  });
});

describe("ReorderStyleEntry", () => {
  it("execute reorders entries", () => {
    const s1: StyleEntry = { id: "s1", type: "class", name: "first", properties: [] };
    const s2: StyleEntry = { id: "s2", type: "class", name: "second", properties: [] };
    const el = { ...baseEl, styles: [s1, s2] };
    const ctx = makeCtx([el]);
    new ReorderStyleEntry("a", ["s1", "s2"], ["s2", "s1"]).execute(ctx);
    expect((ctx.elements[0] as any).styles[0].name).toBe("second");
  });

  it("undo restores original order", () => {
    const s1: StyleEntry = { id: "s1", type: "class", name: "first", properties: [] };
    const s2: StyleEntry = { id: "s2", type: "class", name: "second", properties: [] };
    const el = { ...baseEl, styles: [s1, s2] };
    const ctx = makeCtx([el]);
    const cmd = new ReorderStyleEntry("a", ["s1", "s2"], ["s2", "s1"]);
    cmd.execute(ctx);
    cmd.undo(ctx);
    expect((ctx.elements[0] as any).styles[0].name).toBe("first");
  });
});

describe("UpdateStyleProperty", () => {
  it("execute modifies a property value", () => {
    const entry: StyleEntry = { id: "s1", type: "class", name: "card", properties: [{ name: "padding", value: "8px" }] };
    const el = { ...baseEl, styles: [entry] };
    const ctx = makeCtx([el]);
    new UpdateStyleProperty("a", "s1", 0, { name: "padding", value: "8px" }, { name: "padding", value: "16px" }).execute(ctx);
    expect((ctx.elements[0] as any).styles[0].properties[0].value).toBe("16px");
  });

  it("undo reverts the property", () => {
    const entry: StyleEntry = { id: "s1", type: "class", name: "card", properties: [{ name: "padding", value: "8px" }] };
    const el = { ...baseEl, styles: [entry] };
    const ctx = makeCtx([el]);
    const cmd = new UpdateStyleProperty("a", "s1", 0, { name: "padding", value: "8px" }, { name: "padding", value: "16px" });
    cmd.execute(ctx);
    cmd.undo(ctx);
    expect((ctx.elements[0] as any).styles[0].properties[0].value).toBe("8px");
  });

  it("propertyIndex -1 means add new property", () => {
    const entry: StyleEntry = { id: "s1", type: "class", name: "card", properties: [] };
    const el = { ...baseEl, styles: [entry] };
    const ctx = makeCtx([el]);
    new UpdateStyleProperty("a", "s1", -1, null, { name: "color", value: "#333" }).execute(ctx);
    expect((ctx.elements[0] as any).styles[0].properties).toHaveLength(1);
  });
});

describe("SetStyleEntryType", () => {
  it("execute changes type from class to variable", () => {
    const entry: StyleEntry = { id: "s1", type: "class", name: "card", properties: [] };
    const el = { ...baseEl, styles: [entry] };
    const ctx = makeCtx([el]);
    new SetStyleEntryType("a", "s1", "class", "variable").execute(ctx);
    expect((ctx.elements[0] as any).styles[0].type).toBe("variable");
  });

  it("undo reverts type", () => {
    const entry: StyleEntry = { id: "s1", type: "class", name: "card", properties: [] };
    const el = { ...baseEl, styles: [entry] };
    const ctx = makeCtx([el]);
    const cmd = new SetStyleEntryType("a", "s1", "class", "variable");
    cmd.execute(ctx);
    cmd.undo(ctx);
    expect((ctx.elements[0] as any).styles[0].type).toBe("class");
  });
});

describe("RenameStyleEntry", () => {
  it("execute changes the name", () => {
    const entry: StyleEntry = { id: "s1", type: "class", name: "old-name", properties: [] };
    const el = { ...baseEl, styles: [entry] };
    const ctx = makeCtx([el]);
    new RenameStyleEntry("a", "s1", "old-name", "new-name").execute(ctx);
    expect((ctx.elements[0] as any).styles[0].name).toBe("new-name");
  });

  it("undo reverts name", () => {
    const entry: StyleEntry = { id: "s1", type: "class", name: "old-name", properties: [] };
    const el = { ...baseEl, styles: [entry] };
    const ctx = makeCtx([el]);
    const cmd = new RenameStyleEntry("a", "s1", "old-name", "new-name");
    cmd.execute(ctx);
    cmd.undo(ctx);
    expect((ctx.elements[0] as any).styles[0].name).toBe("old-name");
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd D:/space/OmniLang/editor/client && npx vitest run src/commands/__tests__/style-commands.test.ts
```

Expected: FAIL — modules not found.

- [ ] **Step 3: Implement all 6 command files**

Create `editor/client/src/commands/AddStyleEntry.ts`:
```typescript
import type { Command, CommandContext } from "./types";
import type { StyleEntry } from "../constants/style-properties";

export class AddStyleEntry implements Command {
  readonly type = "AddStyleEntry";
  readonly description: string;
  constructor(
    private elementId: string,
    private entry: StyleEntry,
  ) {
    this.description = `添加样式 ${entry.name || entry.type}`;
  }

  execute(ctx: CommandContext): void {
    const el = ctx.getElement(this.elementId) as any;
    if (!el) return;
    el.styles ||= [];
    el.styles.push(this.entry);
  }

  undo(ctx: CommandContext): void {
    const el = ctx.getElement(this.elementId) as any;
    if (!el?.styles) return;
    el.styles = el.styles.filter((s: StyleEntry) => s.id !== this.entry.id);
  }
}
```

Create `editor/client/src/commands/RemoveStyleEntry.ts`:
```typescript
import type { Command, CommandContext } from "./types";
import type { StyleEntry } from "../constants/style-properties";

export class RemoveStyleEntry implements Command {
  readonly type = "RemoveStyleEntry";
  readonly description: string;
  constructor(
    private elementId: string,
    private entry: StyleEntry,
    private index: number,
  ) {
    this.description = `删除样式 ${this.entry.name || this.entry.type}`;
  }

  execute(ctx: CommandContext): void {
    const el = ctx.getElement(this.elementId) as any;
    if (!el?.styles) return;
    el.styles = el.styles.filter((s: StyleEntry) => s.id !== this.entry.id);
  }

  undo(ctx: CommandContext): void {
    const el = ctx.getElement(this.elementId) as any;
    if (!el) return;
    el.styles ||= [];
    el.styles.splice(this.index, 0, this.entry);
  }
}
```

Create `editor/client/src/commands/ReorderStyleEntry.ts`:
```typescript
import type { Command, CommandContext } from "./types";
import type { StyleEntry } from "../constants/style-properties";

export class ReorderStyleEntry implements Command {
  readonly type = "ReorderStyleEntry";
  readonly description = "重新排序样式";
  constructor(
    private elementId: string,
    private oldOrder: string[],
    private newOrder: string[],
  ) {}

  private applyOrder(ctx: CommandContext, order: string[]): void {
    const el = ctx.getElement(this.elementId) as any;
    if (!el?.styles) return;
    const byId = new Map(el.styles.map((s: StyleEntry) => [s.id, s]));
    el.styles = order.map((id) => byId.get(id)!).filter(Boolean);
  }

  execute(ctx: CommandContext): void {
    this.applyOrder(ctx, this.newOrder);
  }

  undo(ctx: CommandContext): void {
    this.applyOrder(ctx, this.oldOrder);
  }
}
```

Create `editor/client/src/commands/UpdateStyleProperty.ts`:
```typescript
import type { Command, CommandContext } from "./types";
import type { StyleEntry, StyleProperty } from "../constants/style-properties";

export class UpdateStyleProperty implements Command {
  readonly type = "UpdateStyleProperty";
  readonly description: string;
  constructor(
    private elementId: string,
    private entryId: string,
    private propertyIndex: number,
    private oldProperty: StyleProperty | null,
    private newProperty: StyleProperty | null,
  ) {
    this.description = newProperty
      ? `修改样式属性 ${newProperty.name}`
      : `删除样式属性 ${oldProperty?.name}`;
  }

  execute(ctx: CommandContext): void {
    const el = ctx.getElement(this.elementId) as any;
    if (!el?.styles) return;
    const entry = el.styles.find((s: StyleEntry) => s.id === this.entryId);
    if (!entry) return;
    if (this.propertyIndex === -1 && this.newProperty) {
      entry.properties.push(this.newProperty); // add
    } else if (this.newProperty === null) {
      entry.properties.splice(this.propertyIndex, 1); // delete
    } else {
      entry.properties[this.propertyIndex] = this.newProperty; // update
    }
  }

  undo(ctx: CommandContext): void {
    const el = ctx.getElement(this.elementId) as any;
    if (!el?.styles) return;
    const entry = el.styles.find((s: StyleEntry) => s.id === this.entryId);
    if (!entry) return;
    if (this.propertyIndex === -1 && this.newProperty) {
      // Undo add: remove last (assumes it was pushed)
      entry.properties.pop();
    } else if (this.newProperty === null && this.oldProperty) {
      // Undo delete: re-insert
      entry.properties.splice(this.propertyIndex, 0, this.oldProperty);
    } else if (this.oldProperty) {
      entry.properties[this.propertyIndex] = this.oldProperty;
    }
  }
}
```

Create `editor/client/src/commands/SetStyleEntryType.ts`:
```typescript
import type { Command, CommandContext } from "./types";
import type { StyleEntry } from "../constants/style-properties";

export class SetStyleEntryType implements Command {
  readonly type = "SetStyleEntryType";
  readonly description: string;
  constructor(
    private elementId: string,
    private entryId: string,
    private oldType: StyleEntry["type"],
    private newType: StyleEntry["type"],
  ) {
    this.description = `切换样式类型 ${oldType} → ${newType}`;
  }

  execute(ctx: CommandContext): void {
    const el = ctx.getElement(this.elementId) as any;
    const entry = el?.styles?.find((s: StyleEntry) => s.id === this.entryId);
    if (entry) entry.type = this.newType;
  }

  undo(ctx: CommandContext): void {
    const el = ctx.getElement(this.elementId) as any;
    const entry = el?.styles?.find((s: StyleEntry) => s.id === this.entryId);
    if (entry) entry.type = this.oldType;
  }
}
```

Create `editor/client/src/commands/RenameStyleEntry.ts`:
```typescript
import type { Command, CommandContext } from "./types";
import type { StyleEntry } from "../constants/style-properties";

export class RenameStyleEntry implements Command {
  readonly type = "RenameStyleEntry";
  readonly description: string;
  constructor(
    private elementId: string,
    private entryId: string,
    private oldName: string,
    private newName: string,
  ) {
    this.description = `重命名样式 ${oldName} → ${newName}`;
  }

  execute(ctx: CommandContext): void {
    const el = ctx.getElement(this.elementId) as any;
    const entry = el?.styles?.find((s: StyleEntry) => s.id === this.entryId);
    if (entry) entry.name = this.newName;
  }

  undo(ctx: CommandContext): void {
    const el = ctx.getElement(this.elementId) as any;
    const entry = el?.styles?.find((s: StyleEntry) => s.id === this.entryId);
    if (entry) entry.name = this.oldName;
  }
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd D:/space/OmniLang/editor/client && npx vitest run src/commands/__tests__/style-commands.test.ts
```

Expected: All tests PASS.

- [ ] **Step 5: Run full test suite**

```bash
cd D:/space/OmniLang/editor/client && npx vitest run
```

Expected: No regressions.

- [ ] **Step 6: Commit**

```bash
git add editor/client/src/commands/AddStyleEntry.ts editor/client/src/commands/RemoveStyleEntry.ts editor/client/src/commands/ReorderStyleEntry.ts editor/client/src/commands/UpdateStyleProperty.ts editor/client/src/commands/SetStyleEntryType.ts editor/client/src/commands/RenameStyleEntry.ts editor/client/src/commands/__tests__/style-commands.test.ts
git commit -m "feat: add Style Commands (Add/Remove/Reorder/UpdateProperty/SetType/Rename)"
```

---

### Task 2.4: Save — Emit style blocks from StyleEntry[]

**Files:**
- Modify: `editor/client/src/omni-gen/pipeline.ts`
- Modify: `editor/client/src/omni-gen/source-emitter.ts`

**Interfaces:**
- Consumes: `ElementData.styles: StyleEntry[]`
- Produces: `.omni` source with `style { }` blocks and `style: { }` inline styles

**Context:** Currently `source-emitter.ts` emits styles from `comp.style` (a `Record<string, Record<string, string>>`). We need to change the pipeline to collect styles from `StyleEntry[]` instead of the old `resolveStyle → collectStyles` path.

- [ ] **Step 1: Update pipeline to build componentStyle from StyleEntry[]**

In `editor/client/src/omni-gen/pipeline.ts`, add a new function after `collectStyles`:

```typescript
function collectStylesFromEntries(
  elements: Array<{ id: string; styles?: StyleEntry[] }>,
): {
  componentStyle: Record<string, Record<string, string>>;
  elementClassMap: Map<string, string[]>;   // elementId → class names
  elementInlineMap: Map<string, Record<string, string>>; // elementId → inline styles
} {
  const componentStyle: Record<string, Record<string, string>> = {};
  const elementClassMap = new Map<string, string[]>();
  const elementInlineMap = new Map<string, Record<string, string>>();

  for (const el of elements) {
    if (!el.styles?.length) continue;
    const classNames: string[] = [];
    const inlineStyles: Record<string, string> = {};

    for (const entry of el.styles) {
      switch (entry.type) {
        case "class": {
          const rules: Record<string, string> = {};
          for (const prop of entry.properties) {
            rules[prop.name] = prop.value;
          }
          componentStyle[entry.name] = rules;
          classNames.push(entry.name);
          break;
        }
        case "variable": {
          const varName = entry.name; // "--primary-color"
          const className = variableToClassName(varName); // "_v-primary-color"
          componentStyle[className] = { [varName]: entry.properties[0]?.value ?? "" };
          classNames.push(className);
          break;
        }
        case "inline": {
          for (const prop of entry.properties) {
            inlineStyles[prop.name] = prop.value;
          }
          break;
        }
      }
    }

    if (classNames.length > 0) {
      elementClassMap.set(el.id, classNames);
    }
    if (Object.keys(inlineStyles).length > 0) {
      elementInlineMap.set(el.id, inlineStyles);
    }
  }

  return { componentStyle, elementClassMap, elementInlineMap };
}
```

Import `variableToClassName` from `../../constants/style-properties`.

- [ ] **Step 2: Update source-emitter to use the new data**

In `editor/client/src/omni-gen/source-emitter.ts`, add a helper to the `Writer` class or as a standalone for emitting style blocks from the componentStyle map (the existing `emitComponent` already handles the format — it iterates `comp.style`). Verify the existing code works:

```typescript
// Existing code (line 140-149) already works for Record<string, Record<string, string>>
// No changes needed to source-emitter.ts for style block emission
```

But we need to modify how `view { }` elements get their `class:` and `style:` attributes. In `emitViewNode`, update the class and inline style handling:

```typescript
// After existing class logic, add:
const classesFromStyles = elementClassMap.get(node.id);
if (classesFromStyles?.length) {
  const existingClass = classValue; // from existing logic
  const allClasses = existingClass
    ? `${existingClass} ${classesFromStyles.join(" ")}`
    : classesFromStyles.join(" ");
  // emit class: allClasses
}

const inlineFromStyles = elementInlineMap.get(node.id);
if (inlineFromStyles && Object.keys(inlineFromStyles).length > 0) {
  // emit style: { ...inlineFromStyles }
}
```

- [ ] **Step 3: Write integration test**

Add to `editor/client/src/omni-gen/__tests__/style-custom-property.test.ts`:

```typescript
import { collectStylesFromEntries } from "../pipeline";
import type { StyleEntry } from "../../constants/style-properties";

describe("collectStylesFromEntries", () => {
  it("generates componentStyle with class entries", () => {
    const styles: StyleEntry[] = [
      { id: "s1", type: "class", name: "card", properties: [{ name: "padding", value: "16px" }] },
    ];
    const elements = [{ id: "a", styles }];
    const result = collectStylesFromEntries(elements);
    expect(result.componentStyle["card"]).toEqual({ padding: "16px" });
    expect(result.elementClassMap.get("a")).toEqual(["card"]);
  });

  it("generates variable entries with _v- prefix class", () => {
    const styles: StyleEntry[] = [
      { id: "s1", type: "variable", name: "--primary-color", properties: [{ name: "value", value: "#333" }] },
    ];
    const elements = [{ id: "a", styles }];
    const result = collectStylesFromEntries(elements);
    expect(result.componentStyle["_v-primary-color"]).toEqual({ "--primary-color": "#333" });
    expect(result.elementClassMap.get("a")).toEqual(["_v-primary-color"]);
  });

  it("generates inline styles separately", () => {
    const styles: StyleEntry[] = [
      { id: "s1", type: "inline", name: "", properties: [{ name: "padding", value: "8px" }] },
    ];
    const elements = [{ id: "a", styles }];
    const result = collectStylesFromEntries(elements);
    expect(result.elementInlineMap.get("a")).toEqual({ padding: "8px" });
    expect(result.elementClassMap.get("a")).toBeUndefined();
  });
});
```

- [ ] **Step 4: Run integration tests**

```bash
cd D:/space/OmniLang/editor/client && npx vitest run src/omni-gen/__tests__/style-custom-property.test.ts -t "collectStylesFromEntries"
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add editor/client/src/omni-gen/pipeline.ts editor/client/src/omni-gen/source-emitter.ts editor/client/src/omni-gen/__tests__/style-custom-property.test.ts
git commit -m "feat: emit style blocks and inline styles from StyleEntry[] in .omni source"
```

---

### Task 2.5: Parse — Restore StyleEntry[] from .omni AST

**Files:**
- Modify: `editor/client/src/omni-gen/pipeline.ts`

**Interfaces:**
- Consumes: Parsed `.omni` AST (from WASM compiler)
- Produces: `StyleEntry[]` populated on each `ElementData`

- [ ] **Step 1: Add parse function**

In `editor/client/src/omni-gen/pipeline.ts`, add:

```typescript
import { classNameToVariable } from "../constants/style-properties";

/**
 * Parse style { } blocks from a component AST and return StyleEntry[].
 * Also extracts inline style: { } from view elements.
 */
function parseStylesFromAST(
  componentNode: any,    // AST node for the component
  viewElements: Map<string, ElementData>,
): void {
  // 1. Parse style { } blocks → class/variable StyleEntry map by class name
  const styleClassMap = new Map<string, StyleEntry>();
  const styleBlock = componentNode?.style; // StyleRule[] from AST
  if (styleBlock) {
    for (const rule of styleBlock) {
      const className: string = rule.class_name;
      const properties: StyleProperty[] = Object.entries(rule.properties).map(
        ([name, valueExpr]: [string, any]) => ({
          name,  // kebab-case, already from AST
          value: exprToString(valueExpr),
        })
      );

      const varName = classNameToVariable(className);
      if (varName) {
        // This is a variable entry (class name has _v- prefix)
        styleClassMap.set(className, {
          id: generateId(),
          type: "variable",
          name: varName, // "--primary-color"
          properties: properties.length > 0 ? properties : [{ name: "value", value: "" }],
        });
      } else {
        // Regular class entry
        styleClassMap.set(className, {
          id: generateId(),
          type: "class",
          name: className,
          properties,
        });
      }
    }
  }

  // 2. Walk view elements → map class: and style: to StyleEntry[]
  for (const [elId, el] of viewElements) {
    const styles: StyleEntry[] = [];
    const viewNode = findViewNode(componentNode?.view, elId);

    if (viewNode) {
      // Parse class: attribute → find matching StyleEntries
      const classAttr = viewNode.props?.["class"];
      if (classAttr && typeof classAttr === "string") {
        const classNames = classAttr.split(/\s+/);
        for (const cn of classNames) {
          const entry = styleClassMap.get(cn);
          if (entry) {
            styles.push({ ...entry, id: generateId() }); // clone with new id
          }
        }
      }

      // Parse style: attribute → inline StyleEntry
      const inlineStyle = viewNode.inline_style;
      if (inlineStyle && typeof inlineStyle === "object") {
        const properties: StyleProperty[] = Object.entries(inlineStyle).map(
          ([name, value]) => ({ name, value: String(value) })
        );
        if (properties.length > 0) {
          styles.push({
            id: generateId(),
            type: "inline",
            name: "",
            properties,
          });
        }
      }
    }

    // 3. Backward compatibility: no style { } block and no class:/style: attributes
    //    → extract style keys from el.props and create inline StyleEntry
    if (styles.length === 0 && !styleBlock) {
      const styleProps: StyleProperty[] = [];
      for (const key of Object.keys(el.props)) {
        if (STYLE_PROPERTY_NAMES.has(key)) {
          styleProps.push({ name: key, value: String(el.props[key]) });
        }
      }
      if (styleProps.length > 0) {
        styles.push({
          id: generateId(),
          type: "inline",
          name: "",
          properties: styleProps,
        });
      }
    }

    el.styles = styles;
  }
}

function exprToString(expr: any): string {
  if (!expr) return "";
  if (typeof expr === "string") return expr;
  if (typeof expr === "number") return String(expr);
  if (expr.value !== undefined) return String(expr.value);
  if (expr.literal !== undefined) return String(expr.literal);
  return String(expr);
}

let _idCounter = 0;
function generateId(): string {
  return `s_${Date.now()}_${++_idCounter}`;
}

function findViewNode(view: any, elId: string): any | null {
  if (!view) return null;
  if (view.id === elId) return view;
  for (const child of view.children || []) {
    const found = findViewNode(child, elId);
    if (found) return found;
  }
  return null;
}
```

- [ ] **Step 2: Write test**

Add to `editor/client/src/omni-gen/__tests__/style-custom-property.test.ts`:

```typescript
import { parseStylesFromAST } from "../pipeline";

describe("parseStylesFromAST", () => {
  it("restores class type from style { } block", () => {
    const componentAST = {
      style: [
        { class_name: "card", properties: { padding: { value: "16px" }, color: { value: "#333" } } },
      ],
    };
    const elements = new Map([["a", { id: "a", props: {}, styles: [] }]]);
    const viewNode = { id: "a", props: { class: "card" }, inline_style: null, children: [] };
    // Mock findViewNode — attach to componentAST
    componentAST.view = viewNode;

    parseStylesFromAST(componentAST, elements);
    const styles = elements.get("a")!.styles!;
    expect(styles).toHaveLength(1);
    expect(styles[0].type).toBe("class");
    expect(styles[0].name).toBe("card");
    expect(styles[0].properties).toHaveLength(2);
  });

  it("restores variable type from _v- prefixed class", () => {
    const componentAST = {
      style: [
        { class_name: "_v-primary-color", properties: { "--primary-color": { value: "#4F46E5" } } },
      ],
    };
    const elements = new Map([["a", { id: "a", props: {}, styles: [] }]]);
    componentAST.view = { id: "a", props: { class: "_v-primary-color" }, inline_style: null, children: [] };

    parseStylesFromAST(componentAST, elements);
    const styles = elements.get("a")!.styles!;
    expect(styles).toHaveLength(1);
    expect(styles[0].type).toBe("variable");
    expect(styles[0].name).toBe("--primary-color");
  });

  it("restores inline type from style: attribute", () => {
    const componentAST = { style: [] };
    const elements = new Map([["a", { id: "a", props: {}, styles: [] }]]);
    componentAST.view = { id: "a", props: {}, inline_style: { padding: "8px" }, children: [] };

    parseStylesFromAST(componentAST, elements);
    const styles = elements.get("a")!.styles!;
    expect(styles).toHaveLength(1);
    expect(styles[0].type).toBe("inline");
    expect(styles[0].properties[0]).toEqual({ name: "padding", value: "8px" });
  });

  it("backward compat: props style keys → inline StyleEntry when no style block", () => {
    const componentAST = { style: [] };
    const elements = new Map([["a", { id: "a", props: { padding: "16px", label: "Submit", background_color: "#eee" }, styles: [] }]]);
    componentAST.view = { id: "a", props: {}, inline_style: null, children: [] };

    parseStylesFromAST(componentAST, elements);
    const styles = elements.get("a")!.styles!;
    expect(styles[0].type).toBe("inline");
    // Only style keys should be extracted, not "label"
    const propNames = styles[0].properties.map(p => p.name);
    expect(propNames).toContain("padding");
    expect(propNames).toContain("background_color");
    expect(propNames).not.toContain("label");
  });
});
```

- [ ] **Step 3: Run parse tests**

```bash
cd D:/space/OmniLang/editor/client && npx vitest run src/omni-gen/__tests__/style-custom-property.test.ts -t "parseStylesFromAST"
```

Expected: PASS.

- [ ] **Step 4: Run full test suite**

```bash
cd D:/space/OmniLang/editor/client && npx vitest run
```

Expected: No regressions.

- [ ] **Step 5: Commit**

```bash
git add editor/client/src/omni-gen/pipeline.ts editor/client/src/omni-gen/__tests__/style-custom-property.test.ts
git commit -m "feat: parse StyleEntry[] from .omni AST (style blocks, class:, style:, backward compat)"
```

---

## Phase 3: UI Panel

### Task 3.1: StylePropertyEditor — Smart property row component

**Files:**
- Create: `editor/client/src/panels/StylePropertyEditor.tsx`

**Interfaces:**
- Props: `{ property: StyleProperty; onChange: (prop: StyleProperty) => void; onDelete: () => void }`
- Produces: A single property editor row with smart controls

- [ ] **Step 1: Implement StylePropertyEditor**

```tsx
// editor/client/src/panels/StylePropertyEditor.tsx
import React from "react";
import type { StyleProperty } from "../constants/style-properties";
import {
  PROPERTY_UNIT_MAP,
  DROPDOWN_PROPERTIES,
  isColorProperty,
  isCustomProperty,
  getUnitGroup,
  getDropdownOptions,
} from "../constants/style-properties";

interface Props {
  property: StyleProperty;
  onChange: (prop: StyleProperty) => void;
  onDelete: () => void;
}

// Categorized property name list for the dropdown
const PROPERTY_CATEGORIES: { label: string; props: string[] }[] = [
  {
    label: "布局", props: [
      "width", "height", "min-width", "max-width", "min-height", "max-height",
      "margin", "margin-top", "margin-right", "margin-bottom", "margin-left",
      "padding", "padding-top", "padding-right", "padding-bottom", "padding-left",
      "display", "gap", "overflow", "overflow-x", "overflow-y",
    ],
  },
  {
    label: "排版", props: [
      "font-size", "font-weight", "font-family", "color",
      "text-align", "line-height", "letter-spacing",
      "text-decoration", "text-transform", "white-space", "word-break",
    ],
  },
  {
    label: "背景 & 边框", props: [
      "background-color", "background-image", "border", "border-color",
      "border-radius", "box-shadow", "opacity",
    ],
  },
  {
    label: "定位", props: [
      "position", "top", "right", "bottom", "left", "z-index",
    ],
  },
  {
    label: "Flex & Grid", props: [
      "flex-direction", "flex-wrap", "flex-grow", "flex-shrink", "flex-basis",
      "justify-content", "align-items", "align-self",
      "grid-template-columns", "grid-template-rows", "grid-column", "grid-row",
    ],
  },
  {
    label: "变换 & 过渡", props: [
      "transform", "transition", "transition-duration", "transition-property",
      "transition-timing-function", "animation", "animation-name",
      "animation-duration", "animation-iteration-count",
      "animation-direction", "animation-fill-mode", "animation-timing-function",
      "cursor",
    ],
  },
];

const ALL_PROPERTY_NAMES = PROPERTY_CATEGORIES.flatMap((c) => c.props);

function parseValueAndUnit(raw: string): { num: string; unit: string } {
  const match = raw.match(/^(-?[\d.]+)\s*(.*)$/);
  if (match) return { num: match[1], unit: match[2] || "" };
  return { num: raw, unit: "" };
}

const rowStyle: React.CSSProperties = {
  display: "flex", gap: 6, alignItems: "center", marginBottom: 4,
};
const inputStyle: React.CSSProperties = {
  background: "#1e1e2e", color: "#cdd6f4", border: "1px solid #45475a",
  borderRadius: 4, padding: "4px 8px", fontSize: 12, flex: 1,
};
const selectStyle: React.CSSProperties = { ...inputStyle, width: 110, flex: "none" };
const deleteBtnStyle: React.CSSProperties = {
  background: "none", border: "none", color: "#f38ba8", cursor: "pointer",
  fontSize: 14, padding: "0 4px",
};

export const StylePropertyEditor: React.FC<Props> = ({ property, onChange, onDelete }) => {
  const unitGroup = getUnitGroup(property.name);
  const dropdownOpts = getDropdownOptions(property.name);
  const color = isColorProperty(property.name);
  const isCustom = isCustomProperty(property.name);

  // For color properties
  if (color) {
    return (
      <div style={rowStyle}>
        <select
          style={selectStyle}
          value={property.name}
          onChange={(e) => onChange({ ...property, name: e.target.value })}
        >
          {ALL_PROPERTY_NAMES.map((n) => (
            <option key={n} value={n}>{n}</option>
          ))}
        </select>
        <input
          type="color"
          value={property.value.startsWith("#") ? property.value : "#000000"}
          onChange={(e) => onChange({ ...property, value: e.target.value })}
          style={{ width: 28, height: 24, border: "none", cursor: "pointer", flex: "none" }}
        />
        <input
          type="text"
          value={property.value}
          onChange={(e) => onChange({ ...property, value: e.target.value })}
          style={inputStyle}
        />
        <button style={deleteBtnStyle} onClick={onDelete}>×</button>
      </div>
    );
  }

  // For dropdown properties
  if (dropdownOpts !== null) {
    if (dropdownOpts === null) {
      // Free text (null in DROPDOWN_PROPERTIES)
      return (
        <div style={rowStyle}>
          <select
            style={selectStyle}
            value={property.name}
            onChange={(e) => onChange({ ...property, name: e.target.value })}
          >
            {ALL_PROPERTY_NAMES.map((n) => (
              <option key={n} value={n}>{n}</option>
            ))}
            {isCustom && <option value={property.name}>{property.name}</option>}
          </select>
          <input
            type="text"
            value={property.value}
            onChange={(e) => onChange({ ...property, value: e.target.value })}
            style={inputStyle}
          />
          <button style={deleteBtnStyle} onClick={onDelete}>×</button>
        </div>
      );
    }
    // Has dropdown options
    return (
      <div style={rowStyle}>
        <select
          style={selectStyle}
          value={property.name}
          onChange={(e) => onChange({ ...property, name: e.target.value })}
        >
          {ALL_PROPERTY_NAMES.map((n) => (
            <option key={n} value={n}>{n}</option>
          ))}
        </select>
        <select
          style={inputStyle}
          value={property.value}
          onChange={(e) => onChange({ ...property, value: e.target.value })}
        >
          {dropdownOpts.map((opt) => (
            <option key={opt} value={opt}>{opt}</option>
          ))}
        </select>
        <button style={deleteBtnStyle} onClick={onDelete}>×</button>
      </div>
    );
  }

  // For opacity (numeric 0-1)
  if (property.name === "opacity") {
    return (
      <div style={rowStyle}>
        <select
          style={selectStyle}
          value={property.name}
          onChange={(e) => onChange({ ...property, name: e.target.value })}
        >
          {ALL_PROPERTY_NAMES.map((n) => (
            <option key={n} value={n}>{n}</option>
          ))}
        </select>
        <input
          type="range"
          min="0" max="1" step="0.01"
          value={parseFloat(property.value) || 1}
          onChange={(e) => onChange({ ...property, value: e.target.value })}
          style={{ flex: 1 }}
        />
        <input
          type="number"
          min="0" max="1" step="0.01"
          value={property.value}
          onChange={(e) => onChange({ ...property, value: e.target.value })}
          style={{ ...inputStyle, width: 56, flex: "none" }}
        />
        <button style={deleteBtnStyle} onClick={onDelete}>×</button>
      </div>
    );
  }

  // For numeric + unit properties
  if (unitGroup) {
    const { num, unit } = parseValueAndUnit(property.value);
    const units = [...unitGroup.common, ...(unitGroup.advanced || [])];

    return (
      <div style={rowStyle}>
        <select
          style={selectStyle}
          value={property.name}
          onChange={(e) => onChange({ ...property, name: e.target.value })}
        >
          {ALL_PROPERTY_NAMES.map((n) => (
            <option key={n} value={n}>{n}</option>
          ))}
          {isCustom && <option value={property.name}>{property.name}</option>}
        </select>
        <input
          type="text"
          value={num}
          onChange={(e) => {
            const newVal = e.target.value + (unit ? unit : "");
            onChange({ ...property, value: newVal });
          }}
          style={{ ...inputStyle, width: 60, flex: "none" }}
          placeholder={unitGroup.default}
        />
        <select
          value={unit}
          onChange={(e) => {
            const newUnit = e.target.value;
            const newVal = newUnit === "" ? num : num + newUnit;
            onChange({ ...property, value: newVal });
          }}
          style={{ ...selectStyle, width: 64 }}
        >
          {units.map((u) => (
            <option key={u} value={u}>{u || "(无)"}</option>
          ))}
        </select>
        <button style={deleteBtnStyle} onClick={onDelete}>×</button>
      </div>
    );
  }

  // Fallback: generic text input
  return (
    <div style={rowStyle}>
      <select
        style={selectStyle}
        value={property.name}
        onChange={(e) => onChange({ ...property, name: e.target.value })}
      >
        {ALL_PROPERTY_NAMES.map((n) => (
          <option key={n} value={n}>{n}</option>
        ))}
        {isCustom && <option value={property.name}>{property.name}</option>}
      </select>
      <input
        type="text"
        value={property.value}
        onChange={(e) => onChange({ ...property, value: e.target.value })}
        style={inputStyle}
        placeholder="e.g. 16px"
      />
      <button style={deleteBtnStyle} onClick={onDelete}>×</button>
    </div>
  );
};
```

- [ ] **Step 2: Verify TypeScript compiles**

```bash
cd D:/space/OmniLang/editor/client && npx tsc --noEmit src/panels/StylePropertyEditor.tsx
```

Expected: No type errors.

- [ ] **Step 3: Commit**

```bash
git add editor/client/src/panels/StylePropertyEditor.tsx
git commit -m "feat: add StylePropertyEditor component with smart controls"
```

---

### Task 3.2: StyleEntryEditor — Single entry editor (3 modes)

**Files:**
- Create: `editor/client/src/panels/StyleEntryEditor.tsx`

**Interfaces:**
- Props: `{ entry: StyleEntry; elementId: string; onChange: (patch: Partial<StyleEntry>) => void; onClose: () => void }`
- Produces: Expanded editor for one StyleEntry (class/variable/inline modes)

- [ ] **Step 1: Implement StyleEntryEditor**

```tsx
// editor/client/src/panels/StyleEntryEditor.tsx
import React from "react";
import type { StyleEntry, StyleProperty } from "../constants/style-properties";
import { StylePropertyEditor } from "./StylePropertyEditor";

interface Props {
  entry: StyleEntry;
  elementId: string;
  onChange: (patch: Partial<StyleEntry>) => void;
  onClose: () => void;
}

const containerStyle: React.CSSProperties = {
  background: "#181825", border: "1px solid #313244", borderRadius: 6,
  padding: 8, marginBottom: 6,
};
const typeBtnStyle = (active: boolean): React.CSSProperties => ({
  background: active ? "#45475a" : "transparent",
  border: "1px solid #585b70", borderRadius: 4,
  color: "#cdd6f4", cursor: "pointer", fontSize: 11,
  padding: "2px 8px", marginRight: 4,
});
const inputStyle: React.CSSProperties = {
  background: "#1e1e2e", color: "#cdd6f4", border: "1px solid #45475a",
  borderRadius: 4, padding: "4px 8px", fontSize: 12, width: "100%",
  marginTop: 4, marginBottom: 8,
};
const addBtnStyle: React.CSSProperties = {
  background: "none", border: "1px dashed #585b70", borderRadius: 4,
  color: "#a6adc8", cursor: "pointer", fontSize: 11,
  padding: "4px 8px", width: "100%", marginTop: 4,
};

export const StyleEntryEditor: React.FC<Props> = ({ entry, onChange, onClose }) => {
  const isVariable = entry.type === "variable";

  const handleTypeChange = (newType: StyleEntry["type"]) => {
    onChange({ type: newType });
  };

  const handlePropertyChange = (index: number, prop: StyleProperty) => {
    const newProps = [...entry.properties];
    newProps[index] = prop;
    onChange({ properties: newProps });
  };

  const handlePropertyDelete = (index: number) => {
    const newProps = entry.properties.filter((_, i) => i !== index);
    onChange({ properties: newProps });
  };

  const handleAddProperty = () => {
    onChange({
      properties: [...entry.properties, { name: "padding", value: "" }],
    });
  };

  return (
    <div style={containerStyle}>
      {/* Type toggle */}
      <div style={{ display: "flex", marginBottom: 8 }}>
        <button style={typeBtnStyle(entry.type === "class")} onClick={() => handleTypeChange("class")}>
          class
        </button>
        <button style={typeBtnStyle(entry.type === "inline")} onClick={() => handleTypeChange("inline")}>
          inline
        </button>
        <button style={typeBtnStyle(entry.type === "variable")} onClick={() => handleTypeChange("variable")}>
          variable
        </button>
      </div>

      {/* Name field (class and variable) */}
      {entry.type !== "inline" && (
        <input
          type="text"
          value={isVariable ? entry.name : entry.name}
          onChange={(e) => onChange({ name: e.target.value })}
          style={inputStyle}
          placeholder={isVariable ? "--variable-name" : "class-name"}
        />
      )}

      {/* Properties */}
      {isVariable ? (
        /* Variable mode: single value */
        entry.properties.length > 0 && (
          <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
            <span style={{ fontSize: 11, color: "#a6adc8", whiteSpace: "nowrap" }}>
              {entry.name || "--var"}:
            </span>
            <input
              type="text"
              value={entry.properties[0]?.value ?? ""}
              onChange={(e) =>
                onChange({ properties: [{ name: "value", value: e.target.value }] })
              }
              style={{ ...inputStyle, marginTop: 0, marginBottom: 0 }}
              placeholder="#333"
            />
          </div>
        )
      ) : (
        /* Class/inline mode: property list */
        <>
          {entry.properties.map((prop, i) => (
            <StylePropertyEditor
              key={i}
              property={prop}
              onChange={(p) => handlePropertyChange(i, p)}
              onDelete={() => handlePropertyDelete(i)}
            />
          ))}
          <button style={addBtnStyle} onClick={handleAddProperty}>
            + 添加属性
          </button>
        </>
      )}
    </div>
  );
};
```

- [ ] **Step 2: Verify TypeScript compiles**

```bash
cd D:/space/OmniLang/editor/client && npx tsc --noEmit src/panels/StyleEntryEditor.tsx
```

Expected: No type errors.

- [ ] **Step 3: Commit**

```bash
git add editor/client/src/panels/StyleEntryEditor.tsx
git commit -m "feat: add StyleEntryEditor component (class/variable/inline modes)"
```

---

### Task 3.3: PropertyPanel — Style list integration

**Files:**
- Modify: `editor/client/src/panels/PropertyPanel.tsx`

**Interfaces:**
- Consumes: `StyleEntryEditor`, `StyleEntry`, new Command classes, store methods from Task 2.2
- Produces: Style list section in PropertyPanel with accordion expand/collapse, add/delete/reorder

- [ ] **Step 1: Replace old StyleEditor section**

In `editor/client/src/panels/PropertyPanel.tsx`, locate the section (~line 480-487):

```tsx
{/* Style Properties */}
{meta.styleProperties.length > 0 && (
  <>
    <div style={sectionHeaderStyle}>样式</div>
    {meta.styleProperties.map((prop) => (
      <StyleEditor key={prop} name={prop} value={el.props[prop]} onChange={(v) => handleChange(prop, v)} />
    ))}
  </>
)}
```

Replace with:

```tsx
{/* Style List */}
<div style={sectionHeaderStyle}>
  样式
  <button
    onClick={() => {
      const entry: StyleEntry = {
        id: `s_${Date.now()}`,
        type: "inline",
        name: "",
        properties: [],
      };
      const cmd = new AddStyleEntry(el.id, entry);
      useUndoStore.getState().execute(cmd, ctx);
      syncToEditorStore();
      setExpandedStyleId(entry.id);
      debouncedRecompile();
    }}
    style={{
      background: "none", border: "1px solid #585b70", borderRadius: 4,
      color: "#a6adc8", cursor: "pointer", fontSize: 11,
      padding: "2px 8px", marginLeft: "auto",
    }}
  >
    +添加
  </button>
</div>

{/* Render each style entry */}
{(el.styles || []).map((entry: StyleEntry, index: number) => {
  const isExpanded = expandedStyleId === entry.id;
  const displayName = entry.type === "inline"
    ? `inline (${entry.properties.map(p => p.name).join(", ") || "empty"})`
    : entry.type === "variable"
      ? entry.name || "--unnamed"
      : `.${entry.name}`;

  return (
    <div key={entry.id}>
      {/* Collapsed row */}
      <div
        onClick={() => setExpandedStyleId(isExpanded ? null : entry.id)}
        style={{
          display: "flex", alignItems: "center", cursor: "pointer",
          padding: "4px 0", color: "#cdd6f4", fontSize: 12,
          userSelect: "none",
        }}
      >
        <span style={{ marginRight: 6 }}>{isExpanded ? "▼" : "▶"}</span>
        <span style={{ flex: 1 }}>{displayName}</span>
        <button
          onClick={(e) => {
            e.stopPropagation();
            const cmd = new RemoveStyleEntry(el.id, entry, index);
            useUndoStore.getState().execute(cmd, ctx);
            syncToEditorStore();
            if (isExpanded) setExpandedStyleId(null);
            debouncedRecompile();
          }}
          style={{
            background: "none", border: "none", color: "#f38ba8",
            cursor: "pointer", fontSize: 14, padding: "0 4px",
          }}
        >
          ×
        </button>
      </div>

      {/* Expanded editor */}
      {isExpanded && (
        <StyleEntryEditor
          entry={entry}
          elementId={el.id}
          onChange={(patch) => {
            // Use UpdateStyleProperty for property-level changes,
            // RenameStyleEntry for name changes, SetStyleEntryType for type changes
            const merged = { ...entry, ...patch };
            const el2 = useEditorStore.getState().elements.find(e => e.id === el.id);
            if (el2) {
              useEditorStore.getState().updateStyleEntry(el.id, entry.id, patch);
            }
            debouncedRecompile();
          }}
          onClose={() => setExpandedStyleId(null)}
        />
      )}
    </div>
  );
})}
```

Add state for expanded style:
```tsx
const [expandedStyleId, setExpandedStyleId] = useState<string | null>(null);
```

Add imports at top:
```tsx
import { StyleEntryEditor } from "./StyleEntryEditor";
import { AddStyleEntry, RemoveStyleEntry } from "../commands";
import type { StyleEntry } from "../constants/style-properties";
```

- [ ] **Step 2: Also import ctx creation**

Ensure `ctx` is available in the style section scope. The existing `handleChange` callback already creates a `CommandContext` — reuse the same pattern:

```tsx
import { createCommandContext } from "../commands/types"; // or wherever it's defined
```

- [ ] **Step 3: Remove old StyleEditor component**

Delete the `StyleEditor` function component (~lines 549-569). Also remove the old `meta.styleProperties` rendering block.

- [ ] **Step 4: Verify TypeScript compiles**

```bash
cd D:/space/OmniLang/editor/client && npx tsc --noEmit
```

Expected: No type errors.

- [ ] **Step 5: Run full test suite**

```bash
cd D:/space/OmniLang/editor/client && npx vitest run
```

Expected: All tests PASS.

- [ ] **Step 6: Commit**

```bash
git add editor/client/src/panels/PropertyPanel.tsx
git commit -m "feat: integrate StyleEntry list into PropertyPanel (accordion, add/delete)"
```

---

## Verification Checklist

After all phases complete, verify end-to-end:

- [ ] **E2E.1**: Open editor → select a Button → click `+添加` in style section → StyleEntry appears expanded
- [ ] **E2.2**: Add property `padding: 16px` → canvas preview reflects the change in real-time
- [ ] **E2.3**: Add variable `--primary-color: #4F46E5` → reference it via `var(--primary-color)` in a class → canvas preview correct
- [ ] **E2.4**: Save project → inspect `.omni` file → `style { ._v-primary-color { --primary-color: #4F46E5; } }` present → element has `class: _v-primary-color`
- [ ] **E2.5**: Reload project → style list restored correctly (class names, variable names, property values match)
- [ ] **E2.6**: Undo a style change (Ctrl+Z) → property reverts → redo (Ctrl+Y) → property restores
- [ ] **E2.7**: Open old `.omni` file (no `style { }` block) → style properties extracted to inline StyleEntry → no errors
- [ ] **E2.8**: Run `npx vitest run` — all tests pass
- [ ] **E2.9**: Run `cd omni-compiler && cargo test` — all Rust tests pass

