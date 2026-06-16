//! View block and element parser for OmniView components.

use crate::error::ParseError;
use crate::lexer::{Span, TokenKind};
use crate::parser::ast::*;
use crate::parser::Parser;

impl<'a> Parser<'a> {
    /// Parse a view block: view { <Element>* }
    pub fn parse_view_block(&mut self) -> Result<ViewBlock, ParseError> {
        self.advance(); // 'view'
        self.expect_token(TokenKind::LBrace, "expected '{'")?;

        let mut children = Vec::new();
        while self.peek_kind() != TokenKind::RBrace && self.peek_kind() != TokenKind::Eof {
            children.push(self.parse_view_node()?);
        }

        self.expect_token(TokenKind::RBrace, "expected '}'")?;
        Ok(ViewBlock { children })
    }

    /// Parse a single view node: element, directive, or text.
    fn parse_view_node(&mut self) -> Result<ViewNode, ParseError> {
        match self.peek_kind() {
            TokenKind::Lt => {
                // HTML-like element: <Tag ...> or <Tag .../>
                self.parse_element()
            }
            TokenKind::AtIf => self.parse_if_directive(),
            TokenKind::AtFor => self.parse_for_directive(),
            TokenKind::AtSlot => {
                self.advance();
                let name = if self.peek_kind() == TokenKind::LParen {
                    self.advance();
                    let n = self.expect_ident("expected slot name")?;
                    self.expect_token(TokenKind::RParen, "expected ')'")?;
                    Some(n)
                } else {
                    None
                };
                Ok(ViewNode::SlotDirective(name))
            }
            TokenKind::StringLiteral(s) => {
                let text = s.clone();
                self.advance();
                Ok(ViewNode::Text(text))
            }
            TokenKind::LBrace => {
                // Interpolation: { expr }
                self.advance();
                let expr = self.parse_expr()?;
                self.expect_token(TokenKind::RBrace, "expected '}'")?;
                Ok(ViewNode::Interpolation(expr))
            }
            _ => {
                let t = self.advance();
                Err(ParseError {
                    message: format!("Unexpected token in view: {:?}", t.kind),
                    span: t.span,
                })
            }
        }
    }

    /// Parse an HTML-like element: <Tag prop1="val" prop2={expr}> children </Tag>
    fn parse_element(&mut self) -> Result<ViewNode, ParseError> {
        let start = self.peek_span();
        self.advance(); // '<'
        let tag = self.expect_ident("expected element tag")?;

        // Parse props until '>' or '/>'
        let mut props = Vec::new();
        let mut self_closing = false;

        loop {
            match self.peek_kind() {
                TokenKind::Gt => {
                    self.advance();
                    break;
                }
                TokenKind::Slash => {
                    self.advance();
                    self.expect_token(TokenKind::Gt, "expected '>' after '/'")?;
                    self_closing = true;
                    break;
                }
                TokenKind::Ident(ref name) if name.starts_with('@') => {
                    let prop = self.parse_event_prop()?;
                    props.push(prop);
                }
                TokenKind::Ident(_) => {
                    let prop = self.parse_element_prop()?;
                    props.push(prop);
                }
                TokenKind::Bind => {
                    let prop = self.parse_bind_prop()?;
                    props.push(prop);
                }
                TokenKind::At => {
                    let prop = self.parse_event_prop()?;
                    props.push(prop);
                }
                _ => {
                    let t = self.advance();
                    return Err(ParseError {
                        message: format!("Unexpected token in element props: {:?}", t.kind),
                        span: t.span,
                    });
                }
            }
        }

        // Parse children if not self-closing
        let mut children = Vec::new();
        if !self_closing {
            loop {
                match self.peek_kind() {
                    TokenKind::Lt => {
                        // Check if it's a closing tag
                        if self.check_closing_tag(&tag) {
                            self.advance(); // '<'
                            self.advance(); // '/'
                            self.expect_ident("expected closing tag")?;
                            self.expect_token(TokenKind::Gt, "expected '>'")?;
                            break;
                        }
                        children.push(self.parse_view_node()?);
                    }
                    TokenKind::AtIf | TokenKind::AtFor | TokenKind::AtSlot => {
                        children.push(self.parse_view_node()?);
                    }
                    TokenKind::StringLiteral(_) | TokenKind::LBrace => {
                        children.push(self.parse_view_node()?);
                    }
                    // Collect consecutive text tokens (Idents, colons, etc.) into one Text node
                    _ if is_text_token(&self.peek_kind()) => {
                        let mut text = String::new();
                        while is_text_token(&self.peek_kind()) {
                            let t = self.advance();
                            text.push_str(&token_text(&t.kind));
                        }
                        if !text.is_empty() {
                            children.push(ViewNode::Text(text));
                        }
                    }
                    _ => {
                        break;
                    }
                }
            }
        }

        let end = self.peek_span();
        Ok(ViewNode::Element(Element {
            tag,
            props,
            children,
            self_closing,
            span: Span {
                start: start.start,
                end: end.end,
                line: start.line,
                col: start.col,
            },
        }))
    }

