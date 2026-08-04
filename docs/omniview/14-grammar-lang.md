# OmniLang Grammar — Core Language

> **Target audience**: AI agents implementing parsers, tool builders, language server authors.
> **Source**: Extracted from `crates/omnilang-core/src/parser/` and `src/lexer/`.
> **Convention**: Hybrid BNF — productions for structure, prose for disambiguation, code examples throughout.

---

## 1. Lexical Structure

### 1.1 Token Types

The lexer produces a flat `Vec<Token>` from UTF-8 source bytes. All tokens carry a `Span` (byte offset + line/col).

#### Literals

| Token | Lexeme Pattern | Example |
|-------|---------------|---------|
| `IntLiteral(u64)` | `[0-9]+` \| `0x[0-9a-fA-F]+` | `42`, `0xFF` |
| `FloatLiteral(f64)` | `[0-9]+\.[0-9]+` | `3.14` |
| `StringLiteral(String)` | `"..."` with `\n` `\t` `\\` `\"` escapes | `"hello\n"` |
| `TemplateString(String)` | `` `...` `` with `${expr}` interpolation | `` `Hello ${name}` `` |
| `True` / `False` | keyword literals | `true`, `false` |
| `Null` | keyword literal | `null` |

#### Identifiers

| Token | Pattern | Example |
|-------|---------|---------|
| `Ident(String)` | `[a-zA-Z_][a-zA-Z0-9_]*` | `counter`, `UserProfile` |

#### Operators & Punctuation

```
+   -   *   /   %   .   ..   ...   ?   ??   ?.
+=  -=  *=  /=  %=  =   ==  !=   <   >   <=  >=
&&  ||  !   ->  =>  |   $    #    (   )   {
}   [   ]   ;   ,   :   @
```

#### Language Keywords (omni-lang)

```
fn  const  let  mut  if  else  while  for  in  return
break  continue  print  println  async  await  as  is
throw  try  catch  finally  match  struct  class  interface
impl  extends  implements  type  nominal  static  private
pub  override  import  from  export
```

#### UI Keywords (omni-view)

```
emit  component  signal  computed  prop  event  ref
store  view  style  action  getter  theme  design  bind
```

#### At-Directive Keywords (omni-view)

```
@store  @if  @else  @for  @empty  @slot
@mount  @unmount  @effect
@motion  @enter  @hover  @press  @focus
@in-view  @scroll  @drag-horizontal  @drag-vertical
```

Trigger spellings are **kebab-case** (semantic-level DSL, ADR-053): `@in-view`,
`@drag-horizontal`, `@drag-vertical`. The pre-DSL camelCase spellings
(`@inView`, `@dragHorizontal`), the removed triggers (`@leave`, `@unhover`,
`@animate`), and the removed modifier tokens (`@drivenBy`, `@paused`,
`@hoverReverse`, `@inViewOnce`, `@scrollStart`, `@dragConstraints` etc.) are
**not** produced by the lexer. `@hero` still lexes to `AtHero` but the parser
rejects it with a clear diagnostic (hero transitions removed per ADR-032/053).

#### Special

| Token | Description |
|-------|-------------|
| `Eof` | End of token stream |
| `Error(String)` | Lexer error, stops tokenization |

### 1.2 Whitespace & Comments

```
// Single-line comment — extends to end of line
/* Multi-line
   comment — nestable? NO, first */ closes */
```

Whitespace (spaces, tabs, `\r`, `\n`) is skipped between tokens. No significant whitespace.

### 1.3 Contextual Keywords

Certain keyword tokens act as **identifiers** in expression/parameter/field-name context. The parser's `keyword_as_ident_name()` function maps these:

```
// ALWAYS usable as identifiers in expression context:
event signal computed prop ref store action getter view style theme
component bind emit
fn const let mut if else while for in return async await is match
struct class interface impl extends implements type nominal static
private override import from export print println
true false null break continue as pub if_mode
```

