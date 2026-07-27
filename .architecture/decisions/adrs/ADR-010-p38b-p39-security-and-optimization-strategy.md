# ADR-010: P38b + P39 Strategy — Security Chain Fix + MVP Optimization

## Status

Accepted

## Context

Two deferred items from the 2026-07-26 review were reclassified to immediate execution in the 2026-07-27 follow-up:

- **P38b**: Strip `__TAURI_INTERNALS__` from debug window
- **P39**: IR optimization passes (dead signal elimination, simple computed inlining)

The 2026-07-27 deep review revealed that both original scopes needed adjustment:

### P38b Reality

The debug window (`poiesis/src-tauri/debug-window/index.html`, 363 lines) uses `__TAURI_INTERNALS__` for event IPC (emit/listen) to communicate with the main app. It is the transport layer between the sandboxed iframe and the parent window.

However, a **5-step XSS upgrade chain** was discovered:
```
postMessage('*') → no origin validation → eval(comp.js) → innerHTML injection → full Tauri API access
```

The sandboxed iframe (`sandbox="allow-scripts"`, no `allow-same-origin`) already prevents user code from accessing `__TAURI_INTERNALS__`. P38b's real goal is already achieved.

### P39 Reality

`ComputedIntent.deps` is always `vec![]` (initialized at `component.rs:48`, never populated). Dependency tracking was never implemented. A dead code lint exists at AST level but only warns — never removes from IR.

Motion tests: 13 tests covering ~50-60% branch coverage. 7 specific additional tests identified to reach ~90-95%.

## Decision Drivers

1. **P38b sandbox already works**: User code in iframe cannot access Tauri API. Full strip is impossible (event IPC needed). XSS chain fix provides effective defense.
2. **P38b XSS chain is a real vulnerability**: 5 connected weaknesses from postMessage through to innerHTML injection.
3. **P39 dead computed is safer than dead signal**: Computed properties have explicit bodies to analyze. Signals may be referenced implicitly. Computed elimination is a smaller, verifiable first step.
4. **P39 MVP can proceed before motion 95%**: Dead computed elimination only needs the IR tree walk, which already works. Does not depend on motion codegen coverage.
5. **Dependency tracking is a prerequisite for full P39**: `ComputedIntent.deps` must be populated before signal elimination can work correctly.

## Decision

### P38b: Fix XSS chain instead of stripping TAURI_INTERNALS

**9 concrete fixes** (prioritized):

| Priority | Fix | Location | Severity |
|----------|-----|----------|----------|
| P0 | innerHTML → textContent (signal display) | `debug-window/index.html:213-215` | CRITICAL |
| P0 | innerHTML → textContent (action list) | `debug-window/index.html:227-229` | CRITICAL |
| P0 | Add e.source + e.origin validation | `debug-window/index.html:181,277` | CRITICAL |
| P1 | postMessage origin: `'*'` → parent origin | `debug-window/index.html:176,285,319` | HIGH |
| P1 | Capability: `core:default` → `core:event:allow-emit` + `core:event:allow-listen` | `debug-window.json:7` | HIGH |
| P2 | srcdoc + string concat → Blob URL | `debug-window/index.html:324-338` | MEDIUM |
| P2 | Deduplicate HTML files (public/ + src-tauri/) | Both files | MEDIUM |

**`__TAURI_INTERNALS__` is NOT stripped** — it remains as the debug window's IPC transport. The sandbox already isolates user code from it.

### P39: Dead computed elimination as MVP; full optimization deferred

**MVP (Phase 3 Task 3.3)**:
- New `omnilang-core/src/ir/optimize.rs` module (~200 lines)
- `eliminate_dead_computeds()` — walks all IR trees, removes unreferenced computed properties
- Inserted at `analyse.rs:310` between IR merge and lint phases
- Does NOT eliminate signals, inline computeds, or track dependencies

**Deferred (post motion 95% coverage)**:
- Dead signal elimination (requires dependency tracking)
- Simple computed inlining (requires safety validation for expression purity)
- Full `ComputedIntent.deps` population

