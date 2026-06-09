"""Parser: converts lark parse tree to OmniLang AST.

v0.2: Extended transformer handling template strings, optional chains,
destructuring, structs, union types, and more.
"""

from lark import Transformer, Token

from omnilang.ast_nodes import (
    # Top-level
    Program, FunctionDef, Param, StructDef, StructField,
    TaggedUnionDef, Variant,
    ClassDef, ClassMember,
    InterfaceDef, InterfaceMember,
    ImportStmt, ImportTypeStmt, ExportStmt,
    # Statements
    Block, VarDecl, Destructure, Assign, IfStmt, WhileStmt, ForInStmt,
    BreakStmt, ContinueStmt, ReturnStmt, PrintStmt, ExprStmt,
    # Expressions
    Literal, VarRef, BinaryOp, UnaryOp, MethodCall, FuncCall, IndexGet,
    ArrayLiteral, MapLiteral, IfExpr,
    # v0.2 expressions
    TemplateString, OptionalChain, NullCoalesce, ClosureExpr, AwaitExpr, SpreadExpr, StructLiteral,
    MatchExpr, VarPattern, IgnorePattern, RestPattern, ArrayPattern, ObjectPattern,
    # Types
    TypeAnnotation, UnionType, GenericParam,
    # Base
    Stmt,
)
from omnilang.utils import Span, span_from_token, mk_span
from omnilang.parser_expressions import ExpressionMixin

# Backward-compatible aliases
_span_from_token = span_from_token
_mk_span = mk_span


class OmniTransformer(ExpressionMixin, Transformer):
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
        imports = []
        exports = []
        lib_imports = []
        for item in items:
            if isinstance(item, FunctionDef):
                funcs.append(item)
            elif isinstance(item, StructDef):
                structs.append(item)
            elif isinstance(item, TaggedUnionDef):
                tagged_unions.append(item)
            elif isinstance(item, ImportStmt):
                imports.append(item)
            elif isinstance(item, ImportTypeStmt):
                pass
            elif isinstance(item, ExportStmt):
                exports.append(item)
                if item.item and isinstance(item.item, FunctionDef):
                    funcs.append(item.item)
                elif item.item and isinstance(item.item, StructDef):
                    structs.append(item.item)
            elif isinstance(item, ClassDef):
                classes.append(item)
            elif isinstance(item, InterfaceDef):
                pass
            elif isinstance(item, tuple) and len(item) == 3 and item[0] == "impl":
                _, struct_name, methods = item
                for s in structs:
                    if s.name == struct_name:
                        s.methods.extend(methods)
                        break
            elif isinstance(item, str):
                # import_lib returns a string (the library name)
                lib_imports.append(item)
        return Program(functions=funcs, structs=structs,
                       tagged_unions=tagged_unions, classes=classes,
                       imports=imports, exports=exports,
                       lib_imports=lib_imports,
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

    # ── Struct definition ──────────────────────────

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

    # ── Interface definition ────────────────────────

    def interface_def(self, items):
        """Handle interface_def: 'interface' CNAME '{' interface_member* '}'"""
        name = ""
        members = []
        for item in items:
            if isinstance(item, Token) and item.type == "CNAME":
                name = str(item)
            elif isinstance(item, InterfaceMember):
                members.append(item)
            elif isinstance(item, list):
                for sub in item:
                    if isinstance(sub, InterfaceMember):
                        members.append(sub)
        return InterfaceDef(name=name, members=members, span=self._span(*items))

    def interface_member(self, items):
        """Handle interface_member: CNAME ':' type ';' | FN CNAME '(' params ')' ['->' return_type] ';'"""
        name = ""
        type_ann = None
        params = []
        return_type = None
        is_method = False
        for item in items:
            if isinstance(item, Token) and item.type == "CNAME" and not name:
                name = str(item)
            elif isinstance(item, Token) and item.type == "FN":
                is_method = True
            elif isinstance(item, TypeAnnotation) or isinstance(item, UnionType):
                if is_method and not return_type:
                    return_type = item
                elif not is_method:
                    type_ann = item
            elif isinstance(item, Param):
                params.append(item)
            elif isinstance(item, list):
                for sub in item:
                    if isinstance(sub, Param):
                        params.append(sub)
        return InterfaceMember(name=name, type_ann=type_ann, params=params,
                              return_type=return_type, is_method=is_method,
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

    def import_lib(self, items):
        """Handle import_lib: 'import' CNAME ';' — library import like 'import omniview;'"""
        for item in items:
            if isinstance(item, Token) and item.type == "CNAME":
                return str(item)
        return ""

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

    def import_type_stmt(self, items):
        """Handle import_type_stmt: 'import' 'type' '{' CNAME (',' CNAME)* '}' 'from' STRING_LIT ';'"""
        names = []
        path = ""
        for item in items:
            if isinstance(item, Token):
                if item.type == "CNAME":
                    names.append(str(item))
                elif item.type == "STRING_LIT":
                    path = str(item)[1:-1]
        return ImportTypeStmt(names=names, path=path, span=self._span(*items))

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
        for func in list(remaining_functions):
            # Simple heuristic: if function name starts with the struct name
            # or if it was marked as a method. For now, keep functions as top-level.
            pass
    # Full impl method attachment will be implemented in Phase 3 type checker.
