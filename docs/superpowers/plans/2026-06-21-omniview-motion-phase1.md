# OmniView Motion Animation — Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the core OmniView Motion animation system — @motion definitions, @timeline cross-element orchestration, element triggers, stagger, @if/@for animation integration, callbacks, and cross-platform codegen to Web (WAAPI + JS runtime) and Flutter (AnimationController + Tween).

**Architecture:** Extends the existing omni-compiler pipeline (lexer → parser → AST → IR → codegen) with animation-specific nodes at each layer. The Web runtime embeds a JS animation engine (~500 lines) with WAAPI integration and spring physics. Flutter codegen maps motion constructs to AnimationController/Tween. All validation is compile-time; zero runtime animation errors are possible.

**Tech Stack:** Rust (compiler), JavaScript (Web runtime), Dart (Flutter codegen output), WAAPI, Flutter AnimationController

---

## File Map

| Layer | File | What It Does |
|-------|------|-------------|
| Tokens | `omni-compiler/src/lexer/token.rs` | Add motion token variants |
| AST | `omni-compiler/src/parser/ast.rs` | Add MotionDef, animation trigger variants |
| Parser | `omni-compiler/src/parser/view.rs` | Parse motion attributes on elements |
| Parser | `omni-compiler/src/parser/top_level.rs` | Parse @motion blocks |
| Parser | `omni-compiler/src/parser/expr.rs` | Parse easing functions, stagger calls |
| IR | `omni-compiler/src/ir/mod.rs` | Add MotionIntent, TriggerIntent IR types |
| Semantic | `omni-compiler/src/semantic/mod.rs` | Motion validation hooks |
| Semantic | `omni-compiler/src/semantic/lints.rs` | Animation-specific lints |
| Codegen | `omni-compiler/src/codegen/omniview.rs` | Web codegen for all motion features |
| Codegen | `omni-compiler/src/codegen/flutter.rs` | Flutter codegen for all motion features |
| Codegen | `omni-compiler/src/codegen/mod.rs` | Shared tag mapping, runtime module |
| Runtime | `build/omni-motion-runtime.js` | JS animation engine (~500 lines) |
| Tests | `omni-compiler/tests/codegen_tests.rs` | Compilation output tests |
| Tests | `omni-compiler/tests/semantic_tests.rs` | Validation tests |
| E2E | `test/projects/learn-omniview/src/` | Test .omni files |

---

### Task 1: Lexer — Add Motion Tokens

**Files:**
- Modify: `omni-compiler/src/lexer/token.rs`

**Purpose:** Add token variants needed to parse animation syntax.

- [ ] **Step 1: Add only @-prefixed tokens to TokenKind**

Motion keywords (`to`, `from`, `set`, `then`, `with`, `over`, `ease`, `loop`, etc.) are **context-sensitive identifiers**, not lexer tokens. The parser checks identifier string values when inside a `@motion` block or animation expression context. Only `@`-prefixed tokens need lexer-level recognition.

**⚠️ IMPORTANT — Hyphen support**: The current lexer's `read_at_keyword()` (line 674 of `lexer/mod.rs`) only reads `alphanumeric + '_'` characters. Hyphenated `@`-keywords like `@drag-horizontal` will NOT parse. You MUST modify the character class to include `'-'`:

```rust
// In read_at_keyword(), change:
while self.peek().is_some_and(|c| c.is_alphanumeric() || c == '_') {
// To:
while self.peek().is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '-') {
```

In `TokenKind` enum, add:

```rust
// Motion definition
AtMotion,        // @motion
AtTimeline,      // @timeline
// Element trigger attributes
AtEnter,         // @enter
AtLeave,         // @leave
AtHover,         // @hover
AtUnhover,       // @unhover
AtPress,         // @press
AtFocus,         // @focus
AtAnimate,       // @animate
AtInView,        // @in-view
AtScroll,        // @scroll
AtDragHorizontal, // @drag-horizontal
AtDragVertical,   // @drag-vertical
AtDrivenBy,       // @driven-by
AtPaused,         // @paused (element-level override)
AtHoverReverse,   // @hover-reverse
AtInViewOnce,     // @in-view-once
AtInViewRepeat,   // @in-view-repeat
AtInViewThreshold, // @in-view-threshold
AtScrollStart,    // @scroll-start (Phase 2)
AtScrollEnd,      // @scroll-end (Phase 2)
AtScrollScrub,    // @scroll-scrub (Phase 2)
// @if transition mode
IfMode,          // @if-mode
// Drag attributes
DragConstraints, // @drag-constraints (Phase 2)
DragElastic,     // @drag-elastic (Phase 2)
DragScrub,       // @drag-scrub (Phase 2)
DragMomentum,    // @drag-momentum (Phase 2)
DragThreshold,   // @drag-threshold (Phase 2)
DragVelocityMax, // @drag-velocity-max (Phase 2)
```

- [ ] **Step 2: Add @-keyword mappings**

Add `@`-prefixed tokens to the existing `@`-keyword matcher in `read_at_keyword()`. Non-`@` keywords go in the regular `read_ident_or_keyword` match table.

```rust
// In read_at_keyword() (after adding '-' to char class):
"@motion" => TokenKind::AtMotion,
"@timeline" => TokenKind::AtTimeline,
"@enter" => TokenKind::AtEnter,
"@leave" => TokenKind::AtLeave,
"@hover" => TokenKind::AtHover,
"@unhover" => TokenKind::AtUnhover,
"@press" => TokenKind::AtPress,
"@focus" => TokenKind::AtFocus,
"@animate" => TokenKind::AtAnimate,
"@in-view" => TokenKind::AtInView,
"@scroll" => TokenKind::AtScroll,
"@drag-horizontal" => TokenKind::AtDragHorizontal,
"@drag-vertical" => TokenKind::AtDragVertical,
"@driven-by" => TokenKind::AtDrivenBy,
"@paused" => TokenKind::AtPaused,
"@hover-reverse" => TokenKind::AtHoverReverse,
"@in-view-once" => TokenKind::AtInViewOnce,
"@in-view-repeat" => TokenKind::AtInViewRepeat,
"@in-view-threshold" => TokenKind::AtInViewThreshold,
"@scroll-start" => TokenKind::AtScrollStart,
"@scroll-end" => TokenKind::AtScrollEnd,
"@scroll-scrub" => TokenKind::AtScrollScrub,

// In read_ident_or_keyword() (snake_case strings):
"if_mode" => TokenKind::IfMode,
"drag_constraints" => TokenKind::DragConstraints,
"drag_elastic" => TokenKind::DragElastic,
"drag_scrub" => TokenKind::DragScrub,
"drag_momentum" => TokenKind::DragMomentum,
"drag_threshold" => TokenKind::DragThreshold,
"drag_velocity_max" => TokenKind::DragVelocityMax,
```

- [ ] **Step 3: Run existing tests to verify no regressions**

```bash
cd omni-compiler && cargo test 2>&1 | tail -5
```
Expected: all existing tests pass (new tokens not yet used by parser, no behavior change).

- [ ] **Step 4: Commit**

```bash
git add omni-compiler/src/lexer/token.rs
git commit -m "feat: add motion animation tokens to lexer"
```

---

### Task 2: AST — Motion Definition Nodes

**Files:**
- Modify: `omni-compiler/src/parser/ast.rs`

**Purpose:** Define all AST node types for the animation system.

- [ ] **Step 1: Add motion definition AST nodes**

Add to `ast.rs`:

```rust
/// Top-level or component-scoped motion definition
#[derive(Debug, Clone)]
pub struct MotionDef {
    pub name: String,
    pub body: MotionBody,
    pub span: Span,
}

/// Cross-element timeline orchestration
#[derive(Debug, Clone)]
pub struct TimelineDef {
    pub name: String,
    pub items: Vec<TimelineItem>,  // reuse timeline item type from MotionBody
    pub span: Span,
}
impl TimelineDef {
    /// Each item references an element `id` + motion name
    /// Syntax: <id> <motion_ref> over:... start:...
}

#[derive(Debug, Clone)]
pub enum MotionBody {
    Keyframe(KeyframeBody),
    Timeline(TimelineBody),
    CallbackOnly(CallbackBody),
}

/// Keyframe mode: percentage stops with shared over/ease
#[derive(Debug, Clone)]
pub struct KeyframeBody {
    pub stops: Vec<KeyframeStop>,
    pub over: Expr,           // shared duration
    pub ease: Option<EasingExpr>,
    pub color_space: Option<String>,
    pub playback: Vec<PlaybackProp>,
}

#[derive(Debug, Clone)]
pub struct KeyframeStop {
    pub position: StopPosition,  // 0%, 40%, 100%, from, to
    pub properties: Vec<(String, Expr)>,  // { opacity: 0; y: 20px }
    pub ease: Option<EasingExpr>,
}

#[derive(Debug, Clone)]
pub enum StopPosition {
    From,              // syntactic sugar for 0%
    To,                // syntactic sugar for 100%
    Percent(f64),      // 0.0 to 100.0
}

/// Timeline mode: sequential/parallel items with independent timing
#[derive(Debug, Clone)]
pub struct TimelineBody {
    pub items: Vec<TimelineItem>,
    pub playback: Vec<PlaybackProp>,
}

#[derive(Debug, Clone)]
pub struct TimelineItem {
    pub kind: TimelineItemKind,
    pub connector: Connector,     // then, with, or None (first item)
}

#[derive(Debug, Clone)]
pub enum TimelineItemKind {
    Direction(DirectionItem),
    SetBlock(Vec<(String, Expr)>),         // set { ... }
    MotionRef(String),                      // reference named motion
    Stagger(StaggerCall),
    Label(String, Option<Expr>),            // label "name" onEnter: { ... }
}

#[derive(Debug, Clone)]
pub struct DirectionItem {
    pub direction: Direction,
    pub properties: Vec<(String, Expr)>,
    pub over: Expr,
    pub ease: Option<EasingExpr>,
    pub delay: Option<Expr>,
    pub color_space: Option<String>,
    pub start: Option<PositionValue>,
}

#[derive(Debug, Clone)]
pub enum Direction {
    To,
    FromTo,
}

#[derive(Debug, Clone)]
pub enum Connector {
    None,       // first item
    Then,       // sequential
    With,       // parallel
}

#[derive(Debug, Clone)]
pub enum PositionValue {
    After,             // ">"  (default)
    WithPrev,          // "<"  (same as previous)
    Overlap(String),   // "-=0.1s"
    AfterPrev(String), // "+=0.2s"  
    WithOffset(String),// "<+0.3s"
    Absolute(String),  // "1.5s"
}

/// Callback-only mode
#[derive(Debug, Clone)]
pub struct CallbackBody {
    pub callbacks: Vec<CallbackSpec>,
    pub over: Expr,
    pub playback: Vec<PlaybackProp>,
}

#[derive(Debug, Clone)]
pub enum CallbackSpec {
    OnStart(Expr),
    OnUpdate(Expr),
    OnComplete(Expr),
    OnRepeat(Expr),
    OnInterrupt(Expr),
}

#[derive(Debug, Clone)]
pub enum PlaybackProp {
    Loop(LoopValue),
    Direction(DirectionValue),
    Fill(FillValue),
    Delay(Expr),
    Paused(bool),
    RepeatDelay(Expr),
    RepeatRefresh(bool),
    TimeScale(Expr),
    Interrupt(InterruptValue),
    RespectReducedMotion(bool),
    When(WhenValue),
    DelayChildren(Expr),
    StaggerChildren(Expr),
    StaggerFrom(StaggerDirection),
}

#[derive(Debug, Clone)]
pub enum LoopValue { Count(u32), Infinite }

#[derive(Debug, Clone)]
pub enum DirectionValue { Normal, Reverse, Alternate, AlternateReverse }

#[derive(Debug, Clone)]
pub enum FillValue { None, Forwards, Backwards, Both }

#[derive(Debug, Clone)]
pub enum InterruptValue { Stop, Complete, Queue, Replace }

#[derive(Debug, Clone)]
pub enum WhenValue { BeforeChildren, AfterChildren, WithChildren }

/// Easing expression
#[derive(Debug, Clone)]
pub enum EasingExpr {
    Named(String),        // linear, easeIn, power2Out, bounceOut, etc.
    CubicBezier { x1: f64, y1: f64, x2: f64, y2: f64 },
    Spring { tension: f64, friction: f64, velocity: Option<f64> },
    Back { intensity: f64, direction: Option<String> },
    Elastic { amplitude: f64, period: f64, direction: Option<String> },
    Steps { count: u32, direction: String },
}

/// Stagger call
#[derive(Debug, Clone)]
pub struct StaggerCall {
    pub motion: Box<Expr>,     // motion reference or inline tween expression
    pub timing: StaggerTiming,
    pub from: StaggerDirection,
}

#[derive(Debug, Clone)]
pub enum StaggerTiming {
    Each(Expr),         // interval mode
    Amount(Expr),       // distribute mode
}

#[derive(Debug, Clone)]
pub enum StaggerDirection {
    Start, Center, End, Edges,
}
```

