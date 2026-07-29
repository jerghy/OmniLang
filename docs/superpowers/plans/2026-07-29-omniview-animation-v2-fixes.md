# OmniView Animation v2 Review Fixes — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the P0 easing direction normalization bug + complete ADR-032 cleanup (~200 lines dead code), based on 10-specialist v2 architecture review.

**Architecture:** Surgical fix pass — no new features, no architectural changes. The P0 bug is a parser→codegen contract gap (parser stores `"backIn"`, all consumers expect `"in"`). Cleanup removes dead AST/IR/lexer/codegen artifacts from incomplete ADR-032 scoping.

**Tech Stack:** Rust (parser, IR, codegen), TypeScript (runtime package), JS snapshot tests

**Source:** `.architecture/reviews/component-omniview-animation-v2.md`

## Global Constraints

- 837 Rust tests must pass after every task (`cargo test --workspace`)
- 18 JS tests must pass after every task (`cd packages/omni-animation && pnpm test`)
- Clippy must stay clean: `cargo clippy --workspace -- -D warnings`
- Cross-target compilation must stay green: `cargo check --target wasm32-unknown-unknown`
- No architectural changes — fix and cleanup only
- Commit format: `fix:` for bugs, `chore:` for dead code, `refactor:` for structural improvements

---

## Phase 1: P0 Fix — Easing Direction Normalization (1 task)

### Task 1: Fix Back/Elastic easing direction so all backends produce correct curves

**Root cause:** Parser stores full easing name as direction (`"backIn"`), but all 4 consumers (OmniView codegen, React codegen, Flutter codegen, JS runtime) check for suffix form (`"in"`, `"In"`, `'in'`). Result: `backIn`/`backInOut`/`elasticIn`/`elasticInOut` all fall through to `*Out` curves.

**Files:**
- Modify: `crates/omnilang-core/src/parser/expr/motion.rs:55-63`
- Modify: `omnilang/src/codegen/react/motion/keyframes.rs:127-133`
- Modify: `omnilang/src/codegen/flutter/motion.rs:95-96`

**Interfaces:**
- Produces: `EasingIR::Back { direction: Option<String> }` — direction now lowercase suffix (`"in"`, `"inout"`) or `None` for `"out"`
- Produces: `EasingIR::Elastic { direction: Option<String> }` — same convention
- Consumes: React `translate_easing` checks lowercase `"in"`/`"inout"`/fallback
- Consumes: Flutter `format_easing_flutter` checks direction for correct `Curves` constant

- [ ] **Step 1: Normalize direction in parser to lowercase suffix form**

In `crates/omnilang-core/src/parser/expr/motion.rs:55-63`, change:

```rust
// BEFORE (lines 55-63):
"backIn" | "backOut" | "backInOut" => {
    EasingExpr::Back {
        direction: Some(name.clone()),
    }
}
"elasticIn" | "elasticOut" | "elasticInOut" => {
    EasingExpr::Elastic {
        direction: Some(name.clone()),
    }
}

// AFTER:
"backIn" | "backOut" | "backInOut" => {
    let dir = name.strip_prefix("back").unwrap_or("out");
    let dir_lower = dir.to_lowercase();
    EasingExpr::Back {
        direction: if dir_lower == "out" { None } else { Some(dir_lower) },
    }
}
"elasticIn" | "elasticOut" | "elasticInOut" => {
    let dir = name.strip_prefix("elastic").unwrap_or("out");
    let dir_lower = dir.to_lowercase();
    EasingExpr::Elastic {
        direction: if dir_lower == "out" { None } else { Some(dir_lower) },
    }
}
```

This normalizes `"backIn"` → `direction: Some("in")`, `"backOut"` → `direction: None`, `"backInOut"` → `direction: Some("inout")`. Same for elastic. The `None` for "out" is correct — all consumers treat missing direction as "out".

- [ ] **Step 2: Fix React `translate_easing` to match lowercase direction**

In `omnilang/src/codegen/react/motion/keyframes.rs:127-133`, change case:

```rust
// BEFORE (lines 127-133):
EasingIR::Back { direction } => {
    let dir = direction.as_deref().unwrap_or("Out");
    match dir {
        "In" => "cubic-bezier(0.600,0.165,0.735,0.045)".to_string(),
        "InOut" => "cubic-bezier(0.680,-0.550,0.265,1.550)".to_string(),
        _ => "cubic-bezier(0.175,0.885,0.320,1.275)".to_string(),
    }
}

// AFTER:
EasingIR::Back { direction } => {
    let dir = direction.as_deref().unwrap_or("out");
    match dir {
        "in" => "cubic-bezier(0.600,-0.280,0.735,0.045)".to_string(),
        "inout" => "cubic-bezier(0.680,-0.550,0.265,1.550)".to_string(),
        _ => "cubic-bezier(0.175,0.885,0.320,1.275)".to_string(),
    }
}
```

Note: This also fixes the BackIn value bug — `0.165` → `-0.280` (was a separate hardcoded-value error that happened to be in the same function).

- [ ] **Step 3: Fix Flutter `format_easing_flutter` to check direction**

In `omnilang/src/codegen/flutter/motion.rs:95-96`, change:

