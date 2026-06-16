# OmniLang Grammar Specification

> **Version:** 1.0 (2026-06-16)
> **Source:** Derived from the Rust parser implementation (`omni-compiler/src/parser/`)
> **Status:** Authoritative reference

This document describes the OmniLang grammar as implemented by the Rust parser. It is
a practical reference for developers, not a formal language specification.

---

## 1. Lexical Structure

### 1.1 Comments

Two comment styles, identical to C/JavaScript:

```
// Single-line comment — runs to end of line

/* Block comment — can span
   multiple lines */
```

Block comments do not nest.

### 1.2 Identifiers

Identifiers start with a letter (`a-z`, `A-Z`), underscore (`_`), or any non-ASCII
non-whitespace character. Subsequent characters also include digits (`0-9`).

```
myVariable
_counter
_data2
你好
```

### 1.3 Keywords

These words are reserved and cannot be used as bare identifiers (but can appear
as contextual keywords in certain positions):

**OmniLang keywords:**

`fn` `const` `let` `mut` `if` `else` `while` `for` `in` `return` `break` `continue`
`print` `println` `as` `async` `await` `match` `struct` `class` `interface` `impl`
`extends` `implements` `type` `nominal` `static` `private` `pub` `override`
`import` `from` `export` `true` `false` `null`

**OmniView keywords:**

`component` `signal` `computed` `prop` `event` `ref` `store` `view` `style`
`action` `getter` `theme` `bind`

**At-keywords (view directives):**

`@store` `@if` `@else` `@for` `@empty` `@slot` `@native` `@fallback` `@error`
`@mount` `@unmount` `@effect`

### 1.4 Literals

#### Integer Literals

Decimal, hexadecimal, octal, and binary:

```
42
0xFF        // hex (prefix 0x or 0X)
0o77        // octal (prefix 0o or 0O)
0b1010      // binary (prefix 0b or 0B)
```

Internally stored as `i64`. Overflow during literal parsing silently wraps/truncates.

#### Float Literals

```
3.14
1.0
2.5e10
1e-3
1.5E+2
```

Internally stored as `f64`.

#### String Literals

Double-quoted strings with escape sequences:

```
"hello world"
"line1\nline2"
"tab\there"
"quote \"inside\""
"unicode: \u{1F600}"
```

Escape sequences: `\"` `\\` `\n` `\t` `\r` `\u{XXXX}`

#### Template Strings

Backtick-quoted strings with no interpolation (literal backtick strings):

```
`SELECT * FROM users WHERE id = ?`
```

#### Single-Quoted Strings

Used for view text content. Support `\'` and `\\` escapes. Stop at `<` or `>`
(view HTML boundaries):

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
.       Dot
...     Spread
?       Question mark
?.      Optional chaining
+       Plus
-       Minus
*       Star
/       Slash
%       Percent
+=      Plus assign
-=      Minus assign
*=      Star assign
/=      Slash assign
%=      Percent assign
=       Assign
==      Equal
!=      Not equal
<       Less than
>       Greater than
<=      Less or equal
>=      Greater or equal
&&      Logical AND
||      Logical OR
!       Logical NOT
->      Arrow (return type)
=>      Fat arrow (inline function)
|       Pipe
$       Dollar (const prefix)
#       Hash
( )     Parentheses
{ }     Braces
[ ]     Brackets
;       Semicolon
,       Comma
:       Colon
@       At (event/directive prefix)
```

---

## 2. Program Structure

A source file is a sequence of imports followed by declarations:

```
import*
declaration*
```

The parser processes tokens at the top level and dispatches to the appropriate
declaration parser for each keyword it encounters. Anything that is not a
recognized keyword at the top level produces a parse error.

### Top-level entry order

```
<import>*
<component> | <store> | <theme> | <fn> | <pub fn>
| <struct> | <pub struct>
| <type> | <pub type>
| <export>*
```

---

## 3. Declarations

### 3.1 Function Declaration

```
fn name(param: Type, ...) [-> ReturnType] {
    statement*
    [trailing_expr]
}
```

Functions are private by default. Use `pub fn` for public visibility.

**Example:**

```omni
fn greet(name: string) -> string {
    return "Hello, " + name + "!";
}