- [ ] **Step 2: Add animation trigger variants to existing ElementProp**

```rust
pub enum ElementProp {
    // ... existing variants
    MotionTrigger(MotionTrigger),
    StatesMap(Vec<(String, StateDef)>),
    AnimateSignal(Expr),
}

pub struct MotionTrigger {
    pub trigger: TriggerKind,
    pub motion_expr: Expr,      // motion reference or inline tween
}

pub enum TriggerKind {
    Enter, Leave, Hover, Unhover, Press, Focus,
    Animate, InView, Scroll,
    DragHorizontal, DragVertical,
    Timeline,  // for @timeline attribute
}

pub struct StateDef {
    pub name: String,
    pub properties: Vec<(String, Expr)>,
    pub over: Option<Expr>,
    pub ease: Option<EasingExpr>,
}
```

- [ ] **Step 3: Add @if mode field to IfDirective**

```rust
pub struct IfDirective {
    // ... existing fields
    pub transition_mode: Option<String>,  // "sync" | "wait" | None (default sync)
}
```

- [ ] **Step 4: Run existing tests**

```bash
cd omni-compiler && cargo test 2>&1 | tail -5
```
Expected: compilation succeeds (new types added but not yet used in parser).

- [ ] **Step 5: Commit**

```bash
git add omni-compiler/src/parser/ast.rs
git commit -m "feat: add motion animation AST node types"
```

---

### Task 3: Parser — @motion Block Definitions

**Files:**
- Modify: `omni-compiler/src/parser/top_level.rs`
- Modify: `omni-compiler/src/parser/expr.rs`
- Modify: `omni-compiler/src/parser/mod.rs`

**Purpose:** Parse `@motion NAME { ... }` blocks at both top-level and component scope.

- [ ] **Step 0: Add missing parser helper methods**

Motion parsing needs these helpers. Add to `parser/mod.rs` or `stmt.rs`:

```rust
impl<'a> Parser<'a> {
    /// Peek at the string value of the next identifier without consuming it.
    /// Handles both TokenKind::Ident(name) and keyword tokens via keyword_as_ident_name().
    pub fn peek_ident_value(&self) -> Option<String> {
        let tok = self.peek();
        match &tok.kind {
            TokenKind::Ident(name) => Some(name.clone()),
            other => {
                let kw = self.keyword_as_ident_name(other.clone());
                Some(kw)
            }
        }
    }
}
```

Other helpers are inline — no separate `parse_number()`, `parse_keyed_number()`, or `expect_ident_eq()` exist. Use `expect_ident() + string comparison` and `parse_expr()` directly.

- [ ] **Step 1: Add easing function parser**

In `omni-compiler/src/parser/expr.rs`, add:

```rust
impl<'a> Parser<'a> {
    /// Parse easing expression: easeOut, power2In, backOut(intensity:1.7), cubic(x1:0,y1:0,x2:1,y2:1), spring(tension:200,friction:15), steps(count:5,direction:end)
    pub fn parse_easing(&mut self) -> ParseResult<EasingExpr> {
        let name = self.expect_ident()?;
        // Match against known easing names
        let easing = match name.as_str() {
            "linear" => EasingExpr::Named("linear".into()),
            "easeIn" | "easeOut" | "easeInOut" => EasingExpr::Named(name.clone()),
            "power1In" | "power1Out" | "power1InOut" |
            "power2In" | "power2Out" | "power2InOut" |
            "power3In" | "power3Out" | "power3InOut" |
            "power4In" | "power4Out" | "power4InOut" |
            "bounceIn" | "bounceOut" | "bounceInOut" => EasingExpr::Named(name.clone()),
            "backIn" | "backOut" | "backInOut" => {
                let intensity = if self.peek_kind() == TokenKind::LParen {
                    self.advance(); // '('
                    self.expect_ident_eq("intensity")?;
                    self.expect_token(TokenKind::Colon, "expected ':'")?;
                    let val = self.parse_number()?;
                    self.expect_token(TokenKind::RParen, "expected ')'")?;
                    val
                } else { 1.70158 };
                let dir = Some(name.clone()); // "backOut" -> direction is encoded in name
                EasingExpr::Back { intensity, direction: Some(name) }
            },
            "elasticIn" | "elasticOut" | "elasticInOut" => {
                if self.peek_kind() == TokenKind::LParen {
                    self.parse_elastic_params()
                } else {
                    EasingExpr::Elastic { amplitude: 1.0, period: 0.5, direction: Some(name) }
                }
            },
            "cubic" => {
                self.expect_token(TokenKind::LParen, "expected '('")?;
                let x1 = self.parse_keyed_number("x1")?;
                self.expect_token(TokenKind::Comma, "expected ','")?;
                let y1 = self.parse_keyed_number("y1")?;
                self.expect_token(TokenKind::Comma, "expected ','")?;
                let x2 = self.parse_keyed_number("x2")?;
                self.expect_token(TokenKind::Comma, "expected ','")?;
                let y2 = self.parse_keyed_number("y2")?;
                self.expect_token(TokenKind::RParen, "expected ')'")?;
                EasingExpr::CubicBezier { x1, y1, x2, y2 }
            },
            "spring" => {
                self.expect_token(TokenKind::LParen, "expected '('")?;
                let tension = self.parse_keyed_number("tension")?;
                self.expect_token(TokenKind::Comma, "expected ','")?;
                let friction = self.parse_keyed_number("friction")?;
                let velocity = if self.peek_kind() == TokenKind::Comma {
                    self.advance();
                    Some(self.parse_keyed_number("velocity")?)
                } else { None };
                self.expect_token(TokenKind::RParen, "expected ')'")?;
                EasingExpr::Spring { tension, friction, velocity }
            },
            "steps" => {
                self.expect_token(TokenKind::LParen, "expected '('")?;
                let count = self.parse_keyed_number("count")? as u32;
                self.expect_token(TokenKind::Comma, "expected ','")?;
                self.expect_ident_eq("direction")?;
                self.expect_token(TokenKind::Colon, "expected ':'")?;
                let dir = self.expect_ident()?;
                self.expect_token(TokenKind::RParen, "expected ')'")?;
                EasingExpr::Steps { count, direction: dir }
            },
            _ => return Err(self.error(format!("unknown easing: '{}'", name))),
        };
        Ok(easing)
    }
}
```

- [ ] **Step 2: Add @motion block parser in top_level.rs**

```rust
impl<'a> Parser<'a> {
    /// Parse @motion NAME { body }
    pub fn parse_motion_def(&mut self) -> ParseResult<MotionDef> {
        let span_start = self.span();
        self.expect_token(TokenKind::AtMotion, "expected '@motion'")?;
        let name = self.expect_ident()?;
        self.expect_token(TokenKind::LBrace, "expected '{'")?;

        let body = self.parse_motion_body()?;

        self.expect_token(TokenKind::RBrace, "expected '}'")?;
        let span = self.span_from(span_start);
        Ok(MotionDef { name, body, span })
    }

    fn parse_motion_body(&mut self) -> ParseResult<MotionBody> {
        // First token determines mode.
        // NOTE: `from` is TokenKind::From keyword, NOT Ident("from").
        // Use peek_ident_value() which handles keywords via keyword_as_ident_name().
        let first = self.peek_ident_value().ok_or(self.error("expected motion body"))?;
        match first.as_str() {
            "from" | "to" => {
                // Keyframe mode. Parser checks: if body later contains "then"/"with"/"start:",
                // that's a compile error (mixed modes).
                self.parse_keyframe_body()
            },
            "set" | "stagger" | "label" => self.parse_timeline_body(),
            "onStart" | "onUpdate" | "onComplete" | "onRepeat" | "onInterrupt" => {
                self.parse_callback_body()
            },
            _ => self.parse_timeline_body(), // IDENT → motion reference
        }
    }
    // ... keyframe/timeline/callback body parsers
}
```

- [ ] **Step 3: Add @timeline parser**

```rust
impl<'a> Parser<'a> {
    pub fn parse_timeline_def(&mut self) -> ParseResult<TimelineDef> {
        let span_start = self.span();
        self.expect_token(TokenKind::AtTimeline, "expected '@timeline'")?;
        let name = self.expect_ident()?;
        self.expect_token(TokenKind::LBrace, "expected '{'")?;
        // Parse items: <id> <motion_ref> over:... start:...
        let mut items = Vec::new();
        let mut connector = Connector::None;
        while self.peek_kind() != TokenKind::RBrace {
            let id = self.expect_ident()?;       // element id
            let motion = self.expect_ident()?;   // motion name
            let over = self.parse_optional_timing()?;
            let start = self.parse_optional_start()?;
            items.push(TimelineItem { kind: TimelineItemKind::Direction(DirectionItem {
                direction: Direction::To,
                properties: vec![],
                over: over.unwrap_or(/* error */),
                ease: /* ... */,
                delay: None,
                color_space: None,
                start,
            }), connector: connector.clone() });
            connector = self.parse_connector()?;
        }
        self.expect_token(TokenKind::RBrace, "expected '}'")?;
        Ok(TimelineDef { name, items, span: self.span_from(span_start) })
    }
}
```

- [ ] **Step 4: Add stagger() custom parser (named params bypass Pratt)**

`stagger(itemIn, each:60ms, from:center)` uses colon-delimited named params. The Pratt parser treats `:` as an invalid infix operator. Add to `expr.rs`:

```rust
fn parse_stagger_call(&mut self) -> ParseResult<StaggerCall> {
    self.expect_ident_eq("stagger")?; // already consumed by caller
    self.expect_token(TokenKind::LParen, "expected '('")?;
    let motion = self.parse_motion_expr()?;   // first positional: motion ref or inline
    self.expect_token(TokenKind::Comma, "expected ','")?;
    // Named params: each:60ms or amount:600ms
    let timing = match self.expect_ident()?.as_str() {
        "each" => { self.expect_token(TokenKind::Colon, "expected ':'")?; StaggerTiming::Each(self.parse_expr()?) }
        "amount" => { self.expect_token(TokenKind::Colon, "expected ':'")?; StaggerTiming::Amount(self.parse_expr()?) }
        n => return Err(self.error(format!("expected 'each' or 'amount', got '{}'", n))),
    };
    let from = if self.peek_kind() == TokenKind::Comma {
        self.advance();
        self.expect_ident_eq("from")?;
        self.expect_token(TokenKind::Colon, "expected ':'")?;
        match self.expect_ident()?.as_str() {
            "start" => StaggerDirection::Start, "center" => StaggerDirection::Center,
            "end" => StaggerDirection::End, "edges" => StaggerDirection::Edges,
            _ => return Err(self.error("expected start|center|end|edges")),
        }
    } else { StaggerDirection::Start };
    self.expect_token(TokenKind::RParen, "expected ')'")?;
    Ok(StaggerCall { motion: Box::new(motion), timing, from })
}
```

Hook: in `parse_ident_expr()` (expr.rs), before the LParen function-call check, add `if name == "stagger" && peek == LParen → return parse_stagger_call()`.

- [ ] **Step 5: Register @motion and @timeline in top_level.rs (NOT mod.rs)**

In `parser/top_level.rs`, hook `parse_motion_def` and `parse_timeline_def` into `parse_module()` (for top-level) and `parse_component_def()` (for component-scoped).

- [ ] **Step 4: Run tests**

```bash
cd omni-compiler && cargo test 2>&1 | tail -5
```

- [ ] **Step 5: Commit**

```bash
git add omni-compiler/src/parser/
git commit -m "feat: add @motion block parser with easing function support"
```

---

### Task 4: Parser — Element Motion Triggers

**Files:**
- Modify: `omni-compiler/src/parser/view.rs`

