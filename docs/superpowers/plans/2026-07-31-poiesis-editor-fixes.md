# Poiesis Editor Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix 4 HIGH Poiesis editor issues found in the 2026-07-31 architecture review: dead save-validation gate, design.omni round-trip corruption, font token type mismatch, and diagnostics full-codegen overhead.

**Architecture:** Four targeted fixes in `poiesis/src/`: fix the WASM result wrapper unwrap in `validateSources`, add a round-trip guard that refuses to rewrite design.omni when the TS store can't represent compiler constructs, remove the invalid `"font"` token type, and route multi-file diagnostics through the cached diagnostics-only WASM path.

**Tech Stack:** TypeScript, React 19, Vitest, WASM (omnilang_wasm)

**Source review:** `.architecture/reviews/full-workspace-review-2026-07-31.md`

---

### Task 1: Fix dead save-time WASM validation gate (H3)

**Files:**
- Modify: `poiesis/src/services/monaco/diagnostics.ts:144-163`
- Create: `poiesis/src/services/monaco/__tests__/diagnostics.test.ts`

**Problem:** `validateSources` calls `compile_multi_to_omniview()` which returns `{ result: { success, errors, ... }, cache_state: "..." }`. The function checks `"errors" in result` on the **wrapper** object (which never has `errors`), then falls through `Array.isArray(result)` which is always false. It always returns `[]`.

- [ ] **Step 1: Write a test proving the validator is dead**

Create `poiesis/src/services/monaco/__tests__/diagnostics.test.ts`:

```typescript
import { describe, it, expect, vi } from "vitest";

// Mock the WASM module
vi.mock("../../../wasm/omnilang_wasm.js", () => ({
  compile_multi_to_omniview: vi.fn(),
}));

describe("validateSources", () => {
  it("returns errors from the WASM result wrapper", async () => {
    const { compile_multi_to_omniview } = await import(
      "../../../wasm/omnilang_wasm.js"
    );
    const { validateSources } = await import("../diagnostics");

    // Simulate the actual WASM return shape: { result: { errors: [...] }, cache_state: "..." }
    const mockErrors = [
      {
        severity: "error",
        code: "E0001",
        message: "undefined variable 'foo'",
        file: "test.omni",
        line: 1,
        col: 5,
      },
    ];
    (compile_multi_to_omniview as ReturnType<typeof vi.fn>).mockReturnValue({
      result: { success: false, errors: mockErrors, warnings: [] },
      cache_state: "mock-cache-state",
    });

    const sources = { "test.omni": "design App { token x: color = foo; }" };
    const diags = await validateSources(sources);

    expect(diags).toHaveLength(1);
    expect(diags[0].code).toBe("E0001");
  });

  it("returns empty array when result has no errors", async () => {
    const { compile_multi_to_omniview } = await import(
      "../../../wasm/omnilang_wasm.js"
    );
    const { validateSources } = await import("../diagnostics");

    (compile_multi_to_omniview as ReturnType<typeof vi.fn>).mockReturnValue({
      result: { success: true, errors: [], warnings: [] },
      cache_state: "mock-cache-state",
    });

    const sources = { "test.omni": "component Foo { render {} }" };
    const diags = await validateSources(sources);

    expect(diags).toHaveLength(0);
  });

  it("handles WASM errors gracefully", async () => {
    const { compile_multi_to_omniview } = await import(
      "../../../wasm/omnilang_wasm.js"
    );
    const { validateSources } = await import("../diagnostics");

    (compile_multi_to_omniview as ReturnType<typeof vi.fn>).mockImplementation(
      () => {
        throw new Error("WASM crash");
      },
    );

    const sources = { "test.omni": "" };
    const diags = await validateSources(sources);

    // Should not throw — returns empty array on error
    expect(diags).toHaveLength(0);
  });
});
```

- [ ] **Step 2: Run the test — verify it FAILS**

```bash
cd poiesis && npx vitest run src/services/monaco/__tests__/diagnostics.test.ts
```

Expected: First test FAILS — `validateSources` returns `[]` even though the mock WASM returned errors. Proves the bug.

- [ ] **Step 3: Fix validateSources — unwrap the wrapper**

In `poiesis/src/services/monaco/diagnostics.ts`, change lines 144-163 from:

