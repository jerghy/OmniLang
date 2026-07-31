# Dynamic Design Tokens — Global Architecture Review Fixes

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the 3 CRITICAL + 7 IMPORTANT gaps found in the global OmniView architecture review, filtered to exclude 5 findings that align with intentional ADR design decisions (document separately).

**Architecture:** All fixes are additive (new diagnostics, single-line guards, documentation) or subtractive (dead code removal). Zero pipeline restructuring. Each fix targets a specific, independently verified gap. Groups A-C are independent and can execute in parallel.

**Tech Stack:** Rust (omnilang-core, omnilang), Markdown (docs), TypeScript (Poiesis types only in optional Group E)

**Reference:** `.architecture/reviews/dynamic-design-tokens-omniview-global-2026-07-31.md`

---

## Intentional Design Decisions (NOT to fix — document only)

These 5 findings from the review align with explicit ADR decisions. Add an "ADR Design Intent" note to the review document:

| Finding | ADR | Design Intent |
|---------|-----|---------------|
| Theme sync is unidirectional | ADR-044 L83 | Store is single source of truth; `__omni_theme.toggle()` is for store-action use only |
| Binary operators not in expression scope | ADR-047 L137 | "Not a full expression system… This is intentional" |
| `var(--name)` fallback is graceful degradation | ADR-047 L128 | Listed as Positive consequence — lenient over crash |
| `SymbolKind::Unknown` exemption | ADR-050 L107,159-162 | Rejecting Unknown would break every cross-file const import |
| Effect consolidation trade-off | ADR-050 L109 | "Per-token dependency granularity is lost… Accepted because setProperty no-op" |

---

## Group A: Critical Lifecycle & Cross-Backend Fixes

### Task A1: Add dynamic token effect dispose/cleanup for hot-reload

**Files:**
- Modify: `omnilang/src/codegen/omniview/design.rs:280-304`
- Modify: `omnilang/src/codegen/omniview/mod.rs:426-431`

**Context:** `emit_dynamic_token_effects()` generates `__omni_effect(() => { ... })` calls whose return value (cleanup function) is discarded. In editor mode, each hot-reload creates new effect closures while old ones remain subscribed to signals. The fix: capture the cleanup function under a well-known window key, call old cleanup before registering new effects.

- [ ] **Step 1: Modify `emit_dynamic_token_effects` to emit cleanup guard**

In `omnilang/src/codegen/omniview/design.rs`, replace lines 291-302 with cleanup-aware output:

```rust
if tokens.is_empty() {
    return String::new();
}
let mut js = String::new();
// Dispose previous dynamic token effects (hot-reload safety)
js.push_str("if (window.__omni_tokenEffectCleanup) window.__omni_tokenEffectCleanup();\n");
js.push_str("window.__omni_tokenEffectCleanup = __omni_effect(() => {\n");
js.push_str("  const root = document.documentElement;\n");
js.push_str("  if (!root) return;\n");
for (name, expr) in tokens {
    let js_expr = expr_to_js(expr);
    let _ = writeln!(
        js,
        "  root.style.setProperty(\"--{}\", __omni_sanitizeCss({}));",
        name, js_expr
    );
}
js.push_str("\n});\n");
js
```

- [ ] **Step 2: Run existing tests to verify no regression**

Run: `cargo test -p omnilang design_codegen`
Expected: all tests pass (the captured return value is transparent to existing assertions)

- [ ] **Step 3: Verify the generated JS pattern**

Run: `cargo test -p omnilang design_codegen_test::js_output_contains_dynamic_token_effects -- --nocapture`
Expected: PASS, output contains `window.__omni_tokenEffectCleanup`

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/omniview/design.rs
git commit -m "fix: add dynamic token effect cleanup for hot-reload safety

Capture __omni_effect return value under window.__omni_tokenEffectCleanup,
calling the old dispose function before registering new effects. Prevents
effect accumulation across editor hot-reload cycles (CRITICAL from global
architecture review).

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task A2: Add compile-time WARNING for design blocks on non-OmniView targets

**Files:**
- Modify: `omnilang/src/codegen/mod.rs:63-119`

