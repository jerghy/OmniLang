# ADR-028: OmniView Codegen as Debug/Development Runtime — Intentional Design Trade-offs

## Status

Accepted

## Context

ADR-012 established OmniView codegen as an independent "debug tool" — not a production deployment target. However, the 2026-07-29 comprehensive architecture review (`.architecture/reviews/component-omniview.md`, 10 specialists) identified 46 findings, some of which were initially classified as bugs or deficiencies. On closer inspection, **14 of these "findings" are intentional design trade-offs** made to optimize OmniView for its debug/development runtime role.

This ADR documents these intentional decisions, their rationales, and their consequences — converting what appeared to be oversights into acknowledged architectural trade-offs. Without this ADR, future reviewers will re-flag the same patterns as bugs, wasting review cycles.

## Decision Drivers

- **Self-contained output**: OmniView generates a single HTML file with zero external dependencies — critical for iframe embedding in the Poiesis editor
- **Human readability over compression**: Generated JS/CSS must be inspectable for debugging; minification is deferred to production targets (React, Flutter)
- **Simplicity over performance**: A ~200-line signal runtime that fits in one screen of code is more debuggable than an optimized reactive engine
- **Editor integration over isolation**: The compiler output carries editor metadata (element tree, nid maps, design IDs) that production backends don't need
- **Synchronous determinism**: Predictable, immediate signal propagation simplifies debugging — no microtask scheduling surprises
- **Correctness by phase ordering**: A documented 6-phase JS execution order ensures DOM exists before bindings, events before effects

## Decision

OmniView codegen is designed as a **debug/development runtime** — not a production deployment target. This single overarching decision explains 14 subordinate design choices that prioritize human readability, self-contained output, synchronous determinism, and editor integration over production concerns like minification, batching, tree-shaking, and runtime performance optimization.

**Architectural Components Affected:**
- `OmniViewGenerator` — the entire codegen module (~11,000 lines) is purpose-built for debug output
- Runtime (`runtime.rs`) — ~200-line inline signal system optimized for simplicity, not performance
- Output format (`OmniViewOutput`) — carries editor metadata alongside generated code
- Editor bridges (`editor.rs`, `debug.rs`) — postMessage and Tauri IPC for Poiesis integration
- CSS-only mode — reuses codegen pipeline for editor canvas styling

**Interface Changes**: None. This ADR documents existing intentional design, not a new change.

The 14 specific design decisions are detailed below.

## Sub-Decisions

### D1: Inline Signal Runtime (~200 lines)

**Decision**: Embed a minimal reactive runtime directly in every generated output, rather than sharing a separate runtime file.

**Rationale**: 
- Self-contained HTML works without a build step, bundler, or CDN
- The entire reactive system fits in one screen of code — a developer can read and understand all of it
- No version skew between compiler and runtime (they ship together)
- ADR-012 §3 explicitly chose "self-contained iframe-ready document" as the output format

**Trade-off**: Multi-page apps duplicate the runtime per page. For production, React/Flutter backends handle this.

**Evidence**: `omnilang/src/codegen/omniview/mod.rs:3-6`, `runtime.rs:17-18`, ADR-012:15-16

---

### D2: String-Based Codegen (Not Structured CodeWriter)

**Decision**: Build JS/HTML/CSS output via `String::push_str()` and `writeln!()` into flat buffers, rather than using the structured `CodeWriter` + trait pattern shared by React/Flutter backends.

**Rationale**:
- OmniView generates the entire app in one pass (not per-file like React/Flutter)
- Flat string buffers match the "one output, one document" model
- The imperative pattern (`self.js.push_str(...)`) is simpler to debug than a tree of `CodeWriter` nodes
- ADR-012 explicitly decided OmniView should NOT be forced to adopt `FormatExpr`/`FormatStmt` traits used by other backends

**Trade-off**: No syntax validation of generated JS; no structured refactoring of output. Accepted because OmniView output is human-inspected during development.

**Evidence**: ADR-012:15-16, ADR-012:22-23, ADR-012:36-37, ADR-012:72-75

---

### D3: No Signal Batching

