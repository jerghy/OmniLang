"""OmniView parser transformer — converts Lark parse trees to OmniView AST.

Uses the Lark Transformer pattern with the grammar in omniview/grammar.lark.

Delegation: Expression and statement parsing can be delegated to OmniLang
via OmniLang's parser_helpers module. When preprocessor output is available,
RAW_BLOCK tokens are resolved by calling OmniLang's parse_inline_block().

LINE BUDGET: ≤1600 lines (Council R9, 2026-06-09). Current: ~1305.
If approaching 1600, split into parser_core.py + parser_ui.py.
"""

from lark import Transformer, Token

from omnilang.ast_nodes import (
    Expr, Literal, VarRef, BinaryOp, UnaryOp,
    FuncCall, MethodCall, IndexGet,
    ArrayLiteral, MapLiteral, IfExpr, TemplateString,
    SpreadExpr,
    TypeAnnotation, UnionType,
    Block as OmniLangBlock,
)
from omnilang.utils import Span

from omniview.ast_nodes import (
    # Top-level
    Program, ComponentDef, StoreDef, ThemeDef,
    # Import
    ImportStmt,
    # Component
    PropDecl, SignalDecl, ComputedDecl, EventDecl, RefDecl, StoreRef,
    # Store
    StoreSignalDecl, StoreGetterDecl, StoreActionDecl,
    # Event handlers
    InlineFn, FnRef, StoreActionRef,
    # Class values
    StyleRefClass, StringClass, ClassMapClass,
    # Element props
    StaticProp, BindProp, EventProp, RefProp, ClassProp, StyleProp,
    ElementProp, Element,
    # View
    ViewBlock,
    # Directives
    IfBranch, IfDirective, ForDirective,
    SlotDirective, NativeBlock, FallbackBlock, ErrorBoundary,
    # Lifecycle
    LifecycleHook,
    # Style & theme
    StyleBlock, StyleRule, ThemeCategory,
    # Functions
    FnDef, Param, Block,
    # OmniView expressions
    FieldAccess, TernaryExpr, ClassMapExpr, StyleObjectExpr,
    # Statements
    Stmt, VarDeclStmt, AssignStmt, IfStmt, WhileStmt, ForInStmt,
    ReturnStmt, ExprStmt, EventEmitStmt,
    # Text
    TextNode, Interpolation,
)