**Purpose:** Parse `@enter={expr}`, `@hover={expr}`, `states: {...}`, `on: {signal}` on elements.

- [ ] **Step 1: Add trigger attribute parsing**

In `parse_element()` prop loop:

```rust
// After existing @-prop handling
TokenKind::AtEnter => {
    let trigger = self.parse_motion_trigger(TriggerKind::Enter)?;
    props.push(ElementProp::MotionTrigger(trigger));
}
TokenKind::AtLeave => {
    let trigger = self.parse_motion_trigger(TriggerKind::Leave)?;
    props.push(ElementProp::MotionTrigger(trigger));
}
TokenKind::AtHover => {
    let trigger = self.parse_motion_trigger(TriggerKind::Hover)?;
    props.push(ElementProp::MotionTrigger(trigger));
}
// ... similarly for AtPress, AtFocus, AtAnimate, AtInView, AtScroll,
//     AtDragHorizontal, AtDragVertical, AtDrivenBy

TokenKind::AtTimeline => {
    let trigger = self.parse_motion_trigger(TriggerKind::Timeline)?;
    props.push(ElementProp::MotionTrigger(trigger));
}

// @paused — boolean override for motion's paused value
TokenKind::AtPaused => {
    self.advance();
    self.expect_token(TokenKind::Eq, "expected '='")?;
    self.expect_token(TokenKind::LBrace, "expected '{'")?;
    let val = matches!(self.peek_kind(), TokenKind::True);
    self.parse_expr()?;  // consume the bool
    self.expect_token(TokenKind::RBrace, "expected '}'")?;
    props.push(ElementProp::PausedOverride(val));
}

// @hover-reverse — "smooth" or "snap"
TokenKind::AtHoverReverse => {
    self.advance();
    self.expect_token(TokenKind::Eq, "expected '='")?;
    self.expect_token(TokenKind::LBrace, "expected '{'")?;
    let mode = self.expect_ident()?;
    self.expect_token(TokenKind::RBrace, "expected '}'")?;
    props.push(ElementProp::HoverReverse(mode));
}

// @in-view-once, @in-view-repeat, @in-view-threshold
TokenKind::AtInViewOnce | TokenKind::AtInViewRepeat | TokenKind::AtInViewThreshold => {
    // Parse as boolean or number expression
    self.advance();
    self.expect_token(TokenKind::Eq, "expected '='")?;
    self.expect_token(TokenKind::LBrace, "expected '{'")?;
    let val = self.parse_expr()?;
    self.expect_token(TokenKind::RBrace, "expected '}'")?;
    // Push as appropriate ElementProp variant
}
```

- [ ] **Step 2: Implement motion trigger value parser**

```rust
fn parse_motion_trigger(&mut self, kind: TriggerKind) -> ParseResult<MotionTrigger> {
    self.expect_token(TokenKind::Eq, "expected '='")?;
    self.expect_token(TokenKind::LBrace, "expected '{'")?;
    let motion_expr = self.parse_motion_expr()?;
    self.expect_token(TokenKind::RBrace, "expected '}'")?;
    Ok(MotionTrigger { trigger: kind, motion_expr })
}

fn parse_motion_expr(&mut self) -> ParseResult<Expr> {
    // Motion expression: either a motion reference (IDENT)
    // or an inline tween (to { ... } over: 300ms)
    // or a stagger call (stagger(...))
    match self.peek_ident_value().ok().as_deref() {
        Some("to") | Some("from") | Some("set") => {
            // Inline tween literal
            self.parse_inline_motion()
        },
        Some("stagger") => {
            self.parse_stagger_call()
        },
        _ => {
            // Named motion reference
            self.parse_expr()
        }
    }
}
```

- [ ] **Step 3: Add states: and on: colon prop parsing**

```rust
// In parse_element_prop():
if name == "states" && self.peek_kind() == TokenKind::Colon {
    self.advance(); // ':'
    self.expect_token(TokenKind::LBrace, "expected '{'")?;
    let mut states = Vec::new();
    while self.peek_kind() != TokenKind::RBrace {
        let state_name = self.expect_ident()?;
        self.expect_token(TokenKind::Colon, "expected ':'")?;
        // Parse property block + optional over/ease
        let props = self.parse_property_block()?;
        let over = if self.peek_is_ident("over") {
            self.advance(); self.expect_token(TokenKind::Colon, "expected ':'")?;
            Some(self.parse_expr()?)
        } else { None };
        let ease = if self.peek_is_ident("ease") {
            self.advance(); self.expect_token(TokenKind::Colon, "expected ':'")?;
            Some(self.parse_easing()?)
        } else { None };
        states.push((state_name, StateDef { properties: props, over, ease }));
        if self.peek_kind() == TokenKind::Comma { self.advance(); }
    }
    self.expect_token(TokenKind::RBrace, "expected '}'")?;
    return Ok(ElementProp::StatesMap(states));
}
if name == "on" && self.peek_kind() == TokenKind::Colon {
    self.advance(); // ':'
    self.expect_token(TokenKind::LBrace, "expected '{'")?;
    let signal = self.parse_expr()?;
    self.expect_token(TokenKind::RBrace, "expected '}'")?;
    return Ok(ElementProp::AnimateSignal(signal));
}
```

- [ ] **Step 4: Run tests**

```bash
cd omni-compiler && cargo test 2>&1 | tail -5
```

- [ ] **Step 5: Commit**

```bash
git add omni-compiler/src/parser/view.rs
git commit -m "feat: parse motion triggers and states:/on: on elements"
```

---

### Task 5: IR — Motion Intermediate Representation

**Files:**
- Modify: `omni-compiler/src/ir/mod.rs`

**Purpose:** Define IR types for lowering AST animation nodes.

- [ ] **Step 1: Add motion IR types**

```rust
/// Motion definition IR
#[derive(Debug, Clone)]
pub struct MotionIntent {
    pub name: String,
    pub body: MotionBodyIR,
}

#[derive(Debug, Clone)]
pub enum MotionBodyIR {
    Keyframe(KeyframeIR),
    Timeline(TimelineIR),
    CallbackOnly(CallbackBodyIR),
}

#[derive(Debug, Clone)]
pub struct KeyframeIR {
    pub stops: Vec<KeyframeStopIR>,
    pub over: ExprIR,
    pub ease: Option<EasingIR>,
    pub color_space: Option<String>,
    pub playback: PlaybackIR,
}

#[derive(Debug, Clone)]
pub struct KeyframeStopIR {
    pub position: f64,  // 0.0 to 1.0
    pub properties: HashMap<String, ExprIR>,
    pub ease: Option<EasingIR>,
}

#[derive(Debug, Clone)]
pub struct TimelineIR {
    pub items: Vec<TimelineItemIR>,
    pub playback: PlaybackIR,
}

#[derive(Debug, Clone)]
pub struct TimelineItemIR {
    pub kind: TimelineItemKindIR,
    pub connector: ConnectorIR,
}

#[derive(Debug, Clone)]
pub enum TimelineItemKindIR {
    Direction {
        dir: DirectionIR,
        properties: HashMap<String, ExprIR>,
        over: ExprIR,
        ease: Option<EasingIR>,
        delay: Option<ExprIR>,
        color_space: Option<String>,
        start: Option<String>,  // position string
    },
    Set(Vec<(String, ExprIR)>),
    MotionRef(String),
    Stagger(Box<MotionExprIR>, StaggerTimingIR, StaggerDirection),
    Label(String, Option<ExprIR>),
}

#[derive(Debug, Clone)]
pub enum DirectionIR { To, FromTo }

#[derive(Debug, Clone)]
pub enum ConnectorIR { None, Then, With }

#[derive(Debug, Clone)]
pub struct CallbackBodyIR {
    pub callbacks: Vec<(CallbackKindIR, ExprIR)>,
    pub over: ExprIR,
    pub playback: PlaybackIR,
}

#[derive(Debug, Clone)]
pub enum CallbackKindIR { OnStart, OnUpdate, OnComplete, OnRepeat, OnInterrupt }

#[derive(Debug, Clone)]
pub struct PlaybackIR {
    pub loop_count: Option<LoopValue>,
    pub direction: Option<DirectionValue>,
    pub fill: Option<FillValue>,
    pub delay: Option<ExprIR>,
    pub paused: bool,
    pub repeat_delay: Option<ExprIR>,
    pub repeat_refresh: bool,
    pub time_scale: Option<ExprIR>,
    pub interrupt: Option<InterruptValue>,
    pub respect_reduced_motion: bool,
    pub when: Option<WhenValue>,
    pub delay_children: Option<ExprIR>,
    pub stagger_children: Option<ExprIR>,
    pub stagger_from: Option<StaggerDirection>,
}

/// Easing IR (mirrors AST EasingExpr)
#[derive(Debug, Clone)]
pub enum EasingIR {
    Named(String),
    CubicBezier { x1: f64, y1: f64, x2: f64, y2: f64 },
    Spring { tension: f64, friction: f64, velocity: Option<f64> },
    Back { intensity: f64, direction: Option<String> },
    Elastic { amplitude: f64, period: f64, direction: Option<String> },
    Steps { count: u32, direction: String },
}

/// Element trigger IR
#[derive(Debug, Clone)]
pub struct TriggerIntent {
    pub kind: TriggerKind,
    pub motion_ref: Option<String>,    // named motion reference
    pub inline_motion: Option<MotionBodyIR>,  // or inline motion
}

#[derive(Debug, Clone)]
pub enum TriggerKind {
    Enter, Leave, Hover, Unhover, Press, Focus,
    Animate, InView, Scroll,
    DragHorizontal, DragVertical,
    Timeline,
}

/// State animation IR
#[derive(Debug, Clone)]
pub struct StatesIR {
    pub states: HashMap<String, StateTweenIR>,
    pub signal: Option<ExprIR>,
}

#[derive(Debug, Clone)]
pub struct StateTweenIR {
    pub properties: HashMap<String, ExprIR>,
    pub over: Option<ExprIR>,
    pub ease: Option<EasingIR>,
}
```

- [ ] **Step 2: Add motion fields to existing IR types**

```rust
// In ElementIR, add:
pub motion_triggers: Vec<TriggerIntent>,
pub states: Option<StatesIR>,
pub paused_override: Option<bool>,

// In ConditionalIntent, add:
pub transition_mode: String,  // "sync" | "wait"

// In ListRenderingIntent, key_expr ALREADY EXISTS (line 307 of ir/mod.rs).
// Task: POPULATE it from the @for key={} parser attribute (currently always None).
// No IR field addition needed — only parser + lowering work.

// In ComponentIR, add:
pub motions: Vec<MotionIntent>,

// In ComponentDef (AST), add:
pub motions: Vec<MotionDef>,
```

- [ ] **Step 3: Run tests**

```bash
cd omni-compiler && cargo test 2>&1 | tail -5
```

- [ ] **Step 4: Commit**

```bash
git add omni-compiler/src/ir/mod.rs
git commit -m "feat: add motion animation IR types"
```

---

### Task 6: Lowering — AST → IR for Motion

**Files:**
- Modify: `omni-compiler/src/ir/lower_logic.rs` (or create `omni-compiler/src/ir/lower_motion.rs`)
- Modify: `omni-compiler/src/ir/mod.rs`

**Purpose:** Convert parsed AST animation nodes into IR types. Follow the EXISTING free-function lowering pattern — NO `Lowerer` struct, NO `LowerResult`. All lowering is infallible (returns the IR type directly, no `Result`). Use `variant_map: &HashMap<String, String>` as the context parameter.

- [ ] **Step 1: Lower @motion definitions — free functions**

