//! Semantic analysis — multi-pass: collect, resolve, type-check, deps, validate.

pub mod symbols;

use std::collections::HashMap;

use crate::error::Diagnostic;
use crate::lexer::Span;
use crate::parser::ast::*;
use symbols::*;

/// Result of semantic analysis.
#[derive(Debug)]
pub struct SemanticResult {
    pub symbols: SymbolTable,
    pub dependencies: HashMap<String, Vec<String>>,
}

/// Run all semantic analysis passes on the parsed program.
pub fn analyze(program: &Program) -> Result<SemanticResult, Vec<Diagnostic>> {
    let analyzer = SemanticAnalyzer::new();
    analyzer.analyze(program)
}

pub struct SemanticAnalyzer {
    symbols: SymbolTable,
    diagnostics: Vec<Diagnostic>,
    dependencies: HashMap<String, Vec<String>>,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        SemanticAnalyzer {
            symbols: SymbolTable::new(),
            diagnostics: Vec::new(),
            dependencies: HashMap::new(),
        }
    }

    pub fn analyze(mut self, program: &Program) -> Result<SemanticResult, Vec<Diagnostic>> {
        // Pass 1: Collect symbols
        self.collect_symbols(program);

        // Pass 2: Resolve references (placeholder for now)
        self.resolve_references(program);

        // Pass 3: Type checking (placeholder)
        self.type_check(program);

        // Pass 4: Collect dependencies (placeholder)
        self.collect_dependencies(program);

        // Pass 5: Validate constraints
        self.validate_constraints(program);

        if self.diagnostics.iter().any(|d| matches!(d.severity, crate::error::Severity::Error)) {
            return Err(self.diagnostics);
        }

        Ok(SemanticResult {
            symbols: self.symbols,
            dependencies: self.dependencies,
        })
    }

    // ── Pass 1: Collect symbols ────────────────────────

    fn collect_symbols(&mut self, program: &Program) {
        for comp in &program.components {
            self.symbols.push_scope(&format!("component:{}", comp.name));

            for prop in &comp.props {
                self.symbols.declare(Symbol {
                    name: prop.name.clone(),
                    kind: SymbolKind::Prop,
                    type_ann: Some(type_to_string(&prop.type_ann)),
                    mutable: false,
                    declared_at: prop.type_ann.span().start,
                }).ok();
            }

            for sig in &comp.signals {
                self.symbols.declare(Symbol {
                    name: sig.name.clone(),
                    kind: SymbolKind::Signal,
                    type_ann: Some(type_to_string(&sig.type_ann)),
                    mutable: true,
                    declared_at: sig.type_ann.span().start,
                }).ok();
            }

            for c in &comp.computeds {
                self.symbols.declare(Symbol {
                    name: c.name.clone(),
                    kind: SymbolKind::Computed,
                    type_ann: Some(type_to_string(&c.type_ann)),
                    mutable: false,
                    declared_at: c.type_ann.span().start,
                }).ok();
            }

            for ev in &comp.events {
                self.symbols.declare(Symbol {
                    name: ev.name.clone(),
                    kind: SymbolKind::Event,
                    type_ann: Some(type_to_string(&ev.type_ann)),
                    mutable: false,
                    declared_at: ev.type_ann.span().start,
                }).ok();
            }

            for r in &comp.refs {
                self.symbols.declare(Symbol {
                    name: r.name.clone(),
                    kind: SymbolKind::Ref,
                    type_ann: Some(type_to_string(&r.type_ann)),
                    mutable: false,
                    declared_at: r.type_ann.span().start,
                }).ok();
            }

            // Functions and actions
            for f in &comp.fns {
                self.collect_fn_symbols(f);
            }
            for a in &comp.actions {
                self.collect_action_symbols(a);
            }

            // Lifecycle hooks
            for lc in &comp.lifecycles {
                self.collect_block_symbols(&lc.body);
            }

            self.symbols.pop_scope(); // component scope
        }

        // Stores
        for store in &program.stores {
            self.symbols.push_scope(&format!("store:{}", store.name));

            for sig in &store.signals {
                self.symbols.declare(Symbol {
                    name: sig.name.clone(),
                    kind: SymbolKind::Signal,
                    type_ann: Some(type_to_string(&sig.type_ann)),
                    mutable: true,
                    declared_at: sig.type_ann.span().start,
                }).ok();
            }

            for g in &store.getters {
                self.symbols.declare(Symbol {
                    name: g.name.clone(),
                    kind: SymbolKind::Getter,
                    type_ann: Some(type_to_string(&g.type_ann)),
                    mutable: false,
                    declared_at: g.type_ann.span().start,
                }).ok();
            }

            for a in &store.actions {
                self.collect_action_symbols(a);
            }

            self.symbols.pop_scope();
        }

        // Top-level functions
        for f in &program.functions {
            self.collect_fn_symbols(f);
        }

        // Structs
        for s in &program.structs {
            let _ = self.symbols.declare(Symbol {
                name: s.name.clone(),
                kind: SymbolKind::Unknown,
                type_ann: None,
                mutable: false,
                declared_at: s.span.start,
            });
        }
    }

    fn collect_fn_symbols(&mut self, f: &FunctionDef) {
        let _ = self.symbols.declare(Symbol {
            name: f.name.clone(),
            kind: SymbolKind::Function,
            type_ann: None,
            mutable: false,
            declared_at: f.span.start,
        });
        self.symbols.push_scope(&format!("fn:{}", f.name));
        for p in &f.params {
            let _ = self.symbols.declare(Symbol {
                name: p.name.clone(),
                kind: SymbolKind::Param,
                type_ann: p.type_ann.as_ref().map(type_to_string),
                mutable: false,
                declared_at: 0,
            });
        }
        self.collect_block_symbols(&f.body);
        self.symbols.pop_scope();
    }

    fn collect_action_symbols(&mut self, a: &ActionDef) {
        let _ = self.symbols.declare(Symbol {
            name: a.name.clone(),
            kind: SymbolKind::Action,
            type_ann: None,
            mutable: false,
            declared_at: 0,
        });
        self.symbols.push_scope(&format!("action:{}", a.name));
        for p in &a.params {
            let _ = self.symbols.declare(Symbol {
                name: p.name.clone(),
                kind: SymbolKind::Param,
                type_ann: p.type_ann.as_ref().map(type_to_string),
                mutable: false,
                declared_at: 0,
            });
        }
        self.collect_block_symbols(&a.body);
        self.symbols.pop_scope();
    }

    fn collect_block_symbols(&mut self, block: &Block) {
        for stmt in &block.stmts {
            match stmt {
                Stmt::VarDecl { name, type_ann, mutable, .. } => {
                    let _ = self.symbols.declare(Symbol {
                        name: name.clone(),
                        kind: SymbolKind::Variable,
                        type_ann: type_ann.as_ref().map(type_to_string),
                        mutable: *mutable,
                        declared_at: 0,
                    });
                }
                Stmt::ForIn { var_name, body, .. } => {
                    let _ = self.symbols.declare(Symbol {
                        name: var_name.clone(),
                        kind: SymbolKind::Variable,
                        type_ann: None,
                        mutable: false,
                        declared_at: 0,
                    });
                    self.collect_block_symbols(body);
                }
                Stmt::If { then_body, else_ifs, else_body, .. } => {
                    self.collect_block_symbols(then_body);
                    for (_, body) in else_ifs {
                        self.collect_block_symbols(body);
                    }
                    if let Some(body) = else_body {
                        self.collect_block_symbols(body);
                    }
                }
                _ => {}
            }
        }
    }

    // ── Pass 2: Resolve references ─────────────────────

    fn resolve_references(&mut self, program: &Program) {
        // Walk all expressions and check VarRef names resolve
        for comp in &program.components {
            for f in &comp.fns {
                self.resolve_block_refs(&f.body);
            }
            for a in &comp.actions {
                self.resolve_block_refs(&a.body);
            }
        }
        for f in &program.functions {
            self.resolve_block_refs(&f.body);
        }
    }

    fn resolve_block_refs(&mut self, block: &Block) {
        for stmt in &block.stmts {
            self.resolve_stmt_refs(stmt);
        }
        if let Some(ref expr) = block.trailing_expr {
            self.resolve_expr_refs(expr);
        }
    }

    fn resolve_stmt_refs(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VarDecl { init, .. } => {
                if let Some(e) = init { self.resolve_expr_refs(e); }
            }
            Stmt::ExprStmt { expr, .. } => self.resolve_expr_refs(expr),
            Stmt::Return { value, .. } => { if let Some(e) = value { self.resolve_expr_refs(e); } }
            Stmt::Assign { value, .. } => self.resolve_expr_refs(value),
            Stmt::If { condition, then_body, else_ifs, else_body, .. } => {
                self.resolve_expr_refs(condition);
                self.resolve_block_refs(then_body);
                for (cond, body) in else_ifs {
                    self.resolve_expr_refs(cond);
                    self.resolve_block_refs(body);
                }
                if let Some(body) = else_body { self.resolve_block_refs(body); }
            }
            Stmt::While { condition, body, .. } => {
                self.resolve_expr_refs(condition);
                self.resolve_block_refs(body);
            }
            Stmt::ForIn { iterable, body, .. } => {
                self.resolve_expr_refs(iterable);
                self.resolve_block_refs(body);
            }
            Stmt::Print { arg, .. } => self.resolve_expr_refs(arg),
            _ => {}
        }
    }

    fn resolve_expr_refs(&mut self, expr: &Expr) {
        match expr {
            Expr::VarRef(name, span) => {
                if !self.symbols.contains(name) {
                    self.diagnostics.push(
                        Diagnostic::error(
                            "E0001",
                            &format!("Undeclared variable '{}'", name),
                            span,
                        )
                    );
                }
            }
            Expr::BinaryOp { left, right, .. } => {
                self.resolve_expr_refs(left);
                self.resolve_expr_refs(right);
            }
            Expr::UnaryOp { operand, .. } => self.resolve_expr_refs(operand),
            Expr::FuncCall { func, args, .. } => {
                self.resolve_expr_refs(func);
                for a in args { self.resolve_expr_refs(a); }
            }
            Expr::MethodCall { object, args, .. } => {
                self.resolve_expr_refs(object);
                for a in args { self.resolve_expr_refs(a); }
            }
            Expr::FieldAccess { object, .. } => self.resolve_expr_refs(object),
            Expr::IndexGet { object, index, .. } => {
                self.resolve_expr_refs(object);
                self.resolve_expr_refs(index);
            }
            Expr::ArrayLiteral { elements, .. } => {
                for e in elements { self.resolve_expr_refs(e); }
            }
            Expr::MapLiteral { entries, .. } => {
                for (k, v) in entries {
                    self.resolve_expr_refs(k);
                    self.resolve_expr_refs(v);
                }
            }
            Expr::IfExpr { condition, then_expr, else_expr, .. } => {
                self.resolve_expr_refs(condition);
                self.resolve_expr_refs(then_expr);
                self.resolve_expr_refs(else_expr);
            }
            _ => {}
        }
    }

    // ── Pass 3: Type checking ──────────────────────────

    fn type_check(&mut self, _program: &Program) {
        // TODO: Full type inference in a future iteration
    }

    // ── Pass 4: Collect dependencies ───────────────────

    fn collect_dependencies(&mut self, program: &Program) {
        for comp in &program.components {
            let mut deps = Vec::new();
            for c in &comp.computeds {
                deps.push(c.name.clone());
            }
            self.dependencies.insert(comp.name.clone(), deps);
        }
    }

    // ── Pass 5: Validate constraints ───────────────────

    fn validate_constraints(&mut self, _program: &Program) {
        // TODO: bind: on signals only, @for key check, break/continue in loops
    }
}

