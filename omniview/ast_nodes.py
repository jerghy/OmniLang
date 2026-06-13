"""AST node definitions for OmniView (frontend component DSL).

All nodes are frozen dataclasses with optional source spans.
OmniView is a frontend component DSL that compiles to Intent IR.
These nodes represent the parsed structure of a .omni file.

Reuses expression types from omnilang.ast_nodes.
"""

from __future__ import annotations

from dataclasses import dataclass

from omnilang.ast_nodes import (
    Expr,
    Literal,
    VarRef,
    BinaryOp,
    UnaryOp,
    FuncCall,
    MethodCall,
    IndexGet,
    ArrayLiteral,
    MapLiteral,
    IfExpr,
    TemplateString,
    AwaitExpr,
    TypeAnnotation,
    UnionType,
    GenericParam,
)
from omnilang.utils import Span


# ── Shared AST Nodes ──────────────────────────────────

@dataclass(frozen=True)
class Param:
    """A function or event handler parameter."""
    name: str
    type: TypeAnnotation | None = None
    span: Span | None = None


@dataclass(frozen=True)
class Stmt:
    """Base class for OmniView statements."""
    pass


@dataclass(frozen=True)
class Block:
    """A block of statements: { stmt1; stmt2; ... }"""
    statements: tuple[Stmt, ...] = ()
    span: Span | None = None


# ── Import ─────────────────────────────────────────────

@dataclass(frozen=True)
class ImportStmt:
    """Cross-file import: import { A, B } from "./module.omni" """
    names: tuple[str, ...] = ()
    path: str = ""
    span: Span | None = None


# ── Top-Level ─────────────────────────────────────────

@dataclass(frozen=True)
class Program:
    """Top-level program containing components, stores, and themes."""
    components: tuple[ComponentDef, ...] = ()
    stores: tuple[StoreDef, ...] = ()
    themes: tuple[ThemeDef, ...] = ()
    imports: tuple[ImportStmt, ...] = ()
    lib_imports: tuple[str, ...] = ()
    span: Span | None = None

    @property
    def has_omniview(self) -> bool:
        return "omniview" in self.lib_imports


# ── Component Definition ──────────────────────────────

@dataclass(frozen=True)
class PropDecl:
    """A component prop declaration."""
    name: str
    type: TypeAnnotation
    default_value: Expr | None = None
    span: Span | None = None


@dataclass(frozen=True)
class SignalDecl:
    """A component signal (local reactive state) declaration."""
    name: str
    type: TypeAnnotation
    initial_value: Expr
    span: Span | None = None


@dataclass(frozen=True)
class ComputedDecl:
    """A computed value derived from signals."""
    name: str
    type: TypeAnnotation
    body: Expr
    span: Span | None = None


@dataclass(frozen=True)
class EventDecl:
    """A component event declaration."""
    name: str
    payload_type: TypeAnnotation
    is_optional: bool = False
    span: Span | None = None


@dataclass(frozen=True)
class RefDecl:
    """A template ref declaration (P2+)."""
    name: str
    ref_type: TypeAnnotation
    span: Span | None = None


@dataclass(frozen=True)
class StoreRef:
    """@store StoreName declaration in a component."""
    store_name: str
    span: Span | None = None


@dataclass(frozen=True)
class ComponentDef:
    """A complete component definition."""
    name: str
    generic_params: tuple[str, ...] = ()  # <T, U> generic type parameters
    props: tuple[PropDecl, ...] = ()
    signals: tuple[SignalDecl, ...] = ()
    computeds: tuple[ComputedDecl, ...] = ()
    events: tuple[EventDecl, ...] = ()
    refs: tuple[RefDecl, ...] = ()
    stores: tuple[StoreRef, ...] = ()
    lifecycles: tuple[LifecycleHook, ...] = ()
    fns: tuple[FnDef, ...] = ()
    view: ViewBlock | None = None
    style: StyleBlock | None = None
    span: Span | None = None


