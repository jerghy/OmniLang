//! Intent IR types and AST -> IR lowering.
//! Mirrors Python omniview/ir_types.py. Serializes to/from JSON via serde.

pub mod logic;
pub mod lower_logic;
pub use logic::*;

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
    StoreFieldAccessExpr {
        store_alias: String,
        field_name: String,
        call_args: Vec<ExprIR>,
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
pub enum ViewTree {
    Element(ElementIR),
    Conditional(Box<ConditionalIntent>),
    ListRendering(Box<ListRenderingIntent>),
    Text(String),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IRProgram {
    pub version: String,
    pub modules: HashMap<String, ModuleIR>,
    pub source_map: HashMap<String, SourceMapEntry>,
}

impl IRProgram {
    /// Collect all components across all modules.
    pub fn all_components(&self) -> HashMap<&str, &ComponentIR> {
        self.modules
            .iter()
            .flat_map(|(_, m)| m.components.iter())
            .map(|(k, v)| (k.as_str(), v))
            .collect()
    }

    /// Collect all stores across all modules.
    pub fn all_stores(&self) -> HashMap<&str, &StoreIR> {
        self.modules
            .iter()
            .flat_map(|(_, m)| m.stores.iter())
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
            let ir = lower_component(comp);
            components.insert(comp.name.clone(), ir);
        }

        for store in &module.stores {
            let ir = lower_store(store);
            stores.insert(store.name.clone(), ir);
        }

        for func in &module.functions {
            let ir = lower_logic::lower_function(func);
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

    IRProgram {
        version: "1.0".to_string(),
        modules,
        source_map: HashMap::new(),
    }
}

fn lower_component(comp: &ComponentDef) -> ComponentIR {
    let states: Vec<StateIntent> = comp
        .signals
        .iter()
        .map(|s| StateIntent {
            name: s.name.clone(),
            r#type: lower_type(&s.type_ann),
            initial_value: lower_expr(&s.init),
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
            body: lower_expr(&c.body),
            deps: vec![],
        })
        .collect();

    let effects: Vec<EffectIntent> = comp
        .lifecycles
        .iter()
        .filter(|lc| matches!(lc.kind, LifecycleKind::Effect))
        .map(|lc| EffectIntent {
            deps: lc.deps.clone(),
            body: lower_block(&lc.body),
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
            body: lower_block(&lc.body),
        })
        .collect();

    let fns: Vec<FunctionIR> = comp
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
            body: lower_block(&f.body),
        })
        .collect();

    let props: HashMap<String, TypeIR> = comp
        .props
        .iter()
        .map(|p| (p.name.clone(), lower_type(&p.type_ann)))
        .collect();

    let view = comp.view.as_ref().map(lower_view_block);

    let styles: Vec<StyleRuleIR> = comp.styles.iter().map(|s| {
        let properties: HashMap<String, ExprIR> = s.properties.iter().map(|(k, v)| {
            let ir_val = match v {
                StyleValue::Number(n) => ExprIR::LiteralExpr {
                    value: serde_json::Value::Number(serde_json::Number::from_f64(*n).unwrap_or_else(|| serde_json::Number::from(0))),
                },
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
                    let s: String = parts.iter().map(|p| match p {
                        StyleValuePart::Ident(id) => id.clone(),
                        StyleValuePart::Number(n) => n.to_string(),
                        StyleValuePart::Dimension(val, unit) => format!("{}{}", val, unit),
                        StyleValuePart::HexColor(h) => h.clone(),
                    }).collect::<Vec<_>>().join(" ");
                    ExprIR::LiteralExpr {
                        value: serde_json::Value::String(s),
                    }
                }
            };
            (k.clone(), ir_val)
        }).collect();
        StyleRuleIR {
            class_name: s.class_name.clone(),
            properties,
            output_mode: "css".into(),
        }
    }).collect();

    ComponentIR {
        name: comp.name.clone(),
        generic_params: vec![],
        props,
        states,
        computeds,
        effects,
        lifecycles,
        fns,
        store_refs: comp.store_refs.clone(),
        refs: vec![],
        view,
        styles,
    }
}

