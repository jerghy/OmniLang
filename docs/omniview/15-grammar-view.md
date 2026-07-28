# OmniLang Grammar — OmniView UI Framework

> **Target audience**: AI agents implementing parsers, tool builders, language server authors.
> **Source**: Extracted from `crates/omnilang-core/src/parser/view/`, `top_level/`, `design.rs`.
> **Prerequisite**: Read `14-grammar-lang.md` first — this document extends the core language grammar.

---

## 1. Component Definition

### 1.1 Top-Level BNF

```
component_def ::= 'component' <ident> '{' component_body '}'

component_body ::= (component_member)*

component_member ::= prop_decl
                   | signal_decl
                   | computed_decl
                   | event_decl
                   | ref_decl
                   | store_ref
                   | store_inline
                   | if_mode_decl
                   | motion_def
                   | timeline_def
                   | hero_def
                   | lifecycle_hook
                   | function_def          // 'fn' — from grammar-lang
                   | action_def
                   | getter_decl
                   | view_block
                   | style_block
```

### 1.2 Prop Declaration

```
prop_decl ::= 'prop' <ident> (':' type_annotation)? ('=' expr)? ';'?
```

Component properties — inputs passed from parent:
```
prop title: String
prop count: i32 = 0
prop items: Array<String>
prop onClick: fn(Event) -> void
```

### 1.3 Signal Declaration

```
signal_decl ::= 'signal' <ident> (':' type_annotation)? ('=' expr)? ';'?
```

Reactive state values — trigger re-render on change:
```
signal counter: i32 = 0
signal isOpen = false
signal name: String
```

### 1.4 Computed Declaration

```
computed_decl ::= 'computed' <ident> ':' type_annotation '=' expr ';'?
```

Derived reactive values — recomputed when dependencies change:
```
computed fullName: String = firstName + " " + lastName
computed total: i32 = price * quantity
```

### 1.5 Event Declaration

```
event_decl ::= 'event' <ident> ('(' (param (',' param)*)? ')')? ';'?
```

Component events — emitted to parent components:
```
event onSubmit(value: String)
event onClose
```

### 1.6 Ref Declaration

```
ref_decl ::= 'ref' <ident> ';'?
```

DOM element reference:
```
ref inputEl
ref container
```

### 1.7 Store Reference

```
store_ref     ::= '@store' <ident> ';'?
store_inline  ::= 'store' <ident> '{' (signal_decl | action_def | getter_decl | function_def)* '}'
```

Two forms:
- **Reference**: `@store AppStore;` — links to a separately defined store
- **Inline**: `store LocalState { signal count = 0; action reset() { count = 0 } }` — defines store inline

### 1.8 If-Mode Declaration

```
if_mode_decl ::= 'if_mode' <ident> ':' <string_literal> ';'?
```

Conditional rendering mode for CSS class variants:
```
if_mode default: "light"
if_mode layout: "compact"
```

### 1.9 Action Definition

```
action_def ::= 'action' <ident> '(' (param (',' param)*)? ')' block
```

Store mutation methods:
```
action increment(amount: i32) {
    count += amount
}
action reset() {
    count = 0;
    name = ""
}
```

### 1.10 Getter Declaration

```
getter_decl ::= 'getter' <ident> ':' type_annotation '=' expr ';'?
```

Derived store values:
```
getter doubleCount: i32 = count * 2
```

### 1.11 Lifecycle Hooks

```
lifecycle_hook ::= '@mount' block
                 | '@unmount' block
                 | '@effect' block
```

```
@mount { print "Component mounted" }
@unmount { print "Component will unmount" }
@effect { print "Signal changed:", counter }
```

### 1.12 Hero Definition

```
hero_def ::= '@hero' <ident> block
```

Hero transition element definition (for shared element transitions between pages).

### 1.13 Component AST

