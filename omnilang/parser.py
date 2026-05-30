"""Parser: converts lark parse tree to OmniLang AST.

v0.2: Extended transformer handling template strings, optional chains,
destructuring, structs, union types, and more.
"""

import ast as py_ast
from lark import Transformer, Token

from omnilang.ast_nodes import (
    # Top-level
    Program, FunctionDef, Param, StructDef, StructField,
    TaggedUnionDef, Variant,
    ClassDef, ClassMember,
    ImportStmt, ExportStmt,
    # Statements
    Block, VarDecl, Destructure, Assign, IfStmt, WhileStmt, ForInStmt,
    BreakStmt, ContinueStmt, ReturnStmt, PrintStmt, ExprStmt,
    # Expressions
    Literal, VarRef, BinaryOp, UnaryOp, MethodCall, FuncCall, IndexGet,
    ArrayLiteral, MapLiteral, IfExpr,
    # v0.2 expressions
    TemplateString, TemplatePart, OptionalChain, ChainLink,
    NullCoalesce, ClosureExpr, AwaitExpr, SpreadExpr, StructLiteral,
    MatchExpr, MatchArm, StructPattern, OrPattern,
    FnType,
    # Patterns
    VarPattern, IgnorePattern, RestPattern, ArrayPattern, ObjectPattern,
    # Types
    TypeAnnotation, UnionType, GenericParam,
    # Base
    Stmt, Expr,
)
from omnilang.utils import Span


def _span_from_token(t) -> Span:
    """Extract span from a Lark token or tree node."""
    return Span(line=t.line or 1, col=t.column or 1,
                end_line=t.end_line or 0, end_col=t.end_column or 0)


def _mk_span(obj) -> Span | None:
    """Create a Span from a lark object if possible."""
    try:
        return Span(line=obj.line or 1, col=obj.column or 1,
                    end_line=obj.end_line or 0, end_col=obj.end_column or 0)
    except (AttributeError, TypeError):
        return None


