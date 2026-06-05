"""OmniLang bridge — parser cache, delegation helpers, and error position mapping.

Provides the API layer between OmniView and OmniLang. OmniView calls
these functions instead of parsing expressions/stmts itself.

Architecture:
    OmniView Transformer → omnilang_bridge → omnilang.parser
                                          → omnilang.parser_helpers
"""

from __future__ import annotations

import functools
from dataclasses import dataclass, field
from typing import Any

from omnilang.ast_nodes import (
    Expr, Literal, VarRef, BinaryOp, UnaryOp, FuncCall, MethodCall,
    ArrayLiteral, MapLiteral, IfExpr, TemplateString, SpreadExpr,
    Block, Stmt, ExprStmt, Assign, VarDecl, IfStmt, ReturnStmt,
    TypeAnnotation, UnionType,
)
from omnilang.utils import Span


# ═══════════════════════════════════════════════════════════
# Parser Cache
# ═══════════════════════════════════════════════════════════

@functools.lru_cache(maxsize=1)
def _get_cached_parser():
    """Return a cached Lark parser instance. Created once and reused."""
    from omnilang.lexer import create_parser
    return create_parser()


def _get_cached_transformer():
    """Return a cached OmniTransformer instance."""
    from omnilang.parser import OmniTransformer
    return OmniTransformer()


# ═══════════════════════════════════════════════════════════
# Expression / Block Parsing
# ═══════════════════════════════════════════════════════════

def parse_expr(expr_text: str) -> Expr:
    """Parse an inline expression from OmniView source.

    Delegates to OmniLang's parse_inline_expr. Handles the case where
    OmniView captures expression text from {expr} interpolation,
    attribute values, @if conditions, @for iterables, etc.

    Args:
        expr_text: Raw expression text (without surrounding braces)

    Returns:
        OmniLang Expr AST node. Falls back to Literal on parse error.
    """
    from omnilang.parser_helpers import parse_inline_expr
    return parse_inline_expr(expr_text)


def parse_block(block_text: str) -> Block:
    """Parse an inline statement block from OmniView source.

    Used for fn bodies, action bodies, lifecycle hook bodies.

    Args:
        block_text: Raw statement text (without surrounding braces)

    Returns:
        OmniLang Block AST node. Empty text returns Block(stmts=[]).
    """
    from omnilang.parser_helpers import parse_inline_block
    return parse_inline_block(block_text)


# ═══════════════════════════════════════════════════════════
# Expression Classification
# ═══════════════════════════════════════════════════════════

def is_store_action_call(expr: Expr) -> tuple[bool, str | None, str | None, list[Expr]]:
    """Check if an expression is a store action call like store.increment().

    Returns:
        (is_store_action, store_name, action_name, call_args)
    """
    if isinstance(expr, MethodCall):
        obj = expr.object
        if isinstance(obj, VarRef) and obj.name == "store":
            return True, "store", expr.method, list(expr.args)
    return False, None, None, []


def is_store_field_access(expr: Expr) -> tuple[bool, str | None, str | None]:
    """Check if an expression is a store field access like store.todos.

    Returns:
        (is_store_field, store_name, field_name)
    """
    if isinstance(expr, MethodCall) and not expr.args:
        obj = expr.object
        if isinstance(obj, VarRef) and obj.name == "store":
            return True, "store", expr.method
    return False, None, None


# ═══════════════════════════════════════════════════════════
# Expression Collection (for dependency analysis)
# ═══════════════════════════════════════════════════════════

def collect_var_refs(expr: Expr) -> list[str]:
    """Collect all variable names referenced in an expression tree.

    Used for dependency collection (computed deps, effect deps).
    Traverses the expression AST recursively.

    Args:
        expr: OmniLang Expr AST node

    Returns:
        List of variable names referenced (may contain duplicates for nested refs)
    """
    names: list[str] = []

    def walk(node):
        if isinstance(node, VarRef):
            names.append(node.name)
        elif isinstance(node, Literal):
            pass
        elif isinstance(node, BinaryOp):
            walk(node.left)
            walk(node.right)
        elif isinstance(node, UnaryOp):
            walk(node.operand)
        elif isinstance(node, MethodCall):
            walk(node.object)
            for a in node.args:
                walk(a)
        elif isinstance(node, FuncCall):
            walk(node.func)
            for a in node.args:
                walk(a)
        elif isinstance(node, ArrayLiteral):
            for e in node.elements:
                walk(e)
        elif isinstance(node, MapLiteral):
            for k, v in node.entries:
                walk(k)
                walk(v)
        elif isinstance(node, IfExpr):
            walk(node.condition)
            walk(node.then_expr)
            walk(node.else_expr)
        elif isinstance(node, SpreadExpr):
            walk(node.expr)
        elif isinstance(node, TemplateString):
            for part in node.parts:
                if hasattr(part, 'value') and isinstance(part.value, Expr):
                    walk(part.value)
        # OptionalChain, NullCoalesce, ClosureExpr, etc. handled minimally

    walk(expr)
    return names


# ═══════════════════════════════════════════════════════════
# Error Position Mapping
# ═══════════════════════════════════════════════════════════

@dataclass
class ExpressionSlot:
    """Records where an expression/block was extracted from in .omni source."""
    placeholder_id: str
    original_text: str
    omni_offset: int       # byte offset in .omni source
    omni_line: int          # 1-based line number in .omni source
    omni_col: int           # 1-based column number in .omni source
    kind: str               # "expr" | "block" | "event" | "style" | "classmap"


@dataclass
class PositionMap:
    """Maps placeholder IDs to their positions in the original .omni source."""
    slots: dict[str, ExpressionSlot] = field(default_factory=dict)

    def add(self, slot: ExpressionSlot) -> None:
        self.slots[slot.placeholder_id] = slot

    def map_span(self, placeholder_id: str, inner_span: Span | None) -> Span | None:
        """Map an OmniLang span (relative to embedded expression) to .omni source position."""
        if inner_span is None:
            return None
        slot = self.slots.get(placeholder_id)
        if slot is None:
            return inner_span
        return Span(
            start_line=slot.omni_line + max(0, inner_span.start_line - 1),
            start_col=slot.omni_col + max(0, inner_span.start_col - 1),
            end_line=slot.omni_line + max(0, inner_span.end_line - 1),
            end_col=slot.omni_col + max(0, inner_span.end_col - 1),
        )


# ═══════════════════════════════════════════════════════════
# Utility
# ═══════════════════════════════════════════════════════════

def expr_to_source(expr: Expr) -> str:
    """Best-effort reconstruction of expression source text from AST.
    Used for debugging and error messages.
    """
    if isinstance(expr, Literal):
        val = expr.value
        if isinstance(val, str):
            return f'"{val}"'
        return str(val)
    elif isinstance(expr, VarRef):
        return expr.name
    elif isinstance(expr, BinaryOp):
        return f"({expr_to_source(expr.left)} {expr.op} {expr_to_source(expr.right)})"
    elif isinstance(expr, UnaryOp):
        return f"{expr.op}{expr_to_source(expr.operand)}"
    elif isinstance(expr, MethodCall):
        obj = expr_to_source(expr.object)
        args = ", ".join(expr_to_source(a) for a in expr.args)
        return f"{obj}.{expr.method}({args})"
    elif isinstance(expr, FuncCall):
        args = ", ".join(expr_to_source(a) for a in expr.args)
        return f"{expr_to_source(expr.func)}({args})"
    elif isinstance(expr, ArrayLiteral):
        elems = ", ".join(expr_to_source(e) for e in expr.elements)
        return f"[{elems}]"
    else:
        return f"<{type(expr).__name__}>"
