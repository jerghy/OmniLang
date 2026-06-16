//! Top-level declaration parsers: component, store, function, struct, theme, import.

use crate::error::ParseError;
use crate::lexer::{Span, TokenKind};
use crate::parser::ast::*;
use crate::parser::Parser;

impl<'a> Parser<'a> {
    // ── Component ──────────────────────────────────────

    pub fn parse_component_def(&mut self) -> Result<ComponentDef, ParseError> {
        let start = self.peek_span();
        self.advance(); // 'component'
        let name = self.expect_ident("expected component name")?;
        self.expect_token(TokenKind::LBrace, "expected '{'")?;

        let mut props = Vec::new();
        let mut signals = Vec::new();
        let mut computeds = Vec::new();
        let mut events = Vec::new();
        let mut refs = Vec::new();
        let mut store_refs = Vec::new();
        let mut lifecycles = Vec::new();
        let mut fns = Vec::new();
        let mut actions = Vec::new();
        let mut view: Option<ViewBlock> = None;
        let mut styles = Vec::new();

        while self.peek_kind() != TokenKind::RBrace && self.peek_kind() != TokenKind::Eof {
            match self.peek_kind() {
                TokenKind::Prop => props.push(self.parse_prop_decl()?),
                TokenKind::Signal => signals.push(self.parse_signal_decl()?),
                TokenKind::Computed => computeds.push(self.parse_computed_decl()?),
                TokenKind::Event => events.push(self.parse_event_decl()?),
                TokenKind::Ref => refs.push(self.parse_ref_decl()?),
                TokenKind::Store => {
                    // "store SomeStore" — store reference
                    self.advance();
                    let store_name = self.expect_ident("expected store name")?;
                    store_refs.push(store_name);
                    if self.peek_kind() == TokenKind::Semicolon {
                        self.advance();
                    }
                }
                TokenKind::Fn => fns.push(self.parse_function_def()?),
                TokenKind::Action => actions.push(self.parse_action_def()?),
                TokenKind::AtMount | TokenKind::AtUnmount | TokenKind::AtEffect => {
                    lifecycles.push(self.parse_lifecycle_hook()?)
                }
                TokenKind::View => {
                    if view.is_some() {
                        return Err(ParseError {
                            message: "Duplicate view block".into(),
                            span: self.peek_span(),
                        });
                    }
                    view = Some(self.parse_view_block()?);
                }
                TokenKind::Style => styles.push(self.parse_style_rule()?),
                _ => {
                    let t = self.advance();
                    return Err(ParseError {
                        message: format!("Unexpected token in component: {:?}", t.kind),
                        span: t.span,
                    });
                }
            }
        }

        let end = self.peek_span();
        self.expect_token(TokenKind::RBrace, "expected '}'")?;

        Ok(ComponentDef {
            name,
            props,
            signals,
            computeds,
            events,
            refs,
            store_refs,
            lifecycles,
            fns,
            actions,
            view,
            styles,
            span: Span {
                start: start.start,
                end: end.end,
                line: start.line,
                col: start.col,
            },
        })
    }

    fn parse_prop_decl(&mut self) -> Result<PropDecl, ParseError> {
        self.advance(); // 'prop'
        let name = self.expect_ident("expected prop name")?;
        self.expect_token(TokenKind::Colon, "expected ':'")?;
        let type_ann = self.parse_type_annotation()?;
        let default = if self.peek_kind() == TokenKind::Eq {
            self.advance();
            Some(self.parse_expr()?)
        } else {
            None
        };
        if self.peek_kind() == TokenKind::Semicolon {
            self.advance();
        }
        Ok(PropDecl {
            name,
            type_ann,
            default,
        })
    }

    fn parse_signal_decl(&mut self) -> Result<SignalDecl, ParseError> {
        self.advance(); // 'signal'
        let name = self.expect_ident("expected signal name")?;
        self.expect_token(TokenKind::Colon, "expected ':'")?;
        let type_ann = self.parse_type_annotation()?;
        self.expect_token(TokenKind::Eq, "expected '='")?;
        let init = self.parse_expr()?;
        if self.peek_kind() == TokenKind::Semicolon {
            self.advance();
        }
        Ok(SignalDecl {
            name,
            type_ann,
            init,
        })
    }

    fn parse_computed_decl(&mut self) -> Result<ComputedDecl, ParseError> {
        self.advance(); // 'computed'
        let name = self.expect_ident("expected computed name")?;
        self.expect_token(TokenKind::Colon, "expected ':'")?;
        let type_ann = self.parse_type_annotation()?;
        self.expect_token(TokenKind::Eq, "expected '='")?;
        let body = self.parse_expr()?;
        if self.peek_kind() == TokenKind::Semicolon {
            self.advance();
        }
        Ok(ComputedDecl {
            name,
            type_ann,
            body,
        })
    }

