"""IR JSON Schema for Rust ↔ Python contract.
Each TypedDict matches a corresponding dataclass in ir_types.py.
Key rule: every IR object carries a _kind field for dispatch.
"""
from typing import TypedDict, Literal, Optional, Any


class TypeIR(TypedDict, total=False):
    kind: str
    type_args: list  # list[TypeIR]
    is_nullable: bool
    needs_clone: bool
    needs_partial_eq: bool


class LiteralExpr(TypedDict):
    _kind: Literal["LiteralExpr"]
    value: Any


class IdentifierExpr(TypedDict):
    _kind: Literal["IdentifierExpr"]
    name: str


class FieldAccessExpr(TypedDict):
    _kind: Literal["FieldAccessExpr"]
    object: dict
    field: str


class BinaryOpExpr(TypedDict):
    _kind: Literal["BinaryOpExpr"]
    op: str
    left: dict
    right: dict


class UnaryOpExpr(TypedDict):
    _kind: Literal["UnaryOpExpr"]
    op: str
    operand: dict


class TernaryIR(TypedDict):
    _kind: Literal["TernaryIR"]
    cond: dict
    then_expr: dict
    else_expr: dict


class StoreFieldAccessExpr(TypedDict):
    _kind: Literal["StoreFieldAccessExpr"]
    store_alias: str
    field_name: str
    call_args: list  # list[ExprIR]


class SignalRefIR(TypedDict):
    owner: Optional[str]
    name: str


class BindingIR(TypedDict):
    name: str
    type: Optional[dict]


class EventHandlerIR(TypedDict):
    _kind: Literal["EventHandlerIR"]
    kind: str  # "inline_fn"|"fn_ref"|"store_action"
    body: Optional[dict]
    ref: Optional[str]


class StateIntent(TypedDict):
    _kind: Literal["StateIntent"]
    name: str
    type: TypeIR
    initial_value: dict
    mutability: str
    update_semantics: str
    accessor_style: str
    is_copy: bool


class MutationIntent(TypedDict):
    _kind: Literal["MutationIntent"]
    target: SignalRefIR
    operation: str
    amount: Optional[dict]
    item: Optional[dict]


class ComputedIntent(TypedDict):
    _kind: Literal["ComputedIntent"]
    name: str
    type: TypeIR
    body: dict
    deps: list[str]


class EffectIntent(TypedDict):
    _kind: Literal["EffectIntent"]
    deps: list[str]
    body: dict


class LifecycleIntent(TypedDict):
    _kind: Literal["LifecycleIntent"]
    kind: str
    body: dict


class ConditionalBranch(TypedDict):
    condition: dict
    body: Optional[dict]


class ConditionalIntent(TypedDict):
    _kind: Literal["ConditionalIntent"]
    branches: list  # list[ConditionalBranch]
    fallback: Optional[dict]


class ListRenderingIntent(TypedDict):
    _kind: Literal["ListRenderingIntent"]
    iterable: dict
    item_binding: dict
    body: Optional[dict]
    key_expr: Optional[dict]
    empty_body: Optional[dict]


class BindIntent(TypedDict):
    _kind: Literal["BindIntent"]
    source: SignalRefIR
    element_type: str
    direction: str


class EventIntent(TypedDict):
    _kind: Literal["EventIntent"]
    event_name: str
    handler: EventHandlerIR


class SlotIntent(TypedDict):
    _kind: Literal["SlotIntent"]
    name: Optional[str]
    body: Optional[dict]


class ElementIR(TypedDict):
    _kind: Literal["ElementIR"]
    tag: str
    props: dict[str, dict]
    bindings: dict[str, SignalRefIR]
    events: dict[str, EventHandlerIR]
    style_classes: Optional[dict]
    inline_style: Optional[dict]
    children: list  # list[ViewTree]
    test_id: Optional[str]
    ref_name: Optional[str]
    construction_kind: str


class StyleRuleIR(TypedDict):
    _kind: Literal["StyleRuleIR"]
    class_name: str
    properties: dict[str, dict]
    output_mode: str


class BlockIR(TypedDict):
    _kind: Literal["BlockIR"]
    statements: list  # list[StatementIR]
    execution_mode: str


class LetStmtIR(TypedDict):
    _kind: Literal["LetStmtIR"]
    name: str
    value: Optional[dict]
    type: Optional[TypeIR]


class IfStmtIR(TypedDict):
    _kind: Literal["IfStmtIR"]
    condition: dict
    then_body: dict
    else_body: Optional[dict]


class ReturnStmtIR(TypedDict):
    _kind: Literal["ReturnStmtIR"]
    value: Optional[dict]


class MutationStmt(TypedDict):
    _kind: Literal["MutationStmt"]
    intent: Optional[dict]


class FunctionIR(TypedDict):
    _kind: Literal["FunctionIR"]
    name: str
    params: list[dict]  # list[{name, type?}]
    return_type: Optional[TypeIR]
    body: dict


class ComponentIR(TypedDict, total=False):
    _kind: Literal["ComponentIR"]
    name: str
    generic_params: list[str]
    props: dict[str, TypeIR]
    states: list  # list[StateIntent]
    computeds: list
    effects: list
    lifecycles: list
    fns: list
    store_refs: list[str]
    refs: list
    view: Optional[dict]
    styles: list


class StoreIR(TypedDict):
    _kind: Literal["StoreIR"]
    name: str
    signals: dict[str, TypeIR]
    signal_defaults: dict[str, Any]
    getters: dict[str, dict]
    actions: list


class SourceMapEntry(TypedDict):
    node_id: str
    omni_span: dict  # {line, col, len}
    target_spans: list  # list[{file, line, col, len, kind}]


class IRProgram(TypedDict):
    version: str
    components: dict[str, ComponentIR]
    stores: dict[str, StoreIR]
    source_map: dict[str, SourceMapEntry]


__all__ = [
    "TypeIR", "LiteralExpr", "IdentifierExpr", "FieldAccessExpr",
    "BinaryOpExpr", "UnaryOpExpr", "TernaryIR", "StoreFieldAccessExpr",
    "SignalRefIR", "BindingIR", "EventHandlerIR",
    "StateIntent", "MutationIntent", "ComputedIntent",
    "EffectIntent", "LifecycleIntent",
    "ConditionalBranch", "ConditionalIntent",
    "ListRenderingIntent", "BindIntent", "EventIntent", "SlotIntent",
    "ElementIR", "StyleRuleIR", "BlockIR",
    "LetStmtIR", "IfStmtIR", "ReturnStmtIR", "MutationStmt",
    "FunctionIR", "ComponentIR", "StoreIR",
    "SourceMapEntry", "IRProgram",
]
