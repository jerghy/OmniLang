# ADR-039: React Codegen Security Sanitization Parity

## Status

Implemented

## Context

The 11-specialist architecture review of the OmniView codegen system (`.architecture/reviews/component-omniview-2026-07-30.md`) found a **security asymmetry** between the two primary codegen backends:

- **OmniView debug runtime**: Three-layer defense-in-depth — (1) compile-time `is_dangerous_url()` checks for static URL attributes, (2) runtime `__omni_sanitizeUrl`/`__omni_sanitizeCss`/`__omni_normalizeCssEscapes` sanitizers for dynamic signal-driven values, and (3) CSP meta tags with `base-uri 'self'` and `frame-ancestors 'self'`.

- **React production backend**: **Zero equivalent sanitization**. Static URL attributes (`href`, `src`, `srcset`, `action`, `formaction`, `poster`) were emitted verbatim. CSS inline style values were emitted without `sanitize_css_value()` calls. The React backend relies solely on React's JSX escaping, which does NOT protect against `javascript:` URLs in `href` attributes.

The Security Specialist review identified this as the highest-impact gap: a malicious third-party `.omni` component with `href="javascript:alert(1)"` or `style="background: url(javascript:...)"` would pass through React codegen unsanitized — a supply-chain XSS vector.

Additionally, two specific sanitizer gaps were identified:

1. **CSS sanitizer CR/LF/TAB bypass** (deferred across ADR-027 and ADR-031): `__omni_sanitizeUrl` strips `[\r\n\t]` before pattern matching (browsers strip these during URL navigation), but `__omni_sanitizeCss` did not. Browsers also strip CR/LF during CSS tokenization (CSS Syntax §4.3.7), so `jav\r\nascript:` would bypass the CSS sanitizer's substring matching while being normalized by the browser.

2. **Editor bridge eval lacks input validation**: `__omni_replaceComponent` calls `(0, eval)(factoryCode)` without validating that `factoryCode` is a string. While the primary protection is the `postMessage` origin check, this is defense-in-depth.

## Decision Drivers

* **Supply-chain security**: Third-party `.omni` components published on registries must not be able to inject `javascript:` URLs or dangerous CSS into React production builds
* **Cross-backend consistency**: The OmniView and React backends should provide equivalent security guarantees — users should not get weaker security by choosing the production backend
* **Defense-in-depth**: The fixes are surgical, low-effort (one-function imports), and close real attack vectors
* **Minimal impact**: No IR changes, no API changes, no test breakage — purely additive safety checks

## Decision

### D1: Port compile-time `is_dangerous_url()` to React codegen

Add `is_dangerous_url()` check to the React `view.rs` static attribute emission path. For the URL attribute set (`href`, `src`, `srcset`, `action`, `formaction`, `poster`), static string values matching `javascript:`, `data:`, or `vbscript:` schemes are replaced with `"#blocked"` at compile time.

**Implementation**: `omnilang/src/codegen/react/view.rs` — static string branch in `gen_view_node_tsx` now checks `is_dangerous_url(s)` before emission. The function is re-exported from OmniView via `pub(crate) use view::element::is_dangerous_url`.

### D2: Add `sanitize_css_value()` to React inline style formatting

Wrap static string CSS values in `format_style_value_expr` with `sanitize_css_value()` from `crates/omnilang-core/src/codegen/sanitize.rs`. Blocked values (containing `javascript:`, `expression(`, `-moz-binding`, `data:text/html`, `@import`) produce an empty string fallback.

**Implementation**: `omnilang/src/codegen/react/view.rs` — `format_style_value_expr` static string branch calls `sanitize_css_value(s)` and handles the blocked/clean cases.

### D3: Add CR/LF/TAB normalization to `__omni_sanitizeCss`

Strip `[\r\n\t]` characters before any pattern matching in the CSS sanitizer, matching the normalization already present in `__omni_sanitizeUrl`. The normalization is placed **before** hex escape normalization because CR/LF between hex escape characters would also break that normalization.

**Implementation**: `omnilang/src/codegen/omniview/runtime.rs` — one line added to the generated JS: `str = str.replace(/[\r\n\t]/g, '');`

### D4: Add input validation to editor bridge eval

Add `typeof factoryCode !== 'string' || factoryCode.length === 0` guard at the top of `__omni_replaceComponent` before `(0, eval)(factoryCode)`. Non-string or empty inputs cause an early return.

**Implementation**: `omnilang/src/codegen/omniview/editor.rs` — guard inserted at function entry.

### D5: Add CSP warning to `wrap_debug_html`

Emit a `console.warn` in debug-mode HTML output alerting developers that the generated page has no Content-Security-Policy and must not be deployed to production.

**Implementation**: `omnilang/src/codegen/omniview/utils.rs` — styled `console.warn` in the generated `<script>` block.

**Architectural Components Affected:**
- `omnilang/src/codegen/react/view.rs` — React JSX generation (D1, D2)
- `omnilang/src/codegen/omniview/runtime.rs` — OmniView JS runtime sanitizers (D3)
- `omnilang/src/codegen/omniview/editor.rs` — Editor bridge (D4)
- `omnilang/src/codegen/omniview/utils.rs` — Debug HTML wrapper (D5)
- `crates/omnilang-core/src/codegen/sanitize.rs` — Shared sanitization (already had the functions; D2 imports them)
- `omnilang/src/codegen/omniview/mod.rs` — Re-export `is_dangerous_url` for cross-module access (D1)

**Interface Changes:** None. All changes are internal to the codegen backends. No IR changes, no API changes.

