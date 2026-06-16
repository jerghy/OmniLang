# Omni Rust Compiler Design Document

> **Status:** Draft — awaiting review
> **Date:** 2026-06-15
> **Author:** jerghy + Claude

## 1. Overview

### 1.1 Problem

The OmniLang visual editor currently has **no usable code generation loop**:

1. Python reverse codegen (`omniview/codegen/source.py`) exists but converts visual state → `.omni` source only, no execution
2. Forward compilation (`.omni` → React/Vue/Flutter) is Python-only, requires HTTP round-trips to `/api/compile`
3. The debug bridge (WebSocket + `omni-runtime.ts`) exists but has no source map, no real-time sync, no breakpoints
4. Python parser depends on Lark (Earley parser), which has no Rust equivalent and cannot run in WASM

### 1.2 Goal

Build a **Rust compiler (`omni-compiler`)** that:

1. Parses `.omni` source → AST → Intent IR
2. Outputs IR as JSON (consumed by Python codegen for React/Vue/Flutter/Dioxus)
3. Outputs OmniView native JS + HTML + CSS + SourceMap (consumed by TS debug framework)
4. Compiles to WASM (for in-browser use) and native DLL/.so (for Python integration)

### 1.3 Non-Goals (Out of Scope for V1)

- Full grammar coverage (see §5.1 for 80/20 split)
- Incremental/fragment compilation (architecture reserves the interface, not implemented)
- TS debug framework (separate project)
- Python codegen migration to Rust (stays in Python, consumes IR JSON)

---

## 2. Three-Layer Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Rust Compiler (基层)                       │
│                                                                    │
│  .omni source ──→ Parser ──→ AST ──→ Semantic ──→ Intent IR      │
│                                              │                    │
│                    ┌─────────────────────────┤                    │
│                    │                         │                    │
│                    ▼                         ▼                    │
│  ┌───────────────────────┐  ┌────────────────────────────────┐   │
│  │  OmniView codegen      │  │  IR Serializer (JSON)           │   │
│  │  → JS + HTML + CSS     │  │  → component_ir.json             │   │
│  │  → SourceMap           │  │  供 Python/TS 消费               │   │
│  └───────────────────────┘  └────────────────────────────────┘   │
│           │                           │                           │
└───────────┼───────────────────────────┼───────────────────────────┘
            │                           │
            ▼                           ▼
┌───────────────────────┐  ┌──────────────────────────────────────┐
│  TS 调试框架            │  │  Python codegen 层                    │
│  （浏览器内）            │  │  （服务器端 / CLI）                   │
│                        │  │                                       │
│  消费: JS+HTML+CSS     │  │  消费: IR JSON                        │
│       + SourceMap      │  │       │                                │
│       │                │  │       ▼                                │
│       ▼                │  │  ReactCodegen / VueCodegen / etc.     │
│  DebugInstrumentor     │  │  输出: .tsx / .vue / .dart / .rs     │
│  iframe 加载            │  │                                       │
│  postMessage ↔ editor  │  │                                       │
└───────────────────────┘  └──────────────────────────────────────┘
```

### 2.1 Layer Responsibilities

| Layer | Input | Output | Must NOT do |
|-------|-------|--------|-------------|
| **Rust** | `.omni` source string | IR JSON + OmniView JS/HTML/CSS + SourceMap | Framework-specific codegen (React, Vue, etc.) |
| **Python** | IR JSON | React `.tsx` / Vue `.vue` / Flutter `.dart` / Dioxus `.rs` | Parse `.omni`, type-check, semantic analysis |
| **TS** | OmniView JS/HTML/CSS + SourceMap | Debug iframe runtime + bidirectional communication | Compile `.omni` → target code |

### 2.2 Key Design Decisions

| # | Decision | Rationale |
|---|----------|-----------|
| 1 | Hand-written Pratt parser + recursive descent | Full control, best error recovery, WASM-friendly |
| 2 | IR transmitted as JSON | Human-readable, Python-native, debuggable |
| 3 | Cover omnilang + omniview grammars | Unified parser, single source of truth |
| 4 | OmniView target: inline mini-signal (~200 lines) | Zero external deps in iframe |
| 5 | WASM built into editor as static asset | No server needed for compilation |
| 6 | Errors as JSON array | Editor frontend directly renders, no secondary parsing |
| 7 | Architecture reserves incremental compilation API | v0.1 full compile only; v0.2+ file-level deps |
| 8 | First milestone: IR JSON + Python React codegen | End-to-end verification before OmniView target |
| 9 | 80/20 grammar coverage; full coverage architected | Ship fast, extensible design |

---

## 3. Rust Compiler Internal Design

### 3.1 Crate Structure

```
omni-compiler/
├── Cargo.toml
├── src/
│   ├── lib.rs                       # WASM/native entry points
│   ├── lexer/
│   │   ├── mod.rs                   # Lexer trait + Token defs
│   │   └── token.rs                 # Token enum (50+ variants)
│   ├── parser/
│   │   ├── mod.rs                   # Parser struct + entry points
│   │   ├── ast.rs                   # All AST node types
│   │   ├── expr.rs                  # Pratt expression parser
│   │   ├── stmt.rs                  # Statement parser (recursive descent)
│   │   ├── top_level.rs             # Component/store/struct/fn parsing
│   │   ├── view.rs                  # View elements + directives
│   │   └── error.rs                 # ParseError types
│   ├── semantic/
│   │   ├── mod.rs                   # Analyzer orchestrator
│   │   ├── resolver.rs              # Symbol table + scope resolution
│   │   ├── type_checker.rs          # Type inference + validation
│   │   ├── symbols.rs               # Symbol/SymbolTable types
│   │   └── diagnostics.rs           # Diagnostic + DiagnosticBag
│   ├── ir/
│   │   ├── mod.rs                   # IR types (ComponentIR, StoreIR, etc.)
│   │   └── lowering.rs              # AST → IR transformer
│   ├── codegen/
│   │   ├── mod.rs                   # Codegen trait + dispatch
│   │   └── omniview.rs              # OmniView: JS + HTML + CSS + SourceMap
│   ├── sourcemap/
│   │   └── mod.rs                   # SourceMap struct + serialization
│   └── error.rs                     # Unified error types
├── tests/
│   ├── lexer_tests.rs
│   ├── parser_tests.rs
│   ├── semantic_tests.rs
│   ├── ir_tests.rs
│   └── codegen_tests.rs
└── benches/
    └── compile_bench.rs
```

### 3.2 Lexer

Hand-written lexer producing a flat `Token` stream. No regex — character-by-character with peek.

```rust
// lexer/token.rs

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords (omnilang)
    Fn, Let, Mut, If, Else, While, For, In, Return, Break, Continue,
    Print, Println, Async, Await, Match,
    Struct, Class, Interface, Impl, Extends, Implements,
    Type, Nominal, Static, Private, Override,
    Import, From, Export,
    True, False, Null,

    // Keywords (omniview)
    Component, Signal, Computed, Prop, Event, Ref, Store, View, Style,
    Action, Getter, Theme,
    AtStore, AtIf, AtElse, AtFor, AtEmpty, AtSlot,
    AtNative, AtFallback, AtError,
    AtMount, AtUnmount, AtEffect,
    Bind,

    // Literals
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    TemplateString(String),

    // Identifiers
    Ident(String),

    // Operators & Punctuation
    Dot, DotDotDot, QuestionMark, QuestionDot,
    Plus, Minus, Star, Slash, Percent,
    PlusEq, MinusEq, StarEq, SlashEq, PercentEq,
    Eq, EqEq, NotEq, Lt, Gt, LtEq, GtEq,
    AndAnd, OrOr, Not,
    Arrow, FatArrow, Pipe,
    LParen, RParen, LBrace, RBrace, LBracket, RBracket,
    Semicolon, Comma, Colon, At,

    // Special
    Eof, Error(String),
}