fn add(a: i32, b: i32) -> i32 {
    a + b  // trailing expression (implicit return)
}

pub fn public_function() {
    println "this is visible to other modules";
}
```

Parameters can omit type annotations:

```
fn log_message(msg) {
    print msg;
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

Fields are comma-separated (trailing comma allowed). The `nominal` keyword marks
the struct as having nominal (not structural) typing.

**Example:**

```omni
pub struct User {
    name: string,
    email: string,
    age: i32,
}

struct Point {
    x: f64,
    y: f64,
}

nominal struct Color {
    r: u8,
    g: u8,
    b: u8,
}
```

### 3.3 Type Alias

```
type Name = TypeAnnotation;
```

**Example:**

```omni
type UserMap = Map<string, User>;
type Callback = fn(string) -> bool;
type Optional<T> = T?;
```

### 3.4 Tagged Union

```
type Name = Variant1 {
    field: Type,
    ...
} | Variant2 | Variant3 {
    field: Type,
    ...
};
```

Variants can have fields (with braces) or be fieldless.

**Example:**

```omni
type Result = Ok { value: string } | Error { code: i32 } | Pending;
```

### 3.5 Component Declaration

```
component Name {
    // Props (input properties)
    prop name: Type [= default_expr];
    prop title: string;
    prop count: i32 = 0;

    // Signals (internal reactive state)
    signal name: Type = init_expr;
    signal count: i32 = 0;

    // Computed values
    computed name: Type = expr;
    // or function-style:
    computed name() -> Type {
        expr
    }

    // Events (output callbacks)
    event name: Type[?];
    event onClick: ClickEvent;

    // Refs (DOM element references)
    ref name: Type;
    ref divRef: Element;

    // Store references
    @store StoreName;
    // or inline store definition:
    store StoreName {
        signal ...;
        action ...;
        fn ...;
    }

    // Constants (stored as signals internally)
    const $prefixed_name = literal;
    const name = literal;

    // Functions (private helpers)
    fn helperName(params) -> Type {
        ...
    }

    // Actions (methods, can be async)
    action doSomething(params) -> Type {
        ...
    }

    // Lifecycle hooks
    @mount { ... }
    @unmount { ... }
    @effect(dep1, dep2) { ... }

    // View (UI template)
    view {
        ...
    }

    // Styles (scoped CSS)
    style className {
        property: value;
        property: value;
    }
}
```

`component`, `store`, and `theme` declarations are always public (no `pub`
keyword needed).

**Example:**

```omni
component Counter {
    prop initial: i32 = 0;
    signal count: i32 = initial;

    computed doubled() -> i32 {
        count * 2
    }

    action increment(amount: i32) {
        count = count + amount;
    }

    @mount {
        println "Counter mounted!";
    }

    view {
        <div>
            <p>Count: {count}</p>
            <p>Doubled: {doubled}</p>
            <button @onClick={increment(1)}>+1</button>
        </div>
    }

    style container {
        padding: 16px;
        background: #f0f0f0;
    }
}
```

### 3.6 Store Declaration

```
store Name {
    signal name: Type = init_expr;
    getter name: Type = expr;
    action name(params) [: Type] [async] {
        ...
    }
}
```

Stores are standalone reactive state containers.

**Example:**

```omni
store AppStore {
    signal user: User? = null;
    signal items: Array<string> = [];

    getter hasItems: bool = items.length > 0;

    action fetchUser(id: string) async {
        let response = api.getUser(id);
        user = response;
    }
}
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

Theme values are string key-value pairs within named categories. Values can be
string literals or identifiers.

**Example:**

```omni
theme AppTheme {
    colors {
        primary: "#333",
        secondary: "#666",
        background: "white",
    }
    spacing {
        small: "8px",
        medium: "16px",
        large: "24px",
    }
}
```

### 3.8 Action Declaration

Actions are methods inside components and stores. They support async:

```
action name(param: Type, ...) [: ReturnType] [async] {
    statement*
}
```

**Example:**

```omni
action save() async {
    let result = await api.post("/save", data);
    status = "saved";
}

action addItem(item: string) {
    items.push(item);
}
```

---

## 4. Statements

### 4.1 Variable Declaration

```
let [mut] name [: Type] [= expr];
```

Variables are immutable by default. Use `mut` to allow reassignment.

**Example:**

```omni
let x = 5;
let name: string = "Alice";
let mut counter = 0;
counter = counter + 1;
```

### 4.2 Assignment

```
target = expr;
target += expr;
target -= expr;
target *= expr;
target /= expr;
target %= expr;
```

Targets can be variable names, field access (`obj.field`), or index access
(`arr[index]`).

**Example:**

```omni
x = 10;
user.name = "Bob";
items[0] = "first";
count += 1;
```

### 4.3 If / Else If / Else

```
if expr {
    statement*
} [else if expr {
    statement*
}]* [else {
    statement*
}]
```

**Example:**

```omni
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
    statement*
}
```

**Example:**

```omni
while x > 0 {
    print x;
    x = x - 1;
}
```

### 4.5 For-In Loop

```
for var in iterable {
    statement*
}
```

**Example:**

```omni
for item in items {
    print item;
}