```
ComponentDef ::= {
    name: String,
    props: Vec<PropDecl>,
    signals: Vec<SignalDecl>,
    computeds: Vec<ComputedDecl>,
    events: Vec<EventDecl>,
    refs: Vec<RefDecl>,
    store_refs: Vec<String>,
    if_mode_default: Option<String>,
    motions: Vec<MotionDef>,
    timelines: Vec<TimelineDef>,
    hero_defs: Vec<HeroDef>,
    lifecycles: Vec<LifecycleHook>,
    fns: Vec<FunctionDef>,
    actions: Vec<ActionDef>,
    getters: Vec<GetterDecl>,
    view: Option<ViewBlock>,
    styles: Vec<StyleRule>,
}
```

---

## 2. View Templates

### 2.1 View Block

```
view_block ::= 'view' '{' (view_node)* '}'
```

### 2.2 View Nodes

```
view_node ::= html_element
            | component_call
            | if_directive
            | for_directive
            | slot_directive
            | interpolation
            | text_content
            | html_comment
```

### 2.3 HTML Element

```
html_element ::= '<' <ident> (element_prop)* ('>' (view_node)* '</' <ident> '>'
                                 | '/>')

element_prop ::= static_prop
               | event_prop
               | bind_prop
               | motion_trigger
               | driven_by
               | paused_override
               | hover_reverse
               | in_view_modifier
               | scroll_modifier
               | drag_modifier
               | hero_attribute
```

**Void elements** (self-closing): `area`, `base`, `br`, `col`, `embed`, `hr`, `img`, `input`, `link`, `meta`, `param`, `source`, `track`, `wbr`. These auto-close — a `</input>` closing tag is silently consumed if present.

**Closing tag matching**: Required for non-void elements. The parser checks `</tagname>` matches the opening tag.

### 2.4 Static Props

```
static_prop ::= <ident> ('-' <ident>)* ('=' (string_value | expr_value))

string_value ::= <string_literal>
expr_value   ::= '{' expr '}'
```

Supports hyphenated names (`margin-left`, `background-color`) and keyword-as-name (`type`):
```
<input id="email" placeholder="Enter email" />
<div class="container" data-value="42" />
<button disabled={isDisabled}>click</button>
```

**Boolean attribute keywords**: `controls`, `disabled`, `readonly`, `required`, `checked`, `selected`, `autofocus`, `multiple`, `loop`, `muted`, `async`, `defer`, `hidden`, `autocomplete`, `autoplay`.

### 2.5 Event Props

```
event_prop ::= '@' <ident> '=' '{' expr '}'
```

Event handlers bound to DOM events, using `@` prefix:
```
<button @press={handleClick}>click</button>
<input @input={handleInput} @focus={handleFocus} />
```

### 2.6 Bind Props

```
bind_prop ::= 'bind' <ident> '=' '{' expr '}'
```

Two-way data binding:
```
<input bind:value={email} />
```

### 2.7 Component Call

```
component_call ::= <ident> ('(' (prop_name ':' expr (',' prop_name ':' expr)*)? ')')? ('{' (view_node)* '}')?
```

**Disambiguation (PascalCase)**: Only identifiers starting with **uppercase** followed by `(` or `{` are parsed as component calls. Lowercase/camelCase names in view context are treated as raw text.

```
Card(title: "Hello", visible: true) {
    Text { "Card body" }
}
Button(label: "Click me")
Flex { Text { "item 1" } Text { "item 2" } }
```

### 2.8 Text Content

Text in views is collected from consecutive non-special tokens. The parser reconstructs whitespace by checking span byte gaps — if token N+1 starts after token N ends, a space is inserted.

Text tokens include: identifiers (camelCase), punctuation, operators outside of `{expr}` context.

```
<div>Hello world</div>
<Text>Score: {count}</Text>
```

### 2.9 Interpolation

```
interpolation ::= '{' expr '}'          // inside view context
```