This means you can write `let signal = 42;` or `obj.component.name` — the parser resolves
`TokenKind::Signal` → `Ident("signal")` when the token appears where an identifier is expected.

### 1.4 Input Limits

- Maximum source size: **64 MB** (`MAX_INPUT_SIZE`)
- Maximum parser recursion depth: **256** (`MAX_RECURSION_DEPTH`)

---

## 2. Type System

### 2.1 Type Annotation Grammar

```
type_annotation ::= named_type ('|' named_type)*
                  | fn_type
                  | '[' type_annotation ']'  // array sugar → Array<T>

named_type ::= <ident> ('<' type_annotation (',' type_annotation)* '>')? '?'?

fn_type ::= 'fn' '(' (type_annotation (',' type_annotation)*)? ')' '->' type_annotation
```

**Disambiguation**: `'[' type_annotation ']'` is syntactic sugar that desugars to `Array<type_annotation>`.

**Optional type**: Trailing `?` wraps the type in `Optional<T>`. Applies to `named_type` only.

**Union type**: `A | B | C` produces `TypeAnnotation::Union`. The `|` operator is right-associative within type position.

**Fn type**: `fn(i32, String) -> bool` produces `TypeAnnotation::Fn { params: [i32, String], return_type: bool }`.

Examples:
```
i32                                       // Named, no args
String?                                    // Optional<String>
Array<String>                              // Named with type arg
Map<String, i32>                           // Named with multiple type args
i32 | String                               // Union
fn(i32) -> bool                            // Fn type
(i32 | String)?                            // Optional union
```

### 2.2 AST Representation

```
TypeAnnotation ::=
  | Named { name: String, type_args: Vec<TypeAnnotation> }
  | Optional { base: Box<TypeAnnotation> }
  | Union { types: Vec<TypeAnnotation> }
  | Fn { params: Vec<TypeAnnotation>, return_type: Box<TypeAnnotation> }
```

---

## 3. Expressions

### 3.1 Pratt Precedence Table

The expression parser uses Pratt parsing (top-down operator precedence).
Binding power: `(left_bp, right_bp)`. Higher = tighter binding. Right-associative operators have `left_bp > right_bp`.

| Precedence | Operators | Assoc | Binding Power |
|-----------|-----------|-------|---------------|
| 1 (lowest) | `=` `+=` `-=` `*=` `/=` `%=` | right | (1, 2) |
| 2 | `??` (nullish coalesce) | left | (4, 5) |
| 3 | `\|\|` | left | (3, 4) |
| 4 | `&&` | left | (5, 6) |
| 5 | `is` | — | (6, 7) |
| 6 | `==` `!=` `<` `>` `<=` `>=` | left | (7, 8) |
| 7 | `+` `-` | left | (9, 10) |
| 8 | `*` `/` `%` | left | (11, 12) |
| 9 | `!` `-` (unary) | prefix | (7, —) |
| 10 | `await` | prefix | (0, —) |
| 11 | `? :` (ternary) | — | (1, 2) |
| 12 (highest) | `.field` `.method()` `[idx]` `(args)` `?.field` | postfix | (13, 14) |

**Key rule**: Binding power 0 for `await` operand means `await a + b` ≡ `await (a + b)` — the lowest-precedence parse.

### 3.2 Primary Expressions (Prefix Parselets)

```
primary ::= literal
          | <ident>
          | <ident> '(' (expr (',' expr)* ','?)? ')'   // function call
          | '(' expr ')'
          | '[' (expr (',' expr)* ','?)? ']'            // array literal
          | '{' (map_entry (',' map_entry)* ','?)? '}'  // map literal
          | '{' block_body '}'                          // block expression
          | '|' param (',' param)* '|' expr             // closure
          | 'if' expr '{' expr '}' 'else' ('{' expr '}' | 'if' ...)
          | 'match' expr '{' match_arm (',' match_arm)* ','? '}'
          | '-' expr                                    // unary negate
          | '!' expr                                    // unary not
          | 'await' expr
          | '...' expr                                  // spread
          | <template_string>

literal ::= <int_literal> | <float_literal> | <string_literal>
          | 'true' | 'false' | 'null'

map_entry ::= <ident> ':' expr          // explicit key: value
            | <ident>                    // shorthand: { key } → { key: key }

// struct literal suffix (handled AFTER Pratt, not as infix):
// Triggered when VarRef starts with uppercase AND followed by '{'
struct_literal ::= <ident> '{' (field_name ':' expr (',' field_name ':' expr)* ','?)? '}'
```

