# OmniLang Grammar Specification

> **Version:** 2.0 (2026-06-16)
> **Source:** Derived directly from the Rust compiler implementation (`omni-compiler/src/`)
> **Scope:** This document describes only what the parser actually implements. Features whose tokens exist but whose parser support is absent are noted in Appendix B.

---

## 1. Lexical Structure

Source: `lexer/token.rs`, `lexer/mod.rs`

### 1.1 Comments

```
// Single-line comment — runs to end of line
/* Block comment — can span multiple lines */
```

Block comments do not nest. Comments are stripped during tokenization and are invisible to the parser.

### 1.2 Identifiers

Identifiers start with an alphabetic character (`a-z`, `A-Z`), underscore (`_`), or any non-ASCII non-whitespace/non-control character. Subsequent characters also include digits (`0-9`).

```
myVar
_counter
data2
你好
café
```

### 1.3 Keywords (Token-Level — All Lexed)

These tokens are recognized by the lexer. Some are parsed at the top level; others exist only as tokens with no corresponding parser logic (see Appendix B).

#### OmniLang Keywords (parsed)

`fn` `let` `mut` `if` `else` `while` `for` `in` `return` `break` `continue`
`print` `println` `struct` `type` `nominal` `pub` `import` `from` `export`
`true` `false` `null`

#### OmniView Keywords (parsed)

`component` `signal` `computed` `prop` `event` `ref` `store` `view` `style`
`action` `getter` `theme` `bind`

#### @-Directive Keywords

`@store` `@if` `@else` `@for` `@empty` `@slot` `@native` `@fallback` `@error`
`@mount` `@unmount` `@effect`

#### Token-Only Keywords (lexed, but no parser support)

`const` `as` `async` `await` `match` `class` `interface` `impl` `extends`
`implements` `static` `private` `override`

These token-only keywords can appear as contextual identifiers in positions where an identifier is expected (parameter names, variable names, field names).

### 1.4 Literals

#### Integer Literals

Decimal, hexadecimal (`0x`/`0X`), octal (`0o`/`0O`), and binary (`0b`/`0B`):

```
42
0xFF
0o77
0b1010
```

Internally stored as `i64`. Overflow during parsing silently wraps/truncates.

#### Float Literals

```
3.14
1.0
2.5e10
1e-3
1.5E+2
```

Must have a digit after the decimal point. Internally stored as `f64`.

#### String Literals

Double-quoted with escape sequences:

```
"hello"
"line1\nline2"
"tab\there"
"quote \"inside\""
"unicode: \u{1F600}"
```

Escape sequences: `\"` `\\` `\n` `\t` `\r` `\u{XXXX}` (1–6 hex digits in braces).

Strings stop at `<` or `>` when inside view text content.

#### Template Strings

Backtick-quoted, **raw text only — no interpolation is parsed**:

```
`SELECT * FROM users WHERE id = ?`
````

The lexer treats everything between backticks as literal text. There is no `${expr}` interpolation in the parser.

#### Single-Quoted Strings

Used for view text content. Supports `\'` and `\\` escapes. Stops at `<` or `>` (HTML boundaries):

```
'text content'
```

#### Boolean Literals

```
true
false
```

#### Null Literal

```
null
```

### 1.5 Operators and Punctuation

```
.       Dot          ...     Spread       ?       Question mark
?.      Opt. chain   +       Plus         -       Minus
*       Star         /       Slash        %       Percent
+=      Plus assign  -=      Minus assign *=      Star assign
/=      Slash assign %=      Percent assign =     Assign
==      Equal        !=      Not equal    <       Less than
>       Greater than <=      Less/equal   >=      Greater/equal
&&      Logical AND  ||      Logical OR   !       Logical NOT
->      Arrow        =>      Fat arrow    |       Pipe
$       Dollar       #       Hash         ( )     Parentheses
{ }     Braces       [ ]     Brackets     ;       Semicolon
,       Comma        :       Colon        @       At
```

---

## 2. Program Structure

