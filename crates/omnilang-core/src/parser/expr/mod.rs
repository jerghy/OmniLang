//! Pratt expression parser.
//!
//! Precedence table (lowest to highest):
//!   1  =, +=, -=, *=, /=, %=      right-assoc
//!   2  ||                           left-assoc
//!   3  &&                           left-assoc
//!   4  ==, !=, <, >, <=, >=        left-assoc
//!   5  +, -                         left-assoc
//!   6  *, /, %                      left-assoc
//!   7  !, - (unary)                 prefix
//!   8  .method(), [idx], () (call)  postfix

use crate::error::ParseError;
use crate::lexer::{Span, TokenKind};
use crate::parser::ast::*;
use crate::parser::stmt::keyword_as_ident_name;
use crate::parser::Parser;

pub mod motion;

/// Binding power: (left_bp, right_bp). Higher = tighter.
type Bp = u8;

impl<'a> Parser<'a> {
    /// Parse an expression starting at current position. Uses Pratt parsing.
    pub fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.enter_recursion()?;
        let mut expr = self.parse_expr_bp(0)?;

        // After Pratt: check for struct literal suffix — TypeName { field: val, ... }
        // NOT handled as infix to avoid stealing `{` from statement blocks (if/while/etc.)
        if self.at(&TokenKind::LBrace) {
            if let Expr::VarRef(ref name, _) = expr {
                if name.chars().next().is_some_and(|c| c.is_uppercase()) {
                    expr = self.parse_struct_literal_suffix(name.clone())?;
                }
            }
        }
        self.leave_recursion();
        Ok(expr)
    }

    /// Pratt: parse expression with minimum binding power.
    fn parse_expr_bp(&mut self, min_bp: Bp) -> Result<Expr, ParseError> {
        let mut left = self.parse_prefix()?;

        loop {
            let bp = self.infix_bp();
            // (0, 0) means "not an infix operator" — break
            if bp == (0, 0) || bp.0 < min_bp {
                break;
            }
            left = self.parse_infix(left, bp)?;
        }
        Ok(left)
    }

    /// Parse struct literal fields after `TypeName {`.
    /// Called by `parse_expr` when VarRef + LBrace is detected.
    fn parse_struct_literal_suffix(&mut self, type_name: String) -> Result<Expr, ParseError> {
        let span_start = self.peek().span;
        self.advance(); // consume '{'

        let mut fields: Vec<(String, Expr)> = Vec::new();
        if !self.at(&TokenKind::RBrace) {
            loop {
                let field_name = match self.advance().kind {
                    TokenKind::Ident(name) => name,
                    other => {
                        return Err(ParseError {
                            message: format!(
                                "Expected field name in struct literal, got {:?}",
                                other
                            ),
                            span: self.peek().span,
                        });
                    }
                };
                self.expect_token(TokenKind::Colon, "expected ':' after field name")?;
                let value = self.parse_expr()?;
                fields.push((field_name, value));
                if self.at(&TokenKind::RBrace) {
                    break;
                }
                self.expect_token(TokenKind::Comma, "expected ',' or '}' after field value")?;
                if self.at(&TokenKind::RBrace) {
                    break; // allow trailing comma
                }
            }
        }
        self.expect_token(TokenKind::RBrace, "expected '}' to close struct literal")?;

        Ok(Expr::StructLiteral {
            type_name,
            fields,
            span: Span {
                start: span_start.start,
                end: self.peek().span.start,
                line: span_start.line,
                col: span_start.col,
            },
        })
    }

    /// Parse a match pattern: `_` | `Ident` | `Ident { binding, ... }` | literal
    fn parse_match_pattern(&mut self) -> Result<MatchPattern, ParseError> {
        let t = self.advance();
        let span = t.span;
        match &t.kind {
            TokenKind::Ident(name) if name == "_" => Ok(MatchPattern {
                kind: MatchPatternKind::Wildcard,
                span,
            }),
            TokenKind::Ident(name) => {
                // Check for variant with bindings: Ident { binding, ... }
                if self.at(&TokenKind::LBrace) {
                    self.advance(); // consume '{'
                    let mut bindings: Vec<String> = Vec::new();
                    if !self.at(&TokenKind::RBrace) {
                        loop {
                            if let TokenKind::Ident(b) = self.advance().kind {
                                bindings.push(b);
                            } else {
                                return Err(ParseError {
                                    message: "expected binding name in pattern".into(),
                                    span: self.peek().span,
                                });
                            }
                            if self.at(&TokenKind::RBrace) {
                                break;
                            }
                            self.expect_token(TokenKind::Comma, "expected ',' or '}' in pattern")?;
                        }
                    }
                    self.expect_token(TokenKind::RBrace, "expected '}' after pattern bindings")?;
                    Ok(MatchPattern {
                        kind: MatchPatternKind::Variant {
                            name: name.clone(),
                            bindings,
                        },
                        span: Span {
                            start: span.start,
                            end: self.peek().span.start,
                            line: span.line,
                            col: span.col,
                        },
                    })
                } else {
                    // Plain variant name, no bindings
                    Ok(MatchPattern {
                        kind: MatchPatternKind::Variant {
                            name: name.clone(),
                            bindings: vec![],
                        },
                        span,
                    })
                }
            }
            TokenKind::IntLiteral(n) => Ok(MatchPattern {
                kind: MatchPatternKind::Literal(Literal::Int(*n)),
                span,
            }),
            TokenKind::StringLiteral(s) => Ok(MatchPattern {
                kind: MatchPatternKind::Literal(Literal::String(s.clone())),
                span,
            }),
            TokenKind::True => Ok(MatchPattern {
                kind: MatchPatternKind::Literal(Literal::Bool(true)),
                span,
            }),
            TokenKind::False => Ok(MatchPattern {
                kind: MatchPatternKind::Literal(Literal::Bool(false)),
                span,
            }),
            other => Err(ParseError {
                message: format!("unexpected token in match pattern: {:?}", other),
                span,
            }),
        }
    }

    /// Parse a match arm body: either a block `{ ... }` or a single expression
    fn parse_match_body(&mut self) -> Result<Block, ParseError> {
        if self.at(&TokenKind::LBrace) {
            self.parse_block()
        } else {
            let expr = self.parse_expr()?;
            let span = *span_of(&expr);
            Ok(Block {
                stmts: vec![],
                trailing_expr: Some(Box::new(expr)),
                span,
            })
        }
    }

    // ── Prefix expressions ─────────────────────────────

    fn parse_prefix(&mut self) -> Result<Expr, ParseError> {
        let t = self.advance();
        let span = t.span;

        match &t.kind {
            TokenKind::IntLiteral(n) => Ok(Expr::Literal(Literal::Int(*n), span)),
            TokenKind::FloatLiteral(n) => Ok(Expr::Literal(Literal::Float(*n), span)),
            TokenKind::StringLiteral(s) => Ok(Expr::Literal(Literal::String(s.clone()), span)),
            TokenKind::TemplateString(s) => {
                let parts = parse_template_parts(s)?;
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

            _ => Err(ParseError {
                message: format!("Unexpected token in expression: {:?}", t.kind),
                span,
            }),
        }
    }

    // ── Infix expressions ──────────────────────────────

    fn infix_bp(&self) -> (Bp, Bp) {
        match self.peek_kind() {
            TokenKind::Eq
            | TokenKind::PlusEq
            | TokenKind::MinusEq
            | TokenKind::StarEq
            | TokenKind::SlashEq
            | TokenKind::PercentEq => (1, 2),
            TokenKind::OrOr => (3, 4),
            TokenKind::AndAnd => (5, 6),
            TokenKind::QuestionQuestion => (4, 5),
            TokenKind::QuestionMark => (1, 2),
            TokenKind::EqEq
            | TokenKind::NotEq
            | TokenKind::Lt
            | TokenKind::Gt
            | TokenKind::LtEq
            | TokenKind::GtEq => (7, 8),
            TokenKind::Plus | TokenKind::Minus => (9, 10),
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent => (11, 12),
            TokenKind::Is => (6, 7),
            TokenKind::Dot | TokenKind::QuestionDot | TokenKind::LParen | TokenKind::LBracket => {
                (13, 14)
            }
            _ => (0, 0),
        }
    }

    fn parse_infix(&mut self, left: Expr, bp: (Bp, Bp)) -> Result<Expr, ParseError> {
        let t = self.advance();
        let span = t.span;

        match &t.kind {
            TokenKind::Plus => self.make_binary(left, BinaryOpKind::Add, bp),
            TokenKind::Minus => self.make_binary(left, BinaryOpKind::Sub, bp),
            TokenKind::Star => self.make_binary(left, BinaryOpKind::Mul, bp),
            TokenKind::Slash => self.make_binary(left, BinaryOpKind::Div, bp),
            TokenKind::Percent => self.make_binary(left, BinaryOpKind::Mod, bp),
            TokenKind::EqEq => self.make_binary(left, BinaryOpKind::Eq, bp),
            TokenKind::NotEq => self.make_binary(left, BinaryOpKind::Neq, bp),
            TokenKind::Lt => self.make_binary(left, BinaryOpKind::Lt, bp),
            TokenKind::Gt => self.make_binary(left, BinaryOpKind::Gt, bp),
            TokenKind::LtEq => self.make_binary(left, BinaryOpKind::LtEq, bp),
            TokenKind::GtEq => self.make_binary(left, BinaryOpKind::GtEq, bp),
            TokenKind::AndAnd => self.make_binary(left, BinaryOpKind::And, bp),
            TokenKind::OrOr => self.make_binary(left, BinaryOpKind::Or, bp),
            TokenKind::Is => {
                // Type check: value is TypeName
                let type_name = self.expect_ident("expected type name after 'is'")?;
                Ok(Expr::IsCheck {
                    value: Box::new(left),
                    type_name,
                    span,
                })
            }
            TokenKind::QuestionQuestion => {
                self.make_binary(left, BinaryOpKind::NullishCoalesce, bp)
            }
            TokenKind::QuestionMark => {
                // Ternary expression: condition ? then_expr : else_expr
                let then_expr = self.parse_expr()?;
                self.expect_token(TokenKind::Colon, "expected ':' in ternary expression")?;
                let else_expr = self.parse_expr_bp(bp.1)?;
                let end = span_of(&else_expr).end;
                Ok(Expr::TernaryExpr {
                    condition: Box::new(left),
                    then_expr: Box::new(then_expr),
                    else_expr: Box::new(else_expr),
                    span: Span {
                        start: span.start,
                        end,
                        line: span.line,
                        col: span.col,
                    },
                })
            }
            TokenKind::Eq => self.parse_assign(left, span),
            TokenKind::PlusEq => self.parse_compound_assign(left, BinaryOpKind::AddAssign, span),
            TokenKind::MinusEq => self.parse_compound_assign(left, BinaryOpKind::SubAssign, span),
            TokenKind::StarEq => self.parse_compound_assign(left, BinaryOpKind::MulAssign, span),
            TokenKind::SlashEq => self.parse_compound_assign(left, BinaryOpKind::DivAssign, span),
            TokenKind::PercentEq => self.parse_compound_assign(left, BinaryOpKind::ModAssign, span),

            // Postfix: method call, index, function call, optional chain
            TokenKind::Dot => {
                let next = self.advance();
                let field_name = match &next.kind {
                    TokenKind::Ident(method) => Some(method.clone()),
                    other => keyword_as_ident_name(other).map(|s| s.to_string()),
                };
                match field_name {
                    Some(field) => {
                        if self.at(&TokenKind::LParen) {
                            self.advance(); // consume '('
                            let args = self.parse_call_args()?;
                            Ok(Expr::MethodCall {
                                object: Box::new(left),
                                method: field,
                                args,
                                span: Span {
                                    start: span.start,
                                    end: self.peek().span.start,
                                    line: span.line,
                                    col: span.col,
                                },
                            })
                        } else {
                            Ok(Expr::FieldAccess {
                                object: Box::new(left),
                                field,
                                span: Span {
                                    start: span.start,
                                    end: next.span.end,
                                    line: span.line,
                                    col: span.col,
                                },
                            })
                        }
                    }
                    _ => Err(ParseError {
                        message: "Expected field name after '.'".into(),
                        span: next.span,
                    }),
                }
            }
            TokenKind::QuestionDot => {
                let next = self.advance();
                let field_name = match &next.kind {
                    TokenKind::Ident(field) => Some(field.clone()),
                    other => keyword_as_ident_name(other).map(|s| s.to_string()),
                };
                match field_name {
                    Some(field) => Ok(Expr::OptionalChain {
                        object: Box::new(left),
                        field,
                        span: Span {
                            start: span.start,
                            end: next.span.end,
                            line: span.line,
                            col: span.col,
                        },
                    }),
                    _ => Err(ParseError {
                        message: "Expected field name after '?.'".into(),
                        span: next.span,
                    }),
                }
            }
            TokenKind::LParen => {
                let args = self.parse_call_args()?;
                Ok(Expr::FuncCall {
                    func: Box::new(left),
                    args,
                    span: Span {
                        start: span.start,
                        end: span.end,
                        line: span.line,
                        col: span.col,
                    },
                })
            }
            TokenKind::LBracket => {
                let index = self.parse_expr()?;
                self.expect_close(TokenKind::RBracket, "expected ']'")?;
                Ok(Expr::IndexGet {
                    object: Box::new(left),
                    index: Box::new(index),
                    span,
                })
            }
            _ => Err(ParseError {
                message: format!("Unexpected infix token: {:?}", t.kind),
                span,
            }),
        }
    }

    // ── Helpers ────────────────────────────────────────

    fn make_binary(
        &mut self,
        left: Expr,
        op: BinaryOpKind,
        bp: (Bp, Bp),
    ) -> Result<Expr, ParseError> {
        let right = self.parse_expr_bp(bp.1)?;
        let span = Span {
            start: span_of(&left).start,
            end: span_of(&right).end,
            line: span_of(&left).line,
            col: span_of(&left).col,
        };
        Ok(Expr::BinaryOp {
            left: Box::new(left),
            op,
            right: Box::new(right),
            span,
        })
    }

    fn parse_assign(&mut self, left: Expr, eq_span: Span) -> Result<Expr, ParseError> {
        let right = self.parse_expr_bp(1)?;
        // Assignment is treated as binary with Assign op
        let span = Span {
            start: span_of(&left).start,
            end: span_of(&right).end,
            line: eq_span.line,
            col: eq_span.col,
        };
        Ok(Expr::BinaryOp {
            left: Box::new(left),
            op: BinaryOpKind::Assign,
            right: Box::new(right),
            span,
        })
    }

    fn parse_compound_assign(
        &mut self,
        left: Expr,
        op: BinaryOpKind,
        op_span: Span,
    ) -> Result<Expr, ParseError> {
        let right = self.parse_expr_bp(1)?;
        let span = Span {
            start: span_of(&left).start,
            end: span_of(&right).end,
            line: op_span.line,
            col: op_span.col,
        };
        Ok(Expr::BinaryOp {
            left: Box::new(left),
            op,
            right: Box::new(right),
            span,
        })
    }

    /// Handle an identifier (or contextual keyword) in expression context:
    /// VarRef, FuncCall, or MethodCall.
    fn parse_ident_expr(&mut self, name: String, span: Span) -> Result<Expr, ParseError> {
        if self.at(&TokenKind::LParen) {
            self.advance(); // consume '('
            let args = self.parse_call_args()?;
            Ok(Expr::FuncCall {
                func: Box::new(Expr::VarRef(name.clone(), span)),
                args,
                span: Span {
                    start: span.start,
                    end: self.peek().span.start,
                    line: span.line,
                    col: span.col,
                },
            })
        } else {
            Ok(Expr::VarRef(name, span))
        }
    }

    pub(crate) fn parse_call_args(&mut self) -> Result<Vec<Expr>, ParseError> {
        let mut args = Vec::new();
        if !self.at(&TokenKind::RParen) {
            args.push(self.parse_expr()?);
            while self.at(&TokenKind::Comma) {
                self.advance();
                if self.at(&TokenKind::RParen) {
                    break;
                }
                args.push(self.parse_expr()?);
            }
        }
        self.expect_close(TokenKind::RParen, "expected ')'")?;
        Ok(args)
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

    fn expect_close(&mut self, expected: TokenKind, msg: &str) -> Result<(), ParseError> {
        let t = self.peek();
        if t.kind != expected {
            return Err(ParseError {
                message: format!("{} — found {:?}", msg, t.kind),
                span: t.span,
            });
        }
        self.advance();
        Ok(())
    }
}

/// Get span from any expression node.
pub fn span_of(expr: &Expr) -> &Span {
    match expr {
        Expr::Literal(_, s) => s,
        Expr::VarRef(_, s) => s,
        Expr::BinaryOp { span, .. } => span,
        Expr::UnaryOp { span, .. } => span,
        Expr::FuncCall { span, .. } => span,
        Expr::MethodCall { span, .. } => span,
        Expr::IndexGet { span, .. } => span,
        Expr::FieldAccess { span, .. } => span,
        Expr::OptionalChain { span, .. } => span,
        Expr::ArrayLiteral { span, .. } => span,
        Expr::MapLiteral { span, .. } => span,
        Expr::IfExpr { span, .. } => span,
        Expr::TernaryExpr { span, .. } => span,
        Expr::TemplateString { span, .. } => span,
        Expr::StructLiteral { span, .. } => span,
        Expr::StoreFieldAccess { span, .. } => span,
        Expr::Block { span, .. } => span,
        Expr::ClosureExpr { span, .. } => span,
        Expr::AwaitExpr { span, .. } => span,
        Expr::MatchExpr { span, .. } => span,
        Expr::IsCheck { span, .. } => span,
        Expr::SpreadExpr { span, .. } => span,
        Expr::NotYetImplemented { span, .. } => span,
        Expr::Error { span, .. } => span,
    }
}

/// Parse a template string containing `${expr}` interpolations into parts.
/// Splits the raw string into literal text and parsed expression segments.
fn parse_template_parts(s: &str) -> Result<Vec<TemplatePart>, ParseError> {
    let mut parts: Vec<TemplatePart> = Vec::new();
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut literal_start = 0;

    while i < len {
        if bytes[i] == b'$' && i + 1 < len && bytes[i + 1] == b'{' {
            // Push literal text before this interpolation
            if i > literal_start {
                parts.push(TemplatePart::Literal(s[literal_start..i].to_string()));
            }

            // Skip past ${
            i += 2;

            // Find matching closing brace, tracking nesting
            let mut depth = 1u32;
            let expr_start = i;
            while i < len && depth > 0 {
                match bytes[i] {
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    _ => {}
                }
                if depth > 0 {
                    i += 1;
                }
            }
            if depth != 0 {
                return Err(ParseError {
                    message: "Unterminated template expression".into(),
                    span: Span {
                        start: 0,
                        end: 0,
                        line: 0,
                        col: 0,
                    },
                });
            }
            let expr_str = s[expr_start..i].trim();
            if expr_str.is_empty() {
                return Err(ParseError {
                    message: "Empty template expression".into(),
                    span: Span {
                        start: 0,
                        end: 0,
                        line: 0,
                        col: 0,
                    },
                });
            }

            // Parse the expression string
            let expr = parse_simple_template_expr(expr_str).map_err(|e| ParseError {
                message: format!("In template expression '{}': {}", expr_str, e),
                span: Span {
                    start: 0,
                    end: 0,
                    line: 0,
                    col: 0,
                },
            })?;
            parts.push(TemplatePart::Expr(expr));

            // Move past the closing }
            i += 1;
            literal_start = i;
        } else {
            i += 1;
        }
    }

    // Push trailing literal text
    if literal_start < len {
        parts.push(TemplatePart::Literal(s[literal_start..].to_string()));
    }

    Ok(parts)
}

/// Parse a simple expression inside `${}` in template strings.
/// Handles identifiers, field access, method calls, and function calls.
fn parse_simple_template_expr(s: &str) -> Result<Expr, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty expression".to_string());
    }

    let span = Span {
        start: 0,
        end: 0,
        line: 0,
        col: 0,
    };

    // Helper: parse an identifier at the start of a string
    fn parse_ident(input: &str) -> Option<(String, usize)> {
        let mut i = 0;
        let bytes = input.as_bytes();
        if i < bytes.len() && (bytes[i].is_ascii_alphabetic() || bytes[i] == b'_') {
            i += 1;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            Some((input[..i].to_string(), i))
        } else {
            None
        }
    }

    // Skip whitespace helper
    fn skip_ws(input: &str, pos: &mut usize) {
        while *pos < input.len() && input.as_bytes()[*pos].is_ascii_whitespace() {
            *pos += 1;
        }
    }

    let mut pos = 0;
    skip_ws(s, &mut pos);

    // Parse the primary expression
    let (mut expr, mut pos) = {
        if let Some((name, end)) = parse_ident(&s[pos..]) {
            (Expr::VarRef(name, span), pos + end)
        } else {
            return Err(format!("unexpected token at '{}'", &s[pos..]));
        }
    };

    // Chain field access / method calls
    skip_ws(s, &mut pos);
    while pos < s.len() {
        match s.as_bytes()[pos] {
            b'.' => {
                pos += 1;
                skip_ws(s, &mut pos);
                if let Some((field, end)) = parse_ident(&s[pos..]) {
                    // Check if it's a method call
                    let mut new_pos = pos + end;
                    skip_ws(s, &mut new_pos);
                    if new_pos < s.len() && s.as_bytes()[new_pos] == b'(' {
                        // Method call
                        pos = new_pos;
                        let mut args = Vec::new();
                        pos += 1; // skip (
                        let arg_str = &s[pos..];
                        let mut depth = 0u32;
                        let mut arg_end = 0;
                        let mut found_close = false;
                        for (j, ch) in arg_str.char_indices() {
                            match ch {
                                '(' => depth += 1,
                                ')' if depth == 0 => {
                                    arg_end = j;
                                    found_close = true;
                                    break;
                                }
                                ')' => depth -= 1,
                                _ => {}
                            }
                        }
                        if !found_close {
                            return Err("unterminated method call".to_string());
                        }
                        let args_section = arg_str[..arg_end].trim();
                        if !args_section.is_empty() {
                            for arg in args_section.split(',') {
                                let arg = arg.trim();
                                if !arg.is_empty() {
                                    let parsed = parse_simple_template_expr(arg)?;
                                    args.push(parsed);
                                }
                            }
                        }
                        expr = Expr::MethodCall {
                            object: Box::new(expr),
                            method: field,
                            args,
                            span,
                        };
                        pos += arg_end + 1; // skip past )
                    } else {
                        // Simple field access
                        expr = Expr::FieldAccess {
                            object: Box::new(expr),
                            field,
                            span,
                        };
                        pos = new_pos;
                    }
                } else {
                    return Err(format!("expected identifier after '.' at '{}'", &s[pos..]));
                }
            }
            b'(' => {
                // Function call on the current expression
                pos += 1;
                let arg_str = &s[pos..];
                let mut args = Vec::new();
                let mut depth = 0u32;
                let mut arg_end = 0;
                let mut found_close = false;
                for (j, ch) in arg_str.char_indices() {
                    match ch {
                        '(' => depth += 1,
                        ')' if depth == 0 => {
                            arg_end = j;
                            found_close = true;
                            break;
                        }
                        ')' => depth -= 1,
                        _ => {}
                    }
                }
                if !found_close {
                    return Err("unterminated function call".to_string());
                }
                let args_section = arg_str[..arg_end].trim();
                if !args_section.is_empty() {
                    for arg in args_section.split(',') {
                        let arg = arg.trim();
                        if !arg.is_empty() {
                            args.push(parse_simple_template_expr(arg)?);
                        }
                    }
                }
                expr = Expr::FuncCall {
                    func: Box::new(expr),
                    args,
                    span,
                };
                pos += arg_end + 1;
            }
            _ => break,
        }
        skip_ws(s, &mut pos);
    }

    Ok(expr)
}
