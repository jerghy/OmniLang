# ADR-027: OmniView Security Sanitization Architecture

## Status

Accepted

## Context

The 2026-07-29 post-ADR-026 architecture review (`.architecture/reviews/component-omniview-2026-07-29-post-remediation.md`) found that the ADR-026 security layer (`__omni_sanitizeUrl`, `__omni_sanitizeCss`, CSP meta tag) was correctly implemented in principle but had scope gaps. The 11-task remediation covered all findings. This ADR records the architectural decisions made during the security hardening, particularly:

1. URL scheme sanitization strategy (blocklist vs allowlist)
2. CSS `url()` sanitization approach (blanket-block vs protocol-aware)
3. `String()` coercion for `toString()` bypass prevention
4. Compile-time defense-in-depth for static URL attributes
5. Content-Security-Policy directive selection

## Decision Drivers

- **Correctness over compatibility**: Sanitization must block known XSS vectors without breaking legitimate use cases (e.g., `background-image: url("https://...")` via signals)
- **Defense-in-depth**: Three-layer protection — compile-time static sanitization, runtime dynamic sanitization, CSP meta tag
- **Minimal runtime overhead**: Sanitization functions execute only on signal value changes (within `__omni_effect` callbacks), not on every render
- **ES5 compatibility**: All runtime JS must use `function`, `var`, `indexOf` — no arrow functions, `const`/`let`, or `includes()`
- **Supply-chain defense**: Static URL attributes in `.omni` source are sanitized at compile time, protecting against malicious third-party component packages

## Decision

### D1: URL scheme blocklist (not allowlist)

**Block all `javascript:`, `data:`, and `vbscript:` schemes in `__omni_sanitizeUrl`.**

The function blocks three scheme prefixes after trimming and lowercasing the input:

```javascript
function __omni_sanitizeUrl(url) {
  var str = String(url);
  var trimmed = str.trim().toLowerCase();
  if (trimmed.indexOf('javascript:') === 0 ||
      trimmed.indexOf('data:') === 0 ||
      trimmed.indexOf('vbscript:') === 0) {
    return '#blocked';
  }
  return str;
}
```

**Rationale for full `data:` block (not just `data:text/html`)**: The v2 review's security layer only blocked `data:text/html`. This missed `data:text/javascript`, `data:application/javascript`, and other executable subtypes. Maintaining a subtype allowlist is fragile — new executable MIME types can emerge. Blocking the entire `data:` scheme is the safer choice.

**Trade-off**: Legitimate `data:image/png;base64,...` URIs in dynamic `href`/`src` attributes are also blocked. This is an accepted security trade-off. The CSP's `img-src 'self' data: https:` still allows static `data:` URIs in `<img src>` attributes (they are set at compile time, not through signals).

**Rejected alternative**: Allowlist approach (`http:`, `https:`, `/`, `#`, `mailto:`, `tel:`) would require maintaining an ever-growing list of safe schemes and would break whenever a new legitimate scheme emerged.

### D2: CSS `url()` two-pass sanitization

**Relax `url()` blocking from blanket to protocol-aware in `__omni_sanitizeCss`.**

The function uses a two-pass approach:

1. **First pass** (unchanged): Block known CSS injection patterns — `javascript:`, `expression(`, `-moz-binding`, `data:text/html`, `@import`, `vbscript:`, `document.`, `window.`
2. **Second pass** (new): If `url()` is present, check only the content after `url(` for `javascript:` or `vbscript:` protocols

```javascript
function __omni_sanitizeCss(value) {
  var str = String(value);
  var lower = str.toLowerCase();
  // First pass: block known injection patterns
  if (lower.indexOf('javascript:') >= 0 ||
      lower.indexOf('expression(') >= 0 ||
      lower.indexOf('-moz-binding') >= 0 ||
      lower.indexOf('data:text/html') >= 0 ||
      lower.indexOf('@import') >= 0 ||
      lower.indexOf('vbscript:') >= 0 ||
      lower.indexOf('document.') >= 0 ||
      lower.indexOf('window.') >= 0) {
    return '';
  }
  // Second pass: validate url() content
  var urlIdx = lower.indexOf('url(');
  if (urlIdx >= 0) {
    var afterUrl = lower.substring(urlIdx);
    if (afterUrl.indexOf('javascript:') >= 0 ||
        afterUrl.indexOf('vbscript:') >= 0) {
      return '';
    }
  }
  return str;
}
```