Source: `parser/mod.rs` — `parse_program()`

A source file is parsed as:

```
import*
declaration*
```

Top-level declarations are dispatched by the first token:

| Token | Declaration |
|-------|-------------|
| `import` | Import statement |
| `export` | Re-export statement |
| `component` | Component definition |
| `store` | Store definition |
| `theme` | Theme definition |
| `fn` | Private function |
| `struct` | Private struct |
| `pub` + `fn` | Public function |
| `pub` + `struct` | Public struct |
| `pub` + `type` | Public type alias |
| `type` | Type alias or Tagged Union (disambiguated by lookahead) |

Anything else at the top level produces a parse error.

---

## 3. Declarations

Source: `parser/top_level.rs`

### 3.1 Function Declaration

```
[pub] fn name(param: Type?, ...) [-> ReturnType] { block }
```

- Parameters: comma-separated `name [: Type]` pairs. Type annotation is optional.
- Return type: optional, after `->`.
- Body: a block `{ ... }`.
- Functions are **private by default**. `pub fn` makes them public.
- `async fn` is **not parsed** — the `async` keyword exists as a token but the parser always sets `is_async: false`.

```
fn greet(name: String) {
    println "Hello, " + name;
}

fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub fn public_helper(x: i32) -> i32 {
    return x * 2;
}
```

### 3.2 Struct Declaration

```
[pub] [nominal] struct Name {
    field: Type,
    field: Type,
    ...
}
```

- Fields: comma-separated `name: Type` pairs. Trailing comma allowed.
- `nominal` marks the struct as having nominal (not structural) typing.
- `pub` makes the struct visible across modules.

```
struct Point {
    x: f64,
    y: f64,
}

pub struct User {
    name: String,
    email: String,
    age: i32,
}

nominal struct Color {
    r: i32,
    g: i32,
    b: i32,
}
```

### 3.3 Type Alias

```
[pub] type Name = TypeAnnotation;
```

```
type UserMap = Map<String, User>;
type ScoreList = Array<i32>;
```

### 3.4 Tagged Union

```
type Name = Variant1 { field: Type, ... } | Variant2 | Variant3 { ... };
```

Variants can have fields (in braces) or be fieldless. The parser disambiguates from type aliases by checking whether the variant name is followed by `{` or `|`.

```
type Option = Some { value: i32 } | None;
type Result = Ok { value: String } | Error { code: i32 } | Pending;
```

### 3.5 Component Declaration

```
component Name {
    prop* signal* computed* event* ref* store_ref* const*
    fn* action* lifecycle* view? style*
}
```

Components are **always public** — `pub` is not needed and not accepted.

#### Props

```
prop name: Type [= default_expr];
```

```
prop title: String;
prop count: i32 = 0;
```

#### Signals

```
signal name: Type = init_expr;
```

```
signal count: i32 = 0;
signal items: Array<String> = [];
```

#### Computed

Two forms — simple expression or function-like:

```
computed name: Type = expr;
computed name() -> Type { block }
```

```
computed doubled: i32 = count * 2;
computed fullName() -> String {
    first + " " + last
}
```

#### Events

```
event name: Type[?];
```

The `?` suffix marks the event as optional.

```
event onClick: ClickEvent;
event onChange: String?;
```

#### Refs

```
ref name: Type;
```

```
ref divRef: Element;
ref inputRef: InputElement;
```

#### Store References

Two forms — reference to an external store, or inline definition:

```
@store StoreName;
store StoreName { signal* action* fn* }
```

`@store` reads a `@`-prefixed token. `store` without `@` allows an inline definition body containing `signal`, `action`, and `fn` items.

#### Constants

```
const name = literal;
const $prefixed = literal;
```

Constants accept only literal values (string, int, float, bool, null). The `$` prefix is optional. Internally stored as signal-like declarations.

#### Functions (inside components)

```
fn name(params) -> Type? { block }
```

Private helper functions within a component. Same syntax as top-level `fn` but always private.

#### Actions

```
action name(params) [: ReturnType] [async] { block }
```