**Architectural Components Affected:**
- `poiesis/src-tauri/debug-window/index.html` — innerHTML→textContent + origin validation
- `poiesis/src-tauri/capabilities/debug-window.json` — atomic permissions
- `poiesis/public/debug-window/index.html` — sync with src-tauri copy
- `omnilang-core/src/ir/optimize.rs` — new module
- `omnilang-core/src/ir/mod.rs` — module registration
- `omnilang-core/src/analyse.rs:310` — optimization call site

## Consequences

### Positive

- **P38b**: XSS chain fully eliminated. Defense in depth: even if one layer fails, others protect. Capability surface reduced 9→2 permissions.
- **P39**: Unused computed properties removed at IR level, reducing generated JS size and runtime overhead. ~200 lines, minimal risk.

### Negative

- **P38b**: `__TAURI_INTERNALS__` remains accessible in parent window (but not sandbox). Debug window HTML duplicated until dedup fix applied.
- **P39**: Only addresses computed properties — dead signals persist. Dependency tracking still unimplemented. Full optimization requires motion 95% + `deps` infrastructure.

### Neutral

- The XSS chain exists only if an attacker can send postMessage to the debug window — requires either same-machine access or another compromised window.
- Dead computed elimination is a pure removal optimization — no behavioral changes.

## Implementation Strategy

### Blast Radius

**P38b**: Low — changes confined to a single HTML file (×2 copies) + capability JSON. No Rust/TS code changes.

**P39**: Medium — new module in IR layer + one call site in analysis pipeline. All existing tests should pass unchanged (dead computed removal is invisible to output).

### Sequencing

```
Week 1: P38b (security P0) → Phase 2 Tasks 2.1-2.3
Week 2-3: P39 MVP → Phase 3 Task 3.3
Week 4: Motion tests → Phase 4 Task 4.1 (enables full P39 signal elimination)
```

### Reversibility

- P38b: All changes are within debug window HTML — fully revertible per-fix.
- P39: Remove `optimize()` call from `analyse.rs:310`. Module can stay.

## Alternatives Considered

### P38b: Full TAURI_INTERNALS strip (original scope)
**Rejected**: Would break debug window functionality. Event IPC is required for compile-result delivery, hot-replace forwarding, and element-click relay. The sandbox already achieves the security goal.

### P38b: Replace postMessage with custom IPC protocol
**Rejected**: Over-engineering. postMessage is the standard browser API for cross-origin iframe communication. Fixing the origin validation is sufficient.

### P39: Skip MVP, go directly to full optimization
**Rejected**: Requires `ComputedIntent.deps` infrastructure and motion 95% coverage. MVP delivers value incrementally.

### P39: Eliminate dead signals before computeds
**Rejected**: Signal references are harder to definitively track (implicit in bindings, props, event handlers). Computed references are explicit in expression trees. Safer first step.

## Pragmatic Enforcer Analysis

**P38b**:
- Necessity: 8/10 — XSS chain is a real vulnerability with concrete exploit path
- Complexity: 2/10 — innerHTML→textContent is a mechanical replacement. Origin validation is 3 lines.
- Ratio: 0.25 ✅

**P39 MVP**:
- Necessity: 6/10 — Dead computed elimination has measurable runtime benefit
- Complexity: 3/10 — ~200 lines of tree-walking code. No new dependencies.
- Ratio: 0.50 ✅

**Recommendation**: ✅ Both approaches approved. P38b fixes are P0 security. P39 MVP is the minimum viable optimization — delivers value without blocking on test coverage.

## Validation

**Acceptance Criteria**:
- [ ] P38b: XSS chain eliminated (textContent, origin validation, atomic capabilities, postMessage origin)
- [ ] P38b: Debug window functionality preserved (compile-result, hot-replace, element-click all work)
- [ ] P39: `eliminate_dead_computeds` test passes
- [ ] P39: Full test suite passes unchanged (dead computed removal is invisible)
- [ ] P39: New motion tests added (7 tests for 95% branch coverage)

## References

- [Deep Technical Review 2026-07-27](../reviews/deep-review-2026-07-27.md) — Findings #6 and #7
- [Implementation Plan](../../../docs/superpowers/plans/2026-07-27-architecture-remediation.md) — Tasks 2.1-2.3, 3.3, 4.1
- [[ADR-006]] — Deferred items reclassification
