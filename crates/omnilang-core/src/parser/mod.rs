//! Parser module — recursive descent + Pratt expression parsing.

pub mod ast;
pub mod design;
pub mod expr;
pub mod stmt;
pub mod top_level;
pub mod view;

use crate::error::ParseError;
use crate::lexer::{Token, TokenKind};
use crate::parser::stmt::keyword_as_ident_name;
use ast::Program;
#[allow(unused_imports)]
use ast::Visibility;
use rustc_hash::FxHashMap;

/// Parse a token stream into an AST Program.
/// Returns `Ok(program)` on success, or `Err(ParseError)` if any parse errors were
/// collected during recovery. The first collected error is returned.
pub fn parse(tokens: Vec<Token>, source: &str) -> Result<Program, ParseError> {
    let mut parser = Parser::new(tokens, source);
    let module = parser.parse_module()?;
    // Return the first collected parse error (if any) so callers know the AST
    // may be incomplete. The remaining errors are discarded at this level.
    if let Some(err) = parser.errors.first() {
        return Err(ParseError {
            message: format!(
                "{} (and {} more error(s))",
                err.message,
                parser.errors.len() - 1
            ),
            span: err.span,
        });
    }
    let mut modules = FxHashMap::default();
    modules.insert("input.omni".to_string(), module);
    Ok(Program { modules })
}

/// Maximum recursion depth for the parser to prevent stack overflow.
const MAX_RECURSION_DEPTH: usize = 256;

