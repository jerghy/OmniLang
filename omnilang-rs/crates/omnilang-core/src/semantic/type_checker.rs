use crate::ast::*;
use crate::errors::ErrorCollector;
use crate::semantic::resolver::ScopeInfo;
use std::collections::HashMap;

/// Built-in method signatures: receiver_type -> {method_name -> (param_types, return_type, is_mut)}
fn builtin_methods() -> HashMap<&'static str, HashMap<&'static str, (&'static [&'static str], &'static str, bool)>> {
    let mut m = HashMap::new();

    // i32 methods
    let mut i32_methods = HashMap::new();
    i32_methods.insert("to_string", (&[][..], "String", false));
    m.insert("i32", i32_methods);

    // f64 methods
    let mut f64_methods = HashMap::new();
    f64_methods.insert("to_string", (&[][..], "String", false));
    m.insert("f64", f64_methods);

    // bool methods
    let mut bool_methods = HashMap::new();
    bool_methods.insert("to_string", (&[][..], "String", false));
    m.insert("bool", bool_methods);

    // String methods
    let mut str_methods = HashMap::new();
    str_methods.insert("to_string", (&[][..], "String", false));
    m.insert("String", str_methods);

    // Array methods
    let mut arr_methods = HashMap::new();
    arr_methods.insert("len", (&[][..], "i32", false));
    arr_methods.insert("is_empty", (&[][..], "bool", false));
    arr_methods.insert("get", (&["i32"][..], "T", false));
    arr_methods.insert("push", (&["T"][..], "void", true));
    arr_methods.insert("pop", (&[][..], "T", true));
    m.insert("Array", arr_methods);

    // Map methods
    let mut map_methods = HashMap::new();
    map_methods.insert("len", (&[][..], "i32", false));
    map_methods.insert("is_empty", (&[][..], "bool", false));
    map_methods.insert("get", (&["K"][..], "V", false));
    map_methods.insert("contains", (&["K"][..], "bool", false));
    map_methods.insert("insert", (&["K", "V"][..], "void", true));
    m.insert("Map", map_methods);

    m
}

pub fn check(
    program: &Program,
    scope_info: &ScopeInfo,
    errors: &mut ErrorCollector,
) -> HashMap<NodeId, TypeRepr> {
    let mut checker = TypeChecker {
        type_map: HashMap::new(),
        scope_info,
        errors,
        var_types: HashMap::new(),
    };

    // Check each function
    for func in &program.functions {
        checker.check_function(func);
    }

    checker.type_map
}

struct TypeChecker<'a> {
    type_map: HashMap<NodeId, TypeRepr>,
    scope_info: &'a ScopeInfo,
    errors: &'a mut ErrorCollector,
    var_types: HashMap<String, TypeRepr>,
}

impl<'a> TypeChecker<'a> {
    fn check_function(&mut self, func: &FunctionDefNode) {
        let return_type = func.return_type.clone().unwrap_or_else(|| TypeRepr::named("void"));

        if let Some(ref body) = func.body {
            self.check_block(body);

            // Verify return type consistency
            if let Some(ref trailing) = body.trailing_expr {
                let trailing_ty = self.infer_expr(trailing);
                let ret_name = return_type.base_name().to_string();
                if ret_name != "void" {
                    if let Some(tt) = trailing_ty {
                        if !self.types_compatible(&tt, &return_type) {
                            self.errors.add(
                                format!("Function '{}' trailing expression type {:?} does not match return type {:?}",
                                    func.name, tt, return_type),
                                Some(trailing.span().clone()),
                                None,
                            );
                        }
                    }
                } else {
                    self.errors.add(
                        format!("Void function '{}' has a trailing expression", func.name),
                        Some(trailing.span().clone()),
                        None,
                    );
                }
            }
        }
    }