Actions are methods on components. They support `async`:

```
action increment(amount: i32) {
    count = count + amount;
}

action fetchData() async {
    let result = api.load();
    data = result;
}
```

#### Lifecycle Hooks

```
@mount { block }
@unmount { block }
@effect(dep1, dep2) { block }
```

Dependencies on `@effect` are optional parenthesized comma-separated identifiers.

#### View

```
view {
    view_node*
}
```

Exactly one `view` block per component. See Section 6.

#### Styles

```
style className {
    property: value;
    ...
}
```

See Section 6.9 for style value formats.

### 3.6 Store Declaration

```
store Name {
    signal* getter* action*
}
```

Stores are **always public**. They contain `signal`, `getter`, and `action` items:

```
store AppStore {
    signal user: String? = null;
    signal items: Array<String> = [];

    getter hasItems: bool = items.len() > 0;

    action addItem(item: String) {
        items.push(item);
    }
}
```

#### Getter

```
getter name: Type = expr;
```

### 3.7 Theme Declaration

```
theme Name {
    categoryName {
        key: value,
        key: value,
    }
}
```

Values are string literals or identifiers. Comma-separated within categories.

```
theme AppTheme {
    colors {
        primary: "#333",
        secondary: "#666",
        background: "white",
    }
    spacing {
        small: "8px",
        medium: "16px",
    }
}
```

---

## 4. Statements

Source: `parser/stmt.rs`

### 4.1 Variable Declaration

```
let [mut] name [: Type] [= expr];
```

Variables are immutable by default. `mut` allows reassignment. Type annotation and initializer are both optional (but at least one is expected).

```
let x = 5;
let name: String = "Alice";
let mut counter: i32 = 0;
let y: i32;          // declared without init
```

### 4.2 Assignment

Assignment is parsed as a binary expression (precedence level 1, right-associative). Targets can be variable names, field access, or index access:

```
x = 10;
user.name = "Bob";
items[0] = "first";
count += 1;
```

Compound assignment operators: `=` `+=` `-=` `*=` `/=` `%=`

### 4.3 If / Else If / Else

```
if expr {
    stmt*
} [else if expr {
    stmt*
}]* [else {
    stmt*
}]
```

```
if score >= 90 {
    grade = "A";
} else if score >= 80 {
    grade = "B";
} else {
    grade = "C";
}
```

### 4.4 While Loop

```
while expr {
    stmt*
}
```

### 4.5 For-In Loop

```
for var_name in iterable_expr {
    stmt*
}
```

`var_name` is a single identifier (no destructuring). `iterable_expr` is any expression.

```
for item in items {
    println item;
}
```

### 4.6 Return

```
return [expr];
```

The expression is optional. The parser skips `return` without an expression if the next token is `}` or `;`.

```
return;
return a + b;
```

### 4.7 Break / Continue

```
break;
continue;
```

Valid only inside `while` or `for` loop bodies. Validation is semantic, not syntactic.

### 4.8 Print

```
print expr;
println expr;
```

Built-in statements — not function calls. `println` appends a newline; `print` does not.

```
println "Hello";
print 42;
```

### 4.9 Expression Statements

Any expression optionally followed by a semicolon:

```
doSomething();
items.push("new");
```

### 4.10 Blocks and Trailing Expressions

A block `{ stmt* expr? }` is parsed as statements followed by an optional trailing expression. If the last item in a block before `}` is an expression statement (no trailing semicolon check — the parser simply checks if the stmt is `ExprStmt`), it is stored as the block's `trailing_expr`.

This enables implicit return in functions:

```
fn double(x: i32) -> i32 {
    let result = x * 2;
    result       // trailing expression — implicit return
}
```

### 4.11 Semicolons

Semicolons are **optional** in most positions. The parser calls `consume_semicolon()` after statements that allow it, consuming one if present but not requiring it.

---

## 5. Expressions

Source: `parser/expr.rs` — Pratt parser

### 5.1 Precedence Table