```
Text { "Hello, " + name }
<div>Count: {counter}</div>
```

**Disambiguation**: `{expr}` inside views first tries parse-as-expression. If the expression parse succeeds AND the next token is `}`, it's an interpolation. Otherwise it falls back to literal text (collecting tokens until `}`).

### 2.10 HTML Comments

```
html_comment ::= '<!--' (any_char)* '-->'
```

HTML comments are parsed and discarded (produce `ViewNode::Text("")`). Note: `-->` is tokenized as `Minus + Arrow` since the lexer merges `->` into a single `Arrow` token.

### 2.11 View AST Types

```
ViewBlock ::= { children: Vec<ViewNode> }

ViewNode ::=
  | Element(Element)
  | Text(String)
  | Interpolation(Expr)
  | IfDirective(IfDirective)
  | ForDirective(ForDirective)
  | SlotDirective { name: Option<String>, default_content: Option<Vec<ViewNode>> }

Element ::= {
    tag: String,
    props: Vec<ElementProp>,
    children: Vec<ViewNode>,
    self_closing: bool,
}

ElementProp ::=
  | Static { name: String, value: Expr }
  | Dynamic { name: String, expr: Expr }
  | Event { name: String, handler: Expr }
  | Bind { name: String, expr: Expr }
  | MotionTrigger(MotionTrigger)
  | DrivenBy(Expr)
  | PausedOverride(Expr)
  | HoverReverse(String)
  | InViewOnce(Expr) | InViewRepeat(Expr) | InViewThreshold(Expr)
  | ScrollStart(String) | ScrollEnd(String) | ScrollScrub(Expr)
  | DragConstraints(String) | DragElastic(Expr) | DragScrub(Expr)
  | DragMomentum(Expr) | DragThreshold(Expr) | DragVelocityMax(Expr)
  | HeroTag(String) | HeroTransition(Expr)
```

---

## 3. View Directives

### 3.1 @if / @else if / @else

```
if_directive ::= '@if' expr ('mode' '=' <string_literal>)? '{' (view_node)* '}'
                 ('@else' ('@if' | 'if') expr '{' (view_node)* '}' )*
                 ('@else' '{' (view_node)* '}')?
```

Conditional rendering with optional transition mode:
```
@if (isVisible) {
    <div>visible content</div>
} @else if (isLoading) {
    <div>loading...</div>
} @else {
    <div>fallback</div>
}

@if (show) mode= "wait" {
    Panel { Text { "animated" } }
}
```

**Note**: `@else` supports both `@else @if` and `@else if` (without `@` prefix) for the else-if branch.

### 3.2 @for

```
for_directive ::= '@for' <ident> 'in' expr '{' (view_node)* '}'
```

List rendering:
```
@for item in items {
    <li>{item.name}</li>
}

@for card in cards {
    Card(title: card.title) { Text { card.body } }
}
```

**Note**: The `@for` directive uses **speculative parsing** — if the parse fails (e.g., `@for key={expr} Demo` in text content), it falls back to collecting raw text starting with `@for`.

### 3.3 @slot

```
slot_directive ::= '@slot' ('(' <ident> ')')? ('{' (view_node)* '}')?
```

Named slot with optional default content:
```
@slot(header) {
    <div>Default header</div>
}
@slot(body)
```

### 3.4 @empty / @native / @fallback / @error

Reserved directive tokens. Parser infrastructure exists but full implementation is deferred.

```
@empty   // fallback for empty lists
@native  // native platform fallback
@fallback
@error   // error boundary fallback
```

---

## 4. Store System

### 4.1 Store Definition

```
store_def ::= 'store' <ident> '{' (signal_decl | getter_decl | action_def)* '}'
```

Top-level reactive state container:
```
store AppStore {
    signal counter: i32 = 0
    signal name: String = ""

    getter greeting: String = "Hello, " + name

    action increment() {
        counter += 1
    }
    action setName(newName: String) {
        name = newName
    }
}
```

