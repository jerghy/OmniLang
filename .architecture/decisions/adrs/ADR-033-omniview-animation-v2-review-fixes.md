# ADR-033: OmniView Animation v2 Review — P0 Bug Fix & ADR-032 Cleanup Completion

## Status

Implemented

## Context

A 10-specialist follow-up architecture review was conducted on 2026-07-29 to verify the post-ADR-032 animation subsystem. The prior ADR-032 scoping (Accepted, implemented) removed 21 of 33 features to achieve React-Flutter parity. The v2 review assessed whether the remaining architecture was sound and whether the scoping cleanup was complete.

The review found:

1. **One P0 correctness bug**: The parser stored full easing names as direction values (`"backIn"` → `direction: Some("backIn")`) but all codegen consumers expected suffix form (`"in"`). This caused `backIn`, `backInOut`, `elasticIn`, `elasticInOut` to silently produce `*Out` curves on all three backends (OmniView, React, Flutter).

2. **Incomplete ADR-032 cleanup**: ~200+ lines of dead code survived the scoping — dead lexer tokens, dead types (HeroDef, HeroIntent, MotionExprIR), dead pipeline fields (`path`, `pathRotate`, `hover_reverse`), dead functions (`playback_to_config`, `is_transform_prop`, `format_transform_value`), dead React codegen (`@focus` handler), and stale documentation.

3. **Flutter codegen non-determinism**: `extract_tween_begin`/`extract_tween_end` iterated `FxHashMap::values()` with no property filter, making multi-property keyframe output non-deterministic across compilations.

4. **Silent feature loss**: Several removed features (`@drag-scrub`, `@drag-momentum`, `@hover-reverse`, component-level `@hero`) were silently accepted and discarded — violating ADR-032's validation criterion requiring "clear compiler errors with migration guidance."

5. **Runtime memory leak gaps**: `__omni_scrollRemoveEntry` was called after `el.remove()` (element already disconnected), and `__omni_inview_observers` never evicted disconnected IntersectionObservers.

## Decision Drivers

- **ADR-032 compliance**: The scoping implementation must satisfy ADR-032's own validation criteria — all removed features must produce clear compiler errors, no silent data loss
- **Cross-backend correctness**: The easing normalization bug silently produced wrong animation curves on all targets — a user-visible defect with zero diagnostic
- **Build reproducibility**: Non-deterministic Flutter codegen output undermines CI caching and makes debugging unreliable
- **Codebase hygiene**: Dead code creates cognitive load, false feature promises, and ongoing maintenance tax (every new lint pass that walks `ComponentIR` needed `hero_defs: vec![]`)
- **Defense-in-depth**: String interpolation into generated JS must use `escape_literal_str` consistently, even when parser whitelists guarantee safety

## Decision

Execute a 14-task fix plan across 4 phases, implemented via Subagent-Driven Development (SDD). All changes are surgical — no new features, no architectural changes.

### Phase 1: P0 Bug Fix (1 task)

**Easing direction normalization**: Strip prefix in parser, normalize to lowercase, use `None` for the default "out" direction:
- `"backIn"` → `direction: Some("in")`, `"backOut"` → `direction: None`
- Parser produces uniform lowercase; all consumers (React, Flutter, OmniView/JS) match lowercase

**Also fixed**: React hardcoded BackIn cubic-bezier value (`0.165` → `-0.280`) to match canonical `NAMED_EASINGS` table.

### Phase 2: HIGH Priority Fixes (4 tasks)

1. **Remove dead `path`/`pathRotate`** from AST, IR, lowering, parser — users writing `path: "..."` now get parse errors instead of silent no-ops
2. **Fix Flutter non-determinism** — `extract_tween_begin`/`end` now accept `prop: &str` parameter and look up by key
3. **Add migration errors** for 11 removed features — each produces a clear parse error with migration guidance and ADR-032 reference
4. **Add `escape_literal_str`** to OmniView `easing_to_js` for defense-in-depth

