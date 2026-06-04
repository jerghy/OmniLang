use crate::ast::*;
use crate::codegen::base::CodegenContext;
use std::collections::{HashMap, HashSet};
use std::cell::Cell;

pub struct RustGenerator {
    pub ctx: CodegenContext,
    pub temp_counter: Cell<u32>,
    pub has_map: Cell<bool>,
    pub has_cell: Cell<bool>,
    pub has_refcell: Cell<bool>,
    pub var_types: HashMap<String, TypeRepr>,
    pub for_vars: HashSet<String>,
}

impl RustGenerator {
    pub fn new(type_map: HashMap<NodeId, TypeRepr>, rename_map: Vec<HashMap<String, String>>) -> Self {
        Self {
            ctx: CodegenContext::new(type_map, rename_map),
            temp_counter: Cell::new(0),
            has_map: Cell::new(false),
            has_cell: Cell::new(false),
            has_refcell: Cell::new(false),
            var_types: HashMap::new(),
            for_vars: HashSet::new(),
        }
    }

    pub fn generate(mut self, program: &Program) -> String {
        // Check for Map usage
        for func in &program.functions {
            if let Some(ref body) = func.body {
                self.detect_map_usage_block(&body.stmts);
            }
        }

        // Generate imports
        let mut prelude = String::new();
        if self.has_map.get() { prelude.push_str("use std::collections::HashMap;\n"); }
        if self.has_cell.get() { prelude.push_str("use std::cell::Cell;\n"); }
        if self.has_refcell.get() { prelude.push_str("use std::cell::RefCell;\n"); }
        if !prelude.is_empty() { prelude.push('\n'); }

        // Check for async main
        let has_async_main = program.functions.iter().any(|f| f.name == "main" && f.is_async);
        if has_async_main { prelude.push_str("#[tokio::main]\n"); }

        // Structs
        for s in &program.structs { self.gen_struct(s); }
        // Tagged unions
        for tu in &program.tagged_unions { self.gen_tagged_union(tu); }
        // Functions
        for f in &program.functions { self.gen_function(f); }

        prelude + &self.ctx.result()
    }