    fn check_closing_tag(&self, tag: &str) -> bool {
        let tokens = self.tokens;
        let pos = self.pos;
        if pos + 2 >= tokens.len() {
            return false;
        }
        matches!(tokens[pos].kind, TokenKind::Lt)
            && matches!(tokens[pos + 1].kind, TokenKind::Slash)
            && matches!(&tokens[pos + 2].kind, TokenKind::Ident(t) if t == tag)
    }

    /// Parse a static prop: name="value" or name={expr}
    fn parse_element_prop(&mut self) -> Result<ElementProp, ParseError> {
        let name = self.expect_ident("expected prop name")?;

        match self.peek_kind() {
            TokenKind::Eq => {
                self.advance();
                match self.peek_kind() {
                    TokenKind::StringLiteral(_) => {
                        let t = self.advance();
                        if let TokenKind::StringLiteral(s) = &t.kind {
                            Ok(ElementProp::Static {
                                name,
                                value: Expr::Literal(Literal::String(s.clone()), t.span.clone()),
                            })
                        } else {
                            unreachable!()
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
                            message: format!("Expected string or {{expr}} for prop value, got {:?}", t.kind),
                            span: t.span,
                        })
                    }
                }
            }
            TokenKind::Colon => {
                // style: entries or class: name
                self.advance();
                match self.peek_kind() {
                    TokenKind::LBrace => {
                        self.advance();
                        let mut entries = Vec::new();
                        if self.peek_kind() != TokenKind::RBrace {
                            let key = self.expect_ident("expected style key")?;
                            self.expect_token(TokenKind::Colon, "expected ':'")?;
                            let val = self.parse_expr()?;
                            entries.push((key, val));
                            while self.peek_kind() == TokenKind::Comma {
                                self.advance();
                                let key = self.expect_ident("expected style key")?;
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
                        let val = match &t.kind {
                            TokenKind::StringLiteral(s) => ClassValue::StringLiteral(s.clone()),
                            TokenKind::Ident(s) => ClassValue::StyleRef(s.clone()),
                            _ => return Err(ParseError {
                                message: format!("Expected class name, got {:?}", t.kind),
                                span: t.span,
                            }),
                        };
                        Ok(ElementProp::Class { value: val })
                    }
                }
            }
            _ => {
                // Boolean attribute: just the name, e.g., <input disabled>
                Ok(ElementProp::Static {
                    name,
                    value: Expr::Literal(Literal::Bool(true), Span {
                        start: 0, end: 0, line: 0, col: 0,
                    }),
                })
            }
        }
    }

    /// Parse bind:signalPath for two-way binding.
    fn parse_bind_prop(&mut self) -> Result<ElementProp, ParseError> {
        self.advance(); // 'bind'
        let name = self.expect_ident("expected bind property name")?;
        self.expect_token(TokenKind::Colon, "expected ':'")?;
        let signal_path = self.expect_ident("expected signal path")?;

        // Handle nested paths like "store.field"
        let signal_path = if self.peek_kind() == TokenKind::Dot {
            self.advance();
            let field = self.expect_ident("expected field name")?;
            format!("{}.{}", signal_path, field)
        } else {
            signal_path
        };

        Ok(ElementProp::Bind { name, signal_path })
    }

    /// Parse @eventName={handler} for event binding.
    fn parse_event_prop(&mut self) -> Result<ElementProp, ParseError> {
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
            _ => return Err(ParseError {
                message: format!("Expected @event binding, got {:?}", t.kind),
                span: t.span,
            }),
        };
        let handler = if self.peek_kind() == TokenKind::Eq {
            self.advance();
            match self.peek_kind() {
                TokenKind::LBrace => {
                    self.advance();
                    // Check if it's an inline fn: (params) => { body }
                    let handler = if self.peek_kind() == TokenKind::LParen {
                        self.advance();
                        let params = self.parse_params()?;
                        self.expect_token(TokenKind::RParen, "expected ')'")?;
                        self.expect_token(TokenKind::FatArrow, "expected '=>'")?;
                        let body = self.parse_block()?;
                        EventHandler::InlineFn { params, body }
                    } else {
                        // fn reference: fnName or store.action
                        let t = self.advance();
                        match &t.kind {
                            TokenKind::Ident(name) => {
                                if self.peek_kind() == TokenKind::Dot {
                                    self.advance();
                                    let action = self.expect_ident("expected action name")?;
                                    EventHandler::StoreAction {
                                        store: name.clone(),
                                        action,
                                    }
                                } else {
                                    EventHandler::FnRef(name.clone())
                                }
                            }
                            _ => return Err(ParseError {
                                message: format!("Expected event handler, got {:?}", t.kind),
                                span: t.span,
                            }),
                        }
                    };
                    self.expect_token(TokenKind::RBrace, "expected '}'")?;
                    handler
                }
                _ => {
                    let t = self.advance();
                    return Err(ParseError {
                        message: format!("Expected {{handler}} after '=', got {:?}", t.kind),
                        span: t.span,
                    });
                }
            }
        } else {
            return Err(ParseError {
                message: "Expected '=' after event name".into(),
                span: self.peek_span(),
            });
        };

        Ok(ElementProp::Event {
            name: event_name,
            handler,
        })
    }

