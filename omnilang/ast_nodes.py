"""AST node definitions for OmniLang. All nodes are frozen dataclasses with source spans.

v0.2: Extended from 20+ nodes to 40+ nodes covering template strings,
optional chains, destructuring, structs, union types, and more.
"""

from dataclasses import dataclass, field
from omnilang.utils import Span


# ── Types ──────────────────────────────────────────────

@dataclass(frozen=True)
class TypeAnnotation:
    """A simple named type, optionally with generic type arguments.

    Examples: i32, String, Array<i32>, Map<String, i32>
    """
    name: str  # 'i32'|'f64'|'bool'|'String'|'Array'|'Map'|'void'|'null'|user-type
    type_args: tuple['TypeAnnotation', ...] = ()
    span: Span | None = None

    def __repr__(self):
        if self.type_args:
            args = ", ".join(repr(a) for a in self.type_args)
            return f"{self.name}<{args}>"
        return self.name


@dataclass(frozen=True)
class UnionType:
    """A union type: T | U | V.

    Created by the parser from `T | U` syntax or `T?` sugar (expands to T | null).
    """
    types: list[TypeAnnotation] = field(default_factory=list)
    span: Span | None = None

    def __repr__(self):
        return " | ".join(repr(t) for t in self.types)


@dataclass(frozen=True)
class GenericParam:
    """A generic type parameter, optionally with trait bounds.

    Examples: T, T + Display, U + Clone + Eq
    """
    name: str
    bounds: list[str] = field(default_factory=list)  # trait names
    span: Span | None = None

    def __repr__(self):
        if self.bounds:
            return f"{self.name}: {' + '.join(self.bounds)}"
        return self.name


# Type alias: anything that can appear in a type position
TypeLike = TypeAnnotation | UnionType


# ── Expressions ───────────────────────────────────────

@dataclass(frozen=True)
class Expr:
    """Base class for all expression nodes."""
    pass


@dataclass(frozen=True)
class Literal(Expr):
    value: int | float | str | bool
    kind: str                       # 'i32' | 'f64' | 'String' | 'bool'
    span: Span | None = None


@dataclass(frozen=True)
class VarRef(Expr):
    name: str
    span: Span | None = None


@dataclass(frozen=True)
class BinaryOp(Expr):
    left: Expr
    op: str                        # + - * / % == != < > <= >= && || ??
    right: Expr
    span: Span | None = None


@dataclass(frozen=True)
class UnaryOp(Expr):
    op: str                        # '-' | '!'
    operand: Expr
    span: Span | None = None


@dataclass(frozen=True)
class FuncCall(Expr):
    func: Expr                     # the function expression (typically a VarRef)
    args: list[Expr] = field(default_factory=list)
    span: Span | None = None


@dataclass(frozen=True)
class MethodCall(Expr):
    object: Expr
    method: str
    args: list[Expr] = field(default_factory=list)
    span: Span | None = None


@dataclass(frozen=True)
class IndexGet(Expr):
    object: Expr
    index: Expr
    span: Span | None = None


@dataclass(frozen=True)
class ArrayLiteral(Expr):
    elements: list[Expr] = field(default_factory=list)
    span: Span | None = None


@dataclass(frozen=True)
class MapLiteral(Expr):
    entries: list[tuple[Expr, Expr]] = field(default_factory=list)  # [(key, value), ...]
    span: Span | None = None


@dataclass(frozen=True)
class IfExpr(Expr):
    condition: Expr
    then_expr: Expr
    else_expr: Expr
    span: Span | None = None


# ── v0.2 New Expressions ──────────────────────────────

@dataclass(frozen=True)
class TemplatePart:
    """A part of a template string: either literal text or an interpolated expression."""
    kind: str       # "literal" | "expr"
    value: str | Expr
    span: Span | None = None


@dataclass(frozen=True)
class TemplateString(Expr):
    """A template string literal: `Hello, ${name}!`"""
    parts: list[TemplatePart] = field(default_factory=list)
    span: Span | None = None


@dataclass(frozen=True)
class ChainLink:
    """A single link in an optional chain: field access or index access."""
    kind: str       # "field" | "index" | "call"
    name: str = ""  # field name or method name
    index: Expr | None = None
    args: list[Expr] = field(default_factory=list)  # for method calls
    span: Span | None = None


@dataclass(frozen=True)
class OptionalChain(Expr):
    """Optional chaining expression: obj?.field or obj?.method()"""
    object: Expr
    chain: list[ChainLink] = field(default_factory=list)
    span: Span | None = None


@dataclass(frozen=True)
class NullCoalesce(Expr):
    """Null coalescing expression: left ?? right"""
    left: Expr
    right: Expr
    span: Span | None = None


