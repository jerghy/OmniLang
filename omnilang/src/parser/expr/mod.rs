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
        if self.peek_kind() == TokenKind::LBrace {
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
        let span_start = self.peek().span.clone();
        self.advance(); // consume '{'

        let mut fields: Vec<(String, Expr)> = Vec::new();
        if self.peek_kind() != TokenKind::RBrace {
            loop {
                let field_name = match self.advance().kind {
                    TokenKind::Ident(name) => name,
                    other => {
                        return Err(ParseError {
                            message: format!(
                                "Expected field name in struct literal, got {:?}",
                                other
                            ),
                            span: self.peek().span.clone(),
                        });
                    }
                };
                self.expect_token(TokenKind::Colon, "expected ':' after field name")?;
                let value = self.parse_expr()?;
                fields.push((field_name, value));
                if self.peek_kind() == TokenKind::RBrace {
                    break;
                }
                self.expect_token(TokenKind::Comma, "expected ',' or '}' after field value")?;
                if self.peek_kind() == TokenKind::RBrace {
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
        let span = t.span.clone();
        match &t.kind {
            TokenKind::Ident(name) if name == "_" => Ok(MatchPattern {
                kind: MatchPatternKind::Wildcard,
                span,
            }),
            TokenKind::Ident(name) => {
                // Check for variant with bindings: Ident { binding, ... }
                if self.peek_kind() == TokenKind::LBrace {
                    self.advance(); // consume '{'
                    let mut bindings: Vec<String> = Vec::new();
                    if self.peek_kind() != TokenKind::RBrace {
                        loop {
                            if let TokenKind::Ident(b) = self.advance().kind {
                                bindings.push(b);
                            } else {
                                return Err(ParseError {
                                    message: "expected binding name in pattern".into(),
                                    span: self.peek().span.clone(),
                                });
                            }
                            if self.peek_kind() == TokenKind::RBrace {
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
        if self.peek_kind() == TokenKind::LBrace {
            self.parse_block()
        } else {
            let expr = self.parse_expr()?;
            let span = span_of(&expr).clone();
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
        let span = t.span.clone();

        match &t.kind {
            TokenKind::IntLiteral(n) => Ok(Expr::Literal(Literal::Int(*n), span)),
            TokenKind::FloatLiteral(n) => Ok(Expr::Literal(Literal::Float(*n), span)),
            TokenKind::StringLiteral(s) => Ok(Expr::Literal(Literal::String(s.clone()), span)),
            TokenKind::TemplateString(s) => {
                // Template string without interpolation → treat as regular string
                // TODO: parse ${expr} parts for proper template interpolation
                Ok(Expr::Literal(Literal::String(s.clone()), span))
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
                let else_expr = if self.peek_kind() == TokenKind::If {
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
                while self.peek_kind() != TokenKind::RBrace {
                    let pattern = self.parse_match_pattern()?;
                    self.expect_token(TokenKind::FatArrow, "expected '=>' in match arm")?;
                    let body = self.parse_match_body()?;
                    arms.push(MatchArm { pattern, body });
                    // Optional trailing comma
                    if self.peek_kind() == TokenKind::Comma {
                        self.advance();
                    }
                    if self.peek_kind() == TokenKind::RBrace {
                        break;
                    }
                }
                let end_span = self.peek().span.clone();
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
            kind if keyword_as_ident_name(kind).is_some() => {
                let name = keyword_as_ident_name(kind)
                    .expect("keyword_as_ident_name returned Some as checked above");
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
                        while self.peek_kind() != TokenKind::RBrace
                            && self.peek_kind() != TokenKind::Eof
                        {
                            let stmt = self.parse_stmt()?;
                            if self.peek_kind() == TokenKind::RBrace {
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
                // Spread expression: ...expr (reserved for P2)
                // Parse the spread target to avoid leaving unconsumed tokens
                let _target = self.parse_expr()?;
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
                    other => keyword_as_ident_name(other),
                };
                match field_name {
                    Some(field) => {
                        if self.peek_kind() == TokenKind::LParen {
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
                        span: next.span.clone(),
                    }),
                }
            }
            TokenKind::QuestionDot => {
                let next = self.advance();
                let field_name = match &next.kind {
                    TokenKind::Ident(field) => Some(field.clone()),
                    other => keyword_as_ident_name(other),
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

    /// Handle an identifier (or contextual keyword) in expression context:
    /// VarRef, FuncCall, or MethodCall.
    fn parse_ident_expr(&mut self, name: String, span: Span) -> Result<Expr, ParseError> {
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

    pub(crate) fn parse_call_args(&mut self) -> Result<Vec<Expr>, ParseError> {
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
        Expr::NotYetImplemented { span, .. } => span,
        Expr::Error { span, .. } => span,
    }
}
