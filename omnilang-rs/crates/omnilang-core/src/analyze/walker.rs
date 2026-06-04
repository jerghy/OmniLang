use crate::ast::*;

/// Walk all expressions in an AST node, calling `f` for each.
pub fn walk_expr(expr: &Expr, f: &mut dyn FnMut(&Expr)) {
    f(expr);
    match expr {
        Expr::BinaryOp(b) => { walk_expr(&b.left, f); walk_expr(&b.right, f); }
        Expr::UnaryOp(u) => walk_expr(&u.operand, f),
        Expr::FuncCall(fc) => { walk_expr(&fc.func, f); for a in &fc.args { walk_expr(a, f); } }
        Expr::MethodCall(mc) => { walk_expr(&mc.object, f); for a in &mc.args { walk_expr(a, f); } }
        Expr::IndexGet(ig) => { walk_expr(&ig.object, f); walk_expr(&ig.index, f); }
        Expr::ArrayLiteral(al) => for e in &al.elements { walk_expr(e, f); }
        Expr::MapLiteral(ml) => for (k, v) in &ml.entries { walk_expr(k, f); walk_expr(v, f); }
        Expr::IfExpr(ie) => { walk_expr(&ie.condition, f); walk_expr(&ie.then_expr, f); walk_expr(&ie.else_expr, f); }
        Expr::ClosureExpr(ce) => {
            match &*ce.body { ClosureBody::Block(b) => walk_block(b, f), ClosureBody::Expr(e) => walk_expr(e, f) }
        }
        Expr::MatchExpr(me) => {
            walk_expr(&me.scrutinee, f);
            for arm in &me.arms {
                match &*arm.body { MatchArmBody::Block(b) => walk_block(b, f), MatchArmBody::Expr(e) => walk_expr(e, f) }
            }
        }
        Expr::AwaitExpr(ae) => walk_expr(&ae.expr, f),
        Expr::SpreadExpr(se) => walk_expr(&se.expr, f),
        Expr::TemplateString(ts) => {
            for p in &ts.parts { if let TemplatePartValue::Expr(e) = &p.value { walk_expr(e, f); } }
        }
        Expr::NullCoalesce(nc) => { walk_expr(&nc.left, f); walk_expr(&nc.right, f); }
        Expr::OptionalChain(oc) => {
            walk_expr(&oc.object, f);
            for link in &oc.chain {
                if let Some(ref idx) = link.index { walk_expr(idx, f); }
                for a in &link.args { walk_expr(a, f); }
            }
        }
        Expr::StructLiteral(sl) => for (_, v) in &sl.fields { walk_expr(v, f); }
        _ => {}
    }
}

/// Walk all statements in a block.
pub fn walk_block(block: &BlockNode, f: &mut dyn FnMut(&Expr)) {
    for stmt in &block.stmts { walk_stmt(stmt, f); }
    if let Some(ref trailing) = block.trailing_expr { walk_expr(trailing, f); }
}

/// Walk expressions within a statement.
pub fn walk_stmt(stmt: &Stmt, f: &mut dyn FnMut(&Expr)) {
    match stmt {
        Stmt::VarDecl(vd) => walk_expr(&vd.init, f),
        Stmt::Destructure(d) => walk_expr(&d.init, f),
        Stmt::Assign(a) => {
            walk_expr(&a.value, f);
            if let Some(ref idx) = a.index { walk_expr(idx, f); }
        }
        Stmt::IfStmt(s) => {
            walk_expr(&s.condition, f); walk_block(&s.then_body, f);
            for (cond, body) in &s.else_ifs { walk_expr(cond, f); walk_block(body, f); }
            if let Some(ref b) = s.else_body { walk_block(b, f); }
        }
        Stmt::WhileStmt(s) => { walk_expr(&s.condition, f); walk_block(&s.body, f); }
        Stmt::ForInStmt(s) => { walk_expr(&s.iterable, f); walk_block(&s.body, f); }
        Stmt::ReturnStmt(s) => { if let Some(ref v) = s.value { walk_expr(v, f); } }
        Stmt::PrintStmt(s) => walk_expr(&s.arg, f),
        Stmt::ExprStmt(s) => walk_expr(&s.expr, f),
        _ => {}
    }
}

/// Walk all expressions in the entire program.
pub fn walk_program(program: &Program, f: &mut dyn FnMut(&Expr)) {
    for func in &program.functions {
        if let Some(ref body) = func.body {
            walk_block(body, f);
        }
    }
}

/// Walk all statements in the program.
pub fn walk_program_stmts(program: &Program, f: &mut dyn FnMut(&Stmt)) {
    for func in &program.functions {
        if let Some(ref body) = func.body {
            for stmt in &body.stmts {
                walk_stmt_recursive(stmt, f);
            }
        }
    }
}

fn walk_stmt_recursive(stmt: &Stmt, f: &mut dyn FnMut(&Stmt)) {
    f(stmt);
    match stmt {
        Stmt::IfStmt(s) => {
            for inner in &s.then_body.stmts { walk_stmt_recursive(inner, f); }
            for (_, body) in &s.else_ifs { for inner in &body.stmts { walk_stmt_recursive(inner, f); } }
            if let Some(ref b) = s.else_body { for inner in &b.stmts { walk_stmt_recursive(inner, f); } }
        }
        Stmt::WhileStmt(s) => for inner in &s.body.stmts { walk_stmt_recursive(inner, f); }
        Stmt::ForInStmt(s) => for inner in &s.body.stmts { walk_stmt_recursive(inner, f); }
        _ => {}
    }
}