**Disambiguation — `{ }` (map literal vs block expression)**:

When the parser encounters `{`, it peeks at the next token:
- If the next token is a **statement-starting keyword** (`let`, `if`, `while`, `for`, `return`, `break`, `continue`, `print`, `println`, `throw`, `try`, `emit`, `;`, `{`), parse as a **block expression**.
- Otherwise, parse as a **map literal**.

```
// Map literal (next token is Ident, not a statement keyword):
let m = { a: 1, b: 2 }

// Block expression (next token is 'let', a statement keyword):
let result = { let x = 1; x + 2 }
```

**Disambiguation — Struct literal suffix**:
After Pratt parsing, if the result is a `VarRef` whose name starts with **uppercase** AND the next token is `{`, parse as struct literal:
```
User { name: "Alice", age: 30 }
// NOT a block (name starts uppercase + followed by {)
```

### 3.3 Infix / Postfix Expressions

```
// Binary operators (handled by Pratt infix parselets)
binary ::= expr op expr    // where op ∈ precedence levels 1-8

// Compound assignment (right-assoc, bp 1)
compound_assign ::= expr ('+=' | '-=' | '*=' | '/=' | '%=') expr

// Assignment (right-assoc, bp 1)
assign ::= expr '=' expr

// Ternary (conditional) — triggered by '?' after expression
ternary ::= expr '?' expr ':' expr

// Type check (bp 6)
is_check ::= expr 'is' <ident>

// Nullish coalesce (bp 4)
nullish ::= expr '??' expr

// Field access (bp 13)
field_access ::= expr '.' <ident>

// Optional chain (bp 13)
optional_chain ::= expr '?.' <ident>

// Method call (bp 13)
method_call ::= expr '.' <ident> '(' (expr (',' expr)* ','?)? ')'

// Function call on arbitrary expression (bp 13)
call ::= expr '(' (expr (',' expr)* ','?)? ')'

// Index access (bp 13)
index ::= expr '[' expr ']'
```

### 3.4 Template Strings

```
template_string ::= '`' template_content '`'

template_content ::= (literal_text | '${' expr '}')*
```

Template strings are parsed in two phases:
1. **Lexer**: produces `TokenKind::TemplateString(raw_string)` — the raw content between backticks
2. **Parser**: `parse_template_parts()` splits the raw string into `TemplatePart::Literal` and `TemplatePart::Expr` segments

Inside `${}`, only **simple expressions** are supported: identifiers, field access (`.`), method calls (`.method()`), and function calls (`()`). No operators, no blocks.

```
`Hello ${name}`
`User: ${user.name}`
`Count: ${items.length}`
`Result: ${getValue()}`
```

### 3.5 Match Expression

```
match_expr ::= 'match' expr '{' match_arm (',' match_arm)* ','? '}'

match_arm ::= match_pattern '=>' match_body

match_pattern ::= '_'                                          // wildcard
                | <ident>                                      // variant (no bindings)
                | <ident> '{' <ident> (',' <ident>)* ','? '}'  // variant with bindings
                | <int_literal> | <string_literal>             // literal pattern
                | 'true' | 'false'

match_body ::= '{' block_body '}'                              // block body
             | expr                                            // single expression body
```

Examples:
```
match result {
    Ok { value } => value,
    Err { message } => { print message; null },
    _ => null,
}
```

### 3.6 Closure Expression

```
closure ::= '|' <ident> (',' <ident>)* '|' expr
```

