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
use crate::parser::Parser;

/// Binding power: (left_bp, right_bp). Higher = tighter.
type Bp = u8;

impl<'a> Parser<'a> {
    /// Parse an expression starting at current position. Uses Pratt parsing.
    pub fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_expr_bp(0)
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

    // ── Prefix expressions ─────────────────────────────

    fn parse_prefix(&mut self) -> Result<Expr, ParseError> {
        let t = self.advance();
        let span = t.span.clone();

        match &t.kind {
            TokenKind::IntLiteral(n) => Ok(Expr::Literal(Literal::Int(*n), span)),
            TokenKind::FloatLiteral(n) => Ok(Expr::Literal(Literal::Float(*n), span)),
            TokenKind::StringLiteral(s) => Ok(Expr::Literal(Literal::String(s.clone()), span)),
            TokenKind::True => Ok(Expr::Literal(Literal::Bool(true), span)),
            TokenKind::False => Ok(Expr::Literal(Literal::Bool(false), span)),
            TokenKind::Null => Ok(Expr::Literal(Literal::Null, span)),

            TokenKind::Ident(name) => {
                let name = name.clone();
                // Check for function call
                if self.peek_kind() == TokenKind::LParen {
                    self.advance(); // consume '('
                    let args = self.parse_call_args()?;
                    Ok(Expr::FuncCall {
                        func: Box::new(Expr::VarRef(name.clone(), span.clone())),
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
                if self.peek_kind() != TokenKind::RBracket {
                    elements.push(self.parse_expr()?);
                    while self.peek_kind() == TokenKind::Comma {
                        self.advance();
                        if self.peek_kind() == TokenKind::RBracket {
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
                // Map literal: {key: val, ...} or Struct literal: TypeName{field: val, ...}
                // For now, always parse as map literal
                let entries = self.parse_map_entries()?;
                Ok(Expr::MapLiteral {
                    entries,
                    span: Span {
                        start: span.start,
                        end: span.end,
                        line: span.line,
                        col: span.col,
                    },
                })
            }

            TokenKind::If => {
                // If expression: if condition { expr } else { expr }
                let condition = self.parse_expr()?;
                self.expect_close(TokenKind::LBrace, "expected '{'")?;
                let then_expr = self.parse_expr()?;
                self.expect_close(TokenKind::RBrace, "expected '}'")?;
                self.expect_close(TokenKind::Else, "expected 'else'")?;
                self.expect_close(TokenKind::LBrace, "expected '{'")?;
                let else_expr = self.parse_expr()?;
                self.expect_close(TokenKind::RBrace, "expected '}'")?;
                Ok(Expr::IfExpr {
                    condition: Box::new(condition),
                    then_expr: Box::new(then_expr),
                    else_expr: Box::new(else_expr),
                    span,
                })
            }

            TokenKind::DotDotDot => {
                // Spread expression: ...expr (reserved for P2)
                // Parse the spread target, but mark as not yet implemented
                Ok(Expr::NotYetImplemented {
                    feature: "spread".into(),
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
            TokenKind::EqEq
            | TokenKind::NotEq
            | TokenKind::Lt
            | TokenKind::Gt
            | TokenKind::LtEq
            | TokenKind::GtEq => (7, 8),
            TokenKind::Plus | TokenKind::Minus => (9, 10),
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent => (11, 12),
            TokenKind::Dot | TokenKind::QuestionDot | TokenKind::LParen | TokenKind::LBracket => {
                (13, 14)
            }
            _ => (0, 0),
        }
    }

    fn parse_infix(&mut self, left: Expr, bp: (Bp, Bp)) -> Result<Expr, ParseError> {
        let t = self.advance();
        let span = t.span.clone();

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
            TokenKind::Eq => self.parse_assign(left, span),
            TokenKind::PlusEq => self.parse_compound_assign(left, BinaryOpKind::AddAssign, span),
            TokenKind::MinusEq => self.parse_compound_assign(left, BinaryOpKind::SubAssign, span),
            TokenKind::StarEq => self.parse_compound_assign(left, BinaryOpKind::MulAssign, span),
            TokenKind::SlashEq => self.parse_compound_assign(left, BinaryOpKind::DivAssign, span),
            TokenKind::PercentEq => self.parse_compound_assign(left, BinaryOpKind::ModAssign, span),

            // Postfix: method call, index, function call
            TokenKind::Dot | TokenKind::QuestionDot => {
                let next = self.advance();
                match &next.kind {
                    TokenKind::Ident(method) => {
                        if self.peek_kind() == TokenKind::LParen {
                            self.advance(); // consume '('
                            let args = self.parse_call_args()?;
                            Ok(Expr::MethodCall {
                                object: Box::new(left),
                                method: method.clone(),
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
                                field: method.clone(),
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
                        span: next.span.clone(),
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

    fn parse_call_args(&mut self) -> Result<Vec<Expr>, ParseError> {
        let mut args = Vec::new();
        if self.peek_kind() != TokenKind::RParen {
            args.push(self.parse_expr()?);
            while self.peek_kind() == TokenKind::Comma {
                self.advance();
                if self.peek_kind() == TokenKind::RParen {
                    break;
                }
                args.push(self.parse_expr()?);
            }
        }
        self.expect_close(TokenKind::RParen, "expected ')'")?;
        Ok(args)
    }

    fn parse_map_entries(&mut self) -> Result<Vec<(Expr, Expr)>, ParseError> {
        let mut entries = Vec::new();
        if self.peek_kind() != TokenKind::RBrace {
            let key = self.parse_expr()?;
            self.expect_close(TokenKind::Colon, "expected ':'")?;
            let val = self.parse_expr()?;
            entries.push((key, val));
            while self.peek_kind() == TokenKind::Comma {
                self.advance();
                if self.peek_kind() == TokenKind::RBrace {
                    break;
                }
                let k = self.parse_expr()?;
                self.expect_close(TokenKind::Colon, "expected ':'")?;
                let v = self.parse_expr()?;
                entries.push((k, v));
            }
        }
        self.expect_close(TokenKind::RBrace, "expected '}'")?;
        Ok(entries)
    }

    fn expect_close(&mut self, expected: TokenKind, msg: &str) -> Result<(), ParseError> {
        let t = self.advance();
        if t.kind != expected {
            return Err(ParseError {
                message: format!("{} — found {:?}", msg, t.kind),
                span: t.span,
            });
        }
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
        Expr::ArrayLiteral { span, .. } => span,
        Expr::MapLiteral { span, .. } => span,
        Expr::IfExpr { span, .. } => span,
        Expr::TernaryExpr { span, .. } => span,
        Expr::TemplateString { span, .. } => span,
        Expr::StructLiteral { span, .. } => span,
        Expr::StoreFieldAccess { span, .. } => span,
        Expr::NotYetImplemented { span, .. } => span,
    }
}
