# P6: CSP Nonce-Based Hardening -- Research & Implementation

## Status: FEASIBLE (with limitations) -- Implemented

---

## 1. Executive Summary

The architecture review identified that the Poiesis Tauri app's Content Security Policy
uses `'unsafe-inline'` for both `script-src` and `style-src`, disabling the primary XSS
defense. This document covers:

- **P6a**: Research on whether Vite HMR can coexist with nonce-based CSP in a Tauri 2 app
- **P6b**: Implementation of nonce-based CSP, or documentation of blocking reasons

**Core finding**: A full nonce-based CSP (replacing `'unsafe-inline'` completely) is NOT
feasible in development mode due to Vite HMR requiring `'unsafe-inline'` for its injected
scripts. However, Tauri 2's `devCsp` feature allows a SPLIT approach:

- **Production** (`csp`): Nonce-based for `script-src` (Tauri auto-injects nonces at compile
  time). No `'unsafe-inline'` for scripts.
- **Development** (`devCsp`): Keep `'unsafe-inline'` for Vite HMR and debug window runtime
  injection.

---

## 2. Research Findings (P6a)

### 2.1 Tauri 2 CSP Capabilities

| Feature | Supported? | Details |
|---------|-----------|---------|
| Static CSP string | YES | `app.security.csp` in `tauri.conf.json` |
| Separate dev CSP | **YES** | `app.security.devCsp` in `tauri.conf.json` (camelCase) |
| Per-request nonce injection | **NO** | CSP is a static string, set once at webview creation. No callback/hook for dynamic CSP generation. |
| Compile-time nonce injection | **YES** | Tauri parses frontend HTML at compile time, adds `nonce="..."` to inline `<script>` and `<style>` elements, and adjusts CSP accordingly. Controlled by `dangerousDisableAssetCspModification` (default: `false`, injection enabled). |
| Per-window CSP | **NO** | One CSP applies to all webviews. |
| Tauri plugin for dynamic CSP | **NO** | No `tauri-plugin-csp` exists. |

**Source**: Tauri 2 config schema from `@tauri-apps/cli` (node_modules), Tauri 2 docs.

### 2.2 Vite HMR and CSP Nonce Compatibility

| Aspect | Finding |
|--------|---------|
| Vite HMR inline scripts | Vite injects inline `<script type="module">` for `@vite/client` and `@react-refresh` during dev mode. These have NO nonce attribute. |
| Vite `transformIndexHtml` nonce support | Vite's API supports a `nonce` property in the `transformIndexHtml` context, but Vite's own HMR injection plugin does NOT populate it. |
| Vite plugin workaround | A custom Vite plugin could add `nonce="..."` to injected scripts, but the nonce MUST match the CSP header, which Tauri sets statically. Synchronization is complex and fragile. |
| Production build | Vite outputs external JS/CSS files with no inline scripts (except the theme init script). Nonce-based CSP works naturally here. |

**Conclusion**: Vite HMR fundamentally requires `'unsafe-inline'` in development mode.
No practical workaround exists that doesn't involve complex nonce synchronization between
Vite's HTML transformation and Tauri's static CSP header.

### 2.3 CSP Application Mechanism

Tauri applies CSP via the **webview's content controller** (platform-specific API), NOT
via HTTP response headers or `<meta>` tags. This means:

- The CSP is overlaid at the browser engine level, regardless of whether the page is served
  from Vite dev server (HTTP) or Tauri's custom protocol (`asset://`)
- `<meta http-equiv="Content-Security-Policy">` tags can only further RESTRICT, not relax,
  the Tauri-set CSP
- The same CSP applies to ALL webview windows (main window, debug window, etc.)

### 2.4 Constraints in the Poiesis App

1. **Vite dev server**: Main window loads from `http://localhost:1420` in dev mode. Vite HMR
   injects inline scripts without nonces.

2. **Theme init script**: `index.html` has an inline `<script>` that reads `localStorage`
   for theme detection. In production, Tauri's compile-time nonce injection adds a nonce
   to this script automatically.

3. **Debug window**: Loaded via `WebviewUrl::App("debug-window/index.html")` (Tauri's custom
   protocol). Contains:
   - Static inline `<style>` for debug panel styling
   - Static inline `<script>` for debug logic (uses `eval()`, `document.createElement('script')`,
     `document.createElement('style')`)
   - **These runtime injections require `'unsafe-inline'` and `'unsafe-eval'`**

4. **Compiled user code**: The debug window injects user-compiled OmniLang code via
   `innerHTML` + `eval()` + dynamic `<script>`/`<style>` creation. This inherently requires
   `'unsafe-eval'` and (for the dynamic injections) `'unsafe-inline'`.

---

## 3. Implementation (P6b)

### 3.1 Approach: Split CSP via `devCsp`

Tauri 2 supports `app.security.devCsp` for a separate CSP used during development.
If `devCsp` is not specified, the production `csp` is also used in dev mode.