```rust
// In ir/mod.rs, add after existing lower_* functions:

fn lower_motion_def(def: &MotionDef, variant_map: &HashMap<String, String>) -> MotionIntent {
    let body = match &def.body {
        MotionBody::Keyframe(kf) => MotionBodyIR::Keyframe(lower_keyframe_body(kf, variant_map)),
        MotionBody::Timeline(tl) => MotionBodyIR::Timeline(lower_timeline_body(tl, variant_map)),
        MotionBody::CallbackOnly(cb) => MotionBodyIR::CallbackOnly(lower_callback_body(cb, variant_map)),
    };
    MotionIntent { name: def.name.clone(), body }
}

fn lower_keyframe_body(kf: &KeyframeBody, variant_map: &HashMap<String, String>) -> KeyframeIR {
    let stops: Vec<KeyframeStopIR> = kf.stops.iter().map(|s| {
        let position = match s.position {
            StopPosition::From => 0.0,
            StopPosition::To => 1.0,
            StopPosition::Percent(p) => p / 100.0,
        };
        let properties: HashMap<String, ExprIR> = s.properties.iter()
            .map(|(k, v)| (k.clone(), lower_expr(v, variant_map))).collect();
        let ease = s.ease.as_ref().map(|e| lower_easing(e));
        KeyframeStopIR { position, properties, ease }
    }).collect();
    let over = lower_expr(&kf.over, variant_map);
    let ease = kf.ease.as_ref().map(|e| lower_easing(e));
    let playback = lower_playback_props(&kf.playback, variant_map);
    KeyframeIR { stops, over, ease, color_space: kf.color_space.clone(), playback }
}

fn lower_easing(e: &EasingExpr) -> EasingIR {
    match e {
        EasingExpr::Named(n) => EasingIR::Named(n.clone()),
        EasingExpr::CubicBezier { x1, y1, x2, y2 } => EasingIR::CubicBezier { x1: *x1, y1: *y1, x2: *x2, y2: *y2 },
        EasingExpr::Spring { tension, friction, velocity } => EasingIR::Spring { tension: *tension, friction: *friction, velocity: *velocity },
        EasingExpr::Back { intensity, direction } => EasingIR::Back { intensity: *intensity, direction: direction.clone() },
        EasingExpr::Elastic { amplitude, period, direction } => EasingIR::Elastic { amplitude: *amplitude, period: *period, direction: direction.clone() },
        EasingExpr::Steps { count, direction } => EasingIR::Steps { count: *count, direction: direction.clone() },
    }
}
// ... lower_timeline_body, lower_callback_body, lower_playback_props similarly
```

- [ ] **Step 2: Lower element triggers — extend lower_view_node**

In the existing `lower_view_node()` function (ir/mod.rs, around line 861), extend the `ViewNode::Element` arm to handle new `ElementProp` variants:

```rust
// Inside the ElementProp match in lower_view_node:
ElementProp::MotionTrigger(mt) => {
    let kind = match mt.trigger {
        TriggerKind::Enter => TriggerKind::Enter,
        // ... other mappings
    };
    let (motion_ref, inline_motion) = match &mt.motion_expr {
        Expr::Ident(name) => (Some(name.clone()), None),
        other => (None, Some(lower_inline_motion(other, variant_map))),
    };
    el.motion_triggers.push(TriggerIntent { kind, motion_ref, inline_motion });
}
ElementProp::StatesMap(states) => {
    el.states = Some(lower_states_map(states, variant_map));
}
ElementProp::AnimateSignal(expr) => {
    if let Some(ref mut s) = el.states {
        s.signal = Some(lower_expr(expr, variant_map));
    }
}
```

- [ ] **Step 3: Lower @if mode, @for key, timeline, paused override**

Extend `lower_view_node` branches and `lower_component` to handle:
- `IfDirective.transition_mode` → `ConditionalIntent.transition_mode`
- `ForDirective.key_expr` → POPULATE `ListRenderingIntent.key_expr` (field already exists, line 307)
- `TimelineDef` → lowered as part of component motions
- `ElementProp::PausedOverride` → `el.paused_override`

- [ ] **Step 4: Run tests**

```bash
cd omni-compiler && cargo test 2>&1 | tail -5
```

- [ ] **Step 5: Commit**

```bash
git add omni-compiler/src/ir/
git commit -m "feat: add AST→IR lowering for motion animations"
```

---

### Task 7: Semantic — Motion Validation

**Files:**
- Modify: `omni-compiler/src/semantic/mod.rs`
- Modify: `omni-compiler/src/semantic/lints.rs`

**Purpose:** Implement all 28 compile-time error checks and 9 warnings from spec §20.

- [ ] **Step 1: Write validation test**

In `omni-compiler/tests/semantic_tests.rs`, add:

```rust
#[test]
fn test_motion_invalid_property_type() {
    let source = r#"
        @motion bad {
            to { display: none→block } over: 300ms
        }
    "#;
    let result = compile(source);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("display is not interpolatable"));
}

#[test]
fn test_motion_standalone_from_error() {
    let source = r#"
        @motion bad {
            from { opacity: 0 } over: 300ms
        }
    "#;
    let result = compile(source);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("from must be paired with to"));
}

#[test]
fn test_motion_circular_reference() {
    let source = r#"
        @motion a { b }
        @motion b { a }
    "#;
    let result = compile(source);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("circular reference"));
}

#[test]
fn test_motion_for_without_key() {
    let source = r#"
        component Test {
            view {
                @for item in items {
                    <div @enter={fadeIn} />
                }
            }
        }
    "#;
    let result = compile(source);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("key"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd omni-compiler && cargo test test_motion_invalid -- --nocapture 2>&1 | tail -10
```
Expected: tests fail (validation not yet implemented).

- [ ] **Step 3: Implement property type validation**

In `semantic/mod.rs`:

```rust
/// Validate that a property is animatable
fn validate_property_type(prop_name: &str, from_val: &Expr, to_val: &Expr) -> SemanticResult<()> {
    let prop_type = match prop_name {
        "opacity" | "scale" | "scaleX" | "scaleY" | "scaleZ" => PropertyType::Float,
        "x" | "y" | "translateZ" | "width" | "height" | "borderRadius" | "margin" | "padding" => PropertyType::Length,
        "rotate" | "skewX" | "skewY" | "rotateX" | "rotateY" | "rotateZ" => PropertyType::Angle,
        "color" | "backgroundColor" | "borderColor" | "fill" | "stroke" => PropertyType::Color,
        "strokeWidth" | "strokeDashoffset" | "r" | "cx" | "cy" => PropertyType::Svg,
        "blur" | "brightness" | "contrast" | "saturate" => PropertyType::Filter,
        "boxShadow" => PropertyType::Shadow,
        "clipPath" => PropertyType::Clip,
        "perspective" | "translateZ" | "scaleZ" => PropertyType::ThreeD,
        "display" | "position" | "overflow" | "flexDirection" | "gridTemplate" => {
            return Err(SemanticError::new(format!("'{}' is not interpolatable", prop_name)))
        },
        // SVG path data
        "d" => {
            // Warn if path structure mismatch, but don't error
            validate_path_structure(from_val, to_val)?;
            return Ok(())
        },
        _ => {
            // Unknown properties: warn but allow (could be CSS custom properties)
            return Ok(())
        },
    };
    validate_value_type(from_val, prop_type)?;
    validate_value_type(to_val, prop_type)?;
    Ok(())
}
```

- [ ] **Step 4: Implement all 28 error checks**

Implement validation functions for each error in spec §20.1:
1. `check_interpolatable_property()` 
2. `check_type_match()`
3. `check_no_keyword_value()`
4. `check_from_has_to()`
5. `check_set_in_timeline_only()`
6. `check_no_mixed_modes()`
7. `check_motion_reference_exists()`
8. `check_no_circular_reference()`
9. `check_for_has_key()`
10. `check_driven_by_no_loop()`
11. `check_ease_not_spring()`
12. `check_fill_infinite_loop()`
13. `check_clip_path_same_shape()`
14. `check_box_shadow_structure()`
15. `check_velocity_only_gesture()`
16. `check_motion_not_in_store()`
17. `check_stagger_not_in_keyframe()`
18. `check_valid_easing_name()`
19. `check_when_on_enter_leave_only()`
20. `check_states_has_on()`
21. `check_states_no_reserved_with_on()`
22. `check_if_mode_valid()`
23. `check_drag_constraints_axis()`
24. `check_drag_constraints_no_mixed_parent()`
25. `check_drag_constraints_ref_exists()`
26. `check_drag_elastic_range()`

Also implement 9 warnings from §20.2.

- [ ] **Step 5: Wire validation into compiler pipeline**

In `semantic/mod.rs`, add `validate_motions()` function called during the semantic analysis phase.

- [ ] **Step 6: Run validation tests to verify they pass**

```bash
cd omni-compiler && cargo test semantic_tests 2>&1 | tail -10
```

- [ ] **Step 7: Commit**

```bash
git add omni-compiler/src/semantic/ omni-compiler/tests/semantic_tests.rs
git commit -m "feat: add motion semantic validation (28 errors + 9 warnings)"
```

---

### Task 8: Web Animation Runtime

**Files:**
- Create: `build/omni-motion-runtime.js`

**Purpose:** JavaScript animation engine with WAAPI integration, spring physics, easing functions.

- [ ] **Step 1: Write spring physics solver**

```javascript
// Spring physics solver (analytical closed-form solution)
class SpringSolver {
    constructor(tension, friction, mass = 1, velocity = 0, precision = 0.01) {
        this.tension = tension;
        this.friction = friction;
        this.mass = mass;
        this.velocity = velocity;
        this.precision = precision;
    }

    // Natural frequency ω₀ = sqrt(k/m)
    get omega0() { return Math.sqrt(this.tension / this.mass); }

    // Damping ratio ζ = c / (2*sqrt(k*m))
    get zeta() { return this.friction / (2 * Math.sqrt(this.tension * this.mass)); }

    // Solve position at time t for the damped harmonic oscillator
    solve(from, to, t) {
        const delta = to - from;
        const v0 = this.velocity;
        const zeta = this.zeta;
        const omega0 = this.omega0;

        if (zeta < 1) {
            // Underdamped
            const omegad = omega0 * Math.sqrt(1 - zeta * zeta);
            const envelope = Math.exp(-zeta * omega0 * t);
            const A = delta;
            const B = (v0 + zeta * omega0 * delta) / omegad;
            const pos = to - envelope * (A * Math.cos(omegad * t) + B * Math.sin(omegad * t));
            return pos;
        } else if (Math.abs(zeta - 1) < 1e-6) {
            // Critically damped
            const envelope = Math.exp(-omega0 * t);
            const A = delta;
            const B = v0 + omega0 * delta;
            const pos = to - envelope * (A + B * t);
            return pos;
        } else {
            // Overdamped
            const r1 = -zeta * omega0 + omega0 * Math.sqrt(zeta * zeta - 1);
            const r2 = -zeta * omega0 - omega0 * Math.sqrt(zeta * zeta - 1);
            const c1 = (v0 - r2 * delta) / (r1 - r2);
            const c2 = delta - c1;
            const pos = to - (c1 * Math.exp(r1 * t) + c2 * Math.exp(r2 * t));
            return pos;
        }
    }

    // Find when spring settles (within precision)
    getDuration(from, to) {
        const total = Math.abs(to - from);
        const prec = total < 5 ? this.precision * 0.1 : this.precision;
        let t = 0;
        const dt = 1 / 60;
        for (let i = 0; i < 10000; i++) {
            const pos = this.solve(from, to, t);
            const vel = (this.solve(from, to, t + 0.001) - pos) * 1000; // units/s
            if (Math.abs(pos - to) < prec && Math.abs(vel) < prec) {
                return t * 1000; // return ms
            }
            t += dt;
        }
        return 5000; // fallback: 5 seconds
    }
}
```

- [ ] **Step 2: Write easing function registry**

