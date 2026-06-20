# OmniView Motion Animation — Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the core OmniView Motion animation system — @motion definitions, element triggers, stagger, @if/@for animation integration, callbacks, and cross-platform codegen to Web (WAAPI + JS runtime) and Flutter (AnimationController + Tween).

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

- [ ] **Step 1: Add motion-specific tokens**

In `TokenKind` enum, add:

```rust
// Motion definition
AtMotion,        // @motion
// Motion body keywords  
MotionTo,        // to
MotionFrom,      // from
MotionSet,       // set (instant assignment)
MotionThen,      // then (sequential connector)
MotionWith,      // with (parallel connector)
// Motion parameters
Over,            // over (duration keyword)
Ease,            // ease (easing keyword)
ColorSpace,      // colorSpace
Delay,           // delay (different from existing "delay"? check)
Start,           // start (position), but only in motion context
// Playback
Loop,            // loop
Direction,       // direction
Fill,            // fill
Paused,          // paused
RepeatDelay,     // repeatDelay
RepeatRefresh,   // repeatRefresh
TimeScale,       // timeScale
Interrupt,       // interrupt
RespectReducedMotion, // respectReducedMotion
// Stagger
Stagger,         // stagger( function
// Labels
Label,           // label "name"
// Callbacks
OnStart,         // onStart
OnUpdate,        // onUpdate
OnComplete,      // onComplete
OnRepeat,        // onRepeat
OnInterrupt,     // onInterrupt
// Element triggers
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
// Parent-child
When,            // when
DelayChildren,   // delayChildren
StaggerChildren, // staggerChildren
StaggerFrom,     // staggerFrom
// @if transition mode
IfMode,          // @if-mode
// Drag
DragConstraints, // @drag-constraints  
DragElastic,     // @drag-elastic
DragScrub,       // @drag-scrub
DragMomentum,    // @drag-momentum
DragThreshold,   // @drag-threshold
DragVelocityMax, // @drag-velocity-max
```

- [ ] **Step 2: Add keyword mappings in lexer**

In the keyword lookup function, map string identifiers to motion tokens. Motion keywords are context-sensitive — they're only recognized when the parser is inside a `@motion` block or an animation expression context. At the lexer level, these are still produced as identifiers; the parser does the contextual disambiguation.

For `@`-prefixed trigger tokens (`@enter`, `@leave`, etc.), add them to the existing `@`-keyword matching:

```rust
"@motion" => TokenKind::AtMotion,
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
"@if-mode" => TokenKind::IfMode,
"@drag-constraints" => TokenKind::DragConstraints,
"@drag-elastic" => TokenKind::DragElastic,
"@drag-scrub" => TokenKind::DragScrub,
"@drag-momentum" => TokenKind::DragMomentum,
"@drag-threshold" => TokenKind::DragThreshold,
"@drag-velocity-max" => TokenKind::DragVelocityMax,
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
        // Peek at first token to determine mode
        let first = self.peek_kind();
        match first {
            // "from" | "to" → keyframe mode (check for then/with/start later)
            TokenKind::MotionFrom | TokenKind::MotionTo => {
                // Check if any timeline features present
                // (This requires lookahead — simplified here)
                self.parse_keyframe_body()
            },
            // "set" | IDENT | "stagger(" | "label" → timeline mode
            TokenKind::MotionSet | TokenKind::Stagger | TokenKind::Label => {
                self.parse_timeline_body()
            },
            // Callback keywords → callback-only mode
            TokenKind::OnStart | TokenKind::OnUpdate |
            TokenKind::OnComplete | TokenKind::OnRepeat => {
                self.parse_callback_body()
            },
            _ if first == TokenKind::Ident(_) => {
                // Could be a motion reference (timeline) or from/to (keyframe)
                // Look at identifier value — but this is runtime…
                // For now: identifiers in motion body are timeline references
                self.parse_timeline_body()
            },
            _ => Err(self.error("expected motion body")),
        }
    }
    // ... keyframe/timeline/callback body parsers
}
```

- [ ] **Step 3: Register @motion parsing in mod.rs**

