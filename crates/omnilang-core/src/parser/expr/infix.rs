//! Infix expression parsing (Pratt parser infix parselets).
//!
//! Handles binary operators, method calls, index access, assignments,
//! and all expressions that appear between their operands.

use crate::error::ParseError;
use crate::lexer::{Span, TokenKind};
use crate::parser::ast::*;
use crate::parser::stmt::keyword_as_ident_name;
use crate::parser::Parser;

use super::span_of;
use super::Bp;

impl<'a> Parser<'a> {
    pub(super) fn infix_bp(&self) -> (Bp, Bp) {
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

    pub(super) fn parse_infix(&mut self, left: Expr, bp: (Bp, Bp)) -> Result<Expr, ParseError> {
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
                        kind: None,
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
                        kind: None,
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
                kind: None,
                message: format!("Unexpected infix token: {:?}", t.kind),
                span,
            }),
        }
    }

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
}