```javascript
// Easing functions mapped from OmniView names
const EASING_REGISTRY = {
    // Standard
    linear: t => t,
    easeIn: t => 1 - Math.cos((t * Math.PI) / 2),
    easeOut: t => Math.sin((t * Math.PI) / 2),
    easeInOut: t => -(Math.cos(Math.PI * t) - 1) / 2,
    // Power curves (using exact cubic-bezier equivalents)
    power1In: t => t * t,
    power1Out: t => t * (2 - t),
    power1InOut: t => t < 0.5 ? 2 * t * t : -1 + (4 - 2 * t) * t,
    power2In: t => t * t * t,
    power2Out: t => (--t) * t * t + 1,
    power2InOut: t => t < 0.5 ? 4 * t * t * t : (t - 1) * (2 * t - 2) * (2 * t - 2) + 1,
    power3In: t => t * t * t * t,
    power3Out: t => 1 - (--t) * t * t * t,
    power3InOut: t => t < 0.5 ? 8 * t * t * t * t : 1 - 8 * (--t) * t * t * t,
    power4In: t => t * t * t * t * t,
    power4Out: t => 1 + (--t) * t * t * t * t,
    power4InOut: t => t < 0.5 ? 16 * t * t * t * t * t : 1 + 16 * (--t) * t * t * t * t,
    // Back (with configurable intensity)
    backIn: (t, i = 1.70158) => t * t * ((i + 1) * t - i),
    backOut: (t, i = 1.70158) => (--t) * t * ((i + 1) * t + i) + 1,
    backInOut: (t, i = 1.70158) => {
        const s = i * 1.525;
        return t < 0.5 ? (t *= 2) * t * ((s + 1) * t - s) / 2
                        : ((t = t * 2 - 2) * t * ((s + 1) * t + s) + 2) / 2;
    },
    // Bounce
    bounceOut: t => {
        if (t < 1 / 2.75) return 7.5625 * t * t;
        if (t < 2 / 2.75) return 7.5625 * (t -= 1.5 / 2.75) * t + 0.75;
        if (t < 2.5 / 2.75) return 7.5625 * (t -= 2.25 / 2.75) * t + 0.9375;
        return 7.5625 * (t -= 2.625 / 2.75) * t + 0.984375;
    },
    bounceIn: t => 1 - EASING_REGISTRY.bounceOut(1 - t),
    bounceInOut: t => t < 0.5
        ? EASING_REGISTRY.bounceIn(t * 2) * 0.5
        : EASING_REGISTRY.bounceOut(t * 2 - 1) * 0.5 + 0.5,
    // Elastic
    elasticOut: (t, a = 1, p = 0.5) => {
        if (t === 0 || t === 1) return t;
        return Math.pow(2, -10 * t) * Math.sin((t - 1) * (2 * Math.PI) / p) * a + 1;
    },
    elasticIn: (t, a = 1, p = 0.5) => {
        if (t === 0 || t === 1) return t;
        return -(Math.pow(2, 10 * (t - 1)) * Math.sin((t - 1) * (2 * Math.PI) / p) * a);
    },
    elasticInOut: (t, a = 1, p = 0.5) => {
        if (t === 0 || t === 1) return t;
        return t < 0.5
            ? -0.5 * (Math.pow(2, 10 * (t * 2 - 1)) * Math.sin((t * 2 - 1) * (2 * Math.PI) / (p * 1.5)) * a)
            : Math.pow(2, -10 * (t * 2 - 1)) * Math.sin((t * 2 - 1) * (2 * Math.PI) / (p * 1.5)) * a * 0.5 + 1;
    },
};

function getEasingFn(easeConfig) {
    if (typeof easeConfig === 'string') return EASING_REGISTRY[easeConfig] || EASING_REGISTRY.linear;
    if (easeConfig.type === 'cubicBezier') {
        const { x1, y1, x2, y2 } = easeConfig;
        return t => {
            // Cubic bezier solver via Newton's method
            let lo = 0, hi = 1;
            for (let i = 0; i < 8; i++) {
                const mid = (lo + hi) / 2;
                const x = 3 * x1 * mid * (1 - mid) * (1 - mid) + 3 * x2 * mid * mid * (1 - mid) + mid * mid * mid;
                if (x < t) lo = mid; else hi = mid;
            }
            const mid = (lo + hi) / 2;
            return 3 * y1 * mid * (1 - mid) * (1 - mid) + 3 * y2 * mid * mid * (1 - mid) + mid * mid * mid;
        };
    }
    if (easeConfig.type === 'spring') {
        const solver = new SpringSolver(easeConfig.tension, easeConfig.friction, 1, easeConfig.velocity || 0);
        return t => solver.solve(0, 1, t * (solver.getDuration(0, 1) / 1000)) / 1;
    }
    if (easeConfig.type === 'steps') {
        return t => Math.floor(t * easeConfig.count) / easeConfig.count;
    }
    return EASING_REGISTRY.linear;
}
```

- [ ] **Step 3: Write AnimationPlayer class**

```javascript
class AnimationPlayer {
    constructor(el, config) {
        this.el = el;
        this.config = config;
        this.progress = 0;
        this._playing = false;
        this._completed = false;
        this._iteration = 0;
        this._paused = config.paused || false;
        this._animation = null;
        this._callbacks = config.callbacks || {};
    }

    play() {
        if (this._playing && !this._paused) return;
        this._playing = true;
        this._paused = false;
        this._callbacks.onStart?.();
        this._runAnimation();
    }

    pause() { this._paused = true; this._animation?.pause(); }
    resume() { this._paused = false; this._animation?.play(); }
    reverse() { /* reverse playback direction */ }
    seek(p) { this.progress = Math.max(0, Math.min(1, p)); /* jump to position */ }
    kill() { this._animation?.cancel(); this._playing = false; this._completed = true; }

    get playing() { return this._playing && !this._paused; }
    get completed() { return this._completed; }
    get iteration() { return this._iteration; }

    _runAnimation() {
        // Use WAAPI when possible, fall back to JS animation loop for spring/bounce/elastic
        const { properties, from, to, over, ease } = this.config;
        const canUseWAAPI = this._isWAAPICompatible(ease);

        if (canUseWAAPI) {
            this._runWAAPI(from, to, over, ease);
        } else {
            this._runJSAnimation(from, to, over, ease);
        }
    }

    _isWAAPICompatible(ease) {
        const waapiEases = ['linear', 'easeIn', 'easeOut', 'easeInOut',
            'power1In', 'power1Out', 'power1InOut',
            'power2In', 'power2Out', 'power2InOut',
            'power3In', 'power3Out', 'power3InOut',
            'power4In', 'power4Out', 'power4InOut'];
        if (typeof ease === 'string') return waapiEases.includes(ease);
        return ease?.type === 'cubicBezier' || ease?.type === 'steps';
    }

    _runWAAPI(from, to, over, ease) {
        const keyframes = [this._toCSSProperties(from), this._toCSSProperties(to)];
        const options = {
            duration: over,
            easing: this._toCSSEasing(ease),
            fill: 'forwards',
        };
        this._animation = this.el.animate(keyframes, options);
        this._animation.onfinish = () => {
            this._completed = true;
            this._iteration++;
            this._callbacks.onComplete?.();
        };
    }

    _runJSAnimation(from, to, over, ease) {
        const startTime = performance.now();
        const easeFn = getEasingFn(ease);
        const animate = (now) => {
            const elapsed = now - startTime;
            const t = Math.min(elapsed / over, 1);
            this.progress = easeFn(t);
            this._applyProperties(this.progress, from, to);
            this._callbacks.onUpdate?.();
            if (t < 1) {
                this._rafId = requestAnimationFrame(animate);
            } else {
                this._completed = true;
                this._iteration++;
                this._callbacks.onComplete?.();
            }
        };
        this._rafId = requestAnimationFrame(animate);
    }

    _toCSSProperties(props) {
        const css = {};
        for (const [k, v] of Object.entries(props)) {
            css[PROPERTY_MAP[k] || k] = v;
        }
        // Compose transform properties
        const transforms = [];
        if (props.x != null) transforms.push(`translateX(${props.x})`);
        if (props.y != null) transforms.push(`translateY(${props.y})`);
        if (props.scale != null) transforms.push(`scale(${props.scale})`);
        if (props.rotate != null) transforms.push(`rotate(${props.rotate})`);
        if (transforms.length) css.transform = transforms.join(' ');
        return css;
    }

    _applyProperties(t, from, to) { /* interpolate and apply CSS properties */ }
    _toCSSEasing(ease) { /* convert OmniView easing to CSS easing string */ }
}
```

- [ ] **Step 4: Write OmniView runtime integration**

```javascript
// Global animation registry
window.__omni_animations = new Map();

// Create animation from motion IR config
function __omni_createAnimation(nid, motionConfig) {
    const el = __omni_el(nid);
    if (!el) return null;
    const player = new AnimationPlayer(el, motionConfig);
    const key = `${nid}_${motionConfig.trigger || 'default'}`;
    __omni_animations.set(key, player);
    return player;
}

// Get animation handle for ref API
function __omni_getAnimation(nid, trigger) {
    return __omni_animations.get(`${nid}_${trigger}`);
}

// Reduced motion detection
const __omni_reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
```

- [ ] **Step 5: Embed runtime into codegen output**

The existing signal runtime (~200 lines) is an inline Rust string literal in `emit_signal_runtime()` (omniview.rs:168-255). The motion runtime follows the same pattern: append `emit_animation_runtime()` to `OmniViewGenerator`, emitting the contents of `omni-motion-runtime.js` as a string literal. Alternatively, use `include_str!("../../build/omni-motion-runtime.js")` — a NEW pattern for this codebase but acceptable for an external JS file.

- [ ] **Step 6: Commit**

```bash
git add build/omni-motion-runtime.js
git commit -m "feat: add Web animation runtime with WAAPI integration and spring physics"
```

---

### Task 9: Web Codegen — React + OmniView Native

**Files:**
- Modify: `omni-compiler/src/codegen/react.rs`
- Modify: `omni-compiler/src/codegen/omniview.rs`
- Modify: `omni-compiler/src/codegen/mod.rs`

**Purpose:** Generate React components with animation hooks AND vanilla HTML/CSS/JS for motion animations. React is the priority web target; OmniView native (WAAPI + embedded runtime) is the fallback/standalone target.

- [ ] **Step 0: Create test helper functions**

The plan assumes `compile_to_react()`, `compile_to_js()`, `compile_to_dart()` exist — they don't. Create them first in `codegen_tests.rs`:

```rust
fn compile_to_react(source: &str) -> String {
    let tokens = lexer::tokenize(source);
    let program = parser::parse(&tokens, source).expect("parse");
    let semantics = semantic::analyze(&program);
    let ir_program = ir::lower(&program, &semantics);
    let output = ReactCodegen::generate(&ir_program, &HashMap::new());
    output.files.iter().find(|f| f.path.ends_with(".tsx")).map(|f| f.content.clone()).unwrap_or_default()
}
fn compile_to_js(source: &str) -> String { compile_omniview(source).js }
fn compile_to_dart(source: &str) -> String {
    let tokens = lexer::tokenize(source);
    let program = parser::parse(&tokens, source).expect("parse");
    let semantics = semantic::analyze(&program);
    let ir_program = ir::lower(&program, &semantics);
    let output = FlutterCodegen::generate(&ir_program, &HashMap::new());
    output.files.iter().find(|f| f.path.ends_with(".dart") && !f.path.contains("_store")).map(|f| f.content.clone()).unwrap_or_default()
}
```

- [ ] **Step 1: Write React codegen test**

In `omni-compiler/tests/codegen_tests.rs`:

```rust
#[test]
fn test_react_codegen_motion_fade_in() {
    let source = r#"
        @motion fadeIn {
            to { opacity: 1; y: 0 } over: 300ms ease: easeOut
        }
        component Demo {
            view {
                <div @enter={fadeIn}>Hello</div>
            }
        }
    "#;
    let output = compile_to_react(source);
    assert!(output.contains("useEffect"));       // enter animation on mount
    assert!(output.contains("opacity"));
    assert!(output.contains("duration"));
}

#[test]
fn test_react_codegen_motion_hover() {
    let source = r#"
        @motion scaleUp { to { scale: 1.05 } over: 150ms ease: easeOut }
        component Btn {
            view { <button @hover={scaleUp}>Click</button> }
        }
    "#;
    let output = compile_to_react(source);
    assert!(output.contains("onMouseEnter"));
    assert!(output.contains("onMouseLeave"));
    assert!(output.contains("useState"));        // track hover state
}

#[test]
fn test_react_codegen_motion_stagger() {
    let source = r#"
        @motion itemIn { to { opacity: 1 } over: 300ms }
        component List {
            view {
                <ul @enter={stagger(itemIn, each:60ms, from:center)}>
                    @for item in items key={item.id} {
                        <li @enter={itemIn} />
                    }
                </ul>
            }
        }
    "#;
    let output = compile_to_react(source);
    assert!(output.contains("staggerDelay"));
    assert!(output.contains("transition"));
}
```

- [ ] **Step 2: Write OmniView native codegen test**

In `omni-compiler/tests/codegen_tests.rs`:

