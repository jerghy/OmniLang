use crate::ast::*;
use crate::codegen::base::CodegenContext;
use std::collections::HashMap;
use std::cell::Cell;

pub struct PythonGenerator {
    pub ctx: CodegenContext,
    pub needs_idiv: Cell<bool>,
    pub needs_irem: Cell<bool>,
}

impl PythonGenerator {
    pub fn new(type_map: HashMap<NodeId, TypeRepr>, rename_map: Vec<HashMap<String, String>>) -> Self {
        Self { ctx: CodegenContext::new(type_map, rename_map), needs_idiv: Cell::new(false), needs_irem: Cell::new(false) }
    }

    pub fn generate(mut self, program: &Program) -> String {
        for s in &program.structs { self.gen_struct(s); }
        for tu in &program.tagged_unions { self.gen_tagged_union(tu); }
        for f in &program.functions { self.gen_function(f); }

        let mut helpers = String::new();
        if self.needs_idiv.get() {
            helpers.push_str("def _idiv(a, b):\n    q = abs(a) // abs(b)\n    return q if (a < 0) == (b < 0) else -q\n\n");
        }
        if self.needs_irem.get() {
            if !self.needs_idiv.get() {
                helpers.push_str("def _idiv(a, b):\n    q = abs(a) // abs(b)\n    return q if (a < 0) == (b < 0) else -q\n\n");
            }
            helpers.push_str("def _irem(a, b):\n    return a - _idiv(a, b) * b\n\n");
        }

        let has_async = program.functions.iter().any(|f| f.name == "main" && f.is_async);
        let entry = if has_async {
            "\nif __name__ == \"__main__\":\n    import asyncio\n    asyncio.run(main())\n"
        } else {
            "\nif __name__ == \"__main__\":\n    main()\n"
        };
        helpers + &self.ctx.result() + entry
    }

    fn gen_struct(&mut self, s: &StructDefNode) {
        self.ctx.emit_raw("@dataclass\n");
        self.ctx.emit(&format!("class {}:", s.name));
        self.ctx.indent();
        for f in &s.fields { self.ctx.emit(&format!("{}: {}", f.name, self.py_type(&f.type_ann))); }
        self.ctx.dedent();
        self.ctx.emit("");
    }

    fn gen_tagged_union(&mut self, tu: &TaggedUnionDefNode) {
        for var in &tu.variants {
            self.ctx.emit_raw("@dataclass\n");
            self.ctx.emit(&format!("class {}:", var.name));
            self.ctx.indent();
            for f in &var.fields { self.ctx.emit(&format!("{}: {}", f.name, self.py_type(&f.type_ann))); }
            self.ctx.dedent();
            self.ctx.emit("");
        }
    }

    fn gen_function(&mut self, func: &FunctionDefNode) {
        let prefix = if func.is_async { "async " } else { "" };
        let params = func.params.iter().map(|p| p.name.clone()).collect::<Vec<_>>().join(", ");
        let ret = func.return_type.as_ref().map_or(String::new(), |rt| {
            if rt.base_name() == "void" { String::new() } else { format!(" -> {}", self.py_type(rt)) }
        });
        self.ctx.emit(&format!("{}def {}({}){}:", prefix, func.name, params, ret));
        self.ctx.indent();
        if let Some(ref body) = func.body { self.gen_block(body); } else { self.ctx.emit("pass"); }
        self.ctx.dedent();
        self.ctx.emit("");
    }

    fn gen_block(&mut self, block: &BlockNode) {
        self.ctx.enter_scope();
        for stmt in &block.stmts { self.gen_stmt(stmt); }
        if let Some(ref trailing) = block.trailing_expr {
            let val = self.gen_expr(trailing);
            self.ctx.emit(&format!("return {}", val));
        }
        self.ctx.leave_scope();
    }