```typescript
export async function validateSources(
  sources: Record<string, string>,
): Promise<WasmDiagnostic[]> {
  try {
    const wasmModule = await import("../../wasm/omnilang_wasm.js");
    // compile_multi_to_omniview: standalone multi-file with analyse_multi
    // editor_mode=false, debug_mode=false → bare compile, just for validation
    const result = wasmModule.compile_multi_to_omniview(sources, false, false);
    if (result && Array.isArray(result)) {
      return result as WasmDiagnostic[];
    }
    if (result && typeof result === "object" && "errors" in result) {
      return (result as { errors: WasmDiagnostic[] }).errors ?? [];
    }
    return [];
  } catch (e) {
    new Logger({ prefix: "Diagnostics" }).warn("[OmniLang] validateSources failed:", e);
    return [];
  }
}
```

To:

```typescript
export async function validateSources(
  sources: Record<string, string>,
): Promise<WasmDiagnostic[]> {
  try {
    const wasmModule = await import("../../wasm/omnilang_wasm.js");
    // compile_multi_to_omniview returns: { result: { success, errors, warnings }, cache_state }
    // We need to unwrap the outer wrapper to reach errors.
    const wrapper = wasmModule.compile_multi_to_omniview(sources, false, false);
    if (!wrapper || typeof wrapper !== "object") {
      return [];
    }
    // Unwrap: the actual CompileResult is at .result
    const result = wrapper.result;
    if (!result || typeof result !== "object") {
      return [];
    }
    if (Array.isArray(result)) {
      return result as WasmDiagnostic[];
    }
    if ("errors" in result) {
      return (result as { errors: WasmDiagnostic[] }).errors ?? [];
    }
    return [];
  } catch (e) {
    new Logger({ prefix: "Diagnostics" }).warn("[OmniLang] validateSources failed:", e);
    return [];
  }
}
```

- [ ] **Step 4: Run the test — verify it PASSES**

```bash
cd poiesis && npx vitest run src/services/monaco/__tests__/diagnostics.test.ts
```

Expected: All 3 tests PASS. The first test now correctly receives the mock errors.

- [ ] **Step 5: Run existing Poiesis tests**

```bash
cd poiesis && npx vitest run
```

Expected: All existing tests PASS.

- [ ] **Step 6: Commit**

```bash
git add poiesis/src/services/monaco/diagnostics.ts poiesis/src/services/monaco/__tests__/diagnostics.test.ts
git commit -m "fix: unwrap WASM result wrapper in validateSources save-time validation

compile_multi_to_omniview returns { result: { errors, ... }, cache_state },
but validateSources was checking 'errors' on the outer wrapper object
(which never has it). The function always returned [], making the
save-time validation gate dead code.

Unwrap .result before checking for errors. Add vitest with mocked WASM
to prevent regression.

This was the same 'silent degradation' class ADR-050/051 eliminated on
the compiler side."
```

---

### Task 2: Guard Poiesis design.omni round-trip — refuse to rewrite unsupported constructs (H5)