for key in object {
    if key == "name" {
        break;
    }
}
```

### 4.6 Return

```
return [expr];
```

**Example:**

```omni
return;
return result;
return a + b;
```

### 4.7 Break / Continue

```
break;
continue;
```

Valid inside loop bodies (`while`, `for`).

### 4.8 Print

```
print expr;
println expr;
```

Built-in output (no function call syntax — it is a statement).

**Example:**

```omni
print "hello";
println "hello with newline";
println 42;
```

### 4.9 Expression Statements

Any expression followed by an optional semicolon forms a statement.

```
expr;
```

**Example:**

```omni
doSomething();
counter + 1;
```

### 4.10 Block with Trailing Expression

Blocks implicitly return their last expression if it is not terminated by a
semicolon:

```omni
fn double(x: i32) -> i32 {
    let result = x * 2;
    result  // trailing expression — implicit return
}
```

---

## 5. Expressions

### 5.1 Precedence Table

| Level | Operators | Assoc | Description |
|-------|-----------|-------|-------------|
| 1 | `=` `+=` `-=` `*=` `/=` `%=` | Right | Assignment |
| 2 | `\|\|` | Left | Logical OR |
| 3 | `&&` | Left | Logical AND |
| 4 | `==` `!=` `<` `>` `<=` `>=` | Left | Comparison |
| 5 | `+` `-` | Left | Addition/Subtraction |
| 6 | `*` `/` `%` | Left | Multiplication/Division/Modulo |
| 7 | `!` `-` | Prefix | Unary NOT/Negate |
| 8 | `.` `?.` `()` `[]` | Postfix | Member access, call, index |

### 5.2 Literal Expressions

```
42          // integer
3.14        // float
"hello"     // string
`raw text`  // template string
true        // boolean
false       // boolean
null        // null
```

### 5.3 Identifier / Variable Reference

```
myVariable
```

### 5.4 Binary Operators

```omni
a + b      // addition
a - b      // subtraction
a * b      // multiplication
a / b      // division
a % b      // modulo
a == b     // equality
a != b     // inequality
a < b      // less than
a > b      // greater than
a <= b     // less or equal
a >= b     // greater or equal
a && b     // logical AND (short-circuit)
a || b     // logical OR (short-circuit)
```

### 5.5 Unary Operators

```omni
!condition    // logical NOT
-value        // numeric negation
```

### 5.6 Function Call

```omni
functionName(arg1, arg2)
object.method(arg1, arg2)
```

**Example:**

```omni
greet("Alice")
math.max(10, 20)
user.getName()
items.push("new item")
```

### 5.7 Field Access

```omni
object.field
```

Supports optional chaining with `?.`:

```omni
user?.name     // returns null if user is null instead of error
```

### 5.8 Index Access

```omni
array[index]
map[key]
```

**Example:**

```omni
items[0]
matrix[row][col]
data["name"]
```

### 5.9 If Expression

```
if condition { then_expr } else { else_expr }
```

**Example:**

```omni
let status = if x > 0 { "positive" } else { "non-positive" };
```

### 5.10 Ternary Expression

(Defined in AST but not yet parsed — reserved for future use.)

### 5.11 Closure / Lambda

```
|param1, param2| body_expr
```

**Example:**

```omni
let double = |x| x * 2;
let sum = |a, b| a + b;
items.filter(|x| x > 0)
```

### 5.12 Array Literal

```
[expr, expr, ...]
```

**Example:**

```omni
let empty: Array<i32> = [];
let numbers = [1, 2, 3, 4];
let matrix = [[1, 0], [0, 1]];
```

### 5.13 Map Literal

```
{ key: value, key: value, ... }
```

**Example:**

```omni
let person = {
    name: "Alice",
    age: 30,
};
let config = {
    debug: true,
    port: 8080,
};
```

### 5.14 Template String

(Defined in AST but parsing produces string literals directly — backtick strings
are raw text with no interpolation.)

### 5.15 Parenthesized Expressions

```
(expr)
```

Used for grouping and precedence:

```omni
let result = (a + b) * c;
```

### 5.16 Method Call with Chaining

```
object.method(args)
object?.method(args)   // optional chaining
```

---

## 6. View Syntax

### 6.1 Element

Standard HTML-like elements:

```
<Tag prop="value" prop={expr}> children </Tag>
<Tag prop="value" />
```

**Example:**

```omni
view {
    <div class: container>
        <h1>Welcome</h1>
        <p>This is a paragraph.</p>
        <img src={imageUrl} />
    </div>
}
```

### 6.2 Attributes

#### Static Attributes

```
name="string value"
name={expression}
```

**Example:**

```omni
<input type="text" placeholder="Enter name" />
<input value={userName} />
```

#### Boolean Attributes

Attribute name only — sets value to `true`:

```omni
<input disabled />
<button hidden>Click</button>
```

#### Hyphenated Attribute Names

Hyphenated names like `margin-left` are supported:

```omni
<div data-testid="main" aria-label="Main content">
```

#### Class Attribute

Use `class:` with a style reference or string:

```omni
<div class: container>
<div class: "highlight selected">
<div class: {isActive ? "active" : "inactive"}>
```

#### Style Attribute

Inline styles use `style: { prop: value, ... }`:

```omni
<div style: {
    color: "red",
    fontSize: "16px",
    marginTop: "8px",
}>
```

### 6.3 Event Bindings

Two syntaxes for event handlers:

```
@eventName={handler}
onEventName={handler}
```

The `on`-prefixed form requires the third character to be uppercase
(e.g. `onClick`, `onChange`, `onPress`).

**Handler values:**

```
// Function reference
@onClick={handleClick}