# ── Store Definition ──────────────────────────────────

@dataclass(frozen=True)
class StoreSignalDecl:
    """A store-level signal declaration."""
    name: str
    type: TypeAnnotation
    initial_value: Expr
    span: Span | None = None


@dataclass(frozen=True)
class StoreGetterDecl:
    """A store getter (derived value)."""
    name: str
    type: TypeAnnotation
    body: Expr
    span: Span | None = None


@dataclass(frozen=True)
class StoreActionDecl:
    """A store action (state-mutating function)."""
    name: str
    params: tuple[Param, ...] = ()
    return_type: TypeAnnotation | None = None
    body: Block | None = None
    is_async: bool = False
    span: Span | None = None


@dataclass(frozen=True)
class StoreDef:
    """A store definition (reactive state container)."""
    name: str
    signals: tuple[StoreSignalDecl, ...] = ()
    getters: tuple[StoreGetterDecl, ...] = ()
    actions: tuple[StoreActionDecl, ...] = ()
    span: Span | None = None


# ── Event Handlers ────────────────────────────────────

@dataclass(frozen=True)
class InlineFn:
    """An inline function: fn() { body } or fn(p: T) { body }"""
    params: tuple[Param, ...] = ()
    body: Block | None = None
    span: Span | None = None


@dataclass(frozen=True)
class FnRef:
    """A reference to a component fn: handleClick"""
    fn_name: str
    span: Span | None = None


@dataclass(frozen=True)
class StoreActionRef:
    """A reference to a store action: store.add"""
    action_path: str
    span: Span | None = None


EventHandler = InlineFn | FnRef | StoreActionRef


# ── Class Values ──────────────────────────────────────

@dataclass(frozen=True)
class StyleRefClass:
    """class={style.card}"""
    class_name: str
    span: Span | None = None


@dataclass(frozen=True)
class StringClass:
    """class="flex gap-4" — raw string, Web-only"""
    value: str
    span: Span | None = None


@dataclass(frozen=True)
class ClassMapClass:
    """class={class_map({active: is_active, disabled: !enabled})}"""
    entries: tuple[tuple[str, Expr], ...] = ()
    span: Span | None = None


ClassValue = StyleRefClass | StringClass | ClassMapClass


# ── Element Prop Values ───────────────────────────────

@dataclass(frozen=True)
class StaticProp:
    """A static prop: placeholder="Enter" or variant={expr}"""
    value: Expr | str
    span: Span | None = None


@dataclass(frozen=True)
class BindProp:
    """Two-way binding: bind:value={signal_name}"""
    signal_ref: str
    span: Span | None = None


@dataclass(frozen=True)
class EventProp:
    """An event handler prop."""
    handler: EventHandler
    span: Span | None = None


@dataclass(frozen=True)
class RefProp:
    """ref={name} — DOM node reference."""
    ref_name: str = ""
    span: Span | None = None


@dataclass(frozen=True)
class ClassProp:
    """CSS class binding prop."""
    class_value: ClassValue
    span: Span | None = None


@dataclass(frozen=True)
class StyleProp:
    """Inline style: style={{color: "red", flex: 1}}"""
    properties: tuple[tuple[str, Expr], ...] = ()
    span: Span | None = None


ElementPropValue = StaticProp | BindProp | EventProp | ClassProp | StyleProp


# ── View Tree ─────────────────────────────────────────

@dataclass(frozen=True)
class ElementProp:
    """A single property on an element."""
    name: str
    value: ElementPropValue
    span: Span | None = None


@dataclass(frozen=True)
class Element:
    """An abstract element: <Button onPress={fn}>Click</Button>"""
    tag: str
    props: tuple[ElementProp, ...] = ()
    children: tuple[ViewNode, ...] = ()
    test_id: str | None = None
    span: Span | None = None


@dataclass(frozen=True)
class ViewBlock:
    """The view { ... } body of a component."""
    children: tuple[ViewNode, ...] = ()
    span: Span | None = None