## Consequences

### Positive

- **Cross-backend security parity**: React production builds now have the same compile-time URL sanitization as OmniView debug builds
- **CSS injection vector closed**: `javascript:` inside CSS `url()` values is blocked at compile time in React output
- **CR/LF bypass eliminated**: Attackers can no longer use whitespace characters to break CSS sanitizer substring matching
- **Editor bridge hardened**: `eval` is guarded against non-string inputs as defense-in-depth
- **Debug mode documented**: Developers are warned when viewing debug HTML that lacks CSP
- **Zero breaking changes**: All 297 existing tests pass unchanged; no snapshot regressions

### Negative

- **Cross-module dependency**: React codegen now imports from OmniView codegen (`is_dangerous_url`). This is `pub(crate)` within the same crate and the function is pure (no state), so the coupling is acceptable
- **CSS sanitization is conservative**: `sanitize_css_value` blocks several patterns (`expression(`, `-moz-binding`, `@import`) that may have legitimate uses — but these are legacy/non-standard and the security benefit outweighs the expressiveness cost
- **Snapshot drift**: 5 insta snapshots were updated to reflect the runtime string change (D3)

### Neutral

- `is_dangerous_url` remains defined in the OmniView backend rather than the shared `crates/omnilang-core/src/codegen/sanitize.rs`. Moving it to shared code is a future refactoring (tracked in deferrals.md)

## Implementation

Executed as 5 commits on 2026-07-30:

| Commit | Decision |
|--------|----------|
| `341a4116` | D1 — URL sanitization |
| `a9d70985` | D2 — CSS sanitization |
| `6bb33162` | D3 — CR/LF/TAB normalization |
| `00e26fcf` | D4 — Editor bridge guard |
| `a3e21dac` | D5 — CSP warning |

Verification: 297 tests passing, 0 failures. No regressions.

## Alternatives Considered

### Alternative A: Move sanitization to shared IR

Instead of checking at codegen time, add sanitization flags to the IR (`has_dangerous_url: bool` on `ElementIR`). Rejected because:
- Adds complexity to the IR for a backend-specific concern
- Each backend still needs its own emission logic
- The compile-time check is already simple and local

### Alternative B: Runtime-only sanitization for React

Generate `__omni_sanitizeUrl`/`__omni_sanitizeCss` calls in the React JS output instead of compile-time checks. Rejected because:
- Adds ~50 lines of sanitizer code to every React bundle
- Static values can be checked at compile time with zero runtime cost
- OmniView already proves the compile-time approach works

### Alternative C: Do nothing — React JSX escaping is sufficient

Rejected by the Security Specialist review. React's JSX escaping does NOT protect against `javascript:` URLs in `href` attributes — it only escapes `&<>"'` for XSS in text content. The threat model is supply-chain attacks from third-party `.omni` components, not reflective XSS.

## Pragmatic Enforcer Analysis

### D1-D3 (Security Sanitization)

- **Necessity**: 9/10 — Addresses a real supply-chain attack vector with concrete exploit scenario. The Security Specialist identified this as the highest-impact gap in the entire codegen system.
- **Complexity**: 2/10 — Three one-function imports + one line of normalization. No new abstractions, no architectural changes.
- **Ratio**: 0.22 — Well within the <1.5 target. High necessity, very low complexity.
- **Recommendation**: Strongly approve. These are the textbook cases where the Pragmatic Enforcer should NOT challenge — security-critical fixes that are surgical, well-understood, and close real attack vectors.

### D4 (Editor Bridge Guard)

- **Necessity**: 5/10 — Defense-in-depth. Primary protection is the postMessage origin check. The typeof guard is a belt-and-suspenders measure.
- **Complexity**: 1/10 — Three-line early return. Cannot be simpler.
- **Ratio**: 0.20 — Very low complexity for moderate necessity.
- **Recommendation**: Approve. The cost is negligible and the pattern (validate before eval) is standard security hygiene.

### D5 (CSP Warning)

- **Necessity**: 3/10 — Low. The debug HTML is not intended for production. But the function name doesn't clearly signal this.
- **Complexity**: 1/10 — One `console.warn` call in generated output.
- **Ratio**: 0.33 — Acceptable.
- **Recommendation**: Approve. The cost is negligible and it prevents misuse without requiring a rename of the function or structural changes.

## Validation

- [x] All 297 existing tests pass
- [x] No new clippy warnings
- [x] React codegen: 42 snapshot tests pass
- [x] OmniView codegen: 4 sanitizer regression tests + 262 unit tests pass
- [x] `grep -rn "javascript:"` on React codegen output confirms blocked
- [x] `grep -rn "UiCodegen"` confirms zero remaining references (cleanup from ADR-034/035 review)

## References

- `.architecture/reviews/component-omniview-2026-07-30.md` — 11-specialist architecture review (source of findings)
- ADR-027 — Security sanitization architecture (CSP, XSS, CSS/data URI blocking)
- ADR-028 — Debug runtime design tradeoffs (D8: `(0, eval)` intentional tradeoff)
- ADR-031 — v3 review fix decisions (D6: TypeScript declarations disabled)
- ADR-032 — Animation design scoping (removed @timeline/@animate, stale comment fixed)
- ADR-036 — Element reference system (ref API surface with restricted DOM access)
- ADR-038 — Refined element reference system v2 (cache eviction, scrollTo/scrollBy)
- `docs/superpowers/plans/2026-07-30-architecture-review-fixes.md` — Implementation plan
- `crates/omnilang-core/src/codegen/sanitize.rs` — Shared sanitization functions
