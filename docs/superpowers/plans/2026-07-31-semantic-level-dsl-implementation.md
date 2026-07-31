# Semantic-Level DSL Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rewrite OmniLang compiler to use semantic-level code generation — .omni source expresses intent, compiler generates idiomatic platform-native code for React (Zustand v5 + React 19), Flutter (Riverpod 3.0), and OmniView (updated debug runtime).

**Architecture:** Five-layer implementation: lexer/parser changes (new syntax forms) → IR type changes (foundation) → semantic passes (intent recognition) → codegen rewrites for all P0 targets (idiomatic output) → demo migration + CI gates. React and Flutter codegen are independent after shared layer and can be parallelized. OmniView debug runtime must also be updated to match new semantics.

**Tech Stack:** Rust (compiler), TypeScript/React 19/Zustand v5 (React target), Dart/Flutter/Riverpod 3.0 (Flutter target), JS/DOM (OmniView debug target)

**Spec:** `docs/superpowers/specs/2026-07-31-semantic-level-dsl-syntax-spec.md`

**Note on parallelism:** Tasks marked `[∥]` can run in parallel. Tasks without the marker are sequential dependencies.

---

## File Structure Map

```
crates/omnilang-core/src/
├── lexer/
│   └── keywords.rs           ← Rename trigger tokens: @inView→@in-view, @dragHorizontal→@drag-horizontal
├── parser/
│   ├── top_level/motion.rs   ← over:/easing: with colon+unit, steps() positional, keyframe prop whitelist
│   ├── view/
│   │   ├── element.rs        ← semantic binding attrs (focus, scroll-x, scroll-y, value) → RefBinding prop
│   │   ├── mod.rs            ← @slot name: Type { fallback } — parse type constraint
│   │   └── directives.rs     ← @if mode: colon separator, expr not string; @for [scroll] hint
│   └── ast/
│       ├── view.rs           ← ElementProp::RefBinding { name, signal }, SlotDirective type_constraint field
│       └── motion.rs         ← motion_param with colon, easing_expr, stop_property whitelist
├── ir/
│   ├── types.rs              ← RefOpIR enum, ViewTree::Slot struct, ElementIR.semantic_bindings, PlaybackIR.paused del
│   ├── types_motion.rs       ← TriggerKind::Focus, del paused_override / @paused attr
│   └── lower/
│       ├── view.rs           ← Slot lowering (preserve fallback), semantic binding lowering, ref binding lowering
│       ├── lower_inline_motions.rs  ← NEW: inline motion body → named MotionIntent
│       └── mod.rs            ← Wire in all new passes + semantic binding recognition
├── semantic/
│   ├── validate_ref_ops.rs   ← NEW: MethodCall/FieldAccess on ref → RefOpExpr / compile error
│   └── mod.rs                ← Re-export

└── codegen/shared/
    └── expr.rs               ← OptionalChain override, BlockExpr IIFE, format_match for all backends

omnilang/src/codegen/
├── mod.rs                    ← Design gate: P0 remove (in Phase 3/4 tasks), non-P0 warning→error
├── shared/design.rs          ← NEW: extract shared CSS emitter from omniview/design.rs
├── omniview/
│   ├── view/element.rs       ← Remove paused_override, @paused; add @focus trigger; ref binding attrs
│   ├── mod.rs                ← Slot fallback rendering; semantic binding JS output
│   └── motion.rs             ← @focus trigger handling
├── react/
│   ├── mod.rs                ← Zustand v5 store gen + design gate removal
│   ├── view.rs               ← slots, ref bindings, semantic binding useEffect
│   ├── expr.rs               ← OptionalChain/BlockExpr/Match/RefOpExpr formatting
│   ├── component/
│   │   ├── state.rs          ← useRef gen, semantic binding effect gen
│   │   ├── effect.rs         ← @mount/@unmount/@effect → useEffect
│   │   └── imports.rs        ← Zustand v5 imports, ref types
│   ├── motion/
│   │   ├── mod.rs            ← Native ref + WAAPI (no DOM scanning), all 8 triggers
│   │   └── keyframes.rs      ← Semantic property → WAAPI keyframe objects
│   └── design.rs             ← NEW: Design CSS emission with themes/classes/variants
├── flutter/
│   ├── mod.rs                ← Riverpod 3.0 store gen + design gate removal
│   ├── view.rs               ← slots, ref bindings, semantic binding post-frame callbacks
│   ├── expr.rs               ← OptionalChain/BlockExpr/Match/RefOpExpr formatting
│   ├── stmt.rs               ← setState wrapping for signal mutations
│   ├── motion.rs             ← TweenSequence per-property, Animatable<T>, playback, easing
│   ├── motion_widgets.rs     ← _OmniInViewWidget, _OmniScrollDrivenWidget, _OmniDragWidget
│   ├── ref.rs                ← NEW: GlobalKey + FocusNode + ScrollController + TextEditingController gen
│   └── design.rs             ← NEW: ThemeData + ThemeExtension + BoxDecoration factories
└── tests_regression/
    └── parameterized.rs      ← Expand: design/slots/refs/key_expr/animation/store fixtures

test/projects/learn-omniview/src/
├── pages/
│   ├── MotionDemo.omni           ← animation syntax migration
│   ├── ElementRefDemo.omni       ← ref syntax migration
│   ├── MotionTriggersDemo.omni   ← trigger renames
│   ├── KeyframeAnimationDemo.omni ← keyframe syntax, remove @paused
│   ├── EasingFunctionsDemo.omni  ← steps() syntax, easing keywords
│   ├── AnimPropertiesDemo.omni   ← CSS property → semantic property set
│   ├── CssVarsDemo.omni          ← motion sections (if any semantic violations)
│   └── SignalPatternsDemo.omni   ← @if mode:, @slot syntax
├── components/
│   ├── Modal.omni                ← slot declaration/rendering restructure
│   ├── NotificationsPanel.omni   ← slot restructure
│   └── SlotDefaultDemo.omni      ← slot restructure
└── stores/                       ← (no changes — store syntax unchanged)

tests/fixtures/
├── golden/                       ← regenerate all golden fixtures after migration
├── react/motion_*.omni           ← 5 react motion fixtures — migrate animation syntax
└── parameterized/                ← NEW: design_tokens.omni, slots.omni, refs.omni, animation.omni
```

---

## Phase 0: Lexer & Parser Changes

> All new syntax forms must parse BEFORE IR/codegen work starts.
> Tests verify parsing only — no codegen output validation yet.

### Task 0.1: Rename animation trigger tokens in lexer [∥]

**Files:**
- Modify: `crates/omnilang-core/src/lexer/keywords.rs`

- [ ] **Step 1: Rename @-prefixed trigger tokens**

In `at_keyword_to_token()` function, rename:
```rust
// Old token name                  → New token name
"inView"                          → "in-view"
"inViewOnce"                      → DELETE (becomes independent `once={}` attr)
"inViewRepeat"                    → DELETE (becomes independent `repeat={}` attr)
"inViewThreshold"                 → DELETE (becomes independent `threshold={}` attr)
"dragHorizontal"                  → "drag-horizontal"
"dragVertical"                    → "drag-vertical"
"dragConstraints"                 → DELETE (becomes independent `constraints={}` attr)
"dragElastic"                     → DELETE (becomes independent `elastic={}` attr)
"dragScrub"                       → DELETE (no replacement — removed)
"dragMomentum"                    → DELETE (no replacement — removed)
"dragThreshold"                   → DELETE (becomes independent `threshold={}` attr)
"dragVelocityMax"                 → DELETE (becomes independent `velocity-max={}` attr)
"scrollStart"                     → DELETE (becomes independent `start={}` attr)
"scrollEnd"                       → DELETE (becomes independent `end={}` attr)
"scrollScrub"                     → DELETE (becomes independent `scrub={}` attr)
```

New tokens to register:
```rust
// Add to keyword map:
"in-view"     → AtInView       // renamed from inView
"drag-horizontal" → AtDragHorizontal  // renamed from dragHorizontal  
"drag-vertical"   → AtDragVertical    // renamed from dragVertical
// velocity-max is a regular ident, not a token — parsed as sub-attr key
```

