use crate::ast::*;
use crate::codegen::base::CodegenContext;
use std::collections::HashMap;
use std::cell::Cell;

pub struct DartGenerator {
    pub ctx: CodegenContext,
    pub has_output: Cell<bool>,
}

impl DartGenerator {
    pub fn new(type_map: HashMap<NodeId, TypeRepr>, rename_map: Vec<HashMap<String, String>>) -> Self {
        Self { ctx: CodegenContext::new(type_map, rename_map), has_output: Cell::new(false) }
    }

    pub fn generate(mut self, program: &Program) -> String {
        for s in &program.structs { self.gen_struct(s); }
        for f in &program.functions { self.gen_function(f); }
        let mut result = String::new();
        if self.has_output.get() { result.push_str("import 'dart:io';\n\n"); }
        result.push_str(&self.ctx.result());
        result
    }

    fn gen_struct(&mut self, s: &StructDefNode) {
        let fields: Vec<String> = s.fields.iter().map(|f| format!("final {} {};", self.dart_type(&f.type_ann), f.name)).collect();
        self.ctx.emit(&format!("class {} {{", s.name));
        self.ctx.indent();
        for f in &fields { self.ctx.emit(f); }
        let params: Vec<String> = s.fields.iter().map(|f| format!("required this.{}", f.name)).collect();
        self.ctx.emit(&format!("const {}({{{}}});", s.name, params.join(", ")));
        for method in &s.methods {
            self.gen_method(method);
        }
        self.ctx.dedent();
        self.ctx.emit("}");
        self.ctx.emit("");
    }

    fn gen_method(&mut self, func: &FunctionDefNode) {
        let params: Vec<String> = func.params.iter().map(|p| format!("{} {}", self.dart_type(p.type_ann.as_ref().unwrap_or(&TypeRepr::named("void"))), p.name)).collect();
        let ret = func.return_type.as_ref().map_or("void".to_string(), |rt| self.dart_type(rt));
        self.ctx.emit(&format!("{} {}({}) {{", ret, func.name, params.join(", ")));
        self.ctx.indent();
        if let Some(ref body) = func.body { self.gen_block(body); }
        self.ctx.dedent();
        self.ctx.emit("}");
    }

    fn gen_function(&mut self, func: &FunctionDefNode) {
        let ret = func.return_type.as_ref().map_or("void".to_string(), |rt| self.dart_type(rt));
        let ret_str = if func.is_async { format!("Future<{}>", ret) } else { ret };
        let params: Vec<String> = func.params.iter().map(|p| format!("{} {}", self.dart_type(p.type_ann.as_ref().unwrap_or(&TypeRepr::named("void"))), p.name)).collect();
        let async_kw = if func.is_async { " async" } else { "" };
        self.ctx.emit(&format!("{} {}({}){} {{", ret_str, func.name, params.join(", "), async_kw));
        self.ctx.indent();
        if let Some(ref body) = func.body { self.gen_block(body); }
        self.ctx.dedent();
        self.ctx.emit("}");
        self.ctx.emit("");
    }

    fn gen_block(&mut self, block: &BlockNode) {
        self.ctx.enter_scope();
        for stmt in &block.stmts { self.gen_stmt(stmt); }
        if let Some(ref trailing) = block.trailing_expr {
            let val = self.gen_expr(trailing);
            self.ctx.emit(&format!("return {};", val));
        }
        self.ctx.leave_scope();
    }