### Phase 3: MEDIUM Cleanup (6 tasks)

Removed all dead code artifacts from incomplete ADR-032 scoping:
- 7 dead lexer tokens (`@timeline`, `@leave`, `@unhover`, `@focus`, `@animate`, `@driven-by`, `@hero-transition`)
- Dead types: `HeroDef`, `HeroIntent`, `MotionExprIR`
- Dead fields: `path`, `pathRotate`, `hover_reverse`, `hero_defs`
- Dead functions: `playback_to_config`, `is_transform_prop`, `format_transform_value`
- Dead React codegen: `@focus` handler block (~20 lines per component output)
- Dead snapshot files (6) and stale doc comments (3 files)

### Phase 4: MEDIUM Runtime Fixes (3 tasks)

1. **Swap cleanup order** in @for leave-animation path — `__omni_cleanupElement` now runs before `el.remove()`
2. **Refactor React `translate_easing`** to delegate to canonical `NAMED_EASINGS` table instead of hardcoding values
3. **Add IntersectionObserver cleanup** to `__omni_cleanupElement` — stores `{nid, observer}` pairs and disconnects on element removal

**Architectural Components Affected:**
- Parser: `motion.rs` (easing), `view/mod.rs` (migration errors), `top_level/motion.rs` (path removal), `top_level/component.rs` (hero error)
- Lexer: `token.rs`, `keywords.rs` (7 dead tokens removed)
- AST: `motion.rs` (path, HeroDef removed)
- IR: `types_motion.rs` (path, hover_reverse, HeroIntent, MotionExprIR removed), `types.rs` (hero_defs removed)
- Lowering: `motion.rs`, `view.rs`, `component.rs`
- OmniView codegen: `motion_config.rs` (escaping, dead functions), `motion_triggers.rs` (observer tracking), `motion.rs` (dead function), `runtime.rs` (observer cleanup), `list.rs` (cleanup order)
- React codegen: `keyframes.rs` (easing delegation), `mod.rs` (@focus removal)
- Flutter codegen: `motion.rs` (determinism, easing), `mod.rs` (path removal)
- JS runtime: snapshot and doc updates only

## Consequences

### Positive

- **Easing correctness restored**: `backIn`, `backInOut`, `elasticIn`, `elasticInOut` now produce correct curves on all backends
- **ADR-032 cleanup complete**: All dead code from pre-scoping feature set removed — zero vestigial artifacts remain
- **11 clear migration errors**: Every removed feature produces a specific error message with guidance (e.g., "@focus trigger has been removed. Use @hover and @press instead")
- **Deterministic Flutter output**: Multi-property keyframe codegen is reproducible across compilations
- **Memory leak fixes**: Observer and scroll entry cleanup prevents unbounded growth in long-lived pages
- **Single easing source of truth**: React `translate_easing` now delegates to canonical `NAMED_EASINGS` table, eliminating the 3rd copy of easing data
- **Net code reduction**: -743 lines across 64 code files
- **Zero regression**: All 830+ tests pass, clippy clean, cross-target green

### Negative

- **Merge commit history**: SDD dispatch pattern produced merge commits on the worktree branch; these are cosmetic only
- **Snapshot churn**: 5 React motion snapshots and 6 OmniView regression snapshots regenerated due to codegen changes

### Neutral

- The Vue test file (`omnilang/src/codegen/vue/tests.rs`) shows formatting noise alongside the 9 `hero_defs: vec![]` deletions — purely cosmetic
- Dead lexer tokens removed in Task 8; migration errors added in Task 4 already made them unreachable at the parser level before removal

## Implementation

Executed via SDD (Subagent-Driven Development) across 14 tasks:
- **16 commits** on branch `worktree-omniview-anim-v2-fixes`
- **64 code files** changed (+5,188 / -5,931, net -743 lines)
- **830+ tests** passing, clippy clean, cross-target green
- Each task independently reviewed (spec compliance + code quality)
- Final whole-branch review: MERGE READY verdict
- Merged to `main` on 2026-07-29