**Context:** `ModuleIR.designs` is populated during IR lowering and passed to every backend, but only `codegen/omniview/` reads it. Non-web backends silently produce zero design CSS. Add a WARNING diagnostic to the compile dispatch.

- [ ] **Step 1: Add design block check in compile()**

In `omnilang/src/codegen/mod.rs`, after line 78 (the target match), add a warning check before the match:

```rust
// Warn when design blocks are compiled to targets that don't support them
let mut output = match target {
    #[cfg(feature = "codegen-omniview")]
    Target::OmniView => omniview::OmniViewCodegen::default().generate(ir, source_files),
    #[cfg(feature = "codegen-python")]
    Target::Python => {
        if has_design_blocks(ir) {
            return CompileOutput {
                files: vec![],
                errors: vec![],
                source_map: vec![],
                warnings: vec![CompileWarning {
                    message: "Design blocks are currently only supported on the OmniView target. \
                              Design tokens, classes, and variants will not be emitted in Python output. \
                              Use OmniView target for design system features.".into(),
                    span: None,
                }],
            };
        }
        python::PythonCodegen.generate(ir, source_files)
    }
    // ... repeat for each non-OmniView target
```

- [ ] **Step 2: Add `has_design_blocks()` helper**

In the same file, add before the `compile()` function:

```rust
/// Check whether the IR program contains any non-empty design blocks.
fn has_design_blocks(ir: &IRProgram) -> bool {
    ir.modules.values().any(|m| !m.designs.is_empty())
}
```

- [ ] **Step 3: Apply to all non-OmniView targets**

Add the same check to: JavaScript, TypeScript, Rust, Dart, React, Vue, Dioxus, Flutter. The Python case in Step 1 is the template — copy the pattern.

Use `CompileWarning` from the codegen types. Check the existing type: `grep "struct CompileWarning" omnilang/src/codegen/`

If `CompileWarning` doesn't exist, use `CompileOutput.warnings` field which should be `Vec<String>` — simplify to:

```rust
warnings: vec!["Design blocks are only supported on the OmniView target. ...".into()],
```

- [ ] **Step 4: Run compile tests**

Run: `cargo test -p omnilang`
Expected: all tests pass. Some tests may need updating if they compile design blocks to non-OmniView targets expecting zero warnings.

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/mod.rs
git commit -m "fix: warn when design blocks are compiled to non-OmniView targets

Add has_design_blocks() check in compile() dispatch that emits a warning
for React, Flutter, Vue, Dioxus, Python, JavaScript, TypeScript, Rust,
and Dart targets when the IR contains design blocks. Previously these
backends silently produced zero design CSS output with no diagnostic.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Group B: Documentation Fixes

### Task B1: Update design-system.md with dynamic token usage

**Files:**
- Modify: `docs/omniview/10-design-system.md`

**Context:** The user-facing design system guide has zero mention of dynamic tokens, const references, or the naming convention. Users cannot discover Phase 1 or Phase 2 features.

- [ ] **Step 1: Add "Dynamic Token Values" subsection**

In `docs/omniview/10-design-system.md`, after the existing token definition examples, add:

````markdown
### Dynamic Token Values

Token values can reference compile-time constants or runtime signals, in addition to static literals:

**Static literals** (unchanged):
```omni
token primary: color = "#6366f1";
token spacing: size = "16px";
```

**Compile-time constant references** (Phase 1):
```omni
const BRAND = "#6366f1";
const SPACING = "16px";

design App {
    token primary: color = BRAND;    // resolves to "#6366f1" at compile time
    token spacing: size = SPACING;   // resolves to "16px" at compile time
}
```

**Naming convention for constants:** Use PascalCase (`BrandColor`) or
UPPER_SNAKE_CASE (`BRAND_COLOR`). All-lowercase identifiers are treated
as CSS literal values (e.g., `white`, `ease-in-out`, `solid`).

To force a value to be treated as a CSS literal regardless of case, wrap
it in quotes: `token primary: color = "white";`

**Runtime signal references** (Phase 2):
```omni
store ThemeStore {
    signal primaryColor: string = "#6366f1";
}

design App {
    token primary: color = ThemeStore.primaryColor;
    // Generates a reactive CSS custom property that updates
    // automatically when ThemeStore.primaryColor changes
}
```