/// The recursive descent parser.
pub struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    depth: usize,
    source: &'a str,
    /// Collected parse errors — parser continues past recoverable errors.
    errors: Vec<ParseError>,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: Vec<Token>, source: &'a str) -> Self {
        Parser {
            tokens,
            pos: 0,
            depth: 0,
            source,
            errors: Vec::new(),
        }
    }

    /// Increment recursion depth, returning an error if the limit is exceeded.
    fn enter_recursion(&mut self) -> Result<(), ParseError> {
        self.depth += 1;
        if self.depth > MAX_RECURSION_DEPTH {
            return Err(ParseError {
                message: format!(
                    "maximum recursion depth ({}) exceeded — input may be too deeply nested",
                    MAX_RECURSION_DEPTH
                ),
                span: self.peek_span(),
            });
        }
        Ok(())
    }

    /// Decrement recursion depth on exit.
    fn leave_recursion(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or_else(|| {
            // Static fallback — initialized once
            static EOF: std::sync::OnceLock<Token> = std::sync::OnceLock::new();
            EOF.get_or_init(|| Token::new(TokenKind::Eof, 0, 0, 0, 0))
        })
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    fn advance(&mut self) -> Token {
        if self.pos >= self.tokens.len() {
            return Token::new(TokenKind::Eof, 0, 0, 0, 0);
        }
        // Clone the token instead of replacing it with a placeholder.
        // The placeholder optimisation broke speculative parsing: when a
        // directive parse (e.g. @for) fails, the caller restores `self.pos`
        // and retries as text — but the tokens at the restored position had
        // already been overwritten with PlaceholderDummy, so the fallback
        // text collector saw garbage.
        let token = self.tokens[self.pos].clone();
        self.pos += 1;
        token
    }

    /// Zero-clone variant check. Returns true if current token matches `kind`.
    fn at(&self, kind: &TokenKind) -> bool {
        self.tokens.get(self.pos).is_some_and(|t| &t.kind == kind)
    }

    /// Check-and-consume. Returns true and advances if current token matches `kind`.
    fn advance_if(&mut self, kind: &TokenKind) -> bool {
        if self.at(kind) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    // ── Helpers ────────────────────────────────────────

    /// Returns true if the parser is at the end of the token stream.
    pub fn at_eof(&self) -> bool {
        self.peek_kind() == &TokenKind::Eof
    }

    /// Peek at the string value of the next identifier without consuming it.
    /// Handles both TokenKind::Ident(name) and keyword tokens via keyword_as_ident_name().
    pub fn peek_ident_value(&self) -> Option<String> {
        let tok = self.peek();
        match &tok.kind {
            TokenKind::Ident(name) => Some(name.clone()),
            other => keyword_as_ident_name(other).map(|s| s.to_string()),
        }
    }

    /// Check if the next token is a left brace `{`.
    pub fn peek_is_brace(&self) -> bool {
        self.at(&TokenKind::LBrace)
    }

    /// Consume a semicolon if present (optional semicolon).
    pub fn consume_semicolon(&mut self) {
        self.advance_if(&TokenKind::Semicolon);
    }

    /// Expect and return a string literal token.
    pub fn expect_string(&mut self, msg: &str) -> Result<String, ParseError> {
        let t = self.advance();
        match &t.kind {
            TokenKind::StringLiteral(s) => Ok(s.clone()),
            _ => Err(ParseError {
                message: format!("{} — found {:?}", msg, t.kind),
                span: t.span,
            }),
        }
    }

    /// Construct a ParseError at the current peek position.
    pub fn error(&self, msg: impl Into<String>) -> ParseError {
        ParseError {
            message: msg.into(),
            span: self.peek_span(),
        }
    }

    // ── Error recovery helpers ──────────────────────────
    // These are intentionally unused until error recovery logic is activated.

    /// Record a parse error (non-fatal — parsing continues).
    fn emit_error(&mut self, err: ParseError) {
        self.errors.push(err);
    }

    /// Skip tokens until one of `expected` is found (or EOF).
    /// Returns number of tokens skipped. Does NOT consume the found token.
    fn skip_to(&mut self, expected: &[TokenKind]) -> usize {
        let start = self.pos;
        loop {
            let kind = self.peek_kind();
            if expected.contains(kind) || *kind == TokenKind::Eof {
                break;
            }
            self.advance();
        }
        self.pos - start
    }

    /// Skip to a statement boundary: ';' (consumed), '}', or EOF.
    fn skip_to_stmt_boundary(&mut self) {
        loop {
            match self.peek_kind() {
                TokenKind::Semicolon => {
                    self.advance();
                    return;
                }
                TokenKind::RBrace | TokenKind::Eof => return,
                _ => {
                    self.advance();
                }
            }
        }
    }

    /// Recover from a parse error during statement parsing.
    /// Fixes the recursion depth (leave_recursion was skipped) and skips
    /// to the next statement boundary so parsing can continue.
    fn recover_from_error(&mut self) {
        self.depth = self.depth.saturating_sub(1);
        // Only skip if we're not already at a safe restart position
        // (statement boundary, statement-start keyword, or EOF).
        // This prevents overshooting when the failed parser already consumed
        // the boundary token (e.g. parse_prefix() advances before erroring).
        if !self.at_stmt_restart() {
            self.skip_to_stmt_boundary();
        }
    }

    /// Returns true if the current token is a safe position to resume
    /// statement parsing: either a boundary token, a keyword that starts
    /// a new statement, or a token that can start an expression statement.
    fn at_stmt_restart(&self) -> bool {
        matches!(
            self.peek_kind(),
            TokenKind::Semicolon
                | TokenKind::RBrace
                | TokenKind::Eof
                | TokenKind::Let
                | TokenKind::If
                | TokenKind::While
                | TokenKind::For
                | TokenKind::Return
                | TokenKind::Break
                | TokenKind::Continue
                | TokenKind::Print
                | TokenKind::Println
                | TokenKind::Throw
                | TokenKind::Try
                | TokenKind::Emit
                | TokenKind::LBrace
                | TokenKind::Ident(_)
                | TokenKind::IntLiteral(_)
                | TokenKind::FloatLiteral(_)
                | TokenKind::StringLiteral(_)
                | TokenKind::TemplateString(_)
                | TokenKind::True
                | TokenKind::False
                | TokenKind::Null
                | TokenKind::LParen
                | TokenKind::Minus
                | TokenKind::Not
                | TokenKind::Await
                | TokenKind::Match
                | TokenKind::Mut
        )
    }

    /// Skip to next top-level declaration keyword.
    fn skip_to_top_level(&mut self) {
        const TOP_LEVEL: &[TokenKind] = &[
            TokenKind::Fn,
            TokenKind::Pub,
            TokenKind::Component,
            TokenKind::Store,
            TokenKind::Design,
            TokenKind::Struct,
            TokenKind::Type,
            TokenKind::Const,
            TokenKind::Import,
            TokenKind::Export,
            TokenKind::AtMotion,
            TokenKind::AtTimeline,
            TokenKind::Async,
        ];
        self.skip_to(TOP_LEVEL);
    }
}

#[cfg(test)]
mod roundtrip_tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::lexer::Span;
    use crate::parser::ast::*;

    // ── Helpers ────────────────────────────────────────────

    /// Parse source as a function body and return the first statement.
    /// Appends a semicolon to prevent the expression from becoming a trailing expression.
    fn first_stmt(source: &str) -> Stmt {
        // Wrap with semicolon to ensure it goes into stmts[], not trailing_expr
        let wrapped = format!("pub fn test() {{ {}; }}", source);
        let tokens = tokenize(&wrapped);
        let program = parse(tokens, &wrapped).unwrap();
        let module = program.modules.get("input.omni").unwrap();
        let body = &module.functions[0].body;
        if !body.stmts.is_empty() {
            body.stmts[0].clone()
        } else if let Some(expr) = &body.trailing_expr {
            // Even with semicolon, some constructs end up as trailing_expr
            Stmt::ExprStmt {
                expr: *expr.clone(),
                span: Span {
                    start: 0,
                    end: 0,
                    line: 0,
                    col: 0,
                },
            }
        } else {
            panic!("function body is empty for source: {}", wrapped);
        }
    }

    /// Parse source as a function body, return the ExprStmt's inner expression.
    fn first_expr(source: &str) -> Expr {
        let e = match first_stmt(source) {
            Stmt::ExprStmt { expr, .. } => expr,
            other => panic!("expected ExprStmt, got {:?}", other),
        };
        e
    }

    /// Parse a component definition and return its view children.
    fn view_children(source: &str) -> Vec<ViewNode> {
        let tokens = tokenize(source);
        let program = parse(tokens, source).unwrap();
        let module = program.modules.get("input.omni").unwrap();
        module.components[0]
            .view
            .as_ref()
            .expect("component should have view")
            .children
            .clone()
    }

    // ════════════════════════════════════════════════════════
    // Expression parsing tests
    // ════════════════════════════════════════════════════════

    // ── Binary expressions ────────────────────────────────

    #[test]
    fn expr_binary_add() {
        let expr = first_expr("1 + 2");
        assert!(matches!(
            expr,
            Expr::BinaryOp {
                op: BinaryOpKind::Add,
                ..
            }
        ));
        if let Expr::BinaryOp { left, right, .. } = &expr {
            assert!(matches!(**left, Expr::Literal(Literal::Int(1), _)));
            assert!(matches!(**right, Expr::Literal(Literal::Int(2), _)));
        }
    }

    #[test]
    fn expr_binary_mul() {
        let expr = first_expr("a * b");
        assert!(matches!(
            expr,
            Expr::BinaryOp {
                op: BinaryOpKind::Mul,
                ..
            }
        ));
    }

    #[test]
    fn expr_binary_sub() {
        let expr = first_expr("a - b");
        assert!(matches!(
            expr,
            Expr::BinaryOp {
                op: BinaryOpKind::Sub,
                ..
            }
        ));
    }

    #[test]
    fn expr_binary_div() {
        let expr = first_expr("a / b");
        assert!(matches!(
            expr,
            Expr::BinaryOp {
                op: BinaryOpKind::Div,
                ..
            }
        ));
    }

    #[test]
    fn expr_binary_comparison() {
        let expr = first_expr("a == b");
        assert!(matches!(
            expr,
            Expr::BinaryOp {
                op: BinaryOpKind::Eq,
                ..
            }
        ));
        let expr = first_expr("a != b");
        assert!(matches!(
            expr,
            Expr::BinaryOp {
                op: BinaryOpKind::Neq,
                ..
            }
        ));
        let expr = first_expr("a < b");
        assert!(matches!(
            expr,
            Expr::BinaryOp {
                op: BinaryOpKind::Lt,
                ..
            }
        ));
        let expr = first_expr("a > b");
        assert!(matches!(
            expr,
            Expr::BinaryOp {
                op: BinaryOpKind::Gt,
                ..
            }
        ));
    }

    #[test]
    fn expr_binary_logical() {
        let expr = first_expr("a && b");
        assert!(matches!(
            expr,
            Expr::BinaryOp {
                op: BinaryOpKind::And,
                ..
            }
        ));
        let expr = first_expr("a || b");
        assert!(matches!(
            expr,
            Expr::BinaryOp {
                op: BinaryOpKind::Or,
                ..
            }
        ));
    }

    #[test]
    fn expr_precedence_mul_over_add() {
        // a * b + c should parse as ((a * b) + c)
        let expr = first_expr("a * b + c");
        assert!(matches!(
            expr,
            Expr::BinaryOp {
                op: BinaryOpKind::Add,
                ..
            }
        ));
        if let Expr::BinaryOp { left, right, .. } = &expr {
            assert!(matches!(
                **left,
                Expr::BinaryOp {
                    op: BinaryOpKind::Mul,
                    ..
                }
            ));
            assert!(matches!(**right, Expr::VarRef(_, _)));
            if let Expr::VarRef(name, _) = &**right {
                assert_eq!(name, "c");
            }
        }
    }

    #[test]
    fn expr_precedence_add_over_comparison() {
        // a + b == c + d should parse as ((a + b) == (c + d))
        let expr = first_expr("a + b == c + d");
        assert!(matches!(
            expr,
            Expr::BinaryOp {
                op: BinaryOpKind::Eq,
                ..
            }
        ));
        if let Expr::BinaryOp { left, right, .. } = &expr {
            assert!(matches!(
                **left,
                Expr::BinaryOp {
                    op: BinaryOpKind::Add,
                    ..
                }
            ));
            assert!(matches!(
                **right,
                Expr::BinaryOp {
                    op: BinaryOpKind::Add,
                    ..
                }
            ));
        }
    }

    #[test]
    fn expr_precedence_parenthesized() {
        // (a + b) * c should parse as ((a + b) * c), overriding default precedence
        let expr = first_expr("(a + b) * c");
        assert!(matches!(
            expr,
            Expr::BinaryOp {
                op: BinaryOpKind::Mul,
                ..
            }
        ));
        if let Expr::BinaryOp { left, .. } = &expr {
            assert!(matches!(
                **left,
                Expr::BinaryOp {
                    op: BinaryOpKind::Add,
                    ..
                }
            ));
        }
    }

    // ── Unary expressions ──────────────────────────────────

    #[test]
    fn expr_unary_not() {
        let expr = first_expr("!flag");
        assert!(matches!(
            expr,
            Expr::UnaryOp {
                op: UnaryOpKind::Not,
                ..
            }
        ));
    }

    #[test]
    fn expr_unary_neg() {
        let expr = first_expr("-value");
        assert!(matches!(
            expr,
            Expr::UnaryOp {
                op: UnaryOpKind::Neg,
                ..
            }
        ));
    }

    #[test]
    fn expr_unary_await() {
        let expr = first_expr("await promise");
        assert!(matches!(expr, Expr::AwaitExpr { .. }));
        if let Expr::AwaitExpr { operand, .. } = &expr {
            assert!(matches!(**operand, Expr::VarRef(_, _)));
        }
    }

    // ── Function calls ─────────────────────────────────────

    #[test]
    fn expr_call_no_args() {
        let expr = first_expr("foo()");
        assert!(matches!(
            expr,
            Expr::FuncCall { ref args, .. } if args.is_empty()
        ));
        if let Expr::FuncCall { func, .. } = &expr {
            assert!(matches!(**func, Expr::VarRef(_, _)));
        }
    }

    #[test]
    fn expr_call_with_args() {
        let expr = first_expr("bar(1, \"two\")");
        assert!(matches!(
            expr,
            Expr::FuncCall { ref args, .. } if args.len() == 2
        ));
        if let Expr::FuncCall { args, .. } = &expr {
            assert!(matches!(args[0], Expr::Literal(Literal::Int(1), _)));
            assert!(matches!(&args[1], Expr::Literal(Literal::String(s), _) if s == "two"));
        }
    }

    #[test]
    fn expr_call_nested() {
        let expr = first_expr("foo(bar(x))");
        assert!(matches!(
            expr,
            Expr::FuncCall { ref args, .. } if args.len() == 1
        ));
        if let Expr::FuncCall { args, .. } = &expr {
            assert!(matches!(&args[0], Expr::FuncCall { .. }));
        }
    }

    // ── Member access ──────────────────────────────────────

    #[test]
    fn expr_field_access() {
        let expr = first_expr("obj.field");
        assert!(matches!(
            expr,
            Expr::FieldAccess { ref field, .. } if field == "field"
        ));
        if let Expr::FieldAccess { object, .. } = &expr {
            assert!(matches!(**object, Expr::VarRef(_, _)));
        }
    }

    #[test]
    fn expr_method_call() {
        let expr = first_expr("obj.method()");
        assert!(matches!(
            expr,
            Expr::MethodCall { ref method, ref args, .. }
                if method == "method" && args.is_empty()
        ));
    }

    #[test]
    fn expr_method_call_with_args() {
        let expr = first_expr("obj.method(a, b)");
        assert!(matches!(
            expr,
            Expr::MethodCall { ref method, ref args, .. }
                if method == "method" && args.len() == 2
        ));
    }

    #[test]
    fn expr_chained_field_access() {
        let expr = first_expr("a.b.c");
        // Should parse as FieldAccess { object: FieldAccess { object: VarRef("a"), field: "b" }, field: "c" }
        assert!(matches!(
            expr,
            Expr::FieldAccess { ref field, .. } if field == "c"
        ));
        if let Expr::FieldAccess { object, .. } = &expr {
            assert!(matches!(**object, Expr::FieldAccess { ref field, .. } if field == "b"));
        }
    }

    // ── Index access ───────────────────────────────────────

    #[test]
    fn expr_index_int() {
        let expr = first_expr("arr[0]");
        assert!(matches!(expr, Expr::IndexGet { .. }));
        if let Expr::IndexGet { object, index, .. } = &expr {
            assert!(matches!(**object, Expr::VarRef(_, _)));
            assert!(matches!(**index, Expr::Literal(Literal::Int(0), _)));
        }
    }

    #[test]
    fn expr_index_string() {
        let expr = first_expr("map[\"key\"]");
        assert!(matches!(expr, Expr::IndexGet { .. }));
        if let Expr::IndexGet { index, .. } = &expr {
            assert!(matches!(&**index, Expr::Literal(Literal::String(s), _) if s == "key"));
        }
    }

    #[test]
    fn expr_index_var() {
        let expr = first_expr("map[idx]");
        if let Expr::IndexGet { object, index, .. } = &expr {
            assert!(matches!(**object, Expr::VarRef(_, _)));
            assert!(matches!(**index, Expr::VarRef(_, _)));
        }
    }

    // ── String templates ───────────────────────────────────

    #[test]
    fn expr_template_string_simple() {
        // Template strings without interpolation are parsed as TemplateString
        // with a single Literal part.
        let expr = first_expr("`hello world`");
        assert!(matches!(
            expr,
            Expr::TemplateString { ref parts, .. }
                if parts.len() == 1
                && matches!(parts[0], TemplatePart::Literal(ref s) if s == "hello world")
        ));
    }

    // ── Array literals ─────────────────────────────────────

    #[test]
    fn expr_array_literal_empty() {
        let expr = first_expr("[]");
        assert!(matches!(
            expr,
            Expr::ArrayLiteral { ref elements, .. } if elements.is_empty()
        ));
    }

    #[test]
    fn expr_array_literal_numbers() {
        let expr = first_expr("[1, 2]");
        assert!(matches!(
            expr,
            Expr::ArrayLiteral { ref elements, .. } if elements.len() == 2
        ));
        if let Expr::ArrayLiteral { elements, .. } = &expr {
            assert!(matches!(elements[0], Expr::Literal(Literal::Int(1), _)));
            assert!(matches!(elements[1], Expr::Literal(Literal::Int(2), _)));
        }
    }

    #[test]
    fn expr_array_literal_trailing_comma() {
        let expr = first_expr("[1,]");
        assert!(matches!(
            expr,
            Expr::ArrayLiteral { ref elements, .. } if elements.len() == 1
        ));
    }

    #[test]
    fn expr_array_literal_mixed() {
        let expr = first_expr("[1, \"two\", x]");
        if let Expr::ArrayLiteral { elements, .. } = &expr {
            assert_eq!(elements.len(), 3);
            assert!(matches!(elements[0], Expr::Literal(Literal::Int(1), _)));
            assert!(matches!(&elements[1], Expr::Literal(Literal::String(s), _) if s == "two"));
            assert!(matches!(elements[2], Expr::VarRef(_, _)));
        }
    }

    // ── Object literals ────────────────────────────────────

    #[test]
    fn expr_map_literal_simple() {
        // Use let binding to put {a: 1} in expression context (not statement context)
        let stmt = first_stmt("let _m = {a: 1}");
        if let Stmt::VarDecl {
            init: Some(expr), ..
        } = &stmt
        {
            assert!(matches!(*expr, Expr::MapLiteral { .. }));
            if let Expr::MapLiteral { entries, .. } = expr {
                assert!(!entries.is_empty());
            }
        } else {
            panic!("expected VarDecl with map literal init");
        }
    }

    #[test]
    fn expr_map_literal_multi() {
        let stmt = first_stmt("let _m = {a: 1, b: 2}");
        if let Stmt::VarDecl {
            init: Some(expr), ..
        } = &stmt
        {
            if let Expr::MapLiteral { entries, .. } = expr {
                assert_eq!(entries.len(), 2);
            }
        } else {
            panic!("expected VarDecl with map literal init");
        }
    }

    #[test]
    fn expr_map_literal_shorthand() {
        // Shorthand { x } should be accepted; keys are converted to string literals
        let stmt = first_stmt("let _a = 1; let _m = {_a}");
        let body = {
            // Parse directly to handle multi-stmt
            let wrapped = "pub fn test() { let _a = 1; let _m = {_a} }";
            let tokens = tokenize(wrapped);
            let program = parse(tokens, wrapped).unwrap();
            let module = program.modules.get("input.omni").unwrap();
            module.functions[0].body.stmts[1].clone()
        };
        if let Stmt::VarDecl {
            init: Some(expr), ..
        } = &body
        {
            assert!(matches!(*expr, Expr::MapLiteral { .. }));
        } else {
            panic!("expected VarDecl with map literal init");
        }
    }

    // ── Variable declarations ──────────────────────────────

    #[test]
    fn stmt_var_decl_init() {
        let stmt = first_stmt("let x = 5");
        assert!(matches!(
            stmt,
            Stmt::VarDecl { ref name, ref init, mutable: false, .. }
                if name == "x" && init.is_some()
        ));
    }

    #[test]
    fn stmt_var_decl_no_init() {
        let stmt = first_stmt("let x: i32");
        assert!(matches!(
            stmt,
            Stmt::VarDecl { ref name, ref init, .. }
                if name == "x" && init.is_none()
        ));
    }

    #[test]
    fn stmt_var_decl_mutable() {
        let stmt = first_stmt("let mut x = 10");
        assert!(matches!(
            stmt,
            Stmt::VarDecl { ref name, mutable: true, .. } if name == "x"
        ));
    }

    #[test]
    fn stmt_var_decl_type_annotation() {
        let stmt = first_stmt("let x: String = \"hello\"");
        assert!(matches!(
            stmt,
            Stmt::VarDecl { ref name, ref type_ann, .. } if name == "x" && type_ann.is_some()
        ));
    }

    // ── Assignments ────────────────────────────────────────

    #[test]
    fn stmt_assignment() {
        let stmt = first_stmt("x = 10");
        assert!(matches!(
            stmt,
            Stmt::ExprStmt { ref expr, .. }
        ));
        if let Stmt::ExprStmt { expr, .. } = &stmt {
            assert!(matches!(
                *expr,
                Expr::BinaryOp {
                    op: BinaryOpKind::Assign,
                    ..
                }
            ));
        }
    }

    #[test]
    fn stmt_compound_assign_add() {
        let stmt = first_stmt("x += 1");
        if let Stmt::ExprStmt { expr, .. } = &stmt {
            assert!(matches!(
                *expr,
                Expr::BinaryOp {
                    op: BinaryOpKind::AddAssign,
                    ..
                }
            ));
        }
    }

    #[test]
    fn stmt_compound_assign_sub() {
        let stmt = first_stmt("x -= 1");
        if let Stmt::ExprStmt { expr, .. } = &stmt {
            assert!(matches!(
                *expr,
                Expr::BinaryOp {
                    op: BinaryOpKind::SubAssign,
                    ..
                }
            ));
        }
    }

    // ── If statements ──────────────────────────────────────

    #[test]
    fn stmt_if_simple() {
        let stmt = first_stmt("if (true) { 42 }");
        assert!(matches!(stmt, Stmt::If { ref else_body, ref else_ifs, .. }
            if else_body.is_none() && else_ifs.is_empty()
        ));
        if let Stmt::If {
            condition,
            then_body,
            ..
        } = &stmt
        {
            assert!(matches!(condition, Expr::Literal(Literal::Bool(true), _)));
            assert_eq!(then_body.stmts.len(), 0);
            assert!(then_body.trailing_expr.is_some());
        }
    }

    #[test]
    fn stmt_if_else() {
        let stmt = first_stmt("if (a) { b } else { c }");
        if let Stmt::If { else_body, .. } = &stmt {
            assert!(else_body.is_some());
        }
    }

    #[test]
    fn stmt_if_else_if() {
        let stmt = first_stmt("if (a) { b } else if (c) { d } else { e }");
        if let Stmt::If {
            else_ifs,
            else_body,
            ..
        } = &stmt
        {
            assert!(!else_ifs.is_empty());
            assert!(else_body.is_some());
        }
    }

    // ── While loops ────────────────────────────────────────

    #[test]
    fn stmt_while_loop() {
        let stmt = first_stmt("while (true) { break }");
        assert!(matches!(stmt, Stmt::While { .. }));
        if let Stmt::While {
            condition, body, ..
        } = &stmt
        {
            assert!(matches!(condition, Expr::Literal(Literal::Bool(true), _)));
            assert!(!body.stmts.is_empty());
        }
    }

    // ── For loops ──────────────────────────────────────────

    #[test]
    fn stmt_for_in() {
        let stmt = first_stmt("for item in items { print item }");
        assert!(matches!(stmt, Stmt::ForIn { ref var_name, .. } if var_name == "item"));
        if let Stmt::ForIn { iterable, body, .. } = &stmt {
            assert!(matches!(iterable, Expr::VarRef(_, _)));
            assert!(!body.stmts.is_empty());
        }
    }

    // ── Return / break / continue ──────────────────────────

    #[test]
    fn stmt_return_value() {
        let stmt = first_stmt("return 42");
        assert!(matches!(stmt, Stmt::Return { ref value, .. }
            if value.is_some()
        ));
    }

    #[test]
    fn stmt_return_void() {
        let stmt = first_stmt("return");
        assert!(matches!(stmt, Stmt::Return { ref value, .. }
            if value.is_none()
        ));
    }

    #[test]
    fn stmt_break() {
        let stmt = first_stmt("break");
        assert!(matches!(stmt, Stmt::Break(_)));
    }

    #[test]
    fn stmt_continue() {
        let stmt = first_stmt("continue");
        assert!(matches!(stmt, Stmt::Continue(_)));
    }

    // ── Print ──────────────────────────────────────────────

    #[test]
    fn stmt_print() {
        let stmt = first_stmt("print \"hello\"");
        assert!(matches!(stmt, Stmt::Print { newline: false, .. }));
        if let Stmt::Print { arg, .. } = &stmt {
            assert!(matches!(arg, Expr::Literal(Literal::String(s), _) if s == "hello"));
        }
    }

    #[test]
    fn stmt_println() {
        let stmt = first_stmt("println \"hello\"");
        assert!(matches!(stmt, Stmt::Print { newline: true, .. }));
    }

    // ── Try / catch / throw ────────────────────────────────

    #[test]
    fn stmt_throw() {
        let stmt = first_stmt("throw \"error\"");
        assert!(matches!(stmt, Stmt::Throw { .. }));
    }

    #[test]
    fn stmt_try_catch() {
        let stmt = first_stmt("try { 1 } catch { 2 }");
        assert!(matches!(stmt, Stmt::TryCatch { .. }));
        if let Stmt::TryCatch { catch_var, .. } = &stmt {
            assert!(catch_var.is_none());
        }
    }

    #[test]
    fn stmt_try_catch_with_var() {
        let stmt = first_stmt("try { 1 } catch e { 2 }");
        if let Stmt::TryCatch { catch_var, .. } = &stmt {
            assert_eq!(catch_var.as_deref(), Some("e"));
        }
    }

    #[test]
    fn stmt_try_catch_finally() {
        let stmt = first_stmt("try { 1 } catch { 2 } finally { 3 }");
        assert!(matches!(
            stmt,
            Stmt::TryCatch {
                finally_body: Some(..),
                ..
            }
        ));
    }

    // ── Emit ───────────────────────────────────────────────

    #[test]
    fn stmt_emit_no_args() {
        let stmt = first_stmt("emit done()");
        assert!(matches!(stmt, Stmt::EventEmit { ref name, ref args, .. }
            if name == "done" && args.is_empty()
        ));
    }

    #[test]
    fn stmt_emit_with_args() {
        let stmt = first_stmt("emit update(x, y)");
        if let Stmt::EventEmit { name, args, .. } = &stmt {
            assert_eq!(name, "update");
            assert_eq!(args.len(), 2);
        }
    }

    // ════════════════════════════════════════════════════════
    // View parsing tests
    // ════════════════════════════════════════════════════════

    #[test]
    fn view_simple_element_text() {
        let children = view_children("component App { view { Text { \"hello\" } } }");
        assert_eq!(children.len(), 1);
        match &children[0] {
            ViewNode::Element(el) => {
                assert_eq!(el.tag, "Text");
                // Text { "hello" } is NOT self-closing — it has a children block
                assert!(!el.self_closing);
                assert_eq!(el.children.len(), 1);
                assert!(matches!(&el.children[0], ViewNode::Text(s) if s == "hello"));
            }
            _ => panic!("expected Element"),
        }
    }

    #[test]
    fn view_element_with_props() {
        // Component call syntax: Button(label: "Click") — props in parentheses
        let children = view_children("component App { view { Button(label: \"Click\") } }");
        assert_eq!(children.len(), 1);
        match &children[0] {
            ViewNode::Element(el) => {
                assert_eq!(el.tag, "Button");
                assert_eq!(el.props.len(), 1);
                match &el.props[0] {
                    ElementProp::Static { name, value } => {
                        assert_eq!(name, "label");
                        assert!(
                            matches!(value, Expr::Literal(Literal::String(s), _) if s == "Click")
                        );
                    }
                    _ => panic!("expected Static prop"),
                }
            }
            _ => panic!("expected Element"),
        }
    }

    #[test]
    fn view_nested_elements() {
        let children =
            view_children("component App { view { View { Text { \"a\" } Text { \"b\" } } } }");
        assert_eq!(children.len(), 1);
        match &children[0] {
            ViewNode::Element(el) => {
                assert_eq!(el.tag, "View");
                assert_eq!(el.children.len(), 2);
                assert!(matches!(&el.children[0], ViewNode::Element(child) if child.tag == "Text"));
                assert!(matches!(&el.children[1], ViewNode::Element(child) if child.tag == "Text"));
            }
            _ => panic!("expected Element"),
        }
    }

    #[test]
    fn view_html_element() {
        let children =
            view_children("component App { view { <div class=\"container\">Text content</div> } }");
        assert_eq!(children.len(), 1);
        match &children[0] {
            ViewNode::Element(el) => {
                assert_eq!(el.tag, "div");
                assert!(!el.self_closing);
            }
            _ => panic!("expected Element"),
        }
    }

    #[test]
    fn view_self_closing_html_element() {
        let children = view_children("component App { view { <br/> } }");
        assert_eq!(children.len(), 1);
        match &children[0] {
            ViewNode::Element(el) => {
                assert_eq!(el.tag, "br");
                assert!(el.self_closing);
            }
            _ => panic!("expected Element"),
        }
    }

    #[test]
    fn view_if_directive() {
        let children =
            view_children("component App { view { @if (show) { Text { \"visible\" } } } }");
        assert_eq!(children.len(), 1);
        assert!(matches!(&children[0], ViewNode::IfDirective(_)));
        if let ViewNode::IfDirective(dir) = &children[0] {
            assert_eq!(dir.body.len(), 1);
        }
    }

    #[test]
    fn view_for_directive() {
        // @for does NOT use parentheses — syntax: @for item in items { body }
        let children =
            view_children("component App { view { @for item in items { Text { item } } } }");
        assert_eq!(children.len(), 1);
        assert!(matches!(&children[0], ViewNode::ForDirective(_)));
        if let ViewNode::ForDirective(dir) = &children[0] {
            assert_eq!(dir.item_name, "item");
            assert_eq!(dir.body.len(), 1);
        }
    }

    #[test]
    fn view_interpolation() {
        let children = view_children("component App { view { Text { {name} } } }");
        assert_eq!(children.len(), 1);
        match &children[0] {
            ViewNode::Element(el) => {
                assert_eq!(el.children.len(), 1);
                assert!(matches!(&el.children[0], ViewNode::Interpolation(_)));
            }
            _ => panic!("expected Element"),
        }
    }

    #[test]
    fn view_component_call_syntax() {
        let children = view_children(
            "component App { view { Card(title: \"Hello\") { Text { \"body\" } } } }",
        );
        assert_eq!(children.len(), 1);
        match &children[0] {
            ViewNode::Element(el) => {
                assert_eq!(el.tag, "Card");
                assert_eq!(el.props.len(), 1);
                assert!(!el.self_closing);
                assert_eq!(el.children.len(), 1);
            }
            _ => panic!("expected Element"),
        }
    }

    #[test]
    fn view_element_prop_types() {
        let children = view_children(
            "component App { view { <button id=\"btn\" disabled={true}>click</button> } }",
        );
        assert_eq!(children.len(), 1);
        match &children[0] {
            ViewNode::Element(el) => {
                assert_eq!(el.tag, "button");
                assert_eq!(el.props.len(), 2);
                // Check string prop
                match &el.props[0] {
                    ElementProp::Static { name, value } => {
                        assert_eq!(name, "id");
                        assert!(
                            matches!(value, Expr::Literal(Literal::String(s), _) if s == "btn")
                        );
                    }
                    _ => panic!("expected Static prop"),
                }
                // Check expression prop
                match &el.props[1] {
                    ElementProp::Static { name, value } => {
                        assert_eq!(name, "disabled");
                        assert!(matches!(value, Expr::Literal(Literal::Bool(true), _)));
                    }
                    _ => panic!("expected Static prop"),
                }
            }
            _ => panic!("expected Element"),
        }
    }

    #[test]
    fn view_text_only() {
        let children = view_children("component App { view { Text { \"hello world\" } } }");
        assert_eq!(children.len(), 1);
        match &children[0] {
            ViewNode::Element(el) => {
                assert_eq!(el.tag, "Text");
                assert_eq!(el.children.len(), 1);
                assert!(matches!(&el.children[0], ViewNode::Text(s) if s == "hello world"));
            }
            _ => panic!("expected Element"),
        }
    }

    #[test]
    fn view_string_literal_prop() {
        let children =
            view_children("component App { view { Input(placeholder: \"Enter name\") } }");
        assert_eq!(children.len(), 1);
        match &children[0] {
            ViewNode::Element(el) => {
                assert_eq!(el.tag, "Input");
                assert!(el.self_closing);
                assert_eq!(el.props.len(), 1);
                match &el.props[0] {
                    ElementProp::Static { name, value } => {
                        assert_eq!(name, "placeholder");
                        assert!(
                            matches!(value, Expr::Literal(Literal::String(s), _) if s == "Enter name")
                        );
                    }
                    _ => panic!("expected Static prop"),
                }
            }
            _ => panic!("expected Element"),
        }
    }

    // ════════════════════════════════════════════════════════
    // Original roundtrip tests (preserved)
    // ════════════════════════════════════════════════════════

    #[test]
    fn roundtrip_function_def() {
        let source = "pub fn main() { return 42; }";
        let tokens = tokenize(source);
        let program = parse(tokens, source).unwrap();
        assert_eq!(program.modules.len(), 1);
        let module = program.modules.get("input.omni").unwrap();
        assert_eq!(module.functions.len(), 1);
        let func = &module.functions[0];
        assert_eq!(func.name, "main");
        assert!(matches!(func.visibility, Visibility::Public));
    }

    #[test]
    fn roundtrip_struct_def() {
        let source = "struct Point { x: i32, y: i32 }";
        let tokens = tokenize(source);
        let program = parse(tokens, source).unwrap();
        let module = program.modules.get("input.omni").unwrap();
        assert_eq!(module.structs.len(), 1);
        assert_eq!(module.structs[0].name, "Point");
        assert_eq!(module.structs[0].fields.len(), 2);
    }

    #[test]
    fn roundtrip_enum_def() {
        let source = "type Color = Red | Green | Blue";
        let tokens = tokenize(source);
        let program = parse(tokens, source).unwrap();
        let module = program.modules.get("input.omni").unwrap();
        assert_eq!(module.tagged_unions.len(), 1);
    }

    #[test]
    fn roundtrip_type_alias() {
        let source = "type UserId = i32;";
        let tokens = tokenize(source);
        let program = parse(tokens, source).unwrap();
        let module = program.modules.get("input.omni").unwrap();
        assert_eq!(module.type_aliases.len(), 1);
        assert_eq!(module.type_aliases[0].name, "UserId");
    }

    #[test]
    fn roundtrip_import_export() {
        let source = "import { add } from \"./utils/math.omni\";\nexport { main } from \"./utils/math.omni\";";
        let tokens = tokenize(source);
        let program = parse(tokens, source).unwrap();
        let module = program.modules.get("input.omni").unwrap();
        assert_eq!(module.imports.len(), 1);
        assert_eq!(module.exports.len(), 1);
    }

    #[test]
    fn test_reject_invalid_hex() {
        let source = "let x = 0xFFG;";
        let tokens = tokenize(source);
        let result = parse(tokens, source);
        assert!(
            result.is_err(),
            "Invalid hex '0xFFG' should produce parse error"
        );
    }

    #[test]
    fn test_reject_invalid_float() {
        let source = "let x = 1.2.3;";
        let tokens = tokenize(source);
        let result = parse(tokens, source);
        assert!(
            result.is_err(),
            "Invalid float '1.2.3' should produce parse error"
        );
    }

    // ── Error recovery tests ────────────────────────────

    #[test]
    fn error_recovery_mid_function() {
        // Syntax error mid-function should not prevent the rest from parsing.
        // `let x = ;` fails at expression parsing; recovery skips to `;`
        // boundary so `let y = 42;` and `return y;` are still parsed.
        let source = "pub fn main() { let x = ; let y = 42; return y; }";
        let tokens = tokenize(source);
        let mut parser = Parser::new(tokens, source);
        let module = parser.parse_module().unwrap();
        assert_eq!(
            parser.errors.len(),
            1,
            "exactly one parse error should be collected"
        );
        assert_eq!(module.functions.len(), 1, "function should still parse");
        assert_eq!(
            module.functions[0].body.stmts.len(),
            2,
            "function body should contain 2 valid statements after recovery"
        );
    }

    #[test]
    fn error_recovery_garbage_between_functions() {
        // Garbage between top-level declarations should be skipped,
        // allowing both adjacent functions to parse.
        let source = "pub fn foo() { return 1; } quux pub fn bar() { return 2; }";
        let tokens = tokenize(source);
        let mut parser = Parser::new(tokens, source);
        let module = parser.parse_module().unwrap();
        assert_eq!(
            parser.errors.len(),
            1,
            "exactly one parse error should be collected for the garbage ident"
        );
        assert_eq!(
            module.functions.len(),
            2,
            "both functions should parse despite intervening garbage"
        );
        assert_eq!(module.functions[0].name, "foo");
        assert_eq!(module.functions[1].name, "bar");
    }
}