# ── Directives ────────────────────────────────────────

@dataclass(frozen=True)
class IfBranch:
    """A single @if branch."""
    condition: Expr
    body: tuple[ViewNode, ...] = ()
    span: Span | None = None


@dataclass(frozen=True)
class IfDirective:
    """@if cond { ... } [@else if cond { ... }]* [@else { ... }]"""
    branches: tuple[IfBranch, ...] = ()
    else_body: tuple[ViewNode, ...] | None = None
    span: Span | None = None


@dataclass(frozen=True)
class ForDirective:
    """@for item in iterable { body } [@empty { empty_body }]"""
    item_name: str
    iterable: Expr
    body: tuple[ViewNode, ...] = ()
    empty_body: tuple[ViewNode, ...] | None = None
    span: Span | None = None


@dataclass(frozen=True)
class SlotDirective:
    """@slot or @slot name="name" — slot definition point"""
    name: str | None = None
    span: Span | None = None


@dataclass(frozen=True)
class NativeBlock:
    """@native(target) { raw_content }"""
    target: str
    raw_content: str
    span: Span | None = None


@dataclass(frozen=True)
class FallbackBlock:
    """@fallback { content }"""
    children: tuple[ViewNode, ...] = ()
    span: Span | None = None


@dataclass(frozen=True)
class ErrorBoundary:
    """@error(name) { fallback content } — cross-framework error boundary."""
    name: str = ""
    children: tuple[ViewNode, ...] = ()
    span: Span | None = None


# ── Lifecycle ─────────────────────────────────────────

@dataclass(frozen=True)
class LifecycleHook:
    """@mount / @unmount / @effect"""
    kind: str  # 'mount' | 'unmount' | 'effect'
    body: Block | None = None
    deps: tuple[str, ...] | None = None
    span: Span | None = None


# ── Style & Theme ─────────────────────────────────────

@dataclass(frozen=True)
class StyleRule:
    """.className { property: value; }"""
    class_name: str
    properties: tuple[tuple[str, Expr], ...] = ()
    span: Span | None = None


@dataclass(frozen=True)
class StyleBlock:
    """style { .className { ... } }"""
    rules: tuple[StyleRule, ...] = ()
    span: Span | None = None


@dataclass(frozen=True)
class ThemeCategory:
    """A category within a theme: colors { primary: '#1890ff'; }"""
    name: str
    values: tuple[tuple[str, str], ...] = ()
    span: Span | None = None


@dataclass(frozen=True)
class ThemeDef:
    """theme { category { key: value; } }"""
    categories: tuple[ThemeCategory, ...] = ()
    span: Span | None = None


# ── Functions ─────────────────────────────────────────

@dataclass(frozen=True)
class FnDef:
    """A component function definition."""
    name: str
    params: tuple[Param, ...] = ()
    return_type: TypeAnnotation | None = None
    body: Block | None = None
    is_async: bool = False
    span: Span | None = None


# ── OmniView-Specific Expressions ─────────────────────

@dataclass(frozen=True)
class FieldAccess(Expr):
    """Field access: obj.field"""
    object: Expr
    field: str
    span: Span | None = None


@dataclass(frozen=True)
class TernaryExpr(Expr):
    """Ternary: condition ? then_expr : else_expr"""
    condition: Expr
    then_expr: Expr
    else_expr: Expr
    span: Span | None = None


@dataclass(frozen=True)
class ClassMapExpr(Expr):
    """class_map({active: is_active, disabled: !enabled})"""
    entries: tuple[tuple[str, Expr], ...] = ()
    span: Span | None = None


@dataclass(frozen=True)
class StyleObjectExpr(Expr):
    """Inline style object: {color: "red", flex: 1}"""
    properties: tuple[tuple[str, Expr], ...] = ()
    span: Span | None = None


# ── Statements ────────────────────────────────────────

