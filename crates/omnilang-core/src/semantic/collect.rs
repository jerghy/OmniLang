//! Pass 1: Collect symbols from the AST.
//!
//! Walks all modules, components, stores, functions, and blocks to declare every
//! symbol (variables, props, signals, computed, events, refs, actions, getters,
//! params, etc.) into the [`SymbolTable`](super::symbols::SymbolTable).

use super::helpers::type_to_string;
use super::symbols::{Symbol, SymbolKind};
use super::SemanticAnalyzer;
use crate::parser::ast::*;

impl SemanticAnalyzer {
    /// Collect all module-level symbols (components, stores, functions, structs,
    /// tagged unions, imports) and recurse into each scope.
    pub(super) fn collect_module_symbols(&mut self, module: &Module) {
        // 批次 2/3 内建语义函数：navigate/back（导航，spec §2.3）+ bp（断点
        // 条件，批次 3 spec §2.2）是全局内建（parser 侧经 BUILTIN_FUNCTIONS
        // 登记、以普通 FuncCall 承载），语义层在此按 @form/@load 隐式符号
        // 先例（SymbolKind::Unknown + declared_at: 0——对 dead-code/type lints
        // 不可见）播种到模块根作用域，使组件 fn/事件处理器/生命周期体内的
        // navigate()/back()/bp() 调用可解析。缺失时 resolve 会对调用点抛
        // E0001「Undeclared variable」——保留名已禁止组件成员声明同名，用户
        // 无法自行补声明。
        for name in ["navigate", "back", "bp"] {
            self.declare_or_warn(Symbol {
                name: name.into(),
                kind: SymbolKind::Unknown,
                type_ann: None,
                mutable: false,
                declared_at: 0,
            });
        }

        for comp in &module.components {
            self.symbols.push_scope(&format!("component:{}", comp.name));

            for prop in &comp.props {
                self.declare_or_warn(Symbol {
                    name: prop.name.clone(),
                    kind: SymbolKind::Prop,
                    type_ann: Some(type_to_string(&prop.type_ann)),
                    mutable: false,
                    declared_at: prop.type_ann.span().start,
                });
                // Traverse default value expression for inner VarDecls (block expressions)
                if let Some(ref default) = prop.default {
                    self.collect_expr_symbols(default);
                }
            }

            for sig in &comp.signals {
                self.declare_or_warn(Symbol {
                    name: sig.name.clone(),
                    kind: SymbolKind::Signal,
                    type_ann: Some(type_to_string(&sig.type_ann)),
                    mutable: true,
                    declared_at: sig.type_ann.span().start,
                });
                // Traverse init expression for inner VarDecls (block expressions)
                self.collect_expr_symbols(&sig.init);
            }

            for c in &comp.computeds {
                self.declare_or_warn(Symbol {
                    name: c.name.clone(),
                    kind: SymbolKind::Computed,
                    type_ann: Some(type_to_string(&c.type_ann)),
                    mutable: false,
                    declared_at: c.type_ann.span().start,
                });
                // Traverse computed body expression for inner VarDecls (block expressions)
                self.collect_expr_symbols(&c.body);
            }

            for ev in &comp.events {
                self.declare_or_warn(Symbol {
                    name: ev.name.clone(),
                    kind: SymbolKind::Event,
                    type_ann: Some(type_to_string(&ev.type_ann)),
                    mutable: false,
                    declared_at: ev.type_ann.span().start,
                });
            }

            for r in &comp.refs {
                self.declare_or_warn(Symbol {
                    name: r.name.clone(),
                    kind: SymbolKind::Ref,
                    type_ann: Some(type_to_string(&r.type_ann)),
                    mutable: false,
                    declared_at: r.type_ann.span().start,
                });
            }

            // Functions, actions, getters
            for f in &comp.fns {
                self.collect_fn_symbols(f);
            }
            for a in &comp.actions {
                self.collect_action_symbols(a);
            }
            for g in &comp.getters {
                self.declare_or_warn(Symbol {
                    name: g.name.clone(),
                    kind: SymbolKind::Getter,
                    type_ann: Some(type_to_string(&g.type_ann)),
                    mutable: false,
                    declared_at: g.type_ann.span().start,
                });
                // Traverse getter body expression for inner VarDecls (block expressions)
                self.collect_expr_symbols(&g.body);
            }

            // Lifecycle hooks
            for lc in &comp.lifecycles {
                self.collect_block_symbols(&lc.body);
            }

            // Add store references from @store declarations to component scope
            for store_ref in &comp.store_refs {
                self.declare_or_warn(Symbol {
                    name: store_ref.clone(),
                    kind: SymbolKind::Store,
                    type_ann: None,
                    mutable: false,
                    declared_at: 0,
                });
            }

            // Body-level slot declarations (ADR-053 D5) register their names
            // as symbols so `{header}` interpolations resolve to a slot
            // symbol instead of relying on interpolation suppression.
            // Duplicate slot names surface as E0002 warnings via
            // declare_or_warn. The unnamed (`@slot default`) slot has no
            // name to register. v2 (design D4): slots carry no type — the
            // type constraint was deleted (slot = parent-provided content).
            for slot in &comp.slots {
                if let Some(ref name) = slot.name {
                    self.declare_or_warn(Symbol {
                        name: name.clone(),
                        kind: SymbolKind::Slot,
                        type_ann: None,
                        mutable: false,
                        declared_at: slot.span.start,
                    });
                }
            }

            // Builtin implicit symbols for @form / @load intents (spec
            // 2026-08-08 §4.1/§4.2). V-F8 (validate_form_load) reserves
            // formErrors/formValid/formSubmitting and loading/error/data as
            // compiler-generated names — users cannot declare them, so the
            // semantic pass must seed the user-visible surfaces here: `form`
            // (FieldAccess object for form.errors/form.valid/form.submitting)
            // plus the @load three-state signals. SymbolKind::Unknown with
            // declared_at: 0 keeps them invisible to dead-code/type lints;
            // type_ann mirrors the codegen-side shapes so binary-op checks
            // (`error != ""`) stay well-typed.
            let (has_form, loads) =
                scan_form_load(comp.view.as_ref().map(|v| v.children.as_slice()));
            if has_form {
                self.declare_or_warn(Symbol {
                    name: "form".into(),
                    kind: SymbolKind::Unknown,
                    type_ann: None,
                    mutable: false,
                    declared_at: 0,
                });
            }
            // Task 3 起 @load 可命名（`@load history={fetchHistory}`）——
            // 命名实例信号面 `{name}Loading/Error/Data/Retry/Reset`（分页
            // 追加 `{name}Page/HasMore/LoadingMore/LoadMore`）须同样入组件
            // 作用域，否则视图体引用 `{name}Data` 报 E0001（spec 2026-08-09
            // §2.1/§2.2 信号表）；匿名 v0 形态沿用既有播种（loading/error/
            // data/retry，并补充 reset——codegen 侧 Reset 回调无条件可用）。
            // 重复同名由 merge_scan 去重（first-wins，与旧行为一致）。
            for load in &loads {
                let prefix = |suffix: &str| {
                    if load.name.is_empty() {
                        suffix.to_lowercase().to_string()
                    } else {
                        format!("{}{}", load.name, suffix)
                    }
                };
                for (suffix, ty) in [
                    ("Loading", Some("Bool")),
                    ("Error", Some("String")),
                    ("Data", Some("Array")),
                    ("Retry", None),
                    ("Reset", None),
                ] {
                    self.declare_or_warn(Symbol {
                        name: prefix(suffix),
                        kind: SymbolKind::Unknown,
                        type_ann: ty.map(str::to_string),
                        mutable: false,
                        declared_at: 0,
                    });
                }
                if load.paginated {
                    for (suffix, ty) in [
                        ("Page", Some("i32")),
                        ("HasMore", Some("Bool")),
                        ("LoadingMore", Some("Bool")),
                        ("LoadMore", None),
                    ] {
                        self.declare_or_warn(Symbol {
                            name: prefix(suffix),
                            kind: SymbolKind::Unknown,
                            type_ann: ty.map(str::to_string),
                            mutable: false,
                            declared_at: 0,
                        });
                    }
                }
            }

            self.symbols.pop_scope(); // component scope

            // Collect computed dependencies (merged from former Pass 3)
            let deps: Vec<String> = comp.computeds.iter().map(|c| c.name.clone()).collect();
            self.dependencies.insert(comp.name.clone(), deps);
        }

        // Stores — declare store names as module-level symbols so store field
        // accesses in design token expressions (e.g. `ThemeStore.primaryColor`)
        // resolve without producing E0001 for the store name.
        for store in &module.stores {
            self.declare_or_warn(Symbol {
                name: store.name.clone(),
                kind: SymbolKind::Store,
                type_ann: None,
                mutable: false,
                declared_at: 0,
            });
            self.symbols.push_scope(&format!("store:{}", store.name));

            for sig in &store.signals {
                self.declare_or_warn(Symbol {
                    name: sig.name.clone(),
                    kind: SymbolKind::Signal,
                    type_ann: Some(type_to_string(&sig.type_ann)),
                    mutable: true,
                    declared_at: sig.type_ann.span().start,
                });
                // Traverse init expression for inner VarDecls (block expressions)
                self.collect_expr_symbols(&sig.init);
            }

            for g in &store.getters {
                self.declare_or_warn(Symbol {
                    name: g.name.clone(),
                    kind: SymbolKind::Getter,
                    type_ann: Some(type_to_string(&g.type_ann)),
                    mutable: false,
                    declared_at: g.type_ann.span().start,
                });
                // Traverse getter body expression for inner VarDecls (block expressions)
                self.collect_expr_symbols(&g.body);
            }

            for a in &store.actions {
                self.collect_action_symbols(a);
            }

            self.symbols.pop_scope();
        }

        // Top-level functions
        for f in &module.functions {
            self.collect_fn_symbols(f);
        }

        // Structs
        for s in &module.structs {
            self.declare_or_warn(Symbol {
                name: s.name.clone(),
                kind: SymbolKind::Unknown,
                type_ann: None,
                mutable: false,
                declared_at: s.span.start,
            });
        }

        // Tagged unions — register variant names as symbols so bare variant
        // expressions like `Loading` (which parse as VarRef) resolve correctly
        for tu in &module.tagged_unions {
            for variant in &tu.variants {
                self.declare_or_warn(Symbol {
                    name: variant.name.clone(),
                    kind: SymbolKind::Unknown,
                    type_ann: Some(tu.name.clone()),
                    mutable: false,
                    declared_at: 0,
                });
            }
        }

        // Module-level const declarations — collect as Const symbols
        for const_def in &module.consts {
            self.declare_or_warn(Symbol {
                name: const_def.name.clone(),
                kind: SymbolKind::Const,
                type_ann: infer_type_from_expr(&const_def.value),
                mutable: false,
                declared_at: const_def.span.start,
            });
        }

        // Imports — declare imported names as symbols so they can be resolved.
        // Cross-file validation (validate_cross_file) checks these against pub symbols.
        for import in &module.imports {
            match import {
                ImportStmt::Named(ni) => {
                    for name in &ni.names {
                        self.declare_or_warn(Symbol {
                            name: name.clone(),
                            kind: SymbolKind::Unknown,
                            type_ann: None,
                            mutable: false,
                            declared_at: ni.span.start,
                        });
                    }
                }
                ImportStmt::TypeOnly(ti) => {
                    for name in &ti.names {
                        self.declare_or_warn(Symbol {
                            name: name.clone(),
                            kind: SymbolKind::Unknown,
                            type_ann: None,
                            mutable: false,
                            declared_at: ti.span.start,
                        });
                    }
                }
                ImportStmt::Namespace(nsi) => {
                    self.declare_or_warn(Symbol {
                        name: nsi.alias.clone(),
                        kind: SymbolKind::Unknown,
                        type_ann: None,
                        mutable: false,
                        declared_at: nsi.span.start,
                    });
                }
                ImportStmt::Lib(_) => {
                    // "import omniview" — marker import, no symbols to declare
                }
            }
        }
    }