fn lower_store(store: &StoreDef) -> StoreIR {
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
                _ => serde_json::Value::String(format!("{:?}", s.init)),
            };
            (s.name.clone(), val)
        })
        .collect();

    let getters: HashMap<String, ExprIR> = store
        .getters
        .iter()
        .map(|g| (g.name.clone(), lower_expr(&g.body)))
        .collect();

    let actions: Vec<serde_json::Value> = store
        .actions
        .iter()
        .map(|_| serde_json::Value::Null)
        .collect();

    StoreIR {
        name: store.name.clone(),
        signals,
        signal_defaults,
        getters,
        actions,
    }
}

fn lower_view_block(view: &ViewBlock) -> ViewTree {
    if let Some(first) = view.children.first() {
        lower_view_node(first)
    } else {
        ViewTree::Text("".into())
    }
}

fn lower_view_node(node: &ViewNode) -> ViewTree {
    match node {
        ViewNode::Element(el) => {
            let mut props = HashMap::new();
            let mut bindings = HashMap::new();
            let mut events = HashMap::new();

            for prop in &el.props {
                match prop {
                    ElementProp::Static { name, value } => {
                        props.insert(name.clone(), lower_expr(value));
                    }
                    ElementProp::Bind { name, signal_path } => {
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
                            EventHandler::InlineFn { body, .. } => EventHandlerIR {
                                kind: "inline_fn".into(),
                                body: Some(lower_block(body)),
                                r#ref: None,
                            },
                            EventHandler::FnRef(fn_name) => EventHandlerIR {
                                kind: "fn_ref".into(),
                                body: None,
                                r#ref: Some(fn_name.clone()),
                            },
                            EventHandler::StoreAction { store, action } => EventHandlerIR {
                                kind: "store_action".into(),
                                body: None,
                                r#ref: Some(format!("{}.{}", store, action)),
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
                                entries.iter()
                                    .map(|(k, _v)| k.clone())
                                    .collect::<Vec<_>>()
                                    .join(" ")
                            }
                        };
                        props.insert("class".into(), ExprIR::LiteralExpr {
                            value: serde_json::Value::String(class_str),
                        });
                    }
                    ElementProp::Style { entries: _entries } => {
                        // style: { key: value, ... } — passed as inline_style
                        // TODO: lower style entries to inline_style field
                    }
                    ElementProp::TestId(id) => {
                        // test-id="..." attribute
                        props.insert("data-testid".into(), ExprIR::LiteralExpr {
                            value: serde_json::Value::String(id.clone()),
                        });
                    }
                    ElementProp::Ref(r) => {
                        // ref="..." attribute
                        props.insert("data-ref".into(), ExprIR::LiteralExpr {
                            value: serde_json::Value::String(r.clone()),
                        });
                    }
                }
            }

            let children: Vec<ViewTree> = el.children.iter().map(lower_view_node).collect();

            ViewTree::Element(ElementIR {
                tag: el.tag.clone(),
                props,
                bindings,
                events,
                style_classes: None,
                inline_style: None,
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
            // Represent interpolation as a text node containing an IdentifierExpr
            if let Expr::VarRef(name, _) = expr {
                ViewTree::Text(format!("{{{}}}", name))
            } else {
                ViewTree::Text("{...}".into())
            }
        }
        ViewNode::IfDirective(if_dir) => {
            let branches: Vec<ConditionalBranch> = std::iter::once(ConditionalBranch {
                condition: lower_expr(&if_dir.condition),
                body: Some(ViewTree::Element(ElementIR {
                    tag: "fragment".into(),
                    props: HashMap::new(),
                    bindings: HashMap::new(),
                    events: HashMap::new(),
                    style_classes: None,
                    inline_style: None,
                    children: if_dir.body.iter().map(lower_view_node).collect(),
                    test_id: None,
                    ref_name: None,
                    construction_kind: "conditional".into(),
                source_line: 0,
                source_col: 0,
                })),
            })
            .chain(
                if_dir
                    .else_ifs
                    .iter()
                    .map(|(cond, body)| ConditionalBranch {
                        condition: lower_expr(cond),
                        body: Some(ViewTree::Element(ElementIR {
                            tag: "fragment".into(),
                            props: HashMap::new(),
                            bindings: HashMap::new(),
                            events: HashMap::new(),
                            style_classes: None,
                            inline_style: None,
                            children: body.iter().map(lower_view_node).collect(),
                            test_id: None,
                            ref_name: None,
                            construction_kind: "conditional".into(),
                            source_line: 0,
                            source_col: 0,
                        })),
                    }),
            )
            .collect();

            let fallback = if_dir.else_body.as_ref().map(|body| {
                ViewTree::Element(ElementIR {
                    tag: "fragment".into(),
                    props: HashMap::new(),
                    bindings: HashMap::new(),
                    events: HashMap::new(),
                    style_classes: None,
                    inline_style: None,
                    children: body.iter().map(lower_view_node).collect(),
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
                        children: nodes.iter().map(lower_view_node).collect(),
                        test_id: None,
                        ref_name: None,
                        construction_kind: "list".into(),
                        source_line: 0,
                        source_col: 0,
                    }))
                }
            };

            ViewTree::ListRendering(Box::new(ListRenderingIntent {
                iterable: lower_expr(&for_dir.iterable),
                item_binding: BindingIR {
                    name: for_dir.item_name.clone(),
                    r#type: None,
                },
                body: lower_body(&for_dir.body),
                key_expr: None,
                empty_body: for_dir.empty_body.as_ref().and_then(|nodes| lower_body(nodes.as_slice())),
            }))
        }
        ViewNode::SlotDirective(name) => ViewTree::Text(format!("<slot name={:?}>", name)),
    }
}

