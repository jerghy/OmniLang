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
                   | slot_decl
                   | motion_def
                   | lifecycle_hook
                   | function_def          // 'fn' — from grammar-lang
                   | action_def
                   | getter_decl
                   | const_decl
                   | view_block
                   | style_block
```

> Removed per semantic-level DSL (ADR-053): `if_mode_decl` (component member),
> `hero_def`, `timeline_def`. `@timeline` is no longer a lexer token at all.

### 1.2 Prop Declaration

```
prop_decl ::= 'prop' <ident> ':' type_annotation ('=' expr)? ';'?
```

Component properties — inputs passed from parent. The type annotation is
**required**; the default value is optional:
```
prop title: String
prop count: i32 = 0
prop items: Array<String>
prop onClick: fn(Event) -> void
```

### 1.3 Signal Declaration

```
signal_decl ::= 'signal' <ident> ':' type_annotation '=' expr ';'?
```

Reactive state values — trigger re-render on change. Both the type annotation
and the initializer are **required**:
```
signal counter: i32 = 0
signal isOpen: bool = false
signal name: String = ""
```

### 1.4 Computed Declaration

```
computed_decl ::= 'computed' <ident> ':' type_annotation '=' expr ';'?
                | 'computed' <ident> '(' ')' ('->' type_annotation)? block
```

Derived reactive values — recomputed when dependencies change. Two forms:
single-expression (colon form) or block form with empty parens (`()` only —
no parameters):
```
computed fullName: String = firstName + " " + lastName
computed total: i32 = price * quantity
computed label() -> String {
    if count > 100 { return "Overflow" }
    return "Normal"
}
```

### 1.5 Event Declaration

```
event_decl ::= 'event' <ident> ':' type_annotation '?'? ';'?
```

Component events — emitted to parent components. The `?` suffix marks an
optional event (parent may ignore it). Event payloads are typed values, not
parameter lists — handlers receive the payload as a function argument:
```
event onSubmit: String
event onClose
event onOptional: bool?
```

### 1.6 Ref Declaration

```
ref_decl ::= 'ref' <ident> ':' type_annotation ';'?
```

DOM element reference — the type annotation is **required**:
```
ref inputEl: Element
ref container: Element
```

The ref is bound to an element with `@ref={name}` or `ref: name` in the view
(ADR-042 / ADR-053 D2).

### 1.7 Store Reference

```
store_ref     ::= '@store' <ident> ';'?
store_inline  ::= 'store' <ident> '{' (signal_decl | action_def | getter_decl)* '}'
```

Two forms:
- **Reference**: `@store AppStore;` — links to a separately defined store
- **Inline**: `store LocalState { signal count = 0; action reset() { count = 0 } }` — defines store inline

`fn` members are a compile error in stores — use `getter` or `action`
(parser/top_level/store.rs).

### 1.8 Slot Declaration

```
slot_decl ::= '@slot' slot_name slot_default?

slot_name           ::= <ident>          // named slot (default slot: bare `@slot` or name `default`)
slot_default        ::= '{' (view_node)* '}'   // fallback content
```

Declares a content slot filled by the parent (ADR-053 D5). The literal name
`default` (or a bare `@slot`) declares the default/unnamed slot; `view`/`style`
are excluded as slot names in body context (a bare `@slot` may precede the
`view { ... }` block):

```
@slot                     // default slot
@slot header              // named slot
@slot footer { <span>Default footer</span> }   // fallback content
```

> v2（design D4）：slot = 父传内容，纯净三形态（default + named + fallback）。
> `@slot name: Type` 类型约束、遗留 `@slot(name)` 指令、`slot="name"` 消费属性
> 均已删除——写这些形式是编译错误（含迁移提示）。

Slots render in the view via interpolation (`{header}`, `{children}`) and are
filled by the parent via named props (`header={<div>...</div>}`) or child
nodes (default slot). See §3.3.

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
                 | '@effect' ('(' deps ')')? block

deps ::= <ident> (',' <ident>)*
```

```
@mount { print "Component mounted" }
@unmount { print "Component will unmount" }
@effect { print "Signal changed:", counter }
@effect(searchQuery) { results = performSearch(searchQuery) }   // track deps
```

