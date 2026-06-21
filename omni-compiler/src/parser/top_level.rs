//! Top-level declaration parsers: component, store, function, struct, theme, import.

use crate::error::ParseError;
use crate::lexer::{Span, TokenKind};
use crate::parser::ast::*;
use crate::parser::Parser;

impl<'a> Parser<'a> {
    // ── Top-Level Module ───────────────────────────────

    /// Parse the top-level of a single source module.
    /// Returns a `Module` with all top-level declarations populated.
    pub fn parse_module(&mut self) -> Result<Module, ParseError> {
        let mut module = Module {
            name: "input".to_string(),
            ..Default::default()
        };

        while self.peek_kind() != TokenKind::Eof {
            match self.peek_kind() {
                TokenKind::Component => {
                    module.components.push(self.parse_component_def()?);
                }
                TokenKind::Store => {
                    module.stores.push(self.parse_store_def()?);
                }
                TokenKind::Theme => {
                    module.themes.push(self.parse_theme_def()?);
                }
                TokenKind::Import => {
                    module.imports.push(self.parse_import()?);
                }
                TokenKind::Export => {
                    module.exports.push(self.parse_export()?);
                }
                TokenKind::AtMotion => {
                    module.motions.push(self.parse_motion_def()?);
                }
                TokenKind::AtTimeline => {
                    module.timelines.push(self.parse_timeline_def()?);
                }
                TokenKind::Async => {
                    self.advance(); // 'async'
                    self.expect_token(TokenKind::Fn, "expected 'fn' after 'async'")?;
                    module
                        .functions
                        .push(self.parse_function_def_after_keyword(Visibility::Private, true)?);
                }
                TokenKind::Fn => {
                    module
                        .functions
                        .push(self.parse_function_def(Visibility::Private)?);
                }
                TokenKind::Struct => {
                    module
                        .structs
                        .push(self.parse_struct_def(Visibility::Private)?);
                }
                TokenKind::Pub => {
                    self.advance(); // 'pub'
                    match self.peek_kind() {
                        TokenKind::Async => {
                            self.advance(); // 'async'
                            self.expect_token(TokenKind::Fn, "expected 'fn' after 'async'")?;
                            module.functions.push(
                                self.parse_function_def_after_keyword(Visibility::Public, true)?,
                            );
                        }
                        TokenKind::Fn => {
                            module
                                .functions
                                .push(self.parse_function_def(Visibility::Public)?);
                        }
                        TokenKind::Struct => {
                            module
                                .structs
                                .push(self.parse_struct_def(Visibility::Public)?);
                        }
                        TokenKind::Type => {
                            module
                                .type_aliases
                                .push(self.parse_type_alias(Visibility::Public)?);
                        }
                        _ => {
                            let t = self.advance();
                            return Err(ParseError {
                                message: format!(
                                    "Expected fn, struct, or type after 'pub', got {:?}",
                                    t.kind
                                ),
                                span: t.span,
                            });
                        }
                    }
                }
                TokenKind::Type => {
                    // Peek ahead to determine if tagged union or type alias.
                    // Tagged unions have the form: type Name = Variant1 | Variant2 { ... }
                    // Type aliases have the form: type Name = SomeTypeAnnotation;
                    let saved = self.pos;
                    self.advance(); // 'type'
                    let name_ok = self.expect_ident("expected type name").is_ok();
                    let eq_ok = self.expect_token(TokenKind::Eq, "expected '='").is_ok();
                    self.pos = saved;

                    if !name_ok || !eq_ok {
                        // Can't parse — try tagged union (original behavior)
                        module.tagged_unions.push(self.parse_tagged_union_def()?);
                    } else {
                        // Advance past 'type' Name '='
                        self.advance(); // 'type'
                        self.advance(); // name
                        self.advance(); // '='

                        // Check if next token starts a variant: Ident { ... } or Ident | ...
                        let is_tagged = match self.peek_kind() {
                            TokenKind::Ident(_) => self.tokens.get(self.pos + 1).is_some_and(|t| {
                                t.kind == TokenKind::LBrace || t.kind == TokenKind::Pipe
                            }),
                            _ => false,
                        };

                        self.pos = saved; // reset to 'type'

                        if is_tagged {
                            module.tagged_unions.push(self.parse_tagged_union_def()?);
                        } else {
                            module
                                .type_aliases
                                .push(self.parse_type_alias(Visibility::Private)?);
                        }
                    }
                }
                _ => {
                    let t = self.advance();
                    return Err(ParseError {
                        message: format!("Unexpected token at top level: {:?}", t.kind),
                        span: t.span,
                    });
                }
            }
        }

        Ok(module)
    }

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
                                Expr::Literal(Literal::String(s.clone()), t.span.clone())
                            }
                            TokenKind::IntLiteral(n) => {
                                Expr::Literal(Literal::Int(*n), t.span.clone())
                            }
                            TokenKind::FloatLiteral(f) => {
                                Expr::Literal(Literal::Float(*f), t.span.clone())
                            }
                            TokenKind::True => Expr::Literal(Literal::Bool(true), t.span.clone()),
                            TokenKind::False => Expr::Literal(Literal::Bool(false), t.span.clone()),
                            TokenKind::Null => Expr::Literal(Literal::Null, t.span.clone()),
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
                    motions.push(self.parse_motion_def()?);
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
            let body = self.parse_block()?;
            // Wrap the block body as the computed expression
            // Use trailing expression if present
            let body_expr = if let Some(trailing) = body.trailing_expr {
                *trailing
            } else {
                Expr::Literal(
                    Literal::Null,
                    Span {
                        start: 0,
                        end: 0,
                        line: 0,
                        col: 0,
                    },
                )
            };
            return Ok(ComputedDecl {
                name,
                type_ann,
                body: body_expr,
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
            _ => {
                // Unknown lifecycle directive — report error instead of panicking
                return Err(ParseError {
                    message: format!("unknown lifecycle directive: {:?}", self.peek_kind()),
                    span: self.peek_span().clone(),
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

    // ── Store ──────────────────────────────────────────

    pub fn parse_store_def(&mut self) -> Result<StoreDef, ParseError> {
        let start = self.peek_span();
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
            span: Span {
                start: start.start,
                end: start.end,
                line: start.line,
                col: start.col,
            },
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

    pub fn parse_struct_def(&mut self, visibility: Visibility) -> Result<StructDef, ParseError> {
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
            visibility,
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
        let start = self.peek_span();
        self.advance(); // 'import'

        match self.peek_kind() {
            TokenKind::Type => {
                // import type { A, B } from "./path.omni";
                self.advance(); // 'type'
                let names = self.parse_import_name_list()?;
                self.expect_token(TokenKind::From, "expected 'from' after 'import type'")?;
                let path = self.expect_string("expected module path string")?;
                self.consume_semicolon();
                let end = self.peek_span();
                Ok(ImportStmt::TypeOnly(TypeImport {
                    names,
                    path,
                    span: Span {
                        start: start.start,
                        end: end.end,
                        line: start.line,
                        col: start.col,
                    },
                }))
            }
            TokenKind::LBrace => {
                // import { A, B } from "./path.omni";
                let names = self.parse_import_name_list()?;
                self.expect_token(TokenKind::From, "expected 'from'")?;
                let path = self.expect_string("expected module path string")?;
                self.consume_semicolon();
                let end = self.peek_span();
                Ok(ImportStmt::Named(NamedImport {
                    names,
                    path,
                    span: Span {
                        start: start.start,
                        end: end.end,
                        line: start.line,
                        col: start.col,
                    },
                }))
            }
            TokenKind::StringLiteral(_) => {
                // import "./path.omni" as name;
                let path = self.expect_string("expected module path string")?;
                self.expect_token(TokenKind::As, "expected 'as' after path")?;
                let alias = self.expect_ident("expected alias name")?;
                self.consume_semicolon();
                let end = self.peek_span();
                Ok(ImportStmt::Namespace(NamespaceImport {
                    path,
                    alias,
                    span: Span {
                        start: start.start,
                        end: end.end,
                        line: start.line,
                        col: start.col,
                    },
                }))
            }
            _ => {
                // import omniview; — bare import (lib marker)
                let name = self.expect_ident("expected import name, '{', or string")?;
                self.consume_semicolon();
                Ok(ImportStmt::Lib(name))
            }
        }
    }

    /// Parse `{ Name1, Name2 }` import/export name list.
    fn parse_import_name_list(&mut self) -> Result<Vec<String>, ParseError> {
        self.expect_token(TokenKind::LBrace, "expected '{'")?;
        let mut names = Vec::new();
        if self.peek_kind() != TokenKind::RBrace {
            names.push(self.expect_ident("expected import name")?);
            while self.peek_kind() == TokenKind::Comma {
                self.advance();
                if self.peek_kind() == TokenKind::RBrace {
                    break; // trailing comma
                }
                names.push(self.expect_ident("expected import name")?);
            }
        }
        self.expect_token(TokenKind::RBrace, "expected '}'")?;
        Ok(names)
    }

    // ── Export ─────────────────────────────────────────

    pub fn parse_export(&mut self) -> Result<ExportFrom, ParseError> {
        let start = self.peek_span();
        self.advance(); // 'export'
        let names = self.parse_import_name_list()?;
        self.expect_token(TokenKind::From, "expected 'from'")?;
        let path = self.expect_string("expected module path string")?;
        self.consume_semicolon();
        let end = self.peek_span();
        Ok(ExportFrom {
            names,
            path,
            span: Span {
                start: start.start,
                end: end.end,
                line: start.line,
                col: start.col,
            },
        })
    }

    // ── Type Alias ─────────────────────────────────────

    pub fn parse_type_alias(&mut self, visibility: Visibility) -> Result<TypeAlias, ParseError> {
        let start = self.peek_span();
        self.advance(); // 'type'
        let name = self.expect_ident("expected type name")?;
        self.expect_token(TokenKind::Eq, "expected '='")?;
        let type_ann = self.parse_type_annotation()?;
        self.consume_semicolon();
        let end = self.peek_span();
        Ok(TypeAlias {
            visibility,
            name,
            type_ann,
            span: Span {
                start: start.start,
                end: end.end,
                line: start.line,
                col: start.col,
            },
        })
    }

    // ── Motion / Timeline block parsers ─────────────────

    /// Parse @motion NAME { body }
    pub fn parse_motion_def(&mut self) -> Result<MotionDef, ParseError> {
        let span_start = self.peek_span();
        self.expect_token(TokenKind::AtMotion, "expected '@motion'")?;
        let name = self.expect_ident("expected motion name")?;
        self.expect_token(TokenKind::LBrace, "expected '{'")?;
        let body = self.parse_motion_body()?;
        self.expect_token(TokenKind::RBrace, "expected '}'")?;
        Ok(MotionDef {
            name,
            body,
            span: span_start,
        })
    }

    fn parse_motion_body(&mut self) -> Result<MotionBody, ParseError> {
        // First token determines mode.
        let first = self
            .peek_ident_value()
            .ok_or_else(|| self.error("expected motion body"))?;
        match first.as_str() {
            "from" | "to" => self.parse_keyframe_body(),
            "set" | "stagger" | "label" => self.parse_timeline_body(),
            "onStart" | "onUpdate" | "onComplete" | "onRepeat" | "onInterrupt" => {
                self.parse_callback_body()
            }
            _ => self.parse_timeline_body(), // IDENT => motion reference
        }
    }

    /// Parse keyframe body: from { ... } to { ... } with optional over/ease/playback
    fn parse_keyframe_body(&mut self) -> Result<MotionBody, ParseError> {
        let mut stops = Vec::new();

        // Parse first stop
        stops.push(self.parse_keyframe_stop()?);

        // Parse remaining stops while they start with "to" or percentage
        while let Some(ident) = self.peek_ident_value() {
            if ident == "to" || ident == "from" {
                stops.push(self.parse_keyframe_stop()?);
            } else if ident.chars().all(|c| c.is_ascii_digit() || c == '.') {
                // Percentage stop: "50" means 50%
                stops.push(self.parse_keyframe_stop()?);
            } else {
                break;
            }
        }

        // Parse shared over/ease/color_space
        let mut over = Expr::Literal(
            Literal::Float(0.3),
            Span {
                start: 0,
                end: 0,
                line: 0,
                col: 0,
            },
        );
        let mut ease = None;
        let mut color_space = None;

        self.parse_shared_timing(&mut over, &mut ease, &mut color_space)?;

        // Parse optional path/pathRotate
        let mut path: Option<String> = None;
        let mut path_rotate: Option<bool> = None;
        loop {
            match self.peek_ident_value() {
                Some(ref s) if s == "path" => {
                    self.advance();
                    self.expect_token(TokenKind::Colon, "expected ':'")?;
                    path = Some(self.expect_string("expected path string")?);
                }
                Some(ref s) if s == "pathRotate" => {
                    self.advance();
                    self.expect_token(TokenKind::Colon, "expected ':'")?;
                    path_rotate = Some(match self.peek_kind() {
                        TokenKind::True => { self.advance(); true }
                        TokenKind::False => { self.advance(); false }
                        _ => return Err(self.error("expected bool for pathRotate")),
                    });
                }
                _ => break,
            }
            if self.peek_kind() == TokenKind::Comma {
                self.advance();
            }
        }

        let playback = self.parse_playback_props()?;

        Ok(MotionBody::Keyframe(KeyframeBody {
            stops,
            over,
            ease,
            color_space,
            playback,
            path,
            path_rotate,
        }))
    }

    /// Parse a single keyframe stop: "from { ... }", "to { ... }", or "50% { ... }"
    fn parse_keyframe_stop(&mut self) -> Result<KeyframeStop, ParseError> {
        let position = match self.peek_ident_value() {
            Some(ref s) if s == "from" => {
                self.advance();
                StopPosition::From
            }
            Some(ref s) if s == "to" => {
                self.advance();
                StopPosition::To
            }
            Some(ref s) if s.chars().all(|c| c.is_ascii_digit() || c == '.') => {
                let val: f64 = s.parse().unwrap_or(50.0);
                self.advance();
                StopPosition::Percent(val)
            }
            _ => return Err(self.error("expected 'from', 'to', or percentage")),
        };

        self.expect_token(TokenKind::LBrace, "expected '{'")?;
        let mut properties = Vec::new();
        while self.peek_kind() != TokenKind::RBrace && self.peek_kind() != TokenKind::Eof {
            let key = self.expect_ident("expected property name")?;
            self.expect_token(TokenKind::Colon, "expected ':'")?;
            let value = self.parse_expr()?;
            properties.push((key, value));
            if self.peek_kind() == TokenKind::Comma {
                self.advance();
            }
        }
        self.expect_token(TokenKind::RBrace, "expected '}'")?;

        // Optional per-stop ease
        let ease = if self.peek_ident_value().map_or(false, |s| s == "ease") {
            self.advance();
            Some(self.parse_easing()?)
        } else {
            None
        };

        Ok(KeyframeStop {
            position,
            properties,
            ease,
        })
    }

    /// Parse shared over/ease/color_space after keyframe stops
    fn parse_shared_timing(
        &mut self,
        over: &mut Expr,
        ease: &mut Option<EasingExpr>,
        color_space: &mut Option<String>,
    ) -> Result<(), ParseError> {
        loop {
            match self.peek_ident_value() {
                Some(ref s) if s == "over" => {
                    self.advance();
                    *over = self.parse_expr()?;
                }
                Some(ref s) if s == "ease" => {
                    self.advance();
                    *ease = Some(self.parse_easing()?);
                }
                Some(ref s) if s == "color_space" => {
                    self.advance();
                    *color_space = Some(self.expect_ident("expected color space")?);
                }
                _ => break,
            }
            // Optional comma between timing props
            if self.peek_kind() == TokenKind::Comma {
                self.advance();
            }
        }
        Ok(())
    }

    /// Parse timeline body: sequence of items with connectors
    pub(crate) fn parse_timeline_body(&mut self) -> Result<MotionBody, ParseError> {
        let mut items = Vec::new();
        let mut connector = Connector::None;
        while self.peek_kind() != TokenKind::RBrace && self.peek_kind() != TokenKind::Eof {
            let kind = self.parse_timeline_item()?;

            items.push(TimelineItem {
                kind,
                connector: connector.clone(),
            });

            // Parse connector between items
            if self.peek_kind() != TokenKind::RBrace && self.peek_kind() != TokenKind::Eof {
                connector = self.parse_connector()?;
            }
        }

        let playback = self.parse_playback_props()?;

        Ok(MotionBody::Timeline(TimelineBody { items, playback }))
    }

    /// Parse a single timeline item
    fn parse_timeline_item(&mut self) -> Result<TimelineItemKind, ParseError> {
        // Check for stagger(...), label(...), set {...}, or IDENT motion_ref
        match self.peek_ident_value() {
            Some(ref s) if s == "stagger" => {
                self.advance();
                Ok(TimelineItemKind::Stagger(self.parse_stagger_call()?))
            }
            Some(ref s) if s == "label" => {
                self.advance();
                self.expect_token(TokenKind::LParen, "expected '('")?;
                let label_name = self.expect_ident("expected label name")?;
                let position = if self.peek_kind() == TokenKind::Comma {
                    self.advance();
                    Some(self.parse_expr()?)
                } else {
                    None
                };
                self.expect_token(TokenKind::RParen, "expected ')'")?;
                Ok(TimelineItemKind::Label(label_name, position))
            }
            Some(ref s) if s == "set" => {
                self.advance();
                self.expect_token(TokenKind::LBrace, "expected '{'")?;
                let mut properties = Vec::new();
                while self.peek_kind() != TokenKind::RBrace && self.peek_kind() != TokenKind::Eof {
                    let key = self.expect_ident("expected property name")?;
                    self.expect_token(TokenKind::Colon, "expected ':'")?;
                    let value = self.parse_expr()?;
                    properties.push((key, value));
                    if self.peek_kind() == TokenKind::Comma {
                        self.advance();
                    }
                }
                self.expect_token(TokenKind::RBrace, "expected '}'")?;
                Ok(TimelineItemKind::SetBlock(properties))
            }
            Some(ref s) if s == "from" => {
                // Direction item starting with "from"
                let dir = self.parse_direction_item()?;
                Ok(TimelineItemKind::Direction(dir))
            }
            _ => {
                // Motion reference or direction item
                let elem_or_motion = self.expect_ident("expected motion reference or 'from'")?;
                // Check if next token is a motion name (ident) or direction keyword
                match self.peek_ident_value() {
                    Some(ref s) if s == "from" || s == "to" => {
                        // This was an element id, now parse direction
                        let _elem_id = elem_or_motion;
                        let dir = self.parse_direction_item()?;
                        Ok(TimelineItemKind::Direction(dir))
                    }
                    Some(ref s) if s == "over" || s == "ease" || s == "delay" => {
                        // Motion reference with inline over
                        Ok(TimelineItemKind::MotionRef(elem_or_motion))
                    }
                    Some(ref s) if s == "then" || s == "with" => {
                        // Motion reference, connector follows
                        Ok(TimelineItemKind::MotionRef(elem_or_motion))
                    }
                    _ => {
                        // Plain motion reference
                        Ok(TimelineItemKind::MotionRef(elem_or_motion))
                    }
                }
            }
        }
    }

    /// Parse a direction item (from/to with over/ease/delay)
    fn parse_direction_item(&mut self) -> Result<DirectionItem, ParseError> {
        let direction = match self.peek_ident_value() {
            Some(ref s) if s == "from" => {
                self.advance();
                Direction::FromTo
            }
            Some(ref s) if s == "to" => {
                self.advance();
                Direction::To
            }
            _ => return Err(self.error("expected 'from' or 'to'")),
        };

        self.expect_token(TokenKind::LBrace, "expected '{'")?;
        let mut properties = Vec::new();
        while self.peek_kind() != TokenKind::RBrace && self.peek_kind() != TokenKind::Eof {
            let key = self.expect_ident("expected property name")?;
            self.expect_token(TokenKind::Colon, "expected ':'")?;
            let value = self.parse_expr()?;
            properties.push((key, value));
            if self.peek_kind() == TokenKind::Comma {
                self.advance();
            }
        }
        self.expect_token(TokenKind::RBrace, "expected '}'")?;

        // Parse over/ease/delay/color_space/start
        let mut over = Expr::Literal(
            Literal::Float(0.3),
            Span {
                start: 0,
                end: 0,
                line: 0,
                col: 0,
            },
        );
        let mut ease = None;
        let mut delay = None;
        let mut color_space = None;
        let mut start = None;
        let mut path: Option<String> = None;
        let mut path_rotate: Option<bool> = None;

        self.parse_timeline_item_timing(
            &mut over,
            &mut ease,
            &mut delay,
            &mut color_space,
            &mut start,
            &mut path,
            &mut path_rotate,
        )?;

        Ok(DirectionItem {
            direction,
            properties,
            over,
            ease,
            delay,
            color_space,
            start,
            path,
            path_rotate,
        })
    }

    /// Parse timeline item timing: over:, ease:, delay:, start:, path:, pathRotate:
    fn parse_timeline_item_timing(
        &mut self,
        over: &mut Expr,
        ease: &mut Option<EasingExpr>,
        delay: &mut Option<Expr>,
        color_space: &mut Option<String>,
        start: &mut Option<PositionValue>,
        path: &mut Option<String>,
        path_rotate: &mut Option<bool>,
    ) -> Result<(), ParseError> {
        loop {
            match self.peek_ident_value() {
                Some(ref s) if s == "over" => {
                    self.advance();
                    *over = self.parse_expr()?;
                }
                Some(ref s) if s == "ease" => {
                    self.advance();
                    *ease = Some(self.parse_easing()?);
                }
                Some(ref s) if s == "delay" => {
                    self.advance();
                    *delay = Some(self.parse_expr()?);
                }
                Some(ref s) if s == "color_space" => {
                    self.advance();
                    *color_space = Some(self.expect_ident("expected color space")?);
                }
                Some(ref s) if s == "start" => {
                    self.advance();
                    *start = Some(self.parse_position_value()?);
                }
                Some(ref s) if s == "path" => {
                    self.advance();
                    *path = Some(self.expect_string("expected path string")?);
                }
                Some(ref s) if s == "pathRotate" => {
                    self.advance();
                    *path_rotate = Some(match self.peek_kind() {
                        TokenKind::True => { self.advance(); true }
                        TokenKind::False => { self.advance(); false }
                        _ => return Err(self.error("expected bool for pathRotate")),
                    });
                }
                _ => break,
            }
            if self.peek_kind() == TokenKind::Comma {
                self.advance();
            }
        }
        Ok(())
    }

    /// Parse position value for start:
    fn parse_position_value(&mut self) -> Result<PositionValue, ParseError> {
        match self.peek_ident_value() {
            Some(ref s) if s == "after" => {
                self.advance();
                if self.peek_kind() == TokenKind::LParen {
                    self.advance();
                    let id = self.expect_ident("expected element id")?;
                    self.expect_token(TokenKind::RParen, "expected ')'")?;
                    Ok(PositionValue::AfterPrev(id))
                } else {
                    Ok(PositionValue::After)
                }
            }
            Some(ref s) if s == "with" => {
                self.advance();
                self.expect_token(TokenKind::LParen, "expected '('")?;
                let _id = self.expect_ident("expected element id")?;
                self.expect_token(TokenKind::RParen, "expected ')'")?;
                Ok(PositionValue::WithPrev)
            }
            _ => {
                // Raw expression (percentage, duration)
                let _expr = self.parse_expr()?;
                // Try to interpret as Overlap or Absolute
                // For now, store as After with offset
                Ok(PositionValue::Absolute(
                    // Very basic: try to parse as a string or number
                    "0ms".into(),
                ))
            }
        }
    }

    /// Parse @timeline NAME { body }
    pub fn parse_timeline_def(&mut self) -> Result<TimelineDef, ParseError> {
        let span_start = self.peek_span();
        self.expect_token(TokenKind::AtTimeline, "expected '@timeline'")?;
        let name = self.expect_ident("expected timeline name")?;
        self.expect_token(TokenKind::LBrace, "expected '{'")?;

        let mut items = Vec::new();
        let mut connector = Connector::None;
        while self.peek_kind() != TokenKind::RBrace && self.peek_kind() != TokenKind::Eof {
            let item = self.parse_timeline_item()?;

            items.push(TimelineItem {
                kind: item,
                connector: connector.clone(),
            });

            connector = self.parse_connector()?;
        }
        self.expect_token(TokenKind::RBrace, "expected '}'")?;

        Ok(TimelineDef {
            name,
            items,
            span: span_start,
        })
    }

    /// Parse connector: "then", "with", or defaults to Then
    fn parse_connector(&mut self) -> Result<Connector, ParseError> {
        match self.peek_ident_value() {
            Some(ref s) if s == "then" => {
                self.advance();
                Ok(Connector::Then)
            }
            Some(ref s) if s == "with" => {
                self.advance();
                Ok(Connector::With)
            }
            _ => Ok(Connector::Then), // default to sequential
        }
    }

    /// Parse callback-only motion body
    fn parse_callback_body(&mut self) -> Result<MotionBody, ParseError> {
        let mut callbacks = Vec::new();

        loop {
            match self.peek_ident_value() {
                Some(ref s) if s == "onStart" => {
                    self.advance();
                    let body = self.parse_block()?;
                    let expr = body.trailing_expr.unwrap_or_else(|| {
                        Box::new(Expr::Literal(
                            Literal::Null,
                            Span {
                                start: 0,
                                end: 0,
                                line: 0,
                                col: 0,
                            },
                        ))
                    });
                    callbacks.push(CallbackSpec::OnStart(*expr));
                }
                Some(ref s) if s == "onUpdate" => {
                    self.advance();
                    let body = self.parse_block()?;
                    let expr = body.trailing_expr.unwrap_or_else(|| {
                        Box::new(Expr::Literal(
                            Literal::Null,
                            Span {
                                start: 0,
                                end: 0,
                                line: 0,
                                col: 0,
                            },
                        ))
                    });
                    callbacks.push(CallbackSpec::OnUpdate(*expr));
                }
                Some(ref s) if s == "onComplete" => {
                    self.advance();
                    let body = self.parse_block()?;
                    let expr = body.trailing_expr.unwrap_or_else(|| {
                        Box::new(Expr::Literal(
                            Literal::Null,
                            Span {
                                start: 0,
                                end: 0,
                                line: 0,
                                col: 0,
                            },
                        ))
                    });
                    callbacks.push(CallbackSpec::OnComplete(*expr));
                }
                Some(ref s) if s == "onRepeat" => {
                    self.advance();
                    let body = self.parse_block()?;
                    let expr = body.trailing_expr.unwrap_or_else(|| {
                        Box::new(Expr::Literal(
                            Literal::Null,
                            Span {
                                start: 0,
                                end: 0,
                                line: 0,
                                col: 0,
                            },
                        ))
                    });
                    callbacks.push(CallbackSpec::OnRepeat(*expr));
                }
                Some(ref s) if s == "onInterrupt" => {
                    self.advance();
                    let body = self.parse_block()?;
                    let expr = body.trailing_expr.unwrap_or_else(|| {
                        Box::new(Expr::Literal(
                            Literal::Null,
                            Span {
                                start: 0,
                                end: 0,
                                line: 0,
                                col: 0,
                            },
                        ))
                    });
                    callbacks.push(CallbackSpec::OnInterrupt(*expr));
                }
                _ => break,
            }
        }

        let mut over = Expr::Literal(
            Literal::Float(0.3),
            Span {
                start: 0,
                end: 0,
                line: 0,
                col: 0,
            },
        );
        if self.peek_ident_value().map_or(false, |s| s == "over") {
            self.advance();
            over = self.parse_expr()?;
        }

        let playback = self.parse_playback_props()?;

        Ok(MotionBody::CallbackOnly(CallbackBody {
            callbacks,
            over,
            playback,
        }))
    }

    /// Parse playback properties (loop, direction, fill, delay, paused, etc.)
    fn parse_playback_props(&mut self) -> Result<Vec<PlaybackProp>, ParseError> {
        let mut props = Vec::new();
        loop {
            match self.peek_ident_value() {
                Some(ref s) if s == "loop" => {
                    self.advance();
                    let value = if matches!(
                        self.peek_kind(),
                        TokenKind::IntLiteral(_) | TokenKind::FloatLiteral(_)
                    ) {
                        let t = self.advance();
                        match &t.kind {
                            TokenKind::IntLiteral(n) => LoopValue::Count(*n as u32),
                            _ => LoopValue::Infinite,
                        }
                    } else {
                        LoopValue::Infinite
                    };
                    props.push(PlaybackProp::Loop(value));
                }
                Some(ref s) if s == "direction" => {
                    self.advance();
                    let dir = match self.expect_ident("expected direction")?.as_str() {
                        "normal" => DirectionValue::Normal,
                        "reverse" => DirectionValue::Reverse,
                        "alternate" => DirectionValue::Alternate,
                        "alternateReverse" => DirectionValue::AlternateReverse,
                        other => {
                            return Err(self.error(format!(
                                "expected normal|reverse|alternate|alternateReverse, got '{}'",
                                other
                            )))
                        }
                    };
                    props.push(PlaybackProp::Direction(dir));
                }
                Some(ref s) if s == "fill" => {
                    self.advance();
                    let fill = match self.expect_ident("expected fill value")?.as_str() {
                        "none" => FillValue::None,
                        "forwards" => FillValue::Forwards,
                        "backwards" => FillValue::Backwards,
                        "both" => FillValue::Both,
                        other => {
                            return Err(self.error(format!(
                                "expected none|forwards|backwards|both, got '{}'",
                                other
                            )))
                        }
                    };
                    props.push(PlaybackProp::Fill(fill));
                }
                Some(ref s) if s == "delay" => {
                    self.advance();
                    props.push(PlaybackProp::Delay(self.parse_expr()?));
                }
                Some(ref s) if s == "paused" => {
                    self.advance();
                    props.push(PlaybackProp::Paused(true));
                }
                Some(ref s) if s == "repeatDelay" => {
                    self.advance();
                    props.push(PlaybackProp::RepeatDelay(self.parse_expr()?));
                }
                Some(ref s) if s == "repeatRefresh" => {
                    self.advance();
                    props.push(PlaybackProp::RepeatRefresh(true));
                }
                Some(ref s) if s == "timeScale" => {
                    self.advance();
                    props.push(PlaybackProp::TimeScale(self.parse_expr()?));
                }
                Some(ref s) if s == "interrupt" => {
                    self.advance();
                    let val = match self.expect_ident("expected interrupt value")?.as_str() {
                        "stop" => InterruptValue::Stop,
                        "complete" => InterruptValue::Complete,
                        "queue" => InterruptValue::Queue,
                        "replace" => InterruptValue::Replace,
                        other => {
                            return Err(self.error(format!(
                                "expected stop|complete|queue|replace, got '{}'",
                                other
                            )))
                        }
                    };
                    props.push(PlaybackProp::Interrupt(val));
                }
                Some(ref s) if s == "respectReducedMotion" => {
                    self.advance();
                    props.push(PlaybackProp::RespectReducedMotion(true));
                }
                Some(ref s) if s == "delayChildren" => {
                    self.advance();
                    props.push(PlaybackProp::DelayChildren(self.parse_expr()?));
                }
                Some(ref s) if s == "staggerChildren" => {
                    self.advance();
                    props.push(PlaybackProp::StaggerChildren(self.parse_expr()?));
                }
                Some(ref s) if s == "staggerFrom" => {
                    self.advance();
                    let from = match self.expect_ident("expected stagger direction")?.as_str() {
                        "start" => StaggerDirection::Start,
                        "center" => StaggerDirection::Center,
                        "end" => StaggerDirection::End,
                        "edges" => StaggerDirection::Edges,
                        other => {
                            return Err(self.error(format!(
                                "expected start|center|end|edges, got '{}'",
                                other
                            )))
                        }
                    };
                    props.push(PlaybackProp::StaggerFrom(from));
                }
                _ => break,
            }
        }
        Ok(props)
    }
}