**Restrictions:**
- Dynamic tokens (Phase 2) cannot have theme overrides. Use store-level
  theme switching instead.
- Design blocks are currently only supported on the OmniView target.
  Compiling to React, Flutter, or other targets will produce a warning.
````

- [ ] **Step 2: Commit**

```bash
git add docs/omniview/10-design-system.md
git commit -m "docs: add dynamic token usage to design system guide

Document compile-time const references (Phase 1), runtime signal
references (Phase 2), naming convention, theme restrictions, and
cross-backend limitation.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task B2: Fix grammar BNF in 15-grammar-view.md

**Files:**
- Modify: `docs/omniview/15-grammar-view.md:625-653`

**Context:** The token_def BNF is wrong: missing type annotation (`: color`), missing terminating semicolon, uses old color-* naming convention, shows no type annotation syntax.

- [ ] **Step 1: Replace outdated token_def grammar**

In `docs/omniview/15-grammar-view.md`, replace lines 625-653 with:

````markdown
### 6.2 Token Definition

```
token_def    ::= 'token' <ident> ':' <token_type> '=' <token_value> ';'
                ['@themes' <ident> (',' <ident>)* ';']
                ['@default-theme' <ident> ';']

token_type   ::= 'color' | 'size' | 'radius' | 'fontFamily' | 'fontSize'
               | 'fontWeight' | 'lineHeight' | 'letterSpacing' | 'shadow'
               | 'duration' | 'easing' | 'opacity' | 'zIndex'

token_value  ::= <string_literal>           // "#6366f1", "16px"
               | <ident>                    // CSS keyword or const reference
               | <expr>                     // Phase 2: runtime expression
```

Theme overrides: `token primary: color = "#6366f1" dark "#a5b4fc";`
Theme values may be literals or const references (not runtime expressions).

```
design AppDesign {
    @themes light, dark;
    @default-theme light;

    token primary: color = "#6366f1" dark "#a5b4fc";
    token spacing: size = "16px";
    token radiusMd: radius = "8px";
    token fontBase: fontFamily = "Inter";
    token durationFast: duration = "150ms";
    token easeOut: easing = "cubic-bezier(0.16, 1, 0.3, 1)";
}
```
````

- [ ] **Step 2: Commit**

```bash
git add docs/omniview/15-grammar-view.md
git commit -m "docs: fix grammar BNF for token definitions

Correct token_def to include type annotation, semicolon, and modern
token_type enum. Add token_value production. Replace outdated color-*
naming convention example with actual syntax.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task B3: Fix obsolete const description in 14-grammar-lang.md

**Files:**
- Modify: `docs/omniview/14-grammar-lang.md:660`

**Context:** The const description says "Parsed but discarded at the AST level" — obsolete since Phase 1 stores const values in `Module.consts`.

- [ ] **Step 1: Update const description**

In `docs/omniview/14-grammar-lang.md`, find the const section (around line 660) and replace the "parsed but discarded" sentence:

```markdown
Module-level constants. Values are stored in the AST (`Module.consts`)
and resolved at compile time for use in design token values and imports.
Constants are implicitly public within the module and can be imported
across files via the standard `import { NAME } from "./file.omni"` syntax.

Example:
```omni
const BRAND = "#6366f1";
const SPACING = "16px";

design App {
    token primary: color = BRAND;    // compile-time resolution
}
```
```

- [ ] **Step 2: Commit**