### 1.12 Component AST

```
ComponentDef ::= {
    name: String,
    props: Vec<PropDecl>,
    signals: Vec<SignalDecl>,
    computeds: Vec<ComputedDecl>,
    events: Vec<EventDecl>,
    refs: Vec<RefDecl>,
    store_refs: Vec<String>,
    slots: Vec<SlotDecl>,
    motions: Vec<MotionDef>,
    lifecycles: Vec<LifecycleHook>,   // deps: Vec<String> on @effect
    fns: Vec<FunctionDef>,
    actions: Vec<ActionDef>,
    getters: Vec<GetterDecl>,
    view: Option<ViewBlock>,
    styles: Vec<StyleRule>,
}
```

> Removed per semantic-level DSL (ADR-053): `if_mode_default`, `timelines`,
> `hero_defs` — no longer parsed as component members.

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
               | semantic_binding
               | motion_trigger
               | class_prop          // class: name / class: Design.Class / class: {k: v}
               | variant_prop        // variant: Design.Variant(dim=val, ...)
               | style_prop          // style: { css-key: expr, ... }
               | slot_content        // name={ <view nodes> }  (ADR-053 D5)
               | ref_binding         // ref: name  or  @ref={name}
```

> Removed per semantic-level DSL (ADR-053): `driven_by` (`@drivenBy`),
> `paused_override` (`@paused`), `hover_reverse` (`@hoverReverse`), the
> in-view/scroll/drag modifier attributes (`@inViewOnce`, `@scrollStart`,
> `@dragConstraints`, etc.), and the hero attributes (`@hero`,
> `@heroTransition` — `@hero` is rejected with a diagnostic).

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
event_prop ::= ('@' <ident> | <ident>) '=' '{' expr '}'
```

Event handlers bound to DOM events. `@onEventName` is the canonical spelling
(`@onPress`); the bare `onEventName` form (without `@`) is also accepted.
`@ref={name}` is a ref binding, not an event:
```
<button @onPress={handleClick}>click</button>
<input @onInput={handleInput} @onFocus={handleFocus} />
```

> Note: `@press`, `@focus` etc. (without `on`) are **motion triggers**, not
> events — see §7.4.

### 2.6 Bind Props

```
bind_prop ::= 'bind' ':' <ident> '=' ('{' signal_path '}' | <ident>)
```

Two-way data binding:
```
<input bind:value={email} />
<input type="checkbox" bind:checked={agreed} />
```

### 2.7 Semantic Bindings (Ref System)

```
semantic_binding ::= ('focus' | 'scroll-x' | 'scroll-y') '=' '{' expr '}'
```

Compiler-recognized ref bindings (ADR-053 D2) — the value must be a bare
signal reference (`{sig}`) or qualified store signal (`{Store.sig}`); the
element must also carry `@ref={name}`. Other `{expr}` values on these names
stay generic static props. The `value` semantic binding is REMOVED (design
D2 值绑定收敛): `bind:value` is the sole two-way form, `value={expr}` is a
plain one-way prop — `value={sig}` without `@ref` is exactly that plain
one-way prop (v4 5.4 discriminator); only the v1 two-way shape (`value={sig}`
+ `@ref`) is a compile error with the migration hint「双向绑定请用 bind:value；
单向显示请用 value={getter()}（计算表达式）或 value={computed}」:
```
<input @ref={searchBox} focus={isSearchActive} bind:value={searchText} />
<ScrollView @ref={scrollPane} scroll-y={scrollPosition} />
```

### 2.8 Component Call

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

### 2.9 Text Content

Text in views is collected from consecutive non-special tokens. The parser reconstructs whitespace by checking span byte gaps — if token N+1 starts after token N ends, a space is inserted.

Text tokens include: identifiers (camelCase), punctuation, operators outside of `{expr}` context.

```
<div>Hello world</div>
<Text>Score: {count}</Text>
```

### 2.10 Interpolation

```
interpolation ::= '{' expr '}'          // inside view context
```

```
Text { "Hello, " + name }
<div>Count: {counter}</div>
```

