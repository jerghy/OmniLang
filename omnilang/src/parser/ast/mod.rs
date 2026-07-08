//! AST node types for the OmniLang parser.
//!
//! All AST nodes defined per spec §3.3.1. The parser produces a Program from token stream.
//! Types follow the Python AST structure with Rust idioms (Box for recursion, enum dispatch).

pub mod design;
pub mod motion;
pub mod view;
pub use design::*;
pub use motion::*;
pub use view::*;

use rustc_hash::FxHashMap;
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
    OptionalChain {
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

    /// Block expression: { stmts; trailing_expr }
    Block {
        body: Block,
        span: Span,
    },

    /// Closure / lambda expression: |params| body
    ClosureExpr {
        params: Vec<String>,
        body: Box<Expr>,
        span: Span,
    },

    /// await expression: await <expr>
    AwaitExpr {
        operand: Box<Expr>,
        span: Span,
    },

    /// match expression: match expr { Pattern => body, ... }
    MatchExpr {
        matched: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },

    /// Spread expression: ...expr (array/object spread)
    SpreadExpr {
        operand: Box<Expr>,
        span: Span,
    },

    /// Reserved for P2 features; parser emits this instead of crashing
    NotYetImplemented {
        feature: String,
        span: Span,
    },

    /// Placeholder for a syntax error in an expression.
    /// Downstream passes should skip or propagate this node.
    Error {
        message: String,
        span: Span,
    },

    /// Type check: value is TypeName
    IsCheck {
        value: Box<Expr>,
        type_name: String,
        span: Span,
    },
}

#[derive(Debug, Clone)]
pub enum Literal {
    Int(u64),
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
    NullishCoalesce,
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    ModAssign,
}

impl BinaryOpKind {
    pub fn as_ir_str(&self) -> &'static str {
        match self {
            BinaryOpKind::Add => "add",
            BinaryOpKind::Sub => "sub",
            BinaryOpKind::Mul => "mul",
            BinaryOpKind::Div => "div",
            BinaryOpKind::Mod => "mod",
            BinaryOpKind::Eq => "eq",
            BinaryOpKind::Neq => "neq",
            BinaryOpKind::Lt => "lt",
            BinaryOpKind::Gt => "gt",
            BinaryOpKind::LtEq => "lteq",
            BinaryOpKind::GtEq => "gteq",
            BinaryOpKind::And => "and",
            BinaryOpKind::Or => "or",
            BinaryOpKind::NullishCoalesce => "??",
            BinaryOpKind::Assign => "assign",
            BinaryOpKind::AddAssign => "addassign",
            BinaryOpKind::SubAssign => "subassign",
            BinaryOpKind::MulAssign => "mulassign",
            BinaryOpKind::DivAssign => "divassign",
            BinaryOpKind::ModAssign => "modassign",
        }
    }
}

#[derive(Debug, Clone)]
pub enum UnaryOpKind {
    Not,
    Neg,
    Await,
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
    Throw {
        expr: Expr,
        span: Span,
    },
    TryCatch {
        try_body: Block,
        catch_var: Option<String>,
        catch_body: Option<Block>,
        finally_body: Option<Block>,
        span: Span,
    },

    /// Placeholder for a statement that could not be parsed.
    Error {
        message: String,
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

// ── Match Expression ──────────────────────────────────

/// A single arm in a match expression: Pattern => body
#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: MatchPattern,
    pub body: Block,
}

/// A pattern in a match arm: Variant { bindings } or Wildcard
#[derive(Debug, Clone)]
pub struct MatchPattern {
    pub kind: MatchPatternKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum MatchPatternKind {
    /// Variant pattern: Ok { value, response }
    Variant { name: String, bindings: Vec<String> },
    /// Wildcard pattern: _
    Wildcard,
    /// Literal pattern: 42, "hello", true (basic support)
    Literal(Literal),
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

// ── Visibility ────────────────────────────────────────

/// Visibility modifier for declarations.
pub use crate::ir::visibility::Visibility;

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

// ── Module & Program ──────────────────────────────────

/// A single source module — corresponds to one .omni file.
#[derive(Debug, Clone, Default)]
pub struct Module {
    pub name: String,
    pub functions: Vec<FunctionDef>,
    pub structs: Vec<StructDef>,
    pub tagged_unions: Vec<TaggedUnionDef>,
    pub type_aliases: Vec<TypeAlias>,
    pub components: Vec<ComponentDef>,
    pub stores: Vec<StoreDef>,
    pub themes: Vec<ThemeDef>,
    pub designs: Vec<DesignDef>,
    pub imports: Vec<ImportStmt>,
    pub exports: Vec<ExportFrom>,
    pub motions: Vec<MotionDef>,
    pub timelines: Vec<TimelineDef>,
}

#[derive(Debug, Clone, Default)]
pub struct Program {
    pub modules: FxHashMap<String, Module>,
}
