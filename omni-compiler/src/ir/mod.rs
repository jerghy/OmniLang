//! Intent IR types and AST -> IR lowering.
//! Mirrors Python omniview/ir_types.py. Serializes to/from JSON via serde.

pub mod logic;
pub mod lower_logic;
pub use logic::*;

use std::cell::Cell;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::parser::ast::*;
use crate::semantic::SemanticResult;
use crate::sourcemap::SourceMapEntry;

// ═══════════════════════════════════════════════════════
// Expression IR
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "_kind")]
pub enum ExprIR {
    LiteralExpr {
        value: serde_json::Value,
    },
    IdentifierExpr {
        name: String,
    },
    FieldAccessExpr {
        object: Box<ExprIR>,
        field: String,
    },
    BinaryOpExpr {
        op: String,
        left: Box<ExprIR>,
        right: Box<ExprIR>,
    },
    UnaryOpExpr {
        op: String,
        operand: Box<ExprIR>,
    },
    TernaryIR {
        cond: Box<ExprIR>,
        then_expr: Box<ExprIR>,
        else_expr: Box<ExprIR>,
    },
    CallExpr {
        name: String,
        args: Vec<ExprIR>,
    },
    ArrayExpr {
        elements: Vec<ExprIR>,
    },
    StructExpr {
        type_name: String,
        fields: Vec<(String, ExprIR)>,
    },
    StoreFieldAccessExpr {
        store_alias: String,
        field_name: String,
        call_args: Vec<ExprIR>,
    },
    MethodCallExpr {
        object: Box<ExprIR>,
        method: String,
        args: Vec<ExprIR>,
    },
    /// Block expression in expression position — preserves all statements,
    /// with the last expression providing the value.
    BlockExpr {
        statements: Vec<StatementIR>,
        tail: Box<ExprIR>,
    },
    /// Array/map index access: arr[0] or map["key"]
    IndexExpr {
        object: Box<ExprIR>,
        index: Box<ExprIR>,
    },
    /// Closure / lambda expression: |params| body
    ClosureExpr {
        params: Vec<String>,
        body: Box<ExprIR>,
    },
    /// await expression: await <expr>
    AwaitExpr {
        operand: Box<ExprIR>,
    },
    /// Tagged union variant construction: Ok { value: 42 }
    VariantLiteral {
        union_type: String,
        variant: String,
        fields: Vec<(String, ExprIR)>,
    },
    /// match expression: match expr { Pattern => body, ... }
    MatchExpr {
        matched: Box<ExprIR>,
        arms: Vec<MatchArmIR>,
    },
}

/// A single arm in a match expression IR.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchArmIR {
    pub pattern: MatchPatternIR,
    pub body: BlockIR,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchPatternIR {
    #[serde(rename = "_kind")]
    pub kind: MatchPatternKindIR,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MatchPatternKindIR {
    #[serde(rename = "variant")]
    Variant {
        name: String,
        bindings: Vec<String>,
    },
    #[serde(rename = "wildcard")]
    Wildcard,
    #[serde(rename = "literal")]
    Literal {
        value: serde_json::Value,
    },
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

fn is_false(b: &bool) -> bool {
    !*b
}

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
    #[serde(default)]
    pub params: Vec<String>,
}

// ═══════════════════════════════════════════════════════
// Block IR & Statements
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockIR {
    pub statements: Vec<StatementIR>,
    pub execution_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "_kind")]
pub enum StatementIR {
    LetStmtIR {
        name: String,
        value: Option<ExprIR>,
        r#type: Option<TypeIR>,
    },
    IfStmtIR {
        condition: ExprIR,
        then_body: BlockIR,
        else_body: Option<BlockIR>,
    },
    ReturnStmtIR {
        value: Option<ExprIR>,
    },
    MutationStmt {
        intent: Option<MutationIntent>,
    },
    EventEmitStmtIR {
        event_name: String,
        args: Vec<ExprIR>,
    },
    ExprStmtIR {
        expr: ExprIR,
    },
    BlockStmtIR {
        body: BlockIR,
    },
    WhileStmtIR {
        condition: ExprIR,
        body: BlockIR,
    },
    ForInStmtIR {
        var_name: String,
        iterable: ExprIR,
        body: BlockIR,
    },
    BreakStmtIR,
    ContinueStmtIR,
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
    /// Source location in .omni file
    #[serde(default)]
    pub source_line: u32,
    #[serde(default)]
    pub source_col: u32,
}

// ═══════════════════════════════════════════════════════
// ViewTree recursive type
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum ViewTree {
    Element(ElementIR),
    Conditional(Box<ConditionalIntent>),
    ListRendering(Box<ListRenderingIntent>),
    /// A template interpolation `{ expr }` — the inner ExprIR is lowered from the AST,
    /// so codegen can emit proper JS for any expression (VarRef, if-else, binary, call, …).
    Interpolation(Box<ExprIR>),
    Text(String),
    /// A `<slot>` placeholder directive. `Some("name")` for named slots,
    /// `None` for the default/unnamed slot.
    Slot(Option<String>),
}

