//! Recursive descent statement parser.

use crate::error::ParseError;
use crate::lexer::{Span, TokenKind};
use crate::parser::ast::*;
use crate::parser::expr::span_of;
use crate::parser::Parser;

impl<'a> Parser<'a> {
    /// Parse a block: { stmt* expr? }
    pub fn parse_block(&mut self) -> Result<Block, ParseError> {
        self.expect_token(TokenKind::LBrace, "expected '{'")?;
        let start_span = self.peek_span();

        let mut stmts = Vec::new();
        let mut trailing_expr: Option<Expr> = None;

        while self.peek_kind() != TokenKind::RBrace && self.peek_kind() != TokenKind::Eof {
            let stmt = self.parse_stmt()?;
            // Check if this statement is actually a trailing expression
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

        Ok(Block {
            stmts,
            trailing_expr: trailing_expr.map(Box::new),
            span: Span {
                start: start_span.start,
                end: end_span.end,
                line: start_span.line,
                col: start_span.col,
            },
        })
    }

    /// Parse a single statement.
    pub fn parse_stmt(&mut self) -> Result<Stmt, ParseError> {
        match self.peek_kind() {
            TokenKind::Let => self.parse_var_decl(),
            TokenKind::If => self.parse_if_stmt(),
            TokenKind::While => self.parse_while_stmt(),
            TokenKind::For => self.parse_for_stmt(),
            TokenKind::Return => self.parse_return_stmt(),
            TokenKind::Break => self.parse_break_stmt(),
            TokenKind::Continue => self.parse_continue_stmt(),
            TokenKind::Print | TokenKind::Println => self.parse_print_stmt(),
            TokenKind::LBrace => {
                let block = self.parse_block()?;
                Ok(Stmt::ExprStmt {
                    expr: Expr::NotYetImplemented {
                        feature: "block expression".into(),
                        span: block.span.clone(),
                    },
                    span: block.span,
                })
            }
            TokenKind::Semicolon => {
                self.advance();
                // Empty statement
                Ok(Stmt::ExprStmt {
                    expr: Expr::VarRef(
                        "".into(),
                        Span {
                            start: 0,
                            end: 0,
                            line: 0,
                            col: 0,
                        },
                    ),
                    span: Span {
                        start: 0,
                        end: 0,
                        line: 0,
                        col: 0,
                    },
                })
            }
            _ => {
                // Expression statement or assignment
                let expr = self.parse_expr()?;
                let span = span_of(&expr).clone();

                // Check for semicolon (optional)
                if self.peek_kind() == TokenKind::Semicolon {
                    self.advance();
                }

                Ok(Stmt::ExprStmt { expr, span })
            }
        }
    }

    // ── Individual statement parsers ───────────────────

    fn parse_var_decl(&mut self) -> Result<Stmt, ParseError> {
        let start = self.peek_span();
        self.advance(); // 'let'
        let mutable = self.peek_kind() == TokenKind::Mut;
        if mutable {
            self.advance();
        }

        let name = self.expect_ident("expected variable name")?;
        let type_ann = if self.peek_kind() == TokenKind::Colon {
            self.advance();
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        let init = if self.peek_kind() == TokenKind::Eq {
            self.advance();
            Some(self.parse_expr()?)
        } else {
            None
        };

        if self.peek_kind() == TokenKind::Semicolon {
            self.advance();
        }

        let end = self.peek_span();
        Ok(Stmt::VarDecl {
            name,
            type_ann,
            mutable,
            init,
            span: Span {
                start: start.start,
                end: end.end,
                line: start.line,
                col: start.col,
            },
        })
    }

    fn parse_if_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.peek_span();
        self.advance(); // 'if'
        let condition = self.parse_expr()?;
        let then_body = self.parse_block()?;

        let mut else_ifs = Vec::new();
        let mut else_body = None;

        while self.peek_kind() == TokenKind::Else {
            self.advance();
            if self.peek_kind() == TokenKind::If {
                self.advance();
                let cond = self.parse_expr()?;
                let body = self.parse_block()?;
                else_ifs.push((cond, body));
            } else {
                else_body = Some(self.parse_block()?);
                break;
            }
        }

        Ok(Stmt::If {
            condition,
            then_body,
            else_ifs,
            else_body,
            span: Span {
                start: start.start,
                end: self.peek_span().end,
                line: start.line,
                col: start.col,
            },
        })
    }

    fn parse_while_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.peek_span();
        self.advance(); // 'while'
        let condition = self.parse_expr()?;
        let body = self.parse_block()?;
        Ok(Stmt::While {
            condition,
            body,
            span: Span {
                start: start.start,
                end: start.end,
                line: start.line,
                col: start.col,
            },
        })
    }

