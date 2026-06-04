use crate::span::Span;
use std::sync::atomic::{AtomicU64, Ordering};

// ── Node ID System ──────────────────────────────────────────

pub type NodeId = u64;

static NEXT_NODE_ID: AtomicU64 = AtomicU64::new(1);

pub fn new_node_id() -> NodeId {
    NEXT_NODE_ID.fetch_add(1, Ordering::Relaxed)
}

pub fn reset_node_ids() {
    NEXT_NODE_ID.store(1, Ordering::Relaxed);
}

// ── Type Representations ────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum TypeRepr {
    Named {
        node_id: NodeId,
        name: String,
        type_args: Vec<TypeRepr>,
    },
    Union {
        node_id: NodeId,
        types: Vec<TypeRepr>,
    },
    Fn {
        node_id: NodeId,
        param_types: Vec<TypeRepr>,
        return_type: Box<TypeRepr>,
    },
}

impl TypeRepr {
    pub fn named(name: &str) -> Self {
        TypeRepr::Named {
            node_id: new_node_id(),
            name: name.to_string(),
            type_args: vec![],
        }
    }

    pub fn named_with_args(name: &str, type_args: Vec<TypeRepr>) -> Self {
        TypeRepr::Named {
            node_id: new_node_id(),
            name: name.to_string(),
            type_args,
        }
    }

    pub fn union(types: Vec<TypeRepr>) -> Self {
        TypeRepr::Union {
            node_id: new_node_id(),
            types,
        }
    }

    pub fn node_id(&self) -> NodeId {
        match self {
            TypeRepr::Named { node_id, .. } => *node_id,
            TypeRepr::Union { node_id, .. } => *node_id,
            TypeRepr::Fn { node_id, .. } => *node_id,
        }
    }

    pub fn base_name(&self) -> &str {
        match self {
            TypeRepr::Named { name, .. } => name,
            TypeRepr::Union { .. } => "",
            TypeRepr::Fn { .. } => "fn",
        }
    }

    pub fn is_copy_type(&self) -> bool {
        match self {
            TypeRepr::Named { name, .. } => matches!(name.as_str(), "i32" | "f64" | "bool"),
            _ => false,
        }
    }

    pub fn is_string(&self) -> bool {
        matches!(self, TypeRepr::Named { name, .. } if name == "String")
    }

    pub fn unwrap_optional(&self) -> &TypeRepr {
        match self {
            TypeRepr::Union { types, .. } => {
                for t in types {
                    if let TypeRepr::Named { name, .. } = t {
                        if name != "null" {
                            return t;
                        }
                    }
                }
                &types[0]
            }
            _ => self,
        }
    }
}

impl std::fmt::Display for TypeRepr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeRepr::Named { name, type_args, .. } => {
                write!(f, "{}", name)?;
                if !type_args.is_empty() {
                    write!(f, "<")?;
                    for (i, arg) in type_args.iter().enumerate() {
                        if i > 0 { write!(f, ", ")?; }
                        write!(f, "{}", arg)?;
                    }
                    write!(f, ">")?;
                }
                Ok(())
            }
            TypeRepr::Union { types, .. } => {
                for (i, t) in types.iter().enumerate() {
                    if i > 0 { write!(f, " | ")?; }
                    write!(f, "{}", t)?;
                }
                Ok(())
            }
            TypeRepr::Fn { param_types, return_type, .. } => {
                write!(f, "fn(")?;
                for (i, p) in param_types.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", p)?;
                }
                write!(f, ") -> {}", return_type)
            }
        }
    }
}

// ── Operator Kinds ──────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOpKind {
    Add, Sub, Mul, Div, Mod,
    Eq, Ne, Lt, Gt, Le, Ge,
    And, Or,
}