```rust
// BEFORE (lines 95-96):
Some(EasingIR::Back { .. }) => "Curves.easeOutBack".to_string(),
Some(EasingIR::Elastic { .. }) => "Curves.easeOutElastic".to_string(),

// AFTER:
Some(EasingIR::Back { direction }) => {
    match direction.as_deref() {
        Some("in") => "Curves.easeInBack".to_string(),
        Some("inout") => "Curves.easeInOutBack".to_string(),
        _ => "Curves.easeOutBack".to_string(),
    }
}
Some(EasingIR::Elastic { direction }) => {
    match direction.as_deref() {
        Some("in") => "Curves.easeInElastic".to_string(),
        Some("inout") => "Curves.easeInOutElastic".to_string(),
        _ => "Curves.easeOutElastic".to_string(),
    }
}
```

- [ ] **Step 4: Verify — run tests**

```bash
cargo test --workspace  # 837 tests must pass
cd packages/omni-animation && pnpm test  # 18 tests must pass
cargo clippy --workspace -- -D warnings
```

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/parser/expr/motion.rs \
        omnilang/src/codegen/react/motion/keyframes.rs \
        omnilang/src/codegen/flutter/motion.rs
git commit -m "fix(animation): normalize Back/Elastic easing direction to lowercase suffix

Parser stored full name ('backIn') as direction, but all codegen
backends and JS runtime expected suffix form ('in'). This caused
backIn, backInOut, elasticIn, elasticInOut to silently produce
*Out curves on all targets.

- Parser: strip prefix, normalize to lowercase, None for 'out'
- React: match lowercase 'in'/'inout', fix BackIn cubic-bezier
  value (0.165→-0.280) to match canonical NAMED_EASINGS table
- Flutter: check direction for correct Curves.easeInBack, etc.

10-specialist architecture review v2 finding (P0)"
```

---

## Phase 2: HIGH Priority Fixes (5 tasks)

### Task 2: Remove dead `path` / `pathRotate` fields from entire pipeline (H1)

**Root cause:** These fields are parsed, lowered into IR, but NO codegen backend reads them. Users writing `path: "M10 10"` get silent no-ops — violates ADR-032's "no silent feature loss" principle.

**Files:**
- Modify: `crates/omnilang-core/src/parser/ast/motion.rs:34-38` — remove fields from `KeyframeBody`
- Modify: `crates/omnilang-core/src/ir/types_motion.rs:33-38` — remove fields from `KeyframeIR`
- Modify: `crates/omnilang-core/src/ir/lower/motion.rs:72-73` — remove lowering pass-through
- Modify: `crates/omnilang-core/src/parser/top_level/motion.rs:94-156` — remove parser for `path`/`pathRotate`
- Modify: `omnilang/src/codegen/flutter/motion.rs:11-23` — remove `comp_has_path_motion`/`motion_has_path`

**Interfaces:**
- Removes: `KeyframeBody.path: Option<String>`, `KeyframeBody.path_rotate: Option<bool>`
- Removes: `KeyframeIR.path: Option<String>`, `KeyframeIR.path_rotate: Option<bool>`

- [ ] **Step 1: Remove fields from AST type**

In `crates/omnilang-core/src/parser/ast/motion.rs:29-38`, remove `path` and `path_rotate`:

```rust
// BEFORE:
pub struct KeyframeBody {
    pub stops: Vec<KeyframeStop>,
    pub over: Expr,
    pub ease: Option<EasingExpr>,
    pub playback: Vec<PlaybackProp>,
    /// Path for motion path animation: path: "M10 10 ..."
    pub path: Option<String>,
    /// Whether to rotate element along the path
    pub path_rotate: Option<bool>,
}

// AFTER:
pub struct KeyframeBody {
    pub stops: Vec<KeyframeStop>,
    pub over: Expr,
    pub ease: Option<EasingExpr>,
    pub playback: Vec<PlaybackProp>,
}
```

- [ ] **Step 2: Remove fields from IR type**

In `crates/omnilang-core/src/ir/types_motion.rs:27-40`, remove `path` and `path_rotate`:

```rust
// Remove lines 33-37 (path and path_rotate fields with their serde defaults and source_spans)
// BEFORE lines 33-37:
    /// Path for motion path animation
    #[serde(default)]
    pub path: Option<String>,
    /// Whether to rotate element along the path
    #[serde(default)]
    pub path_rotate: Option<bool>,

// AFTER: Remove these 5 lines entirely
```

- [ ] **Step 3: Remove lowering pass-through**

In `crates/omnilang-core/src/ir/lower/motion.rs:67-75`, remove `path` and `path_rotate` from `KeyframeIR` construction:

```rust
// Remove lines 72-73:
        path: kf.path.clone(),
        path_rotate: kf.path_rotate,