**Rationale**: The previous blanket `url()` block prevented legitimate use cases like `style={{ backgroundImage: signal }}` with `url("https://example.com/bg.png")`. Developers would be forced to bypass the sanitizer entirely, weakening security more than a targeted check would. The two-pass approach allows `url("https://...")` and `url("/path/...")` while still blocking `url(javascript:alert(1))`.

**Known limitation**: The second pass is technically redundant — the first pass already catches `javascript:` and `vbscript:` anywhere in the string. It serves as defense-in-depth: if someone later removes `javascript:` from the first pass, the second pass still catches `url(javascript:...)`.

**Known bypass**: CSS escape sequences (e.g., `\6aavascript:` for `javascript:`) are not detected by substring matching. Risk is accepted because signal values come from developer-controlled Omni source, not end-user input.

> **⚠️ 2026-07-29 更新**：此 CSS 转义绕过已在 [[ADR-030]] D4 中修复——`normalize_css_escapes` 函数同时添加到 Rust 编译时和 JS 运行时 sanitizer 中。

**Related concern**: The Rust-side compile-time `sanitize_css_value` function (`crates/omnilang-core/src/codegen/sanitize.rs:18`) still blanket-blocks `url(`. This inconsistency between compile-time and runtime sanitizers should be resolved in a future follow-up.

### D3: `String()` coercion in both sanitizers

**Replace `typeof value !== 'string'` guard with `var str = String(value)`.**

**Rationale**: If a computed signal returns an object with a custom `toString()` method (e.g., `{toString: () => "javascript:alert(1)"}`), the old `typeof !== 'string'` guard would return the raw object. When the DOM API later coerces it via `.toString()`, the dangerous value bypasses sanitization entirely. `String(value)` coerces immediately, then sanitizes the result.

**Rejected alternative**: Adding a separate `typeof` check for objects and throwing an error. This would break legitimate computed values that return numbers (which `String()` converts safely).

### D4: Compile-time static URL sanitization

**Add `is_dangerous_url()` function to check literal URL attribute values at compile time.**

The function mirrors `__omni_sanitizeUrl`'s blocklist but operates on static attribute values in `.omni` source:

```rust
pub(crate) fn is_dangerous_url(value: &str) -> bool {
    let trimmed = value.trim();
    let lower = trimmed.to_ascii_lowercase();
    lower.starts_with("javascript:")
        || lower.starts_with("data:")
        || lower.starts_with("vbscript:")
}
```

When a dangerous URL is detected, the compiler emits `#blocked` as the attribute value and pushes a warning. This provides defense-in-depth against supply-chain compromises where a third-party `.omni` component package contains literal `javascript:` or `data:` URLs.

**Rationale**: Dynamic bindings are protected by `__omni_sanitizeUrl` at runtime, but literal values in `.omni` source bypass that layer entirely. Compile-time checking closes this gap.

### D5: CSP directives

**Add `base-uri 'self'` and `frame-ancestors 'self'` to the CSP meta tag for standalone HTML output.**

The full CSP:

```
default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline';
img-src 'self' data: https:; connect-src 'self'; base-uri 'self';
frame-ancestors 'self';
```

- **`base-uri 'self'`**: Prevents `<base href>` tag hijacking of resource URLs. Without this, a compromised component emitting `<base href="https://evil.com/">` could redirect all relative resource loads.
- **`frame-ancestors 'self'`**: Prevents clickjacking via cross-origin iframe embedding. The OmniView output is embeddable only by same-origin pages.
- **`style-src 'unsafe-inline'`**: Required because OmniView compiles component styles into inline `<style>` blocks rather than external CSS files.

**Note**: Editor mode (`editor_mode=true`) intentionally excludes CSP, relying on iframe sandbox attributes instead. This is documented in `html_document_wrapper`'s docstring.

## Consequences

### Positive