impl BinaryOpKind {
    pub fn from_str(op: &str) -> Self {
        match op {
            "+" => Self::Add, "-" => Self::Sub, "*" => Self::Mul,
            "/" => Self::Div, "%" => Self::Mod,
            "==" => Self::Eq, "!=" => Self::Ne,
            "<" => Self::Lt, ">" => Self::Gt,
            "<=" => Self::Le, ">=" => Self::Ge,
            "&&" => Self::And, "||" => Self::Or,
            _ => panic!("Unknown binary operator: {}", op),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Add => "+", Self::Sub => "-", Self::Mul => "*",
            Self::Div => "/", Self::Mod => "%",
            Self::Eq => "==", Self::Ne => "!=",
            Self::Lt => "<", Self::Gt => ">",
            Self::Le => "<=", Self::Ge => ">=",
            Self::And => "&&", Self::Or => "||",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompoundOp {
    Add, Sub, Mul, Div, Mod,
}

impl CompoundOp {
    pub fn from_str(op: &str) -> Self {
        match op {
            "+=" => Self::Add, "-=" => Self::Sub,
            "*=" => Self::Mul, "/=" => Self::Div, "%=" => Self::Mod,
            _ => panic!("Unknown compound operator: {}", op),
        }
    }
}

// ── AST Node Types ──────────────────────────────────────────

// Forward declarations for Box references
#[derive(Debug, Clone)]
pub enum Expr {
    Literal(LiteralNode),
    VarRef(VarRefNode),
    BinaryOp(BinaryOpNode),
    UnaryOp(UnaryOpNode),
    FuncCall(FuncCallNode),
    MethodCall(MethodCallNode),
    IndexGet(IndexGetNode),
    ArrayLiteral(ArrayLiteralNode),
    MapLiteral(MapLiteralNode),
    IfExpr(IfExprNode),
    TemplateString(TemplateStringNode),
    OptionalChain(OptionalChainNode),
    NullCoalesce(NullCoalesceNode),
    ClosureExpr(ClosureExprNode),
    AwaitExpr(AwaitExprNode),
    SpreadExpr(SpreadExprNode),
    StructLiteral(StructLiteralNode),
    MatchExpr(MatchExprNode),
}

impl Expr {
    pub fn node_id(&self) -> NodeId {
        match self {
            Expr::Literal(n) => n.node_id,
            Expr::VarRef(n) => n.node_id,
            Expr::BinaryOp(n) => n.node_id,
            Expr::UnaryOp(n) => n.node_id,
            Expr::FuncCall(n) => n.node_id,
            Expr::MethodCall(n) => n.node_id,
            Expr::IndexGet(n) => n.node_id,
            Expr::ArrayLiteral(n) => n.node_id,
            Expr::MapLiteral(n) => n.node_id,
            Expr::IfExpr(n) => n.node_id,
            Expr::TemplateString(n) => n.node_id,
            Expr::OptionalChain(n) => n.node_id,
            Expr::NullCoalesce(n) => n.node_id,
            Expr::ClosureExpr(n) => n.node_id,
            Expr::AwaitExpr(n) => n.node_id,
            Expr::SpreadExpr(n) => n.node_id,
            Expr::StructLiteral(n) => n.node_id,
            Expr::MatchExpr(n) => n.node_id,
        }
    }

    pub fn span(&self) -> &Span {
        match self {
            Expr::Literal(n) => &n.span,
            Expr::VarRef(n) => &n.span,
            Expr::BinaryOp(n) => &n.span,
            Expr::UnaryOp(n) => &n.span,
            Expr::FuncCall(n) => &n.span,
            Expr::MethodCall(n) => &n.span,
            Expr::IndexGet(n) => &n.span,
            Expr::ArrayLiteral(n) => &n.span,
            Expr::MapLiteral(n) => &n.span,
            Expr::IfExpr(n) => &n.span,
            Expr::TemplateString(n) => &n.span,
            Expr::OptionalChain(n) => &n.span,
            Expr::NullCoalesce(n) => &n.span,
            Expr::ClosureExpr(n) => &n.span,
            Expr::AwaitExpr(n) => &n.span,
            Expr::SpreadExpr(n) => &n.span,
            Expr::StructLiteral(n) => &n.span,
            Expr::MatchExpr(n) => &n.span,
        }
    }
}

// ── Expression Nodes ────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct LiteralNode {
    pub node_id: NodeId,
    pub span: Span,
    pub value: LiteralValue,
    pub kind: String, // "i32", "f64", "String", "bool"
}

#[derive(Debug, Clone, PartialEq)]
pub enum LiteralValue {
    Int(i32),
    Float(f64),
    Str(String),
    Bool(bool),
}

#[derive(Debug, Clone)]
pub struct VarRefNode {
    pub node_id: NodeId,
    pub span: Span,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct BinaryOpNode {
    pub node_id: NodeId,
    pub span: Span,
    pub left: Box<Expr>,
    pub op: BinaryOpKind,
    pub right: Box<Expr>,
}

#[derive(Debug, Clone)]
pub struct UnaryOpNode {
    pub node_id: NodeId,
    pub span: Span,
    pub op: UnaryOpKind,
    pub operand: Box<Expr>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOpKind {
    Neg, Not,
}

#[derive(Debug, Clone)]
pub struct FuncCallNode {
    pub node_id: NodeId,
    pub span: Span,
    pub func: Box<Expr>,
    pub args: Vec<Expr>,
}

#[derive(Debug, Clone)]
pub struct MethodCallNode {
    pub node_id: NodeId,
    pub span: Span,
    pub object: Box<Expr>,
    pub method: String,
    pub args: Vec<Expr>,
}

#[derive(Debug, Clone)]
pub struct IndexGetNode {
    pub node_id: NodeId,
    pub span: Span,
    pub object: Box<Expr>,
    pub index: Box<Expr>,
}

#[derive(Debug, Clone)]
pub struct ArrayLiteralNode {
    pub node_id: NodeId,
    pub span: Span,
    pub elements: Vec<Expr>,
}

#[derive(Debug, Clone)]
pub struct MapLiteralNode {
    pub node_id: NodeId,
    pub span: Span,
    pub entries: Vec<(Expr, Expr)>, // (key, value)
}

#[derive(Debug, Clone)]
pub struct IfExprNode {
    pub node_id: NodeId,
    pub span: Span,
    pub condition: Box<Expr>,
    pub then_expr: Box<Expr>,
    pub else_expr: Box<Expr>,
}

#[derive(Debug, Clone)]
pub struct TemplateStringNode {
    pub node_id: NodeId,
    pub span: Span,
    pub parts: Vec<TemplatePart>,
}

#[derive(Debug, Clone)]
pub struct TemplatePart {
    pub kind: String, // "literal" or "expr"
    pub value: TemplatePartValue,
}

#[derive(Debug, Clone)]
pub enum TemplatePartValue {
    Literal(String),
    Expr(Box<Expr>),
}

#[derive(Debug, Clone)]
pub struct OptionalChainNode {
    pub node_id: NodeId,
    pub span: Span,
    pub object: Box<Expr>,
    pub chain: Vec<ChainLink>,
}

#[derive(Debug, Clone)]
pub struct ChainLink {
    pub kind: String, // "field", "index", "call"
    pub name: Option<String>,
    pub index: Option<Box<Expr>>,
    pub args: Vec<Expr>,
}

#[derive(Debug, Clone)]
pub struct NullCoalesceNode {
    pub node_id: NodeId,
    pub span: Span,
    pub left: Box<Expr>,
    pub right: Box<Expr>,
}

#[derive(Debug, Clone)]
pub struct ClosureExprNode {
    pub node_id: NodeId,
    pub span: Span,
    pub params: Vec<ParamNode>,
    pub return_type: Option<TypeRepr>,
    pub body: Box<ClosureBody>,
    pub is_arrow: bool,
}

#[derive(Debug, Clone)]
pub enum ClosureBody {
    Block(BlockNode),
    Expr(Expr),
}

#[derive(Debug, Clone)]
pub struct AwaitExprNode {
    pub node_id: NodeId,
    pub span: Span,
    pub expr: Box<Expr>,
}

#[derive(Debug, Clone)]
pub struct SpreadExprNode {
    pub node_id: NodeId,
    pub span: Span,
    pub expr: Box<Expr>,
}

#[derive(Debug, Clone)]
pub struct StructLiteralNode {
    pub node_id: NodeId,
    pub span: Span,
    pub type_name: String,
    pub fields: Vec<(String, Expr)>,
}

#[derive(Debug, Clone)]
pub struct MatchExprNode {
    pub node_id: NodeId,
    pub span: Span,
    pub scrutinee: Box<Expr>,
    pub arms: Vec<MatchArmNode>,
}

#[derive(Debug, Clone)]
pub struct MatchArmNode {
    pub node_id: NodeId,
    pub span: Span,
    pub pattern: MatchPattern,
    pub guard: Option<Box<Expr>>,
    pub body: Box<MatchArmBody>,
}

#[derive(Debug, Clone)]
pub enum MatchArmBody {
    Expr(Expr),
    Block(BlockNode),
}

// ── Pattern Nodes ───────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum MatchPattern {
    Ignore,
    Wildcard,
    Var(String),
    Literal(LiteralValue),
    Struct(StructPatternNode),
    Array(ArrayPatternNode),
    Or(Vec<MatchPattern>),
}

#[derive(Debug, Clone)]
pub struct StructPatternNode {
    pub name: String,
    pub fields: Vec<(String, MatchPattern)>,
}

#[derive(Debug, Clone)]
pub struct ArrayPatternNode {
    pub elements: Vec<MatchPattern>,
    pub rest: Option<String>,
}

// ── Statement Nodes ─────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Stmt {
    VarDecl(VarDeclNode),
    Destructure(DestructureNode),
    Assign(AssignNode),
    IfStmt(IfStmtNode),
    WhileStmt(WhileStmtNode),
    ForInStmt(ForInStmtNode),
    BreakStmt(BreakStmtNode),
    ContinueStmt(ContinueStmtNode),
    ReturnStmt(ReturnStmtNode),
    PrintStmt(PrintStmtNode),
    ExprStmt(ExprStmtNode),
}

impl Stmt {
    pub fn span(&self) -> &Span {
        match self {
            Stmt::VarDecl(n) => &n.span,
            Stmt::Destructure(n) => &n.span,
            Stmt::Assign(n) => &n.span,
            Stmt::IfStmt(n) => &n.span,
            Stmt::WhileStmt(n) => &n.span,
            Stmt::ForInStmt(n) => &n.span,
            Stmt::BreakStmt(n) => &n.span,
            Stmt::ContinueStmt(n) => &n.span,
            Stmt::ReturnStmt(n) => &n.span,
            Stmt::PrintStmt(n) => &n.span,
            Stmt::ExprStmt(n) => &n.span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BlockNode {
    pub node_id: NodeId,
    pub span: Span,
    pub stmts: Vec<Stmt>,
    pub trailing_expr: Option<Box<Expr>>,
}

#[derive(Debug, Clone)]
pub struct VarDeclNode {
    pub node_id: NodeId,
    pub span: Span,
    pub mutable: bool,
    pub name: String,
    pub type_ann: Option<TypeRepr>,
    pub init: Expr,
}

#[derive(Debug, Clone)]
pub struct DestructureNode {
    pub node_id: NodeId,
    pub span: Span,
    pub mutable: bool,
    pub pattern: DestructurePattern,
    pub init: Expr,
}

#[derive(Debug, Clone)]
pub enum DestructurePattern {
    Array(Vec<DestructureElement>, Option<String>), // elements, optional rest var name
    Object(Vec<(String, String)>), // (field_name, bind_name)
}

#[derive(Debug, Clone)]
pub enum DestructureElement {
    Bind(String),
    Ignore,
}

#[derive(Debug, Clone)]
pub struct AssignNode {
    pub node_id: NodeId,
    pub span: Span,
    pub target: String,
    pub index: Option<Box<Expr>>,
    pub op: String, // "=" | "+=" | "-=" | "*=" | "/=" | "%="
    pub value: Expr,
}

#[derive(Debug, Clone)]
pub struct IfStmtNode {
    pub node_id: NodeId,
    pub span: Span,
    pub condition: Expr,
    pub then_body: BlockNode,
    pub else_ifs: Vec<(Expr, BlockNode)>,
    pub else_body: Option<BlockNode>,
}

#[derive(Debug, Clone)]
pub struct WhileStmtNode {
    pub node_id: NodeId,
    pub span: Span,
    pub condition: Expr,
    pub body: BlockNode,
}

#[derive(Debug, Clone)]
pub struct ForInStmtNode {
    pub node_id: NodeId,
    pub span: Span,
    pub var_name: String,
    pub iterable: Expr,
    pub body: BlockNode,
}

#[derive(Debug, Clone)]
pub struct BreakStmtNode {
    pub node_id: NodeId,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ContinueStmtNode {
    pub node_id: NodeId,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ReturnStmtNode {
    pub node_id: NodeId,
    pub span: Span,
    pub value: Option<Box<Expr>>,
}

#[derive(Debug, Clone)]
pub struct PrintStmtNode {
    pub node_id: NodeId,
    pub span: Span,
    pub newline: bool,
    pub arg: Expr,
}

#[derive(Debug, Clone)]
pub struct ExprStmtNode {
    pub node_id: NodeId,
    pub span: Span,
    pub expr: Expr,
}

// ── Top-Level Nodes ─────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ParamNode {
    pub node_id: NodeId,
    pub span: Span,
    pub name: String,
    pub type_ann: Option<TypeRepr>,
    pub is_rest: bool,
}

#[derive(Debug, Clone)]
pub struct StructFieldNode {
    pub node_id: NodeId,
    pub span: Span,
    pub name: String,
    pub type_ann: TypeRepr,
    pub default: Option<Box<Expr>>,
}

#[derive(Debug, Clone)]
pub struct VariantNode {
    pub node_id: NodeId,
    pub span: Span,
    pub name: String,
    pub fields: Vec<StructFieldNode>,
}

#[derive(Debug, Clone)]
pub struct TaggedUnionDefNode {
    pub node_id: NodeId,
    pub span: Span,
    pub name: String,
    pub generic_params: Vec<GenericParamNode>,
    pub variants: Vec<VariantNode>,
}

#[derive(Debug, Clone)]
pub struct StructDefNode {
    pub node_id: NodeId,
    pub span: Span,
    pub name: String,
    pub generic_params: Vec<GenericParamNode>,
    pub fields: Vec<StructFieldNode>,
    pub methods: Vec<FunctionDefNode>,
    pub is_nominal: bool,
}

#[derive(Debug, Clone)]
pub struct FunctionDefNode {
    pub node_id: NodeId,
    pub span: Span,
    pub name: String,
    pub params: Vec<ParamNode>,
    pub return_type: Option<TypeRepr>,
    pub generic_params: Vec<GenericParamNode>,
    pub body: Option<BlockNode>,
    pub is_async: bool,
}

#[derive(Debug, Clone)]
pub struct GenericParamNode {
    pub node_id: NodeId,
    pub span: Span,
    pub name: String,
    pub bounds: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ClassDefNode {
    pub node_id: NodeId,
    pub span: Span,
    pub name: String,
    pub extends: Option<String>,
    pub implements: Vec<String>,
    pub members: Vec<ClassMemberNode>,
}

#[derive(Debug, Clone)]
pub struct ClassMemberNode {
    pub node_id: NodeId,
    pub span: Span,
    pub name: String,
    pub type_ann: Option<TypeRepr>,
    pub func_def: Option<Box<FunctionDefNode>>,
    pub is_static: bool,
    pub visibility: String, // "public" | "private"
    pub init: Option<Box<Expr>>,
    pub is_override: bool,
}

#[derive(Debug, Clone)]
pub struct InterfaceMemberNode {
    pub node_id: NodeId,
    pub span: Span,
    pub name: String,
    pub type_ann: Option<TypeRepr>,
    pub params: Vec<ParamNode>,
    pub return_type: Option<TypeRepr>,
    pub is_method: bool,
}

#[derive(Debug, Clone)]
pub struct InterfaceDefNode {
    pub node_id: NodeId,
    pub span: Span,
    pub name: String,
    pub members: Vec<InterfaceMemberNode>,
}

#[derive(Debug, Clone)]
pub struct ImportStmtNode {
    pub node_id: NodeId,
    pub span: Span,
    pub names: Vec<(String, Option<String>)>, // (name, optional alias)
    pub path: String,
    pub is_type_import: bool,
}

#[derive(Debug, Clone)]
pub struct ExportStmtNode {
    pub node_id: NodeId,
    pub span: Span,
    pub item: Box<TopLevelItem>,
}

// ── Top-Level Items ─────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum TopLevelItem {
    Function(FunctionDefNode),
    Struct(StructDefNode),
    TaggedUnion(TaggedUnionDefNode),
    Class(ClassDefNode),
    Interface(InterfaceDefNode),
    Import(ImportStmtNode),
    Export(ExportStmtNode),
}

#[derive(Debug, Clone)]
pub struct ImplBlock {
    pub node_id: NodeId,
    pub span: Span,
    pub struct_name: String,
    pub generic_params: Vec<GenericParamNode>,
    pub methods: Vec<FunctionDefNode>,
}

// ── Program ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Program {
    pub node_id: NodeId,
    pub span: Span,
    pub functions: Vec<FunctionDefNode>,
    pub structs: Vec<StructDefNode>,
    pub tagged_unions: Vec<TaggedUnionDefNode>,
    pub classes: Vec<ClassDefNode>,
    pub interfaces: Vec<InterfaceDefNode>,
    pub impl_blocks: Vec<ImplBlock>,
    pub imports: Vec<ImportStmtNode>,
    pub exports: Vec<ExportStmtNode>,
}

impl Program {
    pub fn new() -> Self {
        Self {
            node_id: new_node_id(),
            span: Span::dummy(),
            functions: vec![],
            structs: vec![],
            tagged_unions: vec![],
            classes: vec![],
            interfaces: vec![],
            impl_blocks: vec![],
            imports: vec![],
            exports: vec![],
        }
    }

    /// Attach impl block methods to their corresponding structs.
    pub fn attach_impl_methods(&mut self) {
        let mut struct_methods: std::collections::HashMap<String, Vec<FunctionDefNode>> =
            std::collections::HashMap::new();

        for impl_block in &self.impl_blocks {
            struct_methods
                .entry(impl_block.struct_name.clone())
                .or_default()
                .extend(impl_block.methods.clone());
        }

        for s in &mut self.structs {
            if let Some(methods) = struct_methods.remove(&s.name) {
                s.methods.extend(methods);
            }
        }
    }
}

impl Default for Program {
    fn default() -> Self {
        Self::new()
    }
}