@dataclass(frozen=True)
class VarDeclStmt(Stmt):
    """let name[: Type] = expr;"""
    name: str
    type: TypeAnnotation | None = None
    value: Expr | None = None
    is_mutable: bool = False
    span: Span | None = None


@dataclass(frozen=True)
class AssignStmt(Stmt):
    """target = expr; | target += expr;"""
    target: str
    op: str = "="
    value: Expr | None = None
    span: Span | None = None


@dataclass(frozen=True)
class IfStmt(Stmt):
    """if expr { ... } [else { ... }]"""
    condition: Expr
    then_body: Block
    else_body: Block | None = None
    span: Span | None = None


@dataclass(frozen=True)
class WhileStmt(Stmt):
    """while expr { ... }"""
    condition: Expr
    body: Block
    span: Span | None = None


@dataclass(frozen=True)
class ForInStmt(Stmt):
    """for item in iterable { ... }"""
    item_name: str
    iterable: Expr
    body: Block
    span: Span | None = None


@dataclass(frozen=True)
class ReturnStmt(Stmt):
    """return expr;"""
    value: Expr | None = None
    span: Span | None = None


@dataclass(frozen=True)
class ExprStmt(Stmt):
    """Expression used as a statement."""
    expr: Expr | None = None
    span: Span | None = None


@dataclass(frozen=True)
class EventEmitStmt(Stmt):
    """event event_name(args);"""
    event_name: str
    args: tuple[Expr, ...] = ()
    span: Span | None = None


# ── Text Content ──────────────────────────────────────

@dataclass(frozen=True)
class TextNode:
    """Plain text content within an element."""
    content: str
    span: Span | None = None


@dataclass(frozen=True)
class Interpolation:
    """Interpolated expression: {count}"""
    expr: Expr
    span: Span | None = None


# ── Type Aliases (defined after all constituent types) ─

ViewNode = Element | IfDirective | ForDirective | SlotDirective | NativeBlock | FallbackBlock | TextNode | Interpolation

TextContent = TextNode | Interpolation


# ── Exports ───────────────────────────────────────────

__all__ = [
    # Re-exports from omnilang.ast_nodes
    "Expr", "Literal", "VarRef", "BinaryOp", "UnaryOp",
    "FuncCall", "MethodCall", "IndexGet",
    "ArrayLiteral", "MapLiteral", "IfExpr",
    "TemplateString", "AwaitExpr",
    "TypeAnnotation", "UnionType", "GenericParam",
    # Shared
    "Param", "Stmt", "Block",
    # Import
    "ImportStmt",
    # Top-level
    "Program",
    # Component
    "ComponentDef", "PropDecl", "SignalDecl", "ComputedDecl",
    "EventDecl", "RefDecl", "StoreRef",
    # Store
    "StoreDef", "StoreSignalDecl", "StoreGetterDecl", "StoreActionDecl",
    # Event handlers
    "InlineFn", "FnRef", "StoreActionRef",
    # Class values
    "StyleRefClass", "StringClass", "ClassMapClass",
    # Element props
    "StaticProp", "BindProp", "EventProp", "ClassProp", "StyleProp",
    "ElementProp",
    # View tree
    "ViewBlock", "Element",
    # Directives
    "IfBranch", "IfDirective", "ForDirective",
    "SlotDirective", "NativeBlock", "FallbackBlock",
    # Lifecycle
    "LifecycleHook",
    # Style & theme
    "StyleBlock", "StyleRule", "ThemeDef", "ThemeCategory",
    # Functions
    "FnDef",
    # Expressions
    "FieldAccess", "TernaryExpr", "ClassMapExpr", "StyleObjectExpr",
    # Statements
    "VarDeclStmt", "AssignStmt", "IfStmt", "WhileStmt", "ForInStmt",
    "ReturnStmt", "ExprStmt", "EventEmitStmt",
    # Text
    "TextNode", "Interpolation",
    # Type aliases
    "ViewNode",
]
