use crate::ast::*;
use crate::errors::ErrorCollector;
use crate::span::Span;
use std::collections::HashMap;

/// A scope node in the scope tree.
#[derive(Debug, Clone)]
pub struct Scope {
    pub name: String,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub symbols: HashMap<String, VarDeclInfo>,
    pub is_loop: bool,
    pub is_function: bool,
}

/// Information about a declared variable.
#[derive(Debug, Clone)]
pub struct VarDeclInfo {
    pub name: String,
    pub node_id: NodeId,
    pub mutable: bool,
    pub type_ann: Option<TypeRepr>,
    pub span: Span,
}

/// Result of the resolver pass.
#[derive(Debug, Clone)]
pub struct ScopeInfo {
    pub scopes: Vec<Scope>,
    pub global_scope: usize,
    pub func_scopes: HashMap<String, usize>,
    pub var_decl_to_scope: HashMap<NodeId, usize>,
    /// Set of function names for call resolution
    pub func_names: Vec<String>,
}

impl ScopeInfo {
    pub fn get_scope(&self, id: usize) -> &Scope {
        &self.scopes[id]
    }

    pub fn lookup_var(&self, name: &str, scope_id: usize) -> Option<&VarDeclInfo> {
        let mut sid = Some(scope_id);
        while let Some(s) = sid {
            let scope = &self.scopes[s];
            if let Some(info) = scope.symbols.get(name) {
                return Some(info);
            }
            sid = scope.parent;
        }
        None
    }
}

pub fn resolve(program: &Program, errors: &mut ErrorCollector) -> ScopeInfo {
    let mut resolver = Resolver {
        scopes: vec![],
        func_names: vec![],
        var_decl_to_scope: HashMap::new(),
        func_scopes: HashMap::new(),
        errors,
    };

    let global = resolver.push_scope("global", None);
    let mut info = ScopeInfo {
        scopes: vec![],
        global_scope: global,
        func_scopes: HashMap::new(),
        var_decl_to_scope: HashMap::new(),
        func_names: vec![],
    };

    // Collect function names first
    for f in &program.functions {
        resolver.func_names.push(f.name.clone());
    }

    // Resolve each function
    for f in &program.functions {
        resolver.resolve_function(f);
    }

    info.scopes = resolver.scopes;
    info.func_scopes = resolver.func_scopes;
    info.func_names = resolver.func_names;
    info.var_decl_to_scope = resolver.var_decl_to_scope;
    info
}

struct Resolver<'a> {
    scopes: Vec<Scope>,
    func_names: Vec<String>,
    var_decl_to_scope: HashMap<NodeId, usize>,
    func_scopes: HashMap<String, usize>,
    errors: &'a mut ErrorCollector,
}

impl<'a> Resolver<'a> {
    fn push_scope(&mut self, name: &str, parent: Option<usize>) -> usize {
        let id = self.scopes.len();
        self.scopes.push(Scope {
            name: name.to_string(),
            parent,
            children: vec![],
            symbols: HashMap::new(),
            is_loop: name == "for" || name == "while",
            is_function: name == "function",
        });
        if let Some(p) = parent {
            self.scopes[p].children.push(id);
        }
        id
    }

    fn resolve_function(&mut self, func: &FunctionDefNode) {
        let func_scope = self.push_scope("function", Some(0)); // parent = global
        self.func_scopes.insert(func.name.clone(), func_scope);

        // Register parameters
        for param in &func.params {
            let info = VarDeclInfo {
                name: param.name.clone(),
                node_id: param.node_id,
                mutable: false,
                type_ann: param.type_ann.clone(),
                span: param.span.clone(),
            };
            self.scopes[func_scope].symbols.insert(param.name.clone(), info);
            self.var_decl_to_scope.insert(param.node_id, func_scope);
        }

        if let Some(ref body) = func.body {
            self.resolve_block(body, func_scope);
        }
    }

    fn resolve_block(&mut self, block: &BlockNode, scope: usize) {
        for stmt in &block.stmts {
            self.resolve_stmt(stmt, scope);
        }
    }