- **XSS surface reduced**: URL scheme injection via reactive `href`/`src`/`srcset`/`action`/`formaction`/`poster` bindings is blocked at runtime. CSS injection via signal-driven styles is blocked. Static URL attributes are blocked at compile time.
- **Defense-in-depth**: Three-layer protection covers compile-time, runtime, and HTTP-level (CSP) attack surfaces.
- **`toString()` bypass eliminated**: Both sanitizers coerce inputs to strings before validation, preventing object-to-string coercion attacks.
- **CSS `url()` functionality preserved**: Signal-driven `background-image: url("https://...")` works correctly, removing pressure to disable the sanitizer.
- **Supply-chain protection**: Compile-time static URL check catches malicious URLs in third-party `.omni` components.

### Negative

- **Legitimate `data:` URIs blocked at runtime**: Signal-driven `data:image/png;base64,...` in `href`/`src` attributes are blocked. Workaround: use static attributes or external URLs.
- **CSS escape sequence bypass**: `url(\6aavascript:alert(1))` is not detected by substring matching. Risk accepted for OmniView's threat model (signals are developer-controlled, not end-user-controlled).
> **⚠️ 2026-07-29 更新**：CSS 转义绕过已在 [[ADR-030]] D4 中修复。然而 ADR-031 指出 URL sanitizer 的 CR/LF/TAB 绕过修复未扩展到 `__omni_sanitizeCss`（推迟为 MEDIUM follow-up）。
- **Compile-time vs runtime CSS sanitizer inconsistency**: Rust-side `sanitize_css_value` still blanket-blocks `url(`, while JS runtime allows it with protocol checking. Static CSS values cannot use `url()`. Follow-up needed.
> **⚠️ 2026-07-29 更新**：Rust/JS CSS sanitizer 不一致已在 [[ADR-030]] D4 中部分解决（Rust 端现在也允许带安全协议的 `url()`）。

### Neutral

- **Runtime overhead**: Sanitization functions execute only on signal value changes (inside `__omni_effect` callbacks). No measurable performance impact.
- **ES5 compatibility maintained**: All sanitization functions use ES5 patterns exclusively.
- **Snapshot churn**: 5 insta snapshot files updated across multiple commits to reflect sanitizer function body changes.

## Implementation Strategy

### Blast Radius

**Impact Scope**: Restricted to generated JS runtime and compile-time attribute emission. No changes to the Omni language syntax, IR, or parser.

**Affected Components**:
- `omnilang/src/codegen/omniview/runtime.rs` — JS sanitizer function bodies
- `omnilang/src/codegen/omniview/view/element.rs` — compile-time URL sanitization + runtime sanitizer integration
- `omnilang/src/codegen/omniview/utils.rs` — CSP meta tag
- `omnilang/src/codegen/omniview/view/style.rs` — CSS sanitizer integration

**Risk Mitigation**: All changes verified by regression tests (snapshot tests for JS output, unit tests for compile-time sanitization). Zero snapshot differences for refactoring commits.

### Reversibility

**Reversibility Level**: High. Each sanitizer change is localized to a single function. Reversing any individual decision requires changing the function body string literal and accepting updated snapshots.

### Sequencing & Timing

**Prerequisites** (all satisfied):
- [x] ADR-026 security layer is in place (baseline `__omni_sanitizeUrl`, `__omni_sanitizeCss`, CSP)
- [x] Post-ADR-026 architecture review completed
- [x] All 11 remediation tasks implemented and pushed

**Implemented in**: 11 commits (2026-07-29), branch `main`, commits `ba79fe84` through `0b2bf34d`

## Alternatives Considered

### URL allowlist instead of blocklist

**Approach**: Only allow specific URL schemes (`http:`, `https:`, `/`, `#`, `mailto:`, `tel:`) and block everything else.

**Pros**: Future-proof against new dangerous schemes. Clear security boundary.

**Cons**: Breaks whenever a new legitimate scheme emerges (e.g., `steam:`, `slack:`, custom protocol handlers). Requires maintaining a growing allowlist. Blocked by default is riskier for backward compatibility.

**Decision**: Rejected. Blocklist is simpler and more compatible. The three blocked schemes (`javascript:`, `data:`, `vbscript:`) are the only ones with known XSS potential.