    fn collect_fn_symbols(&mut self, f: &FunctionDef) {
        self.declare_or_warn(Symbol {
            name: f.name.clone(),
            kind: SymbolKind::Function,
            type_ann: f.return_type.as_ref().map(type_to_string),
            mutable: false,
            declared_at: f.span.start,
        });
        self.symbols.push_scope(&format!("fn:{}", f.name));
        for p in &f.params {
            self.declare_or_warn(Symbol {
                name: p.name.clone(),
                kind: SymbolKind::Param,
                type_ann: p.type_ann.as_ref().map(type_to_string),
                mutable: false,
                declared_at: p.type_ann.as_ref().map(|t| t.span().start).unwrap_or(0),
            });
        }
        self.collect_block_symbols(&f.body);
        self.symbols.pop_scope();
    }

    fn collect_action_symbols(&mut self, a: &ActionDef) {
        self.declare_or_warn(Symbol {
            name: a.name.clone(),
            kind: SymbolKind::Action,
            type_ann: None,
            mutable: false,
            declared_at: 0,
        });
        self.symbols.push_scope(&format!("action:{}", a.name));
        for p in &a.params {
            self.declare_or_warn(Symbol {
                name: p.name.clone(),
                kind: SymbolKind::Param,
                type_ann: p.type_ann.as_ref().map(type_to_string),
                mutable: false,
                declared_at: p.type_ann.as_ref().map(|t| t.span().start).unwrap_or(0),
            });
        }
        self.collect_block_symbols(&a.body);
        self.symbols.pop_scope();
    }