```

- [ ] **Step 4: Remove parser for `path`/`pathRotate` keywords**

In `crates/omnilang-core/src/parser/top_level/motion.rs:94-156`, remove the two `path`/`pathRotate` parsing loops (they appear twice — once before playback props, once after). Replace with an empty block or remove the variable declarations. Specifically:

Remove lines 94-124 (first pass: parse path/pathRotate before playback). Remove lines 128-156 (second pass: parse path/pathRotate after playback). Remove `path` and `path_rotate` variable declarations at lines 95-96.

The `KeyframeBody` constructor at lines 158-165 becomes:

```rust
Ok(KeyframeBody {
    stops,
    over,
    ease,
    playback,
})
```

- [ ] **Step 5: Remove Flutter dead path-check functions**

In `omnilang/src/codegen/flutter/motion.rs:11-23`, remove `comp_has_path_motion` and `motion_has_path`:

```rust
// Remove lines 11-23 entirely:
pub(crate) fn comp_has_path_motion(ir: &IRProgram) -> bool { ... }
pub(crate) fn motion_has_path(motion: &MotionIntent) -> bool { ... }
```

Also check `omnilang/src/codegen/flutter/mod.rs` for callers of `comp_has_path_motion` (likely used for `dart:ui` import guard) and remove those references.

- [ ] **Step 6: Fix any compile errors from field removal**

Search for all references to `kf.path`, `kf.path_rotate`, `KeyframeIR { path:`, and fix them (typically in test fixtures).

```bash
grep -rn "\.path" --include="*.rs" crates/omnilang-core/src/ om nilang/src/codegen/ | grep -v "target/"
grep -rn "path_rotate" --include="*.rs" crates/ om nilang/
```

- [ ] **Step 7: Verify**

```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "chore(animation): remove dead path/pathRotate fields from pipeline

These fields were parsed and lowered into IR but no codegen backend
ever emitted path animation code. Users writing path: '...' got
silent no-ops — violates ADR-032's no-silent-feature-loss principle.

Removes ~50 lines across 5 files: AST, IR, lowering, parser, Flutter
codegen. Emit parse error if path/pathRotate keywords are written."
```

---

### Task 3: Fix Flutter `extract_tween_begin`/`end` non-determinism (H2)

**Root cause:** Both functions iterate `FxHashMap::values()` and return first match. When a keyframe stop has multiple properties, the returned value depends on HashMap iteration order (non-deterministic due to hash randomization).

**Files:**
- Modify: `omnilang/src/codegen/flutter/motion.rs:39-71` — add `prop: &str` parameter
- Modify: `omnilang/src/codegen/flutter/mod.rs:220-224` — update callers

**Interfaces:**
- Old: `extract_tween_begin(kf: &KeyframeIR) -> String`
- New: `extract_tween_begin(kf: &KeyframeIR, prop: &str) -> String`
- Old: `extract_tween_end(kf: &KeyframeIR) -> String`
- New: `extract_tween_end(kf: &KeyframeIR, prop: &str) -> String`

- [ ] **Step 1: Add property name parameter to both functions**

In `omnilang/src/codegen/flutter/motion.rs:39-71`:

```rust
// BEFORE (lines 39-55):
pub(crate) fn extract_tween_begin(kf: &KeyframeIR) -> String {
    for stop in &kf.stops {
        if stop.position == 0.0 {
            for value in stop.properties.values() {
                if let ExprIR::LiteralExpr {
                    value: serde_json::Value::Number(n),
                } = value
                {
                    return n.to_string();
                }
            }
        }
    }
    "0.0".to_string()
}

// AFTER:
pub(crate) fn extract_tween_begin(kf: &KeyframeIR, prop: &str) -> String {
    for stop in &kf.stops {
        if stop.position == 0.0 {
            if let Some(value) = stop.properties.get(prop) {
                if let ExprIR::LiteralExpr {
                    value: serde_json::Value::Number(n),
                } = value
                {
                    return n.to_string();
                }
            }
        }
    }
    "0.0".to_string()
}
```

Same pattern for `extract_tween_end` (lines 57-71) — add `prop: &str` parameter and use `stop.properties.get(prop)`.

- [ ] **Step 2: Update callers in `flutter/mod.rs`**

In `omnilang/src/codegen/flutter/mod.rs:220-224`, update:

```rust
// BEFORE (lines 222-223):
let begin = extract_tween_begin(kf);
let end = extract_tween_end(kf);

// AFTER — use the first animated property:
let properties = extract_keyframe_properties(kf);
let primary_prop = properties.first().map(|s| s.as_str()).unwrap_or("opacity");
let begin = extract_tween_begin(kf, primary_prop);
let end = extract_tween_end(kf, primary_prop);
```

- [ ] **Step 3: Verify**

```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/flutter/motion.rs omnilang/src/codegen/flutter/mod.rs
git commit -m "fix(flutter): make extract_tween_begin/end deterministic via property name param

Both functions iterated FxHashMap::values() and returned first match,
making Flutter codegen output non-deterministic for multi-property
keyframes. Now accept a prop: &str parameter and look up by key."
```

---

### Task 4: Add migration errors for silently-dropped removed features (H3)

**Root cause:** `@drag-scrub`, `@drag-momentum`, `@hover-reverse`, `@hero` (component-level) are parsed/accepted but silently discarded by the lowerer or ignored by all backends. ADR-032 requires "clear compiler errors with migration guidance."

**Files:**
- Modify: `crates/omnilang-core/src/parser/top_level/component.rs:97` — error on `@hero` in component context
- Modify: `crates/omnilang-core/src/parser/view/mod.rs:560-566` — error on removed trigger tokens
- Modify: `crates/omnilang-core/src/ir/lower/view.rs:522-523` — error instead of silent drop for drag-scrub/momentum
- Modify: `crates/omnilang-core/src/parser/view/mod.rs:485-497` — error on `@hover-reverse`

- [ ] **Step 1: Add parse error for `@hero` in component context**

In `crates/omnilang-core/src/parser/top_level/component.rs:97` (the `TokenKind::AtHero` match arm inside `parse_component_body`), change from parsing HeroDef to returning an error:

```rust
TokenKind::AtHero => {
    self.advance();
    return Err(self.error(
        "@hero transitions are no longer supported. \
         Use @enter or @in-view triggers on individual elements instead. \
         See ADR-032."
    ));
}
```

- [ ] **Step 2: Add migration errors for removed trigger tokens**

In `crates/omnilang-core/src/parser/view/mod.rs:560-566`, add explicit match arms before the catch-all `_ =>`:

```rust
// Add before the catch-all arm:
TokenKind::AtFocus => {
    self.advance();
    return Err(self.error(
        "@focus trigger has been removed. Use @hover and @press instead. See ADR-032."
    ));
}
TokenKind::AtLeave | TokenKind::AtUnhover => {
    self.advance();
    return Err(self.error(
        "@leave/@unhover triggers have been removed. Use @enter with reverse \
         direction instead. See ADR-032."
    ));
}
TokenKind::AtTimeline => {
    self.advance();
    return Err(self.error(
        "@timeline trigger has been removed. Use individual @enter triggers \
         with delay values instead. See ADR-032."
    ));
}
TokenKind::AtAnimate => {
    self.advance();
    return Err(self.error(
        "@animate trigger has been removed. Use @enter trigger instead. See ADR-032."
    ));
}
TokenKind::AtDrivenBy => {
    self.advance();
    return Err(self.error(
        "@driven-by trigger has been removed. Use signal bindings in keyframe \
         properties instead. See ADR-032."
    ));
}
```

- [ ] **Step 3: Add migration errors for @drag-scrub/@drag-momentum**

In `crates/omnilang-core/src/parser/view/mod.rs`, find where `@drag-scrub` and `@drag-momentum` are parsed (around lines 541-554) and add explicit error arms:

```rust
// Replace the parsing arms with error messages:
TokenKind::AtDragScrub => {
    self.advance();
    return Err(self.error(
        "@drag-scrub has been removed. Drag animations now use the element's \
         drag behavior configuration directly. See ADR-032."
    ));
}
TokenKind::AtDragMomentum => {
    self.advance();
    return Err(self.error(
        "@drag-momentum has been removed. See ADR-032."
    ));
}
```

Also remove the silent-drop comment at `ir/lower/view.rs:522-523`:
```rust
// Remove comment and silent-drop lines:
// "no longer supported -- silently ignore"
```

- [ ] **Step 4: Add migration error for @hover-reverse**

In `crates/omnilang-core/src/parser/view/mod.rs:485-497`, replace the `@hover-reverse` parsing with an error:

```rust
TokenKind::AtHoverReverse => {
    self.advance();
    return Err(self.error(
        "@hover-reverse has been removed. Use separate @hover triggers with \
         different named motions for enter and leave. See ADR-032."
    ));
}
```

- [ ] **Step 5: Verify**

```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

- [ ] **Step 6: Commit**

```bash
git add crates/omnilang-core/src/parser/ crates/omnilang-core/src/ir/lower/view.rs
git commit -m "fix(animation): add migration errors for silently-dropped removed features

@drag-scrub, @drag-momentum, @hover-reverse, component-level @hero,
and 6 removed trigger tokens (@focus, @leave, @unhover, @timeline,
@animate, @driven-by) now produce clear parse errors with migration
guidance instead of being silently accepted and discarded.

Per ADR-032 validation criteria: 'All removed features must produce
clear compiler errors with migration guidance.'"
```

---

### Task 5: Add `escape_literal_str` to OmniView `easing_to_js` (H5)

**Root cause:** `easing_to_js` in `motion_config.rs` inserts easing name/direction strings into JS string literals without escaping. Currently safe because parser whitelists values, but a defense-in-depth gap.

**Files:**
- Modify: `omnilang/src/codegen/omniview/motion_config.rs:18,28,33`

- [ ] **Step 1: Add `escape_literal_str` calls**

In `omnilang/src/codegen/omniview/motion_config.rs:16-38`:

```rust
// BEFORE line 18:
EasingIR::Named(name) => format!("\"{}\"", name),

// AFTER:
EasingIR::Named(name) => format!("\"{}\"", escape_literal_str(name, '"')),
```

```rust
// BEFORE lines 28-30:
format!(
    "{{type:\"back\",direction:\"{}\"}}",
    dir
)

// AFTER:
format!(
    "{{type:\"back\",direction:\"{}\"}}",
    escape_literal_str(dir, '"')
)
```

```rust
// BEFORE lines 34-36:
format!(
    "{{type:\"elastic\",direction:\"{}\"}}",
    dir
)

// AFTER:
format!(
    "{{type:\"elastic\",direction:\"{}\"}}",
    escape_literal_str(dir, '"')
)
```

- [ ] **Step 2: Verify**

```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

- [ ] **Step 3: Commit**

```bash
git add omnilang/src/codegen/omniview/motion_config.rs
git commit -m "fix(security): add escape_literal_str to easing_to_js in OmniView codegen

Defense-in-depth: easing name and direction strings are currently
safe (parser whitelist), but add escaping to prevent injection if
a future code path feeds user-controlled strings into EasingIR."
```

---

## Phase 3: MEDIUM — ADR-032 Cleanup (6 tasks)

### Task 6: Remove dead @focus JS codegen from React backend (M1)

**Files:**
- Modify: `omnilang/src/codegen/react/motion/mod.rs:127-165`

- [ ] **Step 1: Remove @focus setup (lines 127-144)**

Delete lines 127-144 (from `// @focus: focus/blur animations` through the closing `});`).

- [ ] **Step 2: Remove @focus cleanup (lines 160-165)**

Delete lines 160-165 (the `focusHandlers.forEach(...)` cleanup block).

- [ ] **Step 3: Update snapshot tests**

```bash
# Regenerate snapshots after codegen change
cargo test --workspace
# If snapshot tests fail, review and accept the new snapshots
```

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/react/motion/mod.rs
git commit -m "chore(react): remove dead @focus JS codegen from React motion backend

@focus was removed by ADR-032 but emit_motion_effects_tsx still
emitted ~20 lines of unreachable focus handler setup/cleanup JS.
querySelectorAll('[data-motion-focus]') can never match any element
since no TriggerKind::Focus exists and no data-motion-focus
attribute is ever emitted.

ADR-032 parked finding M1."
```

---

### Task 7: Remove dead functions (M3: `playback_to_config`, `is_transform_prop`, `format_transform_value`)

**Files:**
- Modify: `omnilang/src/codegen/omniview/motion_config.rs:110-170` — remove 2 functions
- Modify: `omnilang/src/codegen/omniview/motion.rs:25-34` — remove 1 function
- Modify: `omnilang/src/codegen/omniview/tests_regression/motion.rs` — remove tests for these functions

- [ ] **Step 1: Remove `playback_to_config` and `is_transform_prop`**

In `motion_config.rs`, delete lines 109-170 (the `playback_to_config` method with its `#[allow(dead_code)]`, and `is_transform_prop` with its `#[allow(dead_code)]`).

- [ ] **Step 2: Remove `format_transform_value`**

In `motion.rs`, delete lines 25-34 (the `format_transform_value` method with its `#[allow(dead_code)]`).

- [ ] **Step 3: Remove corresponding tests**

In `omnilang/src/codegen/omniview/tests_regression/motion.rs`, remove tests that call these dead functions. Search for test functions referencing `playback_to_config`, `is_transform_prop`, `format_transform_value` and remove them.

- [ ] **Step 4: Verify**

```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/omniview/motion_config.rs \
        omnilang/src/codegen/omniview/motion.rs \
        omnilang/src/codegen/omniview/tests_regression/motion.rs
git commit -m "chore(animation): remove dead playback_to_config, is_transform_prop, format_transform_value

Three functions with #[allow(dead_code)] survived ADR-032 scoping
with zero production callers. Their tests tested nothing in production.
Remove all three functions and their tests (~170 lines total)."
```

---

### Task 8: Remove dead `MotionExprIR` enum + dead lexer tokens (M3 + M4)

**Files:**
- Modify: `crates/omnilang-core/src/ir/types_motion.rs:100-104` — remove `MotionExprIR`
- Modify: `crates/omnilang-core/src/ir/mod.rs:46` — remove re-export
- Modify: `crates/omnilang-core/src/lexer/token.rs:80-92` — remove 7 dead At* tokens
- Modify: `crates/omnilang-core/src/lexer/keywords.rs:95-107` — remove keyword mappings
- Modify: `crates/omnilang-core/src/lexer/mod.rs` — remove dead token tests

- [ ] **Step 1: Remove `MotionExprIR`**

In `ir/types_motion.rs:100-104`, delete:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum MotionExprIR {
    Ref(String),
    Inline(MotionBodyIR),
}
```

In `ir/mod.rs`, remove the re-export of `MotionExprIR`.

- [ ] **Step 2: Remove 7 dead lexer tokens**

In `lexer/token.rs`, remove these variants from `TokenKind`:
- `AtTimeline` (line ~80)
- `AtLeave` (line ~82)
- `AtUnhover` (line ~84)
- `AtFocus` (line ~86)
- `AtAnimate` (line ~87)
- `AtDrivenBy` (line ~92)
- `AtHeroTransition` (line ~109)

In `lexer/keywords.rs`, remove the corresponding keyword mappings (lines ~95-107, ~125).

- [ ] **Step 3: Clean up `style.rs` text-content allowlist**

In `crates/omnilang-core/src/parser/view/style.rs:586-590`, remove the dead token variants from `is_text_content_token`:

```rust
// Remove: TokenKind::AtFocus, TokenKind::AtLeave, TokenKind::AtUnhover,
// TokenKind::AtAnimate, TokenKind::AtTimeline, TokenKind::AtDrivenBy
```

- [ ] **Step 4: Remove dead token tests from lexer/mod.rs**

Search for and remove test cases that assert recognition of the removed tokens.

- [ ] **Step 5: Verify**

```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

- [ ] **Step 6: Commit**

```bash
git add crates/omnilang-core/src/
git commit -m "chore(animation): remove dead MotionExprIR and 7 removed-trigger lexer tokens

MotionExprIR had zero construction or match sites. 7 At* token
variants (AtTimeline, AtLeave, AtUnhover, AtFocus, AtAnimate,
AtDrivenBy, AtHeroTransition) were tokenized but never consumed
by any parser after ADR-032. Removes ~60 lines of dead code.

ADR-032 parked findings M2 + new findings from v2 review."
```

---

### Task 9: Clean up dead HeroDef parsing chain (M5)

**Files:**
- Modify: `crates/omnilang-core/src/parser/top_level/component.rs:97-138` — HeroDef parsing already blocked by Task 4 error
- Modify: `crates/omnilang-core/src/parser/ast/motion.rs:104-110` — remove `HeroDef` type
- Modify: `crates/omnilang-core/src/parser/ast/view.rs:22` — remove `hero_defs` from `ComponentDef`
- Modify: `crates/omnilang-core/src/ir/types_motion.rs:107-114` — remove `HeroIntent` type
- Modify: `crates/omnilang-core/src/ir/types.rs:741` — remove `hero_defs` from `ComponentIR`
- Modify: `crates/omnilang-core/src/ir/lower/component.rs:266` — remove `hero_defs: vec![]`
- Modify: `crates/omnilang-core/src/ir/optimize.rs:355-357` — remove dead hero walk

- [ ] **Step 1: Remove types**

Delete `HeroDef` from `parser/ast/motion.rs:104-110`.
Delete `HeroIntent` from `ir/types_motion.rs:107-114`.
Remove `hero_defs` field from `ComponentDef` in `parser/ast/view.rs`.
Remove `hero_defs` field from `ComponentIR` in `ir/types.rs`.

- [ ] **Step 2: Remove lowering and optimizer references**

Remove `hero_defs: vec![]` from `ir/lower/component.rs:266`.
Remove the hero walk at `ir/optimize.rs:355-357`.

- [ ] **Step 3: Fix all constructors**

Search for every `hero_defs:` in the codebase (constructor sites, lint files, test fixtures) and remove the field. Expect ~11+ files to touch.

```bash
grep -rn "hero_defs" --include="*.rs" crates/ om nilang/
```

- [ ] **Step 4: Verify**

```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/
git commit -m "chore(animation): remove dead HeroDef/HeroIntent types and parsing chain

HeroDef was parsed at component level but lowerer always produced
hero_defs: vec![]. Remove ~40 lines of dead parsing + 2 dead types
+ 11+ constructor stubs. @hero in component context now produces
a migration error (see Task 4)."
```

---

### Task 10: Remove dead `hover_reverse` from IR and lowering (M6)

**Files:**
- Modify: `crates/omnilang-core/src/ir/types_motion.rs:149` — remove `hover_reverse` from `MotionBehavior`
- Modify: `crates/omnilang-core/src/ir/lower/view.rs:232,485,576,584` — remove lowering

- [ ] **Step 1: Remove field from IR type**

In `ir/types_motion.rs:149`, remove:
```rust
pub hover_reverse: Option<String>,
```

- [ ] **Step 2: Remove from lowering**

In `ir/lower/view.rs`:
- Line 232: remove `let mut hover_reverse: Option<String> = None;`
- Line 485: remove `hover_reverse = Some(s.clone());`
- Line 576: remove `|| hover_reverse.is_some()` from the condition
- Line 584: remove `hover_reverse,` from the constructor

- [ ] **Step 3: Remove lexer tokens for `@hover-reverse`**

In `lexer/token.rs` and `keywords.rs`, remove `AtHoverReverse` and `HoverReverse` keyword mapping (these are now dead since Task 4 already errors on them).

- [ ] **Step 4: Verify**

```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

- [ ] **Step 5: Commit**

```bash
git add crates/omnilang-core/src/
git commit -m "chore(animation): remove dead hover_reverse from IR and lowering

hover_reverse was parsed and lowered into MotionBehavior but no
codegen backend read it. Removed from IR type, lowering, and lexer.
@hover-reverse in .omni source now produces a migration error (Task 4)."
```

---

### Task 11: Clean up stale doc comments and dead snapshot files (M10 + M11)

**Files:**
- Modify: `omnilang/src/codegen/omniview/motion_triggers.rs:7` — update doc comment
- Modify: `omnilang/src/codegen/omniview/view/element.rs:355` — update doc comment
- Modify: `omnilang/src/codegen/react/component/motion_effects.rs:12` — update doc comment
- Delete: 6 dead `.snap` files in `omnilang/src/codegen/omniview/tests_regression/snapshots/`
- Delete/Update: 4 dead golden `.omni` fixture files in `tests/fixtures/golden/`

- [ ] **Step 1: Update doc comments**

In `motion_triggers.rs:7`, change from:
```
/// Used by hover, press, focus, unhover, leave, animate, timeline and their
/// inline variants, which all follow the common pattern:
```
To:
```
/// Used by hover and press (and their inline variants), which follow
/// the common pattern:
```

In `view/element.rs:355`, change from mentioning `@focus, @leave` to mentioning only the 7 active triggers.

In `motion_effects.rs:12`, remove `@focus` mention.

- [ ] **Step 2: Remove dead snapshot files**

Delete these 6 unreferenced snapshot files:
```
omnilang/src/codegen/omniview/tests_regression/snapshots/
  omnilang__codegen__omniview__tests_regression__motion__emit_motion_js_callback_only.snap
  omnilang__codegen__omniview__tests_regression__motion__emit_motion_js_timeline.snap
  omnilang__codegen__omniview__tests_regression__motion__timeline_with_items.snap
  omnilang__codegen__omniview__tests_regression__motion__emit_states_js_reserved.snap
  omnilang__codegen__omniview__tests_regression__motion__driven_by.snap
  omnilang__codegen__omniview__tests_regression__motion__stagger_children.snap
```

- [ ] **Step 3: Remove/update dead golden fixture files**

Remove or update these golden fixture files that reference removed features:
```
tests/fixtures/golden/test/projects/learn-omniview/src/pages/HeroTransitionDemo.omni
tests/fixtures/golden/test/projects/learn-omniview/src/pages/TimelineMotionDemo.omni
tests/fixtures/golden/test/projects/learn-omniview/src/pages/MotionTriggersDemo.omni
tests/fixtures/golden/test/projects/learn-omniview/src/components/PathTimelineDemo.omni
```

For `MotionTriggersDemo.omni`, update to use only retained triggers (remove @unhover, @focus, @animate usages). For the others, either update to equivalent retained-feature syntax or remove entirely if the demo concept is no longer applicable.

- [ ] **Step 4: Verify**

```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

- [ ] **Step 5: Commit**

```bash
git add omnilang/src/codegen/ tests/fixtures/golden/
git commit -m "chore(animation): update stale doc comments and remove dead snapshots

Update doc comments in motion_triggers.rs, element.rs, and
motion_effects.rs to reflect 7 retained triggers. Remove 6
unreferenced snapshot files and update 4 golden fixture files
that referenced removed features.

ADR-032 parked finding M3 + new findings from v2 review."
```

---

## Phase 4: MEDIUM — Runtime Fixes (3 tasks)

### Task 12: Fix `__omni_scrollRemoveEntry` cleanup order (M7)

**Root cause:** In @for leave-animation path, `el.remove()` is called before `__omni_cleanupElement`. `__omni_scrollRemoveEntry` calls `__omni_el(nid)` which finds the element is disconnected and returns null, short-circuiting the cleanup.

**Files:**
- Modify: `omnilang/src/codegen/omniview/list.rs:353,357` — swap call order

- [ ] **Step 1: Swap cleanup order**

In `list.rs`, find the @for leave-animation cleanup section. Change:

```js
// BEFORE (conceptual):
el.remove();
__omni_cleanupElement(nid);

// AFTER:
__omni_cleanupElement(nid);
el.remove();
```

The exact change depends on how the JS is generated. Search for `el.remove()` near `__omni_cleanupElement` calls in the @for path and swap the order.

- [ ] **Step 2: Verify**

```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

- [ ] **Step 3: Commit**

```bash
git add omnilang/src/codegen/omniview/list.rs
git commit -m "fix(omniview): swap cleanup order in @for leave-animation path

__omni_scrollRemoveEntry was called after el.remove(), so the
element was already disconnected from DOM. __omni_el(nid) returned
null and the scroll entry was never cleaned up.

Move __omni_cleanupElement before el.remove() so the element is
still in DOM when scroll entries are evicted."
```

---

### Task 13: Refactor React `translate_easing` to delegate to canonical table (M8)

**Root cause:** React `translate_easing` hardcodes cubic-bezier values for Back/Elastic structural variants instead of delegating to `NAMED_EASINGS`. This is the architectural root cause of the BackIn value bug (fixed in Task 1) and creates ongoing drift risk.

**Files:**
- Modify: `omnilang/src/codegen/react/motion/keyframes.rs:121-157`

- [ ] **Step 1: Rewrite `translate_easing` to delegate**

```rust
fn translate_easing(ease: &EasingIR) -> String {
    match ease {
        EasingIR::Named(name) => translate_named_easing(name),
        EasingIR::CubicBezier { x1, y1, x2, y2 } => {
            format!("cubic-bezier({},{},{},{})", x1, y1, x2, y2)
        }
        EasingIR::Back { direction } => {
            let dir = direction.as_deref().unwrap_or("out");
            let name = format!("back{}", {
                // Capitalize first letter: "in" -> "In", "inout" -> "InOut", "out" -> "Out"
                let mut c = dir.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                }
            });
            omnilang_core::codegen::shared::easing::resolve_easing(&name)
                .unwrap_or("cubic-bezier(0.175,0.885,0.320,1.275)") // fallback = backOut
                .to_string()
        }
        EasingIR::Elastic { direction } => {
            let dir = direction.as_deref().unwrap_or("out");
            let name = format!("elastic{}", {
                let mut c = dir.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                }
            });
            omnilang_core::codegen::shared::easing::resolve_easing(&name)
                .unwrap_or("cubic-bezier(0.250,0.460,0.450,0.940)") // fallback = elasticOut
                .to_string()
        }
    }
}
```

Now `translate_easing` for Back/Elastic constructs the canonical easing name (`"backIn"`, `"elasticOut"`, etc.) and delegates to the shared `NAMED_EASINGS` table via `resolve_easing()`. This eliminates the hardcoded cubic-bezier values entirely and ensures React output matches the canonical table.

- [ ] **Step 2: Verify**

```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