- [ ] **Step 2: Update TokenKind enum**

Rename variants in the `TokenKind` enum (file: `crates/omnilang-core/src/lexer/mod.rs`):
```rust
AtInView        // was AtInView (unchanged name, changed token text)
AtDragHorizontal // was AtDragHorizontal
AtDragVertical   // was AtDragVertical
// DELETE:
// AtInViewOnce, AtInViewRepeat, AtInViewThreshold
// AtDragConstraints, AtDragElastic, AtDragScrub, AtDragMomentum, AtDragThreshold, AtDragVelocityMax
// AtScrollStart, AtScrollEnd, AtScrollScrub
```

- [ ] **Step 3: Add `focus` as a regular keyword (not @-prefixed token)**

`@focus` already exists as `AtFocus`. No change needed — it was there before, just unused.

- [ ] **Step 4: Build and fix all TokenKind references**

Run: `cargo build -p omnilang-core 2>&1`
Expected: compile errors wherever deleted TokenKind variants are matched. Fix each:
- `parser/view/element.rs` — sub-attribute parsing (previously @-prefixed tokens, now regular ident-based sub-attrs)
- `parser/view/mod.rs` — trigger dispatch in `parse_view_node`
- `parser/ast/view.rs` — `ElementProp` variants for deleted tokens
- All lowering code that matches deleted `ElementProp` variants
- All codegen code that reads deleted element prop fields

- [ ] **Step 5: Run parser tests**

```bash
cargo test -p omnilang-core -- parser 2>&1
```
Expected: parser tests pass after fixing all TokenKind references.

- [ ] **Step 6: Commit**

```bash
git commit -m "refactor(lexer): rename animation trigger tokens to semantic DSL

Rename: @inView→@in-view, @dragHorizontal→@drag-horizontal,
@dragVertical→@drag-vertical. Delete sub-attribute tokens
(@inViewOnce, @dragConstraints, @scrollStart, etc.) —
they become independent ident-based attrs (once=, constraints=, start=, etc.)
per semantic-level DSL spec §3.2."
```

### Task 0.2: Update motion body parser syntax [∥]

**Files:**
- Modify: `crates/omnilang-core/src/parser/top_level/motion.rs`
- Modify: `crates/omnilang-core/src/parser/expr/motion.rs`
- Modify: `crates/omnilang-core/src/parser/ast/motion.rs`

- [ ] **Step 1: Change motion_param parsing to colon-based**

In `parse_shared_timing()` (motion.rs:176-205), replace:
```
Old:  over 500          (ident + number, no separator)
New:  over: 300ms       (keyword, colon, expression with unit)

Old:  ease power2Out    (ident + easing name)
New:  easing: ease-out  (keyword, colon, easing expression)
```

Rewrite `parse_shared_timing`:
```rust
fn parse_shared_timing(&mut self, over: &mut Expr, ease: &mut Option<EasingExpr>) -> Result<(), ParseError> {
    loop {
        match self.peek_ident_value() {
            Some(ref s) if s == "over" => {
                self.advance(); // 'over'
                self.expect_token(TokenKind::Colon, "expected ':' after 'over'")?;
                *over = self.parse_duration_expr()?;  // parses "300ms", "1.5s", "0.5"
            }
            Some(ref s) if s == "easing" => {
                self.advance(); // 'easing'
                self.expect_token(TokenKind::Colon, "expected ':' after 'easing'")?;
                *ease = Some(self.parse_easing()?);  // unchanged — already handles cubic-bezier, steps, named
            }
            _ => break,
        }
        if *self.peek_kind() == TokenKind::Comma { self.advance(); }
    }
    Ok(())
}

fn parse_duration_expr(&mut self) -> Result<Expr, ParseError> {
    // Parse a duration literal: 300ms, 1.5s, 0.5 (seconds default)
    let t = self.advance();
    match &t.kind {
        TokenKind::IntLiteral(n) => {
            let val_str = format!("{}", n);
            if self.peek_ident_value().is_some_and(|s| s == "ms" || s == "s") {
                let unit = self.advance_ident();
                Ok(Expr::Literal(Literal::String(format!("{}{}", val_str, unit)), t.span))
            } else {
                Ok(Expr::Literal(Literal::Int(*n), t.span))
            }
        }
        TokenKind::FloatLiteral(f) => {
            let val_str = format!("{}", f);
            if self.peek_ident_value().is_some_and(|s| s == "ms" || s == "s") {
                let unit = self.advance_ident();
                Ok(Expr::Literal(Literal::String(format!("{}{}", val_str, unit)), t.span))
            } else {
                Ok(Expr::Literal(Literal::Float(*f), t.span))
            }
        }
        _ => Err(self.error("expected duration value (e.g. 300ms, 1.5s)"))
    }
}
```

- [ ] **Step 2: Add semantic_anim_prop whitelist in stop_property parsing**

In `parse_keyframe_stop()` (motion.rs:103-173), `parse_css_property_name` currently accepts any CSS-like ident. Replace with whitelist:

```rust
const SEMANTIC_ANIM_PROPS: &[&str] = &[
    "opacity", "scale", "scaleX", "scaleY",
    "translateX", "translateY", "translateZ",
    "rotate", "rotateX", "rotateY", "rotateZ",
    "skewX", "skewY",
    "backgroundColor", "color",
    "width", "height",
];

fn parse_semantic_anim_prop(&mut self) -> Result<String, ParseError> {
    let name = self.expect_ident("expected animation property name")?;
    if SEMANTIC_ANIM_PROPS.contains(&name.as_str()) {
        Ok(name)
    } else {
        Err(self.error(format!(
            "'{}' is not a recognized animation property. Use one of: {}",
            name, SEMANTIC_ANIM_PROPS.join(", ")
        )))
    }
}
```

Replace `parse_css_property_name` call with `parse_semantic_anim_prop` in `parse_keyframe_stop`.

- [ ] **Step 3: Update stop_position to accept bare number (not just 'from'/'to')**

Current parser already handles `IntLiteral`/`FloatLiteral` as percentage. Add `%` suffix consumption if present. This is already partially implemented (lines 118-141) — verify it handles `50%` (with percent sign) correctly.

- [ ] **Step 4: Update easing_expr AST to match new syntax**

In `parser/ast/motion.rs`, add `EasingExpr` variant for `steps(count, direction)`:
```rust
pub enum EasingExpr {
    Named(String),                              // ease-in, ease-out, linear
    CubicBezier { x1: f64, y1: f64, x2: f64, y2: f64 },
    Steps { count: u32, direction: String },    // steps(6, end)
    Back { direction: String },
    Elastic { direction: String },
}
```

- [ ] **Step 5: Update parse_easing to handle steps() with positional args**

In `parser/expr/motion.rs:parse_easing()`, update `steps` parsing:
```rust
// Old: steps(count: 6, direction: end)
// New: steps(6, end)
"steps" => {
    self.advance(); // 'steps'
    self.expect_token(TokenKind::LParen, "expected '('")?;
    let count: u32 = self.expect_int("expected step count")?;
    self.expect_token(TokenKind::Comma, "expected ','")?;
    let direction = self.expect_ident("expected 'start' or 'end'")?;
    self.expect_token(TokenKind::RParen, "expected ')'")?;
    EasingExpr::Steps { count, direction }
}
```

- [ ] **Step 6: Update easing named values to kebab-case**

Map old easing names to new semantic names:
```rust
fn normalize_easing_name(name: &str) -> String {
    match name {
        "easeIn" | "ease-in" => "ease-in".into(),
        "easeOut" | "ease-out" => "ease-out".into(),
        "easeInOut" | "ease-in-out" => "ease-in-out".into(),
        "power2Out" => "ease-out".into(),  // closest semantic equivalent
        "backOut" => "back-out".into(),
        "elasticOut" => "elastic-out".into(),
        // Accept both forms, normalize to kebab-case
        other => other.to_lowercase(),
    }
}
```