```rust
#[test]
fn test_codegen_motion_fade_in() {
    let source = r#"
        @motion fadeIn {
            to { opacity: 1; y: 0 } over: 300ms ease: easeOut
        }
        component Demo {
            view {
                <div @enter={fadeIn}>Hello</div>
            }
        }
    "#;
    let output = compile_to_js(source);
    assert!(output.contains("__omni_createAnimation"));
    assert!(output.contains("opacity"));
    assert!(output.contains("300"));
}

#[test]
fn test_codegen_motion_stagger() {
    let source = r#"
        @motion itemIn {
            to { opacity: 1 } over: 300ms
        }
        component List {
            view {
                <ul @enter={stagger(itemIn, each:60ms, from:center)}>
                    @for item in items key={item.id} {
                        <li @enter={itemIn} />
                    }
                </ul>
            }
        }
    "#;
    let output = compile_to_js(source);
    assert!(output.contains("stagger"));
    assert!(output.contains("60"));
}

#[test]
fn test_codegen_motion_spring() {
    let source = r#"
        @motion bounce {
            to { scale: 1 } over: 500ms ease: spring(tension:200, friction:15)
        }
    "#;
    let output = compile_to_js(source);
    assert!(output.contains("spring"));
    assert!(output.contains("200"));
}
```

- [ ] **Step 3: Run tests to verify they fail**

```bash
cd omni-compiler && cargo test test_react_codegen test_codegen_motion -- --nocapture 2>&1 | tail -10
```
Expected: tests fail (codegen not yet implemented).

- [ ] **Step 4: Implement React motion codegen**

**Prerequisite:** Add `useRef` to the existing React import list (react.rs line 54):
```rust
// Change: import React, { useState, useEffect, useMemo } from 'react';
// To:     import React, { useState, useEffect, useMemo, useRef } from 'react';
```

In `react.rs`, add motion animation generation. React target uses a lightweight approach: emit `useState` + `useEffect` + `useRef` hooks with WAAPI calls, no external animation library dependency.

```rust
fn emit_react_motion_trigger(&self, el: &ElementIR) -> String {
    for trigger in &el.motion_triggers {
        match trigger.kind {
            TriggerKind::Enter => {
                // useEffect + WAAPI on mount
                format!(r#"
                const {}Ref = useRef(null);
                useEffect(() => {{
                    const el = {}Ref.current;
                    if (!el) return;
                    const anim = el.animate([{}], [{}], {{
                        duration: {},
                        easing: '{}',
                        fill: 'forwards'
                    }});
                    return () => anim.cancel();
                }}, []);
                "#, /* ... */)
            },
            TriggerKind::Hover => {
                // useState for hover + onMouseEnter/onMouseLeave
                format!(r#"
                const [{}Hovered, set{}Hovered] = useState(false);
                // onMouseEnter={{() => setHovered(true)}}
                // onMouseLeave={{() => setHovered(false)}}
                "#, /* ... */)
            },
            // ... other triggers
        }
    }
}

fn emit_react_stagger(&self, stagger: &StaggerCall, items: &[ElementIR]) -> String {
    // Generate per-item delay based on stagger config
    // Each child gets style={{ transitionDelay: `${index * each}ms` }}
    format!("style={{ transitionDelay: `${{index * {each}}}ms` }}", each = stagger.timing.each_ms)
}
```

- [ ] **Step 5: Implement OmniView native motion codegen**

In `omniview.rs`, add (lower priority, used for standalone HTML output):

```rust
/// Generate JS for a motion definition
fn emit_motion_def(&mut self, motion: &MotionIntent) -> String {
    let mut out = String::new();
    match &motion.body {
        MotionBodyIR::Keyframe(kf) => {
            // Generate WAAPI keyframe animation or JS animation
            out.push_str(&self.emit_keyframe_js(&motion.name, kf));
        },
        MotionBodyIR::Timeline(tl) => {
            out.push_str(&self.emit_timeline_js(&motion.name, tl));
        },
        MotionBodyIR::CallbackOnly(cb) => {
            out.push_str(&self.emit_callback_js(&motion.name, cb));
        },
    }
    out
}

fn emit_motion_config_js(&self, config: &MotionConfig) -> String {
    // Convert motion IR to JS config object for AnimationPlayer
    let easing_js = match &config.ease {
        Some(e) => self.emit_easing_js(e),
        None => "\"linear\"".to_string(),
    };
    let from_str = self.emit_property_map_js(&config.from);
    let to_str = self.emit_property_map_js(&config.to);
    let ease_str = match &config.ease {
        Some(e) => self.emit_easing_js(e),
        None => "\"linear\"".to_string(),
    };
    let paused_str = if config.playback.paused { "true" } else { "false" };
    format!(r#"{{
        properties: [{prop_list}],
        from: {{{from_str}}},
        to: {{{to_str}}},
        over: {over_ms},
        ease: {ease_str},
        paused: {paused_str},
        callbacks: {{}}
    }}"#)
}

fn emit_element_trigger_js(&mut self, el: &ElementIR) -> String {
    let mut out = String::new();
    for trigger in &el.motion_triggers {
        let motion_config = self.resolve_motion_config(trigger);
        let trigger_name = self.trigger_to_js(&trigger.kind);
        out.push_str(&format!(
            "__omni_createAnimation('{}', {{trigger: '{}', ...{}}});\n",
            el.nid, trigger_name, self.emit_motion_config_js(&motion_config)
        ));
    }
    // Generate enter/leave hook calls for @if integration
    // Generate hover event bindings with reverse behavior
    out
}
```

- [ ] **Step 6: Implement @timeline codegen**

Generate JS that orchestrates multiple elements by `id`:

```rust
fn emit_timeline_js(&self, timeline: &TimelineIntent) -> String {
    let mut out = String::new();
    for item in &timeline.items {
        // item.id → document.getElementById(item.id)
        // item.motion_name → resolve to motion config
        let start_delay = self.compute_start_delay(item);
        out.push_str(&format!(
            "setTimeout(() => {{ 
                const el = __omni_el('{nid}');  // use __omni_el() convention, not getElementById
                if (el) __omni_createAnimation('{nid}', {{trigger:'timeline', ...{config}}});
            }}, {delay});\n",
            id = item.element_id,
            config = self.emit_motion_config_js(&item.motion_config),
            delay = start_delay
        ));
    }
    out
}
// Flutter: generate GlobalKey lookups for each id
// final _headerKey = GlobalKey(); → Header(key: _headerKey)
```

On Flutter, compile `id="header"` → `GlobalKey` and use it for the widget lookup.

- [ ] **Step 7: Implement @if double-branch codegen**

Generate JS that:
- Renders both branches with display:none wrappers
- On condition change: plays @leave on old branch, sets display:block + plays @enter on new branch
- For wait mode: awaits @leave completion before showing new branch
- Removes old branch from DOM after @leave completes

- [ ] **Step 8: Implement @for key + animation codegen**

- Add key-based identity tracking to list rendering
- Generate `data-for-key` attributes
- Implement `_leavingItems` map in runtime
- Stagger indices based on item positions

- [ ] **Step 9: Implement @hero (same page)**

Generate FLIP animation for same-page hero transitions:

```javascript
// Runtime: record element position before DOM change
function __omni_hero_before(tag) {
    const el = document.querySelector(`[data-hero="${tag}"]`);
    if (!el) return null;
    const rect = el.getBoundingClientRect();
    return { el, rect, tag };
}

// Runtime: after DOM change, compute FLIP and animate
function __omni_hero_after(before, tag) {
    const after = document.querySelector(`[data-hero="${tag}"]`);
    if (!before || !after) return;
    const newRect = after.getBoundingClientRect();
    const deltaX = before.rect.left - newRect.left;
    const deltaY = before.rect.top - newRect.top;
    after.animate([
        { transform: `translate(${deltaX}px, ${deltaY}px) scale(${before.rect.width / newRect.width})` },
        { transform: 'translate(0, 0) scale(1)' }
    ], { duration: 400, easing: 'ease-out', fill: 'forwards' });
}
```

Flutter: compile @hero tag directly to Flutter's `Hero(tag: "...", child: ...)` widget.

- [ ] **Step 10: Embed runtime JS in output**

```rust
fn emit_animation_runtime(&self) -> String {
    // Embed the contents of build/omni-motion-runtime.js inline
    include_str!("../../build/omni-motion-runtime.js").to_string()
}
```

- [ ] **Step 11: Run codegen tests**

```bash
cd omni-compiler && cargo test test_react_codegen test_codegen_motion -- --nocapture 2>&1 | tail -10
```
Expected: all React + OmniView codegen tests pass.

- [ ] **Step 12: Commit**

```bash
git add omni-compiler/src/codegen/react.rs omni-compiler/src/codegen/omniview.rs omni-compiler/tests/codegen_tests.rs
git commit -m "feat: add React + OmniView codegen for motion animations"
```

---

### Task 10: Flutter Codegen

**Files:**
- Modify: `omni-compiler/src/codegen/flutter.rs`

**Purpose:** Generate Dart code using AnimationController and Tween.

- [ ] **Step 1: Write Flutter codegen test**

```rust
#[test]
fn test_flutter_codegen_motion_fade() {
    // Flutter codegen requires a component — bare @motion won't produce output
    let source = r#"
        component Demo {
            @motion fadeIn { to { opacity: 1 } over: 300ms ease: easeOut }
            view { <div @enter={fadeIn}>Hello</div> }
        }
    "#;
    let output = compile_to_dart(source);
    assert!(output.contains("AnimationController"));
    assert!(output.contains("Tween"));
    assert!(output.contains("Duration(milliseconds: 300)"));
}

#[test]
fn test_flutter_codegen_spring() {
    let source = r#"
        component Demo {
            @motion bounce { to { scale: 1 } over: 500ms ease: spring(tension:200, friction:15) }
            view { <div @enter={bounce}>Hello</div> }
        }
    "#;
    let output = compile_to_dart(source);
    assert!(output.contains("SpringSimulation"));
    assert!(output.contains("stiffness: 200"));
    assert!(output.contains("damping: 15"));
    assert!(output.contains("package:flutter/physics.dart"));  // spring needs physics import
}
```

- [ ] **Step 2: Run tests to verify fail**

```bash
cd omni-compiler && cargo test test_flutter_codegen_motion -- --nocapture 2>&1 | tail -10
```

- [ ] **Step 3: Implement Flutter codegen**

In `flutter.rs`:

```rust
fn emit_motion_controller_dart(&self, motion: &MotionIntent) -> String {
    let body = match &motion.body {
        MotionBodyIR::Keyframe(kf) => self.emit_keyframe_dart(kf),
        MotionBodyIR::Timeline(tl) => self.emit_timeline_dart(tl),
        MotionBodyIR::CallbackOnly(cb) => self.emit_callback_dart(cb),
    };
    format!(r#"
    late final AnimationController _{}Controller;
    late final Animation<{}> _{}Animation;
    "#, motion.name, self.dart_animation_type(&body), motion.name)
}

fn emit_flutter_element_trigger(&self, trigger: &TriggerIntent) -> String {
    match trigger.kind {
        TriggerKind::Enter => {
            // In initState: _fadeInController = AnimationController(
            //   duration: Duration(milliseconds: 300), vsync: this);
            // _fadeInController.forward();
            format!(r#"
            _{trigger}_controller = AnimationController(
              duration: Duration(milliseconds: {dur}), vsync: this);
            _{trigger}_animation = Tween(begin: {from}, end: {to})
              .chain(CurveTween(curve: {curve}))
              .animate(_{trigger}_controller);
            _{trigger}_controller.forward();
            "#)
        },
        TriggerKind::Hover => {
            // MouseRegion + onEnter/onExit
            format!(r#"
            MouseRegion(
              onEnter: (_) => _{trigger}_controller.forward(),
              onExit: (_) => _{trigger}_controller.reverse(),
              child: ...
            )
            "#)
        },
        // ... other triggers
    }
}

fn emit_flutter_spring_dart(&self, spring: &SpringConfig) -> String {
    // NOTE: When spring easing is used, add to file header:
    // import 'package:flutter/physics.dart';
    format!(
        "SpringSimulation(SpringDescription(mass: 1, stiffness: {}, damping: {}), 0, 1, {})",
        spring.tension, spring.friction,
        spring.velocity.unwrap_or(0.0)
    )
}
```

- [ ] **Step 4: Run tests to verify pass**

```bash
cd omni-compiler && cargo test test_flutter_codegen_motion -- --nocapture 2>&1 | tail -10
```

- [ ] **Step 5: Commit**

```bash
git add omni-compiler/src/codegen/flutter.rs omni-compiler/tests/codegen_tests.rs
git commit -m "feat: add Flutter codegen for motion animations (AnimationController + Tween)"
```

