use crate::ast::*;
use crate::codegen::base::CodegenContext;
use std::collections::{HashMap, HashSet};
use std::cell::RefCell;

// JS target: generates idiomatic JavaScript (Node.js) with helper functions
pub struct JavaScriptGenerator {
    pub ctx: CodegenContext,
    pub needed_helpers: RefCell<HashSet<String>>,
    pub var_types: RefCell<HashMap<String, String>>,
}

impl JavaScriptGenerator {
    pub fn new(type_map: HashMap<NodeId, TypeRepr>, rename_map: Vec<HashMap<String, String>>) -> Self {
        Self {
            ctx: CodegenContext::new(type_map, rename_map),
            needed_helpers: RefCell::new(HashSet::new()),
            var_types: RefCell::new(HashMap::new()),
        }
    }

    pub fn generate(mut self, program: &Program) -> String {
        for s in &program.structs { self.gen_struct(s); }
        for f in &program.functions { self.gen_function(f); }
        self.ctx.emit("main();");

        let mut result = String::new();
        let helpers = self.get_helpers();
        if !helpers.is_empty() { result.push_str(&helpers); result.push('\n'); }
        result.push_str(&self.ctx.result());
        result
    }

    fn need_helper(&self, name: &str) { self.needed_helpers.borrow_mut().insert(name.to_string()); }

    fn get_helpers(&self) -> String {
        let needed = self.needed_helpers.borrow();
        let mut out = String::new();
        if needed.contains("_idiv") {
            out.push_str("function _idiv(a, b) {\n    if (b === 0) throw new Error(\"division by zero\");\n    const r = Math.trunc(a / b);\n    return r === 0 ? 0 : r;\n}\n");
        }
        if needed.contains("_irem") {
            out.push_str("function _irem(a, b) {\n    if (b === 0) throw new Error(\"division by zero\");\n    return a % b;\n}\n");
        }
        if needed.contains("_arrlen") { out.push_str("function _arrlen(arr) { return arr.length; }\n"); }
        if needed.contains("_arrget") { out.push_str("function _arrget(arr, i) {\n    if (i < 0 || i >= arr.length) throw new Error(\"index out of bounds\");\n    return arr[i];\n}\n"); }
        if needed.contains("_arrset") { out.push_str("function _arrset(arr, i, val) {\n    if (i < 0 || i >= arr.length) throw new Error(\"index out of bounds\");\n    arr[i] = val;\n}\n"); }
        if needed.contains("_arrpop") { out.push_str("function _arrpop(arr) {\n    if (arr.length === 0) throw new Error(\"pop from empty array\");\n    return arr.pop();\n}\n"); }
        if needed.contains("_maplen") { out.push_str("function _maplen(map) { return map.size; }\n"); }
        if needed.contains("_mapget") { out.push_str("function _mapget(map, key) {\n    if (!map.has(key)) throw new Error(\"Key not found\");\n    return map.get(key);\n}\n"); }
        if needed.contains("_arrClone") { out.push_str("function _arrClone(arr) { return arr.slice(); }\n"); }
        if needed.contains("_mapClone") { out.push_str("function _mapClone(map) { return new Map(map); }\n"); }
        out
    }

    fn gen_struct(&mut self, s: &StructDefNode) {
        let params = s.fields.iter().map(|f| f.name.clone()).collect::<Vec<_>>().join(", ");
        self.ctx.emit(&format!("class {} {{", s.name));
        self.ctx.indent();
        self.ctx.emit(&format!("constructor({}) {{", params));
        self.ctx.indent();
        for f in &s.fields {
            self.ctx.emit(&format!("this.{} = {};", f.name, self.js_default(&f.type_ann)));
        }
        self.ctx.dedent();
        self.ctx.emit("}");
        for method in &s.methods {
            self.gen_method(method);
        }
        self.ctx.dedent();
        self.ctx.emit("}");
        self.ctx.emit("");
    }

    fn gen_method(&mut self, func: &FunctionDefNode) {
        let params = func.params.iter().map(|p| p.name.clone()).collect::<Vec<_>>().join(", ");
        self.ctx.emit(&format!("{}({}) {{", func.name, params));
        self.ctx.indent();
        if let Some(ref body) = func.body { self.gen_block(body); }
        self.ctx.dedent();
        self.ctx.emit("}");
    }

    fn gen_function(&mut self, func: &FunctionDefNode) {
        let prefix = if func.is_async { "async " } else { "" };
        let params = func.params.iter().map(|p| p.name.clone()).collect::<Vec<_>>().join(", ");
        self.ctx.emit(&format!("{}function {}({}) {{", prefix, func.name, params));
        self.ctx.indent();
        if let Some(ref body) = func.body { self.gen_block(body); }
        self.ctx.dedent();
        self.ctx.emit("}");
        self.ctx.emit("");
    }

    fn gen_block(&mut self, block: &BlockNode) {
        for stmt in &block.stmts { self.gen_stmt(stmt); }
        if let Some(ref trailing) = block.trailing_expr {
            let val = self.gen_expr(trailing);
            self.ctx.emit(&format!("return {};", val));
        }
    }

