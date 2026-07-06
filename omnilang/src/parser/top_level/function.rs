use crate::error::ParseError;
use crate::lexer::{Span, TokenKind};
use crate::parser::ast::*;
use crate::parser::Parser;

impl<'a> Parser<'a> {
    // ── Function / Action ──────────────────────────────

    /// Parse a function after 'fn' keyword has already been consumed.
    pub fn parse_function_def_after_keyword(
        &mut self,
        visibility: Visibility,
        is_async: bool,
    ) -> Result<FunctionDef, ParseError> {
        let start = self.peek_span();
        // 'fn' already consumed by caller
        let name = self.expect_ident("expected function name")?;
        self.expect_token(TokenKind::LParen, "expected '('")?;
        let params = self.parse_params()?;
        self.expect_token(TokenKind::RParen, "expected ')'")?;

        let return_type = if self.peek_kind() == TokenKind::Arrow {
            self.advance();
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        let body = self.parse_block()?;

        Ok(FunctionDef {
            visibility,
            name,
            params,
            return_type,
            is_async,
            body,
            span: Span {
                start: start.start,
                end: start.end,
                line: start.line,
                col: start.col,
            },
        })
    }

    pub fn parse_function_def(
        &mut self,
        visibility: Visibility,
    ) -> Result<FunctionDef, ParseError> {
        let start = self.peek_span();
        self.advance(); // 'fn'
        let name = self.expect_ident("expected function name")?;
        self.expect_token(TokenKind::LParen, "expected '('")?;
        let params = self.parse_params()?;
        self.expect_token(TokenKind::RParen, "expected ')'")?;

        let return_type = if self.peek_kind() == TokenKind::Arrow {
            self.advance();
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        let is_async = if self.peek_kind() == TokenKind::Async {
            self.advance();
            true
        } else {
            false
        };

        let body = self.parse_block()?;

        Ok(FunctionDef {
            visibility,
            name,
            params,
            return_type,
            is_async,
            body,
            span: Span {
                start: start.start,
                end: start.end,
                line: start.line,
                col: start.col,
            },
        })
    }

    pub fn parse_action_def(&mut self) -> Result<ActionDef, ParseError> {
        let start = self.peek_span();
        self.advance(); // 'action'
        let name = self.expect_ident("expected action name")?;
        // Parentheses are optional for actions with no parameters
        let params = if self.peek_kind() == TokenKind::LParen {
            self.advance();
            let p = self.parse_params()?;
            self.expect_token(TokenKind::RParen, "expected ')'")?;
            p
        } else {
            Vec::new()
        };

        let return_type = if self.peek_kind() == TokenKind::Colon {
            self.advance();
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        let is_async = if self.peek_kind() == TokenKind::Async {
            self.advance();
            true
        } else {
            false
        };

        let body = self.parse_block()?;

        Ok(ActionDef {
            name,
            params,
            return_type,
            body,
            is_async,
            span: Span {
                start: start.start,
                end: start.end,
                line: start.line,
                col: start.col,
            },
        })
    }

    pub fn parse_params(&mut self) -> Result<Vec<Param>, ParseError> {
        let mut params = Vec::new();
        if self.peek_kind() != TokenKind::RParen {
            let name = self.expect_ident("expected param name")?;
            let type_ann = if self.peek_kind() == TokenKind::Colon {
                self.advance();
                Some(self.parse_type_annotation()?)
            } else {
                None
            };
            params.push(Param { name, type_ann });

            while self.peek_kind() == TokenKind::Comma {
                self.advance();
                if self.peek_kind() == TokenKind::RParen {
                    break;
                }
                let name = self.expect_ident("expected param name")?;
                let type_ann = if self.peek_kind() == TokenKind::Colon {
                    self.advance();
                    Some(self.parse_type_annotation()?)
                } else {
                    None
                };
                params.push(Param { name, type_ann });
            }
        }
        Ok(params)
    }

    // ── Lifecycle ──────────────────────────────────────

    pub(super) fn parse_lifecycle_hook(&mut self) -> Result<LifecycleHook, ParseError> {
        let kind = match self.peek_kind() {
            TokenKind::AtMount => {
                self.advance();
                LifecycleKind::Mount
            }
            TokenKind::AtUnmount => {
                self.advance();
                LifecycleKind::Unmount
            }
            TokenKind::AtEffect => {
                self.advance();
                let deps = self.parse_effect_deps()?;
                let body = self.parse_block()?;
                return Ok(LifecycleHook {
                    kind: LifecycleKind::Effect,
                    body,
                    deps,
                });
            }
            _ => {
                // Unknown lifecycle directive — report error instead of panicking
                return Err(ParseError {
                    message: format!("unknown lifecycle directive: {:?}", self.peek_kind()),
                    span: self.peek_span(),
                });
            }
        };
        let body = self.parse_block()?;
        Ok(LifecycleHook {
            kind,
            body,
            deps: vec![],
        })
    }

    fn parse_effect_deps(&mut self) -> Result<Vec<String>, ParseError> {
        let mut deps = Vec::new();
        // Parse: (dep1, dep2) — deps can be dot-paths like "TodoStore.items"
        if self.peek_kind() == TokenKind::LParen {
            self.advance();
            if self.peek_kind() != TokenKind::RParen {
                deps.push(self.parse_effect_dep_name()?);
                while self.peek_kind() == TokenKind::Comma {
                    self.advance();
                    if self.peek_kind() == TokenKind::RParen {
                        break;
                    }
                    deps.push(self.parse_effect_dep_name()?);
                }
            }
            self.expect_token(TokenKind::RParen, "expected ')'")?;
        }
        Ok(deps)
    }

    /// Parse a single effect dependency name, which may be a dot-path like `TodoStore.items`.
    fn parse_effect_dep_name(&mut self) -> Result<String, ParseError> {
        let path = self.expect_ident("expected dep name")?;
        if self.peek_kind() == TokenKind::Dot {
            self.advance();
            let field = self.expect_ident("expected field name")?;
            Ok(format!("{}.{}", path, field))
        } else {
            Ok(path)
        }
    }
}