### 4.2 Store Field Access

In expressions, store fields accessed via `$` prefix or store alias:
```
$counter           // direct signal access
$store.counter     // store-qualified access
```

AST type: `Expr::StoreFieldAccess { store_alias: String, field_name: String }`

### 4.3 Store AST Types

```
StoreDef ::= {
    name: String,
    signals: Vec<SignalDecl>,
    getters: Vec<GetterDecl>,
    actions: Vec<ActionDef>,
}

SignalDecl ::= {
    name: String,
    type_ann: Option<TypeAnnotation>,
    init: Option<Expr>,
}

GetterDecl ::= {
    name: String,
    type_ann: TypeAnnotation,
    body: Expr,
}

ActionDef ::= {
    name: String,
    params: Vec<Param>,
    body: Block,
}
```

---

## 5. Style System

### 5.1 Style Block

```
style_block ::= 'style' '{' style_rule+ '}'

style_rule ::= class_selector block
             | element_selector block
             | pseudo_selector block

// Selector forms:
class_selector   ::= '.' <ident>              // .button
element_selector ::= <ident> (!'.' | !'{')     // div (without class prefix)
pseudo_selector  ::= '&' ':' <ident>           // &:hover
                   | '&' '::' <ident>          // &::before
```

Each style rule body is a `{ }` block containing CSS property declarations:
```
component Button {
    prop label: String
    prop variant: String = "primary"

    style {
        .base {
            background-color: blue
            color: white
            font-size: 16
            padding: 8 16
        }
        .primary {
            background-color: blue
        }
        .secondary {
            background-color: gray
        }
        &:hover {
            opacity: 0.8
        }
    }

    view {
        <button class="base {variant}">
            {label}
        </button>
    }
}
```

### 5.2 CSS Property Syntax

```
style_property ::= property_name ':' style_value

property_name ::= <ident> ('-' <ident>)*          // font-size, min-height
                | '--' (<ident> | '-')+            // --custom-property

style_value ::= number           // raw number (auto-px)
              | <ident>           // keyword: red, center, flex
              | <hex_color>       // #RRGGBB or #RGB
              | dimension         // 16px, 2em, 100%
              | string            // "value"
              | list              // space-separated: 8 16
```

**Number auto-px**: Bare numbers in CSS values are auto-suffixed with `px` at codegen time. `16` → `16px`.

**Space-separated list values**: `padding: 8 16` produces `8px 16px`. `border: 1 solid red` produces `1px solid red`.

### 5.3 Style AST Types

```
StyleRule ::= {
    selector: String,           // ".base", "&:hover"
    properties: Vec<(String, StyleValue)>,
}

StyleValue ::=
  | Number(f64)
  | Ident(String)
  | HexColor(String)
  | Dimension(f64, String)     // (value, unit: "px", "em", "%")
  | String(String)
  | List(Vec<StyleValuePart>)

StyleValuePart ::=
  | Number(f64)
  | Ident(String)
  | HexColor(String)
  | Dimension(f64, String)
```

### 5.4 Dynamic Style Classes

In view templates, classes can include dynamic expressions via `{expr}`:
```
<button class="base {variant} {isActive ? "active" : ""}">
```

The `class` attribute recognizes space-separated tokens where `{...}` groups are evaluated as expressions.

---

## 6. Design System

### 6.1 Design Block

```
design_def ::= 'design' <ident> '{' design_body '}'

design_body ::= (token_def | class_def | variant_def)*
```

The design block body is parsed as a raw string (not token-by-token). A dedicated sub-parser (`parse_design_body`) processes the body string.

### 6.2 Token Definition

```
token_def ::= 'token' <ident> '=' value

// Token categories (by naming convention):
//   color-*     → color tokens
//   size-*      → spacing/size tokens
//   font-*      → typography tokens
//   radius-*    → border radius tokens
//   shadow-*    → box shadow tokens
//   z-*         → z-index tokens
//   duration-*  → animation duration tokens
//   ease-*      → easing curve tokens
```