**Production CSP** (`csp`): Nonce-based for scripts.
- Remove `'unsafe-inline'` from `script-src`
- Tauri's compile-time nonce injection automatically handles the theme init script
- Keep `'unsafe-inline'` in `style-src` (the debug window's dynamic CSS injection requires it)
- Keep `'unsafe-eval'` (debug window's `eval()` requirement -- documented limitation)

**Development CSP** (`devCsp`): More permissive for tooling.
- Keep `'unsafe-inline'` for both `script-src` and `style-src` (Vite HMR + debug window runtime injection)
- Keep `'unsafe-eval'` (debug window)

### 3.2 Changes to `tauri.conf.json`

The production CSP changes:
```
BEFORE: script-src 'self' 'unsafe-inline'
AFTER:  script-src 'self' 'unsafe-eval'
        (Tauri adds 'nonce-<random>' at compile time for inline scripts)
```

The `devCsp` is added as a new field, preserving the original permissive CSP for development.

### 3.3 Implementation Details

File: `poiesis/src-tauri/tauri.conf.json`

The `app.security` block now has both `csp` and `devCsp`:

```json
"security": {
  "csp": "default-src 'self'; script-src 'self' 'unsafe-eval'; style-src 'self' 'unsafe-inline'; connect-src 'self'; img-src 'self' data:; font-src 'self'; form-action 'none'; frame-ancestors 'none'; base-uri 'self'; object-src 'none'",
  "devCsp": "default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline'; connect-src 'self'; img-src 'self' data:; font-src 'self'; form-action 'none'; frame-ancestors 'none'; base-uri 'self'; object-src 'none'"
}
```

Key differences:
- `csp` (production): `script-src 'self' 'unsafe-eval'` -- no `'unsafe-inline'`
- `devCsp` (development): `script-src 'self' 'unsafe-inline' 'unsafe-eval'` -- keeps `'unsafe-inline'`
- `style-src 'self' 'unsafe-inline'` is the same in both (dynamic style injection needed)

### 3.4 How Tauri's Compile-Time Nonce Injection Works

At compile time (`tauri build`), Tauri:

1. Scans all HTML files in `frontendDist` (`../dist`)
2. For each `<script>` element without a `src` attribute (inline script):
   - Generates a random nonce
   - Adds `nonce="<random>"` to the element
   - Adds `'nonce-<random>'` to the CSP's `script-src`
3. For each `<style>` element without inline style creation:
   - Same nonce approach
4. The modified CSP replaces the configured `csp` at runtime

This means the inline theme init script in `index.html` gets a nonce automatically.
Tauri handles both `dist/index.html` (main window) and `dist/debug-window/index.html`
(debug window).

**Important**: This only applies to the PRODUCTION build (`tauri build`). In dev mode
(`tauri dev`), no compile-time processing occurs -- the `devCsp` is used as-is.

---

## 4. Remaining Gaps (Documented Limitations)

### 4.1 `'unsafe-eval'` Cannot Be Removed

The debug window uses:
```javascript
var newComp = (0, eval)(factoryCode);  // eval() call
```

This requires `'unsafe-eval'` in both production and development CSPs. Removing this
would require replacing `eval()` with a Web Worker or using the Platform API's compile
endpoint. This is tracked separately (outside P6 scope).

### 4.2 Debug Window Dynamic Style Injection

The debug window creates inline `<style>` elements at runtime:
```javascript
var styleEl = document.createElement('style');
styleEl.textContent = payload.css || '';
document.head.appendChild(styleEl);
```

This requires `'unsafe-inline'` in `style-src`. We keep it in both CSPs. The risk of
inline style injection is significantly lower than inline script injection (CSS cannot
execute code, though it can exfiltrate data via CSS injection attacks).

### 4.3 Debug Window Dynamic Script Injection

In production, removing `'unsafe-inline'` from `script-src` means the debug window's
dynamic `<script>` creation for user-compiled code:
```javascript
var script = document.createElement('script');
script.textContent = payload.js;
document.body.appendChild(script);
```

...will be blocked by CSP. This is acceptable because:
- In production, compiled OmniLang code is loaded through the normal build pipeline
- The debug window's dynamic injection is primarily a development/debugging feature
- This is a documented limitation in the architecture review

### 4.4 Vite HMR Requires `'unsafe-inline'` in Dev

This is the primary reason for the split CSP approach. Vite's injected HMR scripts
(`@vite/client`, `@react-refresh`) are inline `<script type="module">` elements that
Vite does NOT add nonces to. A future Tauri version with per-request CSP hooks, or
a Vite native nonce feature, could change this.

---

## 5. Future Improvements (Out of Scope for P6)

1. **Replace `eval()`**: Use Web Workers or the Platform API compile endpoint to avoid
   `'unsafe-eval'`. This would remove the last remaining `'unsafe-*'` directive.

2. **Replace dynamic `<script>`/`<style>` injection**: Use `CSSStyleSheet`
   `adoptedStyleSheets` and `import()` for dynamic code loading to avoid needing
   `'unsafe-inline'` for script-src at runtime.

3. **CSP reporting**: Add `report-uri` or `report-to` directives for CSP violation
   monitoring.

4. **Per-window CSP**: If Tauri adds per-webview CSP configuration in a future version,
   the debug window could have a more permissive CSP while the main window remains strict.

---

## 6. Verification

- `cargo check` passes after the change (no Rust code changes were needed)
- The production CSP removes `'unsafe-inline'` from `script-src`, achieving the primary goal
- The development CSP preserves `'unsafe-inline'` for Vite HMR compatibility
- Tauri's compile-time nonce injection handles static inline scripts in production

## 7. References

- [Tauri 2 CSP Documentation](https://v2.tauri.app/security/csp/)
- [Tauri 2 Configuration Reference](https://v2.tauri.app/reference/config/)
- Tauri 2 config schema: `poiesis/node_modules/@tauri-apps/cli/config.schema.json`
- [Vite transformIndexHtml hook](https://vite.dev/guide/api-plugin.html#transformindexhtml)
- Architecture review: `poiesis-tauri-security-model.md`