### CSS `url()` allowlist for safe schemes only

**Approach**: In the second pass, only allow `url()` containing `https://`, `http://`, `/`, and `data:image/` — block everything else.

**Pros**: Stronger security guarantee. No bypass possible via unknown schemes inside `url()`.

**Cons**: More complex implementation. Still breaks on new legitimate URL schemes. The two-pass blocklist approach already blocks the known-dangerous protocols.

**Decision**: Deferred. Current two-pass approach is sufficient. Revisit if new CSS `url()` attack vectors emerge.

### Compile-time-only sanitization (no runtime layer)

**Approach**: Rely entirely on compile-time sanitization for both static and dynamic values.

**Pros**: Zero runtime overhead. Simpler architecture (single sanitization point).

**Cons**: Impossible in practice — signal-driven dynamic values are resolved at runtime, not compile time. The compiler cannot know what value a signal will hold when the user interacts with the page.

**Decision**: Rejected as architecturally infeasible.

## Pragmatic Enforcer Analysis

**Reviewer**: Pragmatic Enforcer
**Mode**: Balanced

**Overall Decision Complexity Assessment**: The sanitization architecture is appropriate for the problem. Each decision addresses a concrete, demonstrated attack vector from the architecture review. No speculative hardening was added.

**Necessity Assessment**: 9/10
- **Current need**: All attack vectors addressed (URL scheme injection, CSS injection, `toString()` bypass, supply-chain URLs, clickjacking, base-tag hijacking) were confirmed real by the security specialist review
- **Future need**: Defense-in-depth layers (CSP, compile-time check) address future risks
- **Cost of waiting**: Exploitable XSS in production OmniView output
- **Evidence**: v2 architecture review found the sanitization gaps; post-review confirmed scope expansion needed

**Complexity Assessment**: 2/10
- **Added complexity**: ~60 lines of runtime JS (two functions), ~20 lines of Rust (one helper function), one CSP string change
- **Maintenance burden**: Low — sanitization functions are self-contained with clear blocklists
- **Learning curve**: Low — standard security patterns (scheme checking, CSP directives)
- **Dependencies introduced**: None

**Pragmatic Score**:
- **Necessity**: 9/10
- **Complexity**: 2/10
- **Ratio**: 0.22 (excellent — well below 1.5 target)

**Recommendation**: ✅ Approve decision. The sanitization architecture addresses real, confirmed vulnerabilities with minimal complexity. Each layer carries its weight.

## Validation

**Acceptance Criteria:**
- [x] `__omni_sanitizeUrl` blocks `javascript:`, `data:`, `vbscript:` schemes
- [x] `__omni_sanitizeCss` blocks known CSS injection patterns while allowing `url("https://...")`
- [x] `String()` coercion prevents `toString()` bypass in both sanitizers
- [x] CSP includes `base-uri 'self'` and `frame-ancestors 'self'`
- [x] Compile-time URL check catches dangerous static attribute values
- [x] All existing tests pass; no regressions
- [x] New tests verify sanitizer function body correctness (3 regression tests)

**Testing Approach**:
- Runtime behavior verified through embedded function body assertions in `tests_regression/sanitizers.rs`
- Compile-time behavior verified through `test_is_dangerous_url` (9 assertions: 5 dangerous, 4 safe)
- Snapshot regression tests confirm no unintended JS output changes outside sanitizer functions

## References

- [Post-ADR-026 Architecture Review](.architecture/reviews/component-omniview-2026-07-29-post-remediation.md)
- [ADR-026: OmniView Review v2 Fix Decisions](.architecture/decisions/adrs/ADR-026-omniview-review-v2-fixes.md)
- [Session 2026-07-29 Post-Review Fixes](../memory/session-2026-07-29-post-review-fixes.md) (memory)
- [Fix Plan](docs/superpowers/plans/2026-07-29-omniview-post-review-fixes.md)
- [CSP Level 2 Specification](https://www.w3.org/TR/CSP2/)
- [RFC 3986: URI Generic Syntax](https://datatracker.ietf.org/doc/html/rfc3986)