The body is a single expression (no block — use `{ }` block expression for multi-statement):
```
|x| x + 1
|a, b| a + b
|items| { let sum = 0; for item in items { sum += item }; sum }
```

### 3.7 If-Expression (Expression-Level)

```
if_expr ::= 'if' expr '{' expr '}' 'else' ('{' expr '}' | if_expr)
```

Different from statement `if` — both branches are single expressions (no statements):
```
let result = if x > 0 { "positive" } else { "non-positive" }
let grade = if score >= 90 { "A" } else if score >= 80 { "B" } else { "C" }
```

### 3.8 Expression AST (Complete)

```
Expr ::=
  | Literal(Literal, Span)
  | VarRef(String, Span)
  | BinaryOp { left: Box<Expr>, op: BinaryOpKind, right: Box<Expr> }
  | UnaryOp { op: UnaryOpKind, operand: Box<Expr> }
  | FuncCall { func: Box<Expr>, args: Vec<Expr> }
  | MethodCall { object: Box<Expr>, method: String, args: Vec<Expr> }
  | IndexGet { object: Box<Expr>, index: Box<Expr> }
  | FieldAccess { object: Box<Expr>, field: String }
  | OptionalChain { object: Box<Expr>, field: String }
  | ArrayLiteral { elements: Vec<Expr> }
  | MapLiteral { entries: Vec<(Expr, Expr)> }
  | IfExpr { condition: Box<Expr>, then_expr: Box<Expr>, else_expr: Box<Expr> }
  | TernaryExpr { condition: Box<Expr>, then_expr: Box<Expr>, else_expr: Box<Expr> }
  | TemplateString { parts: Vec<TemplatePart> }
  | StructLiteral { type_name: String, fields: Vec<(String, Expr)> }
  | StoreFieldAccess { store_alias: String, field_name: String }
  | Block { body: Block }
  | ClosureExpr { params: Vec<String>, body: Box<Expr> }
  | AwaitExpr { operand: Box<Expr> }
  | MatchExpr { matched: Box<Expr>, arms: Vec<MatchArm> }
  | SpreadExpr { operand: Box<Expr> }
  | IsCheck { value: Box<Expr>, type_name: String }
  | NotYetImplemented { feature: String }
  | Error { message: String }

Literal ::= Int(u64) | Float(f64) | String(String) | Bool(bool) | Null

BinaryOpKind ::= Add | Sub | Mul | Div | Mod
               | Eq | Neq | Lt | Gt | LtEq | GtEq
               | And | Or | NullishCoalesce
               | Assign | AddAssign | SubAssign | MulAssign | DivAssign | ModAssign

UnaryOpKind ::= Not | Neg | Await

TemplatePart ::= Literal(String) | Expr(Expr)
```

---

## 4. Statements

### 4.1 Statement Grammar

```
stmt ::= var_decl
       | if_stmt
       | while_stmt
       | for_stmt
       | return_stmt
       | break_stmt
       | continue_stmt
       | print_stmt
       | throw_stmt
       | try_catch_stmt
       | emit_stmt
       | block_stmt
       | expr_stmt
       | empty_stmt
```

### 4.2 Variable Declaration

```
var_decl ::= 'let' 'mut'? <ident> (':' type_annotation)? ('=' expr)? ';'?
```

- Semicolon is **optional** (consumed if present)
- `mut` makes the variable mutable
- Type annotation and initializer are independent (both optional)

```
let x = 5;
let mut count = 0;
let name: String;
let mut items: Array<String> = [];
```

### 4.3 If Statement

```
if_stmt ::= 'if' expr block ('else' 'if' expr block)* ('else' block)?
```

The condition does NOT require parentheses:
```
if x > 0 { print "positive" }
if a { doA() } else if b { doB() } else { doC() }
```

### 4.4 While Loop

```
while_stmt ::= 'while' expr block
```

Condition does NOT require parentheses:
```
while count < 10 { count += 1 }
```

### 4.5 For-In Loop