| Level | Operators | Assoc | Description |
|-------|-----------|-------|-------------|
| 1 | `=` `+=` `-=` `*=` `/=` `%=` | Right | Assignment |
| 2 | `\|\|` | Left | Logical OR |
| 3 | `&&` | Left | Logical AND |
| 4 | `==` `!=` `<` `>` `<=` `>=` | Left | Comparison |
| 5 | `+` `-` | Left | Addition/Subtraction |
| 6 | `*` `/` `%` | Left | Multiplication/Division/Modulo |
| 7 | `!` `-` (prefix) | — | Unary NOT/Negate |
| 8 | `.` `?.` `()` `[]` | — | Postfix (member, call, index) |

### 5.2 Literal Expressions

```
42              // IntLiteral(i64)
3.14            // FloatLiteral(f64)
"hello"         // StringLiteral(String)
true            // BoolLiteral(true)
false           // BoolLiteral(false)
null            // Null
```

Template strings (backtick-quoted) are lexed as `TemplateString(String)` tokens but are **not parsed as expressions** — the parser's `parse_prefix` does not handle `TemplateString`.

### 5.3 Variable Reference

```
myVariable
count
result
```

Contextual keywords are accepted as identifiers in expression position:

```
// These all work as variable names in expressions:
event.data
signal.value
style.name
```

### 5.4 Unary Operators

```
!condition      // logical NOT
-value          // numeric negation
```

Both bind at precedence level 7 (prefix).

### 5.5 Binary Operators

All binary operators are left-associative except assignment (right-associative):

```
a + b           // addition
a - b           // subtraction
a * b           // multiplication
a / b           // division
a % b           // modulo
a == b          // equality
a != b          // inequality
a < b           // less than
a > b           // greater than
a <= b          // less or equal
a >= b          // greater or equal
a && b          // logical AND (short-circuit semantic)
a || b          // logical OR (short-circuit semantic)
```

### 5.6 Assignment Expressions

```
x = value
x += delta
x -= delta
x *= factor
x /= divisor
x %= modulus
```

Assignment is parsed as a right-associative binary expression (precedence 1). The left side can be a `VarRef`, `FieldAccess`, or `IndexGet`.

### 5.7 Function Call

```
functionName(arg1, arg2)
func()
```

The parser calls `parse_ident_expr`: if the identifier is followed by `(`, it parses a `FuncCall`. Arguments are comma-separated expressions with trailing comma allowed.

### 5.8 Method Call and Field Access

```
object.method(args)     // MethodCall — name followed by '('
object.field            // FieldAccess — name without '('
```

The parser distinguishes at the `.` token: if the name after `.` is followed by `(`, it's a `MethodCall`; otherwise it's a `FieldAccess`.

### 5.9 Optional Chaining

```
object?.field
object?.method(args)
```

The `?.` token triggers the same field/method dispatch with optional-chaining semantics (enforced during codegen, not parsing).

### 5.10 Index Access

```
array[index]
map[key]
matrix[row][col]
```

The `[` after an expression triggers `parse_infix` for `LBracket`, which parses the index expression and expects `]`.

### 5.11 Closure / Lambda

```
|param1, param2| body_expr
|| body_expr                    // no parameters
```

Parameters are comma-separated bare identifiers (no type annotations). The body is a single expression. **There is no `->` return type syntax in closures.**

```
let double = |x| x * 2;
let sum = |a, b| a + b;
let constant = || 42;
```

### 5.12 Array Literal

```
[expr1, expr2, ...]
[]
```

Comma-separated expressions between `[` and `]`. Trailing comma allowed. Empty arrays are valid.

```
let numbers = [1, 2, 3];
let empty: Array<i32> = [];
let nested = [[1, 0], [0, 1]];
```

### 5.13 Map Literal

```
{ key: value, key: value, ... }
{}
```

Key-value pairs separated by `:`, pairs separated by `,`. Trailing comma allowed. Keys are expressions (not just identifiers). Empty maps are valid.

```
let person = { name: "Alice", age: 30 };
let config = { debug: true, port: 8080 };
```