```
design Colors {
    token color-primary = #3B82F6
    token color-bg = #FFFFFF
    token color-text = #111827
    token size-sm = 8
    token size-md = 16
    token size-lg = 24
    token font-base = "Inter"
    token radius-md = 8
    token duration-fast = 150
}
```

### 6.3 Token Usage (via $ prefix)

Tokens are referenced in style blocks and props using `$` prefix:
```
.base {
    background-color: $color-primary
    padding: $size-md
    font-family: $font-base
}
```

### 6.4 Class Definition

```
class_def ::= 'class' <ident> '{' (style_property)* '}'    // deferred — partial support
```

### 6.5 Variant Definition

```
variant_def ::= 'variant' <ident> '{' variant_values '}'   // deferred — partial support
```

Part of the design token system. Full class and variant parsing is deferred to Tasks A3/A4.

### 6.6 Theme Definition

```
theme_def ::= 'theme' <ident> '{' (style_property)* '}'    // reserved keyword, limited parsing

// Theme IR data was removed from ModuleIR in v5 (zero consumers across 9 backends).
// Parser keyword and AST ThemeDef are retained for future use.
```

### 6.7 Design AST Types

```
DesignDef ::= {
    name: String,
    tokens: Vec<DesignToken>,
    classes: Vec<DesignClass>,
    variants: Vec<DesignVariant>,
}

ThemeDef ::= { name: String, entries: Vec<ThemeEntry> }
```

---

## 7. Animation System

### 7.1 Motion Definition

```
motion_def ::= '@motion' <ident> '{' motion_body '}'

motion_body ::= keyframe_body callbacks?
              | timeline_body
              | callback_body
```

**Disambiguation** — the first token of the body determines the mode:
| First token | Mode |
|-------------|------|
| `0%` — `100%` or `IntLiteral` / `FloatLiteral` | Keyframe (percentage-based) |
| `from` / `to` | Keyframe (from-to syntax) |
| `set` / `stagger` / `label` | Timeline body |
| `onStart` / `onUpdate` / `onComplete` / `onRepeat` / `onInterrupt` | Callback body |
| Any other `<ident>` | Timeline body (motion reference) |

### 7.2 Keyframe Body

```
keyframe_body ::= (keyframe_stop | keyframe_config)+ callbacks?

keyframe_stop ::= percentage '{' style_property+ '}'
                | ('from' | 'to') '{' style_property+ '}'

percentage ::= <int_literal> '%' | <float_literal> '%'

keyframe_config ::= <ident> '=' value
// Examples: duration=500, ease="easeOut", delay=200
```

```
@motion fadeIn {
    from {
        opacity: 0
        scale: 0.8
    }
    to {
        opacity: 1
        scale: 1
    }
    duration=300
    ease="easeOut"
}
```

### 7.3 Timeline Body

```
timeline_body ::= (timeline_step)+

timeline_step ::= 'set' '{' (style_property)+ '}'
                | 'stagger' '{' stagger_config? (timeline_step)+ '}'
                | 'label' <string_literal>
                | <ident>                                // motion reference
                | 'if_mode' <ident> ':' <ident> timeline_step  // conditional

stagger_config ::= 'each' '=' number ';'
```

```
@motion cardEntrance {
    stagger {
        each=60;
        fadeIn
        slideUp
    }
}
```

### 7.4 Callback Body

```
callback_body ::= (callback_spec)+

callback_spec ::= ('onStart' | 'onUpdate' | 'onComplete' | 'onRepeat' | 'onInterrupt') block
```

```
@motion pulse {
    0% { scale: 1 }
    50% { scale: 1.2 }
    100% { scale: 1 }
    duration=600

    onStart { print "Animation started" }
    onComplete { print "Animation done" }
}
```

