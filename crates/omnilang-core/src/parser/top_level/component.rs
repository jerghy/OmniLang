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
        let mut inline_stores = Vec::new();
        let mut motions: Vec<MotionDef> = Vec::new();
        let mut lifecycles = Vec::new();
        let mut fns = Vec::new();
        let mut actions = Vec::new();
        let mut getters = Vec::new();
        let mut slots: Vec<SlotDecl> = Vec::new();
        let mut view: Option<ViewBlock> = None;
        let mut styles = Vec::new();

        while *self.peek_kind() != TokenKind::RBrace && *self.peek_kind() != TokenKind::Eof {
            match self.peek_kind() {
                TokenKind::Prop => props.push(self.parse_prop_decl()?),
                TokenKind::Signal => {
                    let sig = self.parse_signal_decl()?;
                    self.reject_theme_marker(&sig)?;
                    signals.push(sig);
                }
                TokenKind::Computed => computeds.push(self.parse_computed_decl()?),
                TokenKind::Event => events.push(self.parse_event_decl()?),
                TokenKind::Ref => refs.push(self.parse_ref_decl()?),
                TokenKind::AtStore => {
                    // @store StoreName; — store reference with @ prefix
                    self.advance();
                    let store_name = self.expect_ident("expected store name")?;
                    store_refs.push(store_name);
                    if *self.peek_kind() == TokenKind::Semicolon {
                        self.advance();
                    }
                }
                TokenKind::Store => {
                    // "store SomeStore" or "store SomeStore { ... }" (inline definition)
                    self.advance();
                    let store_name = self.expect_ident("expected store name")?;
                    if *self.peek_kind() == TokenKind::LBrace {
                        // Inline store definition — parse and extract signals/actions
                        inline_stores.push(store_name.clone());
                        self.advance(); // '{'
                        while *self.peek_kind() != TokenKind::RBrace
                            && *self.peek_kind() != TokenKind::Eof
                        {
                            match self.peek_kind() {
                                TokenKind::Signal => {
                                    let sig = self.parse_signal_decl()?;
                                    self.reject_theme_marker(&sig)?;
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
                                // Task 3.5 (域 5 BNF v2): 内联 store 与顶层
                                // store 成员集统一——`fn` 成员是错误（用
                                // getter 或 action），不再静默转为组件 fn。
                                TokenKind::Fn => {
                                    return Err(ParseError {
                                        kind: None,
                                        message: "store 内 fn 成员不允许；用 getter 或 action"
                                            .into(),
                                        span: self.peek_span(),
                                    });
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
                    if *self.peek_kind() == TokenKind::Semicolon {
                        self.advance();
                    }
                }
                TokenKind::Const => {
                    self.advance(); // 'const'
                                    // Handle $prefixed identifiers: $logoUrl
                    let name = if *self.peek_kind() == TokenKind::Dollar {
                        self.advance();
                        let id = self.expect_ident("expected identifier after $")?;
                        format!("${}", id)
                    } else {
                        self.expect_ident("expected constant name")?
                    };
                    self.expect_token(TokenKind::Eq, "expected '='")?;
                    let value = if *self.peek_kind() == TokenKind::LBracket {
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
                                    kind: None,
                                    message: format!(
                                        "Expected literal value for const, got {:?}",
                                        t.kind
                                    ),
                                    span: t.span,
                                })
                            }
                        }
                    };
                    if *self.peek_kind() == TokenKind::Semicolon {
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
                        is_theme: false,
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
                            kind: None,
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
                TokenKind::AtSlot => {
                    // Body-level slot declaration (ADR-053 D5):
                    // `@slot name: Type { fallback }` — same syntax as the
                    // view-level directive, shared parsing helper. The
                    // `view`/`style` keywords are excluded as slot names
                    // here (bare `@slot` precedes `view { ... }` blocks).
                    slots.push(self.parse_slot_directive_common(true)?);
                }
                _ => {
                    let t = self.advance();
                    return Err(ParseError {
                        kind: None,
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
            inline_stores,
            lifecycles,
            fns,
            actions,
            getters,
            motions,
            slots,
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
        let default = if *self.peek_kind() == TokenKind::Eq {
            self.advance();
            Some(self.parse_expr()?)
        } else {
            None
        };
        if *self.peek_kind() == TokenKind::Semicolon {
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
        // v2 域 5 (design D1): `signal mode: String @theme = "light";`
        // — the @theme marker sits between the type and the initializer
        // (theme_signal ::= 'signal' ident ':' 'String' '@theme').
        // The lexer produces Ident("@theme") for unknown @-words.
        let is_theme = if matches!(self.peek_kind(), TokenKind::Ident(k) if k == "@theme") {
            self.advance();
            true
        } else {
            false
        };
        self.expect_token(TokenKind::Eq, "expected '='")?;
        let init = self.parse_expr()?;
        if *self.peek_kind() == TokenKind::Semicolon {
            self.advance();
        }
        Ok(SignalDecl {
            name,
            type_ann,
            init,
            is_theme,
        })
    }

    /// Reject the `@theme` marker outside top-level stores.
    ///
    /// v2 域 5 决策：@theme 是「当前主题模式」标记，语义上绑定到顶层 store
    /// 的 signal（编译器需要按 store 查找并订阅）。组件 signal 与内联 store
    /// signal（解析期扁平化为组件级声明，无法保留 @theme 语义）一律硬诊断。
    fn reject_theme_marker(&self, sig: &SignalDecl) -> Result<(), ParseError> {
        if sig.is_theme {
            return Err(ParseError {
                kind: None,
                message: "@theme 标记仅允许顶层 store 的 signal——将主题信号放在顶层 store 中 \
                     （store ThemeStore { signal mode: String @theme = \"light\"; }）"
                    .to_string(),
                span: self.peek_span(),
            });
        }
        Ok(())
    }

    fn parse_computed_decl(&mut self) -> Result<ComputedDecl, ParseError> {
        self.advance(); // 'computed'
        let name = self.expect_ident("expected computed name")?;

        // Handle function-like computed: computed name() -> Type { body }
        if *self.peek_kind() == TokenKind::LParen {
            self.advance(); // '('
            self.expect_token(TokenKind::RParen, "expected ')'")?;
            let type_ann = if *self.peek_kind() == TokenKind::Arrow {
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
        if *self.peek_kind() == TokenKind::Semicolon {
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
        let is_optional = if *self.peek_kind() == TokenKind::QuestionMark {
            self.advance();
            true
        } else {
            false
        };
        if *self.peek_kind() == TokenKind::Semicolon {
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
        if *self.peek_kind() == TokenKind::Semicolon {
            self.advance();
        }
        Ok(RefDecl { name, type_ann })
    }
}