    fn gen_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VarDecl(vd) => {
                let init_val = self.gen_expr(&vd.init);
                let final_init = self.wrap_clone(&vd.init, vd.type_ann.as_ref(), init_val);
                self.ctx.emit(&format!("{} = {}", self.ctx.rename(&vd.name), final_init));
            }
            Stmt::Assign(a) => {
                let val = self.gen_expr(&a.value);
                if let Some(ref idx) = a.index {
                    let idx_val = self.gen_expr(idx);
                    self.ctx.emit(&format!("{}[{}] = {}", a.target, idx_val, val));
                } else {
                    match a.op.as_str() {
                        "/=" => { self.needs_idiv.set(true); self.ctx.emit(&format!("{} = _idiv({}, {})", a.target, a.target, val)); }
                        "%=" => { self.needs_irem.set(true); self.ctx.emit(&format!("{} = _irem({}, {})", a.target, a.target, val)); }
                        _ => self.ctx.emit(&format!("{} {}= {}", a.target, &a.op[..a.op.len()-1], val)),
                    }
                }
            }
            Stmt::IfStmt(s) => {
                let cond = self.gen_expr(&s.condition);
                self.ctx.emit(&format!("if {}:", cond));
                self.ctx.indent(); self.ctx.enter_scope(); self.gen_block(&s.then_body); self.ctx.leave_scope(); self.ctx.dedent();
                for (ce, body) in &s.else_ifs {
                    let c = self.gen_expr(ce);
                    self.ctx.emit(&format!("elif {}:", c));
                    self.ctx.indent(); self.ctx.enter_scope(); self.gen_block(body); self.ctx.leave_scope(); self.ctx.dedent();
                }
                if let Some(ref eb) = s.else_body {
                    self.ctx.emit("else:");
                    self.ctx.indent(); self.ctx.enter_scope(); self.gen_block(eb); self.ctx.leave_scope(); self.ctx.dedent();
                }
            }
            Stmt::WhileStmt(s) => {
                let cond = self.gen_expr(&s.condition);
                self.ctx.emit(&format!("while {}:", cond));
                self.ctx.indent(); self.ctx.enter_scope(); self.gen_block(&s.body); self.ctx.leave_scope(); self.ctx.dedent();
            }
            Stmt::ForInStmt(s) => {
                let iter = self.gen_expr(&s.iterable);
                self.ctx.emit(&format!("for {} in {}:", self.ctx.rename(&s.var_name), iter));
                self.ctx.indent(); self.ctx.enter_scope(); self.gen_block(&s.body); self.ctx.leave_scope(); self.ctx.dedent();
            }
            Stmt::BreakStmt(_) => self.ctx.emit("break"),
            Stmt::ContinueStmt(_) => self.ctx.emit("continue"),
            Stmt::ReturnStmt(s) => {
                if let Some(ref v) = s.value {
                    let val = self.gen_expr(v);
                    self.ctx.emit(&format!("return {}", val));
                } else { self.ctx.emit("return"); }
            }
            Stmt::PrintStmt(s) => {
                let arg = self.gen_expr(&s.arg);
                let line = if s.newline { format!("print({})", arg) } else { format!("print({}, end='')", arg) };
                self.ctx.emit(&line);
            }
            Stmt::ExprStmt(s) => { let val = self.gen_expr(&s.expr); self.ctx.emit(&val); }
            _ => {}
        }
    }

    fn wrap_clone(&self, init: &Expr, ann: Option<&TypeRepr>, val: String) -> String {
        if let Expr::VarRef(_) = init {
            if let Some(a) = ann {
                match a.base_name() { "Array" => return format!("list({})", val), "Map" => return format!("dict({})", val), _ => {} }
            }
        }
        val
    }

    pub fn gen_expr(&self, expr: &Expr) -> String {
        match expr {
            Expr::Literal(lit) => match &lit.value {
                LiteralValue::Int(v) => v.to_string(), LiteralValue::Float(v) => v.to_string(),
                LiteralValue::Str(s) => format!("\"{}\"", s), LiteralValue::Bool(true) => "True".into(), LiteralValue::Bool(false) => "False".into(),
            },
            Expr::VarRef(vr) => self.ctx.rename(&vr.name),
            Expr::BinaryOp(b) => {
                let left = self.gen_expr(&b.left);
                let right = self.gen_expr(&b.right);
                match b.op {
                    BinaryOpKind::Div => { self.needs_idiv.set(true); format!("_idiv({}, {})", left, right) }
                    BinaryOpKind::Mod => { self.needs_irem.set(true); format!("_irem({}, {})", left, right) }
                    _ => format!("({} {} {})", left, b.op.as_str(), right),
                }
            }
            Expr::UnaryOp(u) => {
                let op = match u.op { UnaryOpKind::Neg => "-", UnaryOpKind::Not => "not " };
                format!("({}{})", op, self.gen_expr(&u.operand))
            }
            Expr::FuncCall(fc) => {
                let name = self.gen_expr(&fc.func);
                let args = fc.args.iter().map(|a| self.gen_expr(a)).collect::<Vec<_>>().join(", ");
                match name.as_str() {
                    "all" => format!("asyncio.gather({})", args),
                    _ => format!("{}({})", name, args),
                }
            }
            Expr::MethodCall(mc) => {
                let obj = self.gen_expr(&mc.object);
                let args = mc.args.iter().map(|a| self.gen_expr(a)).collect::<Vec<_>>().join(", ");
                match mc.method.as_str() {
                    "len" => format!("len({})", obj), "is_empty" => format!("len({}) == 0", obj),
                    "get" => format!("{}[{}]", obj, args), "push" => format!("{}.append({})", obj, args),
                    "pop" => format!("{}.pop()", obj), "contains" => format!("{} in {}", args, obj),
                    "insert" => {
                        let k = mc.args.first().map(|a| self.gen_expr(a)).unwrap_or_default();
                        let v = mc.args.get(1).map(|a| self.gen_expr(a)).unwrap_or_default();
                        format!("{}[{}] = {}", obj, k, v)
                    }
                    "to_string" => {
                        let obj_ty = self.ctx.type_map.get(&mc.object.node_id());
                        let is_bool = obj_ty.map_or(false, |t| t.base_name() == "bool");
                        if is_bool { format!("str({}).lower()", obj) }
                        else { format!("str({})", obj) }
                    }
                    _ => format!("{}.{}({})", obj, mc.method, args),
                }
            }
            Expr::IndexGet(ig) => format!("{}[{}]", self.gen_expr(&ig.object), self.gen_expr(&ig.index)),
            Expr::ArrayLiteral(al) => {
                let e = al.elements.iter().map(|x| self.gen_expr(x)).collect::<Vec<_>>().join(", ");
                format!("[{}]", e)
            }
            Expr::MapLiteral(ml) => {
                let e = ml.entries.iter().map(|(k,v)| format!("{}: {}", self.gen_expr(k), self.gen_expr(v))).collect::<Vec<_>>().join(", ");
                format!("{{{}}}", e)
            }
            Expr::IfExpr(ie) => {
                let c = self.gen_expr(&ie.condition); let t = self.gen_expr(&ie.then_expr); let e = self.gen_expr(&ie.else_expr);
                format!("({} if {} else {})", t, c, e)
            }
            Expr::TemplateString(ts) => {
                let mut s = String::from("f\"");
                for part in &ts.parts {
                    match &part.value {
                        TemplatePartValue::Literal(l) => s.push_str(&l.replace('{', "{{").replace('}', "}}")),
                        TemplatePartValue::Expr(e) => { s.push('{'); s.push_str(&self.gen_expr(e)); s.push('}'); }
                    }
                }
                s.push('"'); s
            }
            Expr::NullCoalesce(nc) => {
                let l = self.gen_expr(&nc.left); format!("({} if {} is not None else {})", l, l, self.gen_expr(&nc.right))
            }
            Expr::StructLiteral(sl) => {
                let f = sl.fields.iter().map(|(k,v)| format!("{}={}", k, self.gen_expr(v))).collect::<Vec<_>>().join(", ");
                format!("{}({})", sl.type_name, f)
            }
            Expr::AwaitExpr(ae) => format!("await {}", self.gen_expr(&ae.expr)),
            _ => "<expr>".into(),
        }
    }

    fn py_type(&self, ty: &TypeRepr) -> String {
        match ty {
            TypeRepr::Named { name, type_args, .. } => match name.as_str() {
                "i32" => "int", "f64" => "float", "bool" => "bool", "String" => "str", "void" => "None",
                "Array" => if !type_args.is_empty() { return format!("list[{}]", self.py_type(&type_args[0])) } else { return "list".into() },
                "Map" => if type_args.len() > 1 { return format!("dict[{}, {}]", self.py_type(&type_args[0]), self.py_type(&type_args[1])) } else { return "dict".into() },
                _ => return name.clone(),
            }.to_string(),
            TypeRepr::Union { types, .. } => types.iter().map(|t| self.py_type(t)).collect::<Vec<_>>().join(" | "),
            _ => "Any".into(),
        }
    }
}