Accept both camelCase (existing code) and kebab-case (new spec) during a transition period. Emit deprecation warning for camelCase.

- [ ] **Step 7: Update AST types**

In `parser/ast/motion.rs`, update `MotionBody` and `KeyframeBody` to use new types:
```rust
pub struct KeyframeBody {
    pub stops: Vec<KeyframeStop>,
    pub over: Expr,              // duration: "300ms", "1.5s"
    pub easing: Option<EasingExpr>,  // renamed from 'ease'
    pub playback: Vec<PlaybackProp>,
}

pub struct MotionParam {
    pub key: String,    // "over", "easing"
    pub value: Expr,    // duration value or easing expression
}
```

- [ ] **Step 8: Build, test, fix lowering**

Run: `cargo build -p omnilang-core 2>&1`
Fix all compilation errors in lowering passes that consume the old AST types.

Run: `cargo test -p omnilang-core -- parser 2>&1`
Expected: motion parser tests may fail — update test expectations.

- [ ] **Step 9: Commit**

```bash
git commit -m "refactor(parser): update motion body syntax to semantic DSL

Motion params: over: 300ms / easing: ease-out (colon separator, duration units).
Keyframe properties: semantic_anim_prop whitelist replaces CSS property names.
Easing: steps(6, end) positional args; kebab-case easing names.
Backward compat: accept camelCase easing names with deprecation warning."
```

### Task 0.3: Parse semantic binding attributes on elements [∥]

**Files:**
- Modify: `crates/omnilang-core/src/parser/view/element.rs`
- Modify: `crates/omnilang-core/src/parser/ast/view.rs`

- [ ] **Step 1: Add new ElementProp variant for semantic bindings**

In `parser/ast/view.rs`, add:
```rust
pub enum ElementProp {
    // ... existing variants ...

    /// Semantic ref binding: focus={sig}, scroll-x={sig}, scroll-y={sig}, value={sig}
    /// These are compiler-recognized — not generic static props.
    RefBinding {
        /// The semantic binding name: "focus", "scroll-x", "scroll-y", "value"
        binding: String,
        /// The signal expression driving the binding
        signal: Expr,
    },
}
```

- [ ] **Step 2: Recognize semantic binding names during element prop parsing**

In `parser/view/element.rs`, inside the attribute parsing loop, after the event prop check and before the generic static prop fallback:

```rust
// Check for semantic binding attributes (focus, scroll-x, scroll-y, value)
// These are only valid when the element also has @ref={name}
const SEMANTIC_BINDINGS: &[&str] = &["focus", "scroll-x", "scroll-y", "value"];
if SEMANTIC_BINDINGS.contains(&attr_name.as_str()) {
    self.expect_token(TokenKind::Eq, "expected '='")?;
    self.expect_token(TokenKind::LBrace, "expected '{'")?;
    let signal = self.parse_expr()?;
    self.expect_token(TokenKind::RBrace, "expected '}'")?;
    return Ok(ElementProp::RefBinding {
        binding: attr_name,
        signal,
    });
}
```

- [ ] **Step 3: Validate that semantic bindings only appear on elements with @ref**

This validation happens in the semantic pass (Task 2.1), not in the parser. The parser just recognizes the syntax. Semantic pass checks:
- Element has `@ref={name}` binding
- The ref name matches a declared ref in the component
- The binding name is valid for the element type

- [ ] **Step 4: Build and test**

```bash
cargo build -p omnilang-core 2>&1
cargo test -p omnilang-core -- parser 2>&1
```

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(parser): parse semantic ref binding attributes