**Decision**: Every `signal.value = v` triggers immediate, synchronous subscriber notification — no microtask deferral, no batch queue, no topological ordering.

**Rationale**:
- Synchronous = predictable = debuggable. A developer setting `count = 5` can immediately inspect the DOM and see the result.
- Microtask batching introduces timing complexity (effects fire "later") that complicates debugging
- The ~200-line runtime budget deliberately excludes batch infrastructure
- Pragmatic Enforcer assessment: necessity 10/10 for the simple version, batching is "build the right thing later"

**Trade-off**: Cascading updates in deep computed chains cause redundant DOM writes. For complex production apps, this causes jank. Accepted because OmniView is a development/debug target; production apps use React/Flutter backends.

**Evidence**: `runtime.rs:36-46`, component-omniview.md:315, component-omniview.md:636-638

---

### D4: No Output Minification

**Decision**: Generated JS/CSS is human-readable with full indentation, comments (e.g., `/// ── Component: X ──`), and descriptive variable names. No post-processing pass removes whitespace or shortens identifiers.

**Rationale**:
- Debugging requires readable output — a developer can open the generated HTML and trace signal flow
- Comment markers identify which component generated which code block
- Minification adds a build step and complexity that contradicts the "self-contained single file" philosophy
- Minification is deferred as an optional long-term feature (component-omniview.md §Long-term)

**Trade-off**: Output is 3-5× larger than minified equivalent. For an iframe-embedded editor preview, this is irrelevant.

**Evidence**: component-omniview.md:318, component-omniview.md:348-351, component-omniview.md:873-876

---

### D5: No Tree-Shaking or Code Splitting

**Decision**: All generated code is included in a single file with no dead code elimination. Every component, every signal, every design token is emitted.

**Rationale**:
- Self-contained iframe-ready document design (ADR-012) precludes code splitting — there's only one output file
- Tree-shaking would require data-flow analysis across component boundaries, which contradicts the simple one-pass codegen model
- In development, you want to see all code — dead code elimination would hide components from inspection
- The motion runtime is conditionally included (`project_uses_motion()` check) as a pragmatic exception

**Trade-off**: Output includes unused code. Production targets (React with Vite/Rollup) handle tree-shaking at the bundler level.

**Evidence**: `mod.rs:3-6`, component-omniview.md:318, component-omniview.md:490

---

### D6: `wrap_debug_html` Without CSP

**Decision**: The debug HTML wrapper (`wrap_debug_html`) emits inline `<script>` blocks with zero Content-Security-Policy protection. The production wrapper (`html_document_wrapper`) includes a full CSP when not in editor mode.

**Rationale**:
- `wrap_debug_html` is explicitly marked "debug-ready" — it exists for `omnic run` development server and WASM preview
- CSP would block the inline scripts and styles that the debug workflow requires
- The production path (`build_file_output` → `html_document_wrapper`) correctly includes CSP
- Iframe sandboxing in the editor provides an alternative security boundary

**Trade-off**: Using `wrap_debug_html` in a production context would be insecure. Mitigated by naming convention (`debug`) and separate production path.

**Evidence**: `utils.rs:189-190`, `utils.rs:235-253`, `mod.rs:143` (passes `!self.editor_mode` to CSP flag)

---

### D7: CSS-Only Mode for Editor Canvas

**Decision**: The `css_only: bool` flag on `OmniViewCodegen` skips all JS emission and only collects CSS + element metadata. Used by `resolve_styles()` for the Poiesis editor canvas where React handles DOM rendering.

**Rationale**:
- The Poiesis editor renders components in React, but needs OmniView's CSS output for visual accuracy
- Reusing the same codegen pipeline (with JS disabled) guarantees style consistency between editor preview and exported output
- `debug_assert!` guards at JS entry points (`assert_not_css_only()`) prevent accidental JS emission in CSS-only mode

**Trade-off**: The same codegen pipeline has two distinct output paths, increasing internal complexity. Mitigated by debug assertions.

**Evidence**: `mod.rs:521-522`, `mod.rs:610-626`, `condition.rs:1-5`, `element.rs:499-503`, compiler-internals.md:193-196

---

