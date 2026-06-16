//! Intent IR types and AST -> IR lowering.
//! Mirrors Python omniview/ir_types.py. Serializes to/from JSON via serde.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::parser::ast::Program;
use crate::semantic::SemanticResult;
use crate::sourcemap::SourceMapEntry;

// ═══════════════════════════════════════════════════════
// Expression IR
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "_kind")]
pub enum ExprIR {
    LiteralExpr { value: serde_json::Value },
    IdentifierExpr { name: String },
    FieldAccessExpr { object: Box<ExprIR>, field: String },
    BinaryOpExpr { op: String, left: Box<ExprIR>, right: Box<ExprIR> },
    UnaryOpExpr { op: String, operand: Box<ExprIR> },
    TernaryIR { cond: Box<ExprIR>, then_expr: Box<ExprIR>, else_expr: Box<ExprIR> },
    StoreFieldAccessExpr { store_alias: String, field_name: String, call_args: Vec<ExprIR> },
}

// ═══════════════════════════════════════════════════════
// Type IR
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeIR {
    pub kind: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub type_args: Vec<TypeIR>,
    #[serde(skip_serializing_if = "is_false", default)]
    pub is_nullable: bool,
    #[serde(skip_serializing_if = "is_false", default)]
    pub needs_clone: bool,
    #[serde(skip_serializing_if = "is_false", default)]
    pub needs_partial_eq: bool,
}

fn is_false(b: &bool) -> bool { !*b }

// ═══════════════════════════════════════════════════════
// Signal & Binding References
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalRefIR {
    pub owner: Option<String>,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindingIR {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<TypeIR>,
}

// ═══════════════════════════════════════════════════════
// Event Handler IR
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventHandlerIR {
    pub kind: String,
    pub body: Option<BlockIR>,
    pub r#ref: Option<String>,
}

// ═══════════════════════════════════════════════════════
// Block IR
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockIR {
    pub statements: Vec<StatementIR>,
    pub execution_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "_kind")]
pub enum StatementIR {
    LetStmtIR { name: String, value: Option<ExprIR>, r#type: Option<TypeIR> },
    IfStmtIR { condition: ExprIR, then_body: BlockIR, else_body: Option<BlockIR> },
    ReturnStmtIR { value: Option<ExprIR> },
    MutationStmt { intent: Option<MutationIntent> },
    EventEmitStmtIR { event_name: String, args: Vec<ExprIR> },
    ExprStmtIR { expr: ExprIR },
}

// ═══════════════════════════════════════════════════════
// Core Intent Types
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateIntent {
    pub name: String,
    pub r#type: TypeIR,
    pub initial_value: ExprIR,
    pub mutability: String,
    pub update_semantics: String,
    pub accessor_style: String,
    pub is_copy: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationIntent {
    pub target: SignalRefIR,
    pub operation: String,
    pub amount: Option<ExprIR>,
    pub item: Option<ExprIR>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputedIntent {
    pub name: String,
    pub r#type: TypeIR,
    pub body: ExprIR,
    pub deps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectIntent {
    pub deps: Vec<String>,
    pub body: BlockIR,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleIntent {
    pub kind: String,
    pub body: BlockIR,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionalBranch {
    pub condition: ExprIR,
    pub body: Option<ViewTree>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionalIntent {
    pub branches: Vec<ConditionalBranch>,
    pub fallback: Option<ViewTree>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListRenderingIntent {
    pub iterable: ExprIR,
    pub item_binding: BindingIR,
    pub body: Option<ViewTree>,
    pub key_expr: Option<ExprIR>,
    pub empty_body: Option<ViewTree>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindIntent {
    pub source: SignalRefIR,
    pub element_type: String,
    pub direction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventIntent {
    pub event_name: String,
    pub handler: EventHandlerIR,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotIntent {
    pub name: Option<String>,
    pub body: Option<ViewTree>,
}

// ═══════════════════════════════════════════════════════
// Element IR
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementIR {
    pub tag: String,
    pub props: HashMap<String, ExprIR>,
    pub bindings: HashMap<String, SignalRefIR>,
    pub events: HashMap<String, EventHandlerIR>,
    pub style_classes: Option<serde_json::Value>,
    pub inline_style: Option<serde_json::Value>,
    pub children: Vec<ViewTree>,
    pub test_id: Option<String>,
    pub ref_name: Option<String>,
    pub construction_kind: String,
}

// ═══════════════════════════════════════════════════════
// ViewTree recursive type
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ViewTree {
    Element(ElementIR),
    Conditional(Box<ConditionalIntent>),
    ListRendering(Box<ListRenderingIntent>),
    Slot(Box<SlotIntent>),
    Text(String),
}

// ═══════════════════════════════════════════════════════
// Style
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleRuleIR {
    pub class_name: String,
    pub properties: HashMap<String, ExprIR>,
    pub output_mode: String,
}

// ═══════════════════════════════════════════════════════
// Function IR
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionIR {
    pub name: String,
    pub params: Vec<ParamIR>,
    pub return_type: Option<TypeIR>,
    pub body: BlockIR,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamIR {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<TypeIR>,
}

// ═══════════════════════════════════════════════════════
// Top-Level IR
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentIR {
    pub name: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub generic_params: Vec<String>,
    pub props: HashMap<String, TypeIR>,
    pub states: Vec<StateIntent>,
    pub computeds: Vec<ComputedIntent>,
    pub effects: Vec<EffectIntent>,
    pub lifecycles: Vec<LifecycleIntent>,
    pub fns: Vec<FunctionIR>,
    pub store_refs: Vec<String>,
    pub refs: Vec<serde_json::Value>,
    pub view: Option<ViewTree>,
    pub styles: Vec<StyleRuleIR>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreIR {
    pub name: String,
    pub signals: HashMap<String, TypeIR>,
    pub signal_defaults: HashMap<String, serde_json::Value>,
    pub getters: HashMap<String, ExprIR>,
    pub actions: Vec<serde_json::Value>,
}

// ═══════════════════════════════════════════════════════
// Top-level IR Program
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IRProgram {
    pub version: String,
    pub components: HashMap<String, ComponentIR>,
    pub stores: HashMap<String, StoreIR>,
    pub source_map: HashMap<String, SourceMapEntry>,
}

/// Lower AST -> IR.
pub fn lower(program: &Program, semantics: &SemanticResult) -> IRProgram {
    let _ = (program, semantics);
    IRProgram {
        version: "1.0".to_string(),
        components: HashMap::new(),
        stores: HashMap::new(),
        source_map: HashMap::new(),
    }
}