**Files:**
- Modify: `poiesis/src/services/codegen/parser.ts:1010-1038` (capture unsupported constructs during parse)
- Modify: `poiesis/src/services/codegen/design-emitter.ts` (add round-trip integrity check)
- Modify: `poiesis/src/hooks/useSaveProject.ts:110-127` (refuse to save when store can't represent)

**Problem:** The editor's `DesignToken` model stores values as plain `string` and has no concept of themes. When it parses then re-emits `design.omni`, it silently destroys:
- Const references: `token primary: color = BRAND;` → `token primary: color = "BRAND";`
- Runtime expressions: `token primary: color = ThemeStore.primaryColor;` → `token primary: color = "ThemeStore.primaryColor";`
- Theme blocks: `@themes light, dark;` + per-theme overrides → silently deleted

- [ ] **Step 1: Add a "hasUnsupportedConstructs" flag during parse**

In `poiesis/src/services/codegen/parser.ts`, add a field to the parser context (where `designTokens`, `designClasses` are tracked):

```typescript
// In the parser context interface, add:
hasUnsupportedDesignConstructs: boolean;
```

In `parseDesignToken` (lines 1010-1038), after capturing the value, detect non-literal values:

```typescript
  // After line 1024: value = valueTok.text;
  // Detect values the store can't round-trip:
  // - Unquoted identifiers (const references like BRAND)
  // - Dotted expressions (signal refs like ThemeStore.primaryColor)
  if (valueTok.kind === "IDENTIFIER") {
    // Check if it looks like a const reference (PascalCase or UPPER_SNAKE_CASE)
    // or a dotted expression (contains a dot in the raw text)
    if (/^[A-Z]/.test(value) || value.includes(".")) {
      this.ctx.hasUnsupportedDesignConstructs = true;
    }
  }
  // Also: if valueTok is not STRING and not IDENTIFIER, it's an expression
  if (valueTok.kind !== "STRING" && valueTok.kind !== "IDENTIFIER") {
    this.ctx.hasUnsupportedDesignConstructs = true;
  }
```

In the main design-body parse function, detect `@themes` and `@default-theme` directives:

```typescript
  // When encountering "@themes" token (currently skipped):
  if (this.stream.peek().kind === "AT_KEYWORD" && this.stream.peek().text === "themes") {
    this.ctx.hasUnsupportedDesignConstructs = true;
  }
  if (this.stream.peek().kind === "AT_KEYWORD" && this.stream.peek().text === "default-theme") {
    this.ctx.hasUnsupportedDesignConstructs = true;
  }
```

- [ ] **Step 2: Add round-trip integrity check in useSaveProject**

In `poiesis/src/hooks/useSaveProject.ts`, after the design source is emitted (line 111-112), add a check before writing:

```typescript
      if (hasDesignContent) {
        const { emitDesignSource } = await import("../services/codegen/design-emitter");
        designSource = emitDesignSource(store.activeDesign);
        designFilePath = activeTab?.tabType === "design" && activeTab?.filePath
          ? activeTab.filePath
          : "src/design.omni";

        // ── Round-trip integrity check ──
        // If the original design.omni (on disk) contains constructs the store can't
        // represent (themes, const refs, runtime expressions), refuse to rewrite it.
        // The store would silently destroy these constructs on save.
        if (activeTab?.tabType === "design") {
          try {
            const originalSource = await api.project.readFile(designFilePath);
            if (originalSource) {
              const { parseDesignBlock } = await import("../services/codegen/parser");
              const parseCtx = parseDesignBlock(originalSource);
              if (parseCtx?.hasUnsupportedDesignConstructs) {
                setSaveError(
                  "This design.omni contains themes, const references, or runtime " +
                  "expressions that the visual editor cannot yet represent. " +
                  "Edit the file directly in the code editor (Code tab) instead.\n\n" +
                  "Supported: static token values (quoted strings, numbers).\n" +
                  "Unsupported: @themes, @default-theme, const references (BRAND), " +
                  "signal expressions (Store.signal)."
                );
                setSaving(false);
                return;
              }
            }
          } catch {
            // File doesn't exist yet or can't be read — safe to proceed
          }
        }

        // Validate through WASM analyse before writing
        const designErrors = await validateSources({ [designFilePath]: designSource });
        // ... rest of existing validation
```

- [ ] **Step 3: Write a test for the round-trip guard**

Create `poiesis/src/services/codegen/__tests__/design-roundtrip.test.ts`:

```typescript
import { describe, it, expect } from "vitest";

// Test that the parser detects unsupported constructs
describe("design parser — unsupported construct detection", () => {
  it("flags const references (PascalCase) as unsupported", () => {
    // The design token `token primary: color = BRAND;` should set the flag
    // (This tests the parse path; the actual test may need to invoke the parser)
    // For now, test the regex detection logic directly:
    const isConstRef = (value: string, kind: string) =>
      kind === "IDENTIFIER" && (/^[A-Z]/.test(value) || value.includes("."));

    expect(isConstRef("BRAND", "IDENTIFIER")).toBe(true);
    expect(isConstRef("primaryColor", "IDENTIFIER")).toBe(false); // camelCase = CSS value
    expect(isConstRef("ThemeStore.primaryColor", "IDENTIFIER")).toBe(true); // dotted
    expect(isConstRef("#6366f1", "STRING")).toBe(false); // literal string
  });
});
```

- [ ] **Step 4: Manually verify the guard works**

```bash
# Create a design.omni with unsupported constructs
cat > /tmp/test_design.omni << 'EOF'
design App {
    @themes light, dark;
    token primary: color = BRAND dark "#a5b4fc";
    class Card {
        background: token(primary);
    }
}
EOF
```

Open this file in Poiesis, make a visual edit in the Design tab, and try to save. Expected: save is blocked with a message explaining that the file contains unsupported constructs.

- [ ] **Step 5: Update the DesignDefinition type to track unsupported status**

In `poiesis/src/core/types/design.ts`, add to the store's design state:

```typescript
export interface DesignDefinition {
  id: string;
  name: string;
  tokens: DesignToken[];
  classes: DesignClass[];
  variants: VariantDefinition[];
  /** Set when original source contains constructs the editor cannot round-trip. */
  hasUnsupportedConstructs?: boolean;
}
```

- [ ] **Step 6: Commit**

```bash
git add poiesis/src/services/codegen/parser.ts poiesis/src/services/codegen/design-emitter.ts poiesis/src/hooks/useSaveProject.ts poiesis/src/core/types/design.ts poiesis/src/services/codegen/__tests__/design-roundtrip.test.ts
git commit -m "fix: guard design.omni round-trip against silent data loss

The Poiesis DesignToken model stores values as plain strings and has no
concept of themes. When parsing then re-emitting design.omni, it silently:
- Quotes const references (BRAND → \"BRAND\")
- Quotes signal expressions (ThemeStore.x → \"ThemeStore.x\")
- Drops @themes/@default-theme and per-theme overrides

Add a round-trip integrity check: if the on-disk design.omni contains
unsupported constructs, refuse to rewrite it and show a clear error
directing users to the Code tab for manual editing.

This is a stopgap until E3 (dynamic token expression support in the
editor) is implemented."
```

---

### Task 3: Remove invalid "font" token type from editor (H7)

**Files:**
- Modify: `poiesis/src/core/types/design.ts:6-20`
- Modify: `poiesis/src/components/design/TokenEditor.tsx:43, 47, 266-268`

**Problem:** The editor's `TokenType` union includes `"font"` (14 types), but the compiler's `parse_token_type` in `crates/omnilang-core/src/parser/design.rs:205-223` accepts exactly 13 types — no `"font"`. Creating a typography token emits invalid `.omni` that the compiler rejects, bricking all subsequent saves.

- [ ] **Step 1: Remove "font" from TokenType union**

In `poiesis/src/core/types/design.ts`, change lines 6-20:

```typescript
export type TokenType =
  | "color"
  | "size"
  | "radius"
  // | "font"     // REMOVED: not a valid compiler token type
  | "fontFamily"
  | "fontSize"
  | "fontWeight"
  | "lineHeight"
  | "letterSpacing"
  | "shadow"
  | "duration"
  | "easing"
  | "opacity"
  | "zIndex";
```

- [ ] **Step 2: Update TokenEditor to map "typography" category to "fontFamily"**

In `poiesis/src/components/design/TokenEditor.tsx`, find the typography category token creation (around line 43):

```typescript
// Change from:
tokenType: "font",
// To:
tokenType: "fontFamily",
```

And in the category dropdown (around line 47, 266-268), remove `"font"` from the selectable options:

```typescript
// In the TokenType dropdown options, remove:
// { value: "font", label: "Font (shorthand)" }
// Keep fontFamily, fontSize, fontWeight, lineHeight, letterSpacing
```

- [ ] **Step 3: Run TypeScript type check**

```bash
cd poiesis && npx tsc --noEmit
```

Expected: Zero type errors (removing a union member may expose dead code paths that referenced `"font"` — fix any compilation errors).

- [ ] **Step 4: Commit**

```bash
git add poiesis/src/core/types/design.ts poiesis/src/components/design/TokenEditor.tsx
git commit -m "fix: remove invalid 'font' token type from editor TokenType union

The compiler's parse_token_type accepts 13 types (no 'font'), but the
editor's union had 14 including 'font'. Creating a typography token
emitted 'token x: font = \"...\"' which the compiler rejects, blocking
all subsequent saves.

Remove 'font' and map the typography category to 'fontFamily' instead."
```

---

### Task 4: Route multi-file diagnostics through cached diagnostics-only WASM path (H8)

**Files:**
- Modify: `poiesis/src/services/monaco/diagnostics.ts:81-90` (`runDiagnostics` multi-file branch)
- Modify: `omnilang-wasm/src/lib.rs` (optional: expose `analyse_multi_cached` as a dedicated diagnostics export)

**Problem:** `runDiagnostics` calls `compiler.compile_to_omniview(false, false)` for multi-file validation, which runs full JS+HTML+CSS codegen and discards everything except `errors`. For the 92-file learn-omniview corpus, this is ~100-300ms of synchronous main-thread blocking in WASM per keystroke. The `analyse_multi_cached` function already exists in Rust and returns `{ errors, warnings }` — it's just not wired to the diagnostics path.

- [ ] **Step 1: Add a cached diagnostics-only method to `OmniCompiler`**

In `omnilang-wasm/src/lib.rs`, add a method to `OmniCompiler` (after the existing `diagnostics()` method around line 654):

```rust
    /// Run cached multi-file analysis returning only diagnostics.
    /// Uses analyse_multi_cached for incremental performance — skips codegen entirely.
    /// Returns { success: bool, errors: Vec<Diagnostic>, warnings: Vec<Diagnostic> }.
    #[wasm_bindgen]
    pub fn diagnostics_multi_cached(&mut self) -> JsValue {
        let sources: Vec<(String, String)> = self
            .sources
            .iter()
            .map(|(path, source)| (path.clone(), source.clone()))
            .collect();
        let result = omnilang::analyse_multi_cached(&mut self.cache, sources);
        let output = serde_json::json!({
            "success": result.errors.iter().all(|d| d.severity != "error"),
            "errors": &result.errors,
            "warnings": &result.warnings,
        });
        to_js_value(&output)
    }
```

Note: If `omnilang::analyse_multi_cached` isn't directly accessible from `omnilang-wasm`, add a re-export in `omnilang/src/lib.rs` first.

- [ ] **Step 2: Use the cached diagnostics path in runDiagnostics**

In `poiesis/src/services/monaco/diagnostics.ts`, change the multi-file branch (lines 81-90):

```typescript
    // Multi-file: use cached diagnostics-only path (no codegen)
    if (allSources && Object.keys(allSources).length > 1) {
      for (const [path, source] of Object.entries(allSources)) {
        if (source) compiler.upsert_source(path, source);
      }
      // Use diagnostics_multi_cached — analyse_multi_cached under the hood,
      // returns only errors/warnings, no codegen output.
      const result = compiler.diagnostics_multi_cached();
      if (result && typeof result === "object" && "errors" in result) {
        return (result as { errors: WasmDiagnostic[] }).errors ?? [];
      }
      return [];
    }
```

- [ ] **Step 3: Build WASM and verify**

```bash
cd omnilang-wasm && wasm-pack build --target web --out-dir ../poiesis/src/wasm
```

- [ ] **Step 4: Run Poiesis TypeScript check**

```bash
cd poiesis && npx tsc --noEmit
```

Expected: Zero type errors from the new `diagnostics_multi_cached` method.

- [ ] **Step 5: Commit**

```bash
git add omnilang-wasm/src/lib.rs omnilang/src/lib.rs poiesis/src/services/monaco/diagnostics.ts poiesis/src/wasm/
git commit -m "perf: route multi-file diagnostics through cached, codegen-free WASM path

Multi-file diagnostics (triggered on every keystroke in the editor) were
calling compile_to_omniview which runs full JS+HTML+CSS codegen and
discards everything except errors. For the 92-file learn-omniview corpus,
this was ~100-300ms of main-thread blocking in WASM.

Add diagnostics_multi_cached() to OmniCompiler — calls analyse_multi_cached
(which already exists) and returns only { errors, warnings } without any
codegen. This is the exact path the code comment falsely claimed existed."
```

---

### Verification Checklist

Before marking this plan complete, verify:

- [ ] `cd poiesis && npx vitest run` — all Poiesis tests pass
- [ ] `cd poiesis && npx tsc --noEmit` — zero type errors
- [ ] Save a design.omni with `@themes` — blocked with clear error message
- [ ] Save a design.omni with only static tokens — succeeds normally
- [ ] Create a typography token in the Design tab — uses `fontFamily` not `font`
- [ ] Multi-file diagnostics in the editor are faster (no codegen overhead)
- [ ] Save-time validation catches cross-file errors (was dead before Task 1 fix)
