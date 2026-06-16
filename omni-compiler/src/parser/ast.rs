//! AST node types for the OmniLang parser.
//!
//! All AST nodes defined per spec §3.3.1. The parser produces a Program from token stream.
//! Types follow the Python AST structure with Rust idioms (Box for recursion, enum dispatch).

use serde::{Deserialize, Serialize};

use crate::lexer::Span;

// ── Base types ────────────────────────────────────────

/// Type annotation — used in variable declarations, function signatures, etc.
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

// ── Expressions (omnilang + omniview) ─────────────────

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
    StoreFieldAccess {
        store_alias: String,
        field_name: String,
        span: Span,
    },

    /// Closure / lambda expression: |params| body
    ClosureExpr {
        params: Vec<String>,
        body: Box<Expr>,
        span: Span,
    },

    /// Reserved for P2 features; parser emits this instead of crashing
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
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Neq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    ModAssign,
}

#[derive(Debug, Clone)]
pub enum UnaryOpKind {
    Not,
    Neg,
}

#[derive(Debug, Clone)]
pub enum TemplatePart {
    Literal(String),
    Expr(Expr),
}

// ── Statements ────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Stmt {
    VarDecl {
        name: String,
        type_ann: Option<TypeAnnotation>,
        mutable: bool,
        init: Option<Expr>,
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
    Index { object: String, index: Box<Expr> },
    Field { object: String, field: String },
}

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub trailing_expr: Option<Box<Expr>>,
    pub span: Span,
}

// ── Top-Level Declarations ────────────────────────────

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub type_ann: Option<TypeAnnotation>,
}

#[derive(Debug, Clone)]
pub struct FunctionDef {
    pub visibility: Visibility,
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
    pub visibility: Visibility,
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

#[derive(Debug, Clone)]
pub struct TypeAlias {
    pub visibility: Visibility,
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub span: Span,
}

// ── OmniView-Specific AST ─────────────────────────────

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
    Mount,
    Unmount,
    Effect,
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
    Dimension(f64, String),
    HexColor(String),
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

// ── Store & Theme ─────────────────────────────────────

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

// ── Visibility ────────────────────────────────────────

/// Visibility modifier for declarations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    Private,
}

impl Default for Visibility {
    fn default() -> Self {
        Visibility::Private
    }
}

// ── Import / Export Types ─────────────────────────────

/// Named import: `import { A, B } from "./path.omni";`
#[derive(Debug, Clone)]
pub struct NamedImport {
    pub names: Vec<String>,
    pub path: String,
    pub span: Span,
}

/// Namespace import: `import "./path.omni" as name;`
#[derive(Debug, Clone)]
pub struct NamespaceImport {
    pub path: String,
    pub alias: String,
    pub span: Span,
}

/// Type-only import: `import type { A, B } from "./path.omni";`
#[derive(Debug, Clone)]
pub struct TypeImport {
    pub names: Vec<String>,
    pub path: String,
    pub span: Span,
}

/// Re-export: `export { A, B } from "./path.omni";`
#[derive(Debug, Clone)]
pub struct ExportFrom {
    pub names: Vec<String>,
    pub path: String,
    pub span: Span,
}

/// All import variants in one enum
#[derive(Debug, Clone)]
pub enum ImportStmt {
    Named(NamedImport),
    Namespace(NamespaceImport),
    TypeOnly(TypeImport),
    Lib(String), // "omniview" — bare import marker
}

// ── Top-Level Program ─────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct Program {
    pub components: Vec<ComponentDef>,
    pub stores: Vec<StoreDef>,
    pub themes: Vec<ThemeDef>,
    pub functions: Vec<FunctionDef>,
    pub structs: Vec<StructDef>,
    pub type_aliases: Vec<TypeAlias>,
    pub tagged_unions: Vec<TaggedUnionDef>,
    pub imports: Vec<ImportStmt>,
    pub exports: Vec<ExportFrom>,
}