### D8: Editor Hot-Replace with `(0, eval)`

**Decision**: The editor bridge uses `(0, eval)(factoryCode)` (indirect eval) to replace component factories at runtime, enabling live preview without page reload.

**Rationale**:
- Indirect eval `(0, eval)(...)` executes in global scope, avoiding closure scope leakage
- The pattern snapshots all signal values via `.peek()`, evals the new factory, calls `teardown()` on the old component, then restores signal values
- Only active in `editor_mode` — production builds do not include this code
- The `postMessage` origin check (`e.origin === window.location.origin`) restricts messages to same-origin

**Trade-off**: Any XSS in the parent editor page can execute arbitrary code in the preview iframe. Accepted because the editor is a trusted development environment. The production path does not include this code.

**Evidence**: `editor.rs:101-102`, `debug.rs:22-25`, `mod.rs:488-489`, ADR-010:22-23

---

### D9: `debug_mode` and `editor_mode` as Orthogonal Flags

**Decision**: Two boolean flags control editor integration: `editor_mode` (postMessage bridge for browser preview) and `debug_mode` (Tauri IPC bridge for standalone debug window). `debug_mode` requires `editor_mode = true`.

**Rationale**:
- Different communication channels for different editor contexts: postMessage for iframe, Tauri IPC for standalone window
- Orthogonal flags allow the compiler to emit the right bridge code per context
- `debug_assert!(!debug_mode || editor_mode)` enforces the dependency at development time

**Trade-off**: Two parallel bridge implementations (`editor.rs`, `debug.rs`) with overlapping concerns. The Pragmatic Enforcer recommends merging.

**Evidence**: `mod.rs:121-132`, `mod.rs:176-178`, `editor.rs:17-23`, `debug.rs:1-3`

---

### D10: Editor Metadata on OmniViewOutput

**Decision**: `OmniViewOutput` carries 11 editor-specific fields (`element_tree`, `element_classes`, `element_tags`, `per_component_css`, `nid_map`, `components`, `warnings`) alongside standard output fields (`js`, `html`, `css`, `source_map`).

**Rationale**:
- The Poiesis editor runs in TypeScript and consumes compiler output via WASM/FFI
- Re-parsing HTML strings on the TypeScript side to reconstruct element hierarchies would be error-prone and slow
- The compiler already walks the element tree during codegen — collecting metadata is incremental cost
- `ElementTreeNode` explicitly documents its purpose: "used by the Poiesis editor to reconstruct the view hierarchy in React without parsing HTML strings"

**Trade-off**: Output struct carries fields irrelevant to non-editor consumers. Couples OmniView codegen to Poiesis editor needs. The Implementation Strategist recommends extracting into optional `EditorMetadata` struct.

**Evidence**: `mod.rs:62-68`, `mod.rs:82-84`, `mod.rs:126-127`, component-omniview.md:556-558

---

### D11: 6-Phase JS Execution Order

**Decision**: The generated JS follows a documented 6-phase order: (1) Runtime definitions, (2) Store declarations, (3) Component definitions, (4) DOM insertion via innerHTML, (5) Deferred event bindings + reactive effects, (6) Mount lifecycle hooks.

**Rationale**:
- Phases 4-5 are the critical dependency: DOM must exist before event listeners or reactive effects can bind to elements
- Phase 5 is implemented by swapping `self.js` with `self.deferred_js` buffer — effects always emit to the deferred buffer, which is flushed after innerHTML
- The comment warns: "This ordering is critical: reordering these phases will cause silent runtime failures"

**Trade-off**: The buffer-swap pattern adds cognitive complexity to the codegen. Adding a new phase requires understanding the full ordering. Accepted because the alternative (runtime DOM existence checks everywhere) would be slower and more error-prone.

**Evidence**: `view/mod.rs:1-22`, `view/mod.rs:49-78`, component-omniview.md:263, component-omniview.md:587

---

### D12: `debug_assert!` Phase Guards (Not Production Checks)

**Decision**: `assert_can_emit_js()` and `assert_not_css_only()` use `debug_assert!` — active in debug/test builds, compiled out in release. They guard against phase-ordering mistakes during development.