- [ ] **Step 3: Commit**

```bash
git add omnilang/src/codegen/react/motion/keyframes.rs
git commit -m "refactor(react): delegate translate_easing to canonical NAMED_EASINGS table

React translate_easing hardcoded cubic-bezier values for Back/Elastic
structural variants, bypassing the shared NAMED_EASINGS canonical
table. This caused the BackIn value bug (0.165 vs -0.280, fixed in
Task 1) and created ongoing drift risk.

Now constructs canonical easing name (e.g. 'backIn') and delegates
to omnilang_core::codegen::shared::easing::resolve_easing(). This
eliminates the 3rd copy of easing data across the codebase and
ensures React output matches the canonical table."
```

---

### Task 14: Fix `__omni_inview_observers` array leak (M9)

**Root cause:** IntersectionObservers are pushed to `__omni_inview_observers` but never disconnected or evicted, even after the observed element is destroyed.

**Files:**
- Modify: `omnilang/src/codegen/omniview/motion_triggers.rs:106-107` — store nid with observer
- Modify: `omnilang/src/codegen/omniview/runtime.rs:263-275` — cleanup observers

- [ ] **Step 1: Store nid alongside observer**

In `motion_triggers.rs:106-107`, change from:
```js
window.__omni_inview_observers = window.__omni_inview_observers || [];
__omni_inview_observers.push(observer);
```
To:
```js
window.__omni_inview_observers = window.__omni_inview_observers || [];
__omni_inview_observers.push({ nid: "{nid}", observer: observer });
```

