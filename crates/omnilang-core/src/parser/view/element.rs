//! Element property parsing helpers for OmniView components.
//!
//! Extracted from `mod.rs` to keep that file below 800 lines.

use crate::error::ParseError;
use crate::lexer::{Span, TokenKind};
use crate::parser::ast::*;
use crate::parser::stmt::keyword_as_ident_name;
use crate::parser::Parser;

impl<'a> Parser<'a> {
    pub(super) fn check_closing_tag(&self, tag: &str) -> bool {
        let tokens = &self.tokens;
        let pos = self.pos;
        if pos + 2 >= tokens.len() {
            return false;
        }
        matches!(tokens[pos].kind, TokenKind::Lt)
            && matches!(tokens[pos + 1].kind, TokenKind::Slash)
            && matches!(&tokens[pos + 2].kind, TokenKind::Ident(t) if t == tag)
    }

    /// Check if the token at `pos` looks like the start of an attribute
    /// (identifier followed by '=' or ':'), rather than another class name.
    pub(super) fn peek_is_attr_start(&self) -> bool {
        if self.pos + 1 >= self.tokens.len() {
            return false;
        }
        matches!(
            self.tokens[self.pos + 1].kind,
            TokenKind::Eq | TokenKind::Colon
        )
    }

    /// Check if the next token is a known boolean HTML attribute keyword.
    /// Used to prevent these from being treated as class names in
    /// `class: btn primary disabled` scenarios.
    pub(super) fn peek_is_boolean_attr_keyword(&self) -> bool {
        let name: Option<String> = match self.peek_kind() {
            TokenKind::Ident(s) => Some(s.clone()),
            kind => keyword_as_ident_name(kind).map(|s| s.to_string()),
        };
        matches!(
            name.as_deref(),
            Some("controls")
                | Some("disabled")
                | Some("readonly")
                | Some("required")
                | Some("checked")
                | Some("selected")
                | Some("autofocus")
                | Some("multiple")
                | Some("loop")
                | Some("muted")
                | Some("async")
                | Some("defer")
                | Some("hidden")
                | Some("autocomplete")
                | Some("autoplay")
        )
    }