In `parser/mod.rs`, hook `parse_motion_def` into the top-level statement parser and component body parser.

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
    // Motion expression: either an IDENT (motion reference)
    // or a motion literal (to { ... } over: 300ms)
    match self.peek_kind() {
        TokenKind::MotionTo | TokenKind::MotionFrom | TokenKind::MotionSet => {
            // Inline tween — parse as motion inline expression
            self.parse_inline_motion()
        },
        TokenKind::Stagger => {
            self.parse_stagger_call()
        },
        _ => {
            // Motion reference (identifier)
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
    CallbackOnly(CallbackIR),
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
pub struct CallbackIR {
    pub callbacks: Vec<CallbackIR>,
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

// In ListRenderingIntent, add:
pub key_expr: Option<ExprIR>,  // populate from parser

// In ComponentIR, add:
pub motions: Vec<MotionIntent>,
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

### Task 6: Semantic — Motion Validation

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
fn test_motion_paused_and_for_without_key() {
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

### Task 7: Web Animation Runtime

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
            const vel = this.solve(from, to, t + 0.001) - pos;
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

- [ ] **Step 5: Install as build artifact**

Ensure the runtime is copied to `build/index.js` and embedded inline in the OmniView codegen output.

- [ ] **Step 6: Commit**

```bash
git add build/omni-motion-runtime.js
git commit -m "feat: add Web animation runtime with WAAPI integration and spring physics"
```

---

### Task 8: Web Codegen — OmniView

**Files:**
- Modify: `omni-compiler/src/codegen/omniview.rs`
- Modify: `omni-compiler/src/codegen/mod.rs`

**Purpose:** Generate HTML/CSS/JS for motion animations targeting the Web.

- [ ] **Step 1: Write codegen test**

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

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd omni-compiler && cargo test test_codegen_motion -- --nocapture 2>&1 | tail -10
```
Expected: tests fail (codegen not yet implemented).

- [ ] **Step 3: Implement motion codegen for OmniView target**

In `omniview.rs`, add:

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
    format!(r#"{{
        properties: [{}],
        from: {{{}}},
        to: {{{}}},
        over: {},
        ease: {},
        paused: {},
        callbacks: {{}}
    }}"#, /* ... */)
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

- [ ] **Step 4: Implement @if double-branch codegen**

Generate JS that:
- Renders both branches with display:none wrappers
- On condition change: plays @leave on old branch, sets display:block + plays @enter on new branch
- For wait mode: awaits @leave completion before showing new branch
- Removes old branch from DOM after @leave completes

- [ ] **Step 5: Implement @for key + animation codegen**

- Add key-based identity tracking to list rendering
- Generate `data-for-key` attributes
- Implement `_leavingItems` map in runtime
- Stagger indices based on item positions

- [ ] **Step 6: Implement @hero (same page)**

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

- [ ] **Step 7: Embed runtime JS in output**

```rust
fn emit_animation_runtime(&self) -> String {
    // Embed the contents of build/omni-motion-runtime.js inline
    include_str!("../../build/omni-motion-runtime.js").to_string()
}
```

- [ ] **Step 8: Run codegen tests**

```bash
cd omni-compiler && cargo test test_codegen_motion -- --nocapture 2>&1 | tail -10
```
Expected: tests pass.

- [ ] **Step 9: Commit**

```bash
git add omni-compiler/src/codegen/ omni-compiler/tests/codegen_tests.rs
git commit -m "feat: add Web codegen for motion animations (WAAPI + JS runtime)"
```

---

### Task 9: Flutter Codegen

**Files:**
- Modify: `omni-compiler/src/codegen/flutter.rs`

**Purpose:** Generate Dart code using AnimationController and Tween.

- [ ] **Step 1: Write Flutter codegen test**

```rust
#[test]
fn test_flutter_codegen_motion_fade() {
    let source = r#"
        @motion fadeIn { to { opacity: 1 } over: 300ms ease: easeOut }
    "#;
    let output = compile_to_dart(source);
    assert!(output.contains("AnimationController"));
    assert!(output.contains("Tween"));
    assert!(output.contains("Duration(milliseconds: 300)"));
}

#[test]
fn test_flutter_codegen_spring() {
    let source = r#"
        @motion bounce { to { scale: 1 } over: 500ms ease: spring(tension:200, friction:15) }
    "#;
    let output = compile_to_dart(source);
    assert!(output.contains("SpringSimulation"));
    assert!(output.contains("stiffness: 200"));
    assert!(output.contains("damping: 15"));
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

### Task 10: Integration — E2E Tests

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

After all 10 tasks, verify:
- [ ] `cargo test` — all tests pass (target: ~450 tests, up from ~410)
- [ ] `cargo build --release` — production build succeeds
- [ ] Manual E2E: compile MotionDemo.omni → open in browser → animations play correctly
- [ ] Cross-platform: compile same .omni to Flutter → matches Web behavior
- [ ] All 28 compile-time errors produce correct error messages
- [ ] All 9 warnings fire when expected
- [ ] Spec §20 validation table fully covered

## Phase 2 & 3 Overview

### Phase 2: Advanced (next cycle)
- `@in-view` + `@scroll` triggers
- `@drag-horizontal/vertical` + momentum + constraints
- `@driven-by` signal binding
- Path animation
- Timeline labels
- `repeatDelay` / `repeatRefresh`
- FLIP layout animation for `@for` reordering
- `states:` multi-state mapping (interaction + signal-driven already in Phase 1)

### Phase 3: Polish (final cycle)
- Cross-component / cross-page `@hero`
- Animation DevTools integration
- Performance warnings (frame budget monitoring)
- `will-change` auto-management consolidated
- `--omni-motion-speed` CSS variable
- Lottie/Rive animation import