    fn parse_event_decl(&mut self) -> Result<EventDecl, ParseError> {
        self.advance(); // 'event'
        let name = self.expect_ident("expected event name")?;
        self.expect_token(TokenKind::Colon, "expected ':'")?;
        let type_ann = self.parse_type_annotation()?;
        let is_optional = if self.peek_kind() == TokenKind::QuestionMark {
            self.advance();
            true
        } else {
            false
        };
        if self.peek_kind() == TokenKind::Semicolon {
            self.advance();
        }
        Ok(EventDecl {
            name,
            type_ann,
            is_optional,
        })
    }

    fn parse_ref_decl(&mut self) -> Result<RefDecl, ParseError> {
        self.advance(); // 'ref'
        let name = self.expect_ident("expected ref name")?;
        self.expect_token(TokenKind::Colon, "expected ':'")?;
        let type_ann = self.parse_type_annotation()?;
        if self.peek_kind() == TokenKind::Semicolon {
            self.advance();
        }
        Ok(RefDecl { name, type_ann })
    }

    // ── Function / Action ──────────────────────────────

    pub fn parse_function_def(&mut self) -> Result<FunctionDef, ParseError> {
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

        let body = self.parse_block()?;

        Ok(FunctionDef {
            name,
            params,
            return_type,
            is_async: false,
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
        self.advance(); // 'action'
        let name = self.expect_ident("expected action name")?;
        self.expect_token(TokenKind::LParen, "expected '('")?;
        let params = self.parse_params()?;
        self.expect_token(TokenKind::RParen, "expected ')'")?;

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

    fn parse_lifecycle_hook(&mut self) -> Result<LifecycleHook, ParseError> {
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
            _ => unreachable!(),
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
        // Parse: (dep1, dep2)
        if self.peek_kind() == TokenKind::LParen {
            self.advance();
            if self.peek_kind() != TokenKind::RParen {
                deps.push(self.expect_ident("expected dep name")?);
                while self.peek_kind() == TokenKind::Comma {
                    self.advance();
                    if self.peek_kind() == TokenKind::RParen {
                        break;
                    }
                    deps.push(self.expect_ident("expected dep name")?);
                }
            }
            self.expect_token(TokenKind::RParen, "expected ')'")?;
        }
        Ok(deps)
    }

    // ── Store ──────────────────────────────────────────

    pub fn parse_store_def(&mut self) -> Result<StoreDef, ParseError> {
        self.advance(); // 'store'
        let name = self.expect_ident("expected store name")?;
        self.expect_token(TokenKind::LBrace, "expected '{'")?;

        let mut signals = Vec::new();
        let mut getters = Vec::new();
        let mut actions = Vec::new();

        while self.peek_kind() != TokenKind::RBrace && self.peek_kind() != TokenKind::Eof {
            match self.peek_kind() {
                TokenKind::Signal => signals.push(self.parse_signal_decl()?),
                TokenKind::Getter => getters.push(self.parse_getter_decl()?),
                TokenKind::Action => actions.push(self.parse_action_def()?),
                _ => {
                    let t = self.advance();
                    return Err(ParseError {
                        message: format!("Unexpected token in store: {:?}", t.kind),
                        span: t.span,
                    });
                }
            }
        }

        self.expect_token(TokenKind::RBrace, "expected '}'")?;

        Ok(StoreDef {
            name,
            signals,
            getters,
            actions,
        })
    }

    fn parse_getter_decl(&mut self) -> Result<GetterDecl, ParseError> {
        self.advance(); // 'getter'
        let name = self.expect_ident("expected getter name")?;
        self.expect_token(TokenKind::Colon, "expected ':'")?;
        let type_ann = self.parse_type_annotation()?;
        self.expect_token(TokenKind::Eq, "expected '='")?;
        let body = self.parse_expr()?;
        if self.peek_kind() == TokenKind::Semicolon {
            self.advance();
        }
        Ok(GetterDecl {
            name,
            type_ann,
            body,
        })
    }

    // ── Theme ──────────────────────────────────────────

    pub fn parse_theme_def(&mut self) -> Result<ThemeDef, ParseError> {
        self.advance(); // 'theme'
        self.expect_ident("expected theme name")?;
        self.expect_token(TokenKind::LBrace, "expected '{'")?;

        let mut categories = Vec::new();
        while self.peek_kind() != TokenKind::RBrace && self.peek_kind() != TokenKind::Eof {
            let cat_name = self.expect_ident("expected category name")?;
            self.expect_token(TokenKind::LBrace, "expected '{'")?;
            let mut values = Vec::new();
            while self.peek_kind() != TokenKind::RBrace && self.peek_kind() != TokenKind::Eof {
                let key = self.expect_ident("expected key")?;
                self.expect_token(TokenKind::Colon, "expected ':'")?;
                let val_t = self.advance();
                let val = match &val_t.kind {
                    TokenKind::StringLiteral(s) => s.clone(),
                    TokenKind::Ident(s) => s.clone(),
                    _ => {
                        return Err(ParseError {
                            message: "Expected string or identifier value".into(),
                            span: val_t.span,
                        })
                    }
                };
                values.push((key, val));
                if self.peek_kind() == TokenKind::Comma {
                    self.advance();
                }
            }
            self.expect_token(TokenKind::RBrace, "expected '}'")?;
            categories.push(ThemeCategory {
                name: cat_name,
                values,
            });
        }
        self.expect_token(TokenKind::RBrace, "expected '}'")?;
        Ok(ThemeDef { categories })
    }

    // ── Struct ─────────────────────────────────────────

    pub fn parse_struct_def(&mut self) -> Result<StructDef, ParseError> {
        let start = self.peek_span();
        self.advance(); // 'struct'
        let is_nominal = if self.peek_kind() == TokenKind::Nominal {
            self.advance();
            true
        } else {
            false
        };
        let name = self.expect_ident("expected struct name")?;
        self.expect_token(TokenKind::LBrace, "expected '{'")?;

        let mut fields = Vec::new();
        while self.peek_kind() != TokenKind::RBrace && self.peek_kind() != TokenKind::Eof {
            let field_name = self.expect_ident("expected field name")?;
            self.expect_token(TokenKind::Colon, "expected ':'")?;
            let type_ann = self.parse_type_annotation()?;
            fields.push(StructField {
                name: field_name,
                type_ann,
            });
            if self.peek_kind() == TokenKind::Comma {
                self.advance();
            }
        }
        self.expect_token(TokenKind::RBrace, "expected '}'")?;

        Ok(StructDef {
            name,
            is_nominal,
            fields,
            span: Span {
                start: start.start,
                end: start.end,
                line: start.line,
                col: start.col,
            },
        })
    }

    // ── Tagged Union ───────────────────────────────────

    pub fn parse_tagged_union_def(&mut self) -> Result<TaggedUnionDef, ParseError> {
        let start = self.peek_span();
        self.advance(); // 'type'
        let name = self.expect_ident("expected type name")?;
        self.expect_token(TokenKind::Eq, "expected '='")?;

        let mut variants = Vec::new();
        // Variant list: Variant1 { fields } | Variant2 { fields }
        loop {
            let variant_name = self.expect_ident("expected variant name")?;
            let fields = if self.peek_kind() == TokenKind::LBrace {
                self.advance();
                let mut f = Vec::new();
                while self.peek_kind() != TokenKind::RBrace && self.peek_kind() != TokenKind::Eof {
                    let fname = self.expect_ident("expected field name")?;
                    self.expect_token(TokenKind::Colon, "expected ':'")?;
                    let ft = self.parse_type_annotation()?;
                    f.push(StructField {
                        name: fname,
                        type_ann: ft,
                    });
                    if self.peek_kind() == TokenKind::Comma {
                        self.advance();
                    }
                }
                self.expect_token(TokenKind::RBrace, "expected '}'")?;
                f
            } else {
                vec![]
            };
            variants.push(Variant {
                name: variant_name,
                fields,
            });
            if self.peek_kind() != TokenKind::Pipe {
                break;
            }
            self.advance(); // '|'
        }

        Ok(TaggedUnionDef {
            name,
            variants,
            span: Span {
                start: start.start,
                end: start.end,
                line: start.line,
                col: start.col,
            },
        })
    }

    // ── Import ─────────────────────────────────────────

    pub fn parse_import(&mut self) -> Result<ImportStmt, ParseError> {
        self.advance(); // 'import'
        let mut names = Vec::new();
        names.push(self.expect_ident("expected import name")?);

        while self.peek_kind() == TokenKind::Comma {
            self.advance();
            names.push(self.expect_ident("expected import name")?);
        }

        self.expect_token(TokenKind::From, "expected 'from'")?;
        let t = self.advance();
        let path = match &t.kind {
            TokenKind::StringLiteral(s) => s.clone(),
            _ => {
                return Err(ParseError {
                    message: "Expected string path after 'from'".into(),
                    span: t.span,
                })
            }
        };
        if self.peek_kind() == TokenKind::Semicolon {
            self.advance();
        }
        Ok(ImportStmt { names, path })
    }
}