class OmniTransformer(Transformer):
    """Transforms lark parse tree into OmniLang AST nodes.

    v0.2: Extended with all Phase 1 feature transformers.
    """

    # ── Helpers ────────────────────────────────────

    EXPR_TYPES = (
        Literal, VarRef, BinaryOp, UnaryOp, MethodCall, FuncCall,
        IndexGet, ArrayLiteral, MapLiteral, IfExpr,
        TemplateString, OptionalChain, NullCoalesce, ClosureExpr, AwaitExpr, SpreadExpr, StructLiteral,
        MatchExpr,
    )

    TYPE_TYPES = (TypeAnnotation, UnionType)

    @staticmethod
    def _is_expr(obj) -> bool:
        return isinstance(obj, OmniTransformer.EXPR_TYPES)

    @staticmethod
    def _is_type(obj) -> bool:
        return isinstance(obj, OmniTransformer.TYPE_TYPES)

    def _span(self, *items) -> Span | None:
        for item in items:
            s = _mk_span(item)
            if s:
                return s
        return None

    def _extract_token(self, item) -> str:
        """Extract string value from a Token or Tree node."""
        if isinstance(item, Token):
            return str(item)
        if hasattr(item, 'children') and len(item.children) > 0:
            return str(item.children[0])
        return str(item)

    # ── Start / Program ────────────────────────────

    def start(self, items):
        return items[0]

    def program(self, items):
        funcs = []
        structs = []
        tagged_unions = []
        classes = []
        for item in items:
            if isinstance(item, FunctionDef):
                funcs.append(item)
            elif isinstance(item, StructDef):
                structs.append(item)
            elif isinstance(item, TaggedUnionDef):
                tagged_unions.append(item)
            elif isinstance(item, ImportStmt):
                pass  # stored in separate list (future: Program.imports)
            elif isinstance(item, ExportStmt):
                if item.item and isinstance(item.item, FunctionDef):
                    funcs.append(item.item)
                elif item.item and isinstance(item.item, StructDef):
                    structs.append(item.item)
            elif isinstance(item, ClassDef):
                classes.append(item)
            elif isinstance(item, tuple) and len(item) == 3 and item[0] == "impl":
                _, struct_name, methods = item
                for s in structs:
                    if s.name == struct_name:
                        s.methods.extend(methods)
                        break
        return Program(functions=funcs, structs=structs,
                       tagged_unions=tagged_unions, classes=classes,
                       span=self._span(*items))

    def function_def(self, items):
        # items: [ASYNC], FN_token, name_token, [generic_params], [(params)], [return_type], block
        name = ""
        fn_params = []
        return_type = None
        body = None
        generic_params = []
        is_async = False
        for item in items:
            if isinstance(item, Token):
                if item.type == "ASYNC":
                    is_async = True
                elif item.type == "FN":
                    continue
                elif item.type == "CNAME" and not name:
                    name = str(item)
            elif isinstance(item, Param):
                fn_params.append(item)
            elif isinstance(item, list):
                for sub in item:
                    if isinstance(sub, Param):
                        fn_params.append(sub)
                    elif isinstance(sub, GenericParam):
                        generic_params.append(sub)
            elif isinstance(item, GenericParam):
                generic_params.append(item)
            elif self._is_type(item):
                return_type = item
            elif isinstance(item, Block):
                body = item
        return FunctionDef(name=name, params=fn_params, return_type=return_type,
                          generic_params=generic_params, body=body,
                          is_async=is_async, span=self._span(*items))

    # ── Generics ───────────────────────────────────

    def generic_params(self, items):
        return items  # list of GenericParam

    def generic_param(self, items):
        name = str(items[0])
        bounds = []
        for item in items[1:]:
            if isinstance(item, Token) and item.type == "CNAME":
                bounds.append(str(item))
        return GenericParam(name=name, bounds=bounds, span=self._span(*items))

    # ── Params ─────────────────────────────────────

    def params(self, items):
        return items

    def param(self, items):
        name = str(items[0])
        type_ann = None
        is_rest = False
        for item in items[1:]:
            if self._is_type(item):
                type_ann = item
            elif isinstance(item, Token) and str(item) == "...":
                is_rest = True
        # Check for rest param pattern: ...CNAME : type
        if str(items[0]) == "..." and len(items) > 1:
            is_rest = True
            for i, item in enumerate(items):
                if isinstance(item, Token) and item.type == "CNAME" and str(item) != "...":
                    name = str(item)
                elif self._is_type(item):
                    type_ann = item
        return Param(name=name, type_ann=type_ann, is_rest=is_rest, span=self._span(*items))

    def return_type(self, items):
        return items[0] if items else None

    # ── Types ──────────────────────────────────────

    def type(self, items):
        """Handle type → union_type. Returns TypeAnnotation or UnionType."""
        if not items:
            return None
        return items[0]

    def union_type(self, items):
        """Handle union_type: struct_type ("|" struct_type)*"""
        types = []
        for item in items:
            if self._is_type(item):
                types.append(item)
            elif isinstance(item, Token) and str(item) in ("|", "or"):
                continue
        if len(types) == 0:
            return None
        if len(types) == 1:
            return types[0]
        return UnionType(types=types, span=self._span(*items))

    def struct_type(self, items):
        """Handle struct_type: base_type ("<" type_args ">")? | optional_type"""
        if not items:
            return None
        # Could be from optional_type → UnionType
        if isinstance(items[0], UnionType):
            return items[0]
        if len(items) == 1:
            return items[0]
        # base_type with type_args
        base = items[0]
        if isinstance(base, TypeAnnotation):
            args = ()
            for item in items[1:]:
                if isinstance(item, list):
                    for sub in item:
                        if isinstance(sub, TypeAnnotation):
                            args += (sub,)
                elif self._is_type(item):
                    args += (item,)
            if args:
                return TypeAnnotation(name=base.name, type_args=args, span=base.span)
        return base

    def optional_type(self, items):
        """Handle optional_type: base_type "?" → UnionType(T, null)"""
        base = items[0]
        if isinstance(base, TypeAnnotation):
            null_type = TypeAnnotation(name="null", span=base.span)
            return UnionType(types=[base, null_type], span=self._span(*items))
        return base

    def base_type(self, items):
        if not items:
            return None
        t = str(items[0])
        return TypeAnnotation(name=t, span=self._span(*items))

    def TYPE_NAME(self, tok):
        return tok

    def type_args(self, items):
        return list(items)

    # ── Statements ─────────────────────────────────

    def stmt(self, items):
        return items[0]

    def var_decl(self, items):
        # items can be from 3 different grammar alternatives:
        #   LET_KW [MUT_KW] CNAME ":" type "=" expr ";"
        #   LET_KW [MUT_KW] array_pattern "=" expr ";"
        #   LET_KW [MUT_KW] object_pattern "=" expr ";"
        mutable = False
        name = None
        type_ann = None
        pattern = None
        init = None
        for item in items:
            if isinstance(item, Token):
                if item.type == "MUT_KW":
                    mutable = True
                elif item.type == "CNAME":
                    name = str(item)
            elif self._is_type(item) or isinstance(item, UnionType):
                type_ann = item
            elif isinstance(item, (ArrayPattern, ObjectPattern)):
                pattern = item
            elif self._is_expr(item):
                init = item
        if pattern is not None:
            return Destructure(pattern=pattern, init=init, mutable=mutable,
                             span=self._span(*items))
        if name:
            return VarDecl(mutable=mutable, name=name, type_ann=type_ann,
                          init=init, span=self._span(*items))
        return None

    def var_target(self, items):
        """Deprecated — var_decl now handles patterns inline. Kept for compat."""
        if not items:
            return None
        if isinstance(items[0], (ArrayPattern, ObjectPattern)):
            return Destructure(pattern=items[0], init=None, span=self._span(*items))
        name = None
        type_ann = None
        for item in items:
            if isinstance(item, Token) and item.type == "CNAME":
                name = str(item)
            elif self._is_type(item) or isinstance(item, UnionType):
                type_ann = item
        if name:
            return (name, type_ann)
        return None

    def array_pattern(self, items):
        """Handle array_pattern: '[' pattern_elem (',' pattern_elem)* ... ']'"""
        elements = []
        rest = None
        for item in items:
            if isinstance(item, RestPattern):
                rest = item
            elif isinstance(item, (VarPattern, IgnorePattern, ArrayPattern)):
                elements.append(item)
            elif isinstance(item, Token):
                t = str(item)
                if t == "_":
                    elements.append(IgnorePattern(span=self._span(item)))
        return ArrayPattern(elements=elements, rest=rest, span=self._span(*items))

    def pattern_elem(self, items):
        """Handle pattern_elem: CNAME | '_' | array_pattern | object_pattern"""
        if not items:
            return IgnorePattern()
        item = items[0]
        if isinstance(item, (VarPattern, IgnorePattern, ArrayPattern, ObjectPattern)):
            return item
        if isinstance(item, Token):
            t = str(item)
            if t == "_":
                return IgnorePattern(span=self._span(item))
            return VarPattern(name=t, span=self._span(item))
        return IgnorePattern()

    def rest_pattern(self, items):
        """Handle rest_pattern: '...' CNAME"""
        name = ""
        for item in items:
            if isinstance(item, Token) and item.type == "CNAME":
                name = str(item)
        return RestPattern(name=name, span=self._span(*items))

    def object_pattern(self, items):
        """Handle object_pattern: '{' CNAME (',' CNAME)* '}'"""
        fields = []
        for item in items:
            if isinstance(item, Token) and item.type == "CNAME":
                fields.append(str(item))
        return ObjectPattern(fields=fields, span=self._span(*items))

    def assign(self, items):
        # items: name, [index], ASSIGN_OP, expr, ;
        name = str(items[0])
        index = None
        op = "="
        value = None
        for item in items[1:]:
            if isinstance(item, Token) and item.type == "ASSIGN_OP":
                op = str(item)
            elif self._is_expr(item):
                value = item
            elif isinstance(item, list) and not isinstance(item, (ArrayLiteral,)):
                for sub in item:
                    if not self._is_expr(sub):
                        continue
                    index = sub
        return Assign(target=name, index=index, op=op, value=value,
                     span=self._span(*items))

    def if_stmt(self, items):
        condition = None
        then_body = None
        else_ifs = []
        else_body = None
        for item in items:
            if self._is_expr(item) and condition is None:
                condition = item
            elif isinstance(item, Block) and then_body is None:
                then_body = item
            elif isinstance(item, list):
                chain = item
                while chain:
                    if isinstance(chain, list) and len(chain) == 1:
                        chain = chain[0]
                    if isinstance(chain, Block):
                        else_body = chain
                        break
                    elif isinstance(chain, IfStmt):
                        else_ifs.append((chain.condition, chain.then_body))
                        if chain.else_ifs:
                            else_ifs.extend(chain.else_ifs)
                        if chain.else_body:
                            else_body = chain.else_body
                        break
                    elif isinstance(chain, tuple):
                        cond, blk = chain
                        else_ifs.append((cond, blk))
                        break
                    else:
                        break
        return IfStmt(condition=condition, then_body=then_body,
                     else_ifs=else_ifs, else_body=else_body,
                     span=self._span(*items))

    def else_tail(self, items):
        if len(items) == 1:
            return items[0]
        return list(items)

    def while_stmt(self, items):
        condition = None
        body = None
        for item in items:
            if self._is_expr(item) and condition is None:
                condition = item
            elif isinstance(item, Block):
                body = item
        return WhileStmt(condition=condition, body=body, span=self._span(*items))

    def for_stmt(self, items):
        var_name = str(items[0])
        iterable = None
        body = None
        for item in items[1:]:
            if self._is_expr(item) and iterable is None:
                iterable = item
            elif isinstance(item, Block):
                body = item
        return ForInStmt(var_name=var_name, iterable=iterable, body=body,
                        span=self._span(*items))

    def break_stmt(self, items):
        return BreakStmt(span=self._span(*items))

    def continue_stmt(self, items):
        return ContinueStmt(span=self._span(*items))

    def return_stmt(self, items):
        value = None
        for item in items:
            if self._is_expr(item):
                value = item
        return ReturnStmt(value=value, span=self._span(*items))

    def print_stmt(self, items):
        arg = None
        for item in items:
            if self._is_expr(item):
                arg = item
        return PrintStmt(newline=False, arg=arg, span=self._span(*items))

    def println_stmt(self, items):
        arg = None
        for item in items:
            if self._is_expr(item):
                arg = item
        return PrintStmt(newline=True, arg=arg, span=self._span(*items))

    def expr_stmt(self, items):
        expr = None
        for item in items:
            if not isinstance(item, Token):
                expr = item
        return ExprStmt(expr=expr, span=self._span(*items))

    def block_stmt(self, items):
        return items[0]

    # ── Block ──────────────────────────────────────

    def block(self, items):
        stmts = []
        trailing = None
        for item in items:
            if item is None:
                continue
            if isinstance(item, Token):
                continue
            if isinstance(item, Stmt):
                stmts.append(item)
            elif isinstance(item, Destructure):
                stmts.append(item)
            elif self._is_expr(item):
                trailing = item
            elif isinstance(item, list):
                for sub in item:
                    if isinstance(sub, Stmt):
                        stmts.append(sub)
                    elif isinstance(sub, Destructure):
                        stmts.append(sub)
                    elif self._is_expr(sub):
                        trailing = sub
        return Block(stmts=stmts, trailing_expr=trailing, span=self._span(*items))

    def body_contents(self, items):
        return items

    def expr_no_semi(self, items):
        return items[0] if items else None

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
            if isinstance(items[i], Token) and str(items[i]) == "&&":
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
                # Optional chain link — wrap in OptionalChain
                if isinstance(result, OptionalChain):
                    result.chain.append(item)
                else:
                    result = OptionalChain(object=result, chain=[item], span=self._span(result, item))
            elif isinstance(item, list):
                for sub in item:
                    if self._is_expr(sub):
                        result = IndexGet(object=result, index=sub, span=_mk_span(sub))
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
                # Could be index or call_args
                for sub in item:
                    if self._is_expr(sub):
                        # call_args
                        args.append(sub)
                    elif isinstance(sub, Token):
                        continue
            elif self._is_expr(item):
                # index expression
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
                    pass  # return_type already set
            elif isinstance(item, Token) and str(item) == "->":
                pass
        # The last type is the return type
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
        """Handle match_arm: match_pattern ['if' expr] '=>' (block | expr)

        Transformed children: [pattern, guard?, body]
        First non-token is pattern, last non-token is body.
        """
        non_tokens = [i for i in items if not isinstance(i, Token)]
        if not non_tokens:
            return None
        pattern = non_tokens[0]
        body = non_tokens[-1] if len(non_tokens) > 1 else None
        guard = None
        # Check for guard (middle non-token between 'if' token and '=>')
        for i, item in enumerate(items):
            if isinstance(item, Token) and str(item) == "if":
                # The next non-token is the guard
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
        """Handle template_string: TEMPLATE_STRING terminal.

        Post-processes the raw template string to extract ${expr} interpolation
        and create TemplateString with TemplatePart nodes.
        """
        raw = self._extract_token(items[0]) if items else "``"
        # Strip the backtick delimiters
        inner = raw[1:-1] if len(raw) >= 2 else ""

        # Parse the inner content to separate literals from ${expr} parts
        parts = self._parse_template_content(inner)
        return TemplateString(parts=parts, span=self._span(*items))

    def _parse_template_content(self, content: str) -> list[TemplatePart]:
        """Parse template string content into TemplatePart nodes.

        Splits on ${...} patterns and parses each expression.
        Handles escaped \$ and \` sequences.
        """
        parts = []
        i = 0
        current_literal = []

        while i < len(content):
            if content[i] == '\\' and i + 1 < len(content):
                # Escape sequence
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
                # Found interpolation start
                # Flush accumulated literal
                if current_literal:
                    parts.append(TemplatePart(
                        kind="literal", value="".join(current_literal),
                        span=None
                    ))
                    current_literal = []

                # Find matching closing brace
                brace_depth = 1
                j = i + 2
                while j < len(content) and brace_depth > 0:
                    if content[j] == '{':
                        brace_depth += 1
                    elif content[j] == '}':
                        brace_depth -= 1
                    j += 1

                expr_text = content[i + 2:j - 1]
                # Parse the expression using a sub-parse
                try:
                    expr_ast = self._parse_inline_expr(expr_text)
                    parts.append(TemplatePart(
                        kind="expr", value=expr_ast,
                        span=None
                    ))
                except Exception:
                    # If parsing fails, treat as literal (error will be caught by analyzer)
                    parts.append(TemplatePart(
                        kind="literal", value="${" + expr_text + "}",
                        span=None
                    ))

                i = j
                continue
            else:
                current_literal.append(content[i])
                i += 1

        # Flush remaining literal
        if current_literal:
            parts.append(TemplatePart(
                kind="literal", value="".join(current_literal),
                span=None
            ))

        return parts

    def _parse_inline_expr(self, expr_text: str) -> Expr:
        """Parse an inline expression from template string interpolation.

        Uses a sub-parse with a minimal grammar context.
        """
        from omnilang.lexer import create_parser
        # Use a simple expression parse
        parser = create_parser()
        # Wrap in a minimal function context to parse
        source = f"fn __tpl() {{ {expr_text}; }}"
        tree = parser.parse(source)
        # Extract the expression from the block
        transformer = OmniTransformer()
        prog = transformer.transform(tree)
        if prog.functions and prog.functions[0].body:
            stmts = prog.functions[0].body.stmts
            if stmts and isinstance(stmts[0], ExprStmt):
                return stmts[0].expr
        # Fallback: return as literal string
        return Literal(value=expr_text, kind="String")

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

    # ── Struct definition ──────────────────────────

    def struct_def(self, items):
        """Handle struct_def: ['nominal'] 'struct' CNAME [generic_params] '{' struct_field* '}'"""
        name = ""
        generic_params = []
        fields = []
        is_nominal = False
        for item in items:
            if isinstance(item, Token):
                if item.type == "NOMINAL":
                    is_nominal = True
                elif item.type == "CNAME":
                    name = str(item)
            elif isinstance(item, GenericParam):
                generic_params.append(item)
            elif isinstance(item, list):
                for sub in item:
                    if isinstance(sub, GenericParam):
                        generic_params.append(sub)
                    elif isinstance(sub, StructField):
                        fields.append(sub)
            elif isinstance(item, StructField):
                fields.append(item)
        return StructDef(name=name, generic_params=generic_params,
                        fields=fields, is_nominal=is_nominal,
                        span=self._span(*items))

    def struct_field(self, items):
        """Handle struct_field: CNAME ':' type ','?"""
        name = ""
        type_ann = None
        for item in items:
            if isinstance(item, Token) and item.type == "CNAME":
                name = str(item)
            elif self._is_type(item):
                type_ann = item
        return StructField(name=name, type_ann=type_ann, span=self._span(*items))

    def impl_block(self, items):
        """Handle impl_block: 'impl' CNAME [generic_params] '{' function_def* '}'

        Returns methods as FunctionDef list. The caller (program) attaches them
        to the corresponding struct.
        """
        struct_name = ""
        methods = []
        for item in items:
            if isinstance(item, Token) and item.type == "CNAME":
                struct_name = str(item)
            elif isinstance(item, FunctionDef):
                methods.append(item)
            elif isinstance(item, list):
                for sub in item:
                    if isinstance(sub, FunctionDef):
                        methods.append(sub)
        return ("impl", struct_name, methods)

    # ── Class definition ──────────────────────────

    def class_def(self, items):
        """Handle class_def: 'class' CNAME [extends] [implements] '{' class_member* '}'"""
        name = ""
        extends = None
        implements = []
        members = []
        for item in items:
            if isinstance(item, Token) and item.type == "CNAME":
                name = name or str(item)
            elif isinstance(item, tuple) and len(item) == 2 and item[0] == "extends":
                extends = item[1]
            elif isinstance(item, list):
                for sub in item:
                    if isinstance(sub, ClassMember):
                        members.append(sub)
                    elif isinstance(sub, tuple) and sub[0] == "extends":
                        extends = sub[1]
                    elif isinstance(sub, tuple) and sub[0] == "implements":
                        implements.extend(sub[1])
            elif isinstance(item, ClassMember):
                members.append(item)
        return ClassDef(name=name, extends=extends, implements=implements,
                        members=members, span=self._span(*items))

    def extends_clause(self, items):
        return ("extends", str(items[0]) if items else "")

    def implements_clause(self, items):
        names = [str(item) for item in items if isinstance(item, Token) and item.type == "CNAME"]
        return ("implements", names)

    def class_member(self, items):
        is_static = any(isinstance(i, Token) and str(i) == "static" for i in items)
        is_private = any(isinstance(i, Token) and str(i) == "private" for i in items)
        is_override = any(isinstance(i, Token) and str(i) == "@override" for i in items)
        name = ""
        type_ann = None
        init = None
        func_def = None
        for item in items:
            if isinstance(item, Token) and item.type == "CNAME":
                name = str(item)
            elif isinstance(item, TypeAnnotation) or isinstance(item, UnionType):
                type_ann = item
            elif isinstance(item, FunctionDef):
                func_def = item
            elif self._is_expr(item):
                init = item
        return ClassMember(is_static=is_static, visibility="private" if is_private else "public",
                          name=name, type_ann=type_ann, init=init,
                          func_def=func_def, is_override=is_override,
                          span=self._span(*items))

    # ── Tagged union definition ────────────────────

    def tagged_union_def(self, items):
        """Handle tagged_union_def: 'type' CNAME [generic_params] '=' variant ('|' variant)*"""
        name = ""
        generic_params = []
        variants = []
        for item in items:
            if isinstance(item, Token) and item.type == "CNAME":
                name = str(item)
            elif isinstance(item, GenericParam):
                generic_params.append(item)
            elif isinstance(item, Variant):
                variants.append(item)
            elif isinstance(item, list):
                for sub in item:
                    if isinstance(sub, GenericParam):
                        generic_params.append(sub)
                    elif isinstance(sub, Variant):
                        variants.append(sub)
        return TaggedUnionDef(name=name, generic_params=generic_params,
                               variants=variants, span=self._span(*items))

    def import_stmt(self, items):
        """Handle import_stmt: 'import' '{' CNAME (',' CNAME)* '}' 'from' STRING_LIT ';'"""
        names = []
        path = ""
        for item in items:
            if isinstance(item, Token):
                if item.type == "CNAME":
                    names.append(str(item))
                elif item.type == "STRING_LIT":
                    path = str(item)[1:-1]  # strip quotes
        return ImportStmt(names=names, path=path, span=self._span(*items))

    def export_stmt(self, items):
        """Handle export_stmt: 'export' function_def"""
        item = items[0] if items else None
        return ExportStmt(item=item, span=self._span(*items))

    def variant(self, items):
        """Handle variant: CNAME ['{' struct_field* '}']"""
        name = ""
        fields = []
        for item in items:
            if isinstance(item, Token) and item.type == "CNAME":
                name = str(item)
            elif isinstance(item, StructField):
                fields.append(item)
            elif isinstance(item, list):
                for sub in item:
                    if isinstance(sub, StructField):
                        fields.append(sub)
        return Variant(name=name, fields=fields, span=self._span(*items))

    # ── Struct literal field (used by cname_expr) ───

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