    fn collect_block_symbols(&mut self, block: &Block) {
        for stmt in &block.stmts {
            match stmt {
                Stmt::VarDecl {
                    name,
                    type_ann,
                    mutable,
                    ..
                } => {
                    self.declare_or_warn(Symbol {
                        name: name.clone(),
                        kind: SymbolKind::Variable,
                        type_ann: type_ann.as_ref().map(type_to_string),
                        mutable: *mutable,
                        declared_at: 0,
                    });
                }
                Stmt::ForIn { var_name, body, .. } => {
                    self.declare_or_warn(Symbol {
                        name: var_name.clone(),
                        kind: SymbolKind::Variable,
                        type_ann: None,
                        mutable: false,
                        declared_at: 0,
                    });
                    self.collect_block_symbols(body);
                }
                Stmt::If {
                    then_body,
                    else_ifs,
                    else_body,
                    ..
                } => {
                    self.collect_block_symbols(then_body);
                    for (_, body) in else_ifs {
                        self.collect_block_symbols(body);
                    }
                    if let Some(body) = else_body {
                        self.collect_block_symbols(body);
                    }
                }
                Stmt::While { body, .. } => {
                    self.collect_block_symbols(body);
                }
                Stmt::TryCatch {
                    try_body,
                    catch_var,
                    catch_body,
                    finally_body,
                    ..
                } => {
                    self.collect_block_symbols(try_body);
                    if let Some(ref var) = catch_var {
                        self.declare_or_warn(Symbol {
                            name: var.clone(),
                            kind: SymbolKind::Variable,
                            type_ann: None,
                            mutable: false,
                            declared_at: 0,
                        });
                    }
                    if let Some(ref body) = catch_body {
                        self.collect_block_symbols(body);
                    }
                    if let Some(ref body) = finally_body {
                        self.collect_block_symbols(body);
                    }
                }
                _ => {}
            }
        }
    }

