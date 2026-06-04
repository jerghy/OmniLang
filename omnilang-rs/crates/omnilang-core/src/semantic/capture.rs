use crate::ast::*;
use std::collections::HashMap;

/// Analyze closure captures for Rust Cell/RefCell wrapping.
#[derive(Debug, Clone)]
pub struct CaptureInfo {
    pub name: String,
    pub kind: CaptureKind,
    pub var_type: Option<TypeRepr>,
    pub escaped: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureKind {
    Reference,
    Mutable,
    Move,
}

/// Analyze all closures in a program and return capture information.
pub fn analyze_captures(program: &Program) -> HashMap<NodeId, Vec<CaptureInfo>> {
    let mut analyzer = CaptureAnalyzer::new();
    for func in &program.functions {
        if let Some(ref body) = func.body {
            analyzer.analyze_block(body);
        }
    }
    analyzer.captures
}

struct CaptureAnalyzer {
    captures: HashMap<NodeId, Vec<CaptureInfo>>,
    scope_stack: Vec<Vec<String>>,
    assignments: Vec<Vec<String>>,
    in_closure: usize,
    current_closure_captures: HashMap<NodeId, Vec<String>>,
}

impl CaptureAnalyzer {
    fn new() -> Self {
        Self {
            captures: HashMap::new(),
            scope_stack: vec![Vec::new()],
            assignments: vec![Vec::new()],
            in_closure: 0,
            current_closure_captures: HashMap::new(),
        }
    }

    fn analyze_block(&mut self, block: &BlockNode) {
        for stmt in &block.stmts {
            self.analyze_stmt(stmt);
        }
    }

    fn analyze_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VarDecl(vd) => {
                self.scope_stack.last_mut().unwrap().push(vd.name.clone());
                self.analyze_expr(&vd.init);
            }
            Stmt::Assign(a) => {
                self.assignments.last_mut().unwrap().push(a.target.clone());
                self.analyze_expr(&a.value);
            }
            Stmt::IfStmt(s) => {
                self.analyze_expr(&s.condition);
                self.analyze_block(&s.then_body);
                for (cond, body) in &s.else_ifs {
                    self.analyze_expr(cond);
                    self.analyze_block(body);
                }
                if let Some(ref else_body) = s.else_body {
                    self.analyze_block(else_body);
                }
            }
            Stmt::WhileStmt(s) => {
                self.analyze_expr(&s.condition);
                self.analyze_block(&s.body);
            }
            Stmt::ForInStmt(s) => {
                self.analyze_expr(&s.iterable);
                self.scope_stack.last_mut().unwrap().push(s.var_name.clone());
                self.analyze_block(&s.body);
                self.scope_stack.last_mut().unwrap().pop();
            }
            Stmt::ReturnStmt(s) => {
                if let Some(ref v) = s.value {
                    self.analyze_expr(v);
                }
            }
            Stmt::PrintStmt(s) => {
                self.analyze_expr(&s.arg);
            }
            Stmt::ExprStmt(s) => {
                self.analyze_expr(&s.expr);
            }
            _ => {}
        }
    }

    fn analyze_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::ClosureExpr(ce) => {
                self.in_closure += 1;
                self.scope_stack.push(Vec::new());
                self.assignments.push(Vec::new());

                // Register params in closure scope
                for param in &ce.params {
                    self.scope_stack.last_mut().unwrap().push(param.name.clone());
                }

                // Analyze closure body
                match &*ce.body {
                    ClosureBody::Block(b) => self.analyze_block(b),
                    ClosureBody::Expr(e) => self.analyze_expr(e),
                }

                // Collect free variables (captured from outer scopes)
                let mut captures = Vec::new();
                if let Some(closure_vars) = self.scope_stack.last() {
                    for var_name in closure_vars {
                        // Check if this name appears in an outer scope
                        for (_level, scope) in self.scope_stack.iter().rev().skip(1).enumerate() {
                            if scope.contains(var_name) {
                                let is_mut = self.assignments.iter().any(|a| a.contains(var_name));
                                captures.push(CaptureInfo {
                                    name: var_name.clone(),
                                    kind: if is_mut { CaptureKind::Mutable } else { CaptureKind::Reference },
                                    var_type: None,
                                    escaped: true, // Conservative: assume escape
                                });
                                break;
                            }
                        }
                    }
                }

                if !captures.is_empty() {
                    self.captures.insert(ce.node_id, captures);
                }

                self.scope_stack.pop();
                self.assignments.pop();
                self.in_closure -= 1;
            }
            Expr::BinaryOp(b) => {
                self.analyze_expr(&b.left);
                self.analyze_expr(&b.right);
            }
            Expr::FuncCall(fc) => {
                self.analyze_expr(&fc.func);
                for arg in &fc.args {
                    self.analyze_expr(arg);
                }
            }
            Expr::MethodCall(mc) => {
                self.analyze_expr(&mc.object);
                for arg in &mc.args {
                    self.analyze_expr(arg);
                }
            }
            Expr::IfExpr(ie) => {
                self.analyze_expr(&ie.condition);
                self.analyze_expr(&ie.then_expr);
                self.analyze_expr(&ie.else_expr);
            }
            Expr::MatchExpr(me) => {
                self.analyze_expr(&me.scrutinee);
                for arm in &me.arms {
                    match &*arm.body {
                        MatchArmBody::Block(b) => self.analyze_block(b),
                        MatchArmBody::Expr(e) => self.analyze_expr(e),
                    }
                }
            }
            Expr::VarRef(vr) => {
                // Track variable usage in current scope
                if self.in_closure > 0 {
                    // Check if variable is from an outer scope
                    let mut is_outer = true;
                    if let Some(current) = self.scope_stack.last() {
                        if current.contains(&vr.name) {
                            is_outer = false;
                        }
                    }
                    if is_outer {
                        if let Some(last) = self.scope_stack.last_mut() {
                            if !last.contains(&vr.name) {
                                last.push(vr.name.clone());
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