### 7.5 Timeline Definition

```
timeline_def ::= '@timeline' <ident> '{' timeline_body '}'
```

Standalone timeline — can be reused across components:
```
@timeline staggeredCards {
    stagger {
        each=80;
        fadeIn
        cardEntrance
    }
}
```

### 7.6 Motion Triggers (Element Props)

Applied as element attributes. The trigger value is a motion reference:
```
motion_trigger ::= '@enter' '=' '{' expr '}'
                 | '@leave' '=' '{' expr '}'
                 | '@hover' '=' '{' expr '}'
                 | '@unhover' '=' '{' expr '}'
                 | '@press' '=' '{' expr '}'
                 | '@focus' '=' '{' expr '}'
                 | '@animate' '=' '{' expr '}'
                 | '@inView' '=' '{' expr '}'
                 | '@scroll' '=' '{' expr '}'
                 | '@dragHorizontal' '=' '{' expr '}'
                 | '@dragVertical' '=' '{' expr '}'
                 | '@timeline' '=' '{' expr '}'     // trigger by timeline
```

```
<div @enter={fadeIn} @leave={fadeOut} />
<button @hover={scaleUp} @press={bounce} />
<section @inView={slideIn} @inViewOnce={true} @inViewThreshold={0.2} />
```

### 7.7 Motion Modifier Attributes

```
driven_by       ::= '@drivenBy' '=' '{' expr '}'            // external progress signal
paused_override ::= '@paused' '=' '{' expr '}'               // pause state
hover_reverse   ::= '@hoverReverse' '=' (<string_literal> | '{' <ident> '}')

// InView modifiers
in_view_once    ::= '@inViewOnce' '=' '{' expr '}'
in_view_repeat  ::= '@inViewRepeat' '=' '{' expr '}'
in_view_threshold ::= '@inViewThreshold' '=' '{' expr '}'

// Scroll modifiers
scroll_start ::= '@scrollStart' '=' <string_literal>
scroll_end   ::= '@scrollEnd' '=' <string_literal>
scroll_scrub ::= '@scrollScrub' '=' '{' expr '}'

// Drag modifiers
drag_constraints ::= 'dragConstraints' '=' <string_literal>
drag_elastic     ::= 'dragElastic' '=' '{' expr '}'
drag_scrub       ::= 'dragScrub' '=' '{' expr '}'
drag_momentum    ::= 'dragMomentum' '=' '{' expr '}'
drag_threshold   ::= 'dragThreshold' '=' '{' expr '}'
drag_velocity    ::= 'dragVelocityMax' '=' '{' expr '}'
```

### 7.8 Hero Transition Attributes

```
@hero           ::= '@hero' '=' <string_literal>          // shared element tag
@heroTransition ::= '@heroTransition' '=' '{' expr '}'    // transition motion
```

```
<img @hero="avatar" @heroTransition={heroMove} />
```

### 7.9 Motion AST Types

```
MotionDef ::= {
    name: String,
    body: MotionBody,
}

MotionBody ::=
  | Keyframe(KeyframeBody)
  | Timeline(Vec<TimelineStep>)
  | Callbacks(Vec<CallbackSpec>)

KeyframeBody ::= {
    stops: Vec<KeyframeStop>,        // 0% { ... }, 100% { ... }
    config: FxHashMap<String, Value>, // duration, ease, delay, etc.
    callbacks: Vec<CallbackSpec>,
}

TimelineDef ::= { name: String, body: Vec<TimelineStep> }

TimelineStep ::=
  | Set { props: Vec<(String, StyleValue)> }
  | Stagger { config: StaggerConfig, children: Vec<TimelineStep> }
  | Label(String)
  | MotionRef(String)
  | IfMode { mode: String, value: String, body: Box<TimelineStep> }

CallbackSpec ::= { kind: CallbackKind, body: Vec<Stmt> }

TriggerKind ::= Enter | Leave | Hover | Unhover | Press | Focus | Animate
              | InView | Scroll | DragHorizontal | DragVertical | Timeline

MotionTrigger ::= { kind: TriggerKind, motion: Expr }
```