    // ── Directives ─────────────────────────────────────

    fn parse_if_directive(&mut self) -> Result<ViewNode, ParseError> {
        self.advance(); // '@if'
        let condition = self.parse_expr()?;
        self.expect_token(TokenKind::LBrace, "expected '{'")?;

        let mut body = Vec::new();
        while self.peek_kind() != TokenKind::RBrace && self.peek_kind() != TokenKind::Eof {
            body.push(self.parse_view_node()?);
        }
        self.expect_token(TokenKind::RBrace, "expected '}'")?;

        let mut else_ifs = Vec::new();
        let mut else_body = None;

        while self.peek_kind() == TokenKind::AtElse {
            self.advance();
            if self.peek_kind() == TokenKind::AtIf {
                self.advance(); // '@if' after '@else'
                let cond = self.parse_expr()?;
                self.expect_token(TokenKind::LBrace, "expected '{'")?;
                let mut b = Vec::new();
                while self.peek_kind() != TokenKind::RBrace && self.peek_kind() != TokenKind::Eof {
                    b.push(self.parse_view_node()?);
                }
                self.expect_token(TokenKind::RBrace, "expected '}'")?;
                else_ifs.push((cond, b));
            } else {
                self.expect_token(TokenKind::LBrace, "expected '{'")?;
                let mut b = Vec::new();
                while self.peek_kind() != TokenKind::RBrace && self.peek_kind() != TokenKind::Eof {
                    b.push(self.parse_view_node()?);
                }
                self.expect_token(TokenKind::RBrace, "expected '}'")?;
                else_body = Some(b);
                break;
            }
        }

        Ok(ViewNode::IfDirective(IfDirective {
            condition,
            body,
            else_ifs,
            else_body,
        }))
    }

