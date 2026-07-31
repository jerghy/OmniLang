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

pub mod infix;
pub mod motion;
pub mod prefix;
pub mod template;

/// Binding power: (left_bp, right_bp). Higher = tighter.
type Bp = u8;

impl<'a> Parser<'a> {
    /// Parse an expression starting at current position. Uses Pratt parsing.
    ///
    /// Depth accounting lives in `parse_expr_bp` — every expression recursion
    /// path (unary `-`/`!`, `await`, binary ops, assignment, compound-assign,
    /// ternary, parens, call args, index, struct literals) funnels through it.
    pub fn parse_expr(&mut self) -> Result<Expr, ParseError> {
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
        Ok(expr)
    }

    /// Pratt: parse expression with minimum binding power.
    ///
    /// Depth accounting lives here so EVERY recursive Pratt path (unary `-`/`!`,
    /// `await`, binary ops, assignment, compound-assign, ternary) goes through
    /// `MAX_RECURSION_DEPTH`, not just `parse_expr`. Previously only
    /// `parse_expr` entered, so a deep unary chain (`!!!!...`) recursed
    /// unboundedly through `parse_expr_bp` and overflowed the stack (~300
    /// bytes crashed `omnic`).
    ///
    /// `leave_recursion` runs on error paths too: a depth error unwinds the
    /// counter so statement-level recovery can make progress instead of
    /// re-failing immediately at `enter_recursion` forever (which would stall
    /// `parse_block`'s recovery loop without consuming tokens).
    fn parse_expr_bp(&mut self, min_bp: Bp) -> Result<Expr, ParseError> {
        self.enter_recursion()?;
        let result = self.parse_expr_bp_inner(min_bp);
        self.leave_recursion();
        result
    }

    fn parse_expr_bp_inner(&mut self, min_bp: Bp) -> Result<Expr, ParseError> {
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