def parse(source: str, filename: str = "<input>") -> Program:
    """Parse OmniLang source code into an AST."""
    from omnilang.lexer import create_parser
    lark_parser = create_parser()
    tree = lark_parser.parse(source)
    transformer = OmniTransformer()
    prog = transformer.transform(tree)
    # Post-pass: attach impl methods to their structs
    _attach_impl_methods(prog)
    return prog


def parse_file(filepath: str) -> Program:
    """Parse an OmniLang source file into an AST."""
    with open(filepath, "r", encoding="utf-8") as f:
        source = f.read()
    return parse(source, filename=filepath)


def _attach_impl_methods(prog: Program):
    """Post-pass: attach impl block methods to their corresponding struct definitions."""
    # Collect impl methods
    impl_methods: dict[str, list[FunctionDef]] = {}
    remaining_functions = []
    for func in prog.functions:
        remaining_functions.append(func)

    # Check for impl markers in the program span context (unwrapped from transformer)
    # Actually, impl_blocks are transformed inline. The 'program' rule handles
    # struct_def and function_def. We need to check if any function came from impl.
    # For now, the impl_block transformer returns an ("impl", name, methods) tuple,
    # which is handled in program() by attaching to the right struct.
    # Re-process: if program has structs and there are method-only functions...
    for struct in prog.structs:
        methods_to_attach = []
        for func in list(remaining_functions):
            # Simple heuristic: if function name starts with the struct name
            # or if it was marked as a method. For now, keep functions as top-level.
            pass
    # Full impl method attachment will be implemented in Phase 3 type checker.