---

### Task 11: Integration — E2E Tests

**Files:**
- Create: `test/projects/learn-omniview/src/pages/MotionDemo.omni` 
- Modify: `test/projects/learn-omniview/src/main.omni`

**Purpose:** End-to-end verification of the complete animation pipeline.

- [ ] **Step 1: Create a comprehensive test .omni file**

```omni
@motion fadeInUp {
  from { opacity: 0; y: 20px }
  to   { opacity: 1; y: 0 }
  over: 400ms ease: easeOut
}

@motion scalePress {
  to { scale: 0.95 } over: 100ms ease: easeOut
}

@motion listItem {
  from { opacity: 0; x: -20px }
  to   { opacity: 1; x: 0 }
  over: 300ms ease: power2Out
}

@motion pulse {
  to { scale: 1.05 } over: 800ms
  direction: alternate loop: infinite ease: easeInOut
}

component MotionDemo {
  signal items: List = [{id: "a", name: "Item A"}, {id: "b", name: "Item B"}, {id: "c", name: "Item C"}]
  signal isOpen: bool = false
  
  action addItem {
    items = [...items, {id: "d", name: "Item D"}]
  }

  action removeItem {
    items = items.filter(i => i.id != "b")
  }

  view {
    <div>
      <h1 @enter={fadeInUp}>Motion Demo</h1>

      <button @press={scalePress} @hover={pulse} @click={isOpen = !isOpen}>
        Toggle Modal
      </button>

      @if isOpen mode="sync" {
        <div @enter={fadeInUp} @leave={fadeInUp}>
          <h2>Modal Content</h2>
        </div>
      }

      <button @press={scalePress} @click={addItem}>Add Item</button>
      <button @press={scalePress} @click={removeItem}>Remove Item</button>

      <ul @enter={
        to { opacity: 1 } over: 300ms
        when: beforeChildren
        staggerChildren: 60ms
        staggerFrom: start
      }>
        @for item in items key={item.id} {
          <li @enter={listItem} @leave={fadeInUp}>{item.name}</li>
        }
      </ul>

      <div states: {
        idle:    to { scale: 1; opacity: 1 }   over: 150ms,
        hover:   to { scale: 1.05; opacity: 1 } over: 150ms,
        pressed: to { scale: 0.95; opacity: 0.8 } over: 100ms
      }>
        Interactive Panel
      </div>

      <div states: {
        loading: { opacity: 0.7 } over: 200ms,
        success: { opacity: 1; backgroundColor: #00ff00 } over: 300ms,
        error:   { opacity: 1; backgroundColor: #ff0000 } over: 300ms
      } on: {status}>
        Status-driven element
      </div>
    </div>
  }
}
```

- [ ] **Step 2: Compile the test project**

```bash
cd omni-compiler && cargo run -- compile test/projects/learn-omniview/src/pages/MotionDemo.omni -t web -o test/projects/learn-omniview/build/motion-demo/
```
Expected: compilation succeeds.

- [ ] **Step 3: Verify generated output**

Check the generated JS contains:
- `__omni_createAnimation` calls for each @enter/@leave/@hover
- `AnimationPlayer` instances with correct config
- Spring physics for spring easing
- WAAPI keyframes for standard easing
- Stagger delay calculation
- Double-branch @if handling
- Key-based list identity tracking

- [ ] **Step 4: Run the full test suite**

```bash
cd omni-compiler && cargo test 2>&1 | tail -10
```
Expected: all ~410 existing tests + new motion tests pass.

- [ ] **Step 5: Commit**

```bash
git add test/projects/learn-omniview/ omni-compiler/tests/
git commit -m "test: add E2E motion animation test project"
```

---

## Phase 1 Completion Checklist

After all 11 tasks, verify:
- [ ] `cargo test` — all tests pass (target: ~450 tests, up from ~410)
- [ ] `cargo build --release` — production build succeeds
- [ ] Manual E2E: compile MotionDemo.omni → open in browser → animations play correctly
- [ ] Cross-platform: compile same .omni to Flutter → matches Web behavior
- [ ] All 28 compile-time errors produce correct error messages
- [ ] All 9 warnings fire when expected
- [ ] Spec §20 validation table fully covered

---

## Phase 2: Advanced Features

### Task 12: @in-view + @scroll Triggers

**Files:**
- Modify: `omni-compiler/src/codegen/omniview.rs`
- Modify: `omni-compiler/src/codegen/react.rs`
- Modify: `omni-compiler/src/codegen/flutter.rs`

**Purpose:** Implement scroll-driven and viewport-triggered animations, @if mode= parser fix, component-level @if-mode, and @hero foundation.

**⚠️ FOUNDATION WORK FIRST:** @scroll-* sub-attributes were deferred in Phase 1 and need parser → AST → IR → lowering BEFORE codegen.

- [ ] **Step 0a: Add @scroll-* and @if mode= parsing**

In `view.rs`, add parsing for scroll sub-attributes and fix the `@if mode=` parser:

```rust
// In parse_element() prop loop, after existing @-prop handling:
TokenKind::AtScrollStart => {
    self.advance();
    self.expect_token(TokenKind::Eq, "expected '='")?;
    let val = self.parse_string_literal()?;
    props.push(ElementProp::ScrollStart(val));
}
TokenKind::AtScrollEnd => { /* same pattern → ElementProp::ScrollEnd */ }
TokenKind::AtScrollScrub => {
    self.advance();
    self.expect_token(TokenKind::Eq, "expected '='")?;
    self.expect_token(TokenKind::LBrace, "expected '{'")?;
    let val = self.parse_expr()?;
    self.expect_token(TokenKind::RBrace, "expected '}'")?;
    props.push(ElementProp::ScrollScrub(val));
}
```

**Fix `parse_if_directive()` in view.rs:** After parsing the condition expression, check for `mode=` before `{`:
```rust
let transition_mode = if self.peek_kind() == TokenKind::Ident(ref s) && s == "mode" {
    self.advance();
    self.expect_token(TokenKind::Eq, "expected '='")?;
    Some(self.parse_string_literal()?)
} else { None };
// Pass transition_mode to IfDirective { transition_mode, ... }
```
This unblocks the EXISTING wait-mode codegen in omniview.rs and flutter.rs that's currently dead code (transition_mode is always None).

- [ ] **Step 0b: Add component-level @if-mode parsing**

In `parse_component_def()` (top_level.rs), parse `@if-mode:` as a colon-prop on components:
```rust
if self.peek_kind() == TokenKind::IfMode {
    self.advance();
    self.expect_token(TokenKind::Colon, "expected ':'")?;
    let mode = self.parse_string_literal()?;
    comp.if_mode_default = Some(mode);
}
```
Add `if_mode_default: Option<String>` to `ComponentDef` and `ComponentIR`. Lowering propagates it. Codegen: when per-`@if` mode is absent, use component default.

- [ ] **Step 0c: Add @hero foundation (tokens, AST, parser, IR)**

@hero was spec'd as Phase 1 but never built. Add now:

Token: `AtHero` (maps from `"@hero"` in `read_at_keyword()`).

AST:
```rust
pub struct HeroDef { pub tag: String, pub over: Expr, pub ease: Option<EasingExpr> }
// ElementProp variants:
ElementProp::HeroTag(String),           // @hero="tag-name"
ElementProp::HeroTransition(String),    // @hero-transition={motionName}
```

Parser: `@hero` block in top_level.rs (like `@motion` but simpler — just over + ease). `@hero` element attribute in view.rs as string value.

IR: `HeroIntent { tag: String, over: ExprIR, ease: Option<EasingIR> }`. `ElementIR.hero_tag: Option<String>`.

Lowering: direct mapping. Codegen (reuse Task 9 Step 9 `__omni_hero_*` runtime functions).

- [ ] **Step 1: Write @in-view tests**

```rust
#[test]
fn test_in_view_once() {
    let source = r#"
        @motion fadeIn { to { opacity: 1 } over: 300ms }
        component Demo { view { <div @in-view={fadeIn} @in-view-once={true}>Hello</div> } }
    "#;
    let output = compile_to_js(source);
    assert!(output.contains("IntersectionObserver"));
    assert!(output.contains("once"));
}
```

- [ ] **Step 2: Implement @in-view — Web**

OmniView: Generate `IntersectionObserver` wrapping `__omni_createAnimation()` on first intersection.

React: `useRef` + `useEffect` + `IntersectionObserver`, play animation on mount-triggered callback.

- [ ] **Step 3: Implement @in-view — Flutter**

Use `VisibilityDetector` widget (from `visibility_detector` package) or `RenderObject` visibility check in `initState`.

- [ ] **Step 4: Write @scroll tests**

```rust
#[test]
fn test_scroll_scrub() {
    let source = r#"
        @motion parallax { to { y: -50% } over: 1s }
        component Demo { view { <div @scroll={parallax} @scroll-scrub={0.5} @scroll-start="top 80%" @scroll-end="bottom 20%" /> } }
    "#;
    let output = compile_to_js(source);
    assert!(output.contains("scroll"));
    assert!(output.contains("scrub"));
    assert!(output.contains("progress"));
}
```

- [ ] **Step 5: Implement @scroll — Web**

JavaScript scroll listener mapping `(scrollPos - startScroll) / (endScroll - startScroll)` → animation progress via `animation.progress = clamp(value, 0, 1)`. Scrub: apply smoothing with exponential moving average over `scrub` seconds.

- [ ] **Step 6: Implement @scroll — Flutter**

`ScrollController.addListener` → compute progress → drive AnimationController via `_controller.value = progress`.

- [ ] **Step 7: Run tests and commit**

```bash
cd omni-compiler && cargo test test_in_view test_scroll -- --nocapture 2>&1 | tail -10
git add omni-compiler/src/codegen/ omni-compiler/tests/codegen_tests.rs
git commit -m "feat: add @in-view and @scroll triggers for Web and Flutter"
```

---

### Task 13: @drag + Constraints + Momentum

**Files:**
- Modify: `omni-compiler/src/codegen/omniview.rs`
- Modify: `omni-compiler/src/codegen/flutter.rs`

**Purpose:** Gesture-driven drag animations with constraints and inertia.

**⚠️ FOUNDATION WORK FIRST:** All six @drag-* sub-attributes were deferred in Phase 1 and need parser → AST → IR → lowering BEFORE codegen.

- [ ] **Step 0a: Add @drag-* sub-attribute parsing**

In `view.rs`, add parsing for all six drag sub-attributes:

```rust
// @drag-constraints="left:0 right:300" or "parent" or "ref:NAME"
TokenKind::DragConstraints => {
    self.advance(); self.expect_token(TokenKind::Eq, "expected '='")?;
    let val = self.parse_string_literal()?;
    props.push(ElementProp::DragConstraints(val));
}
// @drag-elastic={0.2} — expression, float 0.0-1.0
TokenKind::DragElastic => {
    self.advance(); self.expect_token(TokenKind::Eq, "expected '='")?;
    self.expect_token(TokenKind::LBrace, "expected '{'")?;
    let val = self.parse_expr()?;
    self.expect_token(TokenKind::RBrace, "expected '}'")?;
    props.push(ElementProp::DragElastic(val));
}
// @drag-scrub={true}, @drag-momentum={friction:0.95}, @drag-threshold={30%}, @drag-velocity-max={2000px}
// ... same pattern
```

- [ ] **Step 0b: Add drag AST/IR/lowering**

AST: Add to `ElementProp`: `DragConstraints(String)`, `DragElastic(Expr)`, `DragScrub(Expr)`, `DragMomentum(Expr)`, `DragThreshold(Expr)`, `DragVelocityMax(Expr)`.

IR: Add to `ElementIR`: `drag_constraints: Option<String>`, `drag_elastic: Option<ExprIR>`, `drag_scrub: Option<ExprIR>`, `drag_momentum: Option<ExprIR>`, `drag_threshold: Option<ExprIR>`, `drag_velocity_max: Option<ExprIR>`.

Lowering: Extend `lower_view_node()` ElementProp match with these new variants.

Semantic: Wire up the four drag checks from Task 7 that currently have no AST/IR to validate:
- `check_drag_constraints_axis()` — validate `left/right` only with @drag-horizontal, `top/bottom` only with @drag-vertical
- `check_drag_constraints_no_mixed_parent()` — "parent" can't combine with explicit edges
- `check_drag_constraints_ref_exists()` — `ref:NAME` must reference a sibling element
- `check_drag_elastic_range()` — value must be 0.0..1.0

