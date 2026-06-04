use crate::ast::*;
use crate::errors::ErrorCollector;
use std::collections::{HashMap, HashSet};

pub fn check(
    program: &Program,
    _scope_info: &super::resolver::ScopeInfo,
    type_map: &HashMap<NodeId, TypeRepr>,
    errors: &mut ErrorCollector,
) {
    let mut checker = RestrictChecker {
        type_map,
        errors,
        params: HashSet::new(),
        for_vars: HashSet::new(),
        in_loop: 0,
        in_for: 0,
        for_iterable: None,
    };

    for func in &program.functions {
        checker.check_function(func);
    }
}

struct RestrictChecker<'a> {
    type_map: &'a HashMap<NodeId, TypeRepr>,
    errors: &'a mut ErrorCollector,
    params: HashSet<String>,
    for_vars: HashSet<String>,
    in_loop: usize,
    in_for: usize,
    for_iterable: Option<String>,
}

impl<'a> RestrictChecker<'a> {
    fn check_function(&mut self, func: &FunctionDefNode) {
        // Register parameters as immutable
        for param in &func.params {
            self.params.insert(param.name.clone());
        }
        // Collect type annotations to check (avoid borrow conflict)
        let mut types_to_check: Vec<TypeRepr> = Vec::new();
        for param in &func.params {
            if let Some(ref ann) = param.type_ann { types_to_check.push(ann.clone()); }
        }
        if let Some(ref ann) = func.return_type { types_to_check.push(ann.clone()); }
        for ty in &types_to_check {
            self.check_type_annotation(ty);
        }
        if let Some(ref body) = func.body {
            self.check_block(body);
        }
        self.params.clear();
    }

    fn check_type_annotation(&mut self, ty: &TypeRepr) {
        match ty {
            TypeRepr::Named { name, type_args, .. } => {
                match name.as_str() {
                    "Map" => {
                        if type_args.len() >= 1 {
                            if type_args[0].base_name() == "f64" {
                                self.errors.add(
                                    "f64 is forbidden as Map key type".to_string(),
                                    None, None,
                                );
                            }
                        }
                        if type_args.len() >= 2 {
                            let val_ty = &type_args[1];
                            self.check_type_annotation(val_ty);
                        }
                    }
                    "Array" => {
                        if !type_args.is_empty() {
                            let elem = &type_args[0];
                            if matches!(elem.base_name(), "Array" | "Map") {
                                self.errors.add(
                                    "Nested containers (Array<Array<...>>) are forbidden".to_string(),
                                    None, None,
                                );
                            }
                            self.check_type_annotation(elem);
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn check_block(&mut self, block: &BlockNode) {
        for stmt in &block.stmts { self.check_stmt(stmt); }
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VarDecl(vd) => {
                if let Some(ref ann) = vd.type_ann {
                    let ann = ann.clone();
                    self.check_type_annotation(&ann);
                }
            }
            Stmt::BreakStmt(_) => {
                if self.in_loop == 0 {
                    self.errors.add("break outside of loop".to_string(), Some(stmt.span().clone()), None);
                }
            }
            Stmt::ContinueStmt(_) => {
                if self.in_loop == 0 {
                    self.errors.add("continue outside of loop".to_string(), Some(stmt.span().clone()), None);
                }
            }
            Stmt::ForInStmt(s) => {
                self.in_loop += 1;
                self.in_for += 1;
                self.for_vars.insert(s.var_name.clone());
                if let Expr::VarRef(ref vr) = s.iterable {
                    self.for_iterable = Some(vr.name.clone());
                }
                self.check_block(&s.body);
                self.for_vars.remove(&s.var_name);
                self.for_iterable = None;
                self.in_for -= 1;
                self.in_loop -= 1;
            }
            Stmt::WhileStmt(s) => {
                self.in_loop += 1;
                self.check_block(&s.body);
                self.in_loop -= 1;
            }
            Stmt::IfStmt(s) => {
                self.check_block(&s.then_body);
                for (_, body) in &s.else_ifs { self.check_block(body); }
                if let Some(ref else_body) = s.else_body { self.check_block(else_body); }
            }
            Stmt::Assign(a) => {
                // Check: parameter reassignment
                if self.params.contains(&a.target) {
                    self.errors.add(
                        format!("Cannot reassign function parameter '{}'", a.target),
                        Some(a.span.clone()), None,
                    );
                }
                // Check: for-in loop variable mutation
                if self.for_vars.contains(&a.target) {
                    self.errors.add(
                        format!("Cannot modify for-in loop variable '{}'", a.target),
                        Some(a.span.clone()), None,
                    );
                }
                // Check: for-in iterable mutation
                if self.in_for > 0 {
                    if let Some(ref fi) = self.for_iterable {
                        if a.target == *fi || a.index.is_some() {
                            self.errors.add(
                                "Cannot modify for-in iterable during iteration".to_string(),
                                Some(a.span.clone()), None,
                            );
                        }
                    }
                }
                // Check: float division in compound assignment
                if a.op == "/=" || a.op == "%=" {
                    if let Some(ty) = self.type_map.get(&a.value.node_id()) {
                        if ty.base_name() == "f64" {
                            self.errors.add(
                                "Float division/modulo is forbidden".to_string(),
                                Some(a.span.clone()), None,
                            );
                        }
                    }
                }
            }
            Stmt::ExprStmt(es) => {
                // Check for method calls that mutate the for-in iterable
                if self.in_for > 0 {
                    if let Expr::MethodCall(ref mc) = es.expr {
                        if let Expr::VarRef(ref vr) = *mc.object {
                            if let Some(ref fi) = self.for_iterable {
                                if vr.name == *fi && (mc.method == "push" || mc.method == "pop" || mc.method == "insert") {
                                    self.errors.add(
                                        "Cannot modify for-in iterable during iteration".to_string(),
                                        Some(es.span.clone()), None,
                                    );
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
