//! Prefix expression parsing (Pratt parser prefix parselets).
//!
//! Handles unary operators, grouping, literals, identifiers, and all
//! expressions that start with a token, not an operator between operands.

use crate::error::ParseError;
use crate::lexer::{Span, TokenKind};
use crate::parser::ast::*;
use crate::parser::stmt::keyword_as_ident_name;
use crate::parser::Parser;

use super::span_of;
use super::template::parse_template_parts;

impl<'a> Parser<'a> {
    pub(super) fn parse_prefix(&mut self) -> Result<Expr, ParseError> {
        let t = self.advance();
        let span = t.span;

        match &t.kind {
            TokenKind::IntLiteral(n) => Ok(Expr::Literal(Literal::Int(*n), span)),
            TokenKind::FloatLiteral(n) => Ok(Expr::Literal(Literal::Float(*n), span)),
            TokenKind::StringLiteral(s) => Ok(Expr::Literal(Literal::String(s.clone()), span)),
            TokenKind::TemplateString(s) => {
                let parts = parse_template_parts(s, span)?;
                Ok(Expr::TemplateString { parts, span })
            }
            TokenKind::True => Ok(Expr::Literal(Literal::Bool(true), span)),
            TokenKind::False => Ok(Expr::Literal(Literal::Bool(false), span)),
            TokenKind::Null => Ok(Expr::Literal(Literal::Null, span)),

            TokenKind::Ident(name) => {
                let name = name.clone();
                self.parse_ident_expr(name, span)
            }

            // Await expression must be handled BEFORE contextual keywords.
            // Binding power 0 ensures await a + b ≡ await (a + b).
            TokenKind::Await => {
                let operand = self.parse_expr_bp(0)?;
                let end = span_of(&operand).end;
                Ok(Expr::AwaitExpr {
                    operand: Box::new(operand),
                    span: Span {
                        start: span.start,
                        end,
                        line: span.line,
                        col: span.col,
                    },
                })
            }

            // If-expression must be handled BEFORE contextual keywords
            // because `if` is also in keyword_as_ident_name
            TokenKind::If => {
                // If expression: if condition { expr } else { expr }
                // Supports else-if chains
                let condition = self.parse_expr()?;
                self.expect_close(TokenKind::LBrace, "expected '{'")?;
                let then_expr = self.parse_expr()?;
                self.expect_close(TokenKind::RBrace, "expected '}'")?;
                self.expect_close(TokenKind::Else, "expected 'else'")?;
                // else branch: either else-if or else-block
                let else_expr = if self.at(&TokenKind::If) {
                    self.parse_expr()? // recursively parse else-if as if-expr
                } else {
                    self.expect_close(TokenKind::LBrace, "expected '{' or 'if'")?;
                    let expr = self.parse_expr()?;
                    self.expect_close(TokenKind::RBrace, "expected '}'")?;
                    expr
                };
                Ok(Expr::IfExpr {
                    condition: Box::new(condition),
                    then_expr: Box::new(then_expr),
                    else_expr: Box::new(else_expr),
                    span,
                })
            }

            // Match expression: match expr { Pattern => body, ... }
            TokenKind::Match => {
                let matched = self.parse_expr()?;
                self.expect_close(TokenKind::LBrace, "expected '{' after match expr")?;
                let mut arms: Vec<MatchArm> = Vec::new();
                while !self.at(&TokenKind::RBrace) {
                    let pattern = self.parse_match_pattern()?;
                    self.expect_token(TokenKind::FatArrow, "expected '=>' in match arm")?;
                    let body = self.parse_match_body()?;
                    arms.push(MatchArm { pattern, body });
                    // Optional trailing comma
                    if self.at(&TokenKind::Comma) {
                        self.advance();
                    }
                    if self.at(&TokenKind::RBrace) {
                        break;
                    }
                }
                let end_span = self.peek().span;
                self.expect_close(TokenKind::RBrace, "expected '}' after match arms")?;
                Ok(Expr::MatchExpr {
                    matched: Box::new(matched),
                    arms,
                    span: Span {
                        start: span.start,
                        end: end_span.end,
                        line: span.line,
                        col: span.col,
                    },
                })
            }

            // Contextual keywords as identifiers in expressions
            kind if let Some(name) = keyword_as_ident_name(kind) => {
                let name = name.to_string();
                self.parse_ident_expr(name, span)
            }

            TokenKind::Minus => {
                let operand = self.parse_expr_bp(7)?;
                Ok(Expr::UnaryOp {
                    op: UnaryOpKind::Neg,
                    operand: Box::new(operand),
                    span: Span {
                        start: span.start,
                        end: span.end, // approximate
                        line: span.line,
                        col: span.col,
                    },
                })
            }
            TokenKind::Not => {
                let operand = self.parse_expr_bp(7)?;
                Ok(Expr::UnaryOp {
                    op: UnaryOpKind::Not,
                    operand: Box::new(operand),
                    span: Span {
                        start: span.start,
                        end: span.end,
                        line: span.line,
                        col: span.col,
                    },
                })
            }

            TokenKind::LParen => {
                let expr = self.parse_expr()?;
                self.expect_close(TokenKind::RParen, "expected ')'")?;
                Ok(expr)
            }

            TokenKind::LBracket => {
                let mut elements = Vec::new();
                if !self.at(&TokenKind::RBracket) {
                    elements.push(self.parse_expr()?);
                    while self.at(&TokenKind::Comma) {
                        self.advance();
                        if self.at(&TokenKind::RBracket) {
                            break;
                        }
                        elements.push(self.parse_expr()?);
                    }
                }
                self.expect_close(TokenKind::RBracket, "expected ']'")?;
                Ok(Expr::ArrayLiteral {
                    elements,
                    span: Span {
                        start: span.start,
                        end: self.peek().span.start,
                        line: span.line,
                        col: span.col,
                    },
                })
            }

            TokenKind::LBrace => {
                // Block expression: { stmts; trailing_expr }
                // '{' was already consumed by parse_prefix()
                match self.peek_kind() {
                    // Statement-starting tokens → parse as block expression
                    TokenKind::Let
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
                    | TokenKind::Semicolon
                    | TokenKind::LBrace => {
                        let start_span = self.peek_span();
                        let mut stmts = Vec::new();
                        let mut trailing_expr: Option<Expr> = None;
                        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
                            let stmt = self.parse_stmt()?;
                            if self.at(&TokenKind::RBrace) {
                                if let Stmt::ExprStmt { expr, .. } = &stmt {
                                    trailing_expr = Some(expr.clone());
                                    break;
                                }
                            }
                            stmts.push(stmt);
                        }
                        let end_span = self.peek_span();
                        self.expect_token(TokenKind::RBrace, "expected '}'")?;
                        Ok(Expr::Block {
                            body: Block {
                                stmts,
                                trailing_expr: trailing_expr.map(Box::new),
                                span: Span {
                                    start: start_span.start,
                                    end: end_span.start,
                                    line: start_span.line,
                                    col: start_span.col,
                                },
                            },
                            span,
                        })
                    }
                    _ => {
                        // Map literal: {key: val, ...}
                        let entries = self.parse_map_entries()?;
                        Ok(Expr::MapLiteral { entries, span })
                    }
                }
            }

            TokenKind::Pipe => {
                // Closure expression: |param1, param2| body
                let mut params: Vec<String> = Vec::new();
                loop {
                    match self.peek_kind() {
                        TokenKind::Pipe => {
                            self.advance(); // closing |
                            break;
                        }
                        TokenKind::Comma => {
                            self.advance();
                        }
                        _ => {
                            // Try to parse as identifier (including contextual keywords)
                            if let Some(name) = self.try_parse_ident() {
                                params.push(name);
                            } else {
                                break;
                            }
                        }
                    }
                }
                let body = self.parse_expr()?;
                Ok(Expr::ClosureExpr {
                    params,
                    body: Box::new(body),
                    span: Span {
                        start: span.start,
                        end: self.peek().span.start,
                        line: span.line,
                        col: span.col,
                    },
                })
            }

            TokenKind::DotDotDot => {
                // Spread expression: ...expr
                let target = self.parse_expr()?;
                Ok(Expr::SpreadExpr {
                    operand: Box::new(target),
                    span,
                })
            }

            // '=>' at the start of an expression — rejected with a migration
            // hint (pipe closures |x| are the only general closure form;
            // match-arm and inline-event-handler arrows are consumed by
            // their own parsers and never reach this prefix position).
            TokenKind::FatArrow => Err(self.closure_arrow_error(t.span)),

            _ => Err(ParseError {
                kind: None,
                message: format!("Unexpected token in expression: {:?}", t.kind),
                span,
            }),
        }
    }

    /// Handle an identifier (or contextual keyword) in expression context:
    /// VarRef, FuncCall, or MethodCall.
    pub(super) fn parse_ident_expr(
        &mut self,
        name: String,
        span: Span,
    ) -> Result<Expr, ParseError> {
        if self.at(&TokenKind::LParen) {
            self.advance(); // consume '('
                            // Builtin semantic function calls (spec 2026-08-09 §2.2) get a
                            // dedicated AST node so each target compiler can generate its
                            // platform notification shape (self-generated toast / SnackBar /
                            // self-drawn) instead of a generic FuncCall.
            if super::BUILTIN_FUNCTIONS.contains(&name.as_str()) && name == "showToast" {
                return self.parse_show_toast_call(&name, span);
            }
            // navigate/back (spec 2026-08-09 §2.3) stay generic FuncCalls —
            // the options object `{ params, query }` parses as a MapLiteral
            // argument and survives the IR round-trip; the semantic layer
            // validates (V12/W1) and each target compiler dispatches on the
            // callee name. They are reserved names (V13) via BUILTIN_FUNCTIONS.
            let (args, close_end) = self.parse_call_args()?;
            Ok(Expr::FuncCall {
                func: Box::new(Expr::VarRef(name.clone(), span)),
                args,
                span: Span {
                    start: span.start,
                    end: close_end,
                    line: span.line,
                    col: span.col,
                },
            })
        } else {
            Ok(Expr::VarRef(name, span))
        }
    }

    /// Parse the builtin `showToast(msg, { kind, duration })` call (spec
    /// 2026-08-09 §2.2). The `(` has already been consumed by
    /// `parse_ident_expr`; this parses through the closing `)`.
    ///
    /// Shape: `'showToast' '(' Expr ','? '{' kind? duration? '}'? ')'` — the
    /// options object uses colon-param style. The parse layer captures the
    /// call structurally only; value legality (V6/V7) and the reserved-name
    /// check (V8) belong to the semantic layer. Unknown option keys and
    /// duplicate keys are hard parse errors (they can never be legal).
    fn parse_show_toast_call(&mut self, name: &str, span: Span) -> Result<Expr, ParseError> {
        // First positional argument: the message (required by BNF).
        let msg = Box::new(self.parse_expr()?);

        // Optional trailing comma before the options object.
        if self.at(&TokenKind::Comma) {
            self.advance();
        }

        // Optional `{ kind?, duration? }` options object.
        let mut kind = None;
        let mut duration = None;
        if self.at(&TokenKind::LBrace) {
            self.advance();
            while !self.at(&TokenKind::RBrace) {
                // Snapshot the key token span *before* consuming it — every
                // option-shape error below points precisely at the offending
                // key (or at the token where '}' was expected).
                let key_span = self.peek().span;
                let key = match self.try_parse_ident() {
                    Some(k) => k,
                    None => {
                        // EOF / ')' with no '}' in sight means the options
                        // object lost its closing brace — report that, not a
                        // bogus key error. Any other token is a malformed key.
                        if self.at(&TokenKind::Eof) || self.at(&TokenKind::RParen) {
                            return Err(ParseError {
                                kind: None,
                                message: format!("{} 选项对象未闭合——期望 '}}' 结束选项", name),
                                span: key_span,
                            });
                        }
                        return Err(ParseError {
                            kind: None,
                            message: format!("{} 选项键应为标识符——合法选项: kind/duration", name),
                            span: key_span,
                        });
                    }
                };
                self.expect_close(TokenKind::Colon, "期望 ':' 后接 showToast 选项值")?;
                let value = self.parse_expr()?;
                match key.as_str() {
                    "kind" => {
                        if kind.is_some() {
                            return Err(ParseError {
                                kind: None,
                                message: format!(
                                    "showToast 选项 \"{}\" 重复声明——每个选项只能出现一次",
                                    key
                                ),
                                span: key_span,
                            });
                        }
                        kind = Some(Box::new(value));
                    }
                    "duration" => {
                        if duration.is_some() {
                            return Err(ParseError {
                                kind: None,
                                message: format!(
                                    "showToast 选项 \"{}\" 重复声明——每个选项只能出现一次",
                                    key
                                ),
                                span: key_span,
                            });
                        }
                        duration = Some(Box::new(value));
                    }
                    other => {
                        return Err(ParseError {
                            kind: None,
                            message: format!(
                                "未知 showToast 选项 \"{}\"——合法选项: kind/duration",
                                other
                            ),
                            span: key_span,
                        });
                    }
                }
                if self.at(&TokenKind::Comma) {
                    self.advance();
                }
            }
            self.expect_close(TokenKind::RBrace, "期望 '}' 闭合 showToast 选项")?;
        }

        let close_end = self.peek().span.end;
        self.expect_close(TokenKind::RParen, "期望 ')' 闭合 showToast 调用")?;
        Ok(Expr::BuiltinToastCall {
            msg,
            kind,
            duration,
            span: Span {
                start: span.start,
                end: close_end,
                line: span.line,
                col: span.col,
            },
        })
    }

    /// Convert an expression that is a VarRef or keyword ident to a string literal key.
    fn key_to_string_literal(expr: &Expr) -> Expr {
        match expr {
            Expr::VarRef(name, span) => Expr::Literal(Literal::String(name.clone()), *span),
            _ => expr.clone(),
        }
    }

    fn parse_map_entries(&mut self) -> Result<Vec<(Expr, Expr)>, ParseError> {
        let mut entries = Vec::new();
        if !self.at(&TokenKind::RBrace) {
            // Parse first entry (may be shorthand: { key } or explicit: { key: val })
            let key = self.parse_expr()?;
            if self.at(&TokenKind::Colon) {
                self.advance();
                let val = self.parse_expr()?;
                // Convert explicit key to string literal to avoid undeclared var errors
                let key_lit = Self::key_to_string_literal(&key);
                entries.push((key_lit, val));
            } else {
                // Shorthand: { key } → { key: key } with string literal key
                let key_expr = Self::key_to_string_literal(&key);
                entries.push((key_expr, key));
            }
            while self.at(&TokenKind::Comma) {
                self.advance();
                if self.at(&TokenKind::RBrace) {
                    break;
                }
                let k = self.parse_expr()?;
                if self.at(&TokenKind::Colon) {
                    self.advance();
                    let v = self.parse_expr()?;
                    // Convert explicit key to string literal
                    let key_lit = Self::key_to_string_literal(&k);
                    entries.push((key_lit, v));
                } else {
                    // Shorthand after comma: { a, b, c } or { a: 1, b, c: 3 }
                    let key_expr = Self::key_to_string_literal(&k);
                    entries.push((key_expr, k));
                }
            }
        }
        self.expect_close(TokenKind::RBrace, "expected '}'")?;
        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;

    /// Parse a standalone expression via the parser entry point.
    fn parse_expr(src: &str) -> Result<Expr, ParseError> {
        let tokens = tokenize(src);
        let mut p = Parser::new(tokens, src);
        p.parse_expr()
    }

    /// Parse a full source module and return the collected parse errors.
    fn parse_src(src: &str) -> Vec<ParseError> {
        let (_, errors) = parse(tokenize(src), src);
        errors
    }

    // ════════════════════════════════════════════════════════
    // Closure semantics unification (design doc 域 8 / task 1.1):
    // pipe closures |x| are the ONLY general closure form; fat-arrow
    // '=>' is rejected in general expression positions with a migration
    // hint (legal only in match arms and inline event handlers).
    // ════════════════════════════════════════════════════════

    #[test]
    fn fat_arrow_rejected_in_general_expressions() {
        let err = parse_expr("items.filter(item => item.done)").unwrap_err();
        assert!(
            err.message.contains("|x|"),
            "must hint pipe closure: {}",
            err.message
        );
    }

    #[test]
    fn pipe_closure_parses() {
        assert!(parse_expr("items.filter(|item| item.done)").is_ok());
    }

    #[test]
    fn fat_arrow_rejected_at_expression_start() {
        let err = parse_expr("=> 5").unwrap_err();
        assert!(
            err.message.contains("|x|"),
            "must hint pipe closure: {}",
            err.message
        );
    }

    #[test]
    fn fat_arrow_rejected_in_assignment_rhs() {
        // Before the fix this silently parsed as `f = x` and left `=> x + 1`
        // dangling — now the '=>' must be rejected with the migration hint.
        let err = parse_expr("f = x => x + 1").unwrap_err();
        assert!(
            err.message.contains("|x|"),
            "must hint pipe closure: {}",
            err.message
        );
    }

    #[test]
    fn match_arm_fat_arrow_still_parses() {
        // Regression: the match parser consumes '=>' directly (not via the
        // general expression path) — multi-arm matches must keep working.
        assert!(parse_expr("match x { \"a\" => 1, \"b\" => 2 }").is_ok());
    }

    #[test]
    fn event_inline_fat_arrow_still_parses() {
        // Regression: inline event handlers @onPress={() => { ... }} consume
        // '=>' directly in parse_event_handler_value — must not hit the
        // general-position rejection. (@press is a motion trigger, so the
        // event form is the on-prefixed name.)
        let src = r#"component App { view { <button @onPress={() => { msg = "x" }} /> } }"#;
        let errors = parse_src(src);
        assert!(
            errors.is_empty(),
            "unexpected parse errors: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn event_inline_fat_arrow_with_param_still_parses() {
        let src = r#"component App { view { <button @onPress={(e) => { msg = e.x }} /> } }"#;
        let errors = parse_src(src);
        assert!(
            errors.is_empty(),
            "unexpected parse errors: {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn struct_literal_on_assignment_rhs_parses() {
        // Regression: the assignment RHS went through raw Pratt
        // (parse_expr_bp) which dropped the struct-literal suffix, so
        // `s = S { f: v }` — the full-reconstruction update idiom — was
        // parsed as a call with a block argument and failed with
        // "Unexpected token in expression: Colon" per field.
        let ast = parse_expr(r#"stats = StatisticsData { name: "x", count: 1 }"#)
            .expect("struct literal on assignment RHS must parse");
        match ast {
            Expr::BinaryOp {
                op: BinaryOpKind::Assign,
                right,
                ..
            } => assert!(
                matches!(right.as_ref(), Expr::StructLiteral { .. }),
                "RHS must be a struct literal, got: {:?}",
                right
            ),
            other => panic!("expected assignment, got: {:?}", other),
        }
    }

    #[test]
    fn struct_literal_in_ternary_branches_parses() {
        // Regression: the ternary else-branch also skipped the suffix —
        // `c ? A { x: 1 } : B { y: 2 }` must parse both branches.
        let ast = parse_expr(r#"flag ? A { x: 1 } : B { y: 2 }"#)
            .expect("struct literals in ternary branches must parse");
        match ast {
            Expr::TernaryExpr {
                then_expr,
                else_expr,
                ..
            } => {
                assert!(matches!(then_expr.as_ref(), Expr::StructLiteral { .. }));
                assert!(matches!(else_expr.as_ref(), Expr::StructLiteral { .. }));
            }
            other => panic!("expected ternary, got: {:?}", other),
        }
    }

    // ════════════════════════════════════════════════════════
    // Navigation builtins (spec 2026-08-09 §2.3): navigate(path, {params,
    // query}) / back() parse as generic FuncCalls — the options object keeps
    // its MapLiteral shape (keys become string literals via
    // key_to_string_literal) so the IR round-trip preserves the structure;
    // semantic validation (V12/W1) and target dispatch come later.
    // ════════════════════════════════════════════════════════

    #[test]
    fn test_parse_navigate_call() {
        let ast = parse_expr(r#"navigate("/todos/:id", { params: { id: item } })"#)
            .expect("navigate call must parse");
        match ast {
            Expr::FuncCall { func, args, .. } => {
                assert!(
                    matches!(func.as_ref(), Expr::VarRef(name, _) if name == "navigate"),
                    "callee must be navigate, got: {:?}",
                    func
                );
                assert_eq!(args.len(), 2, "path + options object");
                assert!(
                    matches!(&args[0], Expr::Literal(Literal::String(p), _) if p == "/todos/:id"),
                    "first arg must be the path literal, got: {:?}",
                    args[0]
                );
                // options: { params: { id: item } } — MapLiteral with the
                // params key stringified, value another MapLiteral.
                match &args[1] {
                    Expr::MapLiteral { entries, .. } => {
                        assert_eq!(entries.len(), 1);
                        assert!(
                            matches!(&entries[0].0, Expr::Literal(Literal::String(k), _) if k == "params"),
                            "options key must be 'params', got: {:?}",
                            entries[0].0
                        );
                        assert!(
                            matches!(&entries[0].1, Expr::MapLiteral { .. }),
                            "params value must be a map, got: {:?}",
                            entries[0].1
                        );
                    }
                    other => panic!("expected MapLiteral options, got: {:?}", other),
                }
            }
            other => panic!("expected FuncCall, got: {:?}", other),
        }
    }

    #[test]
    fn test_parse_navigate_query_option() {
        let ast = parse_expr(r#"navigate("/todos", { query: { filter: "all" } })"#).unwrap();
        match ast {
            Expr::FuncCall { args, .. } => {
                assert_eq!(args.len(), 2);
                match &args[1] {
                    Expr::MapLiteral { entries, .. } => {
                        assert!(
                            matches!(&entries[0].0, Expr::Literal(Literal::String(k), _) if k == "query"),
                            "options key must be 'query', got: {:?}",
                            entries[0].0
                        );
                    }
                    other => panic!("expected MapLiteral options, got: {:?}", other),
                }
            }
            other => panic!("expected FuncCall, got: {:?}", other),
        }
    }

    #[test]
    fn test_parse_back_call() {
        let ast = parse_expr("back()").expect("back call must parse");
        match ast {
            Expr::FuncCall { func, args, .. } => {
                assert!(
                    matches!(func.as_ref(), Expr::VarRef(name, _) if name == "back"),
                    "callee must be back, got: {:?}",
                    func
                );
                assert!(args.is_empty(), "back() takes no arguments");
            }
            other => panic!("expected FuncCall, got: {:?}", other),
        }
    }
}