```
for_stmt ::= 'for' <ident> 'in' expr block
```

Only `for-in` is supported (no C-style `for(;;)`, no `for-of`):
```
for item in items { print item }
for i in range(0, 10) { print i }
```

### 4.6 Return / Break / Continue

```
return_stmt   ::= 'return' expr? ';'?
break_stmt    ::= 'break' ';'?
continue_stmt ::= 'continue' ';'?
```

- `return` with no value = void return (produces `Return { value: None }`)
- Semicolons are optional

### 4.7 Print

```
print_stmt ::= ('print' | 'println') expr ';'?
```

- `print` → no trailing newline (`Print { newline: false }`)
- `println` → trailing newline (`Print { newline: true }`)

```
print "Hello"
println "World"
```

### 4.8 Throw / Try-Catch

```
throw_stmt     ::= 'throw' expr ';'?
try_catch_stmt ::= 'try' block ('catch' <ident>? block)? ('finally' block)?
```

At least one of `catch` or `finally` must be present. The catch variable is optional:
```
try { riskyOp() } catch e { print e }
try { riskyOp() } catch { print "failed" }
try { step1() } finally { cleanup() }
try { step1() } catch e { print e } finally { cleanup() }
```

### 4.9 Emit (Event Dispatch)

```
emit_stmt ::= 'emit' <ident> '(' (expr (',' expr)*)? ')' ';'?
```

```
emit done()
emit update(x, y)
```

### 4.10 Block Statement

```
block_stmt ::= '{' (stmt)* (expr ';'?)? '}'
block      ::= '{' (stmt)* (expr ';'?)? '}'    // same — produces Block
```

A block may end with a **trailing expression** (no `;`). If the last item in a block is an `ExprStmt` immediately before `}`, it becomes the `trailing_expr`:
```
{
    let x = 1;
    x + 2          // trailing_expr — value of the block
}
```

**Empty statement**: A lone `;` produces `ExprStmt { expr: Literal(Null) }`.

### 4.11 Expression Statement

```
expr_stmt ::= expr ';'?
```

Fallback when no keyword matches — parses an expression, optionally consumes `;`:
```
counter += 1;
doSomething();
42             // also valid — semicolon is optional
```

### 4.12 Statement AST (Complete)

```
Stmt ::=
  | VarDecl { name: String, type_ann: Option<TypeAnnotation>,
              mutable: bool, init: Option<Expr> }
  | If { condition: Expr, then_body: Block,
         else_ifs: Vec<(Expr, Block)>, else_body: Option<Block> }
  | While { condition: Expr, body: Block }
  | ForIn { var_name: String, iterable: Expr, body: Block }
  | Return { value: Option<Expr> }
  | Break
  | Continue
  | Print { newline: bool, arg: Expr }
  | ExprStmt { expr: Expr }
  | EventEmit { name: String, args: Vec<Expr> }
  | Throw { expr: Expr }
  | TryCatch { try_body: Block, catch_var: Option<String>,
               catch_body: Option<Block>, finally_body: Option<Block> }
  | Error { message: String }

AssignTarget ::= Name(String) | Index { object: String, index: Box<Expr> }
               | Field { object: String, field: String }

Block ::= { stmts: Vec<Stmt>, trailing_expr: Option<Box<Expr>> }
```

---

## 5. Top-Level Declarations

### 5.1 Module Structure

```
module ::= (top_level_decl)* <eof>

top_level_decl ::= function_def
                 | struct_def
                 | tagged_union_def
                 | type_alias
                 | const_decl
                 | import_stmt
                 | export_stmt
                 // omni-view extensions (see grammar-view.md):
                 | component_def | store_def | theme_def | design_def
                 | motion_def
```

### 5.2 Function Definition

```
function_def ::= 'pub'? 'async'? 'fn' <ident>
                 '(' (param (',' param)*)? ')'
                 ('->' type_annotation)?
                 block

param ::= <ident> (':' type_annotation)?
```

- `pub` marks public visibility (default: private)
- `async` marks async function
- Parameters may have optional type annotations