    /// Collect symbols from within expressions.
    ///
    /// Currently handles `Expr::Block` (block expressions that may contain `let` declarations)
    /// and `Expr::ClosureExpr` (closures whose body may contain block expressions).
    /// Recurses into child expressions where VarDecls could appear.
    fn collect_expr_symbols(&mut self, expr: &Expr) {
        match expr {
            Expr::Block { body, .. } => {
                self.symbols.push_scope("block_expr");
                self.collect_block_symbols(body);
                self.symbols.pop_scope();
            }
            Expr::ClosureExpr { params, body, .. } => {
                self.symbols.push_scope("closure");
                for p in params {
                    self.declare_or_warn(Symbol {
                        name: p.clone(),
                        kind: SymbolKind::Variable,
                        type_ann: None,
                        mutable: false,
                        declared_at: 0,
                    });
                }
                self.collect_expr_symbols(body);
                self.symbols.pop_scope();
            }
            Expr::IfExpr {
                condition,
                then_expr,
                else_expr,
                ..
            } => {
                self.collect_expr_symbols(condition);
                self.collect_expr_symbols(then_expr);
                self.collect_expr_symbols(else_expr);
            }
            Expr::TernaryExpr {
                condition,
                then_expr,
                else_expr,
                ..
            } => {
                self.collect_expr_symbols(condition);
                self.collect_expr_symbols(then_expr);
                self.collect_expr_symbols(else_expr);
            }
            _ => {}
        }
    }
}