- [ ] **Step 2: Add observer cleanup to `__omni_cleanupElement`**

In `runtime.rs:263-275`, add observer disconnection after the animation cleanup:

```js
function __omni_cleanupElement(nid) {
  if (!nid) return;
  var prefix = nid + "_";
  __omni_animations.forEach(function(anim, key) {
    if (key.startsWith(prefix)) {
      try { anim.cancel(); } catch(_) {}
      __omni_animations.delete(key);
    }
  });
  if (typeof __omni_scrollRemoveEntry === 'function') {
    __omni_scrollRemoveEntry(nid);
  }
  // Clean up InView observers for this element
  if (window.__omni_inview_observers) {
    window.__omni_inview_observers = window.__omni_inview_observers.filter(function(entry) {
      if (entry.nid === nid) {
        try { entry.observer.disconnect(); } catch(_) {}
        return false;
      }
      return true;
    });
  }
}
```

- [ ] **Step 3: Verify**

```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

- [ ] **Step 4: Commit**

```bash
git add omnilang/src/codegen/omniview/motion_triggers.rs \
        omnilang/src/codegen/omniview/runtime.rs
git commit -m "fix(omniview): cleanup IntersectionObservers in __omni_cleanupElement

InView observers were pushed to __omni_inview_observers array but
never disconnected, causing observer leak on element removal. Now
store {nid, observer} pairs and disconnect+evict in cleanupElement.