**Rationale**:
- These are developer safety nets, not production security boundaries
- Zero runtime cost in release builds (the compiler team explicitly values this)
- `#[track_caller]` attribute ensures assertion failures point to the calling method, not the guard function

**Trade-off**: Release builds skip these checks. If a codegen bug causes phase-ordering violations only in release mode (e.g., through a different optimization path), the violation goes undetected. Mitigated by the fact that codegen is deterministic — the same code runs in both modes.

**Evidence**: `mod.rs:608-626`, component-omniview.md:216, component-omniview.md:587

---

### D13: Source Maps

**Decision**: OmniView generates source maps (`Vec<SourceMapEntry>`) mapping generated DOM nodes (`data-nid`) to `.omni` source locations. This is unique to OmniView among all compilation backends.

**Rationale**:
- Debugging: clicking an element in the browser and tracing it back to `.omni` source
- Editor integration: Poiesis uses source maps to highlight the corresponding source when an element is selected
- ADR-001 notes source map infrastructure exists but only OmniView backend builds them

**Trade-off**: Source map generation adds complexity to every element emission path. Not needed for production output.

**Evidence**: `mod.rs:45-46`, component-omniview.md:437, component-omniview.md:628, ADR-001:73

---

### D14: OmniView as a Debug Tool (Overall Role)

**Decision**: OmniView codegen is designed as a development/debug runtime, not a production deployment target. ADR-012 established this explicitly: "OmniView's use case: generates self-contained JS/HTML/CSS with inline signal runtime — for development debugging, not production deployment target."

**Rationale**:
- The Poiesis editor needs instant preview without build steps
- `omnic run` provides a zero-config dev server
- Production deployment goes through React or Flutter backends (with bundler, tree-shaking, optimization)
- This role division is the core architectural pattern of OmniLang: OmniView for development, React/Flutter for production

**Trade-off**: OmniView will never have the runtime performance of a production framework. It is not designed to. Confusion about this role caused the initial architecture review to flag 14 intentional design decisions as bugs.

**Evidence**: ADR-012:1 (title: "OmniView Codegen as Debug Tool"), ADR-012:15, ADR-012:75, ADR-012:90-93

---

## Consequences

### Positive

1. **All 14 design decisions are now documented as intentional** — future reviewers will not re-flag them as bugs
2. **Clear role separation**: OmniView (debug/dev) vs React/Flutter (production) — each optimized for its purpose
3. **Developer experience optimized**: readable output, synchronous determinism, self-contained files
4. **Architecture review findings can be reclassified**: intentional trade-offs vs actual bugs

### Negative

1. **Role confusion risk**: Without this ADR, developers may use OmniView output in production and encounter performance cliffs
2. **Dual maintenance**: Features added to OmniView (debug) must be separately added to React/Flutter (production)
3. **Documentation gap**: The public docs (`docs/omniview/README.md`) describe OmniView as a "UI framework" without clearly stating its debug-tool role

### Neutral

1. **Some intentional trade-offs have remediation paths**: Signal batching, output minification, and CSS sanitizer alignment are documented as "future if needed" improvements
2. **Pragmatic Enforcer's YAGNI analysis validated**: The framework's complexity is justified for its debug role; production optimization is correctly deferred

## Related ADRs

- [ADR-012: OmniView Codegen as Debug Tool — Independent Implementation](ADR-012-omniview-codegen-debug-tool.md) — Established OmniView's debug-tool role
- [ADR-001: Compiler Pipeline Architecture](ADR-001-compiler-pipeline-architecture.md) — Established multi-backend architecture
- [ADR-003: Codegen Target Prioritization](ADR-003-codegen-target-prioritization.md) — P0/P1/P2 backend tiers
- [ADR-027: Security Sanitization Architecture](ADR-027-omniview-security-sanitization-architecture.md) — Security decisions layered on the debug runtime

## References

- [OmniView Architecture Review (2026-07-29)](../../reviews/component-omniview.md) — The review that triggered this ADR
- [OmniView Compiler Internals](../../../memory/omniview-compiler-internals.md) — 21-module compiler documentation