fn lower_stmt(stmt: &Stmt) -> StatementIR {
    match stmt {
        Stmt::VarDecl { name, init, .. } => StatementIR::LetStmtIR {
            name: name.clone(),
            value: init.as_ref().map(lower_expr),
            r#type: None,
        },
        Stmt::Return { value, .. } => StatementIR::ReturnStmtIR {
            value: value.as_ref().map(lower_expr),
        },
        Stmt::ExprStmt { expr, .. } => {
            if let Some(mutation) = try_lower_mutation(expr) {
                StatementIR::MutationStmt {
                    intent: Some(mutation),
                }
            } else {
                StatementIR::ExprStmtIR {
                    expr: lower_expr(expr),
                }
            }
        }
        Stmt::If {
            condition,
            then_body,
            else_body,
            ..
        } => StatementIR::IfStmtIR {
            condition: lower_expr(condition),
            then_body: lower_block(then_body),
            else_body: else_body.as_ref().map(lower_block),
        },
        Stmt::Print { newline: _, arg, .. } => StatementIR::ExprStmtIR {
            expr: ExprIR::CallExpr {
                name: "print".into(),
                args: vec![lower_expr(arg)],
            },
        },
        _ => StatementIR::ExprStmtIR {
            expr: ExprIR::LiteralExpr {
                value: serde_json::Value::Null,
            },
        },
    }
}