Note: the parser always treats `{...}` in prefix position as a map literal. `StructLiteral` (`TypeName { field: val }`) is defined in the AST but has no parser support.

### 5.14 If Expression

```
if condition { then_expr } else { else_expr }
```

Each branch must be a single expression inside braces:

```
let status = if x > 0 { "positive" } else { "non-positive" };
```

Note: this is parsed in `parse_prefix` (`TokenKind::If`). The condition is an expression, and each branch is a single expression enclosed in braces.

### 5.15 Parenthesized Expressions

```
(expr)
```

Used for grouping and overriding precedence:

```
let result = (a + b) * c;
```

### 5.16 Spread Expression (Reserved)

```
...expr
```

Parsed as `NotYetImplemented { feature: "spread" }`. The parser consumes the spread target to avoid leaving unconsumed tokens, but no semantic meaning is assigned.

---

## 6. View Syntax

Source: `parser/view.rs`

View syntax appears inside `view { ... }` blocks within component declarations.

### 6.1 Elements

```
<Tag attr1="val" attr2={expr}> children </Tag>
<Tag attr="val" />                             <!-- self-closing -->
```

- Tag names are identifiers.
- Closing tags must match the opening tag name.
- Self-closing tags use `/>`.

### 6.2 Attributes

#### Static Attributes

```
name="string value"
name={expression}
```

#### Boolean Attributes

An attribute name without `=` or value — sets the value to `true`:

```
<input disabled />
<button hidden>Click</button>
```

#### Hyphenated Attribute Names

Multiple hyphen-separated segments (`margin-left`, `data-testid`, `aria-label`) are supported. The parser re-joins segments with `-`.

```
<div data-testid="main" aria-label="Main content">
```

#### Class Attribute

```
class: styleRefName
class: "string literal"
class: { dynamic_class_map }
```

The `class:` prefix (identifier followed by `:`) triggers class parsing. The value can be a style reference identifier, a string literal, or a class map expression in braces.

#### Style Attribute

```
style: { key: value, key: value, ... }
```

Inline styles with the `style:` prefix. Keys are identifiers; values are expressions. Comma-separated.

```
<div style: { color: "red", fontSize: "16px" }>
```

#### Test ID

```
data-testid="value"
```

Parsed as a regular hyphenated attribute. The codegen layer may treat this specially.

#### Ref Attribute

```
ref={elementRef}
```

Boolean-like when the attribute name is `ref` with a value.

### 6.3 Event Bindings

Two equivalent forms:

```
@eventName={handler}
onEventName={handler}
```

The `on`-prefixed form requires the third character to be uppercase (e.g., `onClick`, `onChange`). The `@` form accepts any name.

**Handler values** (inside `{ }`):

1. **Function reference:** `{fnName}` — a bare identifier
2. **Function call:** `{fnName(args)}` — wraps the call in an inline `fn() { fnName(args); }`
3. **Store action:** `{store.action}` — parses as `StoreAction { store, action }`
4. **Inline function:** `{(params) => { body }}` — full arrow-function syntax with fat arrow `=>`

```
<button @onClick={increment}>+</button>
<button onPress={handlePress}>Press</button>
<input onInput={(e) => { value = e.target.value; }} />
<form @onSubmit={appStore.save}>...</form>
```

### 6.4 Two-Way Binding

```
bind:property={signalPath}
```

The `bind:` prefix activates two-way signal binding. The signal path can be a simple identifier or a dotted path (`store.field`):

```
<input bind:value={name} />
<input bind:value={store.searchText} />
```

### 6.5 Component Call Syntax

```
Name(prop: value, prop: value) { children }
Name(prop: value)                   // no children
Name                                // no props, no children
```

Uppercase-starting identifiers followed by `(` (props) or `{` (children) are parsed as component invocations. Props use `:` not `=`, and are comma-separated.

```
view {
    <div>
        <Header title="My App" />
        <Card(padding: 16) {
            <p>Card content</p>
        }
    </div>
}
```