- [ ] **Step 1: Write @drag tests**

```rust
#[test]
fn test_drag_horizontal_with_constraints() {
    let source = r#"
        @motion swipeCard { to { x: 300px } over: 300ms ease: spring(tension:200, friction:15) }
        component Demo { view { <Card @drag-horizontal={swipeCard} @drag-constraints="left:0 right:300" @drag-elastic={0.2} @drag-threshold={30%} /> } }
    "#;
    let output = compile_to_js(source);
    assert!(output.contains("pointerdown"));
    assert!(output.contains("constraint"));
    assert!(output.contains("elastic"));
}
```

- [ ] **Step 2: Implement @drag — Web**

`pointerdown` → track delta → constrain to `[min, max]` → apply elastic if past bounds → on `pointerup`, if past threshold play `@drag-momentum` or spring back. Convert gesture velocity to `$velocity`.

- [ ] **Step 3: Implement @drag — Flutter**

`GestureDetector.onHorizontalDragUpdate/onVerticalDragUpdate` → clamp delta + elastic. `onDragEnd` → compute velocity → `_controller.animateWith(SpringSimulation(...))`.

- [ ] **Step 4: Run tests and commit**

```bash
cd omni-compiler && cargo test test_drag -- --nocapture 2>&1 | tail -10
git add omni-compiler/src/codegen/ omni-compiler/tests/codegen_tests.rs
git commit -m "feat: add @drag gesture animations with constraints and momentum"
```

---

### Task 14: @driven-by + Path Animation

**Files:**
- Modify: `omni-compiler/src/codegen/omniview.rs`
- Modify: `omni-compiler/src/codegen/flutter.rs`

**Purpose:** Signal-driven animation progress and path-based motion.

**⚠️ FOUNDATION WORK FIRST:** Path animation syntax (`path:`, `pathRotate:`) was never added to the @motion body parser. Add now.

- [ ] **Step 0a: Add path/ pathRotate parsing to @motion blocks**

In the `@motion` body parser (top_level.rs), after parsing keyframe stops or timeline items:
```rust
let path = if self.peek_is_ident("path") {
    self.advance(); self.expect_token(TokenKind::Colon, "expected ':'")?;
    Some(self.parse_string_literal()?)
} else { None };
let path_rotate = if self.peek_is_ident("pathRotate") {
    self.advance(); self.expect_token(TokenKind::Colon, "expected ':'")?;
    Some(self.parse_bool_literal()?)
} else { None };
```

- [ ] **Step 0b: Add path AST/IR/lowering**

AST: `KeyframeBody { path: Option<String>, path_rotate: Option<bool>, ... }`. Same for timeline `DirectionItem`.

IR: `KeyframeIR { path: Option<String>, path_rotate: Option<bool>, ... }`.

Lowering: Direct field copy.

- [ ] **Step 1: Write @driven-by test**

```rust
#[test]
fn test_driven_by_signal() {
    let source = r#"
        component Demo {
            signal progress: float = 0
            @motion fadeIn { to { opacity: 1 } over: 300ms }
            view { <div @animate={fadeIn} @driven-by={progress}>Hello</div> }
        }
    "#;
    let output = compile_to_js(source);
    assert!(output.contains("clamp"));
    assert!(output.contains("driven"));
}
```

- [ ] **Step 2: Implement @driven-by**

Web: `__omni_effect(() => { player.progress = clamp(signal.value, 0, 1); })` — non-playing, value-bound animation.

Flutter: `animationController.value = clamp(signal, 0.0, 1.0)` in a listener.

- [ ] **Step 3: Write path animation test**

```rust
#[test]
fn test_path_animation() {
    let source = r#"
        @motion followPath { to { offset: 1 } over: 2s path: "M 0,0 C 50,-50 100,50 200,0" }
    "#;
    let output = compile_to_js(source);
    assert!(output.contains("offset-path"));
    assert!(output.contains("M 0,0"));
}
```

- [ ] **Step 4: Implement path animation**

Web: CSS `offset-path` + WAAPI animate `offset-distance`. Flutter: `Path` + `PathMetric.getTangentForOffset` + `AnimationController` driving position.

- [ ] **Step 5: Run tests and commit**

```bash
cd omni-compiler && cargo test test_driven test_path -- --nocapture 2>&1 | tail -10
git commit -m "feat: add @driven-by signal binding and path animation"
```

---

### Task 15: Timeline Labels + repeatDelay/repeatRefresh

**Files:**
- Modify: `omni-compiler/src/parser/ast.rs` (if not in Phase 1)
- Modify: `omni-compiler/src/codegen/omniview.rs`
- Modify: `omni-compiler/src/codegen/flutter.rs`

**Purpose:** Named positions in timelines for seeking, and repeat modifiers.

- [ ] **Step 1: Write timeline label test**

```rust
#[test]
fn test_timeline_labels() {
    let source = r#"
        @motion reveal {
            from { opacity: 0 } over: 300ms
            label "faded"
            then to { y: 0 } over: 200ms
            label "done"
        }
    "#;
    let output = compile_to_js(source);
    assert!(output.contains("\"faded\""));
    assert!(output.contains("\"done\""));
    assert!(output.contains("seek"));
}
```

- [ ] **Step 2: Implement label parsing + seeking**

Store label→time mapping in runtime. `animation.seek("labelName")` looks up the label's time from the motion's computed timeline map.

- [ ] **Step 3: Write repeatDelay/repeatRefresh test**

```rust
#[test]
fn test_repeat_delay_refresh() {
    let source = r#"
        @motion spark {
            from { opacity: 1; x: 0 } to { opacity: 0; x: 50px } over: 400ms
            loop: 3 repeatDelay: 200ms repeatRefresh: true
        }
    "#;
    let output = compile_to_js(source);
    assert!(output.contains("repeatDelay"));
    assert!(output.contains("repeatRefresh"));
}
```

- [ ] **Step 4: Implement repeat modifiers**

`repeatDelay`: pause between loop iterations. `repeatRefresh`: re-evaluate `from` values via `getComputedStyle()` at each iteration start.

- [ ] **Step 5: Run tests and commit**

```bash
cargo test test_timeline_label test_repeat -- --nocapture 2>&1 | tail -10
git commit -m "feat: add timeline labels, repeatDelay, repeatRefresh"
```

---

### Task 16: FLIP Layout Animation for @for Reordering

**Files:**
- Modify: `omni-compiler/src/codegen/omniview.rs`
- Modify: `omni-compiler/src/codegen/flutter.rs`

**Purpose:** Smooth position transitions when @for items change order.

- [ ] **Step 1: Write FLIP test**

```rust
#[test]
fn test_flip_list_reorder() {
    let source = r#"
        component List {
            view {
                @for item in items key={item.id} {
                    <div @enter={fadeIn}>item.name</div>
                }
            }
        }
    "#;
    let output = compile_to_js(source);
    assert!(output.contains("getBoundingClientRect"));
    assert!(output.contains("FLIP"));
}
```

- [ ] **Step 2: Implement FLIP — Web**

Before @for re-render: `getBoundingClientRect()` for all items. After re-render: compute delta for surviving items, apply inverse transform, animate to zero via WAAPI.

- [ ] **Step 3: Implement FLIP — Flutter**

Use `AnimatedList` with `GlobalKey` per item. Compute position deltas in `didUpdateWidget` and animate.

- [ ] **Step 4: Run tests and commit**

```bash
cargo test test_flip -- --nocapture 2>&1 | tail -10
git commit -m "feat: add FLIP layout animation for @for list reordering"
```

---

## Phase 3: Polish

### Task 17: Cross-Component @hero

**Files:**
- Modify: `omni-compiler/src/codegen/omniview.rs`
- Modify: `omni-compiler/src/codegen/flutter.rs`

**Purpose:** Hero transitions between separate pages/components.

- [ ] **Step 1: Write cross-component @hero test**

```rust
#[test]
fn test_hero_cross_component() {
    let source = r#"
        component PageA { view { <img src="thumb.jpg" @hero="product-image" /> } }
        component PageB { view { <img src="full.jpg" @hero="product-image" /> } }
    "#;
    let output = compile_to_js(source);
    assert!(output.contains("data-hero"));
    assert!(output.contains("FLIP"));
}
```

- [ ] **Step 2: Implement — Web**

Global `__omni_hero_before(tag)` / `__omni_hero_after(tag)` runtime functions. Before route change: record hero element positions. After route change: compute FLIP delta and animate.

- [ ] **Step 3: Implement — Flutter**

Direct compilation to `Hero(tag: "product-image", child: ...)` widget. Flutter's Hero handles the transition natively.

- [ ] **Step 4: Commit**

```bash
git commit -m "feat: add cross-component @hero shared element transitions"
```

---

### Task 18: Animation DevTools + Performance

**Files:**
- Create: `build/omni-devtools.js`
- Modify: `omni-compiler/src/codegen/omniview.rs`

**Purpose:** Debug overlay showing running animations, plus performance monitoring.

- [ ] **Step 1: Build DevTools panel**

Inject `window.__OMNI_DEVTOOLS` global with methods:
- `listAnimations()` — returns all running AnimationPlayer instances with name, progress, duration
- `pauseAll()` / `resumeAll()` — global pause/resume
- `setSpeed(0.5)` — global `timeScale`
- `highlight(element)` — flash border on animated elements

- [ ] **Step 2: Add frame budget monitoring**

Track `requestAnimationFrame` timing. Warn when >50 concurrent animations exceed 16ms/frame budget. Degrade gracefully by reducing non-viewport animations to lower frame rates.

- [ ] **Step 3: Add `--omni-motion-speed` CSS variable**

```css
:root { --omni-motion-speed: 1; }
```
All durations multiplied by this variable. `0` disables all animations. Read by runtime on animation creation.

- [ ] **Step 4: Consolidate `will-change` management**

Auto-add/remove `will-change: transform, opacity` based on active animation lifecycle. Remove 500ms after animation completes to free GPU memory.

- [ ] **Step 5: Commit**

```bash
git commit -m "feat: add animation DevTools, performance monitoring, timeScale"
```

---

### Task 19: Lottie/Rive Import

**Files:**
- Create: `omni-compiler/src/parser/lottie.rs`
- Modify: `omni-compiler/src/codegen/omniview.rs`

**Purpose:** Import Lottie/Rive animation files and play them via @motion.

**⚠️ NOTE:** This feature is listed as "Deferred" in spec §25. Task 19 is conditional on spec approval. The `from "..."` syntax conflicts with `from { opacity: 0 }` in @motion blocks — resolved below.

- [ ] **Step 1: Add import syntax (conditional on spec update)**

Use `import()` syntax to avoid conflict with keyframe `from`:
```omni
@motion loadingAnim = import("./loading.lottie") {
  loop: infinite
}
```

Parser: `import("...")` after `@motion NAME =` → parse file extension, validate it's `.lottie`, `.json` (Lottie), or `.riv` (Rive). Distinct from `from { props } to { props }` keyframe syntax.

- [ ] **Step 2: Implement — Web**

Emit `<lottie-player>` web component or Lottie JS runtime, bound to motion triggers.

- [ ] **Step 3: Implement — Flutter**

Emit `Lottie.asset("loading.json")` widget using `lottie` Flutter package.

- [ ] **Step 4: Commit**

```bash
git commit -m "feat: add Lottie/Rive animation file import support"
```

---

## Phase 2 & 3 Completion Checklist

- [ ] `cargo test` — all tests pass (target: ~550 tests)
- [ ] `cargo build --release` — production build succeeds
- [ ] `@in-view` + `@scroll` work on Web and Flutter
- [ ] `@drag` gestures with constraints and elastic rebound
- [ ] `@driven-by` drives animation progress from signal
- [ ] Path animation follows SVG path on both platforms
- [ ] Timeline labels enable seek-by-name
- [ ] `repeatDelay` pauses between loops, `repeatRefresh` re-evaluates from values
- [ ] FLIP transitions smooth @for reordering
- [ ] Cross-component @hero works end-to-end
- [ ] DevTools panel shows running animations with pause/resume/speed controls
- [ ] `--omni-motion-speed` CSS variable globally scales durations
- [ ] `will-change` auto-managed per animation lifecycle
- [ ] Lottie/Rive files importable as @motion