## Alternatives Considered

### Fix only the P0 bug, defer cleanup

Fix the easing direction normalization and leave dead code for later.

**Pros:** Smaller blast radius, faster to ship.
**Cons:** Dead code accumulates further, false feature promises mislead contributors, migration errors remain absent, repeated `hero_defs: vec![]` tax continues.

**Rejected:** The cleanup items are all S-effort and well-understood. Deferring creates more work later as dead code interacts with new features.

### Remove dead lexer tokens without adding migration errors

Just delete the tokens — users get generic "unexpected token" errors.

**Pros:** Even simpler, less code.
**Cons:** Terrible UX. Users migrating from pre-ADR-032 `.omni` files get cryptic errors with no guidance. ADR-032 explicitly requires migration guidance.

**Rejected:** Migration errors are the primary user-facing improvement. Generic errors would strand migration users.

### Keep `path`/`pathRotate` as "future feature" markers

Leave the fields in place as documentation of planned features.

**Pros:** Signals intent.
**Cons:** Silent data loss for users who write the syntax today. Dead fields are not documentation — they're false promises. If path animation is ever added, the IR fields will need redesign anyway.

**Rejected:** YAGNI. Dead fields that silently discard user input are the worst of both worlds.

## Validation

- [x] P0: `backIn`/`backInOut`/`elasticIn`/`elasticInOut` produce correct curves on all backends
- [x] React BackIn cubic-bezier matches canonical `NAMED_EASINGS` table (`-0.280` not `0.165`)
- [x] Flutter `extract_tween_begin`/`end` deterministic — keyed lookup by property name
- [x] 11 removed features produce clear parse errors with migration guidance
- [x] `escape_literal_str` applied to all 3 string interpolation sites in `easing_to_js`
- [x] React `translate_easing` delegates to canonical `NAMED_EASINGS` (no hardcoded values)
- [x] All dead code artifacts removed — zero `hero_defs`, `hover_reverse`, `path`, `pathRotate` references remain
- [x] `__omni_cleanupElement` runs before `el.remove()` in @for leave path
- [x] `__omni_inview_observers` entries disconnected and evicted on element removal
- [x] 830+ Rust tests pass, 18 JS tests pass, clippy clean, cross-target green
- [x] All 14 tasks independently reviewed (spec + quality)

## Pragmatic Enforcer Analysis

**Overall Decision Complexity Assessment**: This is cleanup and bug-fixing — net complexity reduction.

**Necessity Assessment**: 10/10
- The P0 bug silently produces wrong animation output on all backends with zero diagnostic
- ADR-032's own validation criteria were unmet (migration errors absent, silent data loss present)
- Dead code represented ~15% of post-scoping code surface

**Complexity Assessment**: -5/10 (net reduction)
- Added complexity: None — net -743 lines
- Maintenance burden: Lower — zero dead code, single easing source of truth
- Learning curve: Lower — consistent easing format, clear migration errors

**Recommendation**: ✅ This is the minimum fix set. No simpler alternative exists.

**Pragmatic Score**:
- Necessity: 10/10
- Complexity: -5/10 (net reduction)
- Ratio: -0.50 (negative = complexity decreases)

## References

- [ADR-032: OmniView Animation Design Scoping](./ADR-032-omniview-animation-design-scoping.md) — parent ADR, design scoping decision
- [ADR-022: Canonical Easing Table](./ADR-022-canonical-easing-table.md) — easing table deduplication
- [OmniView Animation v2 Architecture Review](../../.architecture/reviews/component-omniview-animation-v2.md) — 10-specialist review
- [v2 Fixes Implementation Plan](../../docs/superpowers/plans/2026-07-29-omniview-animation-v2-fixes.md) — 14-task plan
- [Prior Animation Review](../../.architecture/reviews/component-omniview-animation.md) — original 8-expert review (2026-07-29)