    fn parse_for_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.peek_span();
        self.advance(); // 'for'
        let var_name = self.expect_ident("expected loop variable")?;
        self.expect_token(TokenKind::In, "expected 'in'")?;
        let iterable = self.parse_expr()?;
        let body = self.parse_block()?;
        Ok(Stmt::ForIn {
            var_name,
            iterable,
            body,
            span: Span {
                start: start.start,
                end: start.end,
                line: start.line,
                col: start.col,
            },
        })
    }

    fn parse_return_stmt(&mut self) -> Result<Stmt, ParseError> {
        let span = self.peek_span();
        self.advance(); // 'return'
        let value =
            if self.peek_kind() != TokenKind::Semicolon && self.peek_kind() != TokenKind::RBrace {
                Some(self.parse_expr()?)
            } else {
                None
            };
        if self.peek_kind() == TokenKind::Semicolon {
            self.advance();
        }
        Ok(Stmt::Return { value, span })
    }

    fn parse_break_stmt(&mut self) -> Result<Stmt, ParseError> {
        let span = self.peek_span();
        self.advance();
        if self.peek_kind() == TokenKind::Semicolon {
            self.advance();
        }
        Ok(Stmt::Break(span))
    }

    fn parse_continue_stmt(&mut self) -> Result<Stmt, ParseError> {
        let span = self.peek_span();
        self.advance();
        if self.peek_kind() == TokenKind::Semicolon {
            self.advance();
        }
        Ok(Stmt::Continue(span))
    }

    fn parse_print_stmt(&mut self) -> Result<Stmt, ParseError> {
        let newline = self.peek_kind() == TokenKind::Println;
        let span = self.peek_span();
        self.advance(); // 'print' or 'println'
        let arg = self.parse_expr()?;
        if self.peek_kind() == TokenKind::Semicolon {
            self.advance();
        }
        Ok(Stmt::Print { newline, arg, span })
    }

    // ── Type annotation parser ─────────────────────────

    pub fn parse_type_annotation(&mut self) -> Result<TypeAnnotation, ParseError> {
        let start = self.peek_span();
        let name = self.expect_ident("expected type name")?;

        // Check for generics: Array<String>, Map<String, i32>, etc.
        let type_args = if self.peek_kind() == TokenKind::Lt {
            self.advance();
            let mut args = vec![self.parse_type_annotation()?];
            while self.peek_kind() == TokenKind::Comma {
                self.advance();
                args.push(self.parse_type_annotation()?);
            }
            self.expect_token(TokenKind::Gt, "expected '>'")?;
            args
        } else {
            vec![]
        };

        // Check for optional: Type?
        let is_optional = self.peek_kind() == TokenKind::QuestionMark;
        if is_optional {
            self.advance();
        }

        let span = Span {
            start: start.start,
            end: start.end,
            line: start.line,
            col: start.col,
        };

        if is_optional {
            Ok(TypeAnnotation::Optional {
                base: Box::new(TypeAnnotation::Named {
                    name,
                    type_args,
                    span,
                }),
                span: self.peek_span().clone(),
            })
        } else {
            Ok(TypeAnnotation::Named {
                name,
                type_args,
                span,
            })
        }
    }

    // ── Helpers ────────────────────────────────────────

    pub(crate) fn expect_token(
        &mut self,
        expected: TokenKind,
        msg: &str,
    ) -> Result<(), ParseError> {
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

    /// Try to parse an identifier (including contextual keywords).
    /// Returns Some(name) and consumes the token, or None without consuming.
    pub(crate) fn try_parse_ident(&mut self) -> Option<String> {
        let t = self.peek();
        match &t.kind {
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance();
                Some(name)
            }
            _ => {
                if let Some(name) = keyword_as_ident_name(&t.kind) {
                    self.advance();
                    Some(name)
                } else {
                    None
                }
            }
        }
    }

    pub(crate) fn expect_ident(&mut self, msg: &str) -> Result<String, ParseError> {
        let t = self.advance();
        match &t.kind {
            TokenKind::Ident(name) => Ok(name.clone()),
            _ => {
                // Accept contextual keywords as identifiers
                if let Some(name) = keyword_as_ident_name(&t.kind) {
                    return Ok(name);
                }
                Err(ParseError {
                    message: format!("{} — found {:?}", msg, t.kind),
                    span: t.span,
                })
            }
        }
    }

    pub(crate) fn peek_span(&self) -> Span {
        self.tokens
            .get(self.pos)
            .map(|t| t.span.clone())
            .unwrap_or(Span {
                start: 0,
                end: 0,
                line: 0,
                col: 0,
            })
    }
}