### 6.6 Directives

#### @if / @else if / @else

```
@if condition {
    view_node*
} @else if condition {
    view_node*
} @else {
    view_node*
}
```

Supports both `@else @if` and `@else if` (without second `@`) for else-if chains.

```
@if isLoggedIn {
    <p>Welcome back!</p>
} @else if isLoading {
    <p>Loading...</p>
} @else {
    <button @onClick={login}>Sign In</button>
}
```

#### @for / @empty

```
@for item_name in iterable {
    view_node*
} @empty {
    view_node*
}
```

The `@empty` block is optional and renders when the iterable is empty.

#### @slot

```
@slot            // default slot
@slot(name)      // named slot
```

### 6.7 Interpolation

```
{expression}
```

Inside view blocks, `{...}` containing an expression (not a directive keyword) is an interpolation. Empty braces `{}` produce an empty text node.

```
<h1>{pageTitle}</h1>
<p>Hello, {user.name}!</p>
```

### 6.8 Text Content

Between elements, consecutive text-eligible tokens are coalesced into a single `Text` node. The following token types are eligible as text:

- Identifiers, string literals, numbers
- Punctuation: `:`, `,`, `.`, `+`, `-`, `*`, `/`, `?`, `!`, `(`, `)`, `|`, `$`, `#`, `%`
- Most keyword tokens (e.g., `export`, `import`, `async`, `match`, etc.)

```
<p>This is plain text with punctuation: hello, world!</p>
```

Double-quoted strings in view context are also treated as text nodes.

### 6.9 Style Rules

```
style className {
    property: value;
    property: value;
}
```

Style values support these formats (parsed from token kind):

| Token/Pattern | StyleValue Variant | Example |
|---------------|-------------------|---------|
| `IntLiteral(n)` | `Number(f64)` | `16` → `Number(16.0)` |
| `FloatLiteral(n)` | `Number(f64)` | `1.5` |
| `StringLiteral(s)` | `String(String)` | `"#ffffff"` |
| `Ident` with dimension suffix | `Dimension(f64, unit)` | `16px`, `1.5rem`, `100vh`, `50%` |
| `Ident` starting with `#` | `HexColor(String)` | `#ff0000`, `#333` |
| `Ident` (other) | `Ident(String)` | `center`, `flex`, `bold` |

```
style card {
    padding: 16px;
    border-radius: 8px;
    display: flex;
    background: "#ffffff";
    color: #333;
}
```

---

## 7. Import / Export

Source: `parser/top_level.rs` — `parse_import()`, `parse_export()`

### 7.1 Named Import

```
import { Name1, Name2 } from "./path.omni";
```

Trailing comma allowed. Path must be a string literal.

### 7.2 Namespace Import

```
import "./path.omni" as alias;
```

The path string comes first, followed by `as` and an alias identifier.

### 7.3 Type-Only Import

```
import type { Type1, Type2 } from "./path.omni";
```

### 7.4 Bare Import (Library Marker)

```
import omniview;
import name;
```

A single identifier after `import` (no braces, no string) is parsed as a library import.

### 7.5 Re-export

```
export { Name1, Name2 } from "./path.omni";
```

---

## 8. Visibility

Source: `parser/ast.rs` — `Visibility` enum; `parser/mod.rs` — `parse_program()`

| Declaration | Explicit `pub` | Implicit |
|-------------|---------------|----------|
| `fn` | `pub fn` | private |
| `struct` | `pub struct` | private |
| `type` (alias) | `pub type` | private |
| `component` | — | **always public** |
| `store` | — | **always public** |
| `theme` | — | **always public** |

`component`, `store`, and `theme` do not accept a `pub` modifier. Functions, structs, and type aliases are private by default.

---

## 9. Type Annotations

Source: `parser/stmt.rs` — `parse_type_annotation()`

Type annotations appear after `:` in variable declarations, function parameters, return types, struct fields, and component declarations.

### 9.1 Named Types

```
i32
f64
bool
String
User
```

### 9.2 Generic Type Arguments