@dataclass(frozen=True)
class ClosureExpr(Expr):
    """Closure / lambda expression: |a, b| a + b or (a, b) => a + b

    Represents both pipe-syntax closures and arrow-function closures.
    Arrow functions are syntax sugar that produce the same AST node.
    """
    params: list['Param'] = field(default_factory=list)
    return_type: 'TypeAnnotation | UnionType | None' = None
    body: Expr = None  # single expression (tail return) or Block for multi-statement
    is_arrow: bool = False  # True if source used (params) => syntax
    span: Span | None = None


@dataclass(frozen=True)
class FnType:
    """Function type annotation: fn(i32, String) -> bool"""
    param_types: list = field(default_factory=list)  # list[TypeAnnotation | UnionType]
    return_type: 'TypeAnnotation | None' = None
    span: Span | None = None

    def __repr__(self):
        pts = ", ".join(repr(t) for t in self.param_types)
        return f"fn({pts}) -> {self.return_type}"


@dataclass(frozen=True)
class AwaitExpr(Expr):
    """Await expression: await future"""
    expr: Expr
    span: Span | None = None


@dataclass(frozen=True)
class SpreadExpr(Expr):
    """Spread expression: ...expr"""
    expr: Expr
    span: Span | None = None


# ── Match expressions ──────────────────────────────────

@dataclass(frozen=True)
class MatchArm:
    """A single arm in a match expression: pattern => body"""
    pattern: 'Literal | VarPattern | IgnorePattern | StructPattern | OrPattern' = None
    guard: Expr | None = None  # optional 'if' guard
    body: Expr = None  # single expression or Block
    span: Span | None = None


@dataclass(frozen=True)
class MatchExpr(Expr):
    """Match expression: match value { arm1, arm2, ... }"""
    scrutinee: Expr
    arms: list[MatchArm] = field(default_factory=list)
    span: Span | None = None


@dataclass(frozen=True)
class StructPattern:
    """Pattern for matching struct variants: Some { value } or None"""
    name: str
    fields: list[tuple[str, 'MatchPattern']] = field(default_factory=list)
    span: Span | None = None


@dataclass(frozen=True)
class OrPattern:
    """Or-pattern: pattern1 | pattern2"""
    patterns: list['MatchPattern'] = field(default_factory=list)
    span: Span | None = None


# MatchPattern type alias — defined after all pattern classes below
# MatchPattern = Literal | VarPattern | IgnorePattern | StructPattern | OrPattern


@dataclass(frozen=True)
class StructLiteral(Expr):
    """Struct literal expression: TypeName { field: value, ... }"""
    type_name: str
    fields: list[tuple[str, Expr]] = field(default_factory=list)  # [(name, value), ...]
    span: Span | None = None


# ── Patterns (for destructuring) ──────────────────────

# NOTE: Patterns are standalone frozen dataclasses (no inheritance) to avoid
# the "non-default argument follows default argument" issue with frozen bases.

@dataclass(frozen=True)
class VarPattern:
    """A variable binding pattern: a in let [a, b] = arr;"""
    name: str
    span: Span | None = None


@dataclass(frozen=True)
class IgnorePattern:
    """Wildcard pattern: _"""
    span: Span | None = None


@dataclass(frozen=True)
class RestPattern:
    """Rest pattern: ...rest"""
    name: str
    span: Span | None = None


@dataclass(frozen=True)
class ArrayPattern:
    """Array destructuring pattern: [a, b, ...rest]"""
    elements: list['VarPattern | IgnorePattern | ArrayPattern'] = field(default_factory=list)
    rest: RestPattern | None = None
    span: Span | None = None


@dataclass(frozen=True)
class ObjectPattern:
    """Object destructuring pattern: {name, age}"""
    fields: list[str] = field(default_factory=list)
    span: Span | None = None


# Union type alias for pattern matching
Pattern = VarPattern | IgnorePattern | RestPattern | ArrayPattern | ObjectPattern


# ── Statements ────────────────────────────────────────

@dataclass(frozen=True)
class Stmt:
    """Base class for all statement nodes."""
    pass


@dataclass(frozen=True)
class Block(Stmt):
    stmts: list[Stmt] = field(default_factory=list)
    trailing_expr: Expr | None = None   # tail expression (no semicolon), used as implicit return
    span: Span | None = None


@dataclass(frozen=True)
class VarDecl(Stmt):
    mutable: bool
    name: str
    type_ann: TypeAnnotation | None = None
    init: Expr = None
    span: Span | None = None