    fn resolve_stmt(&mut self, stmt: &Stmt, scope: usize) {
        match stmt {
            Stmt::VarDecl(vd) => {
                // Check for underscore prefix
                if vd.name.starts_with('_') {
                    self.errors.add(
                        format!("Identifier '{}' starts with '_' which is reserved", vd.name),
                        Some(vd.span.clone()),
                        None,
                    );
                }
                // Check for duplicate in same scope
                if self.scopes[scope].symbols.contains_key(&vd.name) {
                    self.errors.add(
                        format!("Variable '{}' already declared in this scope", vd.name),
                        Some(vd.span.clone()),
                        None,
                    );
                }
                let info = VarDeclInfo {
                    name: vd.name.clone(),
                    node_id: vd.node_id,
                    mutable: vd.mutable,
                    type_ann: vd.type_ann.clone(),
                    span: vd.span.clone(),
                };
                self.scopes[scope].symbols.insert(vd.name.clone(), info);
                self.var_decl_to_scope.insert(vd.node_id, scope);

                // Register init expression
                self.resolve_expr(&vd.init, scope);
            }
            Stmt::Destructure(d) => {
                // Register destructured bindings
                match &d.pattern {
                    DestructurePattern::Array(elems, rest) => {
                        for elem in elems {
                            if let DestructureElement::Bind(name) = elem {
                                let info = VarDeclInfo {
                                    name: name.clone(),
                                    node_id: d.node_id,
                                    mutable: d.mutable,
                                    type_ann: None,
                                    span: d.span.clone(),
                                };
                                self.scopes[scope].symbols.insert(name.clone(), info);
                            }
                        }
                        if let Some(name) = rest {
                            let info = VarDeclInfo {
                                name: name.clone(), node_id: d.node_id,
                                mutable: d.mutable, type_ann: None,
                                span: d.span.clone(),
                            };
                            self.scopes[scope].symbols.insert(name.clone(), info);
                        }
                    }
                    DestructurePattern::Object(fields) => {
                        for (_, bind_name) in fields {
                            let info = VarDeclInfo {
                                name: bind_name.clone(), node_id: d.node_id,
                                mutable: d.mutable, type_ann: None,
                                span: d.span.clone(),
                            };
                            self.scopes[scope].symbols.insert(bind_name.clone(), info);
                        }
                    }
                }
                self.resolve_expr(&d.init, scope);
            }
            Stmt::Assign(a) => {
                self.resolve_expr(&a.value, scope);
                if let Some(ref idx) = a.index {
                    self.resolve_expr(idx, scope);
                }
            }
            Stmt::IfStmt(s) => {
                self.resolve_expr(&s.condition, scope);
                let then_scope = self.push_scope("if", Some(scope));
                self.resolve_block(&s.then_body, then_scope);
                for (cond, body) in &s.else_ifs {
                    self.resolve_expr(cond, scope);
                    let elif_scope = self.push_scope("else_if", Some(scope));
                    self.resolve_block(body, elif_scope);
                }
                if let Some(ref else_body) = s.else_body {
                    let else_scope = self.push_scope("else", Some(scope));
                    self.resolve_block(else_body, else_scope);
                }
            }
            Stmt::WhileStmt(s) => {
                self.resolve_expr(&s.condition, scope);
                let loop_scope = self.push_scope("while", Some(scope));
                self.resolve_block(&s.body, loop_scope);
            }
            Stmt::ForInStmt(s) => {
                self.resolve_expr(&s.iterable, scope);
                let for_scope = self.push_scope("for", Some(scope));
                // Register loop variable
                let info = VarDeclInfo {
                    name: s.var_name.clone(), node_id: s.node_id,
                    mutable: false, type_ann: None, // derived from iterable type
                    span: s.span.clone(),
                };
                self.scopes[for_scope].symbols.insert(s.var_name.clone(), info);
                self.var_decl_to_scope.insert(s.node_id, for_scope);
                self.resolve_block(&s.body, for_scope);
            }
            Stmt::ReturnStmt(s) => {
                if let Some(ref v) = s.value {
                    self.resolve_expr(v, scope);
                }
            }
            Stmt::PrintStmt(s) => {
                self.resolve_expr(&s.arg, scope);
            }
            Stmt::ExprStmt(s) => {
                self.resolve_expr(&s.expr, scope);
            }
            _ => {}
        }
    }