    /// Parse a static prop: name="value" or name={expr}
    /// Also handles hyphenated names like margin-left, background-color
    pub(super) fn parse_element_prop(&mut self) -> Result<ElementProp, ParseError> {
        let mut name = self.expect_ident("expected prop name")?;

        // Handle hyphenated attribute names: margin-left, background-color, etc.
        while self.at(&TokenKind::Minus) {
            self.advance(); // consume '-'
            let next = self.expect_ident("expected identifier after '-'")?;
            name = format!("{}-{}", name, next);
        }

        // Check for semantic binding attributes (focus, scroll-x, scroll-y, value).
        // These are compiler-recognized ref bindings, not generic static props —
        // they only bind to a signal expression: name={expr}. The string-literal
        // form (value="text") and bare boolean form stay generic Static props.
        // Validation (element has @ref, ref declared) happens in the semantic pass.
        const SEMANTIC_BINDINGS: &[&str] = &["focus", "scroll-x", "scroll-y", "value"];
        if SEMANTIC_BINDINGS.contains(&name.as_str())
            && self.at(&TokenKind::Eq)
            && matches!(
                self.tokens.get(self.pos + 1).map(|t| &t.kind),
                Some(TokenKind::LBrace)
            )
        {
            self.advance(); // '='
            self.advance(); // '{'
            let signal = self.parse_expr()?;
            self.expect_token(TokenKind::RBrace, "expected '}'")?;
            return Ok(ElementProp::RefBinding {
                binding: name,
                signal,
            });
        }

        let kind = self.peek_kind().clone();
        match kind {
            TokenKind::Eq => {
                self.advance();
                let kind = self.peek_kind().clone();
                match kind {
                    TokenKind::StringLiteral(_) => {
                        let t = self.advance();
                        if let TokenKind::StringLiteral(s) = &t.kind {
                            Ok(ElementProp::Static {
                                name,
                                value: Expr::Literal(Literal::String(s.clone()), t.span),
                            })
                        } else {
                            // Should not reach here — outer match confirmed StringLiteral
                            Err(ParseError {
                                message: "expected string literal after '='".into(),
                                span: t.span,
                            })
                        }
                    }
                    TokenKind::LBrace => {
                        self.advance();
                        let expr = self.parse_expr()?;
                        self.expect_token(TokenKind::RBrace, "expected '}'")?;
                        Ok(ElementProp::Static { name, value: expr })
                    }
                    _ => {
                        let t = self.advance();
                        Err(ParseError {
                            message: format!(
                                "Expected string or {{expr}} for prop value, got {:?}",
                                t.kind
                            ),
                            span: t.span,
                        })
                    }
                }
            }
            TokenKind::Colon => {
                // ref: name — element reference (not a class name)
                if name == "ref" {
                    self.advance(); // consume ':'
                    let ref_name = self.expect_ident("expected ref name")?;
                    return Ok(ElementProp::Ref(ref_name));
                }
                // states: { state1: { ... }, state2: { ... }, ... }
                if name == "states" {
                    self.advance(); // consume ':'
                    self.expect_token(TokenKind::LBrace, "expected '{' after 'states:'")?;
                    while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
                        self.advance();
                    }
                    self.expect_token(TokenKind::RBrace, "expected '}'")?;
                    // states: is no longer supported — silently drop
                    return Ok(ElementProp::MotionTrigger(MotionTrigger {
                        trigger: TriggerKind::Enter,
                        motion_expr: Expr::Literal(
                            Literal::String("__states_removed".into()),
                            self.peek_span(),
                        ),
                    }));
                }
                // on: { signal } — states signal binding (removed)
                if name == "on" {
                    self.advance(); // consume ':'
                    self.expect_token(TokenKind::LBrace, "expected '{' after 'on:'")?;
                    let _signal = self.parse_expr()?;
                    self.expect_token(TokenKind::RBrace, "expected '}'")?;
                    // states signal binding is no longer supported — silently drop
                    return Ok(ElementProp::MotionTrigger(MotionTrigger {
                        trigger: TriggerKind::Enter,
                        motion_expr: Expr::Literal(
                            Literal::String("__states_removed".into()),
                            self.peek_span(),
                        ),
                    }));
                }
                // variant: DesignName.VariantName(dim=val, ...)
                if name == "variant" {
                    self.advance(); // consume ':'
                    let design_name = self.expect_ident("expected design name")?;
                    self.expect_token(TokenKind::Dot, "expected '.' after design name")?;
                    let variant_name = self.expect_ident("expected variant name")?;

                    // Parse optional dimension value pairs: (dim=val, dim=val, ...)
                    let mut dimension_values = Vec::new();
                    if self.at(&TokenKind::LParen) {
                        self.advance(); // consume '('
                        while !self.at(&TokenKind::RParen) && !self.at(&TokenKind::Eof) {
                            let dim = self.expect_ident("expected dimension name")?;
                            self.expect_token(TokenKind::Eq, "expected '='")?;
                            let val = self.expect_ident("expected dimension value")?;
                            dimension_values.push((dim, val));
                            if self.at(&TokenKind::Comma) {
                                self.advance();
                            }
                        }
                        self.expect_token(TokenKind::RParen, "expected ')'")?;
                    }

                    return Ok(ElementProp::Variant {
                        design_name,
                        variant_name,
                        dimension_values,
                    });
                }
                // style: entries or class: name
                self.advance();
                match self.peek_kind() {
                    TokenKind::LBrace if name == "class" => {
                        // class: { key: expr, ... } — dynamic class map
                        self.advance(); // consume '{'
                        let mut entries = Vec::new();
                        if !self.at(&TokenKind::RBrace) {
                            let key = self.expect_ident("expected class key")?;
                            self.expect_token(TokenKind::Colon, "expected ':'")?;
                            let val = self.parse_expr()?;
                            entries.push((key, val));
                            while self.at(&TokenKind::Comma) {
                                self.advance();
                                let key = self.expect_ident("expected class key")?;
                                self.expect_token(TokenKind::Colon, "expected ':'")?;
                                let val = self.parse_expr()?;
                                entries.push((key, val));
                            }
                        }
                        self.expect_token(TokenKind::RBrace, "expected '}'")?;
                        Ok(ElementProp::Class {
                            value: ClassValue::ClassMap(entries),
                        })
                    }
                    TokenKind::LBrace => {
                        self.advance();
                        let mut entries = Vec::new();
                        if !self.at(&TokenKind::RBrace) {
                            let key = self.parse_css_property_name("expected style key")?;
                            self.expect_token(TokenKind::Colon, "expected ':'")?;
                            let val = self.parse_expr()?;
                            entries.push((key, val));
                            while self.at(&TokenKind::Comma) {
                                self.advance();
                                let key = self.parse_css_property_name("expected style key")?;
                                self.expect_token(TokenKind::Colon, "expected ':'")?;
                                let val = self.parse_expr()?;
                                entries.push((key, val));
                            }
                        }
                        self.expect_token(TokenKind::RBrace, "expected '}'")?;
                        Ok(ElementProp::Style { entries })
                    }
                    _ => {
                        let t = self.advance();
                        let first = match &t.kind {
                            TokenKind::StringLiteral(s) => s.clone(),
                            TokenKind::Ident(s) => {
                                let mut class_name = s.clone();
                                // Handle hyphenated class names: theme-bar, btn-add, demo1-r-0, etc.
                                // Also accept IntLiteral after '-' (CSS allows numbers after hyphens: -0, -1, etc.)
                                while self.at(&TokenKind::Minus) {
                                    self.advance(); // consume '-'
                                    let next: String = match self.peek_kind().clone() {
                                        TokenKind::Ident(_) => self.expect_ident("")?,
                                        TokenKind::IntLiteral(n) => {
                                            self.advance();
                                            n.to_string()
                                        }
                                        ref k => {
                                            // Accept keywords (view, style, theme, etc.) after '-'
                                            if let Some(name) = keyword_as_ident_name(k) {
                                                self.advance();
                                                name.to_string()
                                            } else {
                                                return Err(ParseError {
                                                    message: "expected identifier or number after '-' in class name".into(),
                                                    span: self.peek_span(),
                                                });
                                            }
                                        }
                                    };
                                    class_name = format!("{}-{}", class_name, next);
                                }
                                class_name
                            }
                            ref k => {
                                // Accept keywords that can be used as identifiers (e.g., theme, view, style)
                                if let Some(name) = keyword_as_ident_name(k) {
                                    let mut class_name = name.to_string();
                                    while self.at(&TokenKind::Minus) {
                                        self.advance(); // consume '-'
                                        let kind = self.peek_kind().clone();
                                        let next: String = match kind {
                                            TokenKind::Ident(_) => self.expect_ident("")?,
                                            TokenKind::IntLiteral(n) => {
                                                self.advance();
                                                n.to_string()
                                            }
                                            ref k => {
                                                if let Some(name) = keyword_as_ident_name(k) {
                                                    self.advance();
                                                    name.to_string()
                                                } else {
                                                    return Err(ParseError {
                                                        message: "expected identifier or number after '-' in class name".into(),
                                                        span: self.peek_span(),
                                                    });
                                                }
                                            }
                                        };
                                        class_name = format!("{}-{}", class_name, next);
                                    }
                                    class_name
                                } else {
                                    return Err(ParseError {
                                        message: format!("Expected class name, got {:?}", k),
                                        span: t.span,
                                    });
                                }
                            }
                        };
                        // Check for design class reference: DesignName.ClassName
                        // Detect dot-notation in the first class name
                        if self.at(&TokenKind::Dot) {
                            self.advance(); // consume '.'
                            let class_name = self.expect_ident("expected class name after '.'")?;
                            return Ok(ElementProp::Class {
                                value: ClassValue::DesignClass {
                                    design_name: first,
                                    class_name,
                                },
                            });
                        }
                        // Collect additional space-separated class names: class: btn btn-plus
                        // BUT stop if the next identifier is a known boolean HTML attribute
                        // (e.g. <audio class: media controls> — "controls" is an attribute)
                        let mut classes = vec![first];
                        loop {
                            // Peek — is the next token another class name?
                            let next_is_class = match self.peek_kind() {
                                TokenKind::Ident(_) => true,
                                kind => keyword_as_ident_name(kind).is_some(),
                            };
                            if !next_is_class {
                                break;
                            }
                            // Check: is this actually the start of the next attribute?
                            // An ident followed by '=' or ':' is an attribute, not a class name
                            if self.peek_is_attr_start() {
                                break;
                            }
                            // Check: is this a known boolean attribute keyword?
                            // (e.g., controls, disabled, readonly, loop, muted, etc.)
                            if self.peek_is_boolean_attr_keyword() {
                                break;
                            }
                            let next_t = self.advance();
                            match &next_t.kind {
                                TokenKind::Ident(s) => {
                                    let mut cn = s.clone();
                                    while self.at(&TokenKind::Minus) {
                                        self.advance();
                                        let n = self.expect_ident(
                                            "expected identifier after '-' in class name",
                                        )?;
                                        cn = format!("{}-{}", cn, n);
                                    }
                                    classes.push(cn);
                                }
                                k => {
                                    if let Some(name) = keyword_as_ident_name(k) {
                                        let mut cn = name.to_string();
                                        while self.at(&TokenKind::Minus) {
                                            self.advance();
                                            let n = self.expect_ident(
                                                "expected identifier after '-' in class name",
                                            )?;
                                            cn = format!("{}-{}", cn, n);
                                        }
                                        classes.push(cn);
                                    } else {
                                        // Shouldn't happen — we already checked
                                        break;
                                    }
                                }
                            }
                        }
                        // Check for dynamic class expression: class: name { expr }
                        if self.at(&TokenKind::LBrace) {
                            // Parse class: name(s) { conditional_expr }
                            // e.g., class: tab-btn {if activeDemo == "position" { "active" } else { "" }}
                            // Combine static names with dynamic expression result
                            let span = self.peek_span();
                            let static_part =
                                Expr::Literal(Literal::String(classes.join(" ")), span);
                            self.advance(); // consume '{'
                            let dyn_expr = self.parse_expr()?;
                            self.expect_token(TokenKind::RBrace, "expected '}'")?;
                            let combined = Expr::BinaryOp {
                                left: Box::new(static_part),
                                op: BinaryOpKind::Add,
                                right: Box::new(dyn_expr),
                                span,
                            };
                            // Emit as a Static className prop so codegen handles it naturally
                            return Ok(ElementProp::Static {
                                name: "className".into(),
                                value: combined,
                            });
                        }

                        Ok(ElementProp::Class {
                            value: ClassValue::StyleRef(classes.join(" ")),
                        })
                    }
                }
            }
            _ => {
                // Boolean attribute: just the name, e.g., <input disabled>
                Ok(ElementProp::Static {
                    name,
                    value: Expr::Literal(
                        Literal::Bool(true),
                        Span {
                            start: 0,
                            end: 0,
                            line: 0,
                            col: 0,
                        },
                    ),
                })
            }
        }
    }

    /// Parse bind:property={signal} for two-way binding.
    pub(super) fn parse_bind_prop(&mut self) -> Result<ElementProp, ParseError> {
        self.advance(); // 'bind'
        self.expect_token(TokenKind::Colon, "expected ':'")?;
        let name = self.expect_ident("expected bind property name")?;

        // Parse ={signal_path}
        let signal_path = if self.at(&TokenKind::Eq) {
            self.advance(); // '='
            let kind = self.peek_kind().clone();
            match kind {
                TokenKind::LBrace => {
                    self.advance();
                    let path = self.expect_ident("expected signal path")?;
                    // Handle nested paths like "store.field"
                    let path = if self.at(&TokenKind::Dot) {
                        self.advance();
                        let field = self.expect_ident("expected field name")?;
                        format!("{}.{}", path, field)
                    } else {
                        path
                    };
                    self.expect_token(TokenKind::RBrace, "expected '}'")?;
                    path
                }
                TokenKind::Ident(id) => {
                    self.advance();
                    id.clone()
                }
                _ => {
                    return Err(ParseError {
                        message: "Expected {signalPath} or signalName for bind value".into(),
                        span: self.peek_span(),
                    })
                }
            }
        } else {
            return Err(ParseError {
                message: "Expected '=' after bind:property".into(),
                span: self.peek_span(),
            });
        };

        Ok(ElementProp::Bind { name, signal_path })
    }

    /// Parse @eventName={handler} for event binding.
    pub(super) fn parse_event_prop(&mut self) -> Result<ElementProp, ParseError> {
        let t = self.advance();
        let event_name = match &t.kind {
            TokenKind::At => {
                // Traditional: @ is separate token
                self.expect_ident("expected event name")?
            }
            TokenKind::Ident(name) if name.starts_with('@') => {
                // Lexer combined: @onPress as single Ident token
                name[1..].to_string() // strip '@' prefix
            }
            _ => {
                return Err(ParseError {
                    message: format!("Expected @event binding, got {:?}", t.kind),
                    span: t.span,
                })
            }
        };
        // @ref={name} is a ref binding, not an event
        if event_name == "ref" {
            let handler = self.parse_event_handler_value()?;
            let ref_name = match &handler {
                EventHandler::FnRef(n) => n.clone(),
                _ => {
                    return Err(ParseError {
                        message: format!("Expected ref name, got {:?}", handler),
                        span: t.span,
                    })
                }
            };
            return Ok(ElementProp::Ref(ref_name));
        }

        let handler = self.parse_event_handler_value()?;
        Ok(ElementProp::Event {
            name: event_name,
            handler,
        })
    }

    /// Parse onEvent={handler} (no @ prefix) — event name is already known.
    pub(super) fn parse_event_prop_no_at(
        &mut self,
        event_name: &str,
    ) -> Result<ElementProp, ParseError> {
        // Consume the event name ident token
        self.advance();
        let handler = self.parse_event_handler_value()?;
        Ok(ElementProp::Event {
            name: event_name.to_string(),
            handler,
        })
    }

    /// Parse the ={handler} part of an event binding.
    pub(super) fn parse_event_handler_value(&mut self) -> Result<EventHandler, ParseError> {
        if !self.at(&TokenKind::Eq) {
            return Err(ParseError {
                message: "Expected '=' after event name".into(),
                span: self.peek_span(),
            });
        }
        self.advance(); // '='
        match self.peek_kind() {
            TokenKind::LBrace => {
                self.advance();
                let handler = if self.at(&TokenKind::LParen) {
                    // Inline fn: (params) => { body }
                    self.advance();
                    let params = self.parse_params()?;
                    self.expect_token(TokenKind::RParen, "expected ')'")?;
                    self.expect_token(TokenKind::FatArrow, "expected '=>'")?;
                    let body = self.parse_block()?;
                    EventHandler::InlineFn { params, body }
                } else {
                    // Handler expression: fnName, fnName(args), store.action,
                    // assignment (dragStatus = "dragging"), or any expression.
                    // Parse as expression; wrap non-simple cases in inline fn.
                    let expr = self.parse_expr()?;
                    match &expr {
                        Expr::VarRef(name, _) => EventHandler::FnRef(name.clone()),
                        _ => {
                            // Wrap any other expression (fn call, assignment, etc.)
                            // in an inline fn body: () => { expr; }
                            let expr_span = *crate::parser::expr::span_of(&expr);
                            let body = Block {
                                stmts: vec![Stmt::ExprStmt {
                                    expr,
                                    span: expr_span,
                                }],
                                trailing_expr: None,
                                span: expr_span,
                            };
                            EventHandler::InlineFn {
                                params: vec![],
                                body,
                            }
                        }
                    }
                };
                self.expect_token(TokenKind::RBrace, "expected '}'")?;
                Ok(handler)
            }
            _ => {
                let t = self.advance();
                Err(ParseError {
                    message: format!("Expected {{handler}} after '=', got {:?}", t.kind),
                    span: t.span,
                })
            }
        }
    }
}
