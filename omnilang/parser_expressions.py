"""Parser mixin: expression transformers for OmniTransformer.

This module is imported by parser.py — it must NOT import from omnilang.parser
to avoid circular imports. All dependencies come from ast_nodes, utils, and parser_helpers.
"""
from lark import Token

from omnilang.ast_nodes import (
    Literal, VarRef, BinaryOp, UnaryOp, MethodCall, FuncCall, IndexGet,
    ArrayLiteral, MapLiteral, IfExpr,
    TemplateString, OptionalChain, ChainLink,
    NullCoalesce, ClosureExpr, AwaitExpr, SpreadExpr, StructLiteral,
    MatchExpr, MatchArm, StructPattern, OrPattern,
    VarPattern, IgnorePattern, FnType,
    TypeAnnotation, UnionType,
    Block, Param,
)
from omnilang.utils import mk_span
from omnilang.parser_helpers import parse_template_content, parse_inline_expr


class ExpressionMixin:
    """Mixin providing expression transformers. Methods correspond to Lark grammar rules."""

    # ── Index ──────────────────────────────────────

    def index(self, items):
        return items

    # ── Expressions: null coalesce ─────────────────

    def null_coalesce(self, items):
        """Handle null_coalesce: or_expr ("??" or_expr)*"""
        result = items[0]
        i = 1
        while i < len(items):
            if isinstance(items[i], Token) and str(items[i]) == "??":
                result = NullCoalesce(left=result, right=items[i + 1],
                                      span=self._span(result, items[i + 1]))
                i += 2
            else:
                i += 1
        return result

    def or_expr(self, items):
        result = items[0]
        i = 1
        while i < len(items):
            if isinstance(items[i], Token) and (str(items[i]) == "||" or items[i].type == "OR"):
                result = BinaryOp(left=result, op="||", right=items[i + 1],
                                  span=self._span(result, items[i + 1]))
                i += 2
            else:
                i += 1
        return result

    def and_expr(self, items):
        result = items[0]
        i = 1
        while i < len(items):
            if isinstance(items[i], Token) and (str(items[i]) == "&&" or items[i].type == "AND"):
                result = BinaryOp(left=result, op="&&", right=items[i + 1],
                                  span=self._span(result, items[i + 1]))
                i += 2
            else:
                i += 1
        return result

    def comp_expr(self, items):
        result = items[0]
        i = 1
        while i < len(items):
            if isinstance(items[i], Token) and items[i].type == "COMP_OP":
                result = BinaryOp(left=result, op=str(items[i]), right=items[i + 1],
                                  span=self._span(result, items[i + 1]))
                i += 2
            else:
                i += 1
        return result

    def add_expr(self, items):
        result = items[0]
        i = 1
        while i < len(items):
            if isinstance(items[i], Token) and items[i].type == "ADD_OP":
                result = BinaryOp(left=result, op=str(items[i]), right=items[i + 1],
                                  span=self._span(result, items[i + 1]))
                i += 2
            else:
                i += 1
        return result

    def mul_expr(self, items):
        result = items[0]
        i = 1
        while i < len(items):
            if isinstance(items[i], Token) and items[i].type == "MUL_OP":
                result = BinaryOp(left=result, op=str(items[i]), right=items[i + 1],
                                  span=self._span(result, items[i + 1]))
                i += 2
            else:
                i += 1
        return result

    def unary_expr(self, items):
        if len(items) == 2:
            op = str(items[0]) if isinstance(items[0], Token) else items[0]
            return UnaryOp(op=op, operand=items[1], span=self._span(*items))
        return items[0]

    def postfix_expr(self, items):
        result = items[0]
        for item in items[1:]:
            if isinstance(item, MethodCall):
                result = MethodCall(object=result, method=item.method,
                                   args=item.args, span=item.span)
            elif isinstance(item, FuncCall):
                result = FuncCall(func=result, args=item.args, span=item.span)
            elif isinstance(item, IndexGet):
                result = IndexGet(object=result, index=item.index, span=item.span)
            elif isinstance(item, ChainLink):
                if isinstance(result, OptionalChain):
                    result = OptionalChain(
                        object=result.object,
                        chain=list(result.chain) + [item],
                        span=result.span,
                    )
                else:
                    result = OptionalChain(object=result, chain=[item], span=self._span(result, item))
            elif isinstance(item, list):
                for sub in item:
                    if self._is_expr(sub):
                        result = IndexGet(object=result, index=sub, span=mk_span(sub))
                        break
        return result

    def atom_expr(self, items):
        return items[0]

    # ── Optional chain ─────────────────────────────

    def optional_chain(self, items):
        """Handle optional_chain: '?.' CNAME ['(' [call_args] ')'] | '?' index"""
        kind = "field"
        name = ""
        index = None
        args = []
        for item in items:
            if isinstance(item, Token) and item.type == "CNAME":
                name = str(item)
            elif isinstance(item, list):
                for sub in item:
                    if self._is_expr(sub):
                        args.append(sub)
                    elif isinstance(sub, Token):
                        continue
            elif self._is_expr(item):
                index = item
                kind = "index"
        if args:
            kind = "call"
        return ChainLink(kind=kind, name=name, index=index, args=args,
                        span=self._span(*items))

    # ── Closures & Arrow functions ────────────────

    def closure_expr(self, items):
        """Handle closure_expr: PIPE PIPE ['->' type] closure_body
                               | PIPE closure_params PIPE ['->' type] closure_body"""
        params = []
        return_type = None
        body = None
        for item in items:
            if isinstance(item, Token) and item.type in ("PIPE", "ARROW"):
                continue
            if isinstance(item, Param):
                params.append(item)
            elif isinstance(item, list):
                for sub in item:
                    if isinstance(sub, Param):
                        params.append(sub)
            elif isinstance(item, TypeAnnotation) or isinstance(item, UnionType):
                return_type = item
            elif isinstance(item, Block) or self._is_expr(item):
                body = item
        return ClosureExpr(params=params, return_type=return_type, body=body,
                          is_arrow=False, span=self._span(*items))

    def arrow_expr(self, items):
        """Handle arrow_expr: '(' [params] ')' '=>' closure_body"""
        params = []
        body = None
        for item in items:
            if isinstance(item, Param):
                params.append(item)
            elif isinstance(item, list):
                for sub in item:
                    if isinstance(sub, Param):
                        params.append(sub)
            elif isinstance(item, Block) or self._is_expr(item):
                body = item
        return ClosureExpr(params=params, body=body, is_arrow=True,
                          span=self._span(*items))

    def closure_body(self, items):
        """Handle closure_body: block | expr"""
        return items[0] if items else None

    def closure_params(self, items):
        """Handle closure_params: closure_param (',' closure_param)*"""
        return items

    def closure_param(self, items):
        """Handle closure_param: CNAME [':' type]"""
        name = str(items[0])
        type_ann = None
        for item in items[1:]:
            if isinstance(item, TypeAnnotation) or isinstance(item, UnionType):
                type_ann = item
        return Param(name=name, type_ann=type_ann, span=self._span(*items))

    # ── Function type ─────────────────────────────

    def fn_type(self, items):
        """Handle fn_type: 'fn' '(' [type (',' type)*] ')' '->' type"""
        param_types = []
        return_type = None
        for item in items:
            if isinstance(item, TypeAnnotation) or isinstance(item, UnionType):
                if return_type is None:
                    param_types.append(item)
                else:
                    pass
            elif isinstance(item, Token) and str(item) == "->":
                pass
        if len(param_types) > 0:
            return_type = param_types.pop()
        return FnType(param_types=param_types, return_type=return_type,
                     span=self._span(*items))

    # ── Match expression ──────────────────────────

    def match_expr(self, items):
        """Handle match_expr: 'match' expr '{' match_arm* '}'"""
        scrutinee = None
        arms = []
        for item in items:
            if isinstance(item, MatchArm):
                arms.append(item)
            elif self._is_expr(item) and scrutinee is None:
                scrutinee = item
            elif isinstance(item, list):
                for sub in item:
                    if isinstance(sub, MatchArm):
                        arms.append(sub)
        return MatchExpr(scrutinee=scrutinee, arms=arms, span=self._span(*items))

    def match_arm(self, items):
        """Handle match_arm: match_pattern ['if' expr] '=>' (block | expr)"""
        non_tokens = [i for i in items if not isinstance(i, Token)]
        if not non_tokens:
            return None
        pattern = non_tokens[0]
        body = non_tokens[-1] if len(non_tokens) > 1 else None
        guard = None
        for i, item in enumerate(items):
            if isinstance(item, Token) and str(item) == "if":
                for j in range(i + 1, len(items)):
                    if not isinstance(items[j], Token):
                        guard = items[j]
                        break
                break
        return MatchArm(pattern=pattern, guard=guard, body=body, span=self._span(*items))

    def match_pattern(self, items):
        """Handle match_pattern: handles or-patterns '|' and sub-patterns."""
        if not items:
            return IgnorePattern()
        patterns = []
        for item in items:
            if isinstance(item, Token):
                t = str(item)
                if t == "|":
                    continue
                if t == "_":
                    patterns.append(IgnorePattern(span=self._span(item)))
                elif item.type in ("TRUE", "FALSE"):
                    patterns.append(Literal(value=(t == "true"), kind="bool", span=self._span(item)))
                elif item.type == "NUMBER":
                    patterns.append(self.number_lit([item]))
                elif item.type == "STRING_LIT":
                    patterns.append(self.string_lit([item]))
                elif item.type == "CNAME":
                    patterns.append(VarPattern(name=t, span=self._span(item)))
            elif isinstance(item, OrPattern):
                patterns.extend(item.patterns)
            elif isinstance(item, (VarPattern, IgnorePattern, StructPattern, Literal)):
                patterns.append(item)
        if len(patterns) > 1:
            return OrPattern(patterns=patterns, span=self._span(*items))
        return patterns[0] if patterns else IgnorePattern()

    def literal_pattern(self, items):
        """Handle literal_pattern: NUMBER | STRING_LIT | TRUE | FALSE"""
        if not items:
            return None
        item = items[0]
        if isinstance(item, Token):
            if item.type == "NUMBER":
                return self.number_lit([item])
            elif item.type == "STRING_LIT":
                return self.string_lit([item])
            elif item.type == "TRUE":
                return Literal(value=True, kind="bool")
            elif item.type == "FALSE":
                return Literal(value=False, kind="bool")
        return item

    def struct_pattern(self, items):
        """Handle struct_pattern: CNAME ['{' match_field* '}']"""
        name = ""
        fields = []
        for item in items:
            if isinstance(item, Token) and item.type == "CNAME":
                name = str(item)
            elif isinstance(item, tuple) and len(item) == 2:
                fields.append(item)
            elif isinstance(item, list):
                for sub in item:
                    if isinstance(sub, tuple) and len(sub) == 2:
                        fields.append(sub)
        return StructPattern(name=name, fields=fields, span=self._span(*items))

    def match_field(self, items):
        """Handle match_field: CNAME ':' match_pattern"""
        name = ""
        pattern = None
        for item in items:
            if isinstance(item, Token) and item.type == "CNAME":
                name = str(item)
            elif isinstance(item, (VarPattern, IgnorePattern, StructPattern, OrPattern, Literal)):
                pattern = item
        return (name, pattern)

    def await_expr(self, items):
        """Handle await_expr: AWAIT unary_expr"""
        for item in items:
            if self._is_expr(item):
                return AwaitExpr(expr=item, span=self._span(*items))
        return items[0] if items else None

    # ── Spread expression ──────────────────────────

    def spread_expr(self, items):
        """Handle spread_expr: '...' expr"""
        for item in items:
            if self._is_expr(item):
                return SpreadExpr(expr=item, span=self._span(*items))
        return items[0] if items else None

    # ── Template strings ───────────────────────────

    def template_string(self, items):
        """Handle template_string: TEMPLATE_STRING terminal."""
        raw = self._extract_token(items[0]) if items else "``"
        inner = raw[1:-1] if len(raw) >= 2 else ""
        parts = parse_template_content(inner, parse_inline_expr)
        return TemplateString(parts=parts, span=self._span(*items))

    # ── Ternary / if-expr ──────────────────────────

    def if_expr(self, items):
        condition = None
        then_expr = None
        else_expr = None
        mode = "cond"
        for item in items:
            if isinstance(item, Token):
                continue
            if mode == "cond":
                condition = item
                mode = "then"
            elif mode == "then":
                then_expr = item
                mode = "else"
            elif mode == "else":
                else_expr = item
        return IfExpr(condition=condition, then_expr=then_expr, else_expr=else_expr,
                     span=self._span(*items))

    def var_ref(self, items):
        return VarRef(name=str(items[0]), span=self._span(*items))

    def struct_lit(self, items):
        """Handle struct_lit: CNAME '{' struct_lit_field* '}'"""
        type_name = ""
        fields = []
        for item in items:
            if isinstance(item, Token) and item.type == "CNAME":
                type_name = str(item)
            elif isinstance(item, tuple) and len(item) == 2:
                fields.append(item)
            elif isinstance(item, list):
                for sub in item:
                    if isinstance(sub, tuple) and len(sub) == 2:
                        fields.append(sub)
        return StructLiteral(type_name=type_name, fields=fields, span=self._span(*items))

    def method_call(self, items):
        method_name = ""
        args = []
        for item in items:
            if isinstance(item, Token) and item.type == "CNAME":
                method_name = str(item)
            elif self._is_expr(item):
                args.append(item)
            elif isinstance(item, list):
                for sub in item:
                    if self._is_expr(sub):
                        args.append(sub)
        return MethodCall(object=None, method=method_name, args=args,
                         span=self._span(*items))

    def func_call(self, items):
        args = []
        for item in items:
            if self._is_expr(item):
                args.append(item)
            elif isinstance(item, list):
                for sub in item:
                    if self._is_expr(sub):
                        args.append(sub)
        return FuncCall(func=None, args=args, span=self._span(*items))

    def call_args(self, items):
        return items

    def struct_lit_field(self, items):
        """Handle struct_lit_field: CNAME ':' expr"""
        name = ""
        value = None
        for item in items:
            if isinstance(item, Token) and item.type == "CNAME":
                name = str(item)
            elif self._is_expr(item):
                value = item
        return (name, value)

    # ── Literals ───────────────────────────────────

    def number_lit(self, items):
        raw = self._extract_token(items[0])
        if '.' in raw or 'e' in raw.lower():
            try:
                return Literal(value=float(raw), kind="f64", span=self._span(*items))
            except ValueError:
                pass
        try:
            if raw.startswith("0x") or raw.startswith("0X"):
                value = int(raw, 16)
            elif raw.startswith("0o") or raw.startswith("0O"):
                value = int(raw, 8)
            elif raw.startswith("0b") or raw.startswith("0B"):
                value = int(raw, 2)
            else:
                value = int(raw)
        except ValueError:
            value = int(float(raw))
        return Literal(value=value, kind="i32", span=self._span(*items))

    def literal_number(self, items):
        return items[0]

    def string_lit(self, items):
        raw = self._extract_token(items[0])
        inner = raw[1:-1]
        inner = inner.replace('\\n', '\n').replace('\\t', '\t').replace('\\\\', '\\').replace('\\"', '"')
        return Literal(value=inner, kind="String", span=self._span(*items))

    def bool_lit(self, items):
        if not items:
            return Literal(value=False, kind="bool")
        raw = self._extract_token(items[0])
        value = raw == "true"
        return Literal(value=value, kind="bool", span=self._span(*items))

    # Alias handlers
    def literal_int(self, items):
        return items[0]

    def literal_float(self, items):
        return items[0]

    def literal_string(self, items):
        return items[0]

    def literal_bool(self, items):
        return items[0]

    def array_lit(self, items):
        elements = []
        for item in items:
            if self._is_expr(item):
                elements.append(item)
            elif isinstance(item, list):
                for sub in item:
                    if self._is_expr(sub):
                        elements.append(sub)
        return ArrayLiteral(elements=elements, span=self._span(*items))

    def map_lit(self, items):
        entries = []
        current_key = None
        for item in items:
            if self._is_expr(item):
                if current_key is None:
                    current_key = item
                else:
                    entries.append((current_key, item))
                    current_key = None
            elif isinstance(item, list):
                for sub in item:
                    if self._is_expr(sub):
                        if current_key is None:
                            current_key = sub
                        else:
                            entries.append((current_key, sub))
                            current_key = None
        return MapLiteral(entries=entries, span=self._span(*items))

    def map_entry(self, items):
        return items