---

## 8. Error Recovery in Views

### 8.1 Directive Fallback

When `@if` or `@for` directive parsing fails, the parser **restores position** and re-parses the content as raw text. This handles edge cases like `@for key={expr} Demo` in heading elements where `@for` appears as text, not a directive.

### 8.2 Interpolation Fallback

When `{expr}` parsing fails inside views, the parser collects all tokens up to the next `}` as literal text. This prevents malformed expressions from breaking the entire view tree.

### 8.3 HTML Comment Skip

`<!-- ... -->` is detected at `<` + `!` and silently consumed. The parser expects `-->` tokenized as `Minus + Arrow`.

---

## Appendix A: @-Directive Quick Reference

| Directive | Context | Syntax |
|-----------|---------|--------|
| `@if` | View, Top-level | `@if (expr) { ... }` |
| `@else` | View | `@else { ... }` or `@else @if (...) { ... }` |
| `@for` | View | `@for item in expr { ... }` |
| `@slot` | View | `@slot(name) { default }` |
| `@empty` | View | `@empty { ... }` |
| `@native` | View | `@native { ... }` |
| `@fallback` | View | `@fallback { ... }` |
| `@error` | View | `@error { ... }` |
| `@store` | Component | `@store Name;` |
| `@mount` | Component | `@mount { ... }` |
| `@unmount` | Component | `@unmount { ... }` |
| `@effect` | Component | `@effect { ... }` |
| `@motion` | Top-level, Component | `@motion name { ... }` |
| `@timeline` | Top-level, Component | `@timeline name { ... }` |
| `@hero` | Top-level, Element attr | `@hero="tag"` |
| `@heroTransition` | Element attr | `@heroTransition={expr}` |

## Appendix B: Motion Trigger Attributes Quick Reference

| Attribute | Value | Description |
|-----------|-------|-------------|
| `@enter={motion}` | Expression | Play on mount |
| `@leave={motion}` | Expression | Play on unmount |
| `@hover={motion}` | Expression | Play on hover start |
| `@unhover={motion}` | Expression | Reverse on hover end |
| `@press={motion}` | Expression | Play on click/tap |
| `@focus={motion}` | Expression | Play on focus |
| `@animate={motion}` | Expression | Continuous animation |
| `@inView={motion}` | Expression | Play when scrolled into view |
| `@scroll={motion}` | Expression | Linked to scroll position |
| `@dragHorizontal={motion}` | Expression | Linked to horizontal drag |
| `@dragVertical={motion}` | Expression | Linked to vertical drag |
| `@timeline={motion}` | Expression | Drive a timeline animation |

## Appendix C: Full Component Example

```omni
import "omniview";

component TodoApp {
    signal todos: Array<Todo> = []
    signal input: String = ""

    if_mode default: "light"

    @store ThemeStore

    action addTodo() {
        if input != "" {
            todos = [...todos, Todo { text: input, done: false }]
            input = ""
        }
    }

    style {
        .container {
            padding: 16
            max-width: 600
            margin: 0 auto
        }
        .input-row {
            display: flex
            gap: 8
        }
    }

    @mount { print "TodoApp mounted" }

    view {
        <div class="container">
            <h1>Todo List</h1>
            <div class="input-row">
                <input
                    bind:value={input}
                    placeholder="What needs to be done?"
                />
                <button @press={addTodo}>Add</button>
            </div>
            @for todo in todos {
                <div class="todo-item">
                    <input type="checkbox" checked={todo.done} />
                    <span>{todo.text}</span>
                </div>
            }
            @if (todos.length == 0) {
                <p>No todos yet!</p>
            }
        </div>
    }
}
```