Recognize focus={sig}, scroll-x={sig}, scroll-y={sig}, value={sig}
as ElementProp::RefBinding instead of generic Static props.
Validation (element has @ref, ref declared) in semantic pass."
```

### Task 0.4: Parse @slot type constraint and @if mode: syntax [∥]

**Files:**
- Modify: `crates/omnilang-core/src/parser/view/mod.rs` (slot parsing)
- Modify: `crates/omnilang-core/src/parser/view/directives.rs` (if directive parsing)
- Modify: `crates/omnilang-core/src/parser/ast/view.rs` (SlotDirective fields)

- [ ] **Step 1: Add type_constraint field to SlotDirective AST**

In `parser/ast/view.rs`, update `ViewNode::SlotDirective`:
```rust
ViewNode::SlotDirective {
    name: Option<String>,
    default_content: Option<Vec<ViewNode>>,
    type_constraint: Option<TypeAnnotation>,  // NEW: @slot name: Type
}
```

- [ ] **Step 2: Parse optional type annotation after slot name**

In `parser/view/mod.rs:65-96`, after parsing `name`, before parsing `default_content`:
```rust
// After parsing name:
let type_constraint = if self.at(&TokenKind::Colon) {
    self.advance(); // ':'
    Some(self.parse_type_annotation()?)
} else {
    None
};
```

Update the `SlotDirective` construction to include `type_constraint`.

- [ ] **Step 3: Fix @if mode syntax — colon separator + expression not string**

In `parser/view/directives.rs:17-23`, current code:
```rust
let transition_mode = if self.peek_ident_value().is_some_and(|s| s == "mode") {
    self.advance(); // consume 'mode'
    self.expect_token(TokenKind::Eq, "expected '='")?;  // OLD
    Some(self.expect_string("expected string literal")?)  // OLD: string literal
} else {
    None
};
```

Replace with:
```rust
let transition_mode = if self.peek_ident_value().is_some_and(|s| s == "mode") {
    self.advance(); // 'mode'
    self.expect_token(TokenKind::Colon, "expected ':' after 'mode'")?;  // NEW: colon
    // Accept both string literals ("fade", "wait") and idents (fade, wait)
    let mode_val = if *self.peek_kind() == TokenKind::StringLiteral(_) {
        self.expect_string("expected mode value")?
    } else {
        self.expect_ident("expected mode value (fade, wait, none)")?
    };
    // Validate: only "fade", "wait", "none" are valid
    if !matches!(mode_val.as_str(), "fade" | "wait" | "none") {
        return Err(self.error(format!(
            "invalid transition_mode '{}' — use 'fade', 'wait', or 'none'", mode_val
        )));
    }
    Some(mode_val)
} else {
    None
};
```

- [ ] **Step 4: Parse @for [scroll] hint**

In `parser/view/directives.rs:84-131`, after `key_expr` parsing, before `{`:
```rust
// Optional [scroll] hint
let scroll_hint = if *self.peek_kind() == TokenKind::LBracket {
    self.advance(); // '['
    let hint = self.expect_ident("expected 'scroll'")?;
    if hint != "scroll" {
        return Err(self.error("expected 'scroll' inside brackets, e.g. [scroll]"));
    }
    self.expect_token(TokenKind::RBracket, "expected ']'")?;
    true
} else {
    false
};
```

Add `scroll_hint: bool` field to `ForDirective` AST.

- [ ] **Step 5: Build, test, fix lowering**

```bash
cargo build -p omnilang-core 2>&1
# Fix all lowering code that constructs SlotDirective / reads transition_mode / reads ForDirective
cargo test -p omnilang-core -- parser 2>&1
```

- [ ] **Step 6: Commit**

```bash
git commit -m "feat(parser): parse @slot type constraint, @if mode:, @for [scroll]

- @slot name: TypeAnnotation { fallback } — parse type constraint
- @if cond mode: \"wait\" — colon separator, validate values (fade/wait/none)
- @for item in list [scroll] — parse scroll hint for Flutter ListView.builder
- Add ForDirective.scroll_hint field"
```

---

---

## Phase 1: IR Foundation

> All codegen backends depend on IR types. Must complete before Phase 2 semantic passes.

### Task 1.1: Add RefOpIR enum + RefOpExpr + semantic_bindings to IR types

**Files:**
- Modify: `crates/omnilang-core/src/ir/types.rs`

- [ ] **Step 1: Add RefOpIR enum**

Insert after `ViewTree` (after ~line 678):
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum RefOpIR {
    Focus,
    Blur,
    ScrollTo { x: Option<ExprIR>, y: Option<ExprIR> },
    ScrollBy { dx: Option<ExprIR>, dy: Option<ExprIR> },
    Measure,
    Select,
    SetSelection { start: ExprIR, end: ExprIR },
    Value,
    SetValue { value: ExprIR },
    CapturePointer { id: ExprIR },
    ReleasePointer { id: ExprIR },
}
```

- [ ] **Step 2: Add RefOpExpr variant to ExprIR**

```rust
RefOpExpr { ref_name: String, op: RefOpIR },
```

- [ ] **Step 3: Add semantic_bindings to ElementIR**

```rust
pub semantic_bindings: FxHashMap<String, ExprIR>,
```

- [ ] **Step 4: Exhaustively fix all match arms on ExprIR**

Grep for `ExprIR::` match patterns across the entire codebase. Files that must be updated:
```
crates/omnilang-core/src/ir/lower/expr.rs          — default: _ => {}
crates/omnilang-core/src/ir/optimize.rs            — skip (no rewrite needed)
crates/omnilang-core/src/ir/walk.rs                — walk into RefOpExpr fields
crates/omnilang-core/src/codegen/shared/expr.rs     — format RefOpExpr (default: emit comment)
omnilang/src/codegen/react/expr.rs                  — format RefOpExpr (Phase 4)
omnilang/src/codegen/flutter/expr.rs                — format RefOpExpr (Phase 5)
omnilang/src/codegen/omniview/expr.rs               — format RefOpExpr (Phase 6)
omnilang/src/codegen/javascript/expr.rs             — default: panic/unreachable
omnilang/src/codegen/dart/expr.rs                   — default: panic/unreachable
omnilang/src/codegen/python/expr.rs                 — default: panic/unreachable
omnilang/src/codegen/rust/expr.rs                   — default: panic/unreachable
omnilang/src/codegen/vue/expr.rs                    — default: panic/unreachable
omnilang/src/codegen/dioxus/expr.rs                 — default: panic/unreachable
```

Run: `cargo build -p omnilang 2>&1` and fix each error.

- [ ] **Step 5: Fix all ElementIR construction sites**

Grep for `ElementIR {` or `ElementIR::new` and add `semantic_bindings: FxHashMap::default()` to each.

- [ ] **Step 6: Commit**

```bash
git commit -m "feat(ir): add RefOpIR, ExprIR::RefOpExpr, ElementIR.semantic_bindings"
```

### Task 1.2: Fix ViewTree::Slot to preserve fallback + type_constraint

**Files:**
- Modify: `crates/omnilang-core/src/ir/types.rs`
- Modify: `crates/omnilang-core/src/ir/lower/view.rs`
- Modify: all codegen consumers

- [ ] **Step 1: Change ViewTree::Slot variant**

Replace `Slot(Option<String>)` with:
```rust
Slot {
    name: Option<String>,
    fallback: Vec<ViewTree>,
    type_constraint: Option<TypeIR>,
    data_param: Option<String>,
},
```

- [ ] **Step 2: Fix lowering**

```rust
ViewNode::SlotDirective { name, default_content, type_constraint } => {
    let fallback = default_content
        .map(|nodes| nodes.into_iter()
            .map(|n| lower_view_node(n, string_signals, variant_map, semantics, fns))
            .collect())
        .unwrap_or_default();
    let (tc, dp) = if let Some(tc) = type_constraint {
        (Some(lower_type(&tc)), Some(name.clone().unwrap_or_default()))
    } else {
        (None, None)
    };
    ViewTree::Slot { name: name.clone(), fallback, type_constraint: tc, data_param: dp }
},
```

- [ ] **Step 3: Fix all codegen consumers**

Update all `ViewTree::Slot(` match arms across all codegen backends. For non-P0 backends, emit a diagnostic comment. For P0 backends (React, Flutter, OmniView), render fallback when name/parent content is empty.

- [ ] **Step 4: Build and test**

```bash
cargo build -p omnilang 2>&1
cargo test -p omnilang-core 2>&1
```

- [ ] **Step 5: Commit**

```bash
git commit -m "fix(ir): preserve slot fallback content and type constraint"
```

### Task 1.3: Motion IR type changes [∥]

**Files:**
- Modify: `crates/omnilang-core/src/ir/types_motion.rs`
- Modify: `crates/omnilang-core/src/ir/lower/view.rs` (delete PausedOverride lowering)
- Modify: `crates/omnilang-core/src/parser/ast/view.rs` (delete ElementProp::PausedOverride)

- [ ] **Step 1: Add TriggerKind::Focus**

- [ ] **Step 2: Delete PlaybackIR.paused field + all construction sites**

- [ ] **Step 3: Delete MotionBehavior.paused_override field + all lowering**

- [ ] **Step 4: Delete ElementProp::PausedOverride from AST + parser**

- [ ] **Step 5: Add scroll_hint to ListRenderingIntent**

```rust
pub struct ListRenderingIntent {
    // ... existing fields ...
    pub scroll_hint: bool,   // NEW: from @for [scroll]
}
```

- [ ] **Step 6: Lower scroll_hint from ForDirective**

In `ir/lower/view.rs:lower_for_directive`:
```rust
scroll_hint: for_dir.scroll_hint,
```

- [ ] **Step 7: Build, test, fix compilation**

```bash
cargo build -p omnilang 2>&1
cargo test -p omnilang 2>&1
cargo insta review  # update paused_override snapshots
```

- [ ] **Step 8: Commit**

```bash
git commit -m "refactor(ir): add TriggerKind::Focus, ForDirective.scroll_hint; delete paused/paused_override"
```

---

---

## Phase 2: Semantic Passes

> Intent recognition — rewrite generic AST into semantic IR that codegen understands.

### Task 2.1: Create validate_ref_ops pass (method calls + field access)

**Files:**
- Create: `crates/omnilang-core/src/semantic/validate_ref_ops.rs`
- Modify: `crates/omnilang-core/src/semantic/mod.rs`
- Modify: `crates/omnilang-core/src/ir/lower/mod.rs` (wire in after component lowering)

- [ ] **Step 1: Define legal ref method dispatch table**

```rust
use crate::ir::types::{ExprIR, RefOpIR, RefDeclIR};
use crate::semantic::SemanticDiagnostic;

type RefOpConstructor = fn(Vec<ExprIR>) -> RefOpIR;

static LEGAL_REF_METHODS: phf::Map<&'static str, RefOpConstructor> = phf_map! {
    "focus" => |_| RefOpIR::Focus,
    "blur" => |_| RefOpIR::Blur,
    "scrollTo" => |a| RefOpIR::ScrollTo { x: a.get(0).cloned(), y: a.get(1).cloned() },
    "scrollBy" => |a| RefOpIR::ScrollBy { dx: a.get(0).cloned(), dy: a.get(1).cloned() },
    "select" => |_| RefOpIR::Select,
    "setSelection" => |a| RefOpIR::SetSelection { start: a.first().cloned().unwrap_or_default(), end: a.get(1).cloned().unwrap_or_default() },
    "measure" => |_| RefOpIR::Measure,
    "capturePointer" => |a| RefOpIR::CapturePointer { id: a.first().cloned().unwrap_or_default() },
    "releasePointer" => |a| RefOpIR::ReleasePointer { id: a.first().cloned().unwrap_or_default() },
};

static ILLEGAL_REF_METHODS: phf::Map<&'static str, &'static str> = phf_map! {
    "style" => "use style:{...} binding with signals instead",
    "innerHTML" => "use text interpolation {value} instead",
    "addEventListener" => "use @on* event binding syntax instead",
    "querySelector" => "violates component encapsulation",
    "classList" => "use class: binding syntax instead",
    "parentElement" => "violates component encapsulation",
    "textContent" => "use text interpolation {value} instead",
    "getBoundingClientRect" => "use ref.measure() instead",
};

static ILLEGAL_REF_FIELDS: phf::Map<&'static str, &'static str> = phf_map! {
    "offsetWidth" => "use ref.measure().width instead",
    "offsetHeight" => "use ref.measure().height instead",
    "style" => "use style:{...} binding with signals instead",
};
```

- [ ] **Step 2: Implement main validation function**

```rust
pub fn validate_ref_ops(
    component: &mut ComponentIR,
    diagnostics: &mut Vec<SemanticDiagnostic>,
) {
    let ref_names: Vec<String> = component.refs.iter().map(|r| r.name.clone()).collect();

    // Walk view tree expressions
    if let Some(ref mut view) = component.view {
        walk_view_tree_exprs_mut(view, &ref_names, diagnostics);
    }

    // Walk expressions in effects
    for effect in &mut component.effects {
        for stmt in &mut effect.body.statements {
            walk_stmt_exprs_mut(stmt, &ref_names, diagnostics);
        }
    }

    // Walk expressions in actions
    for action in &mut component.actions {
        for stmt in &mut action.body.statements {
            walk_stmt_exprs_mut(stmt, &ref_names, diagnostics);
        }
    }

    // Walk expressions in functions
    for func in &mut component.fns {
        for stmt in &mut func.body.statements {
            walk_stmt_exprs_mut(stmt, &ref_names, diagnostics);
        }
    }
}
```

- [ ] **Step 3: Implement expression walker with MethodCall + FieldAccess handling**

```rust
fn check_expr(expr: &mut ExprIR, ref_names: &[String], diagnostics: &mut Vec<SemanticDiagnostic>) {
    match expr {
        // Method call on ref: ref.focus(), ref.scrollTo(0, 100)
        ExprIR::MethodCallExpr { object, method, args } => {
            if let ExprIR::IdentifierExpr(ref_name) = object.as_ref() {
                if ref_names.contains(ref_name) {
                    if let Some(constructor) = LEGAL_REF_METHODS.get(method.as_str()) {
                        let op = constructor(args.clone());
                        *expr = ExprIR::RefOpExpr { ref_name: ref_name.clone(), op };
                    } else if let Some(hint) = ILLEGAL_REF_METHODS.get(method.as_str()) {
                        diagnostics.push(SemanticDiagnostic::error(
                            format!("ref.{}() is not allowed — {}", method, hint)
                        ));
                    } else {
                        diagnostics.push(SemanticDiagnostic::error(
                            format!("unknown ref operation: ref.{}()", method)
                        ));
                    }
                }
            }
        }
        // Field access on ref: ref.width, ref.offsetWidth, ref.value, ref.style
        ExprIR::FieldAccessExpr { object, field } => {
            if let ExprIR::IdentifierExpr(ref_name) = object.as_ref() {
                if ref_names.contains(ref_name) {
                    match field.as_str() {
                        "width" | "height" => {
                            *expr = ExprIR::RefOpExpr {
                                ref_name: ref_name.clone(),
                                op: RefOpIR::Measure,
                            };
                            // Note: codegen will append .width/.height to the measure result
                        }
                        "value" => {
                            *expr = ExprIR::RefOpExpr {
                                ref_name: ref_name.clone(),
                                op: RefOpIR::Value,
                            };
                        }
                        _ => {
                            if let Some(hint) = ILLEGAL_REF_FIELDS.get(field.as_str()) {
                                diagnostics.push(SemanticDiagnostic::error(
                                    format!("ref.{} is not allowed — {}", field, hint)
                                ));
                            } else {
                                diagnostics.push(SemanticDiagnostic::error(
                                    format!("unknown ref field: ref.{}", field)
                                ));
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }
}
```

- [ ] **Step 4: Implement recursive walkers for ViewTree, StmtIR, ExprIR**

```rust
fn walk_view_tree_exprs_mut(
    tree: &mut ViewTree, ref_names: &[String], diagnostics: &mut Vec<SemanticDiagnostic>
) {
    // Recursively walk all ViewTree nodes and their expression fields
    match tree {
        ViewTree::Element(el) => {
            for expr in el.props.values_mut() { check_expr(expr, ref_names, diagnostics); }
            for child in &mut el.children { walk_view_tree_exprs_mut(child, ref_names, diagnostics); }
        }
        ViewTree::Conditional(cond) => {
            check_expr(&mut cond.condition, ref_names, diagnostics);
            // walk then/else branches...
        }
        ViewTree::ListRendering(list) => {
            check_expr(&mut list.iterable, ref_names, diagnostics);
            if let Some(ref mut key) = list.key_expr { check_expr(key, ref_names, diagnostics); }
            // walk body...
        }
        ViewTree::Interpolation(expr) => check_expr(expr, ref_names, diagnostics),
        ViewTree::Fragment(children) => {
            for child in children { walk_view_tree_exprs_mut(child, ref_names, diagnostics); }
        }
        ViewTree::Slot { ref mut fallback, .. } => {
            for child in fallback { walk_view_tree_exprs_mut(child, ref_names, diagnostics); }
        }
        _ => {}
    }
}

fn walk_stmt_exprs_mut(
    stmt: &mut StmtIR, ref_names: &[String], diagnostics: &mut Vec<SemanticDiagnostic>
) {
    // Match each StmtIR variant, walk its expression fields
}
```

- [ ] **Step 5: Wire into lowering pipeline**

In `ir/lower/mod.rs`, after all components are lowered:
```rust
for module in program.modules.values_mut() {
    for component in module.components.values_mut() {
        crate::semantic::validate_ref_ops::validate_ref_ops(component, &mut diagnostics);
    }
}
```

- [ ] **Step 6: Add tests**

Test cases in `#[cfg(test)] mod tests`:
1. `ref.focus()` → `RefOpExpr { op: Focus }` ✓
2. `ref.scrollTo(0, 100)` → `RefOpExpr { op: ScrollTo { x: Some(0), y: Some(100) } }` ✓
3. `ref.style.color` → diagnostic error ✓
4. `ref.offsetWidth` → diagnostic error with hint ✓
5. `ref.width` → `RefOpExpr { op: Measure }` ✓
6. `obj.focus()` (non-ref) → unchanged ✓
7. `ref.unknownMethod()` → diagnostic error ✓

```bash
cargo test -p omnilang-core -- semantic::validate_ref_ops 2>&1
cargo test -p omnilang 2>&1
```

- [ ] **Step 7: Commit**

```bash
git commit -m "feat(semantic): add validate_ref_ops pass

Scans MethodCallExpr and FieldAccessExpr on declared refs.
Rewrites legal operations to ExprIR::RefOpExpr so codegen knows intent.
Emits compile errors for DOM-specific APIs with migration hints."
```

### Task 2.2: Create lower_inline_motions pass [∥]

**Files:**
- Create: `crates/omnilang-core/src/ir/lower/lower_inline_motions.rs`
- Modify: `crates/omnilang-core/src/ir/lower/mod.rs`

- [ ] **Step 1: Implement the pass**

```rust
pub fn lower_inline_motions(module: &mut ModuleIR) {
    let mut counter = 0u32;
    let mut new_motions = Vec::new();

    for component in module.components.values_mut() {
        if let Some(ref mut view) = component.view {
            walk_and_lower_inline_motions(view, &mut counter, &mut new_motions);
        }
    }
    module.motions.extend(new_motions);
}

fn walk_and_lower_inline_motions(
    tree: &mut ViewTree, counter: &mut u32, new_motions: &mut Vec<MotionIntent>
) {
    match tree {
        ViewTree::Element(el) => {
            for trigger in &mut el.motion {
                if let Some(ref body) = trigger.inline_motion.take() {
                    let name = format!("__anon_m_{}", counter);
                    *counter += 1;
                    new_motions.push(MotionIntent {
                        name: name.clone(),
                        body: body.clone(),
                        source_span: trigger.source_span,
                    });
                    trigger.motion_ref = Some(name);
                }
            }
            for child in &mut el.children {
                walk_and_lower_inline_motions(child, counter, new_motions);
            }
        }
        ViewTree::Conditional(cond) => { /* walk branches */ }
        ViewTree::ListRendering(list) => { /* walk body */ }
        ViewTree::Fragment(children) => {
            for child in children { walk_and_lower_inline_motions(child, counter, new_motions); }
        }
        ViewTree::Slot { ref mut fallback, .. } => {
            for child in fallback { walk_and_lower_inline_motions(child, counter, new_motions); }
        }
        _ => {}
    }
}
```

- [ ] **Step 2: Wire into lowering pipeline**

After component lowering, before codegen:
```rust
for module in &mut program.modules.values_mut() {
    lower_inline_motions::lower_inline_motions(module);
}
```

- [ ] **Step 3: Test and commit**

```bash
cargo test -p omnilang 2>&1
git commit -m "feat(ir): add lower_inline_motions pass"
```

### Task 2.3: Add semantic binding validation pass [∥]

**Files:**
- Create: `crates/omnilang-core/src/semantic/validate_semantic_bindings.rs`
- Modify: `crates/omnilang-core/src/ir/lower/mod.rs`

- [ ] **Step 1: Validate that RefBinding attrs only appear on @ref-bound elements**

Scan all `ElementIR` nodes. For each `ElementProp::RefBinding { binding, signal }`:
1. The element must have `ref_binding: Some(ref_name)` — else error: "semantic binding '{binding}' requires @ref on this element"
2. `ref_name` must be in `component.refs` — else error (already caught by existing validate_refs)
3. The `binding` name must be valid for this element type (e.g., `focus` only on focusable elements — warn if not)

- [ ] **Step 2: Lower RefBinding props into ElementIR.semantic_bindings**

In `ir/lower/view.rs`, during element lowering:
```rust
ElementProp::RefBinding { binding, signal } => {
    el.semantic_bindings.insert(binding, lower_expr(signal, ...));
}
```

- [ ] **Step 3: Test and commit**

```bash
git commit -m "feat(semantic): add semantic binding validation pass"
```

---

---


## Phase 3: Shared Codegen

> Foundation for all backends. Design gate removal happens WITHIN React/Flutter tasks (not before), so codegen exists before the gate opens.

### Task 3.1: Extract shared design CSS emitter

**Files:**
- Create: `omnilang/src/codegen/shared/design.rs`
- Modify: `omnilang/src/codegen/omniview/design.rs` (delegate to shared)

- [ ] **Step 1: Move target-agnostic design CSS emission to shared**

Copy `emit_design_css`, `token_css_value`, `expand_variant` from `omniview/design.rs` to `shared/design.rs`. Remove OmniView-specific parts (editor bridges, `__omni_effect` inline JS).

```rust
pub fn emit_design_css(designs: &[&DesignIR]) -> String;
pub fn token_to_css_value(value: &TokenValueIR, token_type: &TokenTypeIR) -> String;
pub fn class_to_css(class: &ClassIR, tokens: &FxHashMap<String, TokenIR>) -> String;
pub fn variant_to_css(variant: &VariantIR, tokens: &FxHashMap<String, TokenIR>) -> String;
pub fn theme_css_variables(design: &DesignIR, theme: &str) -> String;
```

- [ ] **Step 2: Make omniview/design.rs delegate to shared**

```rust
use crate::codegen::shared::design as shared_design;
pub fn emit_design_css(designs: &[&DesignIR]) -> String {
    let mut css = shared_design::emit_design_css(designs);
    // Add OmniView-specific debug runtime CSS after shared CSS
    css
}
```

- [ ] **Step 3: Build, test, commit**

```bash
cargo build -p omnilang 2>&1
cargo test -p omnilang -- codegen::omniview::design 2>&1
git commit -m "refactor(codegen): extract shared design CSS emitter from omniview/"
```

### Task 3.2: Override OptionalChain/BlockExpr/Match in expr formatters [∥]

**Files:**
- Modify: `crates/omnilang-core/src/codegen/shared/expr.rs`
- Modify: `omnilang/src/codegen/react/expr.rs`
- Modify: `omnilang/src/codegen/flutter/expr.rs`
- Modify: `omnilang/src/codegen/omniview/expr.rs`

- [ ] **Step 1: Fix shared format_optional_chain default to preserve `?.`**

```rust
fn format_optional_chain(&self, object: &ExprIR, field: &str) -> String {
    format!("{}?.{}", self.format_expr(object), field)  // was: {}.{}
}
```

- [ ] **Step 2: Override format_block_expr in ReactFormatExpr** — emit JS IIFE:
```rust
format!("(() => {{ {} return {}; }})()", stmts_joined, tail_expr)
```

- [ ] **Step 3: Override format_block_expr in DartFormatExpr** — emit Dart IIFE:
```rust
format!("(() {{ {} return {}; }})()", stmts_joined, tail_expr)
```

- [ ] **Step 4: Override format_block_expr in OmniView expr** — emit `(function() { ... })()`

- [ ] **Step 5: Implement format_match in DartFormatExpr** — Dart switch IIFE with `default: throw`

- [ ] **Step 6: Default RefOpExpr in shared** — emit diagnostic comment as placeholder:
```rust
ExprIR::RefOpExpr { ref_name, op } => format!("/* RefOp::{op:?} on {ref_name} */"),
```

- [ ] **Step 7: Run tests, review snapshots, commit**

```bash
cargo test -p omnilang 2>&1 && cargo insta review
git commit -m "fix(codegen): override OptionalChain/BlockExpr/Match/RefOpExpr formatting"
```

---

## Phase 4: React Codegen

> All tasks depend on Phase 1-3. Run sequentially. [∥] with Phase 5 (Flutter).

### Task 4.1: Zustand v5 store + C7/C8 fixes + design gate removal

**Files:**
- Modify: `omnilang/src/codegen/react/mod.rs`, `component/state.rs`, `component/imports.rs`, `expr.rs`
- Modify: `omnilang/src/codegen/mod.rs` (React gate only)
- Modify: `omnic/src/main.rs` (remove `--store-mode`)

- [ ] **Step 1: Remove React design block gate** — delete the `has_design_blocks` check at lines 152-161

- [ ] **Step 2: Rewrite `gen_zustand_store` to v5** — `create<T>()()((set) => ({...}))` pattern, standalone getter selector hooks with `useMemo`, exported action hooks with `useCallback` + `setState`

- [ ] **Step 3: Fix C7** — thread store signal map through expr formatting. Store signal mutations emit `useXxxStore.setState({ field: value })` instead of bare `field = value`

- [ ] **Step 4: Fix C8** — apply `rewrite_signal_refs_for_zustand` to ALL expressions in action bodies. Always emit `const state = get();` prologue

- [ ] **Step 5: Delete ReactStoreMode::Hooks + CLI flag**

- [ ] **Step 6: Test, snapshots, commit**

```bash
cargo test -p omnilang -- codegen::react 2>&1 && cargo insta review
git commit -m "refactor(react): Zustand v5 store + C7/C8 fixes + design gate removal"
```

### Task 4.2: React slot codegen

**Files:**
- Modify: `omnilang/src/codegen/react/view.rs`, `mod.rs`

- [ ] **Step 1: Replace slot comment with props rendering** — `{props.{name} ?? (fallback)}` / `{props.children ?? (fallback)}`

- [ ] **Step 2: Add slot props to component TS interface** — typed render prop if type_constraint present

- [ ] **Step 3: Commit**

```bash
git commit -m "feat(react): implement slot codegen with fallback + type constraints"
```

### Task 4.3: React ref semantic binding + RefOpExpr codegen

**Files:**
- Modify: `omnilang/src/codegen/react/component/state.rs`, `view.rs`, `expr.rs`

- [ ] **Step 1: Generate `useRef<T>(null)` for each RefDeclIR**, typed by element tag

- [ ] **Step 2: Wire `@ref={name}` → `ref={nameRef}` JSX prop**

- [ ] **Step 3: Generate `useEffect` for each semantic binding** (`focus`→focus/blur, `scroll-y`→scrollTo, `scroll-x`→scrollTo, `value`→onChange)

- [ ] **Step 4: Implement `format_expr` for all 12 `RefOpIR` variants** — `.focus()`, `.scrollTo({...})`, `.getBoundingClientRect()`, `.select()`, `.setSelectionRange()`, `.setPointerCapture()`, etc.

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(react): implement ref semantic binding + RefOpExpr codegen"
```

### Task 4.4: React animation — per-ref native effects, no DOM scanning

**Files:**
- Modify: `omnilang/src/codegen/react/motion/mod.rs`, `keyframes.rs`, `transforms.rs`
- Modify: `omnilang/src/codegen/react/component/motion_effects.rs`
- Modify: `omnilang/src/codegen/react/view.rs` (remove data-motion-* attributes)

- [ ] **Step 1: @enter** — `useEffect(() => { el.animate(kf, opts); return () => anim.cancel(); }, [])` on element ref

- [ ] **Step 2: @hover/@press** — `useCallback` handlers + `onMouseEnter/Leave`, `onPointerDown/Up` on element ref

- [ ] **Step 3: @in-view** — per-ref `IntersectionObserver` with `useEffect` cleanup

- [ ] **Step 4: @scroll** — per-ref scroll event listener with progress scrubbing

- [ ] **Step 5: @drag-*** — per-ref pointer event listeners with distance-to-progress mapping

- [ ] **Step 6: @focus** (newly re-added) — per-ref `focusin`/`focusout` listeners

- [ ] **Step 7: Semantic properties → WAAPI keyframes** — `opacity`→`opacity`, `scale`/`translateX`/`translateY`/`rotate`→`transform`, `backgroundColor`→`backgroundColor`, `color`→`color`

- [ ] **Step 8: Delete ALL `document.querySelectorAll`, `MutationObserver`, `data-motion-*` emission**

- [ ] **Step 9: Test, snapshots, commit**

```bash
cargo test -p omnilang -- codegen::react::motion 2>&1 && cargo insta review
git commit -m "refactor(react): rewrite animation — native refs, no DOM scanning, all 8 triggers"
```

### Task 4.5: React design CSS emission

**Files:**
- Create: `omnilang/src/codegen/react/design.rs`
- Modify: `omnilang/src/codegen/react/mod.rs`

- [ ] **Step 1: Generate design.css** — CSS custom properties with `:root`/`[data-theme]` scoping, class CSS rules, variant CSS combination classes

- [ ] **Step 2: Generate dynamic token `useEffect`** — `document.documentElement.style.setProperty('--name', value)` for `TokenValueIR::Dynamic`

- [ ] **Step 3: Generate `useThemeStore`** — Zustand store for `theme = "dark"` switching via `data-theme` attribute

- [ ] **Step 4: Wire into React codegen** — emit `design.css` and `useThemeStore.ts` to output files

- [ ] **Step 5: Commit**

```bash
git commit -m "feat(react): add design CSS emission with themes + dynamic tokens"
```

---

## Phase 5: Flutter Codegen

> All tasks depend on Phase 1-3. Run sequentially. [∥] with Phase 4 (React).

### Task 5.1: Riverpod 3.0 store + 6 CRITICAL fixes + design gate removal

**Files:**
- Modify: `omnilang/src/codegen/flutter/mod.rs`, `view.rs`, `expr.rs`, `stmt.rs`, `utils.rs`
- Modify: `omnilang/src/codegen/mod.rs` (Flutter gate only)

- [ ] **Step 1: Remove Flutter design block gate** — delete lines 192-201

- [ ] **Step 2: Rewrite store gen to Riverpod 3.0** — `Notifier<T>` + `NotifierProvider`, immutable state class with `copyWith`, getters as Notifier properties, actions as Notifier methods

- [ ] **Step 3: Fix C2** — generate real `_runEffect_N()` method body, call from `initState`, re-run from `didUpdateWidget` after deps equality check

- [ ] **Step 4: Fix C3** — emit `const CompName({` instead of `const {`

- [ ] **Step 5: Fix C4** — build `store_provider_map: FxHashMap<String, String>` mapping store aliases to provider variable names. Thread through `view.rs` and `expr.rs` for `StoreFieldAccessExpr` → `ref.watch(provider.select((s) => s.field))`

- [ ] **Step 6: Fix C5** — `css_hex_to_dart_color()` conversion function for `#ff0000` → `Color(0xFFFF0000)`

- [ ] **Step 7: Fix C6** — per-widget parameter whitelist, unmapped props → compiler warning

- [ ] **Step 8: Test, snapshots, commit**

```bash
cargo test -p omnilang -- codegen::flutter 2>&1 && cargo insta review
git commit -m "refactor(flutter): Riverpod 3.0 + 6 CRITICAL fixes + design gate removal"
```

### Task 5.2: Flutter ref codegen — GlobalKey + on-demand controllers

**Files:**
- Create: `omnilang/src/codegen/flutter/ref.rs`
- Modify: `omnilang/src/codegen/flutter/mod.rs`, `view.rs`, `expr.rs`

- [ ] **Step 1: Analyze ref usage** — scan component expressions for `RefOpExpr` referencing each ref. Set flags: `needs_focus`, `needs_scroll`, `needs_text`, `needs_measure`

- [ ] **Step 2: Generate per-ref declarations** — `GlobalKey`, on-demand `ScrollController`/`FocusNode`/`TextEditingController`

- [ ] **Step 3: Generate `initState` controller extraction** — `addPostFrameCallback` to extract controllers from `GlobalKey.currentState`

- [ ] **Step 4: Generate `dispose()`** — for all non-null controllers

- [ ] **Step 5: Generate semantic binding handling** — `focus={sig}` → post-frame `requestFocus()`, `scroll-y={sig}` → `animateTo()`, `value={sig}` → TextEditingController listener

- [ ] **Step 6: Wire `@ref={name}` → `key: _nameKey` on Widget constructors**

- [ ] **Step 7: Implement Dart RefOpExpr formatting** — `requestFocus()`, `animateTo()`, `RenderBox.localToGlobal()` + `size`, `TextSelection`

- [ ] **Step 8: Commit**

```bash
git commit -m "feat(flutter): implement ref codegen — GlobalKey + on-demand controllers"
```

### Task 5.3: Flutter slot codegen

**Files:**
- Modify: `omnilang/src/codegen/flutter/view.rs`, `mod.rs`

- [ ] **Step 1: Add `Widget?` params for each slot to Widget constructor**

- [ ] **Step 2: Render `widget.{name} ?? FallbackWidget()` in build**

- [ ] **Step 3: Commit**

```bash
git commit -m "feat(flutter): implement slot codegen with Widget? params + fallback"
```

### Task 5.4: Flutter TweenSequence + semantic animation

**Files:**
- Modify: `omnilang/src/codegen/flutter/motion.rs`, `mod.rs`, `motion_widgets.rs`

- [ ] **Step 1: Per-property Tween/TweenSequence** — 2 stops→`Tween`, 3+ stops→`TweenSequence` with weight-proportional items

- [ ] **Step 2: Semantic property → Flutter widget mapping** — `opacity`→`Opacity`, `scale`→`Transform.scale`, `translateX/Y`→`Transform.translate`, `rotate`→`Transform.rotate`, `backgroundColor`→`Container(color:)`, `color`→`TextStyle`

- [ ] **Step 3: `AnimatedBuilder` chain** — nest widget wrappers for multi-property animations

- [ ] **Step 4: Playback properties** — `loop`→`repeat(count:)`, `direction`→`forward()`/`reverse()`/`repeat(reverse:true)`, `fill`→Tween end value, `delay`→`Future.delayed`

- [ ] **Step 5: Duration parsing** — `300ms`/`1.5s`/`0.5` → `Duration(milliseconds: N)`

- [ ] **Step 6: Easing → Curves mapping** — `ease-in`→`Curves.easeIn`, `cubic-bezier(...)`→`Cubic(...)`, `steps()`→error diagnostic

- [ ] **Step 7: Diagnostics** — `width`/`height` warning, `steps()` error

- [ ] **Step 8: Commit**

```bash
git commit -m "feat(flutter): implement TweenSequence + semantic animation properties"
```

### Task 5.5: Flutter design token codegen

**Files:**
- Create: `omnilang/src/codegen/flutter/design.rs`
- Modify: `omnilang/src/codegen/flutter/mod.rs`

- [ ] **Step 1: Generate typed Dart constants** — `AppTokens` class for all static tokens

- [ ] **Step 2: Generate `BoxDecoration` factories** — `AppDesign.cardDecoration()` for classes

- [ ] **Step 3: Generate `ThemeExtension<T>`** — `copyWith` + `lerp` + static const branches for variants

- [ ] **Step 4: Generate theme switching** — `StateProvider<ThemeMode>` + `MaterialApp(themeMode:)`

- [ ] **Step 5: Generate dynamic token `Provider`** — `ref.watch(dynamicStore.select(...))` → `ThemeData.copyWith`

- [ ] **Step 6: Commit**

```bash
git commit -m "feat(flutter): implement design token codegen — ThemeData + ThemeExtension"
```

---

## Phase 6: OmniView Codegen Updates

> Debug runtime must match new semantics.

### Task 6.1: Update OmniView for all semantic changes

**Files:**
- Modify: `omnilang/src/codegen/omniview/view/element.rs`, `mod.rs`, `motion.rs`, `expr.rs`

- [ ] **Step 1: Semantic ref bindings → `__omni_effect` wrappers** for `focus`, `scroll-y`, `scroll-x`, `value`

- [ ] **Step 2: RefOpExpr formatting** — all 12 operations map to native DOM APIs

- [ ] **Step 3: Slot fallback rendering** — `childrenMap[name] || fallbackHTML`

- [ ] **Step 4: Remove `paused_override`/`@paused` attribute emission**

- [ ] **Step 5: Add `@focus` trigger** — `focusin` event listener in motion.rs

- [ ] **Step 6: Test, snapshots, commit**

```bash
cargo test -p omnilang -- codegen::omniview 2>&1 && cargo insta review
git commit -m "fix(omniview): update debug runtime for semantic DSL changes"
```

---

## Phase 7: Non-P0 Target Updates

### Task 7.1: Update all non-P0 backends for IR changes + design block error

**Files:**
- Modify: `omnilang/src/codegen/mod.rs`
- Modify: `omnilang/src/codegen/{javascript,dart,python,rust,vue,dioxus}/expr.rs`

- [ ] **Step 1: Non-P0 design gate: warning → hard error** for Python, JS, TS, Rust, Dart, Vue, Dioxus

- [ ] **Step 2: Add `RefOpExpr` fallback** to all non-P0 expr formatters — emit diagnostic comment

- [ ] **Step 3: Fix `ViewTree::Slot` match arms** — update from `Slot(name)` to `Slot { name, .. }`

- [ ] **Step 4: Build all backends, fix compilation, test, commit**

```bash
cargo build -p omnilang --all-features 2>&1 && cargo test -p omnilang 2>&1
git commit -m "fix(non-p0): update all backends for IR changes + design block hard error"
```

---

## Phase 8: CI Gates & Testing

### Task 8.1: Expand parameterized test matrix

**Files:**
- Create: fixture `.omni` files in test directory
- Modify: `omnilang/src/codegen/tests_regression/parameterized.rs`

- [ ] **Step 1: Create fixtures** — `design_tokens.omni`, `slots.omni`, `refs.omni`, `animation.omni`, `store.omni`

- [ ] **Step 2: Add cross-target compilation assertions** — each fixture compiles to React, Flutter, OmniView with zero errors and expected output files

- [ ] **Step 3: Commit**

### Task 8.2: Add CI TypeScript + Dart validation [∥]

- [ ] **Step 1: `tsc --noEmit`** on React regression fixture output in CI

- [ ] **Step 2: `dart analyze`** on Flutter regression fixture output in CI

- [ ] **Step 3: Commit**

---

## Phase 9: Demo Project Migration

### Task 9.1: Migrate animation syntax (11 files)

**Files:** All 8 demo pages + 5 React fixtures — see File Structure Map for paths.

- [ ] **Step 1: Rewrite motion body** — `over:`/`easing:` colon syntax, duration units, easing kebab-case

- [ ] **Step 2: Rename triggers** — `@inView`→`@in-view`, `@dragHorizontal`→`@drag-horizontal`, sub-attrs → independent attrs

- [ ] **Step 3: Keyframe properties** — `x`/`y`→`translateX`/`translateY`, CSS-only properties→remove or add to whitelist

- [ ] **Step 4: Handle `AnimPropertiesDemo`** — remove non-semantic CSS property demos (blur, brightness, borderWidth, etc.) or add `--*` passthrough

- [ ] **Step 5: Handle `CssVarsDemo`** — allow `--custom-property` names in whitelist

- [ ] **Step 6: Remove `@paused={...}`** from `KeyframeAnimationDemo.omni`

- [ ] **Step 7: Fix `steps()` syntax** in `EasingFunctionsDemo.omni`

- [ ] **Step 8: Commit**

### Task 9.2: Migrate ref syntax (ElementRefDemo)

- [ ] **Step 1: `ref.style.*` → signal-driven `style:{...}` bindings**

- [ ] **Step 2: `ref.focus()/blur()` → `focus={sig}` binding**

- [ ] **Step 3: `ref.scrollTo/scrollBy` → `scroll-y={sig}` binding**

- [ ] **Step 4: `ref.getBoundingClientRect()` → `ref.measure()`**

- [ ] **Step 5: `ref.offsetWidth/Height` → `ref.measure().width/.height`**

- [ ] **Step 6: Commit**

### Task 9.3: Migrate slot syntax (4 component files)

- [ ] **Step 1: Move `@slot` from view to component body**

- [ ] **Step 2: `@slot(name)` → `@slot name`**

- [ ] **Step 3: Consumer `slot="name"` → named props `name={<div/>}`**

- [ ] **Step 4: Commit**

### Task 9.4: Fix @if mode syntax (2 files)

- [ ] **Step 1: `mode=` → `mode:` separator, validate values (fade/wait/none)**

- [ ] **Step 2: Commit**

### Task 9.5: Regenerate golden fixtures + build output

- [ ] **Step 1: Copy migrated sources to `tests/fixtures/golden/` mirror**

- [ ] **Step 2: Regenerate `build2/` output with updated OmniView codegen**

- [ ] **Step 3: Commit**

---

## Phase 10: Governance Update

### Task 10.1: Update ADR-020, ADR-021, deferrals

- [ ] **Step 1: ADR-020** → status **Implemented** (Riverpod 3.0)

- [ ] **Step 2: ADR-021** → "semantically equivalent behavior for documented feature floor + hard diagnostics"

- [ ] **Step 3: `.architecture/deferrals.md`** → close completed items, add any new deferred items

- [ ] **Step 4: Commit**

---

## Implementation Order

```
Phase 0:  0.1 → 0.2, 0.3, 0.4          [∥ after 0.1]
Phase 1:  1.1 → 1.2, 1.3                [∥ after 1.1]
Phase 2:  2.1, 2.2, 2.3                 [∥]
Phase 3:  3.1, 3.2                       [∥]
Phase 4:  4.1 → 4.2 → 4.3 → 4.4 → 4.5   [sequential]
Phase 5:  5.1 → 5.2 → 5.3 → 5.4 → 5.5   [∥ Phase 4]
Phase 6:  6.1                            [after 4+5]
Phase 7:  7.1                            [∥ Phase 6]
Phase 8:  8.1, 8.2                       [∥, after 4+5+6]
Phase 9:  9.1 → 9.2 → 9.3 → 9.4 → 9.5   [after 8]
Phase 10: 10.1                           [last]
```

**Critical path**: 0.1→1.1→2.1→3.1→4.1-4.5→6.1→8.1→9.x→10.1 (~17 sequential tasks)

**Parallelizable**: Phase 4 (React) ∥ Phase 5 (Flutter). Tasks within Phase 0,1,2,3,8 can run in parallel.

**Total**: 35 tasks across 10 phases. No deferred work. No placeholders.