/// Infer a simple type annotation string from a literal expression.
fn infer_type_from_expr(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Literal(Literal::String(_), _) => Some("String".to_string()),
        Expr::Literal(Literal::Int(_), _) => Some("Int".to_string()),
        Expr::Literal(Literal::Float(_), _) => Some("Float".to_string()),
        Expr::Literal(Literal::Bool(_), _) => Some("Bool".to_string()),
        _ => None,
    }
}

/// Recursively scan view nodes for @form / @load intents (spec 2026-08-08
/// §4.1/§4.2). `walk_view`'s on_element callback never sees
/// `ViewNode::LoadDirective` (it treats it as a leaf) and ElementProp::Form
/// is a bare attribute, so this private scanner walks the AST view tree
/// directly. Returns `(has_form, loads)` — every @load directive's seeding
/// descriptor (Task 3 起支持多实例命名 @load；同名 first-wins 由
/// merge_scan 去重，决定内置信号播种集）。
struct LoadDesc {
    /// @load 实例名（空 = 匿名 v0 形态，播种 loading/error/data/retry/reset）。
    name: String,
    /// 分页模式位（播种 {name}Page/HasMore/LoadingMore/LoadMore）。
    paginated: bool,
}

fn scan_form_load(nodes: Option<&[ViewNode]>) -> (bool, Vec<LoadDesc>) {
    let Some(nodes) = nodes else {
        return (false, Vec::new());
    };
    let mut has_form = false;
    let mut loads = Vec::new();
    for node in nodes {
        match node {
            ViewNode::Element(el) => {
                has_form |= el.props.iter().any(|p| matches!(p, ElementProp::Form));
                merge_scan(
                    &mut has_form,
                    &mut loads,
                    scan_form_load(Some(&el.children)),
                );
            }
            ViewNode::IfDirective(dir) => {
                merge_scan(&mut has_form, &mut loads, scan_form_load(Some(&dir.body)));
                for (_, body) in &dir.else_ifs {
                    merge_scan(&mut has_form, &mut loads, scan_form_load(Some(body)));
                }
                if let Some(body) = &dir.else_body {
                    merge_scan(&mut has_form, &mut loads, scan_form_load(Some(body)));
                }
            }
            ViewNode::ForDirective(dir) => {
                merge_scan(&mut has_form, &mut loads, scan_form_load(Some(&dir.body)));
                if let Some(body) = &dir.empty_body {
                    merge_scan(&mut has_form, &mut loads, scan_form_load(Some(body)));
                }
            }
            ViewNode::SlotDirective {
                default_content, ..
            } => {
                if let Some(body) = default_content {
                    merge_scan(&mut has_form, &mut loads, scan_form_load(Some(body)));
                }
            }
            ViewNode::LoadDirective { spec, .. } => {
                loads.push(LoadDesc {
                    name: spec.name.clone().unwrap_or_default(),
                    paginated: spec.mode.paginated,
                });
            }
            ViewNode::Text(_) | ViewNode::Interpolation(_) => {}
        }
    }
    (has_form, loads)
}

/// Merge a nested scan result into the accumulators (同名 first-wins——
/// 与旧单实例行为一致，重复匿名 @load 不产生 E0002 警告）。
fn merge_scan(has_form: &mut bool, loads: &mut Vec<LoadDesc>, next: (bool, Vec<LoadDesc>)) {
    *has_form |= next.0;
    for load in next.1 {
        if !loads.iter().any(|l| l.name == load.name) {
            loads.push(load);
        }
    }
}
