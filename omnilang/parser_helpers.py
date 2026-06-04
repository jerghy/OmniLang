"""Template string parsing helpers for OmniLang parser."""
from omnilang.ast_nodes import (
    TemplatePart, Expr, Literal, ExprStmt,
)


def parse_template_content(content: str, parse_inline_fn) -> list[TemplatePart]:
    """Parse template string content into TemplatePart nodes.

    Splits on ${...} patterns and parses each expression.
    Handles escaped \$ and \` sequences.

    Args:
        content: The inner content of a template string (without backtick delimiters)
        parse_inline_fn: Callable that parses an inline expression string into an AST node
    """
    parts = []
    i = 0
    current_literal = []

    while i < len(content):
        if content[i] == '\\' and i + 1 < len(content):
            next_char = content[i + 1]
            if next_char in ('$', '`', '\\'):
                current_literal.append(next_char)
                i += 2
                continue
            else:
                current_literal.append(content[i])
                i += 1
                continue
        elif content[i] == '$' and i + 1 < len(content) and content[i + 1] == '{':
            if current_literal:
                parts.append(TemplatePart(
                    kind="literal", value="".join(current_literal),
                    span=None
                ))
                current_literal = []

            brace_depth = 1
            j = i + 2
            while j < len(content) and brace_depth > 0:
                if content[j] == '{':
                    brace_depth += 1
                elif content[j] == '}':
                    brace_depth -= 1
                j += 1

            expr_text = content[i + 2:j - 1]
            try:
                expr_ast = parse_inline_fn(expr_text)
                parts.append(TemplatePart(
                    kind="expr", value=expr_ast,
                    span=None
                ))
            except Exception:
                parts.append(TemplatePart(
                    kind="literal", value="${" + expr_text + "}",
                    span=None
                ))

            i = j
            continue
        else:
            current_literal.append(content[i])
            i += 1

    if current_literal:
        parts.append(TemplatePart(
            kind="literal", value="".join(current_literal),
            span=None
        ))

    return parts


def parse_inline_expr(expr_text: str) -> Expr:
    """Parse an inline expression from template string interpolation.

    Uses a sub-parse with a minimal grammar context.
    """
    from omnilang.lexer import create_parser
    from omnilang.parser import OmniTransformer
    parser = create_parser()
    source = f"fn __tpl() {{ {expr_text}; }}"
    tree = parser.parse(source)
    transformer = OmniTransformer()
    prog = transformer.transform(tree)
    if prog.functions and prog.functions[0].body:
        stmts = prog.functions[0].body.stmts
        if stmts and isinstance(stmts[0], ExprStmt):
            return stmts[0].expr
    return Literal(value=expr_text, kind="String")
