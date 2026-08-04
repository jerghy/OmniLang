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

/// Parse a token stream into an AST Program and all collected parse errors.
/// Always returns a Program (possibly incomplete if errors exist).
/// Returns an empty Vec on clean parse.
pub fn parse(tokens: Vec<Token>, source: &str) -> (Program, Vec<ParseError>) {
    let mut parser = Parser::new(tokens, source);
    let module = parser.parse_module();
    let mut modules = FxHashMap::default();
    modules.insert("input.omni".to_string(), module);
    (Program { modules }, parser.errors)
}

/// Maximum recursion depth for the parser to prevent stack overflow.
///
/// 40 (not 256): the guard must fire BEFORE the stack runs out, and the
/// worst Pratt path (a unary chain) costs ~17 KB per level in debug builds.
/// Measured on a ~1 MiB Windows main-thread stack (what `omnic` runs on):
/// ~57 levels survive, 60 overflow — so the guard fires at 39, leaving a
/// ~34% margin. 39 nested expression levels is far beyond any real source.
const MAX_RECURSION_DEPTH: usize = 40;

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
                kind: None,
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
                kind: None,
                message: format!("{} — found {:?}", msg, t.kind),
                span: t.span,
            }),
        }
    }

    /// Construct a ParseError at the current peek position.
    pub fn error(&self, msg: impl Into<String>) -> ParseError {
        ParseError {
            kind: None,
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
        let (program, _) = parse(tokens, &wrapped);
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
        let (program, _) = parse(tokens, source);
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
            let (program, _) = parse(tokens, wrapped);
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
        assert!(matches!(stmt, Stmt::ExprStmt { expr: _, .. }));
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
            assert!(!dir.scroll_hint);
        }
    }

    #[test]
    fn view_slot_directive_plain() {
        let children = view_children("component App { view { @slot header } }");
        assert_eq!(children.len(), 1);
        match &children[0] {
            ViewNode::SlotDirective {
                name,
                default_content,
            } => {
                assert_eq!(name.as_deref(), Some("header"));
                assert!(default_content.is_none());
            }
            _ => panic!("expected SlotDirective"),
        }
    }

    #[test]
    fn view_slot_type_constraint_hard_error() {
        // D4 (v2): `@slot name: Type` — the type constraint + data_param
        // are deleted; slot = parent-provided content only. The parser must
        // hard-error with the design-doc 域 4 diagnostic text.
        let src = "component App { view { @slot header: String { Text { \"fallback\" } } } }";
        let tokens = tokenize(src);
        let (_, errs) = parse(tokens, src);
        assert!(
            errs.iter().any(|e| {
                e.kind == Some(crate::error::DiagnosticKind::ESlotRemoved)
                    && e.message
                        .contains("类型约束与数据参数已删除；slot = 父传内容")
            }),
            "expected ESlotRemoved error with v2 migration hint, got: {:?}",
            errs.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn view_slot_generic_type_constraint_hard_error() {
        // fn types hit the same deleted-form diagnostic.
        let src = "component App { view { @slot onSelect: fn(String) -> Void { } } }";
        let tokens = tokenize(src);
        let (_, errs) = parse(tokens, src);
        assert!(
            errs.iter().any(|e| {
                e.kind == Some(crate::error::DiagnosticKind::ESlotRemoved)
                    && e.message.contains("类型约束与数据参数已删除")
            }),
            "expected ESlotRemoved error for fn-type constraint, got: {:?}",
            errs.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn element_slot_attribute_consumption_hard_error() {
        // D4 (v2): `slot="name"` consumption is deleted — the only consumer
        // form is the named-props projection `name={<.../>}`.
        let src = "component App { view { <div slot=\"header\">x</div> } }";
        let tokens = tokenize(src);
        let (_, errs) = parse(tokens, src);
        assert!(
            errs.iter().any(|e| {
                e.kind == Some(crate::error::DiagnosticKind::ESlotRemoved)
                    && e.message.contains("改用命名 props：name={...}")
            }),
            "expected ESlotRemoved error with named-props hint, got: {:?}",
            errs.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn element_slot_attribute_consumption_hard_error_on_component_child() {
        // The same diagnostic fires inside a component call's children —
        // that was the legacy `slot="name"` consumption site.
        let src = "component A { view { <B><span slot=\"header\">H</span></B> } }";
        let tokens = tokenize(src);
        let (_, errs) = parse(tokens, src);
        assert!(
            errs.iter().any(|e| {
                e.kind == Some(crate::error::DiagnosticKind::ESlotRemoved)
                    && e.message.contains("改用命名 props：name={...}")
            }),
            "expected ESlotRemoved error at component call site, got: {:?}",
            errs.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn view_slot_directive_named_default_is_unnamed() {
        // Per ADR-053 D5 / syntax spec §5.2, the literal name `default`
        // declares the default (unnamed) slot — `@slot default` → name None.
        let children = view_children("component App { view { @slot default } }");
        match &children[0] {
            ViewNode::SlotDirective { name, .. } => {
                assert!(name.is_none(), "`@slot default` must be the unnamed slot");
            }
            _ => panic!("expected SlotDirective"),
        }
    }

    #[test]
    fn view_slot_directive_view_keyword_is_name() {
        // At view level there is no view-block ambiguity — the `view`
        // keyword remains a valid slot name (pre-D5 behavior preserved).
        let children = view_children("component App { view { @slot view { <div/> } } }");
        match &children[0] {
            ViewNode::SlotDirective {
                name,
                default_content,
                ..
            } => {
                assert_eq!(name.as_deref(), Some("view"));
                assert_eq!(default_content.as_ref().map(|c| c.len()), Some(1));
            }
            _ => panic!("expected SlotDirective"),
        }
    }

    #[test]
    fn view_slot_directive_style_keyword_is_name() {
        // Same as `view` — `style` stays a valid view-level slot name.
        let children = view_children("component App { view { @slot style { <span/> } } }");
        match &children[0] {
            ViewNode::SlotDirective {
                name,
                default_content,
                ..
            } => {
                assert_eq!(name.as_deref(), Some("style"));
                assert_eq!(default_content.as_ref().map(|c| c.len()), Some(1));
            }
            _ => panic!("expected SlotDirective"),
        }
    }

    // ── Body-level @slot declarations (ADR-053 D5) ──────────

    /// Parse a component definition and return it.
    fn component_def(source: &str) -> ComponentDef {
        let tokens = tokenize(source);
        let (program, errors) = parse(tokens, source);
        assert!(
            errors.is_empty(),
            "unexpected parse errors: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
        let module = program.modules.get("input.omni").unwrap();
        assert_eq!(module.components.len(), 1, "expected exactly one component");
        module.components[0].clone()
    }

    #[test]
    fn body_slot_declaration_with_fallback() {
        let comp = component_def(
            "component Modal { @slot header { <h4>x</h4> } view { <div>{header}</div> } }",
        );
        assert_eq!(comp.slots.len(), 1, "one body-level slot expected");
        let slot = &comp.slots[0];
        assert_eq!(slot.name.as_deref(), Some("header"));
        assert!(
            matches!(slot.default_content, Some(ref c) if c.len() == 1),
            "fallback content must be parsed"
        );
    }

    #[test]
    fn body_slot_declaration_default_unnamed() {
        // D5: `@slot default` declares the default/unnamed slot
        let comp = component_def("component Modal { @slot default view { <div/> } }");
        assert_eq!(comp.slots.len(), 1);
        assert!(
            comp.slots[0].name.is_none(),
            "`@slot default` is the unnamed slot"
        );
        assert!(comp.slots[0].default_content.is_none());
    }

    #[test]
    fn body_slot_declaration_bare_unnamed() {
        let comp = component_def("component Modal { @slot view { <div/> } }");
        assert_eq!(comp.slots.len(), 1);
        assert!(comp.slots[0].name.is_none());
    }

    #[test]
    fn body_slot_declaration_unnamed_with_content() {
        let comp = component_def("component Modal { @slot { <p>hi</p> } view { <div/> } }");
        assert_eq!(comp.slots.len(), 1);
        assert!(comp.slots[0].name.is_none());
        assert!(
            matches!(comp.slots[0].default_content, Some(ref c) if c.len() == 1),
            "unnamed slot fallback content must be parsed"
        );
    }

    #[test]
    fn body_slot_type_constraint_hard_error() {
        // D4 (v2): body-level `@slot name: Type` hits the same deleted-form
        // diagnostic as the view-level directive.
        let src = "component Modal { @slot footer: String view { <div/> } }";
        let tokens = tokenize(src);
        let (_, errs) = parse(tokens, src);
        assert!(
            errs.iter().any(|e| {
                e.kind == Some(crate::error::DiagnosticKind::ESlotRemoved)
                    && e.message
                        .contains("类型约束与数据参数已删除；slot = 父传内容")
            }),
            "expected ESlotRemoved error for body typed slot, got: {:?}",
            errs.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn body_slot_paren_form_hard_error() {
        // D4 (v2): legacy `@slot(name)` directive form is deleted — the
        // body-level declaration is `@slot name { ... }` (no parens).
        let src = "component Modal { @slot(header) view { <div/> } }";
        let tokens = tokenize(src);
        let (_, errs) = parse(tokens, src);
        assert!(
            errs.iter().any(|e| {
                e.kind == Some(crate::error::DiagnosticKind::ESlotRemoved)
                    && e.message.contains("改用 body 级声明 @slot name=")
            }),
            "expected ESlotRemoved error for paren form, got: {:?}",
            errs.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn body_slot_paren_default_hard_error() {
        // `@slot(default)` is the same deleted directive form.
        let src = "component Modal { @slot(default) view { <div/> } }";
        let tokens = tokenize(src);
        let (_, errs) = parse(tokens, src);
        assert!(
            errs.iter().any(|e| {
                e.kind == Some(crate::error::DiagnosticKind::ESlotRemoved)
                    && e.message.contains("改用 body 级声明 @slot name=")
            }),
            "expected ESlotRemoved error for @slot(default), got: {:?}",
            errs.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn body_slot_multiple_declarations() {
        let comp = component_def(
            "component Modal { @slot header { <DefaultHeader /> } @slot default view { <div/> } }",
        );
        assert_eq!(comp.slots.len(), 2);
        assert_eq!(comp.slots[0].name.as_deref(), Some("header"));
        assert!(comp.slots[1].name.is_none());
    }

    #[test]
    fn inline_store_recorded_in_component_ast() {
        // `store Name { ... }` in the component body must be recorded in
        // inline_stores (in addition to store_refs) so validators can tell
        // locally-defined stores from module-level store references.
        let comp = component_def(
            "component X { store Local { signal n: i32 = 0 } @store External; view { <div>{Local.n}</div> } }",
        );
        assert_eq!(
            comp.inline_stores,
            vec!["Local"],
            "inline store name recorded"
        );
        assert_eq!(
            comp.store_refs,
            vec!["Local", "External"],
            "inline stores and references both stay in store_refs"
        );
        assert_eq!(
            comp.signals.len(),
            1,
            "inline store signal flattened into component"
        );
        assert_eq!(comp.signals[0].name, "n");
        assert_eq!(
            comp.inline_store_defs.len(),
            1,
            "inline store body retained as a StoreDef (Phase 5.2 — inline stores \
             lower to the same module-level StoreIR as top-level stores)"
        );
        let def = &comp.inline_store_defs[0];
        assert_eq!(def.name, "Local");
        assert_eq!(def.signals.len(), 1, "store signal kept in the StoreDef");
        assert_eq!(def.signals[0].name, "n");
    }

    #[test]
    fn inline_store_body_retains_actions_and_getters() {
        let comp = component_def(
            "component X { store Cart { signal items: Array<String> = []; \
             getter count: i32 = items.len(); action add(x: String) { items = [...items, x]; } } \
             view { <div/> } }",
        );
        let def = &comp.inline_store_defs[0];
        assert_eq!(def.signals.len(), 1);
        assert_eq!(def.getters.len(), 1);
        assert_eq!(def.getters[0].name, "count");
        assert_eq!(def.actions.len(), 1);
        assert_eq!(def.actions[0].name, "add");
        // The flattened copies must agree with the retained bodies.
        assert_eq!(comp.getters.len(), 1);
        assert_eq!(comp.actions.len(), 1);
    }

    // ── @theme signal marker (v2 域 5: signal_decl ::= 'signal' ident ':' type ('@theme')? '=' expr) ──

    #[test]
    fn store_signal_theme_marker_parsed() {
        let src = r#"store ThemeStore { signal mode: String @theme = "light"; }"#;
        let tokens = tokenize(src);
        let (program, errors) = parse(tokens, src);
        assert!(
            errors.is_empty(),
            "expected clean parse, got: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
        let module = program.modules.get("input.omni").unwrap();
        assert_eq!(module.stores.len(), 1);
        let store = &module.stores[0];
        assert_eq!(store.signals.len(), 1);
        assert!(
            store.signals[0].is_theme,
            "mode signal should carry the @theme marker"
        );
    }

    #[test]
    fn store_signal_without_theme_marker_stays_false() {
        let src = r#"store Counter { signal n: i32 = 0; }"#;
        let tokens = tokenize(src);
        let (program, errors) = parse(tokens, src);
        assert!(errors.is_empty());
        let module = program.modules.get("input.omni").unwrap();
        assert!(!module.stores[0].signals[0].is_theme);
    }

    #[test]
    fn component_signal_theme_marker_rejected() {
        // @theme 仅允许顶层 store 的 signal —— 组件 signal 拒绝（解析期硬诊断）。
        let src = r#"component X { signal mode: String @theme = "light"; view { <div/> } }"#;
        let tokens = tokenize(src);
        let (_, errors) = parse(tokens, src);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("@theme 标记仅允许顶层 store")),
            "component signal with @theme must error, got: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn inline_store_signal_theme_marker_rejected() {
        // 内联 store 的 signal 被扁平化为组件级声明，无法保留 @theme 语义 —— 拒绝。
        let src = r#"component X { store Local { signal mode: String @theme = "light"; } view { <div/> } }"#;
        let tokens = tokenize(src);
        let (_, errors) = parse(tokens, src);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("@theme 标记仅允许顶层 store")),
            "inline store signal with @theme must error, got: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn bare_store_reference_not_recorded_as_inline() {
        // `store Name;` without a body is a module-level store reference,
        // not an inline definition.
        let comp = component_def("component X { store External; view { <div/> } }");
        assert!(comp.inline_stores.is_empty());
        assert_eq!(comp.store_refs, vec!["External"]);
    }

    // ── Task 3.5: store BNF 统一（顶层/内联成员集一致） ────

    /// 顶层 store 内 `fn` 成员必须报错（设计文档域 5：用 getter 或 action）。
    #[test]
    fn top_level_store_fn_member_is_error() {
        let source = "store S { fn helper() {} }";
        let tokens = tokenize(source);
        let (_, errors) = parse(tokens, source);
        assert!(
            !errors.is_empty(),
            "fn member in top-level store must error"
        );
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("store 内 fn 成员不允许")),
            "expected the store-fn diagnostic, got: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    /// 内联 store（component 内 `store Name { ... }`）内 `fn` 成员同样必须
    /// 报错——顶层与内联 BNF 统一为 signal|getter|action。
    #[test]
    fn inline_store_fn_member_is_error() {
        let source = "component X { store Local { fn helper() {} } view { <div/> } }";
        let tokens = tokenize(source);
        let (_, errors) = parse(tokens, source);
        assert!(!errors.is_empty(), "fn member in inline store must error");
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("store 内 fn 成员不允许")),
            "expected the store-fn diagnostic, got: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    /// 统一后的成员集 signal|getter|action 在顶层与内联都合法。
    #[test]
    fn store_member_set_uniform_at_both_levels() {
        // 顶层 store：signal + getter + action
        let source =
            "store S { signal n: i32 = 0; getter g: i32 = n + 1; action a() { n = n + 1; } }";
        let tokens = tokenize(source);
        let (program, errors) = parse(tokens, source);
        assert!(
            errors.is_empty(),
            "top-level store with signal/getter/action must parse clean, got: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
        let module = program.modules.get("input.omni").unwrap();
        assert_eq!(module.stores.len(), 1);
        assert_eq!(module.stores[0].signals.len(), 1);
        assert_eq!(module.stores[0].getters.len(), 1);
        assert_eq!(module.stores[0].actions.len(), 1);

        // 内联 store：signal + getter + action（flatten 进 component）
        let comp = component_def(
            "component X { store Local { signal n: i32 = 0; getter g: i32 = n + 1; action a() { n = n + 1; } } view { <div>{n}</div> } }",
        );
        assert_eq!(comp.signals.len(), 1, "inline signal flattened");
        assert_eq!(comp.getters.len(), 1, "inline getter flattened");
        assert_eq!(comp.actions.len(), 1, "inline action flattened");
    }

    // ── Consumer named-prop slot projection (ADR-053 D5) ────

    #[test]
    fn consumer_named_prop_slot_content() {
        // <Modal header={<div>Title</div>}>content</Modal>
        let children = view_children(
            "component App { view { <Modal header={<div>Title</div>}>content</Modal> } }",
        );
        assert_eq!(children.len(), 1);
        match &children[0] {
            ViewNode::Element(el) => {
                assert_eq!(el.tag, "Modal");
                assert_eq!(el.props.len(), 1, "one named-prop expected");
                match &el.props[0] {
                    ElementProp::SlotContent { name, content } => {
                        assert_eq!(name, "header");
                        assert_eq!(content.len(), 1);
                        assert!(matches!(&content[0], ViewNode::Element(e) if e.tag == "div"));
                    }
                    other => panic!("expected SlotContent prop, got {:?}", other),
                }
                assert_eq!(el.children.len(), 1, "default children preserved");
                assert!(matches!(&el.children[0], ViewNode::Text(t) if t == "content"));
            }
            _ => panic!("expected Element"),
        }
    }

    #[test]
    fn consumer_named_prop_slot_content_multiple_nodes() {
        let children =
            view_children("component App { view { <Modal header={<div>a</div><div>b</div>} /> } }");
        match &children[0] {
            ViewNode::Element(el) => match &el.props[0] {
                ElementProp::SlotContent { name, content } => {
                    assert_eq!(name, "header");
                    assert_eq!(content.len(), 2, "sibling elements must all be captured");
                }
                other => panic!("expected SlotContent prop, got {:?}", other),
            },
            _ => panic!("expected Element"),
        }
    }

    #[test]
    fn consumer_named_prop_slot_content_self_closing_element() {
        let children = view_children("component App { view { <Modal header={<br/>} /> } }");
        match &children[0] {
            ViewNode::Element(el) => match &el.props[0] {
                ElementProp::SlotContent { name, content } => {
                    assert_eq!(name, "header");
                    assert_eq!(content.len(), 1);
                }
                other => panic!("expected SlotContent prop, got {:?}", other),
            },
            _ => panic!("expected Element"),
        }
    }

    #[test]
    fn consumer_named_prop_empty_braces_errors() {
        // header={} — empty braces are not a valid slot projection
        let src = "component App { view { <Modal header={} /> } }";
        let tokens = tokenize(src);
        let (_, errors) = parse(tokens, src);
        assert!(!errors.is_empty(), "expected parse error for empty braces");
    }

    #[test]
    fn consumer_regular_expr_prop_unchanged() {
        // {count} interpolation props still parse as Static — the `<`
        // discriminator must not affect expression props.
        let children = view_children(
            "component App { signal count: i32 = 0; view { <Modal count={count} /> } }",
        );
        match &children[0] {
            ViewNode::Element(el) => match &el.props[0] {
                ElementProp::Static { name, value } => {
                    assert_eq!(name, "count");
                    assert!(matches!(value, Expr::VarRef(v, _) if v == "count"));
                }
                other => panic!("expected Static prop, got {:?}", other),
            },
            _ => panic!("expected Element"),
        }
    }

    #[test]
    fn view_if_mode_colon_string() {
        // New syntax: mode: "wait" — colon separator
        let children =
            view_children("component App { view { @if show mode: \"wait\" { Text { \"a\" } } } }");
        assert_eq!(children.len(), 1);
        if let ViewNode::IfDirective(dir) = &children[0] {
            assert_eq!(dir.transition_mode.as_deref(), Some("wait"));
        } else {
            panic!("expected IfDirective");
        }
    }

    #[test]
    fn view_if_mode_colon_ident() {
        // Bare ident values also accepted: mode: fade
        let children =
            view_children("component App { view { @if show mode: fade { Text { \"a\" } } } }");
        if let ViewNode::IfDirective(dir) = &children[0] {
            assert_eq!(dir.transition_mode.as_deref(), Some("fade"));
        } else {
            panic!("expected IfDirective");
        }
    }

    #[test]
    fn view_if_mode_invalid_value_rejected() {
        // Invalid mode value → @if directive rejected → parse error recorded,
        // component skipped (module-level error recovery drops the component).
        let src = "component App { view { @if show mode: \"sync\" { Text { \"a\" } } } }";
        let (program, errors) = parse(tokenize(src), src);
        assert!(!errors.is_empty(), "expected parse error for invalid mode");
        assert!(program.modules["input.omni"].components.is_empty());
    }

    #[test]
    fn view_for_scroll_hint() {
        // @for item in list [scroll] — scroll hint (whitespace-separated bracket)
        let children = view_children(
            "component App { view { @for item in list [scroll] { Text { item } } } }",
        );
        assert_eq!(children.len(), 1);
        if let ViewNode::ForDirective(dir) = &children[0] {
            assert!(dir.scroll_hint);
            assert!(matches!(&dir.iterable, Expr::VarRef(name, _) if name == "list"));
        } else {
            panic!("expected ForDirective");
        }
    }

    #[test]
    fn view_for_scroll_hint_before_key() {
        // Spec §7.3 example: @for item in allItems [scroll] key={item.id}
        let children = view_children(
            "component App { view { @for item in allItems [scroll] key={item.id} { Text { item } } } }",
        );
        if let ViewNode::ForDirective(dir) = &children[0] {
            assert!(dir.scroll_hint);
            assert!(matches!(&dir.iterable, Expr::VarRef(name, _) if name == "allItems"));
            assert!(dir.key_expr.is_some());
        } else {
            panic!("expected ForDirective");
        }
    }

    #[test]
    fn view_for_scroll_hint_after_key() {
        // BNF order: key_expr? scroll_hint? — hint may come after key={...}
        let children = view_children(
            "component App { view { @for item in list key={item.id} [scroll] { Text { item } } } }",
        );
        if let ViewNode::ForDirective(dir) = &children[0] {
            assert!(dir.scroll_hint);
            assert!(dir.key_expr.is_some());
        } else {
            panic!("expected ForDirective");
        }
    }

    #[test]
    fn view_for_adjacent_bracket_is_index_access() {
        // list[scroll] (no whitespace) stays index access, not a scroll hint
        let children =
            view_children("component App { view { @for item in list[scroll] { Text { item } } } }");
        if let ViewNode::ForDirective(dir) = &children[0] {
            assert!(!dir.scroll_hint);
            assert!(matches!(&dir.iterable, Expr::IndexGet { .. }));
        } else {
            panic!("expected ForDirective");
        }
    }

    #[test]
    fn view_for_spaced_index_access_preserved() {
        // Spaced bracket with non-scroll index stays index access
        let children =
            view_children("component App { view { @for item in list [0] { Text { item } } } }");
        if let ViewNode::ForDirective(dir) = &children[0] {
            assert!(!dir.scroll_hint);
            assert!(matches!(&dir.iterable, Expr::IndexGet { .. }));
        } else {
            panic!("expected ForDirective");
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

    #[test]
    fn view_semantic_binding_props() {
        // focus={sig}, scroll-x={sig}, scroll-y={sig}, value={sig} parse as
        // ElementProp::RefBinding — compiler-recognized semantic bindings.
        let children = view_children(
            "component App { view { <div focus={mySig} scroll-x={pos} scroll-y={posY} value={text} /> } }",
        );
        assert_eq!(children.len(), 1);
        match &children[0] {
            ViewNode::Element(el) => {
                assert_eq!(el.tag, "div");
                assert_eq!(el.props.len(), 4);
                let bindings: Vec<(&str, &Expr)> = el
                    .props
                    .iter()
                    .filter_map(|p| match p {
                        ElementProp::RefBinding { binding, signal } => {
                            Some((binding.as_str(), signal))
                        }
                        _ => None,
                    })
                    .collect();
                assert_eq!(
                    bindings.iter().map(|(b, _)| *b).collect::<Vec<_>>(),
                    vec!["focus", "scroll-x", "scroll-y", "value"]
                );
                assert!(
                    bindings
                        .iter()
                        .all(|(_, sig)| matches!(sig, Expr::VarRef(_, _))),
                    "each binding should carry a signal expression"
                );
            }
            _ => panic!("expected Element"),
        }
    }

    #[test]
    fn view_value_string_prop_stays_static() {
        // value="option1" is a generic static prop — RefBinding syntax only
        // applies to the value={expr} form.
        let children =
            view_children("component App { view { <option value=\"option1\">x</option> } }");
        assert_eq!(children.len(), 1);
        match &children[0] {
            ViewNode::Element(el) => {
                assert_eq!(el.tag, "option");
                match &el.props[0] {
                    ElementProp::Static { name, value } => {
                        assert_eq!(name, "value");
                        assert!(
                            matches!(value, Expr::Literal(Literal::String(s), _) if s == "option1")
                        );
                    }
                    other => panic!("expected Static prop, got {:?}", other),
                }
            }
            _ => panic!("expected Element"),
        }
    }

    #[test]
    fn view_computed_semantic_name_prop_stays_static() {
        // A computed expression on a semantic binding name (value={age.to_string()},
        // focus={isActive && ready}) is a generic Static prop — RefBinding is
        // reserved for the bare-signal form value={sig} per the spec.
        let children = view_children(
            "component App { view { \
             <input type=\"number\" value={age.to_string()} focus={isActive && ready} /> } }",
        );
        assert_eq!(children.len(), 1);
        match &children[0] {
            ViewNode::Element(el) => {
                assert_eq!(el.tag, "input");
                assert_eq!(el.props.len(), 3);
                for prop in &el.props {
                    match prop {
                        ElementProp::Static { name, value } => {
                            if name == "value" || name == "focus" {
                                assert!(
                                    matches!(
                                        value,
                                        Expr::MethodCall { .. } | Expr::BinaryOp { .. }
                                    ),
                                    "computed expr should stay a Static prop, got: {:?}",
                                    value
                                );
                            }
                        }
                        other => panic!(
                            "expected Static prop for computed expression, got: {:?}",
                            other
                        ),
                    }
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
        let (program, _) = parse(tokens, source);
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
        let (program, _) = parse(tokens, source);
        let module = program.modules.get("input.omni").unwrap();
        assert_eq!(module.structs.len(), 1);
        assert_eq!(module.structs[0].name, "Point");
        assert_eq!(module.structs[0].fields.len(), 2);
    }

    #[test]
    fn roundtrip_enum_def() {
        let source = "type Color = Red | Green | Blue";
        let tokens = tokenize(source);
        let (program, _) = parse(tokens, source);
        let module = program.modules.get("input.omni").unwrap();
        assert_eq!(module.tagged_unions.len(), 1);
    }

    #[test]
    fn roundtrip_type_alias() {
        let source = "type UserId = i32;";
        let tokens = tokenize(source);
        let (program, _) = parse(tokens, source);
        let module = program.modules.get("input.omni").unwrap();
        assert_eq!(module.type_aliases.len(), 1);
        assert_eq!(module.type_aliases[0].name, "UserId");
    }

    #[test]
    fn roundtrip_import_export() {
        let source = "import { add } from \"./utils/math.omni\";\nexport { main } from \"./utils/math.omni\";";
        let tokens = tokenize(source);
        let (program, _) = parse(tokens, source);
        let module = program.modules.get("input.omni").unwrap();
        assert_eq!(module.imports.len(), 1);
        assert_eq!(module.exports.len(), 1);
    }

    #[test]
    fn test_reject_invalid_hex() {
        let source = "let x = 0xFFG;";
        let tokens = tokenize(source);
        let (_, errors) = parse(tokens, source);
        assert!(
            !errors.is_empty(),
            "Invalid hex '0xFFG' should produce parse error"
        );
    }

    #[test]
    fn test_reject_invalid_float() {
        let source = "let x = 1.2.3;";
        let tokens = tokenize(source);
        let (_, errors) = parse(tokens, source);
        assert!(
            !errors.is_empty(),
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
        let module = parser.parse_module();
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
        let module = parser.parse_module();
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

    #[test]
    fn parse_module_level_const() {
        let src = r##"
            import omniview;
            const BRAND = "#6366f1";
            const SPACING = "16px";
            component App { view { <div /> } }
        "##;
        let tokens = tokenize(src);
        let (program, _) = parse(tokens, src);
        let module = program
            .modules
            .get("input.omni")
            .expect("should have module");
        assert_eq!(module.consts.len(), 2);
        assert_eq!(module.consts[0].name, "BRAND");
        assert_eq!(module.consts[1].name, "SPACING");
        // Check values are string literals
        match &module.consts[0].value {
            Expr::Literal(Literal::String(s), _) => assert_eq!(s, "#6366f1"),
            _ => panic!("expected string literal"),
        }
    }

    // ════════════════════════════════════════════════════════
    // @-keyword text safety & dead-syntax diagnostics (deferral #18)
    // ════════════════════════════════════════════════════════

    #[test]
    fn view_text_at_focus_mid_text() {
        // @focus inside text content must parse as text — not silently truncate
        // the element's children (deferral #18, Gap 1).
        let children = view_children("component App { view { <p>关于 @focus 触发器</p> } }");
        assert_eq!(children.len(), 1);
        match &children[0] {
            ViewNode::Element(el) => {
                assert_eq!(el.tag, "p");
                assert_eq!(el.children.len(), 1, "children must not be truncated");
                assert!(matches!(&el.children[0], ViewNode::Text(s) if s.contains("@focus")));
            }
            _ => panic!("expected Element"),
        }
    }

    #[test]
    fn view_text_at_focus_at_start() {
        // @focus at the start of text content parses as text too.
        let children = view_children("component App { view { <p>@focus me</p> } }");
        assert_eq!(children.len(), 1);
        match &children[0] {
            ViewNode::Element(el) => {
                assert_eq!(el.children.len(), 1);
                assert!(matches!(&el.children[0], ViewNode::Text(s) if s.contains("@focus")));
            }
            _ => panic!("expected Element"),
        }
    }

    #[test]
    fn view_focus_trigger_attr_parses() {
        // @focus={motion} as an element attribute must remain a motion trigger.
        let children = view_children("component App { view { <div @focus={fadeIn}/> } }");
        assert_eq!(children.len(), 1);
        match &children[0] {
            ViewNode::Element(el) => {
                assert_eq!(el.props.len(), 1);
                assert!(matches!(&el.props[0], ElementProp::MotionTrigger(_)));
            }
            _ => panic!("expected Element"),
        }
    }

    #[test]
    fn view_hero_attr_clear_error() {
        // @hero is dead syntax (removed per ADR-032/053) — the parser must
        // report a clear diagnostic instead of the generic "Unexpected token".
        let src = "component App { view { <div @hero=\"card-1\"/> } }";
        let tokens = tokenize(src);
        let (_, errs) = parse(tokens, src);
        assert!(
            errs.iter().any(|e| e.message.contains("hero")),
            "expected a hero-specific error, got: {:?}",
            errs.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn view_paused_attr_clear_error() {
        // @paused is deleted syntax (ADR-053) — it used to parse as a
        // generic event binding with no runtime effect. v2 (Task 5.3) must
        // report the E_PAUSED_REMOVED diagnostic with a migration hint.
        let src = "component App { view { <div @paused={isPaused}/> } }";
        let tokens = tokenize(src);
        let (_, errs) = parse(tokens, src);
        assert!(
            errs.iter().any(|e| {
                e.kind == Some(crate::error::DiagnosticKind::EPausedRemoved)
                    && e.message.contains("paused 已删除，用 @if 条件渲染替代")
            }),
            "expected a paused-specific error, got: {:?}",
            errs.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn view_text_trigger_words_not_dropped() {
        // Other trigger keywords in text must keep their literal @-spelling
        // (token_text fallback must not collapse them to empty strings).
        let children = view_children("component App { view { <p>on @hover over</p> } }");
        assert_eq!(children.len(), 1);
        match &children[0] {
            ViewNode::Element(el) => {
                assert_eq!(el.children.len(), 1);
                assert!(matches!(&el.children[0], ViewNode::Text(s) if s.contains("@hover")));
            }
            _ => panic!("expected Element"),
        }
    }

    #[test]
    fn view_text_all_at_keywords_exact_spellings() {
        // Every @-keyword accepted as text must render its exact literal
        // spelling with no truncation and no dropped words. Exact-equality
        // assertions lock token_text against lexer/keyword drift (e.g. IfMode
        // silently rendering "if_mode" instead of "@if-mode").
        let spellings = [
            "@enter",
            "@hover",
            "@press",
            "@focus",
            "@in-view",
            "@scroll",
            "@drag-horizontal",
            "@drag-vertical",
            "@hero",
            "@store",
            "@motion",
            "@native",
            "@fallback",
            "@error",
            "@mount",
            "@unmount",
            "@effect",
            "@else",
            "@empty",
            "@if-mode",
        ];
        for kw in spellings {
            let src = format!("component App {{ view {{ <p>a {kw} b</p> }} }}");
            let children = view_children(&src);
            assert_eq!(children.len(), 1, "top-level children truncated at {kw}");
            match &children[0] {
                ViewNode::Element(el) => {
                    assert_eq!(el.children.len(), 1, "element children truncated at {kw}");
                    assert!(
                        matches!(&el.children[0], ViewNode::Text(s) if s == &format!("a {kw} b")),
                        "text rendering mismatch for {kw}"
                    );
                }
                _ => panic!("expected Element for {kw}"),
            }
        }
    }

    #[test]
    fn view_text_throw_try_catch_finally_keywords() {
        // Throw/Try/Catch/Finally lex as dedicated keyword tokens; they must
        // be accepted as view text and keep their literal spellings
        // (is_text_token via keyword_as_ident_name + token_text fallback).
        for kw in ["throw", "try", "catch", "finally"] {
            let src = format!("component App {{ view {{ <p>a {kw} b</p> }} }}");
            let children = view_children(&src);
            assert_eq!(children.len(), 1, "top-level children truncated at {kw}");
            match &children[0] {
                ViewNode::Element(el) => {
                    assert_eq!(el.children.len(), 1, "element children truncated at {kw}");
                    assert!(
                        matches!(&el.children[0], ViewNode::Text(s) if s == &format!("a {kw} b")),
                        "text rendering mismatch for {kw}"
                    );
                }
                _ => panic!("expected Element for {kw}"),
            }
        }
    }

    #[test]
    fn view_text_throw_keywords_combined() {
        // All four keywords in a single text run must survive as one node.
        let children = view_children("component App { view { <p>try throw catch finally</p> } }");
        assert_eq!(children.len(), 1);
        match &children[0] {
            ViewNode::Element(el) => {
                assert_eq!(el.children.len(), 1);
                assert!(
                    matches!(&el.children[0], ViewNode::Text(s) if s == "try throw catch finally"),
                    "combined throw-keyword text must render exactly"
                );
            }
            _ => panic!("expected Element"),
        }
    }

    // ── a11y block (域 10 semantic accessibility, design doc 域 10) ──

    /// Parse a component source and return its first element's props.
    fn first_element_props(source: &str) -> Vec<ElementProp> {
        let children = view_children(source);
        match &children[0] {
            ViewNode::Element(el) => el.props.clone(),
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn a11y_block_parses_all_entries() {
        // a11y: { label/role/hidden/live/alt } parses as a dedicated
        // ElementProp::A11y (mirrors the style: { ... } block shape).
        let props = first_element_props(
            "component A11y { signal visible: bool = true; \
             view { <button a11y: { label: \"Close\", role: \"button\", hidden: !visible, \
             live: \"polite\", alt: \"close glyph\" }>x</button> } }",
        );
        let a11y = props
            .iter()
            .find_map(|p| match p {
                ElementProp::A11y(spec) => Some(spec),
                _ => None,
            })
            .unwrap_or_else(|| panic!("expected ElementProp::A11y, got: {:?}", props));
        assert_eq!(a11y.label.as_deref(), Some("Close"));
        assert_eq!(a11y.role.as_deref(), Some("button"));
        assert!(matches!(
            &a11y.hidden,
            Some(Expr::UnaryOp {
                op: UnaryOpKind::Not,
                ..
            })
        ));
        assert_eq!(a11y.live.as_deref(), Some("polite"));
        assert!(matches!(
            &a11y.alt,
            Some(Expr::Literal(Literal::String(s), _)) if s == "close glyph"
        ));
    }

    #[test]
    fn a11y_block_single_entry_parses() {
        // Only one entry — all other fields stay None.
        let props = first_element_props(
            "component A11y { view { <input a11y: { label: \"Search\" } /> } }",
        );
        let a11y = props
            .iter()
            .find_map(|p| match p {
                ElementProp::A11y(spec) => Some(spec),
                _ => None,
            })
            .unwrap_or_else(|| panic!("expected ElementProp::A11y, got: {:?}", props));
        assert_eq!(a11y.label.as_deref(), Some("Search"));
        assert!(
            a11y.role.is_none()
                && a11y.hidden.is_none()
                && a11y.live.is_none()
                && a11y.alt.is_none()
        );
    }

    #[test]
    fn a11y_block_empty_errors() {
        // Empty a11y block is a hard parse error (design doc 域 10: entries
        // are the payload — an empty block carries no intent).
        let source = "component A11y { view { <button a11y: { }>x</button> } }";
        let tokens = tokenize(source);
        let (_, errors) = parse(tokens, source);
        assert!(
            errors.iter().any(|e| e.message.contains("a11y 块不能为空")),
            "expected 'a11y 块不能为空' parse error, got: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn a11y_block_unknown_key_errors() {
        // Unknown a11y keys are rejected — typo'd intent must not silently
        // become a style prop.
        let source = "component A11y { view { <button a11y: { lable: \"x\" }>y</button> } }";
        let tokens = tokenize(source);
        let (_, errors) = parse(tokens, source);
        assert!(
            errors.iter().any(|e| e.message.contains("未知 a11y 键")),
            "expected unknown-key error, got: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn a11y_block_label_role_live_require_string_literal() {
        // label/role/live are string-valued per the BNF — a non-string value
        // is a parse error (hidden/alt accept arbitrary expressions).
        for (key, value) in [("label", "42"), ("role", "true"), ("live", "visible")] {
            let source = format!(
                "component A11y {{ view {{ <button a11y: {{ {key}: {value} }}>x</button> }} }}"
            );
            let tokens = tokenize(&source);
            let (_, errors) = parse(tokens, &source);
            assert!(
                errors.iter().any(|e| e.message.contains("a11y")),
                "expected a string-literal requirement error for {key}, got: {:?}",
                errors.iter().map(|e| &e.message).collect::<Vec<_>>()
            );
        }
    }
}