    fn parse_for_directive(&mut self) -> Result<ViewNode, ParseError> {
        self.advance(); // '@for'
        let item_name = self.expect_ident("expected item variable name")?;
        self.expect_token(TokenKind::In, "expected 'in'")?;
        let iterable = self.parse_expr()?;
        self.expect_token(TokenKind::LBrace, "expected '{'")?;

        let mut body = Vec::new();
        while self.peek_kind() != TokenKind::RBrace && self.peek_kind() != TokenKind::Eof {
            body.push(self.parse_view_node()?);
        }
        self.expect_token(TokenKind::RBrace, "expected '}'")?;

        let mut empty_body = None;
        if self.peek_kind() == TokenKind::AtEmpty {
            self.advance();
            self.expect_token(TokenKind::LBrace, "expected '{'")?;
            let mut b = Vec::new();
            while self.peek_kind() != TokenKind::RBrace && self.peek_kind() != TokenKind::Eof {
                b.push(self.parse_view_node()?);
            }
            self.expect_token(TokenKind::RBrace, "expected '}'")?;
            empty_body = Some(b);
        }

        Ok(ViewNode::ForDirective(ForDirective {
            item_name,
            iterable,
            body,
            empty_body,
        }))
    }

    // ── Style rule ─────────────────────────────────────

    pub fn parse_style_rule(&mut self) -> Result<StyleRule, ParseError> {
        self.advance(); // 'style'
        let class_name = self.expect_ident("expected class name")?;
        self.expect_token(TokenKind::LBrace, "expected '{'")?;

        let mut properties = Vec::new();
        while self.peek_kind() != TokenKind::RBrace && self.peek_kind() != TokenKind::Eof {
            let key = self.expect_ident("expected property name")?;
            self.expect_token(TokenKind::Colon, "expected ':'")?;

            let t = self.advance();
            let value = match &t.kind {
                TokenKind::IntLiteral(n) => StyleValue::Number(*n as f64),
                TokenKind::FloatLiteral(n) => StyleValue::Number(*n),
                TokenKind::StringLiteral(s) => StyleValue::String(s.clone()),
                TokenKind::Ident(s) => {
                    // Check for dimension: 16px, 1.5rem
                    if let Some(unit) = try_parse_dimension(s) {
                        StyleValue::Dimension(unit.0, unit.1)
                    } else if s.starts_with('#') {
                        StyleValue::HexColor(s.clone())
                    } else {
                        StyleValue::Ident(s.clone())
                    }
                }
                _ => return Err(ParseError {
                    message: format!("Unexpected style value: {:?}", t.kind),
                    span: t.span,
                }),
            };

            properties.push((key, value));
            if self.peek_kind() == TokenKind::Semicolon {
                self.advance();
            }
        }

        self.expect_token(TokenKind::RBrace, "expected '}'")?;
        Ok(StyleRule { class_name, properties })
    }
}

/// Try to parse a string like "16px" into (16.0, "px").
fn try_parse_dimension(s: &str) -> Option<(f64, String)> {
    let units = ["px", "rem", "em", "vh", "vw", "%", "pt", "cm", "mm"];
    for unit in &units {
        if s.ends_with(unit) {
            let num_str = &s[..s.len() - unit.len()];
            if let Ok(n) = num_str.parse::<f64>() {
                return Some((n, unit.to_string()));
            }
        }
    }
    None
}

/// Check if a token kind represents text content between view elements.
fn is_text_token(kind: &TokenKind) -> bool {
    use crate::lexer::TokenKind;
    matches!(
        kind,
        TokenKind::Ident(_)
            | TokenKind::Colon
            | TokenKind::Comma
            | TokenKind::Dot
            | TokenKind::IntLiteral(_)
            | TokenKind::FloatLiteral(_)
            | TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Star
            | TokenKind::Slash
            | TokenKind::QuestionMark
            | TokenKind::Not
            | TokenKind::LParen
            | TokenKind::RParen
    )
}

/// Convert a text token to its string representation.
fn token_text(kind: &TokenKind) -> String {
    use crate::lexer::TokenKind;
    match kind {
        TokenKind::Ident(s) => s.clone(),
        TokenKind::IntLiteral(n) => n.to_string(),
        TokenKind::FloatLiteral(f) => f.to_string(),
        TokenKind::Colon => ":".into(),
        TokenKind::Comma => ",".into(),
        TokenKind::Dot => ".".into(),
        TokenKind::Plus => "+".into(),
        TokenKind::Minus => "-".into(),
        TokenKind::Star => "*".into(),
        TokenKind::Slash => "/".into(),
        TokenKind::QuestionMark => "?".into(),
        TokenKind::Not => "!".into(),
        TokenKind::LParen => "(".into(),
        TokenKind::RParen => ")".into(),
        _ => String::new(),
    }
}