// ═══════════════════════════════════════════════════════
// Style & Function IR
// ═══════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleRuleIR {
    pub class_name: String,
    pub properties: HashMap<String, ExprIR>,
    pub output_mode: String,
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
    /// Default values for props (keyed by prop name).
    /// Props without defaults must be provided by the parent.
    #[serde(default)]
    pub prop_defaults: HashMap<String, ExprIR>,
    pub states: Vec<StateIntent>,
    pub computeds: Vec<ComputedIntent>,
    pub effects: Vec<EffectIntent>,
    pub lifecycles: Vec<LifecycleIntent>,
    pub fns: Vec<FunctionIR>,
    pub store_refs: Vec<String>,
    pub refs: Vec<serde_json::Value>,
    /// Event declarations (e.g., `event onDelete: i32`)
    #[serde(default)]
    pub event_names: Vec<String>,
    pub view: Option<ViewTree>,
    pub styles: Vec<StyleRuleIR>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreIR {
    pub name: String,
    pub signals: HashMap<String, TypeIR>,
    pub signal_defaults: HashMap<String, serde_json::Value>,
    pub getters: HashMap<String, ExprIR>,
    pub actions: Vec<FunctionIR>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IRProgram {
    pub version: String,
    pub modules: HashMap<String, ModuleIR>,
    pub source_map: HashMap<String, SourceMapEntry>,
    /// Stdlib modules imported via `import { ... } from "std/<module>"`
    #[serde(default)]
    pub stdlib_imports: Vec<String>,
}

impl Default for IRProgram {
    fn default() -> Self {
        IRProgram {
            version: "0.1".to_string(),
            modules: HashMap::new(),
            source_map: HashMap::new(),
            stdlib_imports: Vec::new(),
        }
    }
}

impl IRProgram {
    /// Collect all components across all modules.
    pub fn all_components(&self) -> HashMap<&str, &ComponentIR> {
        self.modules
            .values()
            .flat_map(|m| m.components.iter())
            .map(|(k, v)| (k.as_str(), v))
            .collect()
    }

    /// Collect all public functions across all modules.
    pub fn all_functions(&self) -> Vec<&FunctionIR> {
        self.modules
            .values()
            .flat_map(|m| m.functions.values())
            .filter(|f| f.visibility == "public")
            .collect()
    }

    /// Collect all stores across all modules.
    pub fn all_stores(&self) -> HashMap<&str, &StoreIR> {
        self.modules
            .values()
            .flat_map(|m| m.stores.iter())
            .map(|(k, v)| (k.as_str(), v))
            .collect()
    }
}

// ═══════════════════════════════════════════════════════
// Lowering: AST -> IR
// ═══════════════════════════════════════════════════════

/// Lower AST -> IR.
pub fn lower(program: &Program, _semantics: &SemanticResult) -> IRProgram {
    let mut modules = HashMap::new();

    // Build variant → union name lookup for tagged union construction
    let mut variant_map: HashMap<String, String> = HashMap::new();
    for module in program.modules.values() {
        for tu in &module.tagged_unions {
            for variant in &tu.variants {
                variant_map.insert(variant.name.clone(), tu.name.clone());
            }
        }
    }

    for (module_name, module) in &program.modules {
        let mut components = HashMap::new();
        let mut stores = HashMap::new();
        let mut functions = HashMap::new();
        let mut structs = HashMap::new();
        let mut enums = HashMap::new();
        let mut type_aliases = HashMap::new();
        let mut imports: Vec<ImportIR> = Vec::new();
        let mut exports: Vec<ExportIR> = Vec::new();

        for comp in &module.components {
            let ir = lower_component(comp, &variant_map);
            components.insert(comp.name.clone(), ir);
        }

        for store in &module.stores {
            let ir = lower_store(store, &variant_map);
            stores.insert(store.name.clone(), ir);
        }

        for func in &module.functions {
            let ir = lower_logic::lower_function(func, &variant_map);
            functions.insert(func.name.clone(), ir);
        }

        for s in &module.structs {
            let ir = lower_logic::lower_struct(s);
            structs.insert(s.name.clone(), ir);
        }

        for tu in &module.tagged_unions {
            let ir = lower_logic::lower_enum(tu);
            enums.insert(tu.name.clone(), ir);
        }

        for ta in &module.type_aliases {
            let ir = lower_logic::lower_type_alias(ta);
            type_aliases.insert(ta.name.clone(), ir);
        }

        for imp in &module.imports {
            imports.push(lower_logic::lower_import(imp));
        }

        for exp in &module.exports {
            exports.push(lower_logic::lower_export(exp));
        }

        let module_ir = ModuleIR {
            name: module_name.clone(),
            functions,
            structs,
            enums,
            type_aliases,
            components,
            stores,
            themes: Vec::new(),
            imports,
            exports,
        };

        modules.insert(module_name.clone(), module_ir);
    }

    // Collect stdlib imports: any "import { ... } from \"std/<module>\""
    let mut stdlib_imports: Vec<String> = Vec::new();
    for module in program.modules.values() {
        for imp in &module.imports {
            if let ImportStmt::Named(ni) = imp {
                if ni.path.starts_with("std/") {
                    let module_name = ni.path.strip_prefix("std/").unwrap().to_string();
                    if !stdlib_imports.contains(&module_name) {
                        stdlib_imports.push(module_name);
                    }
                }
            }
        }
    }

    IRProgram {
        version: "1.0".to_string(),
        modules,
        source_map: HashMap::new(),
        stdlib_imports,
    }
}

fn lower_component(comp: &ComponentDef, variant_map: &HashMap<String, String>) -> ComponentIR {
    let states: Vec<StateIntent> = comp
        .signals
        .iter()
        .map(|s| StateIntent {
            name: s.name.clone(),
            r#type: lower_type(&s.type_ann),
            initial_value: lower_expr(&s.init, variant_map),
            mutability: "mutable".into(),
            update_semantics: "setter-call".into(),
            accessor_style: "direct".into(),
            is_copy: false,
        })
        .collect();

    let computeds: Vec<ComputedIntent> = comp
        .computeds
        .iter()
        .map(|c| ComputedIntent {
            name: c.name.clone(),
            r#type: lower_type(&c.type_ann),
            body: lower_expr(&c.body, variant_map),
            deps: vec![],
        })
        .collect();

    let effects: Vec<EffectIntent> = comp
        .lifecycles
        .iter()
        .filter(|lc| matches!(lc.kind, LifecycleKind::Effect))
        .map(|lc| EffectIntent {
            deps: lc.deps.clone(),
            body: lower_block(&lc.body, variant_map),
        })
        .collect();

    let lifecycles: Vec<LifecycleIntent> = comp
        .lifecycles
        .iter()
        .filter(|lc| !matches!(lc.kind, LifecycleKind::Effect))
        .map(|lc| LifecycleIntent {
            kind: match lc.kind {
                LifecycleKind::Mount => "mount".into(),
                LifecycleKind::Unmount => "unmount".into(),
                LifecycleKind::Effect => "effect".into(),
            },
            body: lower_block(&lc.body, variant_map),
        })
        .collect();

    let mut fns: Vec<FunctionIR> = comp
        .fns
        .iter()
        .map(|f| FunctionIR {
            name: f.name.clone(),
            visibility: "private".to_string(),
            params: f
                .params
                .iter()
                .map(|p| ParamIR {
                    name: p.name.clone(),
                    r#type: p.type_ann.as_ref().map(lower_type),
                })
                .collect(),
            return_type: f.return_type.as_ref().map(lower_type),
            body: lower_block(&f.body, variant_map),
            is_async: f.is_async,
        })
        .collect();
    // Also include component actions as regular functions
    for a in &comp.actions {
        fns.push(FunctionIR {
            name: a.name.clone(),
            visibility: "private".to_string(),
            params: a
                .params
                .iter()
                .map(|p| ParamIR {
                    name: p.name.clone(),
                    r#type: p.type_ann.as_ref().map(lower_type),
                })
                .collect(),
            return_type: a.return_type.as_ref().map(lower_type),
            body: lower_block(&a.body, variant_map),
            is_async: a.is_async,
        });
    }

    let props: HashMap<String, TypeIR> = comp
        .props
        .iter()
        .map(|p| (p.name.clone(), lower_type(&p.type_ann)))
        .collect();

    let prop_defaults: HashMap<String, ExprIR> = comp
        .props
        .iter()
        .filter_map(|p| p.default.as_ref().map(|d| (p.name.clone(), lower_expr(d, variant_map))))
        .collect();

    // Collect input-binding signal names for <input> element auto-binding.
    // Priority: String signals with "input" in name, then any String signal.
    let input_signals: Vec<String> = {
        let string_signals: Vec<_> = comp.signals.iter()
            .filter(|s| matches!(s.type_ann, TypeAnnotation::Named { ref name, .. } if name == "String"))
            .map(|s| s.name.clone())
            .collect();
        // Prefer signals whose name suggests they're for input binding
        let mut candidates: Vec<_> = string_signals
            .iter()
            .filter(|n| n.contains("input") || n.contains("Input"))
            .cloned()
            .collect();
        // Fall back to any String signal
        if candidates.is_empty() {
            candidates = string_signals;
        }
        candidates
    };

    let view = comp
        .view
        .as_ref()
        .map(|v| lower_view_block(v, &input_signals, variant_map));

    let styles: Vec<StyleRuleIR> = comp
        .styles
        .iter()
        .map(|s| {
            let properties: HashMap<String, ExprIR> = s
                .properties
                .iter()
                .map(|(k, v)| {
                    let ir_val = match v {
                        StyleValue::Number(n) => {
                            // JSON does not support NaN or Infinity; fall back to 0
                            // for these edge cases (should not occur in well-formed source).
                            let num = if n.is_finite() {
                                serde_json::Number::from_f64(*n)
                                    .unwrap_or_else(|| serde_json::Number::from(0))
                            } else {
                                serde_json::Number::from(0)
                            };
                            ExprIR::LiteralExpr {
                                value: serde_json::Value::Number(num),
                            }
                        }
                        StyleValue::String(str_val) => ExprIR::LiteralExpr {
                            value: serde_json::Value::String(str_val.clone()),
                        },
                        StyleValue::Ident(id) => ExprIR::LiteralExpr {
                            value: serde_json::Value::String(id.clone()),
                        },
                        StyleValue::HexColor(h) => ExprIR::LiteralExpr {
                            value: serde_json::Value::String(h.clone()),
                        },
                        StyleValue::Dimension(val, unit) => ExprIR::LiteralExpr {
                            value: serde_json::Value::String(format!("{}{}", val, unit)),
                        },
                        StyleValue::List(parts) => {
                            let s: String = parts
                                .iter()
                                .map(|p| match p {
                                    StyleValuePart::Ident(id) => id.clone(),
                                    StyleValuePart::Number(n) => n.to_string(),
                                    StyleValuePart::Dimension(val, unit) => {
                                        format!("{}{}", val, unit)
                                    }
                                    StyleValuePart::HexColor(h) => h.clone(),
                                })
                                .collect::<Vec<_>>()
                                .join(" ");
                            ExprIR::LiteralExpr {
                                value: serde_json::Value::String(s),
                            }
                        }
                    };
                    (k.clone(), ir_val)
                })
                .collect();
            StyleRuleIR {
                class_name: s.class_name.clone(),
                properties,
                output_mode: "css".into(),
            }
        })
        .collect();

    ComponentIR {
        name: comp.name.clone(),
        generic_params: vec![],
        props,
        prop_defaults,
        states,
        computeds,
        effects,
        lifecycles,
        fns,
        store_refs: comp.store_refs.clone(),
        refs: vec![],
        event_names: comp.events.iter().map(|e| e.name.clone()).collect(),
        view,
        styles,
    }
}

fn lower_store(store: &StoreDef, variant_map: &HashMap<String, String>) -> StoreIR {
    let signals: HashMap<String, TypeIR> = store
        .signals
        .iter()
        .map(|s| (s.name.clone(), lower_type(&s.type_ann)))
        .collect();

    let signal_defaults: HashMap<String, serde_json::Value> = store
        .signals
        .iter()
        .map(|s| {
            let val = match &s.init {
                Expr::Literal(Literal::Int(n), _) => serde_json::Value::Number((*n).into()),
                Expr::Literal(Literal::Float(f), _) => {
                    serde_json::json!(f)
                }
                Expr::Literal(Literal::String(s), _) => serde_json::Value::String(s.clone()),
                Expr::Literal(Literal::Bool(b), _) => serde_json::Value::Bool(*b),
                Expr::Literal(Literal::Null, _) => serde_json::Value::Null,
                Expr::ArrayLiteral { elements, .. } => {
                    serde_json::Value::Array(elements.iter().map(expr_to_json_value).collect())
                }
                _ => {
                    // Non-literal initializer: serialize as a placeholder
                    // indicating the expression cannot be evaluated at compile time.
                    serde_json::Value::String("<runtime expr>".into())
                }
            };
            (s.name.clone(), val)
        })
        .collect();

    let getters: HashMap<String, ExprIR> = store
        .getters
        .iter()
        .map(|g| (g.name.clone(), lower_expr(&g.body, variant_map)))
        .collect();

    let actions: Vec<FunctionIR> = store
        .actions
        .iter()
        .map(|action| FunctionIR {
            name: action.name.clone(),
            visibility: "public".into(),
            params: action
                .params
                .iter()
                .map(|p| ParamIR {
                    name: p.name.clone(),
                    r#type: p.type_ann.as_ref().map(lower_type),
                })
                .collect(),
            return_type: action.return_type.as_ref().map(lower_type),
            body: lower_block(&action.body, variant_map),
            is_async: action.is_async,
        })
        .collect();

    StoreIR {
        name: store.name.clone(),
        signals,
        signal_defaults,
        getters,
        actions,
    }
}

/// Convert a simple AST expression to a JSON value (for store signal defaults).
fn expr_to_json_value(expr: &Expr) -> serde_json::Value {
    match expr {
        Expr::Literal(Literal::Int(n), _) => serde_json::Value::Number((*n).into()),
        Expr::Literal(Literal::Float(f), _) => serde_json::json!(f),
        Expr::Literal(Literal::String(s), _) => serde_json::Value::String(s.clone()),
        Expr::Literal(Literal::Bool(b), _) => serde_json::Value::Bool(*b),
        Expr::Literal(Literal::Null, _) => serde_json::Value::Null,
        Expr::ArrayLiteral { elements, .. } => {
            serde_json::Value::Array(elements.iter().map(expr_to_json_value).collect())
        }
        _ => serde_json::Value::String(format!("{:?}", expr)),
    }
}

fn lower_view_block(view: &ViewBlock, string_signals: &[String], variant_map: &HashMap<String, String>) -> ViewTree {
    if let Some(first) = view.children.first() {
        let sig_idx = Cell::new(0usize);
        lower_view_node(first, string_signals, Some(&sig_idx), variant_map)
    } else {
        ViewTree::Text("".into())
    }
}

fn lower_view_node(
    node: &ViewNode,
    string_signals: &[String],
    sig_idx: Option<&Cell<usize>>,
    variant_map: &HashMap<String, String>,
) -> ViewTree {
    match node {
        ViewNode::Element(el) => {
            let mut props = HashMap::new();
            let mut bindings = HashMap::new();
            let mut events = HashMap::new();
            let mut inline_style: Option<serde_json::Value> = None;
            let mut has_value_binding = false;

            for prop in &el.props {
                match prop {
                    ElementProp::Static { name, value } => {
                        props.insert(name.clone(), lower_expr(value, variant_map));
                    }
                    ElementProp::Bind { name, signal_path } => {
                        if name == "value" {
                            has_value_binding = true;
                        }
                        bindings.insert(
                            name.clone(),
                            SignalRefIR {
                                owner: None,
                                name: signal_path.clone(),
                            },
                        );
                    }
                    ElementProp::Event { name, handler } => {
                        let ir_handler = match handler {
                            EventHandler::InlineFn { params, body, .. } => EventHandlerIR {
                                kind: "inline_fn".into(),
                                body: Some(lower_block(body, variant_map)),
                                r#ref: None,
                                params: params.iter().map(|p| p.name.clone()).collect(),
                            },
                            EventHandler::FnRef(fn_name) => EventHandlerIR {
                                kind: "fn_ref".into(),
                                body: None,
                                r#ref: Some(fn_name.clone()),
                                params: vec![],
                            },
                            EventHandler::StoreAction { store, action } => EventHandlerIR {
                                kind: "store_action".into(),
                                body: None,
                                r#ref: Some(format!("{}.{}", store, action)),
                                params: vec![],
                            },
                        };
                        events.insert(name.clone(), ir_handler);
                    }
                    ElementProp::Class { value } => {
                        // class: styleRef or class: "literal-class"
                        let class_str = match value {
                            ClassValue::StyleRef(s) => s.clone(),
                            ClassValue::StringLiteral(s) => s.clone(),
                            ClassValue::ClassMap(entries) => {
                                // Only include classes whose value is a literal true (static classes).
                                // Dynamic class toggling (via signals/function calls) needs JS effects.
                                let mut static_classes: Vec<String> = Vec::new();
                                for (key, val_expr) in entries {
                                    let is_static_true = match val_expr {
                                        Expr::Literal(Literal::Bool(true), _) => true,
                                        _ => false,
                                    };
                                    if is_static_true {
                                        static_classes.push(key.clone());
                                    } else {
                                        // Dynamic class value: not yet supported as JS toggle.
                                        // For now, include the class name statically — the
                                        // inline style mechanism handles the dynamic coloring.
                                        static_classes.push(key.clone());
                                    }
                                }
                                static_classes.join(" ")
                            }
                        };
                        props.insert(
                            "class".into(),
                            ExprIR::LiteralExpr {
                                value: serde_json::Value::String(class_str),
                            },
                        );
                    }
                    ElementProp::Style { entries } => {
                        // style: { key: value, ... } — passed as inline_style
                        // Lower each entry to a JSON representation for codegen
                        let style_map: serde_json::Map<String, serde_json::Value> = entries
                            .iter()
                            .map(|(key, expr)| {
                                let ir_expr = lower_expr(expr, variant_map);
                                let json_val = lower_expr_to_json_from_ir(&ir_expr);
                                (key.clone(), json_val)
                            })
                            .collect();
                        inline_style = Some(serde_json::Value::Object(style_map));
                    }
                    ElementProp::TestId(id) => {
                        // test-id="..." attribute
                        props.insert(
                            "data-testid".into(),
                            ExprIR::LiteralExpr {
                                value: serde_json::Value::String(id.clone()),
                            },
                        );
                    }
                    ElementProp::Ref(r) => {
                        // ref="..." attribute
                        props.insert(
                            "data-ref".into(),
                            ExprIR::LiteralExpr {
                                value: serde_json::Value::String(r.clone()),
                            },
                        );
                    }
                }
            }

            // Auto-bind <input> elements to String signals if no explicit bind:value
            if let Some(sig_counter) = sig_idx {
                let idx = sig_counter.get();
                if el.tag == "input" && !has_value_binding && idx < string_signals.len() {
                    let sig_name = string_signals[idx].clone();
                    sig_counter.set(idx + 1);
                    bindings.insert(
                        "value".into(),
                        SignalRefIR {
                            owner: None,
                            name: sig_name,
                        },
                    );
                }
            }

            let children: Vec<ViewTree> = el
                .children
                .iter()
                .map(|c| lower_view_node(c, string_signals, sig_idx, variant_map))
                .collect();

            ViewTree::Element(ElementIR {
                tag: el.tag.clone(),
                props,
                bindings,
                events,
                style_classes: None,
                inline_style,
                children,
                test_id: None,
                ref_name: None,
                construction_kind: "tag-mapping".into(),
                source_line: el.span.line,
                source_col: el.span.col,
            })
        }
        ViewNode::Text(s) => ViewTree::Text(s.clone()),
        ViewNode::Interpolation(expr) => {
            // Lower the inner expression to ExprIR so codegen can emit proper JS
            ViewTree::Interpolation(Box::new(lower_expr(expr, variant_map)))
        }
        ViewNode::IfDirective(if_dir) => {
            let branches: Vec<ConditionalBranch> = std::iter::once(ConditionalBranch {
                condition: lower_expr(&if_dir.condition, variant_map),
                body: Some(ViewTree::Element(ElementIR {
                    tag: "fragment".into(),
                    props: HashMap::new(),
                    bindings: HashMap::new(),
                    events: HashMap::new(),
                    style_classes: None,
                    inline_style: None,
                    children: if_dir
                        .body
                        .iter()
                        .map(|n| lower_view_node(n, string_signals, None, variant_map))
                        .collect(),
                    test_id: None,
                    ref_name: None,
                    construction_kind: "conditional".into(),
                    source_line: 0,
                    source_col: 0,
                })),
            })
            .chain(if_dir.else_ifs.iter().map(|(cond, body)| {
                ConditionalBranch {
                    condition: lower_expr(cond, variant_map),
                    body: Some(ViewTree::Element(ElementIR {
                        tag: "fragment".into(),
                        props: HashMap::new(),
                        bindings: HashMap::new(),
                        events: HashMap::new(),
                        style_classes: None,
                        inline_style: None,
                        children: body
                            .iter()
                            .map(|n| lower_view_node(n, string_signals, None, variant_map))
                            .collect(),
                        test_id: None,
                        ref_name: None,
                        construction_kind: "conditional".into(),
                        source_line: 0,
                        source_col: 0,
                    })),
                }
            }))
            .collect();

            let fallback = if_dir.else_body.as_ref().map(|body| {
                ViewTree::Element(ElementIR {
                    tag: "fragment".into(),
                    props: HashMap::new(),
                    bindings: HashMap::new(),
                    events: HashMap::new(),
                    style_classes: None,
                    inline_style: None,
                    children: body
                        .iter()
                        .map(|n| lower_view_node(n, string_signals, None, variant_map))
                        .collect(),
                    test_id: None,
                    ref_name: None,
                    construction_kind: "conditional".into(),
                    source_line: 0,
                    source_col: 0,
                })
            });

            ViewTree::Conditional(Box::new(ConditionalIntent { branches, fallback }))
        }
        ViewNode::ForDirective(for_dir) => {
            let lower_body = |nodes: &[ViewNode]| -> Option<ViewTree> {
                if nodes.is_empty() {
                    None
                } else {
                    Some(ViewTree::Element(ElementIR {
                        tag: "fragment".into(),
                        props: HashMap::new(),
                        bindings: HashMap::new(),
                        events: HashMap::new(),
                        style_classes: None,
                        inline_style: None,
                        children: nodes
                            .iter()
                            .map(|n| lower_view_node(n, string_signals, None, variant_map))
                            .collect(),
                        test_id: None,
                        ref_name: None,
                        construction_kind: "list".into(),
                        source_line: 0,
                        source_col: 0,
                    }))
                }
            };

            ViewTree::ListRendering(Box::new(ListRenderingIntent {
                iterable: lower_expr(&for_dir.iterable, variant_map),
                item_binding: BindingIR {
                    name: for_dir.item_name.clone(),
                    r#type: None,
                },
                body: lower_body(&for_dir.body),
                key_expr: None,
                empty_body: for_dir
                    .empty_body
                    .as_ref()
                    .and_then(|nodes| lower_body(nodes.as_slice())),
            }))
        }
        ViewNode::SlotDirective(name) => ViewTree::Slot(name.clone()),
    }
}

fn lower_stmt(stmt: &Stmt, variant_map: &HashMap<String, String>) -> StatementIR {
    match stmt {
        Stmt::VarDecl { name, init, .. } => StatementIR::LetStmtIR {
            name: name.clone(),
            value: init.as_ref().map(|e| lower_expr(e, variant_map)),
            r#type: None,
        },
        Stmt::Return { value, .. } => StatementIR::ReturnStmtIR {
            value: value.as_ref().map(|e| lower_expr(e, variant_map)),
        },
        Stmt::ExprStmt { expr, .. } => {
            // Bare block in statement position: emit a scoped block
            if let Expr::Block { body, .. } = expr {
                return StatementIR::BlockStmtIR {
                    body: lower_block(body, variant_map),
                };
            }
            if let Some(mutation) = try_lower_mutation(expr, variant_map) {
                StatementIR::MutationStmt {
                    intent: Some(mutation),
                }
            } else {
                StatementIR::ExprStmtIR {
                    expr: lower_expr(expr, variant_map),
                }
            }
        }
        Stmt::If {
            condition,
            then_body,
            else_body,
            ..
        } => StatementIR::IfStmtIR {
            condition: lower_expr(condition, variant_map),
            then_body: lower_block(then_body, variant_map),
            else_body: else_body.as_ref().map(|b| lower_block(b, variant_map)),
        },
        Stmt::Print { newline, arg, .. } => StatementIR::ExprStmtIR {
            expr: ExprIR::CallExpr {
                name: if *newline { "println" } else { "print" }.into(),
                args: vec![lower_expr(arg, variant_map)],
            },
        },
        Stmt::While {
            condition, body, ..
        } => StatementIR::WhileStmtIR {
            condition: lower_expr(condition, variant_map),
            body: lower_block(body, variant_map),
        },
        Stmt::ForIn {
            var_name,
            iterable,
            body,
            ..
        } => StatementIR::ForInStmtIR {
            var_name: var_name.clone(),
            iterable: lower_expr(iterable, variant_map),
            body: lower_block(body, variant_map),
        },
        Stmt::Assign { target, value, .. } => {
            let (name, owner) = match target {
                AssignTarget::Name(n) => (n.clone(), None),
                AssignTarget::Field { object, field } => (field.clone(), Some(object.clone())),
                AssignTarget::Index { object, .. } => (object.clone(), None),
            };
            StatementIR::MutationStmt {
                intent: Some(MutationIntent {
                    target: SignalRefIR { owner, name },
                    operation: "direct_assign".into(),
                    amount: Some(lower_expr(value, variant_map)),
                    item: None,
                }),
            }
        }
        Stmt::Break(_) => StatementIR::BreakStmtIR,
        Stmt::Continue(_) => StatementIR::ContinueStmtIR,
        _ => StatementIR::ExprStmtIR {
            expr: ExprIR::LiteralExpr {
                value: serde_json::Value::Null,
            },
        },
    }
}

/// Try to lower an expression statement as a mutation intent.
/// Handles: MethodCall, FuncCall(FieldAccess), and Assignment to signals.
fn try_lower_mutation(expr: &Expr, variant_map: &HashMap<String, String>) -> Option<MutationIntent> {
    // Pattern 1: Expr::MethodCall { object: VarRef("count"), method: "set", args: [value] }
    if let Expr::MethodCall {
        object,
        method,
        args,
        ..
    } = expr
    {
        if let Expr::VarRef(target_name, _) = object.as_ref() {
            return mutation_for_method(target_name, method, args.first(), variant_map);
        }
    }
    // Pattern 2: Expr::FuncCall { func: FieldAccess { VarRef("count"), "set" }, args: [value] }
    if let Expr::FuncCall { func, args, .. } = expr {
        if let Expr::FieldAccess { object, field, .. } = func.as_ref() {
            if let Expr::VarRef(target_name, _) = object.as_ref() {
                return mutation_for_method(target_name, field, args.first(), variant_map);
            }
        }
    }
    // Pattern 3: Assignment to signal: count = value or count += 1
    if let Expr::BinaryOp {
        left, op, right, ..
    } = expr
    {
        if let Expr::VarRef(target_name, _) = left.as_ref() {
            match op {
                BinaryOpKind::Assign => {
                    return Some(MutationIntent {
                        target: SignalRefIR {
                            owner: None,
                            name: target_name.clone(),
                        },
                        operation: "direct_assign".into(),
                        amount: Some(lower_expr(right, variant_map)),
                        item: None,
                    });
                }
                BinaryOpKind::AddAssign => {
                    return Some(MutationIntent {
                        target: SignalRefIR {
                            owner: None,
                            name: target_name.clone(),
                        },
                        operation: "increment".into(),
                        amount: Some(lower_expr(right, variant_map)),
                        item: None,
                    });
                }
                BinaryOpKind::SubAssign => {
                    return Some(MutationIntent {
                        target: SignalRefIR {
                            owner: None,
                            name: target_name.clone(),
                        },
                        operation: "decrement".into(),
                        amount: Some(lower_expr(right, variant_map)),
                        item: None,
                    });
                }
                BinaryOpKind::MulAssign => {
                    return Some(MutationIntent {
                        target: SignalRefIR {
                            owner: None,
                            name: target_name.clone(),
                        },
                        operation: "multiply".into(),
                        amount: Some(lower_expr(right, variant_map)),
                        item: None,
                    });
                }
                BinaryOpKind::DivAssign => {
                    return Some(MutationIntent {
                        target: SignalRefIR {
                            owner: None,
                            name: target_name.clone(),
                        },
                        operation: "divide".into(),
                        amount: Some(lower_expr(right, variant_map)),
                        item: None,
                    });
                }
                BinaryOpKind::ModAssign => {
                    return Some(MutationIntent {
                        target: SignalRefIR {
                            owner: None,
                            name: target_name.clone(),
                        },
                        operation: "modulo".into(),
                        amount: Some(lower_expr(right, variant_map)),
                        item: None,
                    });
                }
                _ => {}
            }
        }
    }
    None
}

fn mutation_for_method(
    target_name: &str,
    method: &str,
    amount: Option<&Expr>,
    variant_map: &HashMap<String, String>,
) -> Option<MutationIntent> {
    let target = SignalRefIR {
        owner: None,
        name: target_name.into(),
    };
    let amount = amount.map(|e| lower_expr(e, variant_map));
    match method {
        "set" | "setValue" => Some(MutationIntent {
            target,
            operation: "direct_assign".into(),
            amount,
            item: None,
        }),
        "inc" | "increment" => Some(MutationIntent {
            target,
            operation: "increment".into(),
            amount,
            item: None,
        }),
        "dec" | "decrement" => Some(MutationIntent {
            target,
            operation: "decrement".into(),
            amount,
            item: None,
        }),
        "push" => Some(MutationIntent {
            target,
            operation: "push".into(),
            amount,
            item: None,
        }),
        "pop" => Some(MutationIntent {
            target,
            operation: "pop".into(),
            amount,
            item: None,
        }),
        _ => None,
    }
}

pub(crate) fn lower_block(block: &Block, variant_map: &HashMap<String, String>) -> BlockIR {
    let mut statements: Vec<StatementIR> = block.stmts.iter().map(|s| lower_stmt(s, variant_map)).collect();

    // Include trailing expression as an expression statement
    if let Some(ref expr) = block.trailing_expr {
        if let Some(mutation) = try_lower_mutation(expr, variant_map) {
            statements.push(StatementIR::MutationStmt {
                intent: Some(mutation),
            });
        } else {
            statements.push(StatementIR::ExprStmtIR {
                expr: lower_expr(expr, variant_map),
            });
        }
    }

    BlockIR {
        statements,
        execution_mode: "client-only".into(),
    }
}

pub(crate) fn lower_expr(expr: &Expr, variant_map: &HashMap<String, String>) -> ExprIR {
    match expr {
        Expr::Literal(lit, _) => {
            let value = match lit {
                Literal::Int(n) => serde_json::Value::Number((*n).into()),
                Literal::Float(f) => serde_json::json!(f),
                Literal::String(s) => serde_json::Value::String(s.clone()),
                Literal::Bool(b) => serde_json::Value::Bool(*b),
                Literal::Null => serde_json::Value::Null,
            };
            ExprIR::LiteralExpr { value }
        }
        Expr::VarRef(name, _) => {
            // Check if this bare identifier is a tagged union variant (no fields)
            if let Some(union_name) = variant_map.get(name) {
                ExprIR::VariantLiteral {
                    union_type: union_name.clone(),
                    variant: name.clone(),
                    fields: vec![],
                }
            } else {
                ExprIR::IdentifierExpr { name: name.clone() }
            }
        }
        Expr::BinaryOp {
            left, op, right, ..
        } => ExprIR::BinaryOpExpr {
            op: format!("{:?}", op).to_lowercase(),
            left: Box::new(lower_expr(left, variant_map)),
            right: Box::new(lower_expr(right, variant_map)),
        },
        Expr::UnaryOp { op, operand, .. } => ExprIR::UnaryOpExpr {
            op: match op {
                UnaryOpKind::Not => "!".into(),
                UnaryOpKind::Neg => "-".into(),
                UnaryOpKind::Await => "await ".into(),
            },
            operand: Box::new(lower_expr(operand, variant_map)),
        },
        Expr::FuncCall { func, args, .. } => {
            // Pattern: signal.get() → signal.value
            if let Expr::FieldAccess { object, field, .. } = func.as_ref() {
                if let Expr::VarRef(obj_name, _) = object.as_ref() {
                    // Signal method calls
                    if field == "get" {
                        return ExprIR::FieldAccessExpr {
                            object: Box::new(ExprIR::IdentifierExpr {
                                name: obj_name.clone(),
                            }),
                            field: "value".into(),
                        };
                    }
                    if field == "set" || field == "inc" || field == "dec" {
                        // Mutation — handled at statement level by try_lower_mutation
                        return ExprIR::IdentifierExpr {
                            name: obj_name.clone(),
                        };
                    }
                    // Store field access
                    return ExprIR::StoreFieldAccessExpr {
                        store_alias: obj_name.clone(),
                        field_name: field.clone(),
                        call_args: args.iter().map(|e| lower_expr(e, variant_map)).collect(),
                    };
                }
            }
            // Regular function call
            if let Expr::VarRef(name, _) = func.as_ref() {
                ExprIR::CallExpr {
                    name: name.clone(),
                    args: args.iter().map(|e| lower_expr(e, variant_map)).collect(),
                }
            } else {
                ExprIR::LiteralExpr {
                    value: serde_json::Value::Null,
                }
            }
        }
        Expr::FieldAccess { object, field, .. } => ExprIR::FieldAccessExpr {
            object: Box::new(lower_expr(object, variant_map)),
            field: field.clone(),
        },
        Expr::MethodCall {
            object,
            method,
            args,
            ..
        } => {
            // get() with no args → signal access (.value)
            // get(i) with args → array/map index access
            if method == "get" && args.is_empty() {
                ExprIR::FieldAccessExpr {
                    object: Box::new(lower_expr(object, variant_map)),
                    field: "value".into(),
                }
            } else if method == "set" {
                // signal.set(value) → handled at statement level by try_lower_mutation
                ExprIR::IdentifierExpr {
                    name: method.clone(),
                }
            } else {
                // General method call: to_string, len, get(i), push, pop, contains, insert, etc.
                ExprIR::MethodCallExpr {
                    object: Box::new(lower_expr(object, variant_map)),
                    method: method.clone(),
                    args: args.iter().map(|e| lower_expr(e, variant_map)).collect(),
                }
            }
        }
        Expr::StoreFieldAccess {
            store_alias,
            field_name,
            ..
        } => ExprIR::StoreFieldAccessExpr {
            store_alias: store_alias.clone(),
            field_name: field_name.clone(),
            call_args: vec![],
        },
        Expr::ArrayLiteral { elements, .. } => ExprIR::ArrayExpr {
            elements: elements.iter().map(|e| lower_expr(e, variant_map)).collect(),
        },
        Expr::MapLiteral { entries, .. } => {
            let map: serde_json::Map<String, serde_json::Value> = entries
                .iter()
                .map(|(k, v)| {
                    let key = match lower_expr(k, variant_map) {
                        ExprIR::LiteralExpr { value } => match value {
                            serde_json::Value::String(s) => s,
                            other => format!("{}", other),
                        },
                        other => {
                            // For non-string literal keys, use the identifiers' name or Debug as fallback
                            match other {
                                ExprIR::IdentifierExpr { name } => name,
                                _ => format!("{:?}", other),
                            }
                        }
                    };
                    (key, lower_expr_to_json(v))
                })
                .collect();
            ExprIR::LiteralExpr {
                value: serde_json::Value::Object(map),
            }
        }
        Expr::StructLiteral {
            type_name, fields, ..
        } => {
            let ir_fields: Vec<(String, ExprIR)> = fields
                .iter()
                .map(|(name, val)| (name.clone(), lower_expr(val, variant_map)))
                .collect();
            // Check if this is a tagged union variant construction
            if let Some(union_name) = variant_map.get(type_name) {
                ExprIR::VariantLiteral {
                    union_type: union_name.clone(),
                    variant: type_name.clone(),
                    fields: ir_fields,
                }
            } else {
                ExprIR::StructExpr {
                    type_name: type_name.clone(),
                    fields: ir_fields,
                }
            }
        }
        Expr::IfExpr {
            condition,
            then_expr,
            else_expr,
            ..
        } => ExprIR::TernaryIR {
            cond: Box::new(lower_expr(condition, variant_map)),
            then_expr: Box::new(lower_expr(then_expr, variant_map)),
            else_expr: Box::new(lower_expr(else_expr, variant_map)),
        },
        Expr::TernaryExpr {
            condition,
            then_expr,
            else_expr,
            ..
        } => ExprIR::TernaryIR {
            cond: Box::new(lower_expr(condition, variant_map)),
            then_expr: Box::new(lower_expr(then_expr, variant_map)),
            else_expr: Box::new(lower_expr(else_expr, variant_map)),
        },
        Expr::Block { body, .. } => {
            // Block in expression position: preserve ALL statements.
            // The last expression statement provides the value; preceding
            // statements (e.g. print, let, mutations) are kept as-is.
            let block_ir = lower_block(body, variant_map);
            if let Some(last) = block_ir.statements.last() {
                match last {
                    StatementIR::ExprStmtIR { expr } => {
                        if block_ir.statements.len() == 1 {
                            // Simple block { expr } — just the expression
                            expr.clone()
                        } else {
                            // Multi-statement block: wrap in BlockExpr
                            ExprIR::BlockExpr {
                                statements: block_ir.statements[..block_ir.statements.len() - 1]
                                    .to_vec(),
                                tail: Box::new(expr.clone()),
                            }
                        }
                    }
                    _ => {
                        // Last statement is not an expression (e.g. return, if)
                        // Preserve all statements; value is null
                        ExprIR::BlockExpr {
                            statements: block_ir.statements,
                            tail: Box::new(ExprIR::LiteralExpr {
                                value: serde_json::Value::Null,
                            }),
                        }
                    }
                }
            } else {
                ExprIR::LiteralExpr {
                    value: serde_json::Value::Null,
                }
            }
        }
        Expr::IndexGet { object, index, .. } => ExprIR::IndexExpr {
            object: Box::new(lower_expr(object, variant_map)),
            index: Box::new(lower_expr(index, variant_map)),
        },
        Expr::ClosureExpr { params, body, .. } => ExprIR::ClosureExpr {
            params: params.clone(),
            body: Box::new(lower_expr(body, variant_map)),
        },
        Expr::AwaitExpr { operand, .. } => ExprIR::AwaitExpr {
            operand: Box::new(lower_expr(operand, variant_map)),
        },
        Expr::MatchExpr { matched, arms, .. } => {
            let ir_arms: Vec<MatchArmIR> = arms
                .iter()
                .map(|arm| {
                    let pattern = match &arm.pattern.kind {
                        MatchPatternKind::Variant { name, bindings } => {
                            MatchPatternIR {
                                kind: MatchPatternKindIR::Variant {
                                    name: name.clone(),
                                    bindings: bindings.clone(),
                                },
                            }
                        }
                        MatchPatternKind::Wildcard => MatchPatternIR {
                            kind: MatchPatternKindIR::Wildcard,
                        },
                        MatchPatternKind::Literal(lit) => {
                            let value = match lit {
                                Literal::Int(n) => serde_json::Value::Number((*n).into()),
                                Literal::String(s) => serde_json::Value::String(s.clone()),
                                Literal::Bool(b) => serde_json::Value::Bool(*b),
                                _ => serde_json::Value::Null,
                            };
                            MatchPatternIR {
                                kind: MatchPatternKindIR::Literal { value },
                            }
                        }
                    };
                    MatchArmIR {
                        pattern,
                        body: lower_block(&arm.body, variant_map),
                    }
                })
                .collect();
            ExprIR::MatchExpr {
                matched: Box::new(lower_expr(matched, variant_map)),
                arms: ir_arms,
            }
        }
        _ => ExprIR::LiteralExpr {
            value: serde_json::Value::Null,
        },
    }
}

/// Lower an expression to a serde_json::Value (for literal extraction).
fn lower_expr_to_json(expr: &Expr) -> serde_json::Value {
    match lower_expr(expr, &HashMap::new()) {
        ExprIR::LiteralExpr { value } => value,
        ExprIR::IdentifierExpr { name } => serde_json::Value::String(name),
        other => serde_json::Value::String(format!("{:?}", other)),
    }
}

/// Convert an ExprIR back to a JSON value for inline_style serialization.
fn lower_expr_to_json_from_ir(expr: &ExprIR) -> serde_json::Value {
    match expr {
        ExprIR::LiteralExpr { value } => value.clone(),
        ExprIR::IdentifierExpr { name } => serde_json::json!({"$signal": name}),
        ExprIR::FieldAccessExpr { object, field } => {
            // Resolve ThemeStore.currentBg → flat JS name ThemeStore_currentBg
            if let ExprIR::IdentifierExpr { name: obj_name } = object.as_ref() {
                serde_json::json!({"$signal": format!("{}_{}", obj_name, field)})
            } else {
                let obj = lower_expr_to_json_from_ir(object);
                serde_json::json!({"$signal_concat": [obj, field.as_str()]})
            }
        }
        ExprIR::BinaryOpExpr { op: _, left, right } => {
            serde_json::json!({
                "$concat": [
                    lower_expr_to_json_from_ir(left),
                    lower_expr_to_json_from_ir(right)
                ]
            })
        }
        ExprIR::CallExpr { name, args } => {
            serde_json::json!({
                "$call": {
                    "name": name,
                    "args": args.iter().map(lower_expr_to_json_from_ir).collect::<Vec<_>>()
                }
            })
        }
        ExprIR::StoreFieldAccessExpr {
            store_alias,
            field_name,
            ..
        } => {
            serde_json::json!({"$signal": format!("{}_{}", store_alias, field_name)})
        }
        other => serde_json::Value::String(format!("{:?}", other)),
    }
}

pub(crate) fn lower_type(ty: &TypeAnnotation) -> TypeIR {
    match ty {
        TypeAnnotation::Named {
            name, type_args, ..
        } => {
            let kind = name.clone();
            let args: Vec<TypeIR> = type_args.iter().map(lower_type).collect();
            TypeIR {
                kind,
                type_args: args,
                is_nullable: false,
                needs_clone: false,
                needs_partial_eq: false,
            }
        }
        TypeAnnotation::Optional { base, .. } => {
            let mut inner = lower_type(base);
            inner.is_nullable = true;
            inner
        }
        TypeAnnotation::Union { types, .. } => {
            // Preserve all union variants by joining kind strings
            let kinds: Vec<String> = types
                .iter()
                .map(|t| {
                    let lowered = lower_type(t);
                    lowered.kind
                })
                .collect();
            TypeIR {
                kind: kinds.join(" | "),
                type_args: vec![],
                is_nullable: false,
                needs_clone: false,
                needs_partial_eq: false,
            }
        }
        TypeAnnotation::Fn { .. } => TypeIR {
            kind: "function".into(),
            type_args: vec![],
            is_nullable: false,
            needs_clone: false,
            needs_partial_eq: false,
        },
    }
}