#[derive(Debug, Clone)]
pub struct Span {
    pub start: usize,   // byte offset
    pub end: usize,
    pub line: u32,
    pub col: u32,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

pub struct Lexer {
    source: Vec<char>,
    pos: usize,
    line: u32,
    col: u32,
}

impl Lexer {
    pub fn new(source: &str) -> Self;
    pub fn tokenize(&mut self) -> Vec<Token>;
    fn advance(&mut self) -> Option<char>;
    fn peek(&self) -> Option<char>;
    fn peek_n(&self, n: usize) -> Option<char>;
    fn skip_whitespace_and_comments(&mut self);
    fn read_string(&mut self) -> Token;
    fn read_number(&mut self) -> Token;
    fn read_ident_or_keyword(&mut self) -> Token;
    fn read_template_string(&mut self) -> Token;
}
```

**Key design rules:**
- Keywords are matched case-sensitively from `read_ident_or_keyword`
- Numbers support hex (`0x`), octal (`0o`), binary (`0b`), decimal, and float formats
- Strings are `"..."` with escape sequences (`\"`, `\\`, `\n`, `\t`, `\r`, `\u{...}`)
- Template strings `` `...${expr}...` `` tokenized as single `TemplateString`; interpolation parsed in semantic phase
- Comments (`//` and `/* */`) are stripped during lexing
- `@`-prefixed keywords treated as single tokens (e.g., `@if` → `TokenKind::AtIf`)
- Multi-char operators (e.g., `+=`, `==`, `!=`, `&&`, `||`, `?.`) consumed greedily

### 3.3 Parser & AST

#### 3.3.1 AST Node Types

All AST nodes live in `parser/ast.rs`. Follow the Python AST structure but with Rust idioms:

```rust
// parser/ast.rs

use std::sync::Arc;

// ── Base types ─────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: u32,
    pub col: u32,
}

// ── Types ───────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum TypeAnnotation {
    Named {
        name: String,
        type_args: Vec<TypeAnnotation>,
        span: Span,
    },
    Optional {
        base: Box<TypeAnnotation>,
        span: Span,
    },
    Union {
        types: Vec<TypeAnnotation>,
        span: Span,
    },
    Fn {
        params: Vec<TypeAnnotation>,
        return_type: Box<TypeAnnotation>,
        span: Span,
    },
}

// ── Expressions (omnilang + omniview overlap) ──────

#[derive(Debug, Clone)]
pub enum Expr {
    Literal(Literal, Span),
    VarRef(String, Span),
    BinaryOp {
        left: Box<Expr>,
        op: BinaryOpKind,
        right: Box<Expr>,
        span: Span,
    },
    UnaryOp {
        op: UnaryOpKind,
        operand: Box<Expr>,
        span: Span,
    },
    FuncCall {
        func: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    MethodCall {
        object: Box<Expr>,
        method: String,
        args: Vec<Expr>,
        span: Span,
    },
    IndexGet {
        object: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    FieldAccess {
        object: Box<Expr>,
        field: String,
        span: Span,
    },
    ArrayLiteral {
        elements: Vec<Expr>,
        span: Span,
    },
    MapLiteral {
        entries: Vec<(Expr, Expr)>,
        span: Span,
    },
    IfExpr {
        condition: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
        span: Span,
    },
    TernaryExpr {
        condition: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
        span: Span,
    },
    TemplateString {
        parts: Vec<TemplatePart>,
        span: Span,
    },
    StructLiteral {
        type_name: String,
        fields: Vec<(String, Expr)>,
        span: Span,
    },

    // Reserved for P2: Match, Closure, Arrow, OptionalChain,
    // NullCoalesce, Spread, Await
    NotYetImplemented {
        feature: String,
        span: Span,
    },
}

#[derive(Debug, Clone)]
pub enum Literal {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Null,
}

#[derive(Debug, Clone)]
pub enum BinaryOpKind {
    Add, Sub, Mul, Div, Mod,
    Eq, Neq, Lt, Gt, LtEq, GtEq,
    And, Or,
    Assign,
    AddAssign, SubAssign, MulAssign, DivAssign, ModAssign,
}

#[derive(Debug, Clone)]
pub enum UnaryOpKind {
    Not, Neg,
}

#[derive(Debug, Clone)]
pub enum TemplatePart {
    Literal(String),
    Expr(Expr),
}

// ── Statements ──────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Stmt {
    VarDecl {
        name: String,
        type_ann: Option<TypeAnnotation>,
        mutable: bool,
        init: Expr,
        span: Span,
    },
    Assign {
        target: AssignTarget,
        op: BinaryOpKind,
        value: Expr,
        span: Span,
    },
    If {
        condition: Expr,
        then_body: Block,
        else_ifs: Vec<(Expr, Block)>,
        else_body: Option<Block>,
        span: Span,
    },
    While {
        condition: Expr,
        body: Block,
        span: Span,
    },
    ForIn {
        var_name: String,
        iterable: Expr,
        body: Block,
        span: Span,
    },
    Return {
        value: Option<Expr>,
        span: Span,
    },
    Break(Span),
    Continue(Span),
    Print {
        newline: bool,
        arg: Expr,
        span: Span,
    },
    ExprStmt {
        expr: Expr,
        span: Span,
    },
    EventEmit {
        name: String,
        args: Vec<Expr>,
        span: Span,
    },
}

#[derive(Debug, Clone)]
pub enum AssignTarget {
    Name(String),
    Index { object: String, index: Expr },
    Field { object: String, field: String },
}

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub trailing_expr: Option<Expr>,
    pub span: Span,
}

// ── Top-Level Declarations ──────────────────────────

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub type_ann: Option<TypeAnnotation>,
}

#[derive(Debug, Clone)]
pub struct FunctionDef {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeAnnotation>,
    pub is_async: bool,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub type_ann: TypeAnnotation,
}

#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: String,
    pub is_nominal: bool,
    pub fields: Vec<StructField>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TaggedUnionDef {
    pub name: String,
    pub variants: Vec<Variant>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Variant {
    pub name: String,
    pub fields: Vec<StructField>,
}

// ── OmniView-Specific AST ───────────────────────────

#[derive(Debug, Clone)]
pub struct ComponentDef {
    pub name: String,
    pub props: Vec<PropDecl>,
    pub signals: Vec<SignalDecl>,
    pub computeds: Vec<ComputedDecl>,
    pub events: Vec<EventDecl>,
    pub refs: Vec<RefDecl>,
    pub store_refs: Vec<String>,
    pub lifecycles: Vec<LifecycleHook>,
    pub fns: Vec<FunctionDef>,
    pub actions: Vec<ActionDef>,
    pub view: Option<ViewBlock>,
    pub styles: Vec<StyleRule>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct PropDecl {
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub default: Option<Expr>,
}

#[derive(Debug, Clone)]
pub struct SignalDecl {
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub init: Expr,
}

#[derive(Debug, Clone)]
pub struct ComputedDecl {
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub body: Expr,
}

#[derive(Debug, Clone)]
pub struct EventDecl {
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub is_optional: bool,
}

#[derive(Debug, Clone)]
pub struct RefDecl {
    pub name: String,
    pub type_ann: TypeAnnotation,
}

#[derive(Debug, Clone)]
pub struct LifecycleHook {
    pub kind: LifecycleKind,
    pub body: Block,
    pub deps: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum LifecycleKind {
    Mount, Unmount, Effect,
}

#[derive(Debug, Clone)]
pub struct ActionDef {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeAnnotation>,
    pub body: Block,
    pub is_async: bool,
}

#[derive(Debug, Clone)]
pub struct ViewBlock {
    pub children: Vec<ViewNode>,
}

#[derive(Debug, Clone)]
pub enum ViewNode {
    Element(Element),
    IfDirective(IfDirective),
    ForDirective(ForDirective),
    SlotDirective(Option<String>),
    Text(String),
    Interpolation(Expr),
}

#[derive(Debug, Clone)]
pub struct Element {
    pub tag: String,
    pub props: Vec<ElementProp>,
    pub children: Vec<ViewNode>,
    pub self_closing: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ElementProp {
    Static { name: String, value: Expr },
    Bind { name: String, signal_path: String },
    Event { name: String, handler: EventHandler },
    Class { value: ClassValue },
    Style { entries: Vec<(String, Expr)> },
    TestId(String),
    Ref(String),
}

#[derive(Debug, Clone)]
pub enum EventHandler {
    InlineFn { params: Vec<Param>, body: Block },
    FnRef(String),
    StoreAction { store: String, action: String },
}

#[derive(Debug, Clone)]
pub enum ClassValue {
    StyleRef(String),
    StringLiteral(String),
    ClassMap(Vec<(String, Expr)>),
}

#[derive(Debug, Clone)]
pub struct IfDirective {
    pub condition: Expr,
    pub body: Vec<ViewNode>,
    pub else_ifs: Vec<(Expr, Vec<ViewNode>)>,
    pub else_body: Option<Vec<ViewNode>>,
}

#[derive(Debug, Clone)]
pub struct ForDirective {
    pub item_name: String,
    pub iterable: Expr,
    pub body: Vec<ViewNode>,
    pub empty_body: Option<Vec<ViewNode>>,
}

#[derive(Debug, Clone)]
pub struct StyleRule {
    pub class_name: String,
    pub properties: Vec<(String, StyleValue)>,
}

#[derive(Debug, Clone)]
pub enum StyleValue {
    Dimension(f64, String),  // e.g., 16px, 1.5rem
    HexColor(String),        // e.g., #ff0000
    String(String),
    Number(f64),
    Ident(String),
    List(Vec<StyleValuePart>),
}

#[derive(Debug, Clone)]
pub enum StyleValuePart {
    Dimension(f64, String),
    Ident(String),
    Number(f64),
    HexColor(String),
}

// ── Store & Theme ────────────────────────────────────

#[derive(Debug, Clone)]
pub struct StoreDef {
    pub name: String,
    pub signals: Vec<SignalDecl>,
    pub getters: Vec<GetterDecl>,
    pub actions: Vec<ActionDef>,
}

#[derive(Debug, Clone)]
pub struct GetterDecl {
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub body: Expr,
}

#[derive(Debug, Clone)]
pub struct ThemeDef {
    pub categories: Vec<ThemeCategory>,
}

#[derive(Debug, Clone)]
pub struct ThemeCategory {
    pub name: String,
    pub values: Vec<(String, String)>,
}

// ── Top-Level Program ───────────────────────────────

#[derive(Debug, Clone)]
pub struct Program {
    pub components: Vec<ComponentDef>,
    pub stores: Vec<StoreDef>,
    pub themes: Vec<ThemeDef>,
    // omnilang-only (no UI):
    pub functions: Vec<FunctionDef>,
    pub structs: Vec<StructDef>,
    pub tagged_unions: Vec<TaggedUnionDef>,
    // reserved P2: classes, interfaces, impl blocks
    pub imports: Vec<ImportStmt>,
}

#[derive(Debug, Clone)]
pub struct ImportStmt {
    pub names: Vec<String>,
    pub path: String,
}
```

#### 3.3.2 Pratt Expression Parser

```rust
// parser/expr.rs

impl Parser {
    /// Parse an expression starting at current position.
    /// Uses Pratt parsing for operator precedence.
    pub fn parse_expr(&mut self) -> ParseResult<Expr>;

    /// Pratt: parse a prefix expression (atom + unary ops).
    fn parse_prefix(&mut self) -> ParseResult<Expr>;

    /// Pratt: parse an infix expression given the left-hand side
    /// and minimum binding power.
    fn parse_infix(&mut self, left: Expr, min_bp: u8) -> ParseResult<Expr>;

    /// Return the binding power for a given token as prefix.
    fn prefix_binding_power(kind: &TokenKind) -> Option<(u8, fn(&mut Parser) -> ParseResult<Expr>)>;

    /// Return the binding power for a given token as infix.
    fn infix_binding_power(kind: &TokenKind) -> Option<(u8, u8, fn(&mut Parser, Expr) -> ParseResult<Expr>)>;
}
```

**Precedence Table** (lowest to highest):

| Precedence | Operators | Associativity |
|-----------|-----------|---------------|
| 1 | `=` `+=` `-=` `*=` `/=` `%=` | right |
| 2 | `\|\|` | left |
| 3 | `&&` | left |
| 4 | `==` `!=` `<` `>` `<=` `>=` | left |
| 5 | `+` `-` | left |
| 6 | `*` `/` `%` | left |
| 7 | `!` `-` (unary) | — (prefix) |
| 8 | `.method()` `[index]` `()` (call) | left (postfix) |

#### 3.3.3 Recursive Descent for Everything Else

```
parse_program()
  ├─ parse_component_def()
  │   ├─ parse_prop_decl()
  │   ├─ parse_signal_decl()
  │   ├─ parse_computed_decl()
  │   ├─ parse_fn_def()
  │   ├─ parse_action_def()
  │   ├─ parse_lifecycle_hook()
  │   ├─ parse_view_block()
  │   │   └─ parse_view_node() → Element, If, For, Slot, Text, Interpolation
  │   │       └─ parse_element_props() → 7 prop types
  │   └─ parse_style_block()
  ├─ parse_store_def()
  ├─ parse_function_def()     // standalone fn (omnilang)
  │   └─ parse_block()
  │       └─ parse_stmt()     // VarDecl, Assign, If, While, For, Return, etc.
  │           └─ parse_expr() // ← Pratt parser
  ├─ parse_struct_def()
  ├─ parse_tagged_union_def()
  └─ parse_import()
```

### 3.4 Semantic Analysis

Multi-pass analysis producing a `SemanticInfo` structure and a `DiagnosticBag`.

```rust
// semantic/mod.rs

pub struct SemanticAnalyzer {
    diagnostics: DiagnosticBag,
    symbols: SymbolTable,
}

impl SemanticAnalyzer {
    pub fn analyze(program: &Program) -> SemanticResult;

    // Pass 1: Collect symbols
    fn collect_symbols(&mut self, program: &Program);
    // - Register all signal, computed, prop, event, ref, store_ref names
    // - Register fn/action names and params
    // - Build hierarchical scope tree (component → fn → block)
    // - Detect duplicate declarations

    // Pass 2: Resolve references
    fn resolve_references(&mut self, program: &Program);
    // - For each VarRef, walk up scope chain to find declaration
    // - Resolve FnRef event handlers to actual fn defs
    // - Resolve store.action references
    // - Report undeclared name errors

    // Pass 3: Type checking
    fn type_check(&mut self, program: &Program);
    // - Infer expression types bottom-up
    // - Check binary op operand compatibility
    // - Check fn call arg count + types
    // - Check return type against declared return type
    // - Check signal/computed type consistency
    // - Check @for iterable is Array type
    // - Check @if condition is bool

    // Pass 4: Collect dependencies
    fn collect_dependencies(&mut self, program: &Program);
    // - Extract signal reads from computed body expressions
    // - Extract deps from @effect hooks
    // - Detect circular dependency chains

    // Pass 5: Validate constraints
    fn validate_constraints(&mut self, program: &Program);
    // - bind: must bind to signal, not prop
    // - @for must have key prop on root element
    // - break/continue must be inside loop
    // - return must be inside fn/action
}

pub struct SemanticResult {
    pub symbols: SymbolTable,
    pub type_map: HashMap<ExprId, TypeAnnotation>,
    pub dependencies: HashMap<String, Vec<String>>,
    pub diagnostics: Vec<Diagnostic>,
}

pub struct Diagnostic {
    pub severity: Severity,       // Error | Warning | Info
    pub code: String,             // e.g., "E0001", "P0214"
    pub message: String,
    pub span: Span,
    pub suggestion: Option<String>,
}

pub enum Severity { Error, Warning, Info }
```

### 3.5 Intent IR

Mirrors Python's `ir_types.py` exactly. Each IR type serializes to/from JSON via `serde`.

```rust
// ir/mod.rs

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentIR {
    pub name: String,
    pub generic_params: Vec<String>,
    pub props: HashMap<String, TypeIR>,
    pub states: Vec<StateIntent>,
    pub computeds: Vec<ComputedIntent>,
    pub effects: Vec<EffectIntent>,
    pub lifecycles: Vec<LifecycleIntent>,
    pub fns: Vec<FunctionIR>,
    pub store_refs: Vec<String>,
    pub refs: Vec<RefIR>,
    pub view: Option<ViewTree>,
    pub styles: Vec<StyleRuleIR>,
}

// (All other IR types match ir_types.py exactly — StateIntent, ComputedIntent,
//  EffectIntent, LifecycleIntent, ConditionalIntent, ListRenderingIntent,
//  ElementIR, EventHandlerIR, TypeIR, ExprIR variants, StatementIR variants, etc.)
```

See Appendix A for the complete IR type definitions.

### 3.6 IR Lowering

```rust
// ir/lowering.rs

pub struct IRFixture;

impl IRFixture {
    pub fn lower_program(program: &Program, semantics: &SemanticResult) -> IRProgram;

    fn lower_component(comp: &ComponentDef, semantics: &SemanticResult) -> ComponentIR;
    fn lower_store(store: &StoreDef) -> StoreIR;
    fn lower_view_node(node: &ViewNode) -> ViewTree;
    fn lower_element(el: &Element) -> ElementIR;
    fn lower_expr(expr: &Expr) -> ExprIR;
    fn lower_stmt(stmt: &Stmt) -> StatementIR;
    fn lower_type(ty: &TypeAnnotation) -> TypeIR;
}
```

### 3.7 OmniView Codegen

Generates standalone JS + HTML + CSS that runs in an iframe with zero dependencies.

```rust
// codegen/omniview.rs

pub struct OmniViewCodegen;

impl OmniViewCodegen {
    pub fn generate(ir: &IRProgram) -> OmniViewOutput;
}

pub struct OmniViewOutput {
    pub js: String,           // Inline signal runtime + generated component code
    pub html: String,          // Static HTML structure
    pub css: String,           // Style rules
    pub source_map: SourceMap, // nodeId → source location mapping
}
```

**Generated JS structure:**

```javascript
// ═══ Inline Mini-Signal Runtime (~200 lines) ═══
(() => {
  let _activeEffect = null;
  const _subs = new Map();

  function _signal(name, init) {
    let val = init;
    const subs = new Set();
    return {
      get: () => { if (_activeEffect) { subs.add(_activeEffect); _activeEffect._deps.add(() => subs.delete(_activeEffect)); } return val; },
      set: (v) => { val = v; subs.forEach(fn => fn()); },
    };
  }

  function _computed(name, fn) {
    let val, dirty = true;
    const s = _signal(name, undefined);
    const run = () => {
      dirty = false;
      const prev = _activeEffect;
      const deps = new Set();
      _activeEffect = { _deps: deps, run: () => { dirty = true; s.set(fn()); } };
      const newVal = fn();
      _activeEffect = prev;
      val = newVal;
      return val;
    };
    run();
    return { get: () => (dirty ? run() : val), _signal: s };
  }

  function _effect(fn) {
    const e = { _deps: new Set(), run: fn };
    const prev = _activeEffect;
    _activeEffect = e;
    fn();
    _activeEffect = prev;
    return () => e._deps.forEach(d => d());
  }

  // ═══ Generated Component ═══
  // signal count: i32 = 0
  const count = _signal("count", 0);

  // fn increment()
  function increment() {
    count.set(count.get() + 1);
  }

  // Initial DOM
  document.getElementById("root").innerHTML = `<div data-nid="el-container-001">
    <button data-nid="el-btn-001" data-on-click="increment">+1</button>
    <span data-nid="el-txt-001">0</span>
  </div>`;

  // Bind events
  document.querySelector('[data-on-click="increment"]').addEventListener('click', increment);

  // Reactive text binding
  _effect(() => {
    document.querySelector('[data-nid="el-txt-001"]').textContent = String(count.get());
  });
})();
```

### 3.8 SourceMap

```rust
// sourcemap/mod.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMap {
    pub entries: Vec<SourceMapEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMapEntry {
    /// Canvas element/node ID, e.g. "el-btn-001"
    pub node_id: String,

    /// Position in .omni source
    pub omni_span: SourceSpan,

    /// Positions in generated output (one entry per occurrence)
    pub target_spans: Vec<TargetSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceSpan {
    pub line: u32,
    pub col: u32,
    pub len: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetSpan {
    /// Which output file: "App.js", "App.html", "App.css"
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub len: u32,
    pub kind: TargetSpanKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TargetSpanKind {
    DomElement,    // data-nid attribute on HTML element
    JsExpression,  // JS expression/statement
    CssRule,       // CSS rule
}
```

### 3.9 Error Format

All errors output as JSON array:

```json
[
  {
    "severity": "error",
    "code": "E0001",
    "message": "Undeclared variable 'coun'",
    "line": 12,
    "col": 5,
    "end_line": 12,
    "end_col": 9,
    "suggestion": "Did you mean 'count'?"
  },
  {
    "severity": "warning",
    "code": "W0001",
    "message": "Unused variable 'temp'",
    "line": 8,
    "col": 3,
    "end_line": 8,
    "end_col": 7,
    "suggestion": null
  }
]
```

---

## 4. WASM & FFI Interface

### 4.1 WASM Entry Point

```rust
// lib.rs

use wasm_bindgen::prelude::*;

/// Compile .omni source to IR JSON.
/// Returns JSON string: { success, ir?, errors[] }
#[wasm_bindgen]
pub fn compile_to_ir(source: &str) -> String;

/// Compile .omni source to OmniView (JS + HTML + CSS + SourceMap).
/// Returns JSON string: { success, files?, source_map?, errors[] }
#[wasm_bindgen]
pub fn compile_to_omniview(source: &str) -> String;

/// Compile a fragment of .omni (for HMR patching).
/// Returns JSON string: { success, patches?, errors[] }
#[wasm_bindgen]
pub fn compile_fragment(source: &str, fragment_type: &str) -> String;

/// Return the compiler version.
#[wasm_bindgen]
pub fn version() -> String;
```

### 4.2 Native FFI (for Python via ctypes/cffi)

```rust
// Exported C-compatible functions
#[no_mangle]
pub extern "C" fn omni_compile_to_ir(source: *const c_char) -> *mut c_char;
#[no_mangle]
pub extern "C" fn omni_compile_to_omniview(source: *const c_char) -> *mut c_char;
#[no_mangle]
pub extern "C" fn omni_free_string(ptr: *mut c_char);
```

### 4.3 Build Targets

```toml
# Cargo.toml
[lib]
crate-type = ["cdylib", "rlib"]

# cdylib → .wasm (for web) / .dll (for Windows) / .so (for Linux/Mac)
# rlib   → .rlib (for Rust consumers)

[features]
default = []
wasm = ["wasm-bindgen", "serde-wasm-bindgen"]
native = []
```

Build commands:
```bash
# WASM for browser
wasm-pack build --target web --out-dir ../editor/public/wasm

# Native DLL for Python
cargo build --release --lib  # → target/release/omni_compiler.dll
```

---

## 5. Scope & Milestones

### 5.1 Grammar Coverage (80/20 Split)

**V1 — Covered:**

| Module | Items |
|--------|-------|
| **omnilang expressions** | Literals, VarRef, BinaryOp (arithmetic + comparison + logic), UnaryOp (-, !), FuncCall, MethodCall, IndexGet, FieldAccess, ArrayLiteral, MapLiteral, IfExpr, TernaryExpr, StructLiteral |
| **omnilang statements** | VarDecl (simple binding only), Assign, If/Else, While, ForIn, Return, Break, Continue, Print, ExprStmt, Block |
| **omnilang top-level** | FunctionDef, StructDef, TaggedUnionDef, ImportStmt |
| **omniview component** | ComponentDef, PropDecl, SignalDecl, ComputedDecl, EventDecl, RefDecl, StoreRef, LifecycleHook, FunctionDef, ActionDef, ViewBlock, StyleBlock |
| **omniview view** | Element (all 7 prop types), IfDirective, ForDirective, SlotDirective, Text, Interpolation |
| **omniview store** | StoreDef, StoreSignal, StoreGetter, StoreAction |

**V2 — Reserved:**

| Module | Items |
|--------|-------|
| om nilang | Closures, Arrow functions, Match expressions, Template strings, Optional chains, Null coalesce, Spread expressions, Async functions, Await |
| omnilang | Generic params, Destructure patterns (array + object), Rest patterns, FnType, Class/Interface/Impl blocks |
| omniview | ThemeDef, NativeBlock, FallbackBlock, ErrorBoundary, bind: prop, class_map prop |

All reserved features emit `ParseError::NotYetImplemented { feature: String, span: Span }`.

### 5.2 Milestones

```
Milestone 1: IR Pipeline (WASM) ──────────────────
   Parser (80%) + Semantic (4 passes) + IR Lowering
   + IR JSON output
   + WASM compile_to_ir() working in browser
   + Python React codegen consumes IR JSON
   + Counter.omni round-trip verified
  ✓ Deliverable: .wasm in editor/public/wasm/

Milestone 2: OmniView Codegen ────────────────────
   OmniView codegen: IR → JS + HTML + CSS
   + SourceMap generation
   + Inline mini-signal runtime
   + WASM compile_to_omniview() working
  ✓ Deliverable: OmniView codegen producing runnable iframe preview

Milestone 3: Native FFI ──────────────────────────
   C-compatible FFI for Python
   + Python bindings (cffi)
   + Replace Python parser/semantic/lowering with Rust
   + Existing 261 tests pass via Rust backend
  ✓ Deliverable: Python omniview.api uses Rust DLL

Milestone 4: Full Grammar ────────────────────────
   Remaining 20% grammar (closures, match, templates, etc.)
   + Remaining semantic passes
   + Python omni codegen parity
  ✓ Deliverable: All Python features available in Rust

Milestone 5: Incremental Compilation ─────────────
   Dependency graph (source node → IR node → output file)
   + compile_fragment() with file-level granularity
   + HMR integration with TS debug framework
  ✓ Deliverable: Real-time sync in debug tab
```

---

## 6. Python 端改造方案

Python 端需要做三件事：
1. **定义 IR JSON Schema** ——成为 Rust ↔ Python 的契约
2. **重构 codegen/** ——从消费 Python IR dataclass 改为消费 JSON dict
3. **改造 api.py 管道** ——支持双路径（Python 原生管道 / Rust IR 管道）

### 6.1 IR JSON Schema

Python 是 IR schema 的"定义者"——Python 的 `ir_types.py` 定义了所有 IR 类型，Rust 按这些类型输出 JSON。

```python
# omniview/ir_schema.py — IR JSON Schema 定义

from typing import TypedDict, Literal, Optional, Union, Any

class TypeIR(TypedDict, total=False):
    kind: str           # "void"|"i32"|"f64"|"bool"|"String"|"Array"|"Map"|"struct"
    type_args: list  # list[TypeIR] for Array<i32>, Map<String,i32>
    is_nullable: bool

class LiteralExpr(TypedDict):
    _kind: Literal["LiteralExpr"]
    value: Any          # int | float | str | bool | None

class IdentifierExpr(TypedDict):
    _kind: Literal["IdentifierExpr"]
    name: str

class BinaryOpExpr(TypedDict):
    _kind: Literal["BinaryOpExpr"]
    op: str             # "+"|"-"|"*"|"/"|"%"|"=="|"!="|"<"|">"|"<="|">="|"&&"|"||"
    left: dict          # ExprIR
    right: dict         # ExprIR

class UnaryOpExpr(TypedDict):
    _kind: Literal["UnaryOpExpr"]
    op: str             # "!"|"-"
    operand: dict

class FieldAccessExpr(TypedDict):
    _kind: Literal["FieldAccessExpr"]
    object: dict
    field: str

class SignalRefIR(TypedDict):
    owner: Optional[str]  # None=component, "store"=store signal
    name: str

class StateIntent(TypedDict):
    _kind: Literal["StateIntent"]
    name: str
    type: TypeIR
    initial_value: dict          # ExprIR
    mutability: str              # "mutable"|"derived"
    update_semantics: str        # "setter-call"
    accessor_style: str          # "direct"

class MutationIntent(TypedDict):
    _kind: Literal["MutationIntent"]
    target: SignalRefIR
    operation: str               # "direct_assign"|"increment"|"decrement"|...
    amount: Optional[dict]       # ExprIR | None

class ComputedIntent(TypedDict):
    _kind: Literal["ComputedIntent"]
    name: str
    type: TypeIR
    body: dict                   # ExprIR
    deps: list[str]

class EffectIntent(TypedDict):
    _kind: Literal["EffectIntent"]
    deps: list[str]
    body: dict                   # BlockIR

class LifecycleIntent(TypedDict):
    _kind: Literal["LifecycleIntent"]
    kind: str                    # "mount"|"unmount"
    body: dict

class ElementIR(TypedDict):
    _kind: Literal["ElementIR"]
    tag: str
    props: dict[str, dict]       # str → ExprIR
    bindings: dict[str, SignalRefIR]
    events: dict[str, EventHandlerIR]
    style_classes: Optional[dict]
    inline_style: Optional[dict]
    children: list               # list[ViewTree] — recursive
    test_id: Optional[str]
    ref_name: Optional[str]
    construction_kind: str

class ConditionalBranch(TypedDict):
    condition: dict
    body: Optional[dict]

class ConditionalIntent(TypedDict):
    _kind: Literal["ConditionalIntent"]
    branches: list               # list[ConditionalBranch]
    fallback: Optional[dict]

class ListRenderingIntent(TypedDict):
    _kind: Literal["ListRenderingIntent"]
    iterable: dict
    item_binding: dict           # BindingIR {name, type?}
    body: Optional[dict]
    key_expr: Optional[dict]
    empty_body: Optional[dict]

class ComponentIR(TypedDict):
    _kind: Literal["ComponentIR"]
    name: str
    props: dict[str, TypeIR]
    states: list                  # list[StateIntent]
    computeds: list               # list[ComputedIntent]
    effects: list                 # list[EffectIntent]
    lifecycles: list              # list[LifecycleIntent]
    fns: list                     # list[FunctionIR]
    store_refs: list[str]
    refs: list                    # list[RefIR]
    view: Optional[dict]          # ViewTree | None
    styles: list                  # list[StyleRuleIR]

class StoreIR(TypedDict):
    _kind: Literal["StoreIR"]
    name: str
    signals: dict[str, TypeIR]
    signal_defaults: dict[str, Any]
    getters: dict[str, dict]
    actions: list

class IRProgram(TypedDict):
    version: str                  # "1.0"
    components: dict[str, ComponentIR]   # name → ComponentIR
    stores: dict[str, StoreIR]            # name → StoreIR
    source_map: SourceMapIR
```

**`_kind` 字段是关键：** 每个 IR 对象都自带 `_kind` 标记（和 Python `ir_types.py` 的 dataclass 名一致）。Python codegen 用 `_kind` 做 dispatch，不需要做类型推断。

### 6.2 Python codegen 重构

当前 codegen 消费 Python dataclass 的 `ComponentIR`。重构后消费 dict。

**重构前的 codegen：**

```python
# omniview/codegen/react.py — 当前
from omniview.ir_types import ComponentIR, ElementIR, StateIntent, ...

class ReactCodegen:
    def generate_component(self, comp: ComponentIR) -> CodegenResult:
        # comp.states, comp.view, etc. → dataclass 属性访问
        for state in comp.states:
            self._gen_state(state)       # state.name, state.type
        if comp.view:
            self._gen_view(comp.view)
```

**重构后的 codegen：**

```python
# omniview/codegen/react.py — 重构后
from omniview.ir_schema import ComponentIR as ComponentIRDict
from omniview.codegen.ir_reader import IRReader

class ReactCodegen:
    def generate_component(self, comp: dict) -> CodegenResult:
        """
        Generate React TSX from ComponentIR dict.
        
        Args:
            comp: dict matching ComponentIR schema (from Rust IR JSON)
        Returns:
            CodegenResult with {Name}.tsx and {Name}.module.css
        """
        name = comp["name"]
        states = comp.get("states", [])      # list of StateIntent dicts
        computeds = comp.get("computeds", [])
        fns = comp.get("fns", [])
        view = comp.get("view")              # ViewTree dict | None
        styles = comp.get("styles", [])
        
        # ... rest of codegen logic, accessing dict keys
        for state in states:
            self._gen_state(state)           # state["name"], state["type"]
```

**改动量估算：**

| 文件 | 当前行数 | 改动 | 说明 |
|------|---------|------|------|
| `codegen/react.py` | 1625 | ~30% 行数改动 | `.name` → `["name"]` |
| `codegen/vue.py` | 970 | ~30% | 同上 |
| `codegen/flutter.py` | 1530 | ~30% | 同上 |
| `codegen/dioxus.py` | 820 | ~30% | 同上 |
| `codegen/omniview.py` | 1210 | ~30% | 同上 |
| **新增** `codegen/ir_reader.py` | 0→150 | NEW | 解析 IR JSON，验证 `_kind`，提供类型化读取 |
| **新增** `ir_schema.py` | 0→350 | NEW | 所有 IR 类型的 TypedDict 定义 |

### 6.3 IR Reader（可选但推荐的辅助层）

```python
# omniview/codegen/ir_reader.py

import json
from typing import Any

class IRReader:
    """Read a Rust-generated IR JSON and validate/enrich it for codegen."""
    
    def __init__(self, ir_json: str | dict):
        if isinstance(ir_json, str):
            self.ir: dict = json.loads(ir_json)
        else:
            self.ir = ir_json
        self._errors: list[str] = []
        self._validate()
    
    def _validate(self) -> None:
        """Check version and top-level structure."""
        v = self.ir.get("version")
        if v != "1.0":
            self._errors.append(f"Unsupported IR version: {v}")
        if "components" not in self.ir and "stores" not in self.ir:
            self._errors.append("IR must contain 'components' or 'stores'")
    
    @property
    def errors(self) -> list[str]:
        return self._errors
    
    @property
    def is_valid(self) -> bool:
        return len(self._errors) == 0
    
    def components(self) -> dict[str, dict]:
        """Return {name: ComponentIR}."""
        return self.ir.get("components", {})
    
    def stores(self) -> dict[str, dict]:
        """Return {name: StoreIR}."""
        return self.ir.get("stores", {})
    
    def source_map(self) -> dict:
        return self.ir.get("source_map", {})
    
    @staticmethod
    def ir_kind(obj: dict) -> str:
        """Read _kind field for dispatch. e.g. 'ElementIR', 'StateIntent'."""
        return obj.get("_kind", "unknown")
```

### 6.4 api.py 双路径

```python
# omniview/api.py — 改造后

def compile_omni(
    source: str,
    filepath: str = "<input>",
    target: str = "react",
    previous_files: dict[str, str] | None = None,
    debug_mode: bool = False,
) -> CompileResult:
    """
    Unified compile entry point.
    
    Automatically selects pipeline:
      - If Rust DLL/CLI available → calls Rust compiler for parse/semantic/IR
      - Falls back to Python pipeline if Rust not available
    """
    # Try Rust path first
    ir_program = _try_rust_compile(source)
    
    if ir_program is not None:
        return _compile_from_ir(ir_program, target, previous_files)
    
    # Fallback: full Python pipeline
    return _compile_ui_mode(source, filepath, target, previous_files, debug_mode)


def _try_rust_compile(source: str) -> dict | None:
    """Try compiling .omni → IR via Rust.
    Returns IRProgram dict on success, None if Rust unavailable or errors.
    """
    # Option A: WASM via wasmtime-py / wasmer
    # Option B: Native DLL via cffi
    # Option C: CLI subprocess: omni-compiler compile --to-ir
    try:
        result = _call_rust_compiler(source)
        if result.get("success"):
            return result["ir"]
        # Rust found errors → return them as compilation errors
        errors = result.get("errors", [])
        if errors:
            return {"_rust_errors": errors}
    except RustCompilerNotFound:
        pass
    return None
```

### 6.5 Editor Server 改造

```python
# editor/server/main.py — 新增/改造的端点

# 1. 改造后的 /api/compile （支持 Rust 后端）
@app.post("/api/compile")
async def api_compile(req: CompileRequest):
    """Compile .omni → target code.
    Automatically uses Rust backend if available, Python fallback if not.
    """
    result = compile_omni(
        source=req.source,
        filepath=req.filepath or "<input>",
        target=req.target,
        previous_files=req.previousFiles,
        debug=req.debug,
    )
    return {
        "success": result.success,
        "files": result.files if result.success else None,
        "errors": result.errors,
        "diff": result.diff,
    }

# 2. 新增：纯 IR 编译端点（调试预览专用）
@app.post("/api/compile-from-ir")
async def api_compile_from_ir(req: CompileFromIRRequest):
    """Compile from IR JSON. Used by TS debug framework when
    Rust WASM already produced IR in-browser."""
    reader = IRReader(req.ir)
    if not reader.is_valid:
        return {"success": False, "errors": reader.errors}
    
    codegen = _get_codegen(req.target)
    results = {}
    errors = []
    
    for name, comp in reader.components().items():
        r = codegen.generate_component(comp)
        results.update(r.files)
        errors.extend(r.errors)
    
    for name, store in reader.stores().items():
        r = codegen.generate_store(store)
        results.update(r.files)
        errors.extend(r.errors)
    
    return {"success": True, "files": results, "errors": errors}

# 3. 改造：/api/run/start 使用 Rust 输出
@app.post("/api/run/start")
async def api_run_start(req: RunStartRequest):
    """Start Vite dev server for compiled app.
    Uses Rust OmniView output if available."""
    
    if req.use_omniview_preview:
        # 调试预览模式：Rust 直接输出 JS/HTML/CSS
        result = _call_rust_omniview(req.source)
        write_omniview_to_run_dir(result.files)
    else:
        # 生产模式：Python codegen 从 IR 生成框架代码
        ir = _call_rust_ir(req.source)
        codegen = _get_codegen(req.target)
        files = codegen.generate(ir)
        write_to_run_dir(files)
    
    return start_vite_server()
```

### 6.6 编辑器中 Rust WASM 加载（TS 侧）

```typescript
// editor/client/src/api/rustCompiler.ts

type CompileResult = {
  success: boolean;
  ir?: IRProgram;
  errors?: Diagnostic[];
};

type OmniViewResult = {
  success: boolean;
  files?: { js: string; html: string; css: string };
  sourceMap?: SourceMap;
  errors?: Diagnostic[];
};

class RustCompiler {
  private wasm: any;  // wasm-bindgen instance
  private ready: boolean = false;

  async init(): Promise<void> {
    if (this.ready) return;
    // Load from static asset
    const wasm = await import('../../public/wasm/omni_compiler.js');
    await wasm.default();  // Initialize WASM
    this.wasm = wasm;
    this.ready = true;
  }

  compileToIR(source: string): CompileResult {
    const json = this.wasm.compile_to_ir(source);
    return JSON.parse(json);
  }

  compileToOmniView(source: string): OmniViewResult {
    const json = this.wasm.compile_to_omniview(source);
    return JSON.parse(json);
  }
}

// Singleton
export const rustCompiler = new RustCompiler();
```

### 6.7 Python 端的文件变更清单

```
新建:
  omniview/ir_schema.py          (~350 lines)  IR JSON schema (TypedDict)
  omniview/codegen/ir_reader.py  (~150 lines)  IR JSON reader/validator
  omniview/codegen/rust_bridge.py (~100 lines)  Call Rust DLL/CLI

修改:
  omniview/api.py               (~50 lines)   双路径 dispatch
  omniview/codegen/react.py     (~500 lines)  dataclass → dict access
  omniview/codegen/vue.py       (~300 lines)  同上
  omniview/codegen/flutter.py   (~460 lines)  同上
  omniview/codegen/dioxus.py    (~250 lines)  同上
  omniview/codegen/omniview.py  (~360 lines)  同上
  omniview/codegen/base.py      (~20 lines)   类型签名更新
  editor/server/main.py         (~80 lines)   新增 /api/compile-from-ir，改造 /api/compile

不动:
  omniview/ast_nodes.py          — 保留（Python 原生管道继续用）
  omniview/parser.py             — 保留（Python 原生管道继续用）
  omniview/preprocessor.py       — 保留（Python 原生管道继续用）
  omniview/semantic/             — 保留（Python 原生管道继续用）
  omniview/lowering.py           — 保留（Python 原生管道继续用）
  
删除: 无 — Python 原生管道完整保留作为 fallback
```

### 6.8 双管道共存策略

```
                     ┌─ Rust compiler available? ──┐
                     │                              │
                     ▼ YES                          ▼ NO
           ┌──────────────────┐          ┌────────────────────┐
           │ Rust pipeline     │          │ Python pipeline     │
           │                    │          │  (unchanged)       │
           │ .omni → Rust DLL  │          │  .omni → preproc   │
           │  → IR JSON        │          │  → Lark parse      │
           │  → Python codegen │          │  → semantic        │
           │  → target code    │          │  → IR lower        │
           │                    │          │  → codegen         │
           └──────────────────┘          └────────────────────┘
                     │                              │
                     └──────┬───────────────────────┘
                            ▼
                    Target code (React/Vue/Flutter/etc.)
```

**检测 Rust 是否可用的方式：**
```python
# omniview/codegen/rust_bridge.py

import subprocess
import ctypes
import json
from pathlib import Path

class RustBridge:
    _dll = None
    
    @classmethod
    def is_available(cls) -> bool:
        """Check if Rust compiler is available (CLI or DLL)."""
        # Option 1: Check for native DLL
        dll_paths = [
            Path(__file__).parent.parent.parent / "omni-compiler" / "target" / "release" / "omni_compiler.dll",
            Path("omni-compiler/target/release/omni_compiler.dll"),
        ]
        for p in dll_paths:
            if p.exists():
                return True
        
        # Option 2: Check for CLI
        try:
            subprocess.run(["omni-compiler", "--version"], capture_output=True, timeout=5)
            return True
        except (FileNotFoundError, subprocess.TimeoutExpired):
            pass
        
        return False
    
    @classmethod
    def compile_to_ir(cls, source: str) -> dict:
        """Compile .omni → IR via Rust. Raises RuntimeError on failure."""
        # Try DLL first, fallback to CLI
        result = cls._try_dll(source) or cls._try_cli(source)
        if result is None:
            raise RuntimeError("Rust compiler not available")
        return result
```

### 6.9 Migration Path (Updated)

```
Phase 0 (Current):
  .omni source
    → Python preprocessor
      → Python Lark parser
        → Python semantic analyzer
          → Python IR lowering
            → Python codegen (React/Vue/Flutter/Dioxus/OmniView)
              → target code

Phase 1 (M1 – IR Pipeline):
  .omni source
    → Rust WASM (parse + semantic + IR) in browser
      → IR JSON
        → Python codegen (consume IR dict, not dataclass)
          → React/Vue/Flutter/Dioxus

  Python pipeline: 完整保留作为 fallback

Phase 2 (M2 – OmniView Codegen):
  .omni source
    → Rust WASM (parse + semantic + IR + OmniView codegen)
      → JS + HTML + CSS + SourceMap (for iframe preview)

  Python codegen: 仍然处理 React/Vue/Flutter/Dioxus（消费 IR JSON）

Phase 3 (M3 – Native FFI):
  .omni source
    → Rust DLL (parse + semantic + IR) → Python codegen → target code
    → Rust DLL (OmniView codegen) → JS/HTML/CSS + SourceMap

  Python pipeline: 仍然保留作为 fallback（不做主路径）
  WASM 仍然存在用于浏览器预览

Phase 4 (M4 – Full Grammar):
  Rust: 补全闭包、match、模板字符串等 20% 语法
  Python pipeline: 可以开始退役（仅保留作为参考实现）

Phase 5 (M5 – Incremental):
  Rust: 片段编译 + 依赖图
  Python codegen: 不变（仍然是框架 target 的 codegen）
```

---

## 7. Testing Strategy

测试分为五个阶段，按照依赖顺序排列。每个阶段有明确的**进入条件**（什么时候能开始写这个测试）和**通过标准**（怎样算通过）。

---

### 7.1 Python IR Schema 测试（不依赖 Rust，最先写）

**文件:** `omniview/tests/test_ir_schema.py`

**进入条件:** `ir_schema.py` 写完  
**通过标准:** 所有 TypedDict 与实际 Python IR dataclass 输出一致

```python
# omniview/tests/test_ir_schema.py

import json
import pytest
from omniview.ir_schema import ComponentIR, ElementIR, StateIntent, IRProgram
from omniview.ir_types import ComponentIR as ComponentIRDataclass
from omniview.lowering import IRFixture

def test_schema_matches_dataclass_for_counter():
    """Python pipeline 输出的 IR → 转成 dict → 符合 ir_schema 的 TypedDict."""
    # 用 Python 原生管道编译 Counter.omni
    from omniview.api import compile_omni
    import omniview.preprocessor as pp
    from omniview.lexer import create_omniview_parser
    from omniview.parser import OmniViewTransformer
    from omniview.semantic.analyzer import SemanticAnalyzer
    
    source = open("build/test/counter/Counter.omni").read()
    pre = pp.preprocess(source)
    tree = create_omniview_parser().parse(pre.processed)
    ast = OmniViewTransformer(pre.block_map).transform(tree)
    analysis = SemanticAnalyzer().analyze(ast)
    ir_map = IRFixture().lower_program(ast)
    
    # 把 Python dataclass IR 转为 dict（模拟 Rust 输出的 JSON）
    comp_ir = ir_map["Counter"]
    ir_dict = _ir_to_dict(comp_ir)
    
    # 验证：dict 符合 TypedDict schema
    _validate_component_ir(ir_dict)
    assert ir_dict["_kind"] == "ComponentIR"
    assert ir_dict["name"] == "Counter"
    assert len(ir_dict["states"]) > 0
    for state in ir_dict["states"]:
        assert state["_kind"] == "StateIntent"
        assert "name" in state
        assert "type" in state
        assert "initial_value" in state


def test_element_ir_structure():
    """ElementIR dict 结构正确——所有必填字段存在、children 是递归的."""
    ir = {
        "_kind": "ElementIR",
        "tag": "Button",
        "props": {"label": {"_kind": "LiteralExpr", "value": "Submit"}},
        "bindings": {},
        "events": {"onPress": {"_kind": "EventHandlerIR", "kind": "fn_ref", "ref": "handleClick"}},
        "style_classes": None,
        "inline_style": None,
        "children": [
            {"_kind": "ElementIR", "tag": "Text", "props": {}, "bindings": {},
             "events": {}, "style_classes": None, "inline_style": None,
             "children": [], "test_id": None, "ref_name": None,
             "construction_kind": "tag-mapping"}
        ],
        "test_id": None,
        "ref_name": None,
        "construction_kind": "tag-mapping",
    }
    _validate_element_ir(ir)


def test_conditional_intent_structure():
    """@if/@else → ConditionalIntent branches 列表结构."""
    ir = {
        "_kind": "ConditionalIntent",
        "branches": [
            {
                "condition": {"_kind": "IdentifierExpr", "name": "isLoggedIn"},
                "body": {"_kind": "ElementIR", "tag": "Text", "props": {},
                         "bindings": {}, "events": {}, "children": [], "test_id": None,
                         "ref_name": None, "construction_kind": "tag-mapping",
                         "style_classes": None, "inline_style": None},
            },
        ],
        "fallback": None,
    }
    validate = _validate_ir_object(ir)
    assert validate is True


def test_list_rendering_intent_structure():
    """@for item in items → ListRenderingIntent 结构."""
    ir = {
        "_kind": "ListRenderingIntent",
        "iterable": {"_kind": "IdentifierExpr", "name": "items"},
        "item_binding": {"name": "item"},
        "body": None,
        "key_expr": None,
        "empty_body": None,
    }
    validate = _validate_ir_object(ir)
    assert validate is True


# ── 辅助函数 ──

def _ir_to_dict(ir_obj) -> dict:
    """将 Python IR dataclass 递归转为 dict（含 _kind 标记）。"""
    if hasattr(ir_obj, '__dataclass_fields__'):
        result = {"_kind": type(ir_obj).__name__}
        for field in ir_obj.__dataclass_fields__:
            val = getattr(ir_obj, field)
            if isinstance(val, list):
                result[field] = [_ir_to_dict(v) for v in val]
            elif isinstance(val, dict):
                result[field] = {k: _ir_to_dict(v) for k, v in val.items()}
            elif hasattr(val, '__dataclass_fields__') or isinstance(val, (str, int, float, bool, type(None))):
                result[field] = _ir_to_dict(val) if hasattr(val, '__dataclass_fields__') else val
            else:
                result[field] = str(val) if not isinstance(val, (dict, list)) else val
        return result
    return ir_obj


def _validate_component_ir(d: dict):
    assert d["_kind"] == "ComponentIR"
    assert isinstance(d["name"], str)
    assert isinstance(d["states"], list)
    assert isinstance(d["computeds"], list)
    assert isinstance(d["view"], (dict, type(None)))


def _validate_element_ir(d: dict):
    assert d["_kind"] == "ElementIR"
    assert isinstance(d["tag"], str)
    assert isinstance(d["props"], dict)
    assert isinstance(d["children"], list)
    for child in d["children"]:
        _validate_ir_object(child)


def _validate_ir_object(d: dict) -> bool:
    """Generic validation: every IR object must have _kind."""
    if isinstance(d, dict):
        assert "_kind" in d, f"IR object missing _kind: {d}"
        for v in d.values():
            _validate_ir_object(v)
    elif isinstance(d, list):
        for item in d:
            _validate_ir_object(item)
    return True
```

---

### 7.2 Python IR Reader 测试（不依赖 Rust）

**文件:** `omniview/tests/test_ir_reader.py`

**进入条件:** `ir_reader.py` 写完  
**通过标准:** 25+ tests

```python
# omniview/tests/test_ir_reader.py

import json
import pytest
from omniview.codegen.ir_reader import IRReader

# ── 从实际 Python 管道生成的 IR JSON fixture ──

@pytest.fixture
def counter_ir_json():
    """用 Python 管道编译 Counter.omni → IR → JSON string."""
    from omniview.api import compile_omni
    # 使用内部 API 拿到中间 IR
    import omniview.preprocessor as pp
    from omniview.lexer import create_omniview_parser
    from omniview.parser import OmniViewTransformer
    from omniview.semantic.analyzer import SemanticAnalyzer
    from omniview.lowering import IRFixture
    
    source = open("build/test/counter/Counter.omni").read()
    pre = pp.preprocess(source)
    tree = create_omniview_parser().parse(pre.processed)
    ast = OmniViewTransformer(pre.block_map).transform(tree)
    analysis = SemanticAnalyzer().analyze(ast)
    ir_map = IRFixture().lower_program(ast)
    # 序列化为 JSON dict（模拟 Rust 输出）
    return {
        "version": "1.0",
        "components": {
            "Counter": _ir_to_dict(ir_map["Counter"])
        },
        "stores": {},
        "source_map": {},
    }


# ── 基础读取测试 ──

def test_read_valid_ir(counter_ir_json):
    reader = IRReader(counter_ir_json)
    assert reader.is_valid
    assert reader.errors == []


def test_read_version(counter_ir_json):
    reader = IRReader(counter_ir_json)
    comps = reader.components()
    assert "Counter" in comps


def test_read_component_name(counter_ir_json):
    reader = IRReader(counter_ir_json)
    comp = reader.components()["Counter"]
    assert comp["name"] == "Counter"


def test_read_states(counter_ir_json):
    reader = IRReader(counter_ir_json)
    comp = reader.components()["Counter"]
    assert len(comp["states"]) >= 1
    assert comp["states"][0]["name"] == "count"


def test_read_view(counter_ir_json):
    reader = IRReader(counter_ir_json)
    comp = reader.components()["Counter"]
    assert comp["view"] is not None
    assert comp["view"]["_kind"] in ("ElementIR",)


# ── 来源测试（JSON string vs dict） ──

def test_accept_json_string(counter_ir_json):
    json_str = json.dumps(counter_ir_json)
    reader = IRReader(json_str)
    assert reader.is_valid


def test_accept_dict(counter_ir_json):
    reader = IRReader(counter_ir_json)
    assert reader.is_valid


# ── 错误处理测试 ──

def test_invalid_version():
    reader = IRReader({"version": "0.5", "components": {}})
    assert not reader.is_valid
    assert any("version" in e.lower() for e in reader.errors)


def test_empty_ir():
    reader = IRReader({})
    assert not reader.is_valid
    assert len(reader.errors) > 0


def test_missing_components_and_stores():
    reader = IRReader({"version": "1.0"})
    assert not reader.is_valid


def test_invalid_json_string():
    reader = IRReader("not valid json")
    assert not reader.is_valid
    assert any("json" in e.lower() for e in reader.errors)


# ── Stores 测试 ──

def test_stores_empty(counter_ir_json):
    reader = IRReader(counter_ir_json)
    assert reader.stores() == {}


def test_stores_with_data():
    ir = {
        "version": "1.0",
        "components": {},
        "stores": {
            "CartStore": {
                "_kind": "StoreIR",
                "name": "CartStore",
                "signals": {"items": {"kind": "Array", "type_args": [{"kind": "String"}]}},
                "signal_defaults": {"items": []},
                "getters": {},
                "actions": [],
            }
        },
        "source_map": {},
    }
    reader = IRReader(ir)
    assert reader.is_valid
    stores = reader.stores()
    assert "CartStore" in stores
    assert stores["CartStore"]["name"] == "CartStore"


# ── Source Map 测试 ──

def test_source_map(counter_ir_json):
    reader = IRReader(counter_ir_json)
    sm = reader.source_map()
    assert isinstance(sm, dict)


# ── ir_kind 静态方法测试 ──

def test_ir_kind():
    assert IRReader.ir_kind({"_kind": "ElementIR"}) == "ElementIR"
    assert IRReader.ir_kind({"_kind": "StateIntent"}) == "StateIntent"
    assert IRReader.ir_kind({}) == "unknown"
    assert IRReader.ir_kind("not a dict") == "unknown"
```

---

### 7.3 Python Codegen with Dict 测试（不依赖 Rust）

**文件:** 每个 codegen 的已有测试文件（修改）
**新增:** `omniview/tests/test_codegen_from_dict.py`

**进入条件:** 对应 codegen 已改为接受 dict 输入  
**通过标准:** 输出与 Python 原生管道的输出**逐字符一致**

```python
# omniview/tests/test_codegen_from_dict.py

import pytest

@pytest.mark.parametrize("target,test_file", [
    ("react", "build/test/counter/Counter.omni"),
    ("react", "build/test/shopping-cart/ShoppingCart.omni"),
    ("vue", "build/test/counter/Counter.omni"),
    ("flutter", "build/test/counter/Counter.omni"),
])
def test_codegen_dict_matches_dataclass(target, test_file):
    """同一个 .omni 文件：Python 原生管道输出 vs dict 输入输出 → 完全一致."""
    from omniview.api import compile_omni
    
    source = open(test_file).read()
    
    # Path A: Python 原生管道（dataclass）
    result_a = compile_omni(source, target=target)
    assert result_a.success, f"Python pipeline errors: {result_a.errors}"
    
    # Path B: 拿到 IR，转 dict，直接调 codegen（跳过 parser/semantic）
    ir_map = _get_ir_from_python_pipeline(source)  # 用 Python 管道拿到 IR
    ir_dict = _ir_to_dict(ir_map)                  # 转成 dict
    
    codegen = _get_codegen(target)
    result_b = codegen.generate_component(ir_dict[list(ir_dict.keys())[0]])
    
    # 逐文件、逐字符对比
    for filename in result_a.files:
        assert filename in result_b.files, f"Missing file: {filename}"
        a = result_a.files[filename].strip()
        b = result_b.files[filename].strip()
        if a != b:
            # 产生 diff
            import difflib
            diff = "\n".join(difflib.unified_diff(
                a.splitlines(), b.splitlines(),
                fromfile=f"python-pipeline/{filename}",
                tofile=f"dict-pipeline/{filename}",
            ))
            pytest.fail(f"Output mismatch for {filename}:\n{diff}")


def test_codegen_handles_empty_component():
    """空组件 → codegen 能正常处理而不是 crash."""
    from omniview.codegen.react import ReactCodegen
    
    empty_comp = {
        "_kind": "ComponentIR",
        "name": "Empty",
        "props": {},
        "states": [],
        "computeds": [],
        "effects": [],
        "lifecycles": [],
        "fns": [],
        "store_refs": [],
        "refs": [],
        "view": None,
        "styles": [],
    }
    codegen = ReactCodegen()
    result = codegen.generate_component(empty_comp)
    assert result.files  # 至少生成一个文件
    assert "Empty" in result.files.get("Empty.tsx", "")


def test_codegen_handles_complex_shopping_cart():
    """ShoppingCart 完整 IR dict → React codegen → 与 Python 管道一致."""
    # 同上结构，以 ShoppingCart.omni 为输入
    pass


def test_all_codegens_accept_dict_format():
    """5 个 codegen 都能接受 dict 格式的 ComponentIR."""
    from omniview.codegen.react import ReactCodegen
    from omniview.codegen.vue import VueCodegen
    from omniview.codegen.flutter import FlutterCodegen
    from omniview.codegen.dioxus import DioxusCodegen
    from omniview.codegen.omniview import OmniViewCodegen as OmniVCodegen
    
    minimal_comp = {
        "_kind": "ComponentIR",
        "name": "Test",
        "props": {},
        "states": [{
            "_kind": "StateIntent",
            "name": "count",
            "type": {"kind": "i32"},
            "initial_value": {"_kind": "LiteralExpr", "value": 0},
            "mutability": "mutable",
            "update_semantics": "setter-call",
            "accessor_style": "direct",
        }],
        "computeds": [],
        "effects": [],
        "lifecycles": [],
        "fns": [],
        "store_refs": [],
        "refs": [],
        "view": None,
        "styles": [],
    }
    
    for codegen_cls in [ReactCodegen, VueCodegen, FlutterCodegen, DioxusCodegen, OmniVCodegen]:
        result = codegen_cls().generate_component(minimal_comp)
        assert result.files  # 每个 codegen 都应产出文件
```

---

### 7.4 RustBridge 可用性测试（不依赖 Rust）

**文件:** `omniview/tests/test_rust_bridge.py`

**进入条件:** `rust_bridge.py` 写完  
**通过标准:** 在 Rust 不可用时优雅降级，不抛异常

```python
# omniview/tests/test_rust_bridge.py

import pytest
from omniview.codegen.rust_bridge import RustBridge

def test_is_available_returns_bool():
    """Rust 编译器不在时返回 False，不抛异常."""
    result = RustBridge.is_available()
    assert isinstance(result, bool)


def test_compile_to_ir_raises_when_not_available():
    """Rust 不可用时 compile_to_ir 应抛出 RuntimeError."""
    if not RustBridge.is_available():
        with pytest.raises(RuntimeError, match="not available"):
            RustBridge.compile_to_ir("component App { view {} }")


def test_compile_to_ir_success_when_available():
    """Rust 可用时 compile_to_ir 返回有效的 IR dict."""
    if RustBridge.is_available():
        result = RustBridge.compile_to_ir("component App { view {} }")
        assert isinstance(result, dict)
        assert "components" in result or "stores" in result
```

---

### 7.5 双管道对比测试（依赖 Rust DLL）

**文件:** `omniview/tests/test_dual_pipeline.py`

**进入条件:** RustBridge.is_available() == True  
**通过标准:** 25+ tests，覆盖所有 test .omni 文件

```python
# omniview/tests/test_dual_pipeline.py

import pytest
from pathlib import Path

TEST_FILES = [
    "build/test/counter/Counter.omni",
    "build/test/shopping-cart/ShoppingCart.omni",
    "build/test/task-manager/TaskManager.omni",
]

@pytest.fixture(autouse=True)
def require_rust():
    from omniview.codegen.rust_bridge import RustBridge
    if not RustBridge.is_available():
        pytest.skip("Rust compiler not available")


@pytest.mark.parametrize("omni_file", TEST_FILES)
def test_ir_output_matches(omni_file):
    """Rust IR JSON 与 Python IR dataclass→dict 结构完全一致."""
    source = Path(omni_file).read_text(encoding="utf-8")
    
    # Path A: Python 管道 → IR dataclass → dict
    python_ir = _python_pipeline_to_dict(source)
    
    # Path B: Rust 管道 → IR JSON → dict
    from omniview.codegen.rust_bridge import RustBridge
    rust_ir = RustBridge.compile_to_ir(source)
    
    # 对比：递归检查所有字段
    _assert_ir_equal(python_ir, rust_ir, path="")


@pytest.mark.parametrize("target", ["react", "vue", "flutter", "dioxus"])
@pytest.mark.parametrize("omni_file", TEST_FILES[:1])  # 只用 Counter 减少时间
def test_codegen_output_matches(target, omni_file):
    """同一 .omni 文件：Rust IR → Python codegen vs Python 全管道 → 输出一致."""
    source = Path(omni_file).read_text(encoding="utf-8")
    
    from omniview.api import compile_omni
    from omniview.codegen.rust_bridge import RustBridge
    from omniview.codegen.react import ReactCodegen
    # ... (按 target 取对应 codegen)
    
    # Path A: Python 全管道
    result_a = compile_omni(source, target=target)
    assert result_a.success
    
    # Path B: Rust IR → Python codegen
    rust_ir = RustBridge.compile_to_ir(source)
    codegen = _get_codegen(target)
    result_b = codegen.generate_component(rust_ir["components"]["Counter"])
    
    for filename in result_a.files:
        assert result_a.files[filename] == result_b.files[filename], \
            f"{target}/{omni_file}: {filename} differs"


def test_semantic_errors_same():
    """有错误的 .omni 文件：Rust 和 Python 报告相同数量 + 相同位置的错误."""
    broken_sources = [
        # 未声明变量
        ("component App { fn foo() { bar } view {} }", "Undeclared"),
        # 类型不匹配
        ('component App { signal count: i32 = "hello"; view {} }', "Type mismatch"),
        # @if 条件不是 bool
        ('component App { signal name: i32 = 0; view { @if name <Text/> } }', "type"),
    ]
    
    for source, expected_error_type in broken_sources:
        python_errors = _get_python_diagnostics(source)
        rust_errors = _get_rust_diagnostics(source)
        
        assert len(python_errors) == len(rust_errors), \
            f"Error count differs for source: {source}\n" \
            f"Python: {python_errors}\nRust: {rust_errors}"
        
        # 至少错误类型一致
        for pe, re in zip(python_errors, rust_errors):
            assert pe["severity"] == re["severity"]


def test_round_trip_benchmark():
    """Rust 编译速度 < Python 编译速度."""
    import time
    
    source = Path("build/test/counter/Counter.omni").read_text(encoding="utf-8")
    
    # Python
    from omniview.api import compile_omni
    t0 = time.time()
    for _ in range(100):
        compile_omni(source, target="react")
    python_time = (time.time() - t0) / 100
    
    # Rust
    from omniview.codegen.rust_bridge import RustBridge
    t0 = time.time()
    for _ in range(100):
        RustBridge.compile_to_ir(source)
    rust_time = (time.time() - t0) / 100
    
    print(f"Python: {python_time*1000:.1f}ms  Rust: {rust_time*1000:.1f}ms")
    assert rust_time < python_time, \
        f"Rust ({rust_time*1000:.1f}ms) should be faster than Python ({python_time*1000:.1f}ms)"


# ── 辅助函数 ──

def _assert_ir_equal(a, b, path=""):
    """Recursively compare two IR dicts, reporting field path on mismatch."""
    if isinstance(a, dict) and isinstance(b, dict):
        # _kind 必须一致
        if "_kind" in a or "_kind" in b:
            assert a.get("_kind") == b.get("_kind"), \
                f"_kind mismatch at {path}: {a.get('_kind')} != {b.get('_kind')}"
        all_keys = set(a.keys()) | set(b.keys())
        for key in sorted(all_keys):
            _assert_ir_equal(a.get(key), b.get(key), f"{path}.{key}")
    elif isinstance(a, list) and isinstance(b, list):
        assert len(a) == len(b), f"List length mismatch at {path}: {len(a)} != {len(b)}"
        for i, (ai, bi) in enumerate(zip(a, b)):
            _assert_ir_equal(ai, bi, f"{path}[{i}]")
    elif a != b:
        # 允许 int/float 互转（JSON 里 0 可能是 int，也可能来自 float）
        if isinstance(a, (int, float)) and isinstance(b, (int, float)):
            assert abs(float(a) - float(b)) < 1e-9, \
                f"Numeric mismatch at {path}: {a} != {b}"
        else:
            assert a == b, f"Value mismatch at {path}: {repr(a)} != {repr(b)}"


def _get_python_diagnostics(source: str) -> list[dict]:
    """Run Python pipeline and return diagnostics."""
    from omniview.api import compile_omni
    result = compile_omni(source, target="react")
    return result.errors


def _get_rust_diagnostics(source: str) -> list[dict]:
    """Run Rust pipeline and return diagnostics."""
    from omniview.codegen.rust_bridge import RustBridge
    result = RustBridge.compile_to_ir(source)
    return result.get("_rust_errors", [])
```

---

### 7.6 Rust 侧测试（Rust cargo test）

**文件:** `omni-compiler/tests/*.rs`

| Test Suite | What It Tests | 关键用例 |
|-----------|---------------|---------|
| `lexer_tests.rs` | All token kinds, edge cases | 空文件、unicode 标识符、16 进制数字、`@if` 整体识别、模板字符串嵌套 |
| `parser_tests.rs` | Each grammar rule → correct AST | Counter.omni 全解析、ShoppingCart.omni 全解析、每个 stmt 类型、每个 expr 类型、嵌套 view 元素、7 种 prop 类型、错误恢复 |
| `semantic_tests.rs` | Symbol resolution, type checking | 未声明变量报错、类型不匹配报错、@for 迭代器非 Array 报错、@if 条件非 bool 报错、循环引用检测 |
| `ir_tests.rs` | AST → IR lowering | StateIntent 生成、ComputedIntent deps、ConditionalIntent branches、ListRenderingIntent key、EventHandler kind dispatch |
| `codegen_tests.rs` | OmniView JS/HTML/CSS output | Counter 输出、signal 声明→JS、view 元素→HTML、style 规则→CSS、SourceMap 条目 |
| `sourcemap_tests.rs` | SourceMap 正确性 | node_id 对应正确的 .omni 行号、target_spans 覆盖所有输出文件、DOM/JS/CSS span kind 正确 |

---

### 7.7 Round-Trip 集成测试

**操作流程（自动化脚本）:**

```bash
#!/bin/bash
# tools/test-round-trip.sh
# 全自动：.omni → Rust → IR → Python codegen → 对比 Python 原生管道

set -e

TEST_FILES=(
  "build/test/counter/Counter.omni"
  "build/test/shopping-cart/ShoppingCart.omni"
  "build/test/task-manager/TaskManager.omni"
)

TARGETS=("react" "vue" "flutter" "dioxus")

for file in "${TEST_FILES[@]}"; do
  echo "=== Testing $file ==="
  
  for target in "${TARGETS[@]}"; do
    echo "  Target: $target"
    
    # Python 原生管道
    python -c "
import sys; sys.path.insert(0, '.')
from omniview.api import compile_omni
src = open('$file').read()
r = compile_omni(src, target='$target')
assert r.success, f'Python pipeline failed: {r.errors}'
open('/tmp/py_output.json', 'w').write(str(r.files))
" 2>/dev/null
    
    # Rust → Python codegen
    python -c "
import sys; sys.path.insert(0, '.')
from omniview.codegen.rust_bridge import RustBridge
from omniview.codegen.react import ReactCodegen  # placeholder; dispatch by target
src = open('$file').read()
ir = RustBridge.compile_to_ir(src)
# ... dispatch codegen
open('/tmp/rust_output.json', 'w').write(str(result.files))
" 2>/dev/null
    
    # Diff
    diff <(python -c "import ast; print(sorted(ast.literal_eval(open('/tmp/py_output.json').read()).items()))") \
         <(python -c "import ast; print(sorted(ast.literal_eval(open('/tmp/rust_output.json').read()).items()))")
  done
done

echo "✓ All round-trip tests passed"
```

---

### 7.8 测试阶段依赖关系

```
Phase 0 — 不依赖 Rust:
  ✓ test_ir_schema.py          (7.1)  进入条件: ir_schema.py 写完
  ✓ test_ir_reader.py          (7.2)  进入条件: ir_reader.py 写完
  ✓ test_codegen_from_dict.py  (7.3)  进入条件: codegen 改为接受 dict
  ✓ test_rust_bridge.py        (7.4)  进入条件: rust_bridge.py 写完

Phase 1 — 依赖 Rust DLL:
  ✓ test_dual_pipeline.py      (7.5)  进入条件: RustBridge.is_available()
  ✓ Rust cargo test            (7.6)  进入条件: cargo build 通过
  ✓ round-trip.sh              (7.7)  进入条件: Phase 0 + Rust DLL
```

**先在 Phase 0 把所有 Python 侧测试跑绿（完全不依赖 Rust），再进入 Phase 1 接入真实 Rust 编译器。**

---

## 8. Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|-----------|
| Hand-written parser has bugs missed by tests | Incorrect IR → wrong codegen | Round-trip verification against Python output as ground truth |
| IR JSON serialization diverges from Python IR dataclasses | Python codegen can't consume | Single source of truth: Python IR types define schema; Rust mirrors exactly |
| WASM binary too large (>1MB) | Slow page load | `wasm-opt` + LTO + strip. Target: <500KB gzipped |
| Semantic analysis passes incomplete | Missed errors, confusing UX | Port Python test cases for each pass; compare diagnostic counts |
| Performance regression vs Python | Slower compile for large projects | Benchmark early: WASM compile for Counter.omni should be <10ms |

---

## Appendix A: Complete IR Type Definitions

Mirrors `omniview/ir_types.py` exactly. See that file for full definitions. Key types:

```
ComponentIR    — Complete component intent
StoreIR        — Global state container
StateIntent    — Reactive state declaration (signal)
ComputedIntent — Derived/computed value
EffectIntent   — Side effect driven by deps
LifecycleIntent— Mount/unmount hooks
ConditionalIntent — @if/@else array
ListRenderingIntent — @for iteration
BindIntent     — Two-way data binding
EventIntent    — Event binding on element
SlotIntent     — Content projection slot
ElementIR      — Abstract element in view tree
```

## Appendix B: Decisions Log

| # | Date | Decision | Rationale |
|---|------|----------|-----------|
| 1 | 2026-06-15 | Hand-written Pratt parser + recursive descent | Full control, WASM-friendly |
| 2 | 2026-06-15 | IR as JSON | Human-readable, debuggable |
| 3 | 2026-06-15 | Cover omnilang + omniview | Unified parser |
| 4 | 2026-06-15 | OmniView target: inline mini-signal | Zero deps in iframe |
| 5 | 2026-06-15 | WASM as static asset | No server compilation needed |
| 6 | 2026-06-15 | Errors as JSON array | Direct editor rendering |
| 7 | 2026-06-15 | Architecture reserves incremental API | v0.1 full compile; v0.2+ incremental |
| 8 | 2026-06-15 | First milestone: IR JSON + Python React | End-to-end verification |
| 9 | 2026-06-15 | 80/20 grammar coverage | Ship fast; extensible design |
| 10 | 2026-06-15 | Rust as lib crate (.wasm + .dll/.so) | Multi-platform build from single codebase |
