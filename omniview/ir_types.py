"""OmniView Intent IR type definitions.

The Intent IR is the intermediate representation between the AST and codegen.
It stores rendering INTENTS (not syntax structures), enabling each framework
to expand them idiomatically.

Design: OmniLang_v0.3_设计方案.md §十 (v3.0-final)
Only 8 pre-embedded fields per IR_FIELD_REDUCTION.md.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from typing import Any, Optional


# ═══════════════════════════════════════════════════════════
# Enums
# ═══════════════════════════════════════════════════════════

class Mutability(Enum):
    MUTABLE = "mutable"      # signal → useState/ref/use_signal/setState
    DERIVED = "derived"       # computed → useMemo/computed/use_memo


class MutationOp(Enum):
    DIRECT_ASSIGN = "direct_assign"
    SPREAD_APPEND = "spread_append"
    SPREAD_PREPEND = "spread_prepend"
    INCREMENT = "increment"
    DECREMENT = "decrement"
    MAP_UPDATE = "map_update"
    FILTER_UPDATE = "filter_update"
    FIELD_UPDATE = "field_update"


class LifecycleKind(Enum):
    MOUNT = "mount"
    UNMOUNT = "unmount"


class BindableElement(Enum):
    TEXT_INPUT = "text_input"
    TEXT_AREA = "text_area"
    CHECKBOX = "checkbox"


class BindDirection(Enum):
    BIDIRECTIONAL = "bidirectional"


class EventHandlerKind(Enum):
    INLINE_FN = "inline_fn"
    FN_REF = "fn_ref"
    STORE_ACTION = "store_action"


class ClassNameKind(Enum):
    STYLE_REF = "style_ref"
    STRING_LITERAL = "string_literal"
    CLASS_MAP = "class_map"


class BinaryOp(Enum):
    ADD="+"; SUB="-"; MUL="*"; DIV="/"; MOD="%"
    EQ="=="; NEQ="!="; LT="<"; GT=">"; LTE="<="; GTE=">="
    AND="&&"; OR="||"


class UnaryOp(Enum):
    NOT="!"; NEG="-"


class TypeKind(Enum):
    VOID="void"; I32="i32"; F64="f64"; BOOL="bool"; STRING="String"
    ARRAY="Array"; MAP="Map"; STRUCT="struct"; COMPONENT="component"
    FUNCTION="function"; NULL="null"


# ═══════════════════════════════════════════════════════════
# Basic Expression IR
# ═══════════════════════════════════════════════════════════

@dataclass(frozen=True)
class ExprIR:
    """Base class for safe expressions."""
    source_span: Optional[Any] = None


@dataclass(frozen=True)
class LiteralExpr(ExprIR):
    value: Any = None


@dataclass(frozen=True)
class IdentifierExpr(ExprIR):
    name: str = ""


@dataclass(frozen=True)
class FieldAccessExpr(ExprIR):
    object: ExprIR | None = None
    field: str = ""


@dataclass(frozen=True)
class BinaryOpExpr(ExprIR):
    op: BinaryOp = BinaryOp.ADD
    left: ExprIR | None = None
    right: ExprIR | None = None


@dataclass(frozen=True)
class UnaryOpExpr(ExprIR):
    op: UnaryOp = UnaryOp.NOT
    operand: ExprIR | None = None


@dataclass(frozen=True)
class TernaryIR(ExprIR):
    cond: ExprIR | None = None
    then_expr: ExprIR | None = None
    else_expr: ExprIR | None = None


@dataclass(frozen=True)
class StoreFieldAccessExpr(ExprIR):
    """Access to a store field (getter/signal/action) — resolved at codegen time."""
    store_alias: str = ""  # "store" or actual store name
    field_name: str = ""    # field/method name
    call_args: list[ExprIR] = field(default_factory=list)  # call args if action call


# ═══════════════════════════════════════════════════════════
# Signal & Binding References
# ═══════════════════════════════════════════════════════════

@dataclass(frozen=True)
class SignalRefIR:
    """Reference to a reactive signal — drives codegen setter/getter."""
    owner: Optional[str] = None   # None=component, "store"=store signal
    name: str = ""


@dataclass(frozen=True)
class BindingIR:
    """Loop variable binding (@for item in items)."""
    name: str = ""
    type: TypeIR | None = None


# ═══════════════════════════════════════════════════════════
# Event Handler IR
# ═══════════════════════════════════════════════════════════

@dataclass(frozen=True)
class EventHandlerIR:
    kind: EventHandlerKind = EventHandlerKind.INLINE_FN
    body: Optional[BlockIR] = None
    ref: Optional[str] = None  # FN_REF → fn name, STORE_ACTION → "store.action"


# ═══════════════════════════════════════════════════════════
# Component Instance IR
# ═══════════════════════════════════════════════════════════

@dataclass(frozen=True)
class ComponentInstanceIR:
    component_name: str = ""
    props: dict[str, ExprIR] = field(default_factory=dict)
    events: dict[str, EventHandlerIR] = field(default_factory=dict)
    slot_content: dict[str, ViewTree] = field(default_factory=dict)
    source_span: Optional[Any] = None


# ═══════════════════════════════════════════════════════════
# Type IR
# ═══════════════════════════════════════════════════════════

@dataclass(frozen=True)
class TypeIR:
    kind: TypeKind = TypeKind.VOID
    type_args: list[TypeIR] = field(default_factory=list)
    is_nullable: bool = False
    # ── Essential pre-embedded fields ──
    needs_clone: bool = False
    needs_partial_eq: bool = False


# ═══════════════════════════════════════════════════════════
# Block IR & Statements
# ═══════════════════════════════════════════════════════════

@dataclass(frozen=True)
class BlockIR:
    statements: list[StatementIR] = field(default_factory=list)
    # ── Essential pre-embedded field ──
    execution_mode: str = "client-only"


@dataclass(frozen=True)
class MutationStmt:
    intent: MutationIntent | None = None


@dataclass(frozen=True)
class EventEmitStmtIR:
    event_name: str = ""
    args: list[ExprIR] = field(default_factory=list)


@dataclass(frozen=True)
class ReturnStmtIR:
    value: Optional[ExprIR] = None


@dataclass(frozen=True)
class ExprStmtIR:
    expr: ExprIR | None = None


@dataclass(frozen=True)
class IfStmtIR:
    condition: ExprIR | None = None
    then_body: BlockIR | None = None
    else_body: BlockIR | None = None


@dataclass(frozen=True)
class LetStmtIR:
    name: str = ""
    value: ExprIR | None = None
    type: TypeIR | None = None


StatementIR = MutationStmt | EventEmitStmtIR | ReturnStmtIR | ExprStmtIR | IfStmtIR | LetStmtIR


# ═══════════════════════════════════════════════════════════
# Style & Layout IR
# ═══════════════════════════════════════════════════════════

@dataclass(frozen=True)
class ClassNameExpr:
    kind: ClassNameKind = ClassNameKind.STYLE_REF
    value: Any = None


@dataclass(frozen=True)
class StyleIntent:
    target_element: str = ""
    classes: list[ClassNameExpr] = field(default_factory=list)


@dataclass(frozen=True)
class LayoutIntent:
    target_element: str = ""
    properties: dict[str, ExprIR] = field(default_factory=dict)


@dataclass(frozen=True)
class StyleRuleIR:
    class_name: str = ""
    properties: dict[str, ExprIR] = field(default_factory=dict)
    # ── Essential pre-embedded field ──
    output_mode: str = "css-modules"
    source_span: Optional[Any] = None


# ═══════════════════════════════════════════════════════════
# Core Intent IR Types (10 types)
# ═══════════════════════════════════════════════════════════

@dataclass(frozen=True)
class StateIntent:
    """Reactive state declaration (signal)."""
    name: str = ""
    type: TypeIR | None = None
    initial_value: ExprIR | None = None
    mutability: Mutability = Mutability.MUTABLE
    # ── Essential pre-embedded fields ──
    update_semantics: str = "setter-call"
    accessor_style: str = "direct"
    is_copy: bool = False


@dataclass(frozen=True)
class MutationIntent:
    """State modification operation."""
    target: SignalRefIR | None = None
    operation: MutationOp = MutationOp.DIRECT_ASSIGN
    amount: Optional[ExprIR] = None
    item: Optional[ExprIR] = None


@dataclass(frozen=True)
class ComputedIntent:
    """Derived/computed value."""
    name: str = ""
    type: TypeIR | None = None
    body: ExprIR | None = None
    deps: list[str] = field(default_factory=list)


@dataclass(frozen=True)
class EffectIntent:
    """Side effect driven by signal dependencies."""
    deps: list[str] = field(default_factory=list)
    body: BlockIR | None = None


@dataclass(frozen=True)
class LifecycleIntent:
    """Mount/unmount lifecycle hooks."""
    kind: LifecycleKind = LifecycleKind.MOUNT
    body: BlockIR | None = None


@dataclass(frozen=True)
class ConditionalBranch:
    condition: ExprIR | None = None
    body: ViewTree | None = None


@dataclass(frozen=True)
class ConditionalIntent:
    """Conditional rendering (@if/@else if/@else)."""
    branches: list[ConditionalBranch] = field(default_factory=list)
    fallback: Optional[ViewTree] = None


@dataclass(frozen=True)
class ListRenderingIntent:
    """Dynamic list rendering (@for)."""
    iterable: ExprIR | None = None
    item_binding: BindingIR | None = None
    body: ViewTree | None = None
    key_expr: ExprIR | None = None
    empty_body: Optional[ViewTree] = None


@dataclass(frozen=True)
class BindIntent:
    """Two-way data binding."""
    source: SignalRefIR | None = None
    element_type: BindableElement = BindableElement.TEXT_INPUT
    direction: BindDirection = BindDirection.BIDIRECTIONAL


@dataclass(frozen=True)
class EventIntent:
    """Event binding on elements."""
    event_name: str = ""
    handler: EventHandlerIR | None = None


@dataclass(frozen=True)
class SlotIntent:
    """Content projection slot."""
    name: Optional[str] = None
    body: Optional[ViewTree] = None


# ═══════════════════════════════════════════════════════════
# Element IR (most numerous node type)
# ═══════════════════════════════════════════════════════════

@dataclass(frozen=True)
class ElementIR:
    """Abstract element node in the view tree."""
    tag: str = ""
    props: dict[str, ExprIR] = field(default_factory=dict)
    bindings: dict[str, SignalRefIR] = field(default_factory=dict)
    events: dict[str, EventHandlerIR] = field(default_factory=dict)
    style_classes: Optional[StyleIntent] = None
    inline_style: Optional[LayoutIntent] = None
    children: list[ViewTree] = field(default_factory=list)
    test_id: Optional[str] = None
    ref_name: Optional[str] = None  # ref={name} → framework-specific node reference
    # ── Essential pre-embedded field ──
    construction_kind: str = "tag-mapping"
    source_span: Optional[Any] = None
    # reserved for P1+ -- accessibility_label, accessibility_role
    # reserved for P3 -- keep_alive, portal_target, validation_rules, virtual_list_config


# ═══════════════════════════════════════════════════════════
# ViewTree recursive type
# ═══════════════════════════════════════════════════════════

# A view tree node is one of: ElementIR, ConditionalIntent, ListRenderingIntent,
# SlotIntent, BindIntent, or a plain str (text content).
ViewTree = ElementIR | ConditionalIntent | ListRenderingIntent | SlotIntent | BindIntent | str | None


# ═══════════════════════════════════════════════════════════
# Top-Level IR
# ═══════════════════════════════════════════════════════════

@dataclass(frozen=True)
class ComponentIR:
    """Complete component intent — the IR input to codegen."""
    name: str = ""
    generic_params: list[str] = field(default_factory=list)  # <T, U>
    props: dict[str, TypeIR] = field(default_factory=dict)
    states: list[StateIntent] = field(default_factory=list)
    computeds: list[ComputedIntent] = field(default_factory=list)
    effects: list[EffectIntent] = field(default_factory=list)
    lifecycles: list[LifecycleIntent] = field(default_factory=list)
    fns: list[dict] = field(default_factory=list)
    store_refs: list[str] = field(default_factory=list)
    refs: list[dict] = field(default_factory=list)  # ref declarations
    view: Optional[ViewTree] = None
    styles: list[StyleRuleIR] = field(default_factory=list)
    source_span: Optional[Any] = None


@dataclass(frozen=True)
class StoreIR:
    """Store intent — global state container."""
    name: str = ""
    signals: dict[str, TypeIR] = field(default_factory=dict)
    signal_defaults: dict[str, Any] = field(default_factory=dict)
    getters: dict[str, ExprIR] = field(default_factory=dict)
    actions: list[dict] = field(default_factory=list)
    source_span: Optional[Any] = None


__all__ = [
    # Enums
    "Mutability", "MutationOp", "LifecycleKind",
    "BindableElement", "BindDirection", "EventHandlerKind",
    "ClassNameKind", "BinaryOp", "UnaryOp", "TypeKind",
    # Expressions
    "ExprIR", "LiteralExpr", "IdentifierExpr", "FieldAccessExpr",
    "BinaryOpExpr", "UnaryOpExpr", "TernaryIR", "StoreFieldAccessExpr",
    # References
    "SignalRefIR", "BindingIR",
    # Handlers
    "EventHandlerIR", "ComponentInstanceIR",
    # Types
    "TypeIR",
    # Blocks & Statements
    "BlockIR", "MutationStmt", "EventEmitStmtIR", "ReturnStmtIR",
    "ExprStmtIR", "IfStmtIR", "LetStmtIR",
    # Style
    "ClassNameExpr", "StyleIntent", "LayoutIntent", "StyleRuleIR",
    # Core Intents
    "StateIntent", "MutationIntent", "ComputedIntent",
    "EffectIntent", "LifecycleIntent",
    "ConditionalBranch", "ConditionalIntent",
    "ListRenderingIntent", "BindIntent", "EventIntent", "SlotIntent",
    # Element & View
    "ElementIR", "ViewTree",
    # Top-level
    "ComponentIR", "StoreIR",
]