/// Try to lower an expression statement as a mutation intent.
/// Handles: MethodCall, FuncCall(FieldAccess), and Assignment to signals.
fn try_lower_mutation(expr: &Expr) -> Option<MutationIntent> {
    // Pattern 1: Expr::MethodCall { object: VarRef("count"), method: "set", args: [value] }
    if let Expr::MethodCall {
        object, method, args, ..
    } = expr
    {
        if let Expr::VarRef(target_name, _) = object.as_ref() {
            return mutation_for_method(target_name, method, args.first());
        }
    }
    // Pattern 2: Expr::FuncCall { func: FieldAccess { VarRef("count"), "set" }, args: [value] }
    if let Expr::FuncCall { func, args, .. } = expr {
        if let Expr::FieldAccess { object, field, .. } = func.as_ref() {
            if let Expr::VarRef(target_name, _) = object.as_ref() {
                return mutation_for_method(target_name, field, args.first());
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
                        amount: Some(lower_expr(right)),
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
                        amount: Some(lower_expr(right)),
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
                        amount: Some(lower_expr(right)),
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
) -> Option<MutationIntent> {
    let target = SignalRefIR {
        owner: None,
        name: target_name.into(),
    };
    let amount = amount.map(lower_expr);
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

pub(crate) fn lower_block(block: &Block) -> BlockIR {
    let mut statements: Vec<StatementIR> = block
        .stmts
        .iter()
        .map(|stmt| lower_stmt(stmt))
        .collect();

    // Include trailing expression as an expression statement
    if let Some(ref expr) = block.trailing_expr {
        if let Some(mutation) = try_lower_mutation(expr) {
            statements.push(StatementIR::MutationStmt {
                intent: Some(mutation),
            });
        } else {
            statements.push(StatementIR::ExprStmtIR {
                expr: lower_expr(expr),
            });
        }
    }

    BlockIR {
        statements,
        execution_mode: "client-only".into(),
    }
}

pub(crate) fn lower_expr(expr: &Expr) -> ExprIR {
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
        Expr::VarRef(name, _) => ExprIR::IdentifierExpr { name: name.clone() },
        Expr::BinaryOp {
            left, op, right, ..
        } => ExprIR::BinaryOpExpr {
            op: format!("{:?}", op).to_lowercase(),
            left: Box::new(lower_expr(left)),
            right: Box::new(lower_expr(right)),
        },
        Expr::UnaryOp { op, operand, .. } => ExprIR::UnaryOpExpr {
            op: format!("{:?}", op).to_lowercase(),
            operand: Box::new(lower_expr(operand)),
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
                        call_args: args.iter().map(lower_expr).collect(),
                    };
                }
            }
            // Regular function call
            if let Expr::VarRef(name, _) = func.as_ref() {
                ExprIR::CallExpr {
                    name: name.clone(),
                    args: args.iter().map(lower_expr).collect(),
                }
            } else {
                ExprIR::LiteralExpr {
                    value: serde_json::Value::Null,
                }
            }
        }
        Expr::FieldAccess { object, field, .. } => ExprIR::FieldAccessExpr {
            object: Box::new(lower_expr(object)),
            field: field.clone(),
        },
        Expr::MethodCall {
            object, method, args, ..
        } => {
            // Lower signal.get() / signal.set(value) / array.len()
            if method == "get" {
                ExprIR::FieldAccessExpr {
                    object: Box::new(lower_expr(object)),
                    field: "value".into(),
                }
            } else if method == "len" {
                // Array length: tasks.len() → tasks.value.length
                ExprIR::FieldAccessExpr {
                    object: Box::new(ExprIR::FieldAccessExpr {
                        object: Box::new(lower_expr(object)),
                        field: "value".into(),
                    }),
                    field: "length".into(),
                }
            } else if method == "set" {
                // signal.set(value) → MutationIntent (handled at statement level)
                // For expression context, return identifier
                ExprIR::IdentifierExpr {
                    name: method.clone(),
                }
            } else if let Expr::VarRef(obj_name, _) = object.as_ref() {
                ExprIR::StoreFieldAccessExpr {
                    store_alias: obj_name.clone(),
                    field_name: method.clone(),
                    call_args: args.iter().map(lower_expr).collect(),
                }
            } else {
                ExprIR::LiteralExpr {
                    value: serde_json::Value::Null,
                }
            }
        },
        Expr::StoreFieldAccess {
            store_alias,
            field_name,
            ..
        } => ExprIR::StoreFieldAccessExpr {
            store_alias: store_alias.clone(),
            field_name: field_name.clone(),
            call_args: vec![],
        },
        Expr::ArrayLiteral { elements, .. } => {
            let items: Vec<serde_json::Value> = elements
                .iter()
                .map(|e| lower_expr_to_json(e))
                .collect();
            ExprIR::LiteralExpr {
                value: serde_json::Value::Array(items),
            }
        }
        Expr::MapLiteral { entries, .. } => {
            let map: serde_json::Map<String, serde_json::Value> = entries
                .iter()
                .map(|(k, v)| {
                    let key = match lower_expr(k) {
                        ExprIR::LiteralExpr { value } => match value {
                            serde_json::Value::String(s) => s,
                            other => format!("{}", other),
                        },
                        other => format!("{:?}", other),
                    };
                    (key, lower_expr_to_json(v))
                })
                .collect();
            ExprIR::LiteralExpr {
                value: serde_json::Value::Object(map),
            }
        }
        Expr::TernaryExpr { condition, then_expr, else_expr, .. } => ExprIR::TernaryIR {
            cond: Box::new(lower_expr(condition)),
            then_expr: Box::new(lower_expr(then_expr)),
            else_expr: Box::new(lower_expr(else_expr)),
        },
        _ => ExprIR::LiteralExpr {
            value: serde_json::Value::Null,
        },
    }
}

/// Lower an expression to a serde_json::Value (for literal extraction).
fn lower_expr_to_json(expr: &Expr) -> serde_json::Value {
    match lower_expr(expr) {
        ExprIR::LiteralExpr { value } => value,
        ExprIR::IdentifierExpr { name } => serde_json::Value::String(name),
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
            if let Some(first) = types.first() {
                lower_type(first)
            } else {
                TypeIR {
                    kind: "void".into(),
                    type_args: vec![],
                    is_nullable: false,
                    needs_clone: false,
                    needs_partial_eq: false,
                }
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