    fn resolve_expr(&mut self, expr: &Expr, scope: usize) {
        match expr {
            Expr::VarRef(vr) => {
                let found = self.lookup_var(&vr.name, scope);
                if !found && !self.func_names.contains(&vr.name) {
                    self.errors.add(
                        format!("Undefined variable '{}'", vr.name),
                        Some(vr.span.clone()),
                        None,
                    );
                }
            }
            Expr::BinaryOp(b) => {
                self.resolve_expr(&b.left, scope);
                self.resolve_expr(&b.right, scope);
            }
            Expr::UnaryOp(u) => {
                self.resolve_expr(&u.operand, scope);
            }
            Expr::FuncCall(fc) => {
                self.resolve_expr(&fc.func, scope);
                for arg in &fc.args {
                    self.resolve_expr(arg, scope);
                }
            }
            Expr::MethodCall(mc) => {
                self.resolve_expr(&mc.object, scope);
                for arg in &mc.args {
                    self.resolve_expr(arg, scope);
                }
            }
            Expr::IndexGet(ig) => {
                self.resolve_expr(&ig.object, scope);
                self.resolve_expr(&ig.index, scope);
            }
            Expr::ArrayLiteral(al) => {
                for e in &al.elements {
                    self.resolve_expr(e, scope);
                }
            }
            Expr::MapLiteral(ml) => {
                for (k, v) in &ml.entries {
                    self.resolve_expr(k, scope);
                    self.resolve_expr(v, scope);
                }
            }
            Expr::IfExpr(ie) => {
                self.resolve_expr(&ie.condition, scope);
                self.resolve_expr(&ie.then_expr, scope);
                self.resolve_expr(&ie.else_expr, scope);
            }
            Expr::TemplateString(ts) => {
                for part in &ts.parts {
                    if let TemplatePartValue::Expr(e) = &part.value {
                        self.resolve_expr(e, scope);
                    }
                }
            }
            Expr::ClosureExpr(ce) => {
                let closure_scope = self.push_scope("closure", Some(scope));
                for param in &ce.params {
                    let info = VarDeclInfo {
                        name: param.name.clone(), node_id: param.node_id,
                        mutable: false, type_ann: param.type_ann.clone(),
                        span: param.span.clone(),
                    };
                    self.scopes[closure_scope].symbols.insert(param.name.clone(), info);
                }
                match &*ce.body {
                    ClosureBody::Block(b) => self.resolve_block(b, closure_scope),
                    ClosureBody::Expr(e) => self.resolve_expr(e, closure_scope),
                }
            }
            Expr::MatchExpr(me) => {
                self.resolve_expr(&me.scrutinee, scope);
                for arm in &me.arms {
                    if let Some(ref g) = arm.guard {
                        self.resolve_expr(g, scope);
                    }
                    match &*arm.body {
                        MatchArmBody::Block(b) => self.resolve_block(b, scope),
                        MatchArmBody::Expr(e) => self.resolve_expr(e, scope),
                    }
                }
            }
            Expr::AwaitExpr(ae) => {
                self.resolve_expr(&ae.expr, scope);
            }
            Expr::StructLiteral(sl) => {
                for (_, val) in &sl.fields {
                    self.resolve_expr(val, scope);
                }
            }
            Expr::NullCoalesce(nc) => {
                self.resolve_expr(&nc.left, scope);
                self.resolve_expr(&nc.right, scope);
            }
            Expr::OptionalChain(oc) => {
                self.resolve_expr(&oc.object, scope);
                for link in &oc.chain {
                    if let Some(ref idx) = link.index {
                        self.resolve_expr(idx, scope);
                    }
                    for arg in &link.args {
                        self.resolve_expr(arg, scope);
                    }
                }
            }
            Expr::SpreadExpr(se) => {
                self.resolve_expr(&se.expr, scope);
            }
            _ => {} // Literals need no resolution
        }
    }

    fn lookup_var(&self, name: &str, scope: usize) -> bool {
        let mut sid = Some(scope);
        while let Some(s) = sid {
            if self.scopes[s].symbols.contains_key(name) {
                return true;
            }
            sid = self.scopes[s].parent;
        }
        false
    }
}