Also fixes the __omni_el._cache stale entry retention."
```

---

## Summary

| Phase | Tasks | Effort | Impact |
|-------|-------|--------|--------|
| 1: P0 | 1 | 30 min | Fixes `backIn`/`elasticIn` producing wrong curves on all backends |
| 2: HIGH | 4 (Tasks 2-5) | 2 hours | Removes ~70 lines dead path fields, fixes Flutter non-determinism, adds migration errors, defense-in-depth escaping |
| 3: MEDIUM Cleanup | 6 (Tasks 6-11) | 3 hours | Removes ~200 lines dead code, dead tokens, dead types, stale docs, dead snapshots |
| 4: MEDIUM Runtime | 3 (Tasks 12-14) | 2 hours | Fixes cleanup order bug, refactors easing delegation, fixes observer leak |

**Total**: 14 tasks, ~7.5 hours, ~300 lines net reduction

### Execution Order

Tasks within each phase are independent and can run in parallel. Phases must run sequentially:

```
Phase 1 (P0) → Phase 2 (HIGH) → Phase 3 (MEDIUM cleanup) → Phase 4 (MEDIUM runtime)
                                      ↓
                              Phase 3 tasks 6-11 can run in parallel
                              Phase 4 tasks 12-14 can run in parallel
```

### Verification Gate After Each Phase

```bash
cargo test --workspace          # Must be 837+ tests passing
cd packages/omni-animation && pnpm test  # Must be 18 tests passing
cargo clippy --workspace -- -D warnings  # Must be clean
cargo check --target wasm32-unknown-unknown  # Cross-target must be green
```