    fn detect_map_usage_block(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            match stmt {
                Stmt::VarDecl(vd) => {
                    if let Some(ref ann) = vd.type_ann {
                        if ann.base_name() == "Map" { self.has_map.set(true); }
                        if ann.base_name() == "Array" {} // Vec is always available
                    }
                }
                Stmt::IfStmt(s) => {
                    self.detect_map_usage_block(&s.then_body.stmts);
                    for (_, body) in &s.else_ifs { self.detect_map_usage_block(&body.stmts); }
                    if let Some(ref else_body) = s.else_body { self.detect_map_usage_block(&else_body.stmts); }
                }
                Stmt::WhileStmt(s) => self.detect_map_usage_block(&s.body.stmts),
                Stmt::ForInStmt(s) => self.detect_map_usage_block(&s.body.stmts),
                _ => {}
            }
        }
    }

    fn gen_struct(&mut self, s: &StructDefNode) {
        let mut derives = vec!["Debug", "Clone"];
        let all_copy = s.fields.iter().all(|f| self.is_copy_type(&f.type_ann));
        if all_copy { derives.push("Copy"); }
        self.ctx.emit(&format!("#[derive({})]", derives.join(", ")));
        self.ctx.emit(&format!("pub struct {} {{", s.name));
        self.ctx.indent();
        for f in &s.fields { self.ctx.emit(&format!("pub {}: {},", f.name, self.rust_type(&f.type_ann))); }
        self.ctx.dedent();
        self.ctx.emit("}");
        self.ctx.emit("");

        if !s.methods.is_empty() {
            self.ctx.emit(&format!("impl {} {{", s.name));
            self.ctx.indent();
            for method in &s.methods { self.gen_method(method); }
            self.ctx.dedent();
            self.ctx.emit("}");
            self.ctx.emit("");
        }
    }

    fn gen_tagged_union(&mut self, tu: &TaggedUnionDefNode) {
        self.ctx.emit(&format!("#[derive(Debug, Clone)]"));
        self.ctx.emit(&format!("pub enum {} {{", tu.name));
        self.ctx.indent();
        for var in &tu.variants {
            if var.fields.is_empty() {
                self.ctx.emit(&format!("{},", var.name));
            } else {
                let fields: Vec<String> = var.fields.iter().map(|f| format!("{}: {}", f.name, self.rust_type(&f.type_ann))).collect();
                self.ctx.emit(&format!("{} {{ {} }},", var.name, fields.join(", ")));
            }
        }
        self.ctx.dedent();
        self.ctx.emit("}");
        self.ctx.emit("");
    }

    fn gen_method(&mut self, func: &FunctionDefNode) {
        let params: Vec<String> = func.params.iter().map(|p| format!("{}: {}", p.name, self.rust_param_type(p.type_ann.as_ref().unwrap_or(&TypeRepr::named("void"))))).collect();
        let ret = func.return_type.as_ref().map_or("()".to_string(), |rt| self.rust_type(rt));
        self.ctx.emit(&format!("pub fn {}(&self, {}) -> {} {{", func.name, params.join(", "), ret));
        self.ctx.indent();
        if let Some(ref body) = func.body { self.gen_block(body); }
        self.ctx.dedent();
        self.ctx.emit("}");
    }

    fn gen_function(&mut self, func: &FunctionDefNode) {
        let prefix = if func.is_async { "async " } else { "" };
        let params: Vec<String> = func.params.iter().map(|p| {
            let void_type = TypeRepr::named("void");
            let ty = p.type_ann.as_ref().unwrap_or(&void_type);
            format!("{}: {}", p.name, self.rust_param_type(ty))
        }).collect();
        let ret = if func.name == "main" && func.return_type.is_none() {
            String::new()
        } else if let Some(ref rt) = &func.return_type {
            if rt.base_name() == "void" { String::new() }
            else { format!(" -> {}", self.rust_type(rt)) }
        } else { String::new() };

        self.ctx.emit(&format!("{}fn {}({}){} {{", prefix, func.name, params.join(", "), ret));
        self.ctx.indent();

        // Track parameter types
        for param in &func.params {
            if let Some(ref ann) = param.type_ann {
                self.var_types.insert(param.name.clone(), ann.clone());
            }
        }

        if let Some(ref body) = func.body { self.gen_block(body); }

        self.ctx.dedent();
        self.ctx.emit("}");
        self.ctx.emit("");
    }

    fn gen_block(&mut self, block: &BlockNode) {
        for stmt in &block.stmts { self.gen_stmt(stmt); }
        if let Some(ref trailing) = block.trailing_expr {
            let val = self.gen_expr(trailing, true);
            self.ctx.emit(&val);
        }
    }

    fn gen_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VarDecl(vd) => {
                let is_container = matches!(vd.type_ann.as_ref().map(|t| t.base_name()), Some("Array" | "Map"));
                let mut_kw = if vd.mutable || is_container { "mut " } else { "" };
                let init = self.gen_expr(&vd.init, true);

                let mut init_val = init;
                // String literal needs String::from
                if let Expr::Literal(ref lit) = &vd.init {
                    if lit.kind == "String" {
                        if let LiteralValue::Str(ref s) = lit.value {
                            init_val = format!("String::from(\"{}\")", s.escape_debug());
                        }
                    }
                }
                // Clone from VarRef for non-Copy types
                if let Expr::VarRef(ref vr) = &vd.init {
                    if let Some(ref ann) = vd.type_ann {
                        if !self.is_copy_type(ann) {
                            if ann.base_name() == "String" {
                                init_val = format!("{}.to_string()", vr.name);
                            } else {
                                init_val = format!("{}.clone()", vr.name);
                            }
                        }
                    }
                }
                // Vec! macro for array literals
                if let Expr::ArrayLiteral(_) = &vd.init {
                    // The gen_expr already produces vec![...]
                }
                // HashMap::from for map literals
                if let Expr::MapLiteral(_) = &vd.init { self.has_map.set(true); }

                if let Some(ref ann) = vd.type_ann {
                    let rt = self.rust_type(ann);
                    self.ctx.emit(&format!("let {} {}: {} = {};", mut_kw, vd.name, rt, init_val));
                    self.var_types.insert(vd.name.clone(), ann.clone());
                } else {
                    self.ctx.emit(&format!("let {} {} = {};", mut_kw, vd.name, init_val));
                }
            }
            Stmt::Assign(a) => {
                let val = self.gen_expr(&a.value, true);
                if let Some(ref idx) = a.index {
                    let idx_val = self.gen_expr(idx, false);
                    // Map uses .insert(), Vec uses [idx as usize] = val
                    let is_map = self.var_types.get(&a.target)
                        .map_or(false, |t| t.base_name() == "Map");
                    if is_map {
                        if idx_val.parse::<i32>().is_ok() {
                            self.ctx.emit(&format!("{}.insert({}, {});", a.target, idx_val, val));
                        } else {
                            self.ctx.emit(&format!("{}.insert({}.clone(), {});", a.target, idx_val, val));
                        }
                    } else {
                        self.ctx.emit(&format!("{}[{} as usize] = {};", a.target, idx_val, val));
                    }
                } else {
                    match a.op.as_str() {
                        "+=" if self.is_string_var(&a.target) => {
                            self.ctx.emit(&format!("{} = format!(\"{{}}{{}}\", {}, {});", a.target, a.target, val));
                        }
                        _ => self.ctx.emit(&format!("{} {}= {};", a.target, &a.op[..a.op.len()-1], val)),
                    }
                }
            }
            Stmt::IfStmt(s) => {
                self.ctx.emit(&format!("if {} {{", self.gen_expr(&s.condition, false)));
                self.ctx.indent(); self.gen_block(&s.then_body); self.ctx.dedent();
                for (cond, body) in &s.else_ifs {
                    self.ctx.emit(&format!("}} else if {} {{", self.gen_expr(cond, false)));
                    self.ctx.indent(); self.gen_block(body); self.ctx.dedent();
                }
                if s.else_body.is_some() {
                    self.ctx.emit("} else {");
                    self.ctx.indent(); self.gen_block(s.else_body.as_ref().unwrap()); self.ctx.dedent();
                }
                self.ctx.emit("}");
            }
            Stmt::WhileStmt(s) => {
                self.ctx.emit(&format!("while {} {{", self.gen_expr(&s.condition, false)));
                self.ctx.indent(); self.gen_block(&s.body); self.ctx.dedent();
                self.ctx.emit("}");
            }
            Stmt::ForInStmt(s) => {
                let iter = self.gen_expr(&s.iterable, false);
                self.for_vars.insert(s.var_name.clone());
                self.ctx.emit(&format!("for {} in &{} {{", s.var_name, iter));
                self.ctx.indent(); self.gen_block(&s.body); self.ctx.dedent();
                self.ctx.emit("}");
            }
            Stmt::BreakStmt(_) => self.ctx.emit("break;"),
            Stmt::ContinueStmt(_) => self.ctx.emit("continue;"),
            Stmt::ReturnStmt(s) => {
                if let Some(ref v) = s.value {
                    let val = self.gen_expr(v, true); // need_ownership for move
                    self.ctx.emit(&format!("return {};", val));
                } else {
                    self.ctx.emit("return;");
                }
            }
            Stmt::PrintStmt(s) => {
                let arg = self.gen_expr(&s.arg, false);
                if s.newline { self.ctx.emit(&format!("println!(\"{{}}\", {});", arg)); }
                else { self.ctx.emit(&format!("print!(\"{{}}\", {});", arg)); }
            }
            Stmt::ExprStmt(s) => { self.ctx.emit(&format!("{};", self.gen_expr(&s.expr, false))); }
            _ => {}
        }
    }

    pub fn gen_expr(&self, expr: &Expr, need_ownership: bool) -> String {
        match expr {
            Expr::Literal(lit) => match &lit.value {
                LiteralValue::Int(v) => v.to_string(),
                LiteralValue::Float(v) => {
                    let mut s = v.to_string();
                    if !s.contains('.') && !s.contains('e') { s.push_str(".0"); }
                    s.push_str("_f64");
                    s
                }
                LiteralValue::Str(s) => format!("String::from(\"{}\")", s.escape_debug()),
                LiteralValue::Bool(true) => "true".to_string(),
                LiteralValue::Bool(false) => "false".to_string(),
            },
            Expr::VarRef(vr) => {
                if self.for_vars.contains(&vr.name) {
                    // For-in loop variable: auto-deref for Copy types
                    format!("*{}", vr.name)
                } else {
                    vr.name.clone()
                }
            }
            Expr::BinaryOp(b) => {
                let left = self.gen_expr(&b.left, false);
                let right = self.gen_expr(&b.right, false);
                match b.op {
                    BinaryOpKind::Add => {
                        // Check if it's string concatenation
                        if self.is_string_expr(&b.left) {
                            format!("format!(\"{{}}{{}}\", {}, {})", left, right)
                        } else {
                            format!("({} + {})", left, right)
                        }
                    }
                    _ => format!("({} {} {})", left, b.op.as_str(), right),
                }
            }
            Expr::UnaryOp(u) => {
                let op = match u.op { UnaryOpKind::Neg => "-", UnaryOpKind::Not => "!" };
                format!("({}{})", op, self.gen_expr(&u.operand, false))
            }
            Expr::FuncCall(fc) => {
                let name = self.gen_expr(&fc.func, false);
                let args: Vec<String> = fc.args.iter().map(|a| {
                    // For non-Copy args, pass by reference
                    let val = self.gen_expr(a, false);
                    if self.is_non_copy_expr(a) { format!("&{}", val) }
                    else { val }
                }).collect();
                match name.as_str() {
                    "all" => format!("futures::future::join_all(vec![{}]).await", args.join(", ")),
                    _ => format!("{}({})", name, args.join(", ")),
                }
            }
            Expr::MethodCall(mc) => {
                let obj = self.gen_expr(&mc.object, false);
                let is_map = self.ctx.type_map.get(&mc.object.node_id())
                    .map_or(false, |t| t.base_name() == "Map");
                // Map methods take keys by reference, not by clone
                let args: Vec<String> = mc.args.iter().map(|a| {
                    let v = self.gen_expr(a, false);
                    if is_map && (mc.method == "get" || mc.method == "contains") {
                        // Map keys: use &key for variables, direct for literals
                        if v.starts_with("String::from") || v.parse::<i32>().is_ok() { v }
                        else { format!("&{}", v) }
                    } else if self.is_non_copy_expr(a) { format!("{}.clone()", v) }
                    else { v }
                }).collect();
                match mc.method.as_str() {
                    "len" => format!("({}.len() as i32)", obj),
                    "is_empty" => format!("{}.is_empty()", obj),
                    "get" => {
                        let is_map = self.ctx.type_map.get(&mc.object.node_id())
                            .map_or(false, |t| t.base_name() == "Map");
                        if is_map {
                            let key = self.emit_map_key(args.first().map(|s| s.as_str()).unwrap_or(""));
                            format!("{}[{}]", obj, key)
                        } else {
                            format!("{}[{} as usize]", obj, args.join(", "))
                        }
                    }
                    "push" => format!("{}.push({})", obj, args.join(", ")),
                    "pop" => format!("{}.pop().expect(\"empty\")", obj),
                    "contains" => {
                        let key = self.emit_map_key(args.first().map(|s| s.as_str()).unwrap_or(""));
                        format!("{}.contains_key({})", obj, key)
                    }
                    "insert" => {
                        let k = self.emit_map_key(args.first().map(|s| s.as_str()).unwrap_or(""));
                        let v = args.get(1).map(|s| s.as_str()).unwrap_or("");
                        format!("{}.insert({}, {})", obj, k, v)
                    }
                    "to_string" => format!("{}.to_string()", obj),
                    _ => format!("{}.{}({})", obj, mc.method, args.join(", ")),
                }
            }
            Expr::IndexGet(ig) => {
                let obj = self.gen_expr(&ig.object, false);
                let idx = self.gen_expr(&ig.index, false);
                let is_map = self.ctx.type_map.get(&ig.object.node_id())
                    .map_or(false, |t| t.base_name() == "Map");
                if is_map {
                    let key = self.emit_map_key(&idx);
                    format!("{}[{}]", obj, key)
                } else {
                    format!("{}[{} as usize]", obj, idx)
                }
            }
            Expr::ArrayLiteral(al) => {
                let elems: Vec<String> = al.elements.iter().map(|e| self.gen_expr(e, true)).collect();
                format!("vec![{}]", elems.join(", "))
            }
            Expr::MapLiteral(ml) => {
                self.has_map.set(true);
                let entries: Vec<String> = ml.entries.iter()
                    .map(|(k, v)| format!("({}, {})", self.gen_expr(k, true), self.gen_expr(v, true)))
                    .collect();
                format!("HashMap::from([{}])", entries.join(", "))
            }
            Expr::IfExpr(ie) => {
                format!("if {} {{ {} }} else {{ {} }}",
                    self.gen_expr(&ie.condition, false),
                    self.gen_expr(&ie.then_expr, need_ownership),
                    self.gen_expr(&ie.else_expr, need_ownership))
            }
            Expr::TemplateString(ts) => {
                let mut format_str = String::new();
                let mut format_args: Vec<String> = vec![];
                for part in &ts.parts {
                    match &part.value {
                        TemplatePartValue::Literal(l) => {
                            format_str.push_str(&l.replace('{', "{{").replace('}', "}}"));
                        }
                        TemplatePartValue::Expr(e) => {
                            format_str.push_str("{}");
                            format_args.push(self.gen_expr(e, false));
                        }
                    }
                }
                if format_args.is_empty() {
                    format!("String::from(\"{}\")", format_str)
                } else {
                    format!("format!(\"{}\", {})", format_str, format_args.join(", "))
                }
            }
            Expr::NullCoalesce(nc) => {
                let left = self.gen_expr(&nc.left, false);
                let right = self.gen_expr(&nc.right, need_ownership);
                format!("{}.unwrap_or({})", left, right)
            }
            Expr::StructLiteral(sl) => {
                let fields: Vec<String> = sl.fields.iter()
                    .map(|(k, v)| format!("{}: {}", k, self.gen_expr(v, true)))
                    .collect();
                format!("{} {{ {} }}", sl.type_name, fields.join(", "))
            }
            Expr::MatchExpr(me) => {
                let scr = self.gen_expr(&me.scrutinee, false);
                let arms: Vec<String> = me.arms.iter().map(|arm| {
                    let pat = self.gen_rust_match_pattern(&arm.pattern);
                    let body = match &*arm.body {
                        MatchArmBody::Expr(e) => self.gen_expr(e, need_ownership),
                        MatchArmBody::Block(b) => {
                            if let Some(ref t) = b.trailing_expr { self.gen_expr(t, need_ownership) }
                            else { "()".to_string() }
                        }
                    };
                    if let Some(ref guard) = arm.guard {
                        format!("{} if {} => {}", pat, self.gen_expr(guard, false), body)
                    } else {
                        format!("{} => {}", pat, body)
                    }
                }).collect();
                format!("match {} {{ {} }}", scr, arms.join(", "))
            }
            Expr::AwaitExpr(ae) => format!("{}.await", self.gen_expr(&ae.expr, false)),
            _ => "<expr>".to_string(),
        }
    }

    fn gen_rust_match_pattern(&self, pat: &MatchPattern) -> String {
        match pat {
            MatchPattern::Wildcard | MatchPattern::Ignore => "_".to_string(),
            MatchPattern::Var(name) => name.clone(),
            MatchPattern::Literal(lit) => match lit {
                LiteralValue::Int(v) => v.to_string(),
                LiteralValue::Str(s) => format!("\"{}\"", s),
                _ => "_".to_string(),
            },
            MatchPattern::Struct(sp) => {
                let fields: Vec<String> = sp.fields.iter()
                    .map(|(k, p)| format!("{}: {}", k, self.gen_rust_match_pattern(p)))
                    .collect();
                format!("{} {{ {} }}", sp.name, fields.join(", "))
            }
            MatchPattern::Or(pats) => pats.iter().map(|p| self.gen_rust_match_pattern(p)).collect::<Vec<_>>().join(" | "),
            _ => "_".to_string(),
        }
    }

    fn rust_type(&self, ty: &TypeRepr) -> String {
        match ty {
            TypeRepr::Named { name, type_args, .. } => match name.as_str() {
                "i32" => "i32".into(), "f64" => "f64".into(), "bool" => "bool".into(),
                "String" => "String".into(), "void" => "()".into(),
                "Array" => if !type_args.is_empty() { format!("Vec<{}>", self.rust_type(&type_args[0])) } else { "Vec<_>".into() },
                "Map" => {
                    self.has_map.set(true);
                    if type_args.len() > 1 { format!("HashMap<{}, {}>", self.rust_type(&type_args[0]), self.rust_type(&type_args[1])) }
                    else { "HashMap<_, _>".into() }
                }
                _ => name.clone(),
            },
            _ => "()".into(),
        }
    }

    fn rust_param_type(&self, ty: &TypeRepr) -> String {
        match ty {
            TypeRepr::Named { name, type_args, .. } => match name.as_str() {
                "String" => "&str".into(),
                "Array" => if !type_args.is_empty() { format!("&[{}]", self.rust_type(&type_args[0])) } else { "&[_]".into() },
                "Map" => {
                    self.has_map.set(true);
                    if type_args.len() > 1 { format!("&HashMap<{}, {}>", self.rust_type(&type_args[0]), self.rust_type(&type_args[1])) }
                    else { "&HashMap<_, _>".into() }
                }
                _ => self.rust_type(ty),
            },
            _ => self.rust_type(ty),
        }
    }

    fn is_copy_type(&self, ty: &TypeRepr) -> bool {
        matches!(ty.base_name(), "i32" | "f64" | "bool")
    }

    fn is_string_var(&self, name: &str) -> bool {
        self.var_types.get(name).map_or(false, |t| t.base_name() == "String")
    }

    fn is_string_expr(&self, expr: &Expr) -> bool {
        if let Expr::VarRef(vr) = expr {
            return self.is_string_var(&vr.name);
        }
        false
    }

    /// Convert a key expression to Rust map key syntax.
    /// String literal `String::from("x")` → `"x"`; variables and integers get `&` prefix.
    fn emit_map_key(&self, key: &str) -> String {
        // Strip String::from(...) wrapper for string literals
        if let Some(inner) = key.strip_prefix("String::from(\"").and_then(|s| s.strip_suffix("\")")) {
            return format!("\"{}\"", inner); // &str literal works directly
        }
        // For all other keys (integers, variables), need & for HashMap
        format!("&{}", key)
    }

    fn is_non_copy_expr(&self, expr: &Expr) -> bool {
        match expr {
            Expr::VarRef(vr) => {
                self.var_types.get(&vr.name).map_or(false, |t| !self.is_copy_type(t))
            }
            Expr::Literal(lit) => lit.kind == "String",
            Expr::ArrayLiteral(_) | Expr::MapLiteral(_) => true,
            _ => false,
        }
    }
}
