//! Recursive descent statement parser.

use crate::error::ParseError;
use crate::lexer::{Span, TokenKind};
use crate::parser::ast::*;
use crate::parser::expr::span_of;
use crate::parser::Parser;

impl<'a> Parser<'a> {
    /// Parse a block: { stmt* expr? }
    pub fn parse_block(&mut self) -> Result<Block, ParseError> {
        self.enter_recursion()?;
        self.expect_token(TokenKind::LBrace, "expected '{'")?;
        let start_span = self.peek_span();

        let mut stmts = Vec::new();
        let mut trailing_expr: Option<Expr> = None;

        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            match self.parse_stmt() {
                Ok(stmt) => {
                    // Check if this statement is actually a trailing expression
                    if self.at(&TokenKind::RBrace) {
                        if let Stmt::ExprStmt { expr, .. } = &stmt {
                            trailing_expr = Some(expr.clone());
                            break;
                        }
                    }
                    stmts.push(stmt);
                }
                Err(e) => {
                    self.emit_error(e);
                    self.recover_from_error();
                    if self.at(&TokenKind::RBrace) {
                        break;
                    }
                }
            }
        }

        let end_span = self.peek_span();
        self.expect_token(TokenKind::RBrace, "expected '}'")?;

        let block = Block {
            stmts,
            trailing_expr: trailing_expr.map(Box::new),
            span: Span {
                start: start_span.start,
                end: end_span.end,
                line: start_span.line,
                col: start_span.col,
            },
        };
        self.leave_recursion();
        Ok(block)
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
            TokenKind::Throw => self.parse_throw_stmt(),
            TokenKind::Try => self.parse_try_catch_stmt(),
            TokenKind::Emit => self.parse_emit_stmt(),
            TokenKind::LBrace => {
                let block = self.parse_block()?;
                let span = block.span;
                Ok(Stmt::ExprStmt {
                    expr: Expr::Block { body: block, span },
                    span,
                })
            }
            TokenKind::Semicolon => {
                self.advance();
                // Empty statement — use a null literal instead of VarRef("")
                let span = Span {
                    start: 0,
                    end: 0,
                    line: 0,
                    col: 0,
                };
                Ok(Stmt::ExprStmt {
                    expr: Expr::Literal(crate::parser::ast::Literal::Null, span),
                    span,
                })
            }
            _ => {
                // Expression statement or assignment
                match self.parse_expr() {
                    Ok(expr) => {
                        let span = *span_of(&expr);

                        // Check for semicolon (optional)
                        self.advance_if(&TokenKind::Semicolon);

                        Ok(Stmt::ExprStmt { expr, span })
                    }
                    Err(e) => {
                        let span = e.span;
                        self.emit_error(e);
                        self.recover_from_error();
                        self.advance_if(&TokenKind::Semicolon);
                        Ok(Stmt::Error {
                            message: "expression parse error".to_string(),
                            span,
                        })
                    }
                }
            }
        }
    }

    // ── Individual statement parsers ───────────────────

    fn parse_var_decl(&mut self) -> Result<Stmt, ParseError> {
        let start = self.peek_span();
        self.advance(); // 'let'
        let mutable = self.at(&TokenKind::Mut);
        if mutable {
            self.advance();
        }

        let name = self.expect_ident("expected variable name")?;
        let type_ann = if self.at(&TokenKind::Colon) {
            self.advance();
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        let init = if self.at(&TokenKind::Eq) {
            self.advance();
            Some(self.parse_expr()?)
        } else {
            None
        };

        self.advance_if(&TokenKind::Semicolon);

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

        while self.at(&TokenKind::Else) {
            self.advance();
            if self.at(&TokenKind::If) {
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
        let value = if !self.at(&TokenKind::Semicolon) && !self.at(&TokenKind::RBrace) {
            Some(self.parse_expr()?)
        } else {
            None
        };
        self.advance_if(&TokenKind::Semicolon);
        Ok(Stmt::Return { value, span })
    }

    fn parse_break_stmt(&mut self) -> Result<Stmt, ParseError> {
        let span = self.peek_span();
        self.advance();
        self.advance_if(&TokenKind::Semicolon);
        Ok(Stmt::Break(span))
    }

    fn parse_continue_stmt(&mut self) -> Result<Stmt, ParseError> {
        let span = self.peek_span();
        self.advance();
        self.advance_if(&TokenKind::Semicolon);
        Ok(Stmt::Continue(span))
    }

    fn parse_print_stmt(&mut self) -> Result<Stmt, ParseError> {
        let newline = self.at(&TokenKind::Println);
        let span = self.peek_span();
        self.advance(); // 'print' or 'println'
        let arg = self.parse_expr()?;
        self.advance_if(&TokenKind::Semicolon);
        Ok(Stmt::Print { newline, arg, span })
    }

    // ── Type annotation parser ─────────────────────────

    pub fn parse_type_annotation(&mut self) -> Result<TypeAnnotation, ParseError> {
        let start = self.peek_span();

        // Check for fn type: fn(param_type) -> return_type
        if self.peek_ident_value().is_some_and(|s| s == "fn") {
            return self.parse_fn_type_annotation(start);
        }

        let name = self.expect_ident("expected type name")?;

        // Check for generics: Array<String>, Map<String, i32>, etc.
        let type_args = if self.at(&TokenKind::Lt) {
            self.advance();
            let mut args = vec![self.parse_type_annotation()?];
            while self.at(&TokenKind::Comma) {
                self.advance();
                args.push(self.parse_type_annotation()?);
            }
            self.expect_token(TokenKind::Gt, "expected '>'")?;
            args
        } else {
            vec![]
        };

        // Check for optional: Type?
        let is_optional = self.at(&TokenKind::QuestionMark);
        if is_optional {
            self.advance();
        }

        let base = TypeAnnotation::Named {
            name,
            type_args,
            span: Span {
                start: start.start,
                end: start.end,
                line: start.line,
                col: start.col,
            },
        };

        let base = if is_optional {
            TypeAnnotation::Optional {
                base: Box::new(base),
                span: self.peek_span(),
            }
        } else {
            base
        };

        // Check for array type: Type[]
        if self.at(&TokenKind::LBracket) {
            self.advance(); // consume '['
            self.expect_token(TokenKind::RBracket, "expected ']'")?;
            let array_span = self.peek_span();
            return Ok(TypeAnnotation::Named {
                name: "Array".into(),
                type_args: vec![base],
                span: Span {
                    start: start.start,
                    end: array_span.end,
                    line: start.line,
                    col: start.col,
                },
            });
        }

        // Check for inline union: Type1 | Type2 | Type3
        if self.at(&TokenKind::Pipe) {
            let union_start = start;
            let mut types = vec![base];
            while self.at(&TokenKind::Pipe) {
                self.advance(); // consume '|'
                types.push(self.parse_type_annotation()?);
            }
            let end = self.peek_span();
            Ok(TypeAnnotation::Union {
                types,
                span: Span {
                    start: union_start.start,
                    end: end.end,
                    line: union_start.line,
                    col: union_start.col,
                },
            })
        } else {
            Ok(base)
        }
    }

    /// Parse fn type annotation: fn(param_type) -> return_type
    fn parse_fn_type_annotation(&mut self, start: Span) -> Result<TypeAnnotation, ParseError> {
        self.advance(); // 'fn'
        self.expect_token(TokenKind::LParen, "expected '(' after 'fn'")?;
        let mut params = Vec::new();
        if !self.at(&TokenKind::RParen) {
            params.push(self.parse_type_annotation()?);
            while self.at(&TokenKind::Comma) {
                self.advance();
                if self.at(&TokenKind::RParen) {
                    break;
                }
                params.push(self.parse_type_annotation()?);
            }
        }
        self.expect_token(TokenKind::RParen, "expected ')'")?;
        self.expect_token(TokenKind::Arrow, "expected '->'")?;
        let return_type = self.parse_type_annotation()?;
        let end = self.peek_span();
        Ok(TypeAnnotation::Fn {
            params,
            return_type: Box::new(return_type),
            span: Span {
                start: start.start,
                end: end.end,
                line: start.line,
                col: start.col,
            },
        })
    }

    // ── throw / try-catch ────────────────────────────

    fn parse_throw_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.peek_span();
        self.advance(); // 'throw'
        let expr = self.parse_expr()?;
        self.advance_if(&TokenKind::Semicolon);
        let end = self.peek_span();
        Ok(Stmt::Throw {
            expr,
            span: Span {
                start: start.start,
                end: end.end,
                line: start.line,
                col: start.col,
            },
        })
    }

    fn parse_try_catch_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.peek_span();
        self.advance(); // 'try'
        let try_body = self.parse_block()?;

        let mut catch_var = None;
        let mut catch_body = None;

        if self.at(&TokenKind::Catch) {
            self.advance(); // 'catch'
                            // Optional catch variable: catch err { ... } or catch { ... }
            if !self.at(&TokenKind::LBrace) {
                catch_var = Some(self.expect_ident("expected catch variable")?);
            }
            catch_body = Some(self.parse_block()?);
        }

        let mut finally_body = None;
        if self.at(&TokenKind::Finally) {
            self.advance(); // 'finally'
            finally_body = Some(self.parse_block()?);
        }

        if catch_body.is_none() && finally_body.is_none() {
            return Err(ParseError {
                message: "expected 'catch' or 'finally' after 'try'".into(),
                span: self.peek_span(),
            });
        }

        Ok(Stmt::TryCatch {
            try_body,
            catch_var,
            catch_body,
            finally_body,
            span: Span {
                start: start.start,
                end: self.peek_span().end,
                line: start.line,
                col: start.col,
            },
        })
    }

    // ── Helpers ────────────────────────────────────────

    fn parse_emit_stmt(&mut self) -> Result<Stmt, ParseError> {
        let span = self.peek_span();
        self.advance(); // 'emit'
        let name = self.expect_ident("expected event name after 'emit'")?;
        self.expect_token(TokenKind::LParen, "expected '(' after event name")?;
        let mut args = Vec::new();
        if !self.at(&TokenKind::RParen) {
            args.push(self.parse_expr()?);
            while self.at(&TokenKind::Comma) {
                self.advance();
                args.push(self.parse_expr()?);
            }
        }
        self.expect_token(TokenKind::RParen, "expected ')'")?;
        self.advance_if(&TokenKind::Semicolon);
        Ok(Stmt::EventEmit { name, args, span })
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
                    Some(name.to_string())
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
                    return Ok(name.to_string());
                }
                Err(ParseError {
                    message: format!("{} — found {:?}", msg, t.kind),
                    span: t.span,
                })
            }
        }
    }

    pub(crate) fn peek_span(&self) -> Span {
        self.tokens.get(self.pos).map(|t| t.span).unwrap_or(Span {
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
pub(crate) fn keyword_as_ident_name(kind: &TokenKind) -> Option<&'static str> {
    match kind {
        // OmniView keywords that can be used as variable/param names
        TokenKind::Event => Some("event"),
        TokenKind::Signal => Some("signal"),
        TokenKind::Computed => Some("computed"),
        TokenKind::Prop => Some("prop"),
        TokenKind::Ref => Some("ref"),
        TokenKind::Store => Some("store"),
        TokenKind::Action => Some("action"),
        TokenKind::Getter => Some("getter"),
        TokenKind::View => Some("view"),
        TokenKind::Style => Some("style"),
        TokenKind::Component => Some("component"),
        TokenKind::Bind => Some("bind"),
        TokenKind::Emit => Some("emit"),
        // om nilang keywords
        TokenKind::Fn => Some("fn"),
        TokenKind::Const => Some("const"),
        TokenKind::Let => Some("let"),
        TokenKind::Mut => Some("mut"),
        TokenKind::If => Some("if"),
        TokenKind::Else => Some("else"),
        TokenKind::While => Some("while"),
        TokenKind::For => Some("for"),
        TokenKind::In => Some("in"),
        TokenKind::Return => Some("return"),
        TokenKind::Async => Some("async"),
        TokenKind::Await => Some("await"),
        TokenKind::Is => Some("is"),
        TokenKind::Match => Some("match"),
        TokenKind::Struct => Some("struct"),
        TokenKind::Class => Some("class"),
        TokenKind::Interface => Some("interface"),
        TokenKind::Impl => Some("impl"),
        TokenKind::Extends => Some("extends"),
        TokenKind::Implements => Some("implements"),
        TokenKind::Type => Some("type"),
        TokenKind::Nominal => Some("nominal"),
        TokenKind::Static => Some("static"),
        TokenKind::Private => Some("private"),
        TokenKind::Override => Some("override"),
        TokenKind::Import => Some("import"),
        TokenKind::From => Some("from"),
        TokenKind::Export => Some("export"),
        TokenKind::Print => Some("print"),
        TokenKind::Println => Some("println"),
        // Literal keywords
        TokenKind::True => Some("true"),
        TokenKind::False => Some("false"),
        TokenKind::Null => Some("null"),
        // Control flow keywords that may appear in text context
        TokenKind::Break => Some("break"),
        TokenKind::Continue => Some("continue"),
        TokenKind::Throw => Some("throw"),
        TokenKind::Try => Some("try"),
        TokenKind::Catch => Some("catch"),
        TokenKind::Finally => Some("finally"),
        TokenKind::As => Some("as"),
        TokenKind::Pub => Some("pub"),
        TokenKind::IfMode => Some("if_mode"),
        _ => None,
    }
}
