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
        let mut if_mode_default: Option<String> = None;
        let mut hero_defs: Vec<HeroDef> = Vec::new();
        let mut lifecycles = Vec::new();
        let mut fns = Vec::new();
        let mut actions = Vec::new();
        let mut getters = Vec::new();
        let mut motions = Vec::new();
        let mut timelines = Vec::new();
        let mut view: Option<ViewBlock> = None;
        let mut styles = Vec::new();

        while self.peek_kind() != TokenKind::RBrace && self.peek_kind() != TokenKind::Eof {
            match self.peek_kind() {
                TokenKind::Prop => props.push(self.parse_prop_decl()?),
                TokenKind::Signal => signals.push(self.parse_signal_decl()?),
                TokenKind::Computed => computeds.push(self.parse_computed_decl()?),
                TokenKind::Event => events.push(self.parse_event_decl()?),
                TokenKind::Ref => refs.push(self.parse_ref_decl()?),
                TokenKind::AtStore => {
                    // @store StoreName; — store reference with @ prefix
                    self.advance();
                    let store_name = self.expect_ident("expected store name")?;
                    store_refs.push(store_name);
                    if self.peek_kind() == TokenKind::Semicolon {
                        self.advance();
                    }
                }
                TokenKind::Store => {
                    // "store SomeStore" or "store SomeStore { ... }" (inline definition)
                    self.advance();
                    let store_name = self.expect_ident("expected store name")?;
                    if self.peek_kind() == TokenKind::LBrace {
                        // Inline store definition — parse and extract signals/actions
                        self.advance(); // '{'
                        while self.peek_kind() != TokenKind::RBrace
                            && self.peek_kind() != TokenKind::Eof
                        {
                            match self.peek_kind() {
                                TokenKind::Signal => {
                                    let sig = self.parse_signal_decl()?;
                                    signals.push(sig);
                                }
                                TokenKind::Action => {
                                    let action = self.parse_action_def()?;
                                    actions.push(action);
                                }
                                TokenKind::Getter => {
                                    let getter = self.parse_getter_decl()?;
                                    getters.push(getter);
                                }
                                TokenKind::Fn => {
                                    let f = self.parse_function_def(Visibility::Private)?;
                                    fns.push(f);
                                }
                                _ => {
                                    // Skip unknown items inside inline store
                                    self.advance();
                                }
                            }
                        }
                        self.expect_token(TokenKind::RBrace, "expected '}'")?;
                    }
                    store_refs.push(store_name);
                    if self.peek_kind() == TokenKind::Semicolon {
                        self.advance();
                    }
                }
                TokenKind::IfMode => {
                    // if_mode: "wait" — component-level transition mode default
                    self.advance();
                    self.expect_token(TokenKind::Colon, "expected ':' after if_mode")?;
                    let mode = self.expect_string("expected string value for if_mode")?;
                    if self.peek_kind() == TokenKind::Semicolon {
                        self.advance();
                    }
                    if_mode_default = Some(mode);
                }
                TokenKind::AtHero => {
                    // @hero NAME { over: ..., ease: ... }
                    let span_start = self.peek_span();
                    self.advance(); // '@hero'
                    let name = self.expect_ident("expected hero name")?;
                    self.expect_token(TokenKind::LBrace, "expected '{'")?;
                    // Parse hero body
                    // Expected fields: over: <expr> [, ease: <easing>]
                    let mut over = Expr::Literal(Literal::Float(0.3), self.peek_span());
                    let mut ease = None;
                    loop {
                        match self.peek_ident_value() {
                            Some(ref s) if s == "over" => {
                                self.advance();
                                self.expect_token(TokenKind::Colon, "expected ':'")?;
                                over = self.parse_expr()?;
                            }
                            Some(ref s) if s == "ease" => {
                                self.advance();
                                self.expect_token(TokenKind::Colon, "expected ':'")?;
                                ease = Some(self.parse_easing()?);
                            }
                            _ => break,
                        }
                        if self.peek_kind() == TokenKind::Comma {
                            self.advance();
                        }
                    }
                    self.expect_token(TokenKind::RBrace, "expected '}'")?;
                    let span_end = self.peek_span();
                    hero_defs.push(HeroDef {
                        name,
                        over,
                        ease,
                        span: Span {
                            start: span_start.start,
                            end: span_end.end,
                            line: span_start.line,
                            col: span_start.col,
                        },
                    });
                }
                TokenKind::Const => {
                    self.advance(); // 'const'
                                    // Handle $prefixed identifiers: $logoUrl
                    let name = if self.peek_kind() == TokenKind::Dollar {
                        self.advance();
                        let id = self.expect_ident("expected identifier after $")?;
                        format!("${}", id)
                    } else {
                        self.expect_ident("expected constant name")?
                    };
                    self.expect_token(TokenKind::Eq, "expected '='")?;
                    let value = if self.peek_kind() == TokenKind::LBracket {
                        // Array literal: [expr, expr, ...]
                        self.parse_expr()?
                    } else {
                        let t = self.advance();
                        match &t.kind {
                            TokenKind::StringLiteral(s) => {
                                Expr::Literal(Literal::String(s.clone()), t.span)
                            }
                            TokenKind::IntLiteral(n) => Expr::Literal(Literal::Int(*n), t.span),
                            TokenKind::FloatLiteral(f) => Expr::Literal(Literal::Float(*f), t.span),
                            TokenKind::True => Expr::Literal(Literal::Bool(true), t.span),
                            TokenKind::False => Expr::Literal(Literal::Bool(false), t.span),
                            TokenKind::Null => Expr::Literal(Literal::Null, t.span),
                            _ => {
                                return Err(ParseError {
                                    message: format!(
                                        "Expected literal value for const, got {:?}",
                                        t.kind
                                    ),
                                    span: t.span,
                                })
                            }
                        }
                    };
                    if self.peek_kind() == TokenKind::Semicolon {
                        self.advance();
                    }
                    // Store as a signal-like declaration for now
                    signals.push(SignalDecl {
                        name,
                        type_ann: TypeAnnotation::Named {
                            name: "auto".into(),
                            type_args: vec![],
                            span: Span {
                                start: 0,
                                end: 0,
                                line: 0,
                                col: 0,
                            },
                        },
                        init: value,
                    });
                }
                TokenKind::Async => {
                    self.advance(); // 'async'
                    self.expect_token(TokenKind::Fn, "expected 'fn' after 'async'")?;
                    fns.push(self.parse_function_def_after_keyword(Visibility::Private, true)?);
                }
                TokenKind::Fn => fns.push(self.parse_function_def(Visibility::Private)?),
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
                TokenKind::Style => {
                    let mut rules = self.parse_style_rules()?;
                    styles.append(&mut rules);
                }
                TokenKind::AtMotion => {
                    let motion = self.parse_motion_def()?;
                    motions.push(motion);
                }
                TokenKind::AtTimeline => {
                    timelines.push(self.parse_timeline_def()?);
                }
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
            if_mode_default,
            hero_defs,
            lifecycles,
            fns,
            actions,
            getters,
            motions,
            timelines,
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

    pub(super) fn parse_signal_decl(&mut self) -> Result<SignalDecl, ParseError> {
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

        // Handle function-like computed: computed name() -> Type { body }
        if self.peek_kind() == TokenKind::LParen {
            self.advance(); // '('
            self.expect_token(TokenKind::RParen, "expected ')'")?;
            let type_ann = if self.peek_kind() == TokenKind::Arrow {
                self.advance(); // '->'
                self.parse_type_annotation()?
            } else {
                TypeAnnotation::Named {
                    name: "auto".into(),
                    type_args: vec![],
                    span: Span {
                        start: 0,
                        end: 0,
                        line: 0,
                        col: 0,
                    },
                }
            };
            let block = self.parse_block()?;
            // Preserve the full block body as a Block expression so the
            // compiled output contains all statements (if-return chains, etc.),
            // not just the trailing expression.
            return Ok(ComputedDecl {
                name,
                type_ann,
                body: Expr::Block {
                    body: block,
                    span: Span {
                        start: 0,
                        end: 0,
                        line: 0,
                        col: 0,
                    },
                },
            });
        }

        // Simple computed: computed name: Type = expr
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
}