    fn gen_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VarDecl(vd) => {
                let keyword = if vd.mutable { "let" } else { "const" };
                let init_val = self.gen_expr(&vd.init);
                let final_init = self.wrap_container_clone(&vd.init, vd.type_ann.as_ref(), init_val);
                let type_name = vd.type_ann.as_ref().map_or("any".into(), |t| t.base_name().to_string());
                self.var_types.borrow_mut().insert(vd.name.clone(), type_name);
                self.ctx.emit(&format!("{} {} = {};", keyword, vd.name, final_init));
            }
            Stmt::Assign(a) => {
                let val = self.gen_expr(&a.value);
                if let Some(ref idx) = a.index {
                    let idx_val = self.gen_expr(idx);
                    self.ctx.emit(&format!("{}[{}] = {};", a.target, idx_val, val));
                } else {
                    match a.op.as_str() {
                        "/=" => { self.need_helper("_idiv"); self.ctx.emit(&format!("{} = _idiv({}, {});", a.target, a.target, val)); }
                        "%=" => { self.need_helper("_irem"); self.ctx.emit(&format!("{} = _irem({}, {});", a.target, a.target, val)); }
                        _ => self.ctx.emit(&format!("{} {}= {};", a.target, &a.op[..a.op.len()-1], val)),
                    }
                }
            }
            Stmt::IfStmt(s) => {
                let cond = self.gen_expr(&s.condition);
                self.ctx.emit(&format!("if ({}) {{", cond));
                self.ctx.indent(); self.gen_block(&s.then_body); self.ctx.dedent();
                for (cond_expr, body) in &s.else_ifs {
                    let c = self.gen_expr(cond_expr);
                    self.ctx.emit(&format!("}} else if ({}) {{", c));
                    self.ctx.indent(); self.gen_block(body); self.ctx.dedent();
                }
                if s.else_body.is_some() {
                    self.ctx.emit("} else {");
                    self.ctx.indent(); self.gen_block(s.else_body.as_ref().unwrap()); self.ctx.dedent();
                }
                self.ctx.emit("}");
            }
            Stmt::WhileStmt(s) => {
                let cond = self.gen_expr(&s.condition);
                self.ctx.emit(&format!("while ({}) {{", cond));
                self.ctx.indent(); self.gen_block(&s.body); self.ctx.dedent();
                self.ctx.emit("}");
            }
            Stmt::ForInStmt(s) => {
                let iter = self.gen_expr(&s.iterable);
                self.ctx.emit(&format!("for (const {} of {}) {{", s.var_name, iter));
                self.ctx.indent(); self.gen_block(&s.body); self.ctx.dedent();
                self.ctx.emit("}");
            }
            Stmt::BreakStmt(_) => self.ctx.emit("break;"),
            Stmt::ContinueStmt(_) => self.ctx.emit("continue;"),
            Stmt::ReturnStmt(s) => {
                let val = s.value.as_ref().map(|v| self.gen_expr(v));
                if let Some(v) = val { self.ctx.emit(&format!("return {};", v)); }
                else { self.ctx.emit("return;"); }
            }
            Stmt::PrintStmt(s) => {
                let arg = self.gen_expr(&s.arg);
                if s.newline { self.ctx.emit(&format!("console.log({});", arg)); }
                else { self.ctx.emit(&format!("process.stdout.write({});", arg)); }
            }
            Stmt::ExprStmt(s) => {
                let val = self.gen_expr(&s.expr);
                self.ctx.emit(&format!("{};", val));
            }
            _ => {}
        }
    }

    fn wrap_container_clone(&self, init: &Expr, type_ann: Option<&TypeRepr>, val: String) -> String {
        if let Expr::VarRef(_) = init {
            if let Some(ann) = type_ann {
                match ann.base_name() {
                    "Array" => { self.need_helper("_arrClone"); return format!("_arrClone({})", val); }
                    "Map" => { self.need_helper("_mapClone"); return format!("_mapClone({})", val); }
                    _ => {}
                }
            }
        }
        // Note: gen_expr for MapLiteral already produces `new Map([...])`, so no extra wrapping
        val
    }

    pub fn gen_expr(&self, expr: &Expr) -> String {
        match expr {
            Expr::Literal(lit) => self.gen_literal(lit),
            Expr::VarRef(vr) => vr.name.clone(),
            Expr::BinaryOp(b) => self.gen_binary(b),
            Expr::UnaryOp(u) => {
                let op = match u.op { UnaryOpKind::Neg => "-", UnaryOpKind::Not => "!" };
                let inner = self.gen_expr(&u.operand);
                format!("({}{})", op, inner)
            }
            Expr::FuncCall(fc) => self.gen_func_call(fc),
            Expr::MethodCall(mc) => self.gen_method_call(mc),
            Expr::IndexGet(ig) => {
                let obj = self.gen_expr(&ig.object);
                let idx = self.gen_expr(&ig.index);
                self.need_helper("_arrget");
                format!("_arrget({}, {})", obj, idx)
            }
            Expr::ArrayLiteral(al) => {
                let elems: Vec<String> = al.elements.iter().map(|e| self.gen_expr(e)).collect();
                format!("[{}]", elems.join(", "))
            }
            Expr::MapLiteral(ml) => {
                let entries: Vec<String> = ml.entries.iter()
                    .map(|(k, v)| format!("[{}, {}]", self.gen_expr(k), self.gen_expr(v)))
                    .collect();
                format!("new Map([{}])", entries.join(", "))
            }
            Expr::IfExpr(ie) => {
                let cond = self.gen_expr(&ie.condition);
                let then = self.gen_expr(&ie.then_expr);
                let else_ = self.gen_expr(&ie.else_expr);
                format!("({} ? {} : {})", cond, then, else_)
            }
            Expr::TemplateString(ts) => {
                let mut s = String::from("`");
                for part in &ts.parts {
                    match &part.value {
                        TemplatePartValue::Literal(l) => s.push_str(l),
                        TemplatePartValue::Expr(e) => { s.push_str("${"); s.push_str(&self.gen_expr(e)); s.push('}'); }
                    }
                }
                s.push('`');
                s
            }
            Expr::NullCoalesce(nc) => {
                let l = self.gen_expr(&nc.left);
                let r = self.gen_expr(&nc.right);
                format!("({} ?? {})", l, r)
            }
            Expr::StructLiteral(sl) => {
                let fields: Vec<String> = sl.fields.iter().map(|(k, v)| format!("{}: {}", k, self.gen_expr(v))).collect();
                format!("{{ {} }}", fields.join(", "))
            }
            Expr::AwaitExpr(ae) => format!("await {}", self.gen_expr(&ae.expr)),
            Expr::SpreadExpr(se) => format!("...{}", self.gen_expr(&se.expr)),
            _ => "<expr>".to_string(),
        }
    }

    fn gen_literal(&self, lit: &LiteralNode) -> String {
        match &lit.value {
            LiteralValue::Int(v) => v.to_string(),
            LiteralValue::Float(v) => v.to_string(),
            LiteralValue::Str(s) => format!("\"{}\"", s.escape_debug()),
            LiteralValue::Bool(true) => "true".into(),
            LiteralValue::Bool(false) => "false".into(),
        }
    }

    fn gen_binary(&self, b: &BinaryOpNode) -> String {
        let left = self.gen_expr(&b.left);
        let right = self.gen_expr(&b.right);
        match b.op {
            BinaryOpKind::Div => { self.need_helper("_idiv"); format!("_idiv({}, {})", left, right) }
            BinaryOpKind::Mod => { self.need_helper("_irem"); format!("_irem({}, {})", left, right) }
            _ => format!("({} {} {})", left, b.op.as_str(), right),
        }
    }

    fn gen_func_call(&self, fc: &FuncCallNode) -> String {
        let name = self.gen_expr(&fc.func);
        let args: Vec<String> = fc.args.iter().map(|a| self.gen_expr(a)).collect();
        match name.as_str() {
            "all" => format!("Promise.all([{}])", args.join(", ")),
            "race" => format!("Promise.race([{}])", args.join(", ")),
            _ => format!("{}({})", name, args.join(", ")),
        }
    }

    fn gen_method_call(&self, mc: &MethodCallNode) -> String {
        let obj = self.gen_expr(&mc.object);
        let args: Vec<String> = mc.args.iter().map(|a| self.gen_expr(a)).collect();
        // Determine if receiver is Map vs Array using type_map
        let is_map = self.ctx.type_map.get(&mc.object.node_id())
            .map_or(false, |t| t.base_name() == "Map");
        match mc.method.as_str() {
            "len" => {
                if is_map { self.need_helper("_maplen"); format!("_maplen({})", obj) }
                else { self.need_helper("_arrlen"); format!("_arrlen({})", obj) }
            }
            "is_empty" => {
                if is_map { self.need_helper("_maplen"); format!("_maplen({}) === 0", obj) }
                else { format!("({}.length === 0)", obj) }
            }
            "get" => {
                if is_map { self.need_helper("_mapget"); format!("_mapget({}, {})", obj, args.join(", ")) }
                else { self.need_helper("_arrget"); format!("_arrget({}, {})", obj, args.join(", ")) }
            }
            "push" => format!("{}.push({})", obj, args.join(", ")),
            "pop" => {
                if is_map { format!("{}.delete({})", obj, args.join(", ")) }
                else { self.need_helper("_arrpop"); format!("_arrpop({})", obj) }
            }
            "contains" => format!("{}.has({})", obj, args.join(", ")),
            "insert" => {
                let k = args.first().map(|s| s.as_str()).unwrap_or("");
                let v = args.get(1).map(|s| s.as_str()).unwrap_or("");
                format!("{}.set({}, {})", obj, k, v)
            }
            "to_string" => format!("String({})", obj),
            _ => format!("{}.{}({})", obj, mc.method, args.join(", ")),
        }
    }

    fn js_default(&self, ty: &TypeRepr) -> String {
        match ty.base_name() {
            "i32"|"f64" => "0".into(), "bool" => "false".into(),
            "String" => "\"\"".into(), "Array" => "[]".into(),
            "Map" => "new Map()".into(), _ => "undefined".into(),
        }
    }
}