```
fn greet() { print("Hello"); }
pub fn add(a: i32, b: i32) -> i32 { return a + b; }
pub async fn fetch(url: String) -> String { return url; }
```

### 5.3 Struct Definition

```
struct_def ::= 'pub'? 'struct' <ident> '{' (struct_field (',' struct_field)* ','?)? '}'

struct_field ::= <ident> ':' type_annotation
```

A `nominal` struct (extending `as` semantics) is NOT in the BNF — the `nominal` keyword
is parsed but the feature is reserved for future use.

```
struct Point { x: i32, y: i32 }
pub struct User {
    name: String,
    age: i32,
}
```

### 5.4 Tagged Union (Enum)

```
tagged_union_def ::= 'type' <ident> '=' variant ('|' variant)*

variant ::= <ident> ('{' (struct_field (',' struct_field)* ','?)? '}')?
```

**Disambiguation** (type alias vs tagged union): The parser checks the token after `=`.
If it's `Ident {` or `Ident |`, parse as tagged union. Otherwise parse as type alias.

```
type Color = Red | Green | Blue
type Result = Ok { value: Data } | Err { message: String }
type Option = Some { value: i32 } | None
```

### 5.5 Type Alias

```
type_alias ::= 'pub'? 'type' <ident> '=' type_annotation ';'?
```

```
type UserId = i32;
type Callback = fn(Event) -> void;
pub type NameMap = Map<String, User>;
```

### 5.6 Constant Declaration

```
const_decl ::= 'const' ('$'? <ident> | '$' <ident>) '=' (expr | array_literal) ';'?
```

Module-level constants. Values are stored in the AST (`Module.consts`)
and resolved at compile time for use in design token values and imports.
Constants are implicitly public within the module and can be imported
across files via the standard `import { NAME } from "./file.omni"` syntax.

Example:
```omni
const BRAND = "#6366f1";
const SPACING = "16px";

design App {
    token primary: color = BRAND;    // compile-time resolution
}
```
```
const MAX_SIZE = 100;
const $COLORS = ["red", "green", "blue"];
```

### 5.7 Import / Export — see §6

### 5.8 Top-Level AST Types

```
FunctionDef ::= {
    visibility: Visibility,
    name: String,
    params: Vec<Param>,
    return_type: Option<TypeAnnotation>,
    is_async: bool,
    body: Block,
}

StructDef ::= {
    visibility: Visibility,
    name: String,
    is_nominal: bool,
    fields: Vec<StructField>,
}

StructField ::= { name: String, type_ann: TypeAnnotation }

TaggedUnionDef ::= {
    name: String,
    variants: Vec<Variant>,
}

Variant ::= { name: String, fields: Vec<StructField> }

TypeAlias ::= {
    visibility: Visibility,
    name: String,
    type_ann: TypeAnnotation,
}

Visibility ::= Public | Private
```

---

## 6. Module System

### 6.1 Import Statements

```
import_stmt ::= named_import | namespace_import | type_import | lib_import

named_import     ::= 'import' '{' <ident> (',' <ident>)* ','? '}' 'from' <string_literal> ';'?
namespace_import ::= 'import' <string_literal> 'as' <ident> ';'?
type_import      ::= 'import' 'type' '{' <ident> (',' <ident>)* ','? '}' 'from' <string_literal> ';'?
lib_import       ::= 'import' 'omniview' ';'?
```

Examples:
```
import { add, sub } from "./utils/math.omni";
import "./config.omni" as config;
import type { User, Post } from "./models.omni";
import omniview;
```

### 6.2 Export Statements

```
export_stmt ::= 'export' '{' <ident> (',' <ident>)* ','? '}' 'from' <string_literal> ';'?
```

Re-exports named items from another module:
```
export { main, helper } from "./utils/math.omni";
```

### 6.3 Import AST Types