**Disambiguation**: `{expr}` inside views first tries parse-as-expression. If the expression parse succeeds AND the next token is `}`, it's an interpolation. Otherwise it falls back to literal text (collecting tokens until `}`).

### 2.11 HTML Comments

```
html_comment ::= '<!--' (any_char)* '-->'
```

HTML comments are parsed and discarded (produce `ViewNode::Text("")`). Note: `-->` is tokenized as `Minus + Arrow` since the lexer merges `->` into a single `Arrow` token.

### 2.12 View AST Types

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
  | Bind { name: String, signal_path: String }
  | RefBinding { binding: String, signal: Expr }     // focus/scroll-x/scroll-y (value removed — D2)
  | Event { name: String, handler: EventHandler }
  | Class { value: ClassValue }                       // StyleRef | ClassMap | DesignClass
  | Variant { design_name: String, variant_name: String, dimension_values: Vec<(String, String)> }
  | Style { entries: Vec<(String, Expr)> }
  | TestId(String)
  | Ref(String)                                       // ref: name  or  @ref={name}
  | SlotContent { name: String, content: Vec<ViewNode> }   // name={ <nodes> }
  | MotionTrigger(MotionTrigger)
```

> Removed per semantic-level DSL (ADR-053): `DrivenBy`, `PausedOverride`,
> `HoverReverse`, `InViewOnce/Repeat/Threshold`, `ScrollStart/End/Scrub`,
> `DragConstraints/Elastic/Scrub/Momentum/Threshold/VelocityMax`,
> `HeroTag`, `HeroTransition`.

---

## 3. View Directives

### 3.1 @if / @else if / @else

```
if_directive ::= '@if' expr transition_mode? '{' (view_node)* '}'
                 ('@else' ('@if' | 'if') expr '{' (view_node)* '}' )*
                 ('@else' '{' (view_node)* '}')?

transition_mode ::= 'mode' ':' (<string_literal> | <ident>)   // "fade" | "wait" | "none"
```

Conditional rendering with optional transition mode — the mode value is
validated at parse time (`fade`, `wait`, `none` only; anything else is a
parse error):
```
@if isVisible {
    <div>visible content</div>
} @else if isLoading {
    <div>loading...</div>
} @else {
    <div>fallback</div>
}

@if show mode: "wait" {
    Panel { Text { "animated" } }
}
```

**Note**: `@else` supports both `@else @if` and `@else if` (without `@` prefix) for the else-if branch.

### 3.2 @for

```
for_directive ::= '@for' <ident> 'in' expr (key_expr)? (scroll_hint)? '{' (view_node)* '}' ('@empty' '{' (view_node)* '}')?

key_expr   ::= 'key=' '{' expr '}'
scroll_hint ::= '[' 'scroll' ']'
```

List rendering:
```
@for item in items {
    <li>{item.name}</li>
}

@for card in cards {
    Card(title: card.title) { Text { card.body } }
}

@for item in items key={item.id} [scroll] {
    <li>{item.name}</li>
} @empty {
    <li>No items</li>
}
```

**Note**: The `@for` directive uses **speculative parsing** — if the parse fails (e.g., `@for key={expr} Demo` in text content), it falls back to collecting raw text starting with `@for`.

### 3.3 @slot

The canonical form is the **body-level declaration** (§1.8, ADR-053 D5).
Slots render in the view as interpolations:

```
slot_render ::= '{' <ident> '}'          // render named slot: {header}
              | '{' 'children' '}'       // render default slot: {children}
```

```
component Modal {
    @slot header
    @slot default

    view {
        <div class="modal-header">{header}</div>
        <div class="modal-body">{children}</div>
    }
}
```

The parent fills slots via **named props** (anonymous content fills the
default slot):
```
<Modal header={<h2>Title</h2>}>
    <p>Body content</p>