// Inline function
@onChange={(event) => {
    value = event.target.value;
}}

// Store action
@onSubmit={store.save}

// Function call expression
@onClick={handleClick(args)}
```

**Example:**

```omni
<button @onClick={increment}>+</button>
<input onInput={(e) => { text = e.value; }} />
<form @onSubmit={appStore.saveData}>...</form>
```

### 6.4 Bind Directive (Two-Way Binding)

```
bind:property={signalPath}
```

**Example:**

```omni
<input bind:value={name} />
<input bind:value={store.field} />
```

### 6.5 Directives

#### @if / @else / @else-if

```
@if condition {
    ...
} @else if condition {
    ...
} @else {
    ...
}
```

**Example:**

```omni
@if isLoggedIn {
    <p>Welcome back, {user.name}!</p>
} @else {
    <button @onClick={login}>Sign In</button>
}
```

#### @for / @empty

```
@for item in iterable {
    ...
} @empty {
    ...
}
```

The `@empty` block renders when the iterable is empty.

**Example:**

```omni
<ul>
    @for item in items {
        <li>{item.name}</li>
    } @empty {
        <li class: empty>No items found</li>
    }
</ul>
```

#### @slot

```
@slot          // default slot
@slot(name)    // named slot
```

**Example:**

```omni
component Card {
    view {
        <div class: card>
            <h2>{title}</h2>
            @slot          <!-- content slot -->
            @slot(footer)  <!-- named footer slot -->
        </div>
    }
}
```

### 6.6 Interpolation

```
{expression}
```

**Example:**

```omni
<h1>{pageTitle}</h1>
<p>{user.name} has {itemCount} items</p>
<div class: {isActive ? "active" : ""}>
```

### 6.7 Text Content

Plain text between elements is treated as text nodes. Consecutive text-like
tokens (identifiers, numbers, punctuation, etc.) are coalesced into a single
text node:

```omni
<p>Hello, {name}! You have {count} messages.</p>
```

Double-quoted strings can also appear as text:

```omni
<button>"Click me"</button>
```

### 6.8 Component Call Syntax

```
Name(prop: value, prop: value) {
    children
}
Name(prop: value)   // self-closing (no children)
```

Lowercase names are HTML elements; uppercase names are component references.
A component call starts with an identifier followed by `(` or `{`.

**Example:**

```omni
view {
    <div>
        <Header title="My App" />
        <UserCard(user: currentUser) {
            <p>Extra content</p>
        }>
        <Footer />
    </div>
}
```

### 6.9 Style Rules

```
style className {
    property: value;
    property: value;
}
```

Style values support:
- Numbers (`16`, `1.5`)
- Dimension strings parsed from identifiers (`16px`, `1.5rem`, `100vh`, `50%`)
- String literals
- Hex colors from identifiers (`#ff0000`)
- Plain identifiers (`center`, `flex`, `bold`)

**Example:**

```omni
style card {
    padding: 16px;
    background: "#ffffff";
    border-radius: 8px;
    display: flex;
    gap: 8px;
}

style heading {
    font-size: 24px;
    font-weight: bold;
    color: "#333";
}
```

---

## 7. Import / Export

### 7.1 Named Import

```
import { Name1, Name2 } from "./path.omni";
```

Trailing commas allowed:

```omni
import { Component1, Component2, } from "./ui.omni";
```

### 7.2 Namespace Import

```
import "./path.omni" as alias;
```

**Example:**

```omni
import "./math.omni" as math;
let result = math.add(1, 2);
```

### 7.3 Type-Only Import

```
import type { Type1, Type2 } from "./path.omni";
```

**Example:**

```omni
import type { User, Config } from "./types.omni";
```

### 7.4 Library Import

```
import name;
```

Used for built-in libraries like `omniview`:

```omni
import omniview;
```

### 7.5 Re-export

```
export { Name1, Name2 } from "./path.omni";
```

**Example:**

```omni
export { Button, Card } from "./ui.omni";
```

---

## 8. Visibility

### 8.1 Explicit Visibility

`pub` makes a declaration publicly visible across modules. It can prefix:

```omni
pub fn name(...) { ... }
pub struct Name { ... }
pub type Name = ...;
```

### 8.2 Implicit Visibility

Without `pub`, function, struct, and type alias declarations default to **private**
(visible only within the current module).

### 8.3 Auto-Public Declarations

`component`, `store`, and `theme` declarations are **always public** —
they do not accept (and do not need) a `pub` modifier:

```omni
component MyComponent { ... }     // always public
store MyStore { ... }               // always public
theme MyTheme { ... }               // always public
```

---

## Appendix A: Type Annotations

Type annotations appear after a colon (`:`) in variable declarations, function
parameters, return types, struct fields, and component declarations.

### Named Types

```
string
i32
f64
bool
User
Array<string>
Map<string, User>
```

### Generic Type Arguments

```
Array<string>
Map<string, i32>
Optional<T>
```

Parsed with `<` and `>` delimiters:

```
let items: Array<string> = [];
```

### Optional Types

A `?` suffix marks a type as optional (nullable):

```
string?
User?
Array<string>?
```

### Function Types (AST-defined, not yet parsed)

Reserved for future use:

```
fn(string) -> bool
```

---

## Appendix B: Semantic Nuances

- **Semicolons are optional** in most positions throughout the grammar.
- **Trailing commas** are allowed in import lists, function arguments, struct
  fields, and array/map literals.
- The parser treats many OmniLang keywords as **contextual identifiers** in
  positions where an identifier is expected (e.g., parameter names can be `type`,
  `style`, `event`, etc.).
- **Blocks with a trailing expression** that is not semicolon-terminated
  implicitly return that expression (Rust-style block expressions).
- The `@` prefix on event names can attach directly to the identifier (e.g.,
  `@onClick` lexes as a single `Ident("@onClick")` token) or be a separate token
  followed by an identifier (`@ onClick`).
- The `bind:` prefix on attribute names activates two-way signal binding.
- View text content tokenizes flexibly: identifiers, numbers, punctuation, and
  many keyword tokens are valid text between elements.