```bash
git add docs/omniview/14-grammar-lang.md
git commit -m "docs: fix obsolete const description in grammar reference

Update from 'parsed but discarded' to reflect Phase 1 behavior: const
values are now stored in Module.consts and used for compile-time
resolution in design token values and cross-file imports.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task B4: Fix broken ADR cross-references

**Files:**
- Modify: `.architecture/decisions/adrs/ADR-050-dynamic-token-validation-gap-closure.md:227-228`
- Modify: `.architecture/reviews/dynamic-design-tokens-2026-07-31.md:437`

**Context:** ADR-050 references wrong filenames. Review report references wrong ADR (ADR-048 instead of ADR-050).

- [ ] **Step 1: Fix ADR-050 references**

In `.architecture/decisions/adrs/ADR-050-dynamic-token-validation-gap-closure.md`, lines 227-228, replace:

```markdown
- [ADR-047: Dynamic Design Tokens Phase 1](ADR-047-dynamic-design-tokens.md)
- [ADR-049: Dynamic Design Tokens Phase 2 Runtime Signal References](ADR-049-dynamic-tokens-phase2-runtime-signals.md)
```

With:

```markdown
- [ADR-047: Dynamic Variable References in Design Tokens — Phase 1](ADR-047-dynamic-variable-design-tokens.md)
- [ADR-049: Dynamic Token Variables — Phase 2 Runtime Signal References](ADR-049-dynamic-token-variables-phase2-runtime-signals.md)
```

- [ ] **Step 2: Fix review report ADR reference**

In `.architecture/reviews/dynamic-design-tokens-2026-07-31.md`, find the line referencing `ADR-048-*.md` and replace `ADR-048` with `ADR-050`.

- [ ] **Step 3: Commit**

```bash
git add .architecture/decisions/adrs/ADR-050-dynamic-token-validation-gap-closure.md
git add .architecture/reviews/dynamic-design-tokens-2026-07-31.md
git commit -m "docs: fix broken ADR cross-references

ADR-050 used wrong filenames for ADR-047 and ADR-049. Review report
referenced ADR-048 instead of ADR-050.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Group C: Validation & Sanitization Symmetry

### Task C1: Add FontWeight to numeric-accepting token types

**Files:**
- Modify: `crates/omnilang-core/src/semantic/resolve.rs:50-58`

**Context:** `check_token_type_compatibility()` allows numeric expressions for Duration, Opacity, ZIndex, Size, and LineHeight — but not FontWeight, despite CSS font-weight accepting numeric values 100-900.

- [ ] **Step 1: Add FontWeight to the numeric_ok match**

In `crates/omnilang-core/src/semantic/resolve.rs`, line 52-58, add `TokenType::FontWeight`:

```rust
let numeric_ok = matches!(
    token_type,
    TokenType::Duration
        | TokenType::Opacity
        | TokenType::ZIndex
        | TokenType::Size
        | TokenType::LineHeight
        | TokenType::FontWeight
);
```

- [ ] **Step 2: Write a test**

In `crates/omnilang-core/src/semantic/mod.rs` (test section), add:

```rust
#[test]
fn semantic_accepts_numeric_for_fontweight_token() {
    // FontWeight should accept Int/Float expressions, same as Duration/Opacity/etc.
    let src = r#"
        fn getWeight(): int { return 700; }
        design Test {
            token bold: fontWeight = getWeight();
        }
    "#;
    let result = analyse_single("test.omni", src);
    // Should not produce E0010 type mismatch diagnostic for this
    let has_type_error = result.diagnostics.iter().any(|d|
        d.message.contains("type") && d.message.contains("fontWeight"));
    assert!(!has_type_error, "FontWeight should accept numeric expressions");
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p omnilang-core semantic_accepts_numeric_for_fontweight`
Expected: PASS

Run: `cargo test -p omnilang-core`
Expected: all 552+ tests pass

- [ ] **Step 4: Commit**