    fn gen_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VarDecl(vd) => {
                let kw = if vd.mutable { "var" } else { "final" };
                let init = self.gen_expr(&vd.init);
                let mut init_val = init;
                if let Expr::VarRef(_) = &vd.init {
                    if let Some(ref ann) = vd.type_ann {
                        match ann.base_name() {
                            "Array" => init_val = format!("List.of({})", init_val),
                            "Map" => init_val = format!("Map.of({})", init_val),
                            _ => {}
                        }
                    }
                }
                if let Some(ref ann) = vd.type_ann {
                    // Dart: 'var' + type annotation is invalid. Use type directly for mutable, 'final Type' for immutable.
                    let decl = if vd.mutable {
                        format!("{} {} = {}", self.dart_type(ann), self.ctx.rename(&vd.name), init_val)
                    } else {
                        format!("final {} {} = {}", self.dart_type(ann), self.ctx.rename(&vd.name), init_val)
                    };
                    self.ctx.emit(&format!("{};", decl));
                } else {
                    self.ctx.emit(&format!("{} {} = {};", kw, self.ctx.rename(&vd.name), init_val));
                }
            }
            Stmt::Assign(a) => {
                let val = self.gen_expr(&a.value);
                if let Some(ref idx) = a.index {
                    let idx_val = self.gen_expr(idx);
                    self.ctx.emit(&format!("{}[{}] = {};", a.target, idx_val, val));
                } else {
                    match a.op.as_str() {
                        "/=" => self.ctx.emit(&format!("{} ~/= {};", a.target, val)),
                        _ => self.ctx.emit(&format!("{} {}= {};", a.target, &a.op[..a.op.len()-1], val)),
                    }
                }
            }
            Stmt::IfStmt(s) => {
                self.ctx.emit(&format!("if ({}) {{", self.gen_expr(&s.condition)));
                self.ctx.indent(); self.gen_block(&s.then_body); self.ctx.dedent();
                for (cond, body) in &s.else_ifs {
                    self.ctx.emit(&format!("}} else if ({}) {{", self.gen_expr(cond)));
                    self.ctx.indent(); self.gen_block(body); self.ctx.dedent();
                }
                if s.else_body.is_some() {
                    self.ctx.emit("} else {");
                    self.ctx.indent(); self.gen_block(s.else_body.as_ref().unwrap()); self.ctx.dedent();
                }
                self.ctx.emit("}");
            }
            Stmt::WhileStmt(s) => {
                self.ctx.emit(&format!("while ({}) {{", self.gen_expr(&s.condition)));
                self.ctx.indent(); self.gen_block(&s.body); self.ctx.dedent();
                self.ctx.emit("}");
            }
            Stmt::ForInStmt(s) => {
                self.ctx.emit(&format!("for (final {} in {}) {{", self.ctx.rename(&s.var_name), self.gen_expr(&s.iterable)));
                self.ctx.indent(); self.gen_block(&s.body); self.ctx.dedent();
                self.ctx.emit("}");
            }
            Stmt::BreakStmt(_) => self.ctx.emit("break;"),
            Stmt::ContinueStmt(_) => self.ctx.emit("continue;"),
            Stmt::ReturnStmt(s) => {
                if let Some(ref v) = s.value { self.ctx.emit(&format!("return {};", self.gen_expr(v))); }
                else { self.ctx.emit("return;"); }
            }
            Stmt::PrintStmt(s) => {
                self.has_output.set(true);
                let arg = self.gen_expr(&s.arg);
                if s.newline { self.ctx.emit(&format!("stdout.writeln({});", arg)); }
                else { self.ctx.emit(&format!("stdout.write({});", arg)); }
            }
            Stmt::ExprStmt(s) => { self.ctx.emit(&format!("{};", self.gen_expr(&s.expr))); }
            _ => {}
        }
    }

    pub fn gen_expr(&self, expr: &Expr) -> String {
        match expr {
            Expr::Literal(lit) => match &lit.value {
                LiteralValue::Int(v) => v.to_string(),
                LiteralValue::Float(v) => v.to_string(),
                LiteralValue::Str(s) => format!("'{}'", s.escape_debug()),
                LiteralValue::Bool(true) => "true".to_string(),
                LiteralValue::Bool(false) => "false".to_string(),
            },
            Expr::VarRef(vr) => self.ctx.rename(&vr.name),
            Expr::BinaryOp(b) => {
                let left = self.gen_expr(&b.left);
                let right = self.gen_expr(&b.right);
                match b.op {
                    BinaryOpKind::Div => format!("({} ~/ {})", left, right),
                    _ => format!("({} {} {})", left, b.op.as_str(), right),
                }
            }
            Expr::UnaryOp(u) => {
                let op = match u.op { UnaryOpKind::Neg => "-", UnaryOpKind::Not => "!" };
                format!("({}{})", op, self.gen_expr(&u.operand))
            }
            Expr::FuncCall(fc) => {
                let name = self.gen_expr(&fc.func);
                let args: Vec<String> = fc.args.iter().map(|a| self.gen_expr(a)).collect();
                match name.as_str() {
                    "all" => format!("Future.wait([{}])", args.join(", ")),
                    "race" => format!("Future.any([{}])", args.join(", ")),
                    _ => format!("{}({})", name, args.join(", ")),
                }
            }
            Expr::MethodCall(mc) => {
                let obj = self.gen_expr(&mc.object);
                let args: Vec<String> = mc.args.iter().map(|a| self.gen_expr(a)).collect();
                match mc.method.as_str() {
                    "len" => format!("{}.length", obj),
                    "is_empty" => format!("{}.isEmpty", obj),
                    "get" => format!("({}[{}] ?? (throw Exception('Key not found')))", obj, args.join(", ")),
                    "push" => format!("{}.add({})", obj, args.join(", ")),
                    "pop" => format!("{}.removeLast()", obj),
                    "contains" => format!("{}.containsKey({})", obj, args.join(", ")),
                    "insert" => format!("{}[{}] = {}", obj, args.first().unwrap_or(&String::new()), args.get(1).unwrap_or(&String::new())),
                    "to_string" => format!("{}.toString()", obj),
                    _ => format!("{}.{}({})", obj, mc.method, args.join(", ")),
                }
            }
            Expr::IndexGet(ig) => format!("{}[{}]", self.gen_expr(&ig.object), self.gen_expr(&ig.index)),
            Expr::ArrayLiteral(al) => {
                let elems: Vec<String> = al.elements.iter().map(|e| self.gen_expr(e)).collect();
                format!("[{}]", elems.join(", "))
            }
            Expr::MapLiteral(ml) => {
                let entries: Vec<String> = ml.entries.iter()
                    .map(|(k, v)| format!("{}: {}", self.gen_expr(k), self.gen_expr(v)))
                    .collect();
                format!("{{{}}}", entries.join(", "))
            }
            Expr::IfExpr(ie) => format!("({} ? {} : {})", self.gen_expr(&ie.condition), self.gen_expr(&ie.then_expr), self.gen_expr(&ie.else_expr)),
            Expr::TemplateString(ts) => {
                let mut s = String::from("'");
                for part in &ts.parts {
                    match &part.value {
                        TemplatePartValue::Literal(l) => {
                            s.push_str(&l.replace('$', "\\$"));
                        }
                        TemplatePartValue::Expr(e) => {
                            s.push_str("${");
                            s.push_str(&self.gen_expr(e));
                            s.push('}');
                        }
                    }
                }
                s.push('\'');
                s
            }
            Expr::NullCoalesce(nc) => format!("({} ?? {})", self.gen_expr(&nc.left), self.gen_expr(&nc.right)),
            Expr::StructLiteral(sl) => {
                let fields: Vec<String> = sl.fields.iter().map(|(k, v)| format!("{}: {}", k, self.gen_expr(v))).collect();
                format!("{}({})", sl.type_name, fields.join(", "))
            }
            Expr::AwaitExpr(ae) => format!("await {}", self.gen_expr(&ae.expr)),
            _ => "<expr>".to_string(),
        }
    }

    fn dart_type(&self, ty: &TypeRepr) -> String {
        match ty {
            TypeRepr::Named { name, type_args, .. } => match name.as_str() {
                "i32" => "int".into(), "f64" => "double".into(), "bool" => "bool".into(),
                "String" => "String".into(), "void" => "void".into(),
                "Array" => if !type_args.is_empty() { format!("List<{}>", self.dart_type(&type_args[0])) } else { "List".into() },
                "Map" => if type_args.len() > 1 { format!("Map<{}, {}>", self.dart_type(&type_args[0]), self.dart_type(&type_args[1])) } else { "Map".into() },
                _ => name.clone(),
            },
            _ => "dynamic".into(),
        }
    }
}