// ── Helpers ────────────────────────────────────────────

/// Convert a TypeAnnotation to a human-readable string.
fn type_to_string(ty: &TypeAnnotation) -> String {
    match ty {
        TypeAnnotation::Named { name, type_args, .. } => {
            if type_args.is_empty() {
                name.clone()
            } else {
                let args: Vec<_> = type_args.iter().map(type_to_string).collect();
                format!("{}<{}>", name, args.join(", "))
            }
        }
        TypeAnnotation::Optional { base, .. } => format!("{}?", type_to_string(base)),
        TypeAnnotation::Union { types, .. } => {
            let t: Vec<_> = types.iter().map(type_to_string).collect();
            t.join(" | ")
        }
        TypeAnnotation::Fn { params, return_type, .. } => {
            let p: Vec<_> = params.iter().map(type_to_string).collect();
            format!("fn({}) -> {}", p.join(", "), type_to_string(return_type))
        }
    }
}

/// Extension trait to get span from TypeAnnotation.
impl TypeAnnotation {
    pub fn span(&self) -> Span {
        match self {
            TypeAnnotation::Named { span, .. } => span.clone(),
            TypeAnnotation::Optional { span, .. } => span.clone(),
            TypeAnnotation::Union { span, .. } => span.clone(),
            TypeAnnotation::Fn { span, .. } => span.clone(),
        }
    }
}