class OmniViewTransformer(Transformer):
    """Transforms OmniView Lark parse trees to OmniView AST nodes.

    When constructed with a preprocessor block map, RAW_BLOCK tokens
    are resolved by delegating to OmniLang's parse_inline_block().
    """

    def __init__(self, block_map: dict | None = None):
        """Initialize the transformer.

        Args:
            block_map: Optional dict mapping placeholder IDs to BlockSlot objects.
                       When provided, RAW_BLOCK tokens are resolved via OmniLang.
        """
        super().__init__()
        self._block_map = block_map or {}
        self._delegated_blocks: dict[str, OmniLangBlock] = {}

    @property
    def delegated_blocks(self) -> dict:
        """Access blocks that were parsed by OmniLang."""
        return self._delegated_blocks

    # ── Helpers ────────────────────────────────────────

    @staticmethod
    def _tok_str(item) -> str:
        """Extract string value from a Token or return the string."""
        if isinstance(item, Token):
            return str(item)
        return str(item)

    @staticmethod
    def _strip_quotes(s: str) -> str:
        """Remove surrounding double quotes from a string literal."""
        if s.startswith('"') and s.endswith('"'):
            return s[1:-1]
        return s

    @staticmethod
    def _strip_backtick(s: str) -> str:
        """Remove surrounding backticks from a template string."""
        if s.startswith('`') and s.endswith('`'):
            return s[1:-1]
        return s

    # ── Top Level ──────────────────────────────────────

    def program(self, items) -> Program:
        components = []
        stores = []
        themes = []
        lib_imports = []
        imports = []
        for item in items:
            if isinstance(item, ComponentDef):
                components.append(item)
            elif isinstance(item, StoreDef):
                stores.append(item)
            elif isinstance(item, ThemeDef):
                themes.append(item)
            elif isinstance(item, ImportStmt):
                imports.append(item)
            elif isinstance(item, str):
                lib_imports.append(item)
        return Program(
            components=tuple(components),
            stores=tuple(stores),
            themes=tuple(themes),
            imports=tuple(imports),
            lib_imports=tuple(lib_imports),
        )

    def import_lib(self, items):
        for item in items:
            if isinstance(item, Token) and item.type == "CNAME":
                return str(item)
        return ""

    def import_stmt(self, items):
        """import { Name1, Name2 } from "./path.omni" ;"""
        names = []
        path = ""
        for item in items:
            if isinstance(item, Token):
                if item.type == "CNAME":
                    names.append(str(item))
                elif item.type == "STRING_LIT":
                    path = self._strip_quotes(str(item))
        return ImportStmt(names=tuple(names), path=path)

    def start(self, items):
        # 'start' wraps 'program' — program is already transformed
        if len(items) == 1 and isinstance(items[0], Program):
            return items[0]
        return self.program(items)

    # ── Component ──────────────────────────────────────

    def component_def(self, items) -> ComponentDef:
        # component CNAME ["<" generic_params ">"] "{" component_body "}"
        name = ""
        generic_params = ()
        body_start = 1
        for i, item in enumerate(items):
            if isinstance(item, Token):
                s = str(item)
                if s not in ('component', '<', '>', '{', '}'):
                    name = s
            elif isinstance(item, tuple) and item and isinstance(item[0], str):
                # generic_params: tuple of strings
                generic_params = item
                body_start = i + 1
            elif isinstance(item, tuple) and not name:
                # Could be generic_params
                pass
        body = items[body_start:]
        props, signals, computeds, events, refs = [], [], [], [], []
        stores, lifecycles, fns = [], [], []
        view, style = None, None

        # Flatten: body may contain a single tuple from component_body
        flat = []
        for item in body:
            if isinstance(item, tuple):
                flat.extend(item)
            else:
                flat.append(item)

        for item in flat:
            if isinstance(item, PropDecl):
                props.append(item)
            elif isinstance(item, SignalDecl):
                signals.append(item)
            elif isinstance(item, ComputedDecl):
                computeds.append(item)
            elif isinstance(item, EventDecl):
                events.append(item)
            elif isinstance(item, RefDecl):
                refs.append(item)
            elif isinstance(item, StoreRef):
                stores.append(item)
            elif isinstance(item, LifecycleHook):
                lifecycles.append(item)
            elif isinstance(item, FnDef):
                fns.append(item)
            elif isinstance(item, ViewBlock):
                view = item
            elif isinstance(item, StyleBlock):
                style = item

        return ComponentDef(
            name=name,
            generic_params=generic_params,
            props=tuple(props),
            signals=tuple(signals),
            computeds=tuple(computeds),
            events=tuple(events),
            refs=tuple(refs),
            stores=tuple(stores),
            lifecycles=tuple(lifecycles),
            fns=tuple(fns),
            view=view,
            style=style,
        )

    def component_body(self, items):
        return tuple(items)

    def generic_params(self, items) -> tuple:
        return tuple(self._tok_str(item) for item in items if isinstance(item, Token))

    # ── Component Members ──────────────────────────────

    def prop_decl(self, items) -> PropDecl:
        # CNAME ":" type ["=" expr] ";"
        name = self._tok_str(items[0])
        type_ann = items[1] if len(items) > 1 else None
        default = items[2] if len(items) > 2 else None
        return PropDecl(name=name, type=type_ann, default_value=default)

    def signal_decl(self, items) -> SignalDecl:
        # CNAME ":" type "=" expr ";"
        name = self._tok_str(items[0])
        type_ann = items[1]
        value = items[2]
        return SignalDecl(name=name, type=type_ann, initial_value=value)

    def computed_decl(self, items) -> ComputedDecl:
        # CNAME ":" type "=" expr ";"
        name = self._tok_str(items[0])
        type_ann = items[1]
        body = items[2]
        return ComputedDecl(name=name, type=type_ann, body=body)

    def event_decl(self, items) -> EventDecl:
        # CNAME ":" type ["?"] ";"
        name = self._tok_str(items[0])
        type_ann = items[1]
        is_optional = False
        # Check if there's a '?' token (it would be items[2] before ';')
        for item in items[2:]:
            s = self._tok_str(item)
            if s == '?':
                is_optional = True
        return EventDecl(name=name, payload_type=type_ann, is_optional=is_optional)

    def ref_decl(self, items) -> RefDecl:
        name = self._tok_str(items[0])
        type_ann = items[1]
        return RefDecl(name=name, ref_type=type_ann)

    def store_ref(self, items) -> StoreRef:
        # AT_STORE CNAME ";"
        name = self._tok_str(items[0]) if items else self._tok_str(items[1]) if len(items) > 1 else ""
        for item in items:
            s = self._tok_str(item)
            if s not in ('@store', ';'):
                name = s
                break
        return StoreRef(store_name=name)

    # ── Store ──────────────────────────────────────────

    def store_def(self, items) -> StoreDef:
        name = self._tok_str(items[0])
        body = items[1:] if len(items) > 1 else []
        signals, getters, actions = [], [], []
        flat = []
        for item in body:
            if isinstance(item, tuple):
                flat.extend(item)
            else:
                flat.append(item)
        for item in flat:
            if isinstance(item, StoreSignalDecl):
                signals.append(item)
            elif isinstance(item, StoreGetterDecl):
                getters.append(item)
            elif isinstance(item, StoreActionDecl):
                actions.append(item)
        return StoreDef(
            name=name,
            signals=tuple(signals),
            getters=tuple(getters),
            actions=tuple(actions),
        )

    def store_body(self, items):
        return tuple(items)

    def store_signal(self, items) -> StoreSignalDecl:
        name = self._tok_str(items[0])
        type_ann = items[1]
        value = items[2] if len(items) > 2 else None
        return StoreSignalDecl(name=name, type=type_ann, initial_value=value)

    def store_getter(self, items) -> StoreGetterDecl:
        name = self._tok_str(items[0])
        type_ann = items[1]
        body = items[2] if len(items) > 2 else None
        return StoreGetterDecl(name=name, type=type_ann, body=body)

    def store_action(self, items) -> StoreActionDecl:
        # ["async"] "action" CNAME "(" [params] ")" ["->" return_type] block
        is_async = False
        name_idx = 0
        for i, item in enumerate(items):
            if self._tok_str(item) == 'async':
                is_async = True
                name_idx += 1
            elif self._tok_str(item) == 'action':
                name_idx += 1
            else:
                break
        name = self._tok_str(items[name_idx]) if name_idx < len(items) else ""
        params = ()
        return_type = None
        body = None
        # Find params, return_type, body in remaining items
        rest = items[name_idx + 1:]
        i = 0
        while i < len(rest):
            item = rest[i]
            if isinstance(item, tuple) and params == ():
                params = item
            elif isinstance(item, TypeAnnotation):
                return_type = item
            elif isinstance(item, (Block,)):
                body = item
            elif isinstance(item, tuple) and len(item) > 0 and isinstance(item[0], Param):
                params = item
            i += 1
        return StoreActionDecl(
            name=name, params=params,
            return_type=return_type, body=body,
            is_async=is_async,
        )

    # ── Theme ──────────────────────────────────────────

    def theme_body(self, items):
        return tuple(item for item in items if isinstance(item, ThemeCategory))

    def theme_def(self, items) -> ThemeDef:
        # "theme" "{" theme_body "}"
        # items may contain the theme_body tuple or individual ThemeCategory items
        categories = []
        for item in items:
            if isinstance(item, ThemeCategory):
                categories.append(item)
            elif isinstance(item, tuple):
                for sub in item:
                    if isinstance(sub, ThemeCategory):
                        categories.append(sub)
        return ThemeDef(categories=tuple(categories))

    def theme_category(self, items) -> ThemeCategory:
        name = self._tok_str(items[0])
        values = []
        for item in items[1:]:
            if isinstance(item, tuple) and len(item) == 2:
                values.append(item)
        return ThemeCategory(name=name, values=tuple(values))

    def theme_value(self, items) -> tuple:
        key = self._tok_str(items[0])
        val = self._strip_quotes(self._tok_str(items[1]))
        return (key, val)

    # ── Lifecycle ──────────────────────────────────────

    def lifecycle_hook(self, items) -> LifecycleHook:
        # AT_MOUNT block | AT_UNMOUNT block | AT_EFFECT "(" CNAME ")" block
        first = self._tok_str(items[0])
        kind = 'mount' if 'mount' in first else 'unmount' if 'unmount' in first else 'effect'
        deps = None
        body = None
        # Find the block (last item) and deps (the CNAME before the block)
        for item in reversed(items):
            if isinstance(item, Block):
                body = item
            elif isinstance(item, Token) and kind == 'effect' and not deps:
                s = str(item)
                if s not in ('@effect', '(', ')', '@mount', '@unmount'):
                    deps = (s,)
        return LifecycleHook(kind=kind, body=body, deps=deps)

    # ── Functions ──────────────────────────────────────

    def fn_def(self, items) -> FnDef:
        # Lark strips literal terminals — items are: [CNAME(name), return_type?, params?, block]
        is_async = False
        name = ""
        params = ()
        return_type = None
        body = None
        for item in items:
            if isinstance(item, Token):
                s = str(item)
                if s == 'async':
                    is_async = True
                elif s not in ('fn',):
                    if not name:
                        name = s
            elif isinstance(item, TypeAnnotation):
                return_type = item
            elif isinstance(item, Block):
                body = item
            elif isinstance(item, tuple):
                # Could be params tuple or nested
                if item and isinstance(item[0], Param):
                    params = item
        return FnDef(
            name=name, params=params,
            return_type=return_type, body=body,
            is_async=is_async,
        )

    def params(self, items) -> tuple:
        return tuple(item for item in items if isinstance(item, Param))

    def param(self, items) -> Param:
        name = self._tok_str(items[0])
        type_ann = items[1] if len(items) > 1 else None
        return Param(name=name, type=type_ann)

    # ── Types ──────────────────────────────────────────

    def type(self, items):
        if len(items) == 1:
            return items[0]
        return items[0] if items else None

    def union_type(self, items) -> TypeAnnotation | UnionType:
        if len(items) == 1:
            return items[0]
        if len(items) > 1:
            return UnionType(types=list(items))
        return items[0] if items else TypeAnnotation(name="void")

    def named_type(self, items) -> TypeAnnotation:
        name = self._tok_str(items[0])
        type_args = ()
        if len(items) > 1 and items[1] is not None:
            ta = items[1]
            if isinstance(ta, tuple):
                type_args = ta
            elif isinstance(ta, (TypeAnnotation, UnionType)):
                type_args = (ta,)
        return TypeAnnotation(name=name, type_args=type_args)

    def optional_type(self, items) -> UnionType:
        # TYPE_NAME "?" or CNAME "?"
        type_name = self._tok_str(items[0])
        return UnionType(types=[
            TypeAnnotation(name=type_name),
            TypeAnnotation(name="null"),
        ])

    def type_args(self, items):
        return tuple(items)

    def return_type(self, items):
        if len(items) == 1:
            return items[0]
        return items[0] if items else None

    # ── View Block ─────────────────────────────────────

    def view_block(self, items) -> ViewBlock:
        # "view" "{" view_content* "}"
        children = []
        for item in items:
            if isinstance(item, (Element, IfDirective, ForDirective, SlotDirective,
                                NativeBlock, FallbackBlock, ErrorBoundary,
                                TextNode, Interpolation)):
                children.append(item)
        return ViewBlock(children=tuple(children))

    def view_content(self, items):
        if len(items) == 1:
            return items[0]
        return items[0] if items else None

    # ── Elements ───────────────────────────────────────

    def element(self, items) -> Element:
        # self-closing: "<" CNAME element_body "/>"
        # paired: "<" CNAME element_body ">" element_children "</" CNAME ">"
        tag = self._tok_str(items[0])
        props = ()
        children = ()
        test_id = None

        # Find element_body and element_children
        for item in items[1:]:
            if isinstance(item, tuple):
                for sub in item:
                    if isinstance(sub, ElementProp):
                        if sub.name == 'test-id':
                            if isinstance(sub.value, StaticProp) and isinstance(sub.value.value, str):
                                test_id = sub.value.value
                        else:
                            props = props + (sub,)
                    elif isinstance(sub, (Element, IfDirective, ForDirective, SlotDirective,
                                         NativeBlock, FallbackBlock, ErrorBoundary, TextNode, Interpolation)):
                        children = children + (sub,)
                    elif isinstance(sub, str):
                        test_id = sub
            elif isinstance(item, ElementProp):
                props = props + (item,)
            elif isinstance(item, (Element, IfDirective, ForDirective, SlotDirective,
                                  NativeBlock, FallbackBlock, ErrorBoundary, TextNode, Interpolation)):
                children = children + (item,)

        return Element(tag=tag, props=props, children=children, test_id=test_id)

    def element_body(self, items):
        return tuple(items)

    def element_children(self, items):
        result = []
        for item in items:
            if isinstance(item, (Element, IfDirective, ForDirective, SlotDirective,
                                NativeBlock, FallbackBlock, ErrorBoundary, TextNode, Interpolation)):
                result.append(item)
        return tuple(result)

    # ── Element Props ──────────────────────────────────

    def element_prop(self, items):
        if len(items) == 1:
            return items[0]
        return items[0] if items else None

    def prop_value(self, items):
        """STRING_LIT → plain string; "{" expr "}" → expression."""
        if len(items) == 1:
            item = items[0]
            if isinstance(item, Token):
                s = str(item)
                # Only strip quotes for STRING_LIT, not other tokens
                if item.type == 'STRING_LIT':
                    return self._strip_quotes(s)
                return s
            return item
        # "{" expr "}" → return the expression (items[1] is the expr)
        if len(items) >= 3:
            return items[1]
        return items[0] if items else ""

    def static_prop(self, items) -> ElementProp:
        name = self._tok_str(items[0])
        value_item = items[1] if len(items) > 1 else ""
        if isinstance(value_item, Token):
            s = str(value_item)
            # STRING_LIT: strip quotes; other tokens: use as-is
            if value_item.type == 'STRING_LIT':
                s = self._strip_quotes(s)
            value = StaticProp(value=s)
        elif isinstance(value_item, (str, int, float, bool)):
            value = StaticProp(value=value_item)
        else:
            # Expression from "{" expr "}"
            value = StaticProp(value=value_item)
        return ElementProp(name=name, value=value)

    def bind_prop(self, items) -> ElementProp:
        # "bind:" CNAME "=" "{" CNAME ("." CNAME)? "}"
        prop_name_raw = self._tok_str(items[0])
        prop_name = prop_name_raw.replace('bind:', '') if prop_name_raw.startswith('bind:') else prop_name_raw
        # Collect CNAME tokens after "{" — may be simple or dotted (store.field)
        names = []
        for item in items[1:]:
            if isinstance(item, Token) and item.type == 'CNAME':
                names.append(str(item))
        signal_ref = ".".join(names) if names else ""
        return ElementProp(name=prop_name, value=BindProp(signal_ref=signal_ref))

    def event_prop(self, items) -> ElementProp:
        # CNAME "=" "{" event_handler "}"
        name = self._tok_str(items[0])
        handler = items[1] if len(items) > 1 else None
        if isinstance(handler, tuple) and len(handler) == 1:
            handler = handler[0]
        # Detect ref={name} — "ref" keyword with bare FnRef → treat as RefProp
        if name == "ref" and isinstance(handler, FnRef):
            return ElementProp(name="ref", value=RefProp(ref_name=handler.fn_name))
        return ElementProp(name=name, value=EventProp(handler=handler))

    def class_prop(self, items) -> ElementProp:
        # "class" "=" "{" class_value "}"
        val = items[0] if len(items) > 0 else StringClass(value="")
        return ElementProp(name="class", value=ClassProp(class_value=val))

    def class_map_prop(self, items) -> ElementProp:
        # "class" "=" "{" "class_map" "(" "{" class_map_entries "}" ")" "}"
        entries = ()
        for item in items:
            if isinstance(item, tuple) and item and isinstance(item[0], tuple) and len(item[0]) == 2:
                entries = item
                break
        return ElementProp(
            name="class",
            value=ClassProp(class_value=ClassMapClass(entries=entries)),
        )

    def class_map_entries(self, items) -> tuple:
        entries = []
        for item in items:
            if isinstance(item, tuple) and len(item) == 2:
                entries.append(item)
        return tuple(entries)

    def class_map_entry(self, items) -> tuple:
        key = self._tok_str(items[0])
        val = items[1] if len(items) > 1 else None
        return (key, val)

    def style_prop(self, items) -> ElementProp:
        # "style" "=" "{" "{" style_entries "}" "}"
        entries = ()
        for item in items:
            if isinstance(item, tuple):
                entries = item
                break
        return ElementProp(
            name="style",
            value=StyleProp(properties=entries if entries else ()),
        )

    def style_entries(self, items) -> tuple:
        entries = []
        for item in items:
            if isinstance(item, tuple) and len(item) == 2:
                entries.append(item)
        return tuple(entries)

    def style_entry(self, items) -> tuple:
        key = self._tok_str(items[0])
        val = items[1] if len(items) > 1 else None
        return (key, val)

    def test_id_prop(self, items) -> ElementProp:
        val = self._strip_quotes(self._tok_str(items[0]))
        return ElementProp(name="test-id", value=StaticProp(value=val))

    def ref_prop(self, items) -> ElementProp:
        # "ref" "=" "{" CNAME "}"
        ref_name = ""
        for item in items:
            s = self._tok_str(item) if isinstance(item, Token) else ""
            if s not in ('ref', '=', '{', '}'):
                ref_name = s
        return ElementProp(name="ref", value=RefProp(ref_name=ref_name))

    def class_value(self, items):
        if len(items) == 1:
            item = items[0]
            s = self._tok_str(item)
            if s.startswith('"'):
                return StringClass(value=self._strip_quotes(s))
            return item
        # "style" "." CNAME
        if len(items) >= 3:
            return StyleRefClass(class_name=self._tok_str(items[2]))
        return StringClass(value="")

    def style_ref_class(self, items):
        if len(items) >= 2:
            return StyleRefClass(class_name=self._tok_str(items[1]))
        return items[0] if items else StyleRefClass(class_name="")

    def string_class(self, items):
        s = self._tok_str(items[0]) if items else ""
        return StringClass(value=self._strip_quotes(s))

    # ── Event Handlers ─────────────────────────────────

    def event_handler(self, items):
        if len(items) == 1:
            return items[0]
        return items[0] if items else None

    def inline_fn_handler(self, items) -> InlineFn:
        # "fn" "(" [handler_params] ")" block
        params = ()
        body = None
        for item in items:
            if isinstance(item, Block):
                body = item
            elif isinstance(item, tuple) and item and isinstance(item[0], Param):
                params = item
        return InlineFn(params=params, body=body)

    def fn_ref_handler(self, items) -> FnRef:
        name = self._tok_str(items[0]) if items else ""
        return FnRef(fn_name=name)

    def store_action_handler(self, items) -> StoreActionRef:
        # CNAME "." CNAME
        parts = [self._tok_str(item) for item in items if isinstance(item, Token)]
        return StoreActionRef(action_path=".".join(parts))

    def handler_params(self, items) -> tuple:
        return tuple(item for item in items if isinstance(item, Param))

    def handler_param(self, items) -> Param:
        name = self._tok_str(items[0])
        type_ann = items[1] if len(items) > 1 else None
        return Param(name=name, type=type_ann)

    # ── Directives ─────────────────────────────────────

    def if_directive(self, items) -> IfDirective:
        # AT_IF guard_expr view_block_body (AT_ELSE "if" guard_expr view_block_body)* [AT_ELSE view_block_body]
        branches = []
        else_body = None
        i = 0
        while i < len(items):
            item = items[i]
            s = self._tok_str(item) if isinstance(item, Token) else ""
            if s in ('@if',):
                cond, body, next_i = self._extract_branch(items, i + 1)
                if cond is not None:
                    branches.append(IfBranch(condition=cond, body=body))
                i = next_i
            elif s in ('@else',):
                # Look ahead: is it @else if or @else?
                next_s = self._tok_str(items[i + 1]) if i + 1 < len(items) and isinstance(items[i + 1], Token) else ""
                if next_s == 'if':
                    # @else if — extract condition and body, skip the "if" token
                    cond, body, next_i = self._extract_branch(items, i + 2)
                    if cond is not None:
                        branches.append(IfBranch(condition=cond, body=body))
                    i = next_i
                else:
                    # Plain @else — find the view_block_body
                    for j in range(i + 1, len(items)):
                        if isinstance(items[j], tuple):
                            else_body = items[j]
                            i = j
                            break
            i += 1

        return IfDirective(branches=tuple(branches), else_body=else_body)

    def _extract_branch(self, items, start_idx):
        """Extract guard_expr and view_block_body starting at start_idx.
        Returns (cond, body, next_idx)."""
        cond = None
        body = ()
        j = start_idx
        while j < len(items):
            sj = self._tok_str(items[j]) if isinstance(items[j], Token) else None
            if sj in ('@else', '@if'):
                break
            if isinstance(items[j], tuple):
                body = items[j]
                j += 1
                break
            if not isinstance(items[j], Token):
                cond = items[j]
            j += 1
        return cond, body, j - 1

    def for_directive(self, items) -> ForDirective:
        # AT_FOR CNAME "in" expr view_block_body [AT_EMPTY view_block_body]
        item_name = ""
        iterable = None
        body = ()
        empty_body = None
        found_first_body = False

        for item in items:
            if isinstance(item, Token):
                s = str(item)
                if s not in ('@for', 'in', '@empty'):
                    item_name = s
            elif isinstance(item, tuple):
                # view_block_body result: tuple of view nodes
                if not found_first_body:
                    body = item
                    found_first_body = True
                else:
                    empty_body = item
            elif isinstance(item, Block):
                body = ()
            elif item is not None and not isinstance(item, Token):
                iterable = item

        return ForDirective(
            item_name=item_name,
            iterable=iterable,
            body=body,
            empty_body=empty_body,
        )

    def view_block_body(self, items):
        """view_block_body: '{' view_content* '}'"""
        result = []
        for item in items:
            if isinstance(item, (Element, IfDirective, ForDirective, SlotDirective,
                                NativeBlock, FallbackBlock, ErrorBoundary, TextNode, Interpolation)):
                result.append(item)
        return tuple(result)

    def slot_directive(self, items) -> SlotDirective:
        # AT_SLOT ["name" "=" STRING_LIT]
        name = None
        for item in items:
            s = self._tok_str(item)
            if s not in ('@slot', 'name', '='):
                name = self._strip_quotes(s)
        return SlotDirective(name=name)

    def native_block(self, items) -> NativeBlock:
        target = ""
        content = ""
        for item in items:
            s = self._tok_str(item)
            if s not in ('@native', '(', ')', '{', '}'):
                if not target:
                    target = s
                else:
                    content = s
        return NativeBlock(target=target, raw_content=content)

    def fallback_block(self, items) -> FallbackBlock:
        # AT_FALLBACK view_block_body
        children = []
        for item in items:
            if isinstance(item, tuple):
                children.extend(item)
            elif isinstance(item, (Element, IfDirective, ForDirective, SlotDirective,
                                  NativeBlock, TextNode, Interpolation)):
                children.append(item)
        return FallbackBlock(children=tuple(children))

    def error_boundary(self, items) -> ErrorBoundary:
        # AT_ERROR "(" CNAME ")" view_block_body
        name = ""
        children = ()
        for item in items:
            s = self._tok_str(item) if isinstance(item, Token) else ""
            if isinstance(item, tuple):
                children = item
            elif s not in ('@error', '(', ')'):
                name = s
        return ErrorBoundary(name=name, children=children)

    # ── Text Content ───────────────────────────────────

    def text_content(self, items):
        if len(items) == 1:
            return items[0]
        return items[0] if items else None

    def TEXT_PLAIN(self, token: Token) -> TextNode:
        return TextNode(content=str(token))

    def interpolation(self, items) -> Interpolation:
        # "{" expr "}"
        expr = items[0] if items else None
        return Interpolation(expr=expr)

    # ── Expressions ────────────────────────────────────

    def expr(self, items):
        if len(items) == 1:
            return items[0]
        return items[0] if items else None

    def ternary_expr(self, items):
        if len(items) == 1:
            return items[0]
        # logic_expr "?" expr ":" expr
        if len(items) >= 5:
            return TernaryExpr(
                condition=items[0],
                then_expr=items[2],
                else_expr=items[4],
            )
        return items[0]

    def logic_expr(self, items):
        result = items[0]
        i = 1
        while i < len(items) - 1:
            op = self._tok_str(items[i])
            right = items[i + 1]
            result = BinaryOp(left=result, op=op, right=right)
            i += 2
        return result

    def comp_expr(self, items):
        result = items[0]
        i = 1
        while i < len(items) - 1:
            op = self._tok_str(items[i])
            right = items[i + 1]
            result = BinaryOp(left=result, op=op, right=right)
            i += 2
        return result

    def add_expr(self, items):
        result = items[0]
        i = 1
        while i < len(items) - 1:
            op = self._tok_str(items[i])
            right = items[i + 1]
            result = BinaryOp(left=result, op=op, right=right)
            i += 2
        return result

    def mul_expr(self, items):
        result = items[0]
        i = 1
        while i < len(items) - 1:
            op = self._tok_str(items[i])
            right = items[i + 1]
            result = BinaryOp(left=result, op=op, right=right)
            i += 2
        return result

    def unary_expr(self, items):
        if len(items) == 1:
            return items[0]
        if len(items) == 2:
            op = self._tok_str(items[0])
            return UnaryOp(op=op, operand=items[1])
        return items[0]

    def postfix_expr(self, items):
        if len(items) == 1:
            return items[0]
        result = items[0]
        i = 1
        while i < len(items):
            item = items[i]
            if isinstance(item, FieldAccess):
                result = FieldAccess(object=result, field=item.field, span=item.span)
            elif isinstance(item, Token) and str(item) == '.':
                # Inlined DOT CNAME: next item is the field name
                if i + 1 < len(items):
                    field_token = items[i + 1]
                    field_name = self._tok_str(field_token)
                    # Check for call args: DOT CNAME "(" call_args ")"
                    if i + 2 < len(items) and isinstance(items[i + 2], Token) and str(items[i + 2]) == '(':
                        args = items[i + 3] if i + 3 < len(items) and isinstance(items[i + 3], tuple) else ()
                        result = MethodCall(object=result, method=field_name, args=args)
                        i += 4
                    else:
                        result = FieldAccess(object=result, field=field_name)
                        i += 1
            elif isinstance(item, tuple):
                # Call args tuple from func_call
                if hasattr(result, 'func'):
                    result = FuncCall(func=result.func, args=item)
                else:
                    result = FuncCall(func=result, args=item)
            i += 1
        return result

    def atom_expr(self, items):
        if len(items) == 1:
            return items[0]
        return items[0] if items else None

    def var_ref(self, items) -> VarRef:
        if len(items) == 1:
            return VarRef(name=self._tok_str(items[0]))
        return VarRef(name=self._tok_str(items[0]) if items else "")

    def member_access(self, items) -> FieldAccess:
        # DOT CNAME
        field = self._tok_str(items[1]) if len(items) > 1 else ""
        return FieldAccess(object=None, field=field)

    def call_args(self, items) -> tuple:
        return tuple(items)

    def func_call_expr(self, items) -> FuncCall:
        # CNAME "(" [call_args] ")" — standalone function call
        func_name = self._tok_str(items[0])
        args = items[1] if len(items) > 1 and isinstance(items[1], tuple) else ()
        return FuncCall(func=VarRef(name=func_name), args=args)

    # ── Literals ───────────────────────────────────────

    def literal(self, items):
        if len(items) == 1:
            return items[0]
        return items[0] if items else None

    def bool_lit(self, items):
        s = self._tok_str(items[0])
        return Literal(value=(s == 'true'), kind='bool')

    def string_lit(self, items):
        s = self._strip_quotes(self._tok_str(items[0]))
        return Literal(value=s, kind='String')

    def number_lit(self, items):
        s = self._tok_str(items[0])
        if '.' in s or 'e' in s.lower():
            return Literal(value=float(s), kind='f64')
        return Literal(value=int(s), kind='i32')

    def array_lit(self, items):
        exprs = []
        for item in items:
            if isinstance(item, Token):
                s = str(item)
                if s in ('[', ']', ','):
                    continue
            if isinstance(item, SpreadExpr):
                exprs.append(item)
            else:
                exprs.append(item)
        return ArrayLiteral(elements=tuple(exprs))

    def array_element(self, items):
        # Inlined rule: SPREAD expr → SpreadExpr, or expr → direct
        if len(items) == 1:
            return items[0]
        # SPREAD token + expr
        if len(items) >= 2:
            expr = items[1]
            return SpreadExpr(expr=expr)
        return items[0] if items else None

    def map_lit(self, items):
        entries = []
        i = 0
        while i + 1 < len(items):
            entry = items[i]
            i += 1
            if isinstance(entry, tuple) and len(entry) == 2:
                entries.append(entry)
        return MapLiteral(entries=tuple(entries))

    def map_entry(self, items) -> tuple:
        if len(items) >= 2:
            return (items[0], items[1])
        return (None, None)

    def template_string(self, items) -> TemplateString:
        raw = self._strip_backtick(self._tok_str(items[0]))
        return TemplateString(raw=raw, spans=())

    def if_expr(self, items) -> IfExpr:
        # "if" guard_expr block "else" block
        cond = None
        then_block = None
        else_block = None
        for item in items:
            s = self._tok_str(item) if isinstance(item, Token) else ""
            if isinstance(item, Block) and then_block is None:
                then_block = item
            elif isinstance(item, Block):
                else_block = item
            elif not isinstance(item, Token) or s not in ('if', 'else'):
                cond = item
        return IfExpr(condition=cond, then_block=then_block, else_block=else_block)

    def guard_expr(self, items):
        if len(items) == 1:
            return items[0]
        return items[0] if items else None

    # ── Statements ─────────────────────────────────────

    def stmt(self, items):
        if len(items) == 1:
            return items[0]
        return items[0] if items else None

    def var_decl(self, items) -> VarDeclStmt:
        # "let" ["mut"] CNAME [":" type] "=" expr ";"
        is_mutable = False
        name = ""
        type_ann = None
        value = None
        i = 0
        while i < len(items):
            s = self._tok_str(items[i]) if isinstance(items[i], Token) else "__expr__"
            if s == 'mut':
                is_mutable = True
            elif s == '=':
                value = items[i + 1] if i + 1 < len(items) else None
                i += 1
            elif isinstance(items[i], TypeAnnotation):
                type_ann = items[i]
            elif s not in ('let', ';') and name == "":
                name = s
            i += 1
        return VarDeclStmt(name=name, type=type_ann, value=value, is_mutable=is_mutable)

    def assign(self, items) -> AssignStmt:
        # CNAME ("." CNAME)? ASSIGN_OP expr ";"
        # Simple: count = 1;  Dotted: store.field = value;
        target_parts = []
        op = "="
        value = None
        found_op = False
        for item in items:
            s = self._tok_str(item) if isinstance(item, Token) else ""
            if isinstance(item, Token) and item.type == 'CNAME' and not found_op:
                target_parts.append(str(item))
            elif s in ('=', '+=', '-=', '*=', '/=', '%='):
                op = s
                found_op = True
            elif s == '.':
                continue
            elif s not in (';',) and found_op:
                value = item
        target = ".".join(target_parts) if target_parts else ""
        return AssignStmt(target=target, op=op, value=value)

    def if_stmt(self, items) -> IfStmt:
        cond = None
        then_body = None
        else_body = None
        for item in items:
            if isinstance(item, Block) and then_body is None:
                then_body = item
            elif isinstance(item, Block):
                else_body = item
            elif isinstance(item, IfStmt):
                else_body = Block(statements=(item,))
            elif not isinstance(item, Token):
                cond = item
        return IfStmt(condition=cond, then_body=then_body, else_body=else_body)

    def while_stmt(self, items) -> WhileStmt:
        cond = None
        body = None
        for item in items:
            if isinstance(item, Block):
                body = item
            elif not isinstance(item, Token) or self._tok_str(item) not in ('while',):
                cond = item
        return WhileStmt(condition=cond, body=body)

    def for_stmt(self, items) -> ForInStmt:
        name = ""
        iterable = None
        body = None
        for item in items:
            s = self._tok_str(item) if isinstance(item, Token) else ""
            if isinstance(item, Block):
                body = item
            elif s not in ('for', 'in', ';'):
                if not name:
                    name = s
                else:
                    iterable = item
            elif not isinstance(item, Token):
                iterable = item
        return ForInStmt(item_name=name, iterable=iterable, body=body)

    def break_stmt(self, items) -> ReturnStmt:
        return ReturnStmt(value=None)  # Placeholder; proper BreakStmt would be better

    def continue_stmt(self, items) -> ReturnStmt:
        return ReturnStmt(value=None)  # Placeholder

    def return_stmt(self, items) -> ReturnStmt:
        value = None
        for item in items:
            s = self._tok_str(item) if isinstance(item, Token) else ""
            if s not in ('return', ';'):
                value = item
        return ReturnStmt(value=value)

    def print_stmt(self, items) -> ExprStmt:
        expr = items[1] if len(items) > 1 else None
        return ExprStmt(expr=expr)

    def event_emit_stmt(self, items) -> EventEmitStmt:
        # "event" CNAME "(" [expr ("," expr)*] ")" ";"
        name = self._tok_str(items[0]) if items else ""
        args = []
        for item in items[1:]:
            s = self._tok_str(item) if isinstance(item, Token) else ""
            if s not in ('(', ')', ',', ';') and item is not None:
                args.append(item)
        return EventEmitStmt(event_name=name, args=tuple(args))

    def expr_stmt(self, items) -> ExprStmt:
        expr = items[0] if items else None
        return ExprStmt(expr=expr)

    # ── Block ──────────────────────────────────────────

    def block(self, items) -> Block:
        # Case 1: RAW_BLOCK token from preprocessor → delegate to OmniLang
        if len(items) == 1 and isinstance(items[0], Token) and items[0].type == 'RAW_BLOCK':
            return self._resolve_raw_block(items[0])

        # Case 2: Standard statement block (no preprocessor)
        stmts = []
        for item in items:
            if isinstance(item, Stmt):
                stmts.append(item)
            elif isinstance(item, (VarDeclStmt, AssignStmt, IfStmt, WhileStmt,
                                  ForInStmt, ReturnStmt, ExprStmt, EventEmitStmt)):
                stmts.append(item)
        return Block(statements=tuple(stmts))

    def _resolve_raw_block(self, token: Token) -> Block:
        """Resolve a RAW_BLOCK placeholder by delegating to OmniLang.

        Looks up the original block text from the preprocessor block map,
        calls OmniLang's parse_inline_block(), and converts the result
        to OmniView AST types.
        """
        placeholder_id = str(token)
        slot = self._block_map.get(placeholder_id)

        if slot is None:
            # No preprocessor data available — return empty block
            return Block(statements=())

        from omnilang.parser_helpers import parse_inline_block as parse_block
        try:
            omni_block = parse_block(slot.original_text)
            self._delegated_blocks[placeholder_id] = omni_block
            # Convert OmniLang Block → OmniView Block
            omni_stmts = self._convert_omnilang_stmts(omni_block.stmts if omni_block.stmts else [])
            return Block(statements=tuple(omni_stmts))
        except Exception:
            # Parse error — return empty block, errors will be diagnosed later
            return Block(statements=())

    def _convert_omnilang_stmts(self, stmts: list) -> list:
        """Convert OmniLang statement nodes to OmniView statement nodes."""
        from omnilang.ast_nodes import (
            VarDecl as OmniVarDecl, Assign as OmniAssign,
            IfStmt as OmniIfStmt, WhileStmt as OmniWhileStmt,
            ForInStmt as OmniForInStmt, ReturnStmt as OmniReturnStmt,
            ExprStmt as OmniExprStmt, Block as OmniBlock,
        )
        result = []
        for s in stmts:
            if isinstance(s, OmniVarDecl):
                result.append(VarDeclStmt(
                    name=s.name,
                    type=s.type_ann,
                    value=s.value,
                    is_mutable=s.is_mutable,
                ))
            elif isinstance(s, OmniAssign):
                result.append(AssignStmt(
                    target=s.target.name if hasattr(s.target, 'name') else str(s.target),
                    op=s.op,
                    value=s.value,
                ))
            elif isinstance(s, OmniExprStmt):
                result.append(ExprStmt(expr=s.expr))
            elif isinstance(s, OmniReturnStmt):
                result.append(ReturnStmt(value=s.value))
            elif isinstance(s, OmniIfStmt):
                result.append(IfStmt(
                    condition=s.condition,
                    then_body=self._convert_omnilang_block(s.then_body),
                    else_body=self._convert_omnilang_block(s.else_body) if s.else_body else None,
                ))
            elif isinstance(s, OmniWhileStmt):
                result.append(WhileStmt(
                    condition=s.condition,
                    body=self._convert_omnilang_block(s.body),
                ))
            # ForInStmt, other types handled as needed
            else:
                # For unknown types, wrap as ExprStmt
                if hasattr(s, 'expr'):
                    result.append(ExprStmt(expr=s.expr))
        return result

    def _convert_omnilang_block(self, omni_block) -> Block | None:
        """Convert an OmniLang Block to OmniView Block."""
        if omni_block is None:
            return None
        stmts = self._convert_omnilang_stmts(omni_block.stmts if omni_block.stmts else [])
        return Block(statements=tuple(stmts))

    # ── Style Block ────────────────────────────────────

    def style_block(self, items) -> StyleBlock:
        rules = []
        for item in items:
            if isinstance(item, StyleRule):
                rules.append(item)
        return StyleBlock(rules=tuple(rules))

    def style_rule(self, items) -> StyleRule:
        # "." CNAME "{" style_properties "}"
        class_name = ""
        properties = []
        for item in items:
            s = self._tok_str(item) if isinstance(item, Token) else ""
            if isinstance(item, tuple) and len(item) == 2:
                properties.append(item)
            elif s not in ('.', '{', '}', ';') and not class_name:
                class_name = s
        return StyleRule(class_name=class_name, properties=tuple(properties))

    def style_property(self, items) -> tuple:
        # CNAME ":" style_value
        key = self._tok_str(items[0]) if items else ""
        val = items[1] if len(items) > 1 else None
        if isinstance(val, Token):
            val = Literal(value=str(val), kind='String')
        return (key, val)

    def style_value_list(self, items):
        """Multi-value shorthand: 8px 24px → '8px 24px'"""
        parts = []
        for item in items:
            if isinstance(item, Token):
                parts.append(str(item))
            elif isinstance(item, Literal):
                parts.append(str(item.value))
            elif hasattr(item, 'value'):
                parts.append(str(item.value))
            else:
                parts.append(str(item))
        return Literal(value=" ".join(parts), kind='String')

    def style_value(self, items):
        if len(items) == 1:
            item = items[0]
            if isinstance(item, Literal):
                return item
            if isinstance(item, Token):
                return Literal(value=str(item), kind='String')
            return item
        return str(items[0]) if items else ""

    def CSS_DIMENSION(self, token: Token) -> Literal:
        return Literal(value=str(token), kind='String')