```bash
git add crates/omnilang-core/src/semantic/resolve.rs
git add crates/omnilang-core/src/semantic/mod.rs
git commit -m "fix: add FontWeight to numeric-accepting token types

FontWeight was missing from the numeric_ok match in
check_token_type_compatibility(), despite CSS font-weight accepting
numeric values 100-900. This caused E0010 type mismatches for valid
numeric font-weight expressions.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task C2: Apply sanitize_css_value to theme and pseudo-state CSS blocks

**Files:**
- Modify: `omnilang/src/codegen/omniview/design.rs:94-95` (theme `[data-theme]` blocks)
- Modify: `omnilang/src/codegen/omniview/design.rs:108-109` (`@media` blocks)
- Modify: `omnilang/src/codegen/omniview/design.rs:151-152` (pseudo-state properties)

**Context:** ADR-050 applied `sanitize_css_value()` to `:root` static token values but missed the theme override and pseudo-state CSS emission paths.

- [ ] **Step 1: Sanitize theme `[data-theme]` block values**

In `omnilang/src/codegen/omniview/design.rs`, line 95, replace:

```rust
let _ = writeln!(css, "  --{}: {};", name, val);
```

With:

```rust
let sanitized = sanitize_css_value(val);
let _ = writeln!(css, "  --{}: {};", name, sanitized);
```

- [ ] **Step 2: Sanitize `@media` block values**

In `omnilang/src/codegen/omniview/design.rs`, line 109, replace:

```rust
let _ = writeln!(css, "    --{}: {};", name, val);
```

With:

```rust
let sanitized = sanitize_css_value(val);
let _ = writeln!(css, "    --{}: {};", name, sanitized);
```

- [ ] **Step 3: Sanitize pseudo-state property values**

In `omnilang/src/codegen/omniview/design.rs`, line 152, replace:

```rust
let _ = writeln!(css, "  {}: {};", camel_to_kebab(prop), val);
```

With:

```rust
let sanitized = sanitize_css_value(val);
let _ = writeln!(css, "  {}: {};", camel_to_kebab(prop), sanitized);
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p omnilang design_codegen`
Expected: all tests pass

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/omniview/design.rs
git commit -m "fix: apply sanitize_css_value to theme and pseudo-state CSS blocks

ADR-050 applied sanitization to :root static tokens but missed three
other CSS emission paths: [data-theme] blocks, @media blocks, and
pseudo-state properties. Apply consistent sanitization across all
static CSS output paths.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task C3: Add const-kind validation to theme_value Variable references

**Files:**
- Modify: `crates/omnilang-core/src/semantic/resolve.rs:367-382`

**Context:** The primary value path validates that `TokenValue::Variable` references are `SymbolKind::Const` (ADR-050 fix), but the theme_values loop only checks symbol existence — non-const symbols pass and silently degrade to `var(--name)`.

- [ ] **Step 1: Add const-kind check to theme_values Variable arm**

In `crates/omnilang-core/src/semantic/resolve.rs`, replace lines 367-382 (the `TokenValue::Variable` arm in the theme_values loop) with:

```rust
TokenValue::Variable(name) => {
    if !self.symbols.contains(name) {
        self.diagnostics.push(Diagnostic::error(
            DiagnosticKind::E0001,
            &format!(
                "Undefined variable '{}' in theme '{}' of design token '{}'",
                name, theme_name, token.name
            ),
            &Span { start: 0, end: 0, line: 0, col: 0 },
        ));
    } else if let Some(sym) = self.symbols.resolve(name) {
        if sym.kind != SymbolKind::Const
            && sym.kind != SymbolKind::Unknown
        {
            self.diagnostics.push(Diagnostic::error(
                DiagnosticKind::E0001,
                &format!(
                    "'{}' is a {} in theme '{}' of design token '{}', not a \
                     compile-time constant. Theme override values must use \
                     const declarations or static literals.",
                    name,
                    format!("{:?}", sym.kind).to_lowercase(),
                    theme_name,
                    token.name,
                ),
                &Span { start: 0, end: 0, line: 0, col: 0 },
            ));
        }
    }
}
```

- [ ] **Step 2: Write a test for non-const theme value rejection**

In `crates/omnilang-core/tests/design_theme_test.rs`, add:

```rust
#[test]
fn rejects_non_const_variable_in_theme_value() {
    let src = r#"
        let darkColor = "#111827";
        design Test {
            @themes light, dark;
            token primary: color = "#ffffff" dark darkColor;
        }
    "#;
    let result = analyse_single("test.omni", src);
    let has_const_error = result.diagnostics.iter().any(|d|
        d.message.contains("not a compile-time constant")
            && d.message.contains("darkColor"));
    assert!(has_const_error, "Should reject non-const variable in theme value");
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p omnilang-core rejects_non_const_variable_in_theme_value`
Expected: PASS

Run: `cargo test -p omnilang-core`
Expected: all 552+ tests pass

- [ ] **Step 4: Commit**

```bash
git add crates/omnilang-core/src/semantic/resolve.rs
git add crates/omnilang-core/tests/design_theme_test.rs
git commit -m "fix: add const-kind validation to theme value Variable references

Mirror the primary-value const-kind check (ADR-050) in the theme_values
validation loop. Previously, theme value Variable references only checked
symbol existence — non-const symbols passed and silently degraded to
var(--name). Now rejects with E0001 diagnostic.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task C4: Add WARNING diagnostic for SymbolKind::Unknown in Variable references

**Files:**
- Modify: `crates/omnilang-core/src/semantic/resolve.rs:297-301`

**Context:** `SymbolKind::Unknown` exemption means cross-file imported symbols bypass const-kind validation. A WARNING (not ERROR) would alert users without breaking cross-file const imports.

- [ ] **Step 1: Add Unknown-kind warning**

In `crates/omnilang-core/src/semantic/resolve.rs`, after line 301 (the `SymbolKind::Unknown` exemption comment), add a warning diagnostic:

```rust
// Allow Unknown — cross-file imported symbols may not have their
// final kind resolved yet at design token validation time.
&& sym.kind != SymbolKind::Unknown
{
    // ... existing error diagnostic ...
}
// New: Warn when a cross-file import has unknown kind
if sym.kind == SymbolKind::Unknown {
    self.diagnostics.push(Diagnostic::warning(
        DiagnosticKind::E0001,
        &format!(
            "Symbol '{}' in design token '{}' has unknown kind \
             (imported from another file). Verify it is declared \
             as a compile-time constant (const).",
            name, token.name
        ),
        &Span { start: 0, end: 0, line: 0, col: 0 },
    ));
}
```

Wait — this logic is inside the `else if` that already handles non-Unknown. Let me restructure more carefully.

The current code at lines 297-327:
```rust
} else if let Some(sym) = self.symbols.resolve(name) {
    if sym.kind != SymbolKind::Const
        && sym.kind != SymbolKind::Unknown
    {
        // error diagnostic
    }
}
```

Add after the closing `}` of the `else if` block (after line 327):

```rust
// Warn for cross-file imports whose kind hasn't been resolved yet
if let Some(sym) = self.symbols.resolve(name) {
    if sym.kind == SymbolKind::Unknown {
        self.diagnostics.push(Diagnostic::warning(
            DiagnosticKind::E0001,
            &format!(
                "Symbol '{}' referenced in design token '{}' was imported \
                 from another file and its kind could not be verified at \
                 compile time. Ensure '{}' is declared as a const in its \
                 source file.",
                name, token.name, name
            ),
            &Span { start: 0, end: 0, line: 0, col: 0 },
        ));
    }
}
```

- [ ] **Step 2: Check if Diagnostic::warning exists**

Run: `grep -n "fn warning" crates/omnilang-core/src/semantic/diagnostics.rs`

If `Diagnostic::warning` doesn't exist, use `Diagnostic::error` with a softer message prefix `"Note:"` instead, or add a severity field. Check the Diagnostic struct:

Run: `grep -n "pub struct Diagnostic" crates/omnilang-core/src/semantic/`

If there's a `severity` or `level` field, use `DiagnosticSeverity::Warning`. If not, add the diagnostic as an error but prefix the message with "Note: " to indicate it's informational.

- [ ] **Step 3: Run tests**

Run: `cargo test -p omnilang-core`
Expected: all 552+ tests pass. Cross-file tests may now produce the warning — verify they still pass.

- [ ] **Step 4: Commit**

```bash
git add crates/omnilang-core/src/semantic/resolve.rs
git commit -m "feat: add warning for Unknown-kind symbols in design token Variable refs

When a cross-file imported symbol is used as a TokenValue::Variable,
emit a warning since the symbol's kind cannot be verified at compile
time. The const-kind check exempts Unknown (ADR-050 D3), but users
should be alerted that their import may not be a compile-time constant.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Group D: Code Quality Improvements

### Task D1: Remove duplicate error_offset assignment

**Files:**
- Modify: `crates/omnilang-core/src/parser/design.rs:356-360`

**Context:** Lines 357 and 360 contain identical assignments — the first is dead code (shadowed by the second). Fixes the Rust Expert's MEDIUM-1 finding.

- [ ] **Step 1: Remove the shadowed assignment**

In `crates/omnilang-core/src/parser/design.rs`, delete lines 356-357 (the first `error_offset` assignment and its comment):

Before (lines 354-361):
```rust
    let expr_tokens: Vec<Token> = design_tokens[start_idx..end_idx].iter().cloned().collect();

    // Capture offset before moving expr_tokens into Parser
    let error_offset = expr_tokens.first().map(|t| t.span.start).unwrap_or(offset);

    // Use the full design body as source (token spans are offsets into it)
    let error_offset = expr_tokens.first().map(|t| t.span.start).unwrap_or(offset);
```

After:
```rust
    let expr_tokens: Vec<Token> = design_tokens[start_idx..end_idx].iter().cloned().collect();

    // Capture offset before moving expr_tokens into Parser.
    // Uses the full design body as source (token spans are offsets into it).
    let error_offset = expr_tokens.first().map(|t| t.span.start).unwrap_or(offset);
```

- [ ] **Step 2: Also fix the clippy warning on the same line**

Replace `.iter().cloned().collect()` with `.to_vec()` per Rust Expert LOW-3 finding:

```rust
    let expr_tokens: Vec<Token> = design_tokens[start_idx..end_idx].to_vec();
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p omnilang-core parse_`
Expected: all parser tests pass

- [ ] **Step 4: Commit**

```bash
git add crates/omnilang-core/src/parser/design.rs
git commit -m "refactor: remove duplicate error_offset assignment, use to_vec()

The first error_offset assignment (line 357) was dead code shadowed by
the identical second assignment (line 360). Merge comments and remove
duplicate. Also replace .iter().cloned().collect() with .to_vec() per
clippy recommendation.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task D2: Add #[non_exhaustive] to TokenValueIR

**Files:**
- Modify: `crates/omnilang-core/src/ir/design.rs:37`

**Context:** `TokenValueIR` is defined in omnilang-core and consumed in the omnilang codegen crate. Adding a third variant would silently match `_` in downstream code. `#[non_exhaustive]` forces compile errors on new variants, alerting maintainers.

- [ ] **Step 1: Add #[non_exhaustive]**

In `crates/omnilang-core/src/ir/design.rs`, line 37, add the attribute:

```rust
#[non_exhaustive]
pub enum TokenValueIR {
    Static(String),
    Dynamic(ExprIR),
}
```

- [ ] **Step 2: Add match wildcards where needed**

The `omnilang` crate matches on `TokenValueIR` in two places. Run the build to find breakages:

Run: `cargo check -p omnilang 2>&1`

Expected: errors about non-exhaustive match on `TokenValueIR`. Fix each by adding a wildcard arm:

```rust
_ => {} // non_exhaustive: future variants fall through
```

- [ ] **Step 3: Fix all match sites**

Apply the wildcard to:
- `omnilang/src/codegen/omniview/mod.rs:418-421` (filter_map collecting Dynamic tokens)
- `omnilang/src/codegen/omniview/design.rs:66` (theme token map filtering Static values)
- `omnilang/src/codegen/omniview/design.rs:78` (:root emission skipping Dynamic)

- [ ] **Step 4: Run full test suite**

Run: `cargo test -p omnilang-core -p omnilang`
Expected: all tests pass

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/ir/design.rs
git add omnilang/src/codegen/omniview/mod.rs
git add omnilang/src/codegen/omniview/design.rs
git commit -m "refactor: add #[non_exhaustive] to TokenValueIR enum

TokenValueIR sits at a crate boundary (omnilang-core → omnilang).
Without #[non_exhaustive], adding a third variant would silently match
wildcards in downstream code. This forces compile errors on new variants.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task D3: Add suggest_similar_name() to token variable undefined diagnostic

**Files:**
- Modify: `crates/omnilang-core/src/semantic/resolve.rs:284-296`

**Context:** Regular VarRef resolution uses `suggest_similar_name()` for "did you mean?" suggestions. Token variable undefined errors don't — making AI self-correction harder.

- [ ] **Step 1: Add similarity suggestion**

In `crates/omnilang-core/src/semantic/resolve.rs`, lines 284-296, replace the plain error message:

```rust
TokenValue::Variable(name) => {
    if !self.symbols.contains(name) {
        let suggestion = suggest_similar_name(&self.symbols, name);
        let mut msg = format!(
            "Undefined variable '{}' in design token '{}'",
            name, token.name
        );
        if let Some(sug) = suggestion {
            let _ = write!(msg, ". Did you mean '{}'?", sug);
        }
        msg.push_str(
            ". Variable references in design token values must use \
             compile-time constants (PascalCase or UPPER_SNAKE_CASE). \
             Lowercase names are treated as CSS values."
        );
        self.diagnostics.push(Diagnostic::error(
            DiagnosticKind::E0001,
            &msg,
            &Span { start: 0, end: 0, line: 0, col: 0 },
        ));
    } else if let Some(sym) = self.symbols.resolve(name) {
```

- [ ] **Step 2: Run tests**

Run: `cargo test -p omnilang-core`
Expected: all tests pass. Tests that check exact error messages may need updating.

- [ ] **Step 3: Commit**

```bash
git add crates/omnilang-core/src/semantic/resolve.rs
git commit -m "feat: add did-you-mean suggestions to token variable E0001 errors

Use the existing suggest_similar_name() function to provide spelling
suggestions when an undefined variable is referenced in a design token
value. Also adds guidance about the naming convention (PascalCase/
UPPER_SNAKE_CASE for variables, lowercase for CSS values).

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Group E: Deferred Items (document, don't implement now)

### Task E1: Document deferred items in review and ADR

**Files:**
- Modify: `.architecture/reviews/dynamic-design-tokens-omniview-global-2026-07-31.md`

- [ ] **Step 1: Add "Intentional Design Decisions" section**

Insert a new section before "Prioritized Recommendations" documenting the 5 findings that align with ADR design intent (see table at the top of this plan).

- [ ] **Step 2: Update deferred items section**

Add the following items as explicitly deferred with ADR justification:

```markdown
### Documented as Deferred (ADR-aligned)

| # | Finding | Deferred Because |
|---|---------|-----------------|
| E1 | `infer_token_expr_type()` chained FieldAccess depth | S effort, upgrade P3-3 LOW→MEDIUM. Recursive FieldAccess resolution needed for `Store.nested.field`. |
| E2 | Per-element effect cleanup in `@for` re-renders | M effort. Requires `__omni_cleanupElement` extension + effect registry. Perf impact only at scale (50+ dynamic elements). |
| E3 | Poiesis `DesignToken` type dynamic expression support | M effort. Requires discriminated union on DesignToken.value + emitter changes. Editor integration not on critical path. |
| E4 | Feature flag `dynamic-tokens` for omnilang-core | S effort. Adds build-time gate. Deferred until cross-backend support exists. |
```

- [ ] **Step 3: Commit**

```bash
git add .architecture/reviews/dynamic-design-tokens-omniview-global-2026-07-31.md
git commit -m "docs: document intentional design decisions and deferred items

Add ADR-aligned design decisions section and deferred items list to the
global architecture review. Distinguishes bugs/gaps from intentional
trade-offs documented in ADR-044, ADR-047, ADR-049, ADR-050.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Execution Order

Groups are independent and can execute in parallel:

```
Group A (Critical) ─┐
Group B (Docs)     ─┼── Parallel
Group C (Validation)─┤
Group D (Quality)   ─┘
                       └── Group E (Deferred docs)
```

**Estimated total effort:** 2-3 hours (Groups A-D), 30 min (Group E docs)

## Verification

After all groups complete:

- [ ] `cargo test -p omnilang-core` — 552+ tests pass (plus new tests from C1, C3, C4)
- [ ] `cargo test -p omnilang` — all codegen tests pass
- [ ] `cargo clippy --workspace` — zero new warnings
- [ ] `cargo fmt --check --all` — clean
- [ ] Manual: open learn-omniview in editor, verify dynamic token effects cleanup on hot-reload
- [ ] Manual: verify docs/omniview/10-design-system.md renders correctly
- [ ] Manual: verify grammar BNF examples compile without errors