    fn check_block(&mut self, block: &BlockNode) {
        for stmt in &block.stmts {
            self.check_stmt(stmt);
        }
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VarDecl(vd) => {
                let init_ty = self.infer_expr(&vd.init);
                // Register variable type
                let var_type = vd.type_ann.clone().or_else(|| init_ty.clone());
                if let Some(ref vt) = var_type {
                    self.var_types.insert(vd.name.clone(), vt.clone());
                }
                if let (Some(ann), Some(init)) = (&vd.type_ann, &init_ty) {
                    if !self.types_compatible(init, ann) {
                        self.errors.add(
                            format!("Type mismatch: variable '{}' declared as {:?} but initialized with {:?}",
                                vd.name, ann, init),
                            Some(vd.span.clone()),
                            None,
                        );
                    }
                }
                if let Some(ann) = &vd.type_ann {
                    self.type_map.insert(vd.node_id, ann.clone());
                } else if let Some(init) = init_ty {
                    self.type_map.insert(vd.node_id, init);
                }
            }
            Stmt::Destructure(d) => {
                let _init_ty = self.infer_expr(&d.init);
                // For now, just validate the init expression is valid
            }
            Stmt::Assign(a) => {
                let val_ty = self.infer_expr(&a.value);
                self.type_map.insert(a.node_id, val_ty.unwrap_or_else(|| TypeRepr::named("void")));
            }
            Stmt::IfStmt(s) => {
                let cond_ty = self.infer_expr(&s.condition);
                if let Some(ct) = cond_ty {
                    if ct.base_name() != "bool" {
                        self.errors.add(
                            "If condition must be bool".to_string(),
                            Some(s.condition.span().clone()),
                            None,
                        );
                    }
                }
                self.check_block(&s.then_body);
                for (cond, body) in &s.else_ifs {
                    let _ = self.infer_expr(cond);
                    self.check_block(body);
                }
                if let Some(ref else_body) = s.else_body {
                    self.check_block(else_body);
                }
            }
            Stmt::WhileStmt(s) => {
                let _ = self.infer_expr(&s.condition);
                self.check_block(&s.body);
            }
            Stmt::ForInStmt(s) => {
                let iter_ty = self.infer_expr(&s.iterable);
                if let Some(ty) = iter_ty {
                    if let TypeRepr::Named { name, type_args, .. } = &ty {
                        if name == "Array" && !type_args.is_empty() {
                            let elem_ty = type_args[0].clone();
                            self.type_map.insert(s.node_id, elem_ty.clone());
                            self.var_types.insert(s.var_name.clone(), elem_ty);
                        }
                    }
                }
                self.check_block(&s.body);
            }
            Stmt::ReturnStmt(s) => {
                if let Some(ref v) = s.value {
                    let _ = self.infer_expr(v);
                }
            }
            Stmt::PrintStmt(s) => {
                let arg_ty = self.infer_expr(&s.arg);
                if let Some(ty) = arg_ty {
                    if ty.base_name() != "String" {
                        self.errors.add(
                            "print/println argument must be String".to_string(),
                            Some(s.arg.span().clone()),
                            None,
                        );
                    }
                }
            }
            Stmt::ExprStmt(s) => {
                let _ = self.infer_expr(&s.expr);
            }
            _ => {}
        }
    }

    /// Infer the type of an expression. Returns None on error.
    pub fn infer_expr(&mut self, expr: &Expr) -> Option<TypeRepr> {
        let ty = match expr {
            Expr::Literal(lit) => {
                Some(TypeRepr::named(&lit.kind))
            }
            Expr::VarRef(vr) => {
                self.var_types.get(&vr.name).cloned()
            }
            Expr::BinaryOp(b) => {
                let left_ty = self.infer_expr(&b.left);
                let right_ty = self.infer_expr(&b.right);

                match b.op {
                    BinaryOpKind::Add => {
                        match (&left_ty, &right_ty) {
                            (Some(l), Some(r)) if l.base_name() == "String" && r.base_name() == "String" => {
                                Some(TypeRepr::named("String"))
                            }
                            (Some(l), Some(r)) if l.base_name() == r.base_name() => {
                                Some(l.clone())
                            }
                            // If one side is unknown, infer from the other
                            (Some(l), None) => Some(l.clone()),
                            (None, Some(r)) => Some(r.clone()),
                            (None, None) => None,
                            (Some(l), Some(_r)) => {
                                // Different types but might be compatible — be lenient
                                Some(l.clone())
                            }
                        }
                    }
                    BinaryOpKind::Div | BinaryOpKind::Mod => {
                        // Float division is forbidden
                        if let Some(ref lt) = left_ty {
                            if lt.base_name() == "f64" {
                                self.errors.add(
                                    "Float division/modulo is forbidden in OmniLang".to_string(),
                                    Some(b.span.clone()),
                                    None,
                                );
                            }
                        }
                        left_ty
                    }
                    BinaryOpKind::Eq | BinaryOpKind::Ne
                    | BinaryOpKind::Lt | BinaryOpKind::Gt
                    | BinaryOpKind::Le | BinaryOpKind::Ge => {
                        Some(TypeRepr::named("bool"))
                    }
                    BinaryOpKind::And | BinaryOpKind::Or => {
                        if let Some(ref lt) = left_ty {
                            if lt.base_name() != "bool" {
                                self.errors.add(
                                    "Logical operators require bool operands".to_string(),
                                    Some(b.span.clone()),
                                    None,
                                );
                            }
                        }
                        Some(TypeRepr::named("bool"))
                    }
                    _ => left_ty,
                }
            }
            Expr::UnaryOp(u) => {
                let operand_ty = self.infer_expr(&u.operand);
                match u.op {
                    UnaryOpKind::Not => {
                        if let Some(ref ot) = operand_ty {
                            if ot.base_name() != "bool" {
                                self.errors.add(
                                    "! operator requires bool operand".to_string(),
                                    Some(u.span.clone()),
                                    None,
                                );
                            }
                        }
                        Some(TypeRepr::named("bool"))
                    }
                    UnaryOpKind::Neg => operand_ty,
                }
            }
            Expr::FuncCall(fc) => {
                // Infer return type from function name
                if let Expr::VarRef(vr) = &*fc.func {
                    // Check built-in functions
                    match vr.name.as_str() {
                        "all" => {
                            // Returns Array of same type
                            if !fc.args.is_empty() {
                                self.infer_expr(&fc.args[0])
                            } else {
                                None
                            }
                        }
                        "race" => {
                            if !fc.args.is_empty() {
                                self.infer_expr(&fc.args[0])
                            } else {
                                None
                            }
                        }
                        _ => {
                            // User function — can't fully infer without function map
                            // Return a placeholder for now
                            None
                        }
                    }
                } else {
                    None
                }
            }
            Expr::MethodCall(mc) => {
                let obj_ty = self.infer_expr(&mc.object);
                let result = if let Some(ty) = obj_ty {
                    let base = ty.base_name();
                    let methods = builtin_methods();
                    if let Some(type_methods) = methods.get(base) {
                        if let Some((param_types, ret_type, _is_mut)) = type_methods.get(mc.method.as_str()) {
                            if mc.args.len() != param_types.len() {
                                self.errors.add(
                                    format!("Method '{}' expects {} arguments, got {}",
                                        mc.method, param_types.len(), mc.args.len()),
                                    Some(mc.span.clone()),
                                    None,
                                );
                            }
                            let ret = if *ret_type == "T" || *ret_type == "V" {
                                if let TypeRepr::Named { type_args, .. } = &ty {
                                    if !type_args.is_empty() && *ret_type == "T" {
                                        type_args[0].clone()
                                    } else if type_args.len() > 1 && *ret_type == "V" {
                                        type_args[1].clone()
                                    } else {
                                        TypeRepr::named(ret_type)
                                    }
                                } else {
                                    TypeRepr::named(ret_type)
                                }
                            } else {
                                TypeRepr::named(ret_type)
                            };
                            Some(ret)
                        } else { None }
                    } else { None }
                } else { None };
                if let Some(ref t) = result {
                    self.type_map.insert(mc.node_id, t.clone());
                }
                result
            }
            Expr::IndexGet(ig) => {
                let obj_ty = self.infer_expr(&ig.object);
                if let Some(ty) = obj_ty {
                    match &ty {
                        TypeRepr::Named { name, type_args, .. } if name == "Array" && !type_args.is_empty() => {
                            Some(type_args[0].clone())
                        }
                        TypeRepr::Named { name, type_args, .. } if name == "Map" && type_args.len() > 1 => {
                            Some(type_args[1].clone())
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            }
            Expr::ArrayLiteral(al) => {
                if let Some(first) = al.elements.first() {
                    self.infer_expr(first).map(|elem_ty| {
                        TypeRepr::named_with_args("Array", vec![elem_ty])
                    })
                } else {
                    Some(TypeRepr::named_with_args("Array", vec![TypeRepr::named("i32")]))
                }
            }
            Expr::MapLiteral(ml) => {
                if let Some((k, v)) = ml.entries.first() {
                    let kt = self.infer_expr(k).unwrap_or_else(|| TypeRepr::named("String"));
                    let vt = self.infer_expr(v).unwrap_or_else(|| TypeRepr::named("i32"));
                    Some(TypeRepr::named_with_args("Map", vec![kt, vt]))
                } else {
                    Some(TypeRepr::named_with_args("Map", vec![TypeRepr::named("String"), TypeRepr::named("i32")]))
                }
            }
            Expr::IfExpr(ie) => {
                let then_ty = self.infer_expr(&ie.then_expr);
                let else_ty = self.infer_expr(&ie.else_expr);
                // Both branches should have compatible types
                if let (Some(ref tt), Some(ref et)) = (&then_ty, &else_ty) {
                    if tt.base_name() != et.base_name() {
                        self.errors.add(
                            "If expression branches have incompatible types".to_string(),
                            Some(ie.span.clone()),
                            None,
                        );
                    }
                }
                then_ty
            }
            Expr::ClosureExpr(ce) => {
                // Try to infer return type from body
                match &*ce.body {
                    ClosureBody::Expr(e) => self.infer_expr(e),
                    ClosureBody::Block(b) => {
                        b.trailing_expr.as_ref().and_then(|e| self.infer_expr(e))
                    }
                }
            }
            Expr::MatchExpr(me) => {
                // Infer from first arm
                me.arms.first().and_then(|arm| {
                    match &*arm.body {
                        MatchArmBody::Expr(e) => self.infer_expr(e),
                        MatchArmBody::Block(b) => {
                            b.trailing_expr.as_ref().and_then(|e| self.infer_expr(e))
                        }
                    }
                })
            }
            Expr::TemplateString(_ts) => Some(TypeRepr::named("String")),
            Expr::StructLiteral(sl) => Some(TypeRepr::named(&sl.type_name)),
            Expr::AwaitExpr(ae) => self.infer_expr(&ae.expr),
            Expr::NullCoalesce(nc) => {
                let lt = self.infer_expr(&nc.left);
                lt // unwrap the optional
            }
            Expr::OptionalChain(oc) => {
                // Optional chain returns Optional<T>
                let _obj_ty = self.infer_expr(&oc.object);
                None // Can't infer without more context
            }
            _ => None,
        };

        if let Some(ref t) = ty {
            self.type_map.insert(expr.node_id(), t.clone());
        }
        ty
    }

    fn lookup_var_type(&self, _name: &str) -> Option<TypeRepr> {
        // Walk the type_map for known variables
        // Simplified — full implementation would use scope_info
        None
    }

    fn types_compatible(&self, a: &TypeRepr, b: &TypeRepr) -> bool {
        match (a, b) {
            (TypeRepr::Named { name: na, type_args: ta, .. },
             TypeRepr::Named { name: nb, type_args: tb, .. }) => {
                if na != nb { return false; }
                if ta.is_empty() || tb.is_empty() { return true; } // incomplete inference
                ta.len() == tb.len() && ta.iter().zip(tb).all(|(a, b)| self.types_compatible(a, b))
            }
            (TypeRepr::Union { types: ua, .. }, _) => {
                ua.iter().any(|t| self.types_compatible(t, b))
            }
            (_, TypeRepr::Union { types: ub, .. }) => {
                ub.iter().any(|t| self.types_compatible(a, t))
            }
            _ => std::mem::discriminant(a) == std::mem::discriminant(b),
        }
    }
}