</Modal>
```

> v2（design D4）：遗留 `@slot(name)` 指令形式已删除——它是编译错误
> （「改用 body 级声明 @slot name=... 」）。view 内的 `@slot name { ... }`
> 指令保留（与 body 级声明同义）。

### 3.4 @empty / @fallback / @error

```
@empty   // fallback for empty lists — implemented (@for ... @empty { ... })
```

`@empty` is fully implemented as the `@for` empty-state suffix. `@fallback`
and `@error` were REMOVED as dead lexer tokens (Task 4.5 — no parser
production ever consumed them; the "reserved directive" status was a silent
literal-text fiction). They now lex as plain `Ident("@fallback")` /
`Ident("@error")` — same disposition as `@native` (Task 4.4 — the spec §1.2
platform-layer escape hatch never had a compiler consumer) — so
`@fallback(...)` / `@error(...)` fail at parse time instead of lexing as
directives.

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

Store fields are accessed via the store-qualified form only (`Store.field`):
```
AppStore.counter    // read a store signal
AppStore.greeting   // read a store getter
AppStore.setName(x) // call a store action
```

(The `$counter` / `$store.counter` access syntax is doc-fiction — removed;
see design 域 5「`$counter`/`$store.counter` 存取语法删除」.)

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

Each style rule body is a `{ }` block containing CSS property declarations (entries separated by `;`):
```
component Button {
    prop label: String
    prop variant: String = "primary"

    style {
        .base {
            background-color: blue;
            color: white;
            font-size: 16;
            padding: 8 16;
        }
        .primary {
            background-color: blue;
        }
        .secondary {
            background-color: gray;
        }
        &:hover {
            opacity: 0.8;
        }
    }

    view {
        <button class: base {variant == "primary" ? "primary" : "secondary"}>
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

### 5.4 Class Attribute (`class:` forms)

Dynamic class binding uses the `class:` prefix forms — the `{expr}` groups
below are only recognized after `class:`, not inside a plain `class="..."`
string attribute:

```
class_prop ::= 'class:' class_list                     // space-separated names
             | 'class:' '{' (class_key ':' expr (',' class_key ':' expr)*)? '}'
             | 'class:' <design_name> '.' <class_name>
             | 'class:' class_list '{' expr '}'        // combined static + dynamic

class_list ::= <ident> ('-' <ident> | '-' <int_literal>)*
             ( ' ' <ident> ('-' <ident> | '-' <int_literal>)* )*
```

Four forms:

- **Static names** — space-separated: `class: base primary`
- **Dynamic map** — `{ key: expr }` pairs, expr is a boolean gate:
  `class: { active: isActive, disabled: isLocked }`
- **Design class reference** — `class: LearnDesign.Card`
- **Combined** — static names plus one `{expr}` whose result is appended:
  `class: tab-btn {if activeTab == "tab1" { "active" } else { "" }}`

```
<button class: base {variant == "primary" ? "primary" : "secondary"}>
<button class: { active: isActive, disabled: isLocked }>
<div class: LearnDesign.Card>
<div class: tab-btn {if activeTab == "tab1" { "active" } else { "" }}>
```

A plain `class="container"` (HTML attribute form) is still accepted as a
static string prop; use the `class:` forms for dynamic and design-system
classes (see `10-design-system.md`).

---

## 6. Design System

### 6.1 Design Block

```
design_def ::= 'design' <ident> '{' design_body '}'

design_body ::= (themes_directive | default_theme_directive
               | token_def | class_def | variant_def)*
```

The design block body is parsed as a raw string (not token-by-token). A dedicated sub-parser (`parse_design_body`) processes the body string.

### 6.2 Token Definition

```
token_def    ::= 'token' <ident> ':' <token_type> '=' <token_value> ';'

token_type   ::= 'color' | 'size' | 'radius' | 'fontFamily' | 'fontSize'
               | 'fontWeight' | 'lineHeight' | 'letterSpacing' | 'shadow'
               | 'duration' | 'easing' | 'opacity' | 'zIndex'

token_value  ::= <string_literal>           // "#6366f1", "16px"
               | <ident>                    // CSS keyword or const reference
               | <expr>                     // Phase 2: runtime expression
```

Theme overrides: `token primary: color = "#6366f1" dark "#a5b4fc";`
Theme values may be literals or const references (not runtime expressions).

```
design AppDesign {
    @themes { light, dark }
    @default-theme light;

    token primary: color = "#6366f1" dark "#a5b4fc";
    token spacing: size = "16px";
    token radiusMd: radius = "8px";
    token fontBase: fontFamily = "Inter";
    token durationFast: duration = "150ms";
    token easeOut: easing = "cubic-bezier(0.16, 1, 0.3, 1)";
}
```

### 6.3 Token Usage (`token(...)` refs)

Tokens are referenced inside `design` classes and variants with the
`token(name)` call form — either a bare `token(name)` value or a mixed
value concatenating string literals with `token(...)` parts:

```
design_value ::= 'token' '(' <ident> ')'                  // token(primary)
               | <string_literal>                         // "16px"
               | mixed                                    // "1px solid " token(border)
```

```
design AppDesign {
    token primary: color = "#6366f1";
    token border: color = "#d4d4d8";

    class Card {
        background: token(primary);                   // bare token ref
        border: "1px solid " token(border);           // mixed literal + token ref
    }
}
```

`token(name)` resolves to `var(--name)` at lowering (tokens are emitted as
`:root { --name: value }` CSS custom properties). In **component** `style`
blocks and inline styles, tokens are referenced directly as CSS variables
(`var(--name)`) — the `token()` form is design-block-only:

```
style {
    .card {
        background: "var(--surface)"
    }
}
```

> The `$name` prefix form is **not** part of the language — `$` reference
> syntax is only considered for a future language version (parser
> `design.rs`). Use `token(name)` in design blocks and `var(--name)`
> elsewhere. See `09-styling.md` and `10-design-system.md` for usage guides.

### 6.4 Class Definition

```
class_def ::= 'class' <ident> '{' (design_property | pseudo_state)* '}'

design_property ::= <property_name> ':' design_value ';'

pseudo_state ::= '&:' <ident> '{' design_property* '}'    // &:hover, &:focus
               | '&::' <ident> '{' design_property* '}'   // &::before (pseudo-element)
```

Fully implemented — reusable CSS rules defined inside `design` blocks.
Values are string literals, CSS keywords, or `token(name)` references
(§6.3); pseudo-states nest under `&:` (pseudo-class, stored as `"hover"`)
or `&::` (pseudo-element, stored as `"::before"`):

```
design AppDesign {
    token surface: color = "#ffffff";
    token border: color = "#d4d4d8";

    class Card {
        background: token(surface);
        border: "1px solid " token(border);
        &:hover {
            box-shadow: "0 4px 12px rgba(0,0,0,0.08)";
        }
    }
}
```

Used in views via `class: DesignName.ClassName` (§5.4). See
`10-design-system.md` §2 for the full guide.

### 6.5 Variant Definition

```
variant_def ::= 'variant' <ident> '{' variant_body '}'

variant_body ::= (dim_def | default_block | base_block | branch)*

dim_def       ::= '@dim' <ident> ':' <ident> ('|' <ident>)* ';'
default_block ::= '@default' '{' (<ident> ':' <ident> ';')* '}'
base_block    ::= 'base' '{' design_property* '}'
branch        ::= <dim_name> '.' <value_name> '{' design_property* '}'
```

Fully implemented — multi-dimension parameterized component styles:
`@dim` declares a dimension and its allowed values, `@default` sets the
fallback value per dimension, `base` holds styles shared by all variants,
and `dim.value` blocks override styles for a specific dimension value:

```
design AppDesign {
    token radiusMd: radius = "8px";
    token primary: color = "#6366f1";
    token danger: color = "#ef4444";
    token ghost: color = "#d4d4d8";

    variant Button {
        @dim color: primary | danger | ghost;
        @dim size: sm | md | lg;
        @default { color: primary; size: md; }
        base {
            border: none;
            border-radius: token(radiusMd);
        }
        color.primary { background: token(primary); color: white; }
        size.md { padding: "8px 16px"; font-size: "14px"; }
    }
}
```

Applied in views via `variant: DesignName.VariantName(dim=val, ...)` (§2.3).
See `10-design-system.md` §3 for the full guide.

### 6.6 Theme Definition

```
themes_directive        ::= '@themes' '{' <ident> (',' <ident>)* '}'
default_theme_directive ::= '@default-theme' <ident> ';'

theme_def ::= 'theme' <ident> '{' (style_property)* '}'    // reserved keyword, limited parsing
```

`@themes` declares the design system's theme names; `@default-theme` sets the
initial theme (used when no `data-theme` attribute is present). Token
definitions provide per-theme value overrides keyed by theme name (§6.2):

```
design AppDesign {
    @themes { light, dark }
    @default-theme light;

    token primary: color = "#6366f1" dark "#a5b4fc";
}
```

> The `theme` keyword remains reserved with limited parsing (AST `ThemeDef`
> retained for future use — Theme IR data was removed from ModuleIR in v5,
> zero consumers across 9 backends). Use the `@themes` / `@default-theme`
> directives for theme declaration.

```
ThemeDef ::= { name: String, entries: Vec<ThemeEntry> }   // reserved — limited parsing
```

### 6.7 Design AST Types

```
DesignDef ::= {
    name: String,
    tokens: Vec<TokenDef>,
    classes: Vec<DesignClassDef>,
    variants: Vec<VariantDef>,
    themes: Vec<String>,               // @themes list
    default_theme: Option<String>,     // @default-theme
}

TokenDef ::= { name: String, token_type: TokenType,
               value: TokenValue, theme_values: Vec<(String, TokenValue)> }

TokenValue ::= Literal(String) | Variable(String) | Expression(Expr)

DesignClassDef ::= { name: String, properties: Vec<DesignProperty>,
                     pseudo_states: Vec<PseudoState> }

DesignProperty ::= { name: String, value: DesignValue }

DesignValue ::= Literal(String) | TokenRef(String) | Mixed(Vec<DesignValuePart>)

PseudoState ::= { pseudo: String,        // "hover" (pseudo-class) | "::before" (pseudo-element)
                  properties: Vec<DesignProperty> }

VariantDef ::= { name: String, dimensions: Vec<VariantDimension>,
                 defaults: Vec<(String, String)>, base: Vec<DesignProperty>,
                 branches: Vec<VariantBranch> }

VariantDimension ::= { name: String, values: Vec<String> }

VariantBranch ::= { dimension: String, value: String,
                    properties: Vec<DesignProperty> }
```

---

## 7. Animation System

> Semantic-level DSL (ADR-052/053) notes: timeline bodies (`set`/`stagger`/
> `label`), callback bodies (`onStart`/`onComplete`), `@timeline` definitions,
> and hero transitions were removed (ADR-032 scoping). `@motion` bodies are
> **keyframe-only**. See `11-motion-animation.md` for the full guide.

### 7.1 Motion Definition

```
motion_def ::= '@motion' <ident> '{' keyframe_body '}'
```

**Disambiguation** — the first token of the body determines the mode:
| First token | Mode |
|-------------|------|
| `0%` — `100%` or `IntLiteral` / `FloatLiteral` | Keyframe (percentage-based) |
| `from` / `to` | Keyframe (from-to syntax) |
| Anything else | Parse error |

### 7.2 Keyframe Body

```
keyframe_body ::= keyframe_stop+ shared_timing? playback_props?

keyframe_stop ::= percentage '{' anim_prop (','? anim_prop)* '}' stop_ease?
                | ('from' | 'to') '{' anim_prop (','? anim_prop)* '}' stop_ease?

percentage ::= <int_literal> '%' | <float_literal> '%'
             | <int_literal> | <float_literal>      // bare number = percent

anim_prop ::= semantic_anim_prop ':' expr
            | '--' ...                              // CSS custom property passthrough (CSS targets only, ADR-053 D3)

stop_ease ::= 'ease' (':' easing_expr)?             // per-stop easing (colon optional)
```

Animation properties are validated against a **19-property semantic
whitelist** (`opacity`, `scale`, `scaleX`, `scaleY`, `translateX`,
`translateY`, `translateZ`, `rotate`, `rotateX`, `rotateY`, `rotateZ`,
`skewX`, `skewY`, `backgroundColor`, `color`, `width`, `height`,
`borderRadius`, `fontSize` — the last two added by v2 Task 5.2, verified
on all three targets); unknown properties are parse errors.
`--custom-property` names pass through for CSS-based targets (ADR-053 D3).
`borderRadius`/`fontSize` values are treated as lengths (bare numbers or
`"Npx"` strings).

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
    over: 300ms
    easing: ease-out
}
```

### 7.3 Shared Timing & Playback Props

```
shared_timing ::= ('over' ':' duration_expr | 'easing' ':' easing_expr)
                  (','? ...)*                      // both optional, comma optional

duration_expr ::= <int_literal> ('ms' | 's')? | <float_literal> ('ms' | 's')?
                // bare number = seconds

playback_props ::= (playback_prop)*

playback_prop ::= 'loop' (int)?                    // bare `loop` = infinite
                | 'direction' ('normal' | 'reverse' | 'alternate' | 'alternateReverse')
                | 'fill' ('none' | 'forwards' | 'backwards' | 'both')
                | 'delay' expr                     // space form — first-play delay in ms
```

```
@motion pulse {
    from { scale: 1 }
    to { scale: 1.05 }
    over: 800ms
    easing: ease-in-out

    loop                    // infinite loop
    direction alternateReverse
    delay 500               // NOTE: space form, not `delay:`
    fill forwards
}
```

> `paused` (playback keyword) and the element-level `@paused={...}` attribute
> were **deleted** (ADR-053; `PlaybackIR.paused` removed). Both now produce a
> hard parse error with a migration hint: 「paused 已删除，用 @if 条件渲染替代」.

### 7.4 Motion Triggers (Element Props)

Applied as element attributes. The trigger value is a motion reference.
Spellings are **kebab-case** (8 triggers):

```
motion_trigger ::= '@enter' '=' '{' expr '}'
                 | '@hover' '=' '{' expr '}'
                 | '@press' '=' '{' expr '}'
                 | '@focus' '=' '{' expr '}'
                 | '@in-view' '=' '{' expr '}'
                 | '@scroll' scroll_params? '=' '{' expr '}'
                 | '@drag-horizontal' drag_params? '=' '{' expr '}'
                 | '@drag-vertical' drag_params? '=' '{' expr '}'

scroll_params ::= ('start:' expr | 'end:' expr | 'scrub:' expr)*
                // colon-form control dimensions (v1 syntax, restored by
                // v2 Task 5.1); any subset, any order; repeated keys are
                // silently overwritten (last wins)

drag_params   ::= ('constraints:' '[' expr ',' expr ']'
                 | 'elastic:' expr | 'threshold:' expr)*
                // drag control dimensions (restored by v2 Task 5.1;
                // velocity_max removed in v4 — zero consumers, see deferral #84)
```

Semantic validation (`validate_motion`): `@scroll` `start`/`end` must be
numeric, must appear as a pair, and must form a strictly increasing range
(`start < end`); `scrub` must be boolean. `@drag-*`
`constraints` must be a numeric `[min, max]` range with `min < max`;
`elastic`/`threshold` must be non-negative numbers. Unknown
keys and malformed values are hard parse errors (no parse-and-ignore); the
other five triggers accept no sub-parameters.

```
<div @enter={fadeIn} />
<button @hover={scaleUp} @press={bounce} />
<input @focus={focusGlow} />
<section @in-view={slideIn} />
<div @scroll={scrollProgress} />
<div @scroll start: 0 end: 1000 scrub: true ={scrollProgress} />
<div @drag-horizontal={dragSlide} @drag-vertical={dragSlideY} />
<div @drag-horizontal constraints: [-100, 100] elastic: 0.5 threshold: 0.3 ={dragSlide} />
```

> Removed per ADR-053: `@leave`, `@unhover`, `@animate`, `@timeline` triggers
> and the legacy dash-form sub-attributes (`@in-view-once`, `@scroll-start`,
> `@drag-constraints` etc.). The control dimensions were **restored by v2
> (2026-08-03, Task 5.1)** as colon-form sub-parameters on `@scroll` /
> `@drag-*` (see `scroll_params` / `drag_params` above; deferral #19 closed).
> `@hero` is rejected with a diagnostic.

### 7.5 Motion AST Types

```
MotionDef ::= { name: String, body: MotionBody, span: Span }

MotionBody ::= Keyframe(KeyframeBody)

KeyframeBody ::= {
    stops: Vec<KeyframeStop>,      // from/to/percentage stops
    over: Expr,                    // duration (default 0.3)
    easing: Option<EasingExpr>,
    playback: Vec<PlaybackProp>,   // Loop | Direction | Fill | Delay
}

KeyframeStop ::= { position: StopPosition,   // From | To | Percent(f64)
                   properties: Vec<(String, Expr)>,
                   ease: Option<EasingExpr> }

EasingExpr ::= Named(String) | CubicBezier { x1, y1, x2, y2 } | Steps { count, direction }
             | Back { direction } | Elastic { direction }

PlaybackProp ::= Loop(LoopValue) | Direction(DirectionValue)
               | Fill(FillValue) | Delay(Expr)

TriggerKind ::= Enter | Hover | Press | Focus | InView | Scroll
              | DragHorizontal | DragVertical

MotionTrigger ::= { kind: TriggerKind, motion_expr: Expr }
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
| `@if` | View | `@if expr { ... }` or `@if expr mode: "wait" { ... }` |
| `@else` | View | `@else { ... }` or `@else @if (...) { ... }` / `@else if (...) { ... }` |
| `@for` | View | `@for item in expr key={expr} [scroll]? { ... }` |
| `@empty` | View | `@empty { ... }` (suffix after `@for` block) |
| `@slot` | Component body | `@slot name { fallback }?` (render: `{name}` / `{children}`; v2 D4 删除了类型约束) |
| `@store` | Component | `@store Name;` |
| `@mount` | Component | `@mount { ... }` |
| `@unmount` | Component | `@unmount { ... }` |
| `@effect` | Component | `@effect (deps)? { ... }` |
| `@motion` | Top-level, Component | `@motion name { from ... to ... }` |
| `@themes` | Design | `@themes { light, dark }` |
| `@default-theme` | Design | `@default-theme light;` |

> Removed per ADR-053: `@timeline`, `@hero`, `@heroTransition`.
> The v1 semicolon-terminated form (`@themes light, dark` with a trailing
> `;`) hard-errors with a migration hint — v2 requires the brace form
> `@themes { light, dark }` (§6.6, parser/design.rs).

## Appendix B: Motion Trigger Attributes Quick Reference

| Attribute | Value | Description |
|-----------|-------|-------------|
| `@enter={motion}` | Expression | Play on mount |
| `@hover={motion}` | Expression | Play on hover start |
| `@press={motion}` | Expression | Play on click/tap |
| `@focus={motion}` | Expression | Play on focus |
| `@in-view={motion}` | Expression | Play when scrolled into view |
| `@scroll={motion}` | Expression | Linked to scroll position |
| `@drag-horizontal={motion}` | Expression | Linked to horizontal drag |
| `@drag-vertical={motion}` | Expression | Linked to vertical drag |

> Removed per ADR-053: `@leave`, `@unhover`, `@animate`, `@timeline`
> triggers, and all trigger sub-attributes (`@inViewOnce`, `@scrollStart`,
> `@dragConstraints`, ...).

## Appendix C: Full Component Example

```omni
import omniview;

struct Todo {
    text: String,
    done: bool,
}

store ThemeStore {
    signal mode: String = "light";
}

component TodoApp {
    signal todos: Array<Todo> = [];
    signal input: String = "";

    @store ThemeStore;

    action addTodo() {
        if input != "" {
            todos = [...todos, Todo { text: input, done: false }];
            input = "";
        }
    }

    style {
        .container {
            padding: 16;
            max-width: 600;
            margin: 0 auto;
        }
        .input-row {
            display: flex;
            gap: 8;
        }
    }

    @mount { print("TodoApp mounted"); }

    view {
        <div class="container">
            <h1>Todo List</h1>
            <div class="input-row">
                <input
                    bind:value={input}
                    placeholder="What needs to be done?"
                />
                <button @onPress={addTodo}>Add</button>
            </div>
            @for todo in todos {
                <div class="todo-item">
                    <input type="checkbox" checked={todo.done} />
                    <span>{todo.text}</span>
                </div>
            } @empty {
                <p>No todos yet!</p>
            }
        </div>
    }
}
```