```
ImportStmt ::=
  | Named { names: Vec<String>, path: String }
  | Namespace { path: String, alias: String }
  | TypeOnly { names: Vec<String>, path: String }
  | Lib(String)  // "omniview"

ExportFrom ::= { names: Vec<String>, path: String }
```

---

## 7. Error Recovery

The parser uses **error recovery** to continue parsing after syntax errors:

### 7.1 Statement-Level Recovery

When a statement parse fails, `recover_from_error()` skips to the next **statement boundary** (`;`, `}`, EOF) or **statement-start token** (`let`, `if`, `while`, `for`, `return`, `break`, `continue`, `print`, `println`, `throw`, `try`, `emit`, or expression starters like `Ident`, literals, `(`, `-`, `!`, `await`, `match`, `mut`).

```
// This file has 1 syntax error but produces valid AST for both functions:
pub fn foo() { return 1; } quux pub fn bar() { return 2; }
//                         ^^^^ skipped, both functions parsed
```

### 7.2 Top-Level Recovery

`skip_to_top_level()` advances to the next top-level keyword (`fn`, `pub`, `component`, `store`, `theme`, `design`, `struct`, `type`, `const`, `import`, `export`, `@motion`, `async`).

---

## Appendix A: Full Operator Precedence Table

| BP (L,R) | Operators | Associativity |
|----------|-----------|---------------|
| (1, 2) | `=` `+=` `-=` `*=` `/=` `%=` | right |
| (3, 4) | `\|\|` | left |
| (4, 5) | `??` | left |
| (5, 6) | `&&` | left |
| (6, 7) | `is` | — |
| (7, 8) | `==` `!=` `<` `>` `<=` `>=` | left |
| (9, 10) | `+` `-` | left |
| (11, 12) | `*` `/` `%` | left |
| prefix 7 | `-` `!` (unary) | — |
| prefix 0 | `await` | — |
| (13, 14) | `.` `?.` `()` `[]` (postfix) | left |

## Appendix B: Complete Keyword List

### Reserved (always keywords, never identifiers)
```
true  false  null  import  from  export  pub  as  in  else
catch  finally  try  throw  while  for  return  break  continue
print  println  async  await  match  if  let  mut  fn  struct
type  const  is  emit  component  store  theme  design  bind
signal  computed  prop  event  ref  view  style  action  getter
```

### Contextual (usable as identifiers in expression/name context)
All reserved keywords above PLUS `if_mode` (also lexed from the `@if-mode`
spelling) — `keyword_as_ident_name()` is the single mapping.

### At-directive tokens (lexed keywords, NOT identifiers)
These lexer tokens are directives or element attributes; unlike the reserved
keywords above they are **not** mapped to identifiers:
```
@store  @if  @else  @for  @empty  @slot
@mount  @unmount  @effect
@motion  @enter  @hover  @press  @focus
@in-view  @scroll  @drag-horizontal  @drag-vertical
@hero
```

Notes:
- `@hero` is lexed but rejected at parse time with a diagnostic (hero transitions removed per ADR-032/053).
- The pre-DSL modifier tokens (`@drivenBy`, `@paused`, `@hoverReverse`, `@inViewOnce`/`@inViewRepeat`/`@inViewThreshold`, `@scrollStart`/`@scrollEnd`/`@scrollScrub`, `@dragConstraints`/`@dragElastic`/`@dragScrub`/`@dragMomentum`/`@dragThreshold`/`@dragVelocityMax`) were removed with the semantic-level DSL (ADR-053).

## Appendix C: Semicolon Rules

| Context | Semicolon |
|---------|-----------|
| Expression statement | Optional (consumed if present) |
| Variable declaration | Optional |
| Return statement | Optional |
| Break / Continue | Optional |
| Print statement | Optional |
| Throw statement | Optional |
| Emit statement | Optional |
| Import / Export | Optional |
| Type alias | Optional |
| Const declaration | Optional (at top level) |
| After `}` | Not needed |

The parser calls `self.advance_if(&TokenKind::Semicolon)` — semicolons are **always optional**
in statement position.