```
Array<String>
Map<String, i32>
Map<String, Array<bool>>
```

Parsed with `<` and `>` delimiters. Multiple type arguments separated by `,`.

### 9.3 Optional Types

A `?` suffix makes a type optional (nullable):

```
String?
User?
Array<String>?
```

### 9.4 Not Implemented

The following type annotation variants are **defined in the AST but not parsed**:
- `Union` types (`i32 | String`) — no parser support in `parse_type_annotation()`
- `Fn` types (`fn(i32) -> bool`) — no parser support

---

## Appendix A: AST Node Reference

Source: `parser/ast.rs`

This appendix lists every AST node the parser can produce, for reference.

### Expressions (`Expr`)

| Variant | Fields | Description |
|---------|--------|-------------|
| `Literal(Literal, Span)` | Literal value | Integer, float, string, bool, null |
| `VarRef(String, Span)` | Variable name | Identifier reference |
| `BinaryOp { left, op, right, span }` | Box<Expr>, BinaryOpKind, Box<Expr> | Binary operation |
| `UnaryOp { op, operand, span }` | UnaryOpKind, Box<Expr> | Unary operation |
| `FuncCall { func, args, span }` | Box<Expr>, Vec<Expr> | Function call |
| `MethodCall { object, method, args, span }` | Box<Expr>, String, Vec<Expr> | Method call |
| `IndexGet { object, index, span }` | Box<Expr>, Box<Expr> | Index access |
| `FieldAccess { object, field, span }` | Box<Expr>, String | Field access |
| `ArrayLiteral { elements, span }` | Vec<Expr> | Array literal |
| `MapLiteral { entries, span }` | Vec<(Expr, Expr)> | Map literal |
| `IfExpr { condition, then_expr, else_expr, span }` | Box<Expr> × 3 | If expression |
| `TernaryExpr { condition, then_expr, else_expr, span }` | Box<Expr> × 3 | Ternary (AST only, not parsed) |
| `TemplateString { parts, span }` | Vec<TemplatePart> | Template string (AST only, not parsed) |
| `StructLiteral { type_name, fields, span }` | String, Vec<(String, Expr)> | Struct literal (AST only, not parsed) |
| `StoreFieldAccess { store_alias, field_name, span }` | String, String | Store field access |
| `ClosureExpr { params, body, span }` | Vec<String>, Box<Expr> | Closure |
| `NotYetImplemented { feature, span }` | String | Placeholder for reserved features |

### BinaryOpKind

`Add` `Sub` `Mul` `Div` `Mod` `Eq` `Neq` `Lt` `Gt` `LtEq` `GtEq` `And` `Or` `Assign` `AddAssign` `SubAssign` `MulAssign` `DivAssign` `ModAssign`

### UnaryOpKind

`Not` `Neg`

### Statements (`Stmt`)

| Variant | Description |
|---------|-------------|
| `VarDecl { name, type_ann, mutable, init, span }` | Variable declaration |
| `Assign { target, op, value, span }` | Assignment (from BinaryOp) |
| `If { condition, then_body, else_ifs, else_body, span }` | If chain |
| `While { condition, body, span }` | While loop |
| `ForIn { var_name, iterable, body, span }` | For-in loop |
| `Return { value, span }` | Return statement |
| `Break(Span)` | Break |
| `Continue(Span)` | Continue |
| `Print { newline, arg, span }` | Print/Println |
| `ExprStmt { expr, span }` | Expression statement |
| `EventEmit { name, args, span }` | Event emit (AST only, not parsed) |

### Top-Level / Declaration Nodes

- `Program { components, stores, themes, functions, structs, type_aliases, tagged_unions, imports, exports }`
- `FunctionDef { visibility, name, params, return_type, is_async, body, span }`
- `StructDef { visibility, name, is_nominal, fields, span }`
- `TypeAlias { visibility, name, type_ann, span }`
- `TaggedUnionDef { name, variants, span }`
- `ComponentDef { name, props, signals, computeds, events, refs, store_refs, lifecycles, fns, actions, view, styles, span }`
- `StoreDef { name, signals, getters, actions }`
- `ThemeDef { categories }`
- `ActionDef { name, params, return_type, body, is_async }`
- `ImportStmt::Named | Namespace | TypeOnly | Lib`
- `ExportFrom { names, path, span }`

