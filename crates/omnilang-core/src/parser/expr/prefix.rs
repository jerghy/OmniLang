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

    /// Handle an identifier (or contextual keyword) in expression context:
    /// VarRef, FuncCall, or MethodCall.
    pub(super) fn parse_ident_expr(&mut self, name: String, span: Span) -> Result<Expr, ParseError> {
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