/// Convert keyword tokens to their identifier name string when used in context
/// where identifiers are expected (parameter names, variable names, etc.).
/// Returns None for tokens that are not valid as identifiers.
pub(crate) fn keyword_as_ident_name(kind: &TokenKind) -> Option<String> {
    match kind {
        // OmniView keywords that can be used as variable/param names
        TokenKind::Event => Some("event".into()),
        TokenKind::Signal => Some("signal".into()),
        TokenKind::Computed => Some("computed".into()),
        TokenKind::Prop => Some("prop".into()),
        TokenKind::Ref => Some("ref".into()),
        TokenKind::Store => Some("store".into()),
        TokenKind::Action => Some("action".into()),
        TokenKind::Getter => Some("getter".into()),
        TokenKind::View => Some("view".into()),
        TokenKind::Style => Some("style".into()),
        TokenKind::Theme => Some("theme".into()),
        TokenKind::Component => Some("component".into()),
        TokenKind::Bind => Some("bind".into()),
        // om nilang keywords
        TokenKind::Fn => Some("fn".into()),
        TokenKind::Const => Some("const".into()),
        TokenKind::Let => Some("let".into()),
        TokenKind::Mut => Some("mut".into()),
        TokenKind::If => Some("if".into()),
        TokenKind::Else => Some("else".into()),
        TokenKind::While => Some("while".into()),
        TokenKind::For => Some("for".into()),
        TokenKind::In => Some("in".into()),
        TokenKind::Return => Some("return".into()),
        TokenKind::Async => Some("async".into()),
        TokenKind::Await => Some("await".into()),
        TokenKind::Match => Some("match".into()),
        TokenKind::Struct => Some("struct".into()),
        TokenKind::Class => Some("class".into()),
        TokenKind::Interface => Some("interface".into()),
        TokenKind::Impl => Some("impl".into()),
        TokenKind::Extends => Some("extends".into()),
        TokenKind::Implements => Some("implements".into()),
        TokenKind::Type => Some("type".into()),
        TokenKind::Nominal => Some("nominal".into()),
        TokenKind::Static => Some("static".into()),
        TokenKind::Private => Some("private".into()),
        TokenKind::Override => Some("override".into()),
        TokenKind::Import => Some("import".into()),
        TokenKind::From => Some("from".into()),
        TokenKind::Export => Some("export".into()),
        TokenKind::Print => Some("print".into()),
        TokenKind::Println => Some("println".into()),
        // Literal keywords
        TokenKind::True => Some("true".into()),
        TokenKind::False => Some("false".into()),
        TokenKind::Null => Some("null".into()),
        _ => None,
    }
}