### View Nodes

- `ViewBlock { children: Vec<ViewNode> }`
- `ViewNode::Element | IfDirective | ForDirective | SlotDirective | Text | Interpolation`
- `Element { tag, props, children, self_closing, span }`
- `ElementProp::Static | Bind | Event | Class | Style | TestId | Ref`
- `EventHandler::InlineFn | FnRef | StoreAction`
- `IfDirective { condition, body, else_ifs, else_body }`
- `ForDirective { item_name, iterable, body, empty_body }`
- `StyleRule { class_name, properties }`
- `StyleValue::Dimension | HexColor | String | Number | Ident | List`

---

## Appendix B: Token-Only Features

These tokens are recognized by the lexer but have **no corresponding parser support**. They do not appear in any AST node produced by the parser. They can be used as contextual identifiers (variable names, parameter names, field names).

| Token(s) | Lexer Keyword | Parser Status |
|----------|--------------|---------------|
| `const` | Yes | Parsed only inside `component` (as signal-like const decl) |
| `as` | Yes | Not parsed anywhere |
| `async` | Yes | Parsed only on `action`, not on top-level `fn` |
| `await` | Yes | Not parsed anywhere |
| `match` | Yes | Not parsed anywhere |
| `class` | Yes | Not parsed at top level |
| `interface` | Yes | Not parsed at top level |
| `impl` | Yes | Not parsed anywhere |
| `extends` | Yes | Not parsed at top level |
| `implements` | Yes | Not parsed at top level |
| `static` | Yes | Not parsed anywhere |
| `private` | Yes | Not parsed anywhere |
| `override` | Yes | Not parsed anywhere |

Additionally, these AST nodes are defined but have **no parser** producing them:
- `TernaryExpr` — defined, not parsed
- `TemplateString` expression — defined, but the parser does not emit TemplateString AST nodes
- `StructLiteral` — defined, not parsed (parser always emits `MapLiteral`)
- `EventEmit` — defined, not parsed
- `TypeAnnotation::Union` — defined, not parsed
- `TypeAnnotation::Fn` — defined, not parsed

---

## Appendix C: Code Generation Targets

Source: `codegen/` directory

| Target | File | Description |
|--------|------|-------------|
| `rust.rs` | Rust | Full support — primary backend |
| `python.rs` | Python | Full support |
| `javascript.rs` | JavaScript (vanilla) | Full support |
| `dart.rs` | Dart | Full support |
| `react.rs` | React (TypeScript + Zustand) | OmniView component → React app |
| `vue.rs` | Vue | OmniView component → Vue SFC |
| `dioxus.rs` | Dioxus | OmniView component → Dioxus (Rust) |
| `flutter.rs` | Flutter | OmniView component → Flutter widget |
| `omniview.rs` | OmniView (native runtime) | OmniView component → browser-native runtime |

---

## Appendix D: Semantic Nuances

- **Semicolons are optional** in most positions. The parser consumes them when present but does not require them.
- **Trailing commas** are allowed in import/export lists, function call arguments, struct fields, array literals, and map literals.
- **Contextual keywords**: Most OmniLang and OmniView keywords can appear as identifiers in expression and parameter positions. The `keyword_as_ident_name()` function maps 40+ keyword tokens back to their identifier strings.
- **Block trailing expressions**: The last expression statement in a block (before `}`) is stored as the block's `trailing_expr`, enabling implicit return in functions.
- **Event handler `@` prefix**: The lexer joins `@identifier` into a single `Ident("@identifier")` token. The parser also handles `@` as a separate token followed by an identifier. Both forms work.
- **View text tokenization**: A wide range of token types are treated as text content between view elements, including identifiers, punctuation, numbers, and most keywords.