@dataclass(frozen=True)
class Destructure(Stmt):
    """Destructuring statement: let [a, b] = expr; or let {x, y} = expr;"""
    pattern: Pattern
    init: Expr
    mutable: bool = False
    span: Span | None = None


@dataclass(frozen=True)
class Assign(Stmt):
    target: str
    index: Expr | None = None     # None = simple assignment, Some = indexed assignment
    op: str = '='                 # '=' | '+=' | '-=' | '*=' | '/=' | '%='
    value: Expr = None            # will be set after construction
    span: Span | None = None


@dataclass(frozen=True)
class IfStmt(Stmt):
    condition: Expr
    then_body: Block
    else_ifs: list[tuple[Expr, Block]] = field(default_factory=list)
    else_body: Block | None = None
    span: Span | None = None


@dataclass(frozen=True)
class WhileStmt(Stmt):
    condition: Expr
    body: Block
    span: Span | None = None


@dataclass(frozen=True)
class ForInStmt(Stmt):
    var_name: str
    iterable: Expr
    body: Block
    span: Span | None = None


@dataclass(frozen=True)
class BreakStmt(Stmt):
    span: Span | None = None


@dataclass(frozen=True)
class ContinueStmt(Stmt):
    span: Span | None = None


@dataclass(frozen=True)
class ReturnStmt(Stmt):
    value: Expr | None = None
    span: Span | None = None


@dataclass(frozen=True)
class PrintStmt(Stmt):
    newline: bool                  # True = println, False = print
    arg: Expr
    span: Span | None = None


@dataclass(frozen=True)
class ExprStmt(Stmt):
    expr: Expr                     # side-effecting expression (assignment, method call)
    span: Span | None = None


# ── Top-level ─────────────────────────────────────────

@dataclass(frozen=True)
class Param:
    name: str
    type_ann: TypeAnnotation | None = None
    is_rest: bool = False          # True for ...rest parameters
    span: Span | None = None


@dataclass(frozen=True)
class StructField:
    """A field in a struct definition."""
    name: str
    type_ann: TypeAnnotation  # simple type (no unions at field level)
    default: Expr | None = None
    span: Span | None = None


@dataclass(frozen=True)
class Variant:
    """A single variant in a tagged union definition."""
    name: str
    fields: list[StructField] = field(default_factory=list)
    span: Span | None = None


@dataclass(frozen=True)
class TaggedUnionDef:
    """Tagged union definition: type Option<T> = Some { value: T } | None"""
    name: str
    generic_params: list[GenericParam] = field(default_factory=list)
    variants: list[Variant] = field(default_factory=list)
    span: Span | None = None


@dataclass(frozen=True)
class StructDef:
    """Struct definition: struct Name { fields }"""
    name: str
    generic_params: list[GenericParam] = field(default_factory=list)
    fields: list[StructField] = field(default_factory=list)
    methods: list['FunctionDef'] = field(default_factory=list)  # from impl blocks
    is_nominal: bool = False  # True if declared with 'nominal struct'
    span: Span | None = None


@dataclass(frozen=True)
class FunctionDef:
    name: str
    params: list[Param] = field(default_factory=list)
    return_type: TypeAnnotation | UnionType | None = None
    generic_params: list[GenericParam] = field(default_factory=list)
    body: Block | None = None
    is_async: bool = False
    span: Span | None = None


@dataclass(frozen=True)
class ClassDef:
    """Class definition: class Name { ... }"""
    name: str = ""
    extends: str | None = None
    implements: list[str] = field(default_factory=list)
    members: list['ClassMember'] = field(default_factory=list)
    span: Span | None = None


@dataclass(frozen=True)
class ClassMember:
    """A member of a class: field or method."""
    name: str = ""
    type_ann: TypeAnnotation | None = None  # for fields
    func_def: FunctionDef | None = None  # for methods
    is_static: bool = False
    visibility: str = "public"  # "public" | "private"
    init: Expr | None = None  # default value
    is_override: bool = False
    span: Span | None = None


@dataclass
@dataclass(frozen=True)
class ImportStmt:
    """Import statement: import { A, B } from "./module" """
    names: list[str] = field(default_factory=list)
    path: str = ""
    span: Span | None = None


@dataclass(frozen=True)
class ExportStmt:
    """Export statement: export fn foo() { ... }"""
    item: FunctionDef | StructDef | TaggedUnionDef | ClassDef | None = None
    span: Span | None = None


@dataclass
class Program:
    functions: list[FunctionDef] = field(default_factory=list)
    structs: list[StructDef] = field(default_factory=list)
    tagged_unions: list[TaggedUnionDef] = field(default_factory=list)
    classes: list[ClassDef] = field(default_factory=list)
    span: Span | None = None
