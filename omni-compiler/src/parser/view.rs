//! View block and element parser for OmniView components.

use crate::error::ParseError;
use crate::lexer::{Span, TokenKind};
use crate::parser::ast::*;
use crate::parser::stmt::keyword_as_ident_name;
use crate::parser::Parser;

/// Convert a StyleValue to a StyleValuePart for multi-value collection.
fn style_value_to_part(value: &StyleValue) -> StyleValuePart {
    match value {
        StyleValue::Number(n) => StyleValuePart::Number(*n),
        StyleValue::Ident(s) => StyleValuePart::Ident(s.clone()),
        StyleValue::HexColor(h) => StyleValuePart::HexColor(h.clone()),
        StyleValue::Dimension(v, u) => StyleValuePart::Dimension(*v, u.clone()),
        StyleValue::String(s) => StyleValuePart::Ident(s.clone()),
        StyleValue::List(parts) => parts
            .first()
            .cloned()
            .unwrap_or(StyleValuePart::Number(0.0)),
    }
}

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
        self.enter_recursion()?;
        let result = match self.peek_kind() {
            TokenKind::Lt => {
                // HTML-like element: <Tag ...> or <Tag .../>
                self.parse_element()
            }
            TokenKind::AtIf => self.parse_if_directive(),
            TokenKind::AtFor => self.parse_for_directive(),
            TokenKind::AtSlot => {
                self.advance();
                let name = if self.peek_kind() == TokenKind::LParen {
                    // @slot(name) syntax
                    self.advance();
                    let n = self.expect_ident("expected slot name")?;
                    self.expect_token(TokenKind::RParen, "expected ')'")?;
                    Some(n)
                } else if matches!(self.peek_kind(), TokenKind::Ident(_))
                    || keyword_as_ident_name(&self.peek_kind()).is_some()
                {
                    // @slot name syntax (without parens)
                    let n = self.expect_ident("expected slot name")?;
                    Some(n)
                } else {
                    // @slot — default/unnamed slot
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
                // Interpolation: { expr } or empty braces {} (JSX comments stripped)
                self.advance();
                if self.peek_kind() == TokenKind::RBrace {
                    self.advance(); // consume '}'
                    Ok(ViewNode::Text(String::new())) // Empty braces → empty text
                } else {
                    let expr = self.parse_expr()?;
                    self.expect_token(TokenKind::RBrace, "expected '}'")?;
                    Ok(ViewNode::Interpolation(expr))
                }
            }
            // Component call syntax: Card(prop: value, ...) { children }
            // Only PascalCase names start component calls; camelCase is raw text
            _ if self.is_component_call_start() => self.parse_component_call(),
            // Raw text content in elements
            _ => self.parse_raw_text_node(),
        };
        self.leave_recursion();
        result
    }

    /// Parse raw text content inside an element.
    /// Collects consecutive non-special tokens until a view boundary is hit,
    /// reconstructing text from token source spans.
    fn parse_raw_text_node(&mut self) -> Result<ViewNode, ParseError> {
        let start_span = self.peek().span.clone();
        let mut end_span = start_span.clone();

        while self.pos < self.tokens.len() {
            let kind = self.peek_kind();
            // Stop at view boundaries
            match kind {
                TokenKind::Lt
                | TokenKind::LBrace
                | TokenKind::RBrace
                | TokenKind::AtIf
                | TokenKind::AtFor
                | TokenKind::AtSlot
                | TokenKind::Eof
                | TokenKind::StringLiteral(_) => break,
                _ => {
                    // Check if this could start a component call
                    if self.is_component_call_start() {
                        break;
                    }
                }
            }
            let t = self.advance();
            end_span = t.span;
        }

        // Reconstruct text from source using byte spans (preserves whitespace).
        // Guard against tests passing empty source — spans then index past end.
        let text = if start_span.start < self.source.len()
            && end_span.end <= self.source.len()
            && start_span.start <= end_span.end
        {
            self.source[start_span.start..end_span.end].to_string()
        } else {
            String::new()
        };
        if text.is_empty() {
            // No text collected — this can happen at EOF. Skip the node.
            return Ok(ViewNode::Text(String::new()));
        }
        Ok(ViewNode::Text(text))
    }

    /// Check if current position starts a component call: Name(...
    /// Only PascalCase names start component calls; lowercase/camelCase is text.
    fn is_component_call_start(&self) -> bool {
        let kind = self.peek_kind();
        // Extract the identifier name, checking for PascalCase
        let name: Option<String> = match &kind {
            TokenKind::Ident(s) => Some(s.clone()),
            _ => keyword_as_ident_name(&kind).map(|s| s.to_string()),
        };
        let Some(name) = name else { return false };
        // Only PascalCase starts a component call (e.g., Card, Button, Flex)
        // camelCase like math_abs or getBtnBg is raw text
        if !name.starts_with(|c: char| c.is_uppercase()) {
            return false;
        }
        // Check next token is '(' (props) or '{' (children only)
        self.tokens
            .get(self.pos + 1)
            .is_some_and(|t| matches!(t.kind, TokenKind::LParen | TokenKind::LBrace))
    }

    /// Parse component call syntax: Name(prop: value, ...) { children }
    fn parse_component_call(&mut self) -> Result<ViewNode, ParseError> {
        let t = self.advance();
        let tag = match &t.kind {
            TokenKind::Ident(name) => name.clone(),
            other => keyword_as_ident_name(other).unwrap_or_else(|| "unknown".into()),
        };

        let mut props = Vec::new();

        // Parse optional props: (name: value, ...)
        if self.peek_kind() == TokenKind::LParen {
            self.advance(); // '('
            while self.peek_kind() != TokenKind::RParen && self.peek_kind() != TokenKind::Eof {
                let prop_name = self.expect_ident("expected prop name")?;
                self.expect_token(TokenKind::Colon, "expected ':'")?;
                let prop_value = self.parse_expr()?;
                props.push(ElementProp::Static {
                    name: prop_name,
                    value: prop_value,
                });
                if self.peek_kind() == TokenKind::Comma {
                    self.advance();
                }
            }
            self.expect_token(TokenKind::RParen, "expected ')'")?;
        }

        // Parse optional children block: { ... }
        let mut children = Vec::new();
        let self_closing = if self.peek_kind() == TokenKind::LBrace {
            self.advance();
            while self.peek_kind() != TokenKind::RBrace && self.peek_kind() != TokenKind::Eof {
                children.push(self.parse_view_node()?);
            }
            self.expect_token(TokenKind::RBrace, "expected '}'")?;
            false
        } else {
            true // Self-closing: no children block, no props block
        };

        Ok(ViewNode::Element(Element {
            tag,
            props,
            children,
            self_closing,
            span: Span {
                start: t.span.start,
                end: self.peek_span().end,
                line: t.span.line,
                col: t.span.col,
            },
        }))
    }

    /// Parse an HTML-like element: <Tag prop1="val" prop2={expr}> children </Tag>
    /// Also handles HTML comments: <!-- ... -->
    fn parse_element(&mut self) -> Result<ViewNode, ParseError> {
        let start = self.peek_span();
        self.advance(); // '<'

        // Handle HTML comment: <!-- ... -->
        if self.peek_kind() == TokenKind::Not {
            self.advance(); // '!'
            self.expect_token(TokenKind::Minus, "expected '-' in HTML comment")?;
            self.expect_token(TokenKind::Minus, "expected '-' in HTML comment")?;
            // Skip until --> (tokenized as Minus + Arrow because lexer
            // merges `->` into a single Arrow token)
            loop {
                if self.peek_kind() == TokenKind::Minus
                    && self.tokens.get(self.pos + 1).map(|t| &t.kind) == Some(&TokenKind::Arrow)
                {
                    self.advance(); // '-'
                    self.advance(); // '->' (Arrow)
                    break;
                }
                if self.peek_kind() == TokenKind::Eof {
                    return Err(ParseError {
                        message: "unterminated HTML comment, expected '-->'".into(),
                        span: self.peek_span(),
                    });
                }
                self.advance();
            }
            // HTML comment emits no content node
            return Ok(ViewNode::Text(String::new()));
        }

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
                TokenKind::Ident(ref name) if is_event_like(name) => {
                    // onPress, onClick, onChange, etc. without @ prefix
                    let prop = self.parse_event_prop_no_at(name)?;
                    props.push(prop);
                }
                TokenKind::Ident(_) => {
                    let prop = self.parse_element_prop()?;
                    props.push(prop);
                }
                // Contextual keywords as attribute names (type, style, etc.)
                ref kind if is_keyword_attr_name(kind) => {
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
                // ── Motion trigger attributes @enter={...}, @hover={...} ──
                TokenKind::AtEnter => {
                    let trigger = self.parse_motion_trigger(TriggerKind::Enter)?;
                    props.push(ElementProp::MotionTrigger(trigger));
                }
                TokenKind::AtLeave => {
                    let trigger = self.parse_motion_trigger(TriggerKind::Leave)?;
                    props.push(ElementProp::MotionTrigger(trigger));
                }
                TokenKind::AtHover => {
                    let trigger = self.parse_motion_trigger(TriggerKind::Hover)?;
                    props.push(ElementProp::MotionTrigger(trigger));
                }
                TokenKind::AtUnhover => {
                    let trigger = self.parse_motion_trigger(TriggerKind::Unhover)?;
                    props.push(ElementProp::MotionTrigger(trigger));
                }
                TokenKind::AtPress => {
                    let trigger = self.parse_motion_trigger(TriggerKind::Press)?;
                    props.push(ElementProp::MotionTrigger(trigger));
                }
                TokenKind::AtFocus => {
                    let trigger = self.parse_motion_trigger(TriggerKind::Focus)?;
                    props.push(ElementProp::MotionTrigger(trigger));
                }
                TokenKind::AtAnimate => {
                    let trigger = self.parse_motion_trigger(TriggerKind::Animate)?;
                    props.push(ElementProp::MotionTrigger(trigger));
                }
                TokenKind::AtInView => {
                    let trigger = self.parse_motion_trigger(TriggerKind::InView)?;
                    props.push(ElementProp::MotionTrigger(trigger));
                }
                TokenKind::AtScroll => {
                    let trigger = self.parse_motion_trigger(TriggerKind::Scroll)?;
                    props.push(ElementProp::MotionTrigger(trigger));
                }
                TokenKind::AtDragHorizontal => {
                    let trigger = self.parse_motion_trigger(TriggerKind::DragHorizontal)?;
                    props.push(ElementProp::MotionTrigger(trigger));
                }
                TokenKind::AtDragVertical => {
                    let trigger = self.parse_motion_trigger(TriggerKind::DragVertical)?;
                    props.push(ElementProp::MotionTrigger(trigger));
                }
                TokenKind::AtDrivenBy => {
                    // @driven-by={signal} — external progress control
                    self.advance();
                    self.expect_token(TokenKind::Eq, "expected '='")?;
                    self.expect_token(TokenKind::LBrace, "expected '{'")?;
                    let signal = self.parse_expr()?;
                    self.expect_token(TokenKind::RBrace, "expected '}'")?;
                    props.push(ElementProp::DrivenBy(signal));
                }
                TokenKind::AtTimeline => {
                    let trigger = self.parse_motion_trigger(TriggerKind::Timeline)?;
                    props.push(ElementProp::MotionTrigger(trigger));
                }
                TokenKind::AtPaused => {
                    self.advance();
                    self.expect_token(TokenKind::Eq, "expected '='")?;
                    self.expect_token(TokenKind::LBrace, "expected '{'")?;
                    let val = match self.parse_expr()? {
                        Expr::Literal(Literal::Bool(b), _) => b,
                        _ => true, // non-literal (e.g., signal ref): default to true
                    };
                    self.expect_token(TokenKind::RBrace, "expected '}'")?;
                    props.push(ElementProp::PausedOverride(val));
                }
                TokenKind::AtHoverReverse => {
                    self.advance();
                    self.expect_token(TokenKind::Eq, "expected '='")?;
                    self.expect_token(TokenKind::LBrace, "expected '{'")?;
                    let mode = self.expect_ident("expected hover reverse mode")?;
                    self.expect_token(TokenKind::RBrace, "expected '}'")?;
                    props.push(ElementProp::HoverReverse(mode));
                }
                TokenKind::AtInViewOnce
                | TokenKind::AtInViewRepeat
                | TokenKind::AtInViewThreshold => {
                    let kind = self.advance().kind.clone();
                    self.expect_token(TokenKind::Eq, "expected '='")?;
                    self.expect_token(TokenKind::LBrace, "expected '{'")?;
                    let val = self.parse_expr()?;
                    self.expect_token(TokenKind::RBrace, "expected '}'")?;
                    match kind {
                        TokenKind::AtInViewOnce => props.push(ElementProp::InViewOnce(val)),
                        TokenKind::AtInViewRepeat => props.push(ElementProp::InViewRepeat(val)),
                        TokenKind::AtInViewThreshold => {
                            props.push(ElementProp::InViewThreshold(val))
                        }
                        _ => {}
                    }
                }
                // Scroll sub-attributes
                TokenKind::AtScrollStart | TokenKind::AtScrollEnd => {
                    let kind = self.advance().kind.clone();
                    self.expect_token(TokenKind::Eq, "expected '='")?;
                    let val = self.expect_string("expected string value")?;
                    match kind {
                        TokenKind::AtScrollStart => props.push(ElementProp::ScrollStart(val)),
                        TokenKind::AtScrollEnd => props.push(ElementProp::ScrollEnd(val)),
                        _ => {}
                    }
                }
                TokenKind::AtScrollScrub => {
                    self.advance();
                    self.expect_token(TokenKind::Eq, "expected '='")?;
                    self.expect_token(TokenKind::LBrace, "expected '{'")?;
                    let val = self.parse_expr()?;
                    self.expect_token(TokenKind::RBrace, "expected '}'")?;
                    props.push(ElementProp::ScrollScrub(val));
                }
                // Drag sub-attributes
                TokenKind::DragConstraints => {
                    self.advance();
                    self.expect_token(TokenKind::Eq, "expected '='")?;
                    let val = self.expect_string("expected string value")?;
                    props.push(ElementProp::DragConstraints(val));
                }
                TokenKind::DragElastic
                | TokenKind::DragScrub
                | TokenKind::DragMomentum
                | TokenKind::DragThreshold
                | TokenKind::DragVelocityMax => {
                    let kind = self.advance().kind.clone();
                    self.expect_token(TokenKind::Eq, "expected '='")?;
                    self.expect_token(TokenKind::LBrace, "expected '{'")?;
                    let val = self.parse_expr()?;
                    self.expect_token(TokenKind::RBrace, "expected '}'")?;
                    match kind {
                        TokenKind::DragElastic => props.push(ElementProp::DragElastic(val)),
                        TokenKind::DragScrub => props.push(ElementProp::DragScrub(val)),
                        TokenKind::DragMomentum => props.push(ElementProp::DragMomentum(val)),
                        TokenKind::DragThreshold => props.push(ElementProp::DragThreshold(val)),
                        TokenKind::DragVelocityMax => props.push(ElementProp::DragVelocityMax(val)),
                        _ => {}
                    }
                }
                // Hero attributes
                TokenKind::AtHero => {
                    // @hero="tag-name"
                    self.advance();
                    self.expect_token(TokenKind::Eq, "expected '='")?;
                    let val = self.expect_string("expected string value")?;
                    props.push(ElementProp::HeroTag(val));
                }
                TokenKind::AtHeroTransition => {
                    // @hero-transition={motionName}
                    self.advance();
                    self.expect_token(TokenKind::Eq, "expected '='")?;
                    self.expect_token(TokenKind::LBrace, "expected '{'")?;
                    let val = self.parse_expr()?;
                    self.expect_token(TokenKind::RBrace, "expected '}'")?;
                    props.push(ElementProp::HeroTransition(val));
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
                    // Collect consecutive text tokens (Idents, colons, etc.) into one Text node.
                    // Reconstruct whitespace between tokens by checking span byte gaps: if token
                    // N+1 starts after token N ends, the lexer skipped whitespace — insert a space.
                    _ if is_text_token(&self.peek_kind()) => {
                        let mut text = String::new();
                        let mut prev_end: Option<usize> = None;
                        while is_text_token(&self.peek_kind()) {
                            let t = self.advance();
                            if let Some(prev) = prev_end {
                                if t.span.start > prev {
                                    text.push(' ');
                                }
                            }
                            text.push_str(&token_text(&t.kind));
                            prev_end = Some(t.span.end);
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

    /// Check if the token at `pos` looks like the start of an attribute
    /// (identifier followed by '=' or ':'), rather than another class name.
    fn peek_is_attr_start(&self) -> bool {
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
    fn peek_is_boolean_attr_keyword(&self) -> bool {
        let name: Option<String> = match self.peek_kind() {
            TokenKind::Ident(s) => Some(s.clone()),
            ref kind => keyword_as_ident_name(kind),
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
    fn parse_element_prop(&mut self) -> Result<ElementProp, ParseError> {
        let mut name = self.expect_ident("expected prop name")?;

        // Handle hyphenated attribute names: margin-left, background-color, etc.
        while self.peek_kind() == TokenKind::Minus {
            self.advance(); // consume '-'
            let next = self.expect_ident("expected identifier after '-'")?;
            name = format!("{}-{}", name, next);
        }

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
                            // Should not reach here — outer match confirmed StringLiteral
                            Err(ParseError {
                                message: "expected string literal after '='".into(),
                                span: t.span.clone(),
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
                    let mut states = Vec::new();
                    while self.peek_kind() != TokenKind::RBrace
                        && self.peek_kind() != TokenKind::Eof
                    {
                        let state_name = self.expect_ident("expected state name")?;
                        self.expect_token(TokenKind::Colon, "expected ':' after state name")?;
                        let props = self.parse_property_block()?;
                        let over = if self.peek_ident_value().map_or(false, |s| s == "over") {
                            self.advance();
                            self.expect_token(TokenKind::Colon, "expected ':'")?;
                            Some(self.parse_expr()?)
                        } else {
                            None
                        };
                        let ease = if self.peek_ident_value().map_or(false, |s| s == "ease") {
                            self.advance();
                            self.expect_token(TokenKind::Colon, "expected ':'")?;
                            Some(self.parse_easing()?)
                        } else {
                            None
                        };
                        states.push((
                            state_name.clone(),
                            StateDef {
                                name: state_name,
                                properties: props,
                                over,
                                ease,
                            },
                        ));
                        if self.peek_kind() == TokenKind::Comma {
                            self.advance();
                        }
                    }
                    self.expect_token(TokenKind::RBrace, "expected '}'")?;
                    return Ok(ElementProp::StatesMap(states));
                }
                // on: { signal }
                if name == "on" {
                    self.advance(); // consume ':'
                    self.expect_token(TokenKind::LBrace, "expected '{' after 'on:'")?;
                    let signal = self.parse_expr()?;
                    self.expect_token(TokenKind::RBrace, "expected '}'")?;
                    return Ok(ElementProp::AnimateSignal(signal));
                }
                // style: entries or class: name
                self.advance();
                match self.peek_kind() {
                    TokenKind::LBrace if name == "class" => {
                        // class: { key: expr, ... } — dynamic class map
                        self.advance(); // consume '{'
                        let mut entries = Vec::new();
                        if self.peek_kind() != TokenKind::RBrace {
                            let key = self.expect_ident("expected class key")?;
                            self.expect_token(TokenKind::Colon, "expected ':'")?;
                            let val = self.parse_expr()?;
                            entries.push((key, val));
                            while self.peek_kind() == TokenKind::Comma {
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
                        let first = match &t.kind {
                            TokenKind::StringLiteral(s) => s.clone(),
                            TokenKind::Ident(s) => {
                                let mut class_name = s.clone();
                                // Handle hyphenated class names: theme-bar, btn-add, demo1-r-0, etc.
                                // Also accept IntLiteral after '-' (CSS allows numbers after hyphens: -0, -1, etc.)
                                while self.peek_kind() == TokenKind::Minus {
                                    self.advance(); // consume '-'
                                    let next: String = match self.peek_kind() {
                                        TokenKind::Ident(_) => self.expect_ident("")?,
                                        TokenKind::IntLiteral(n) => {
                                            self.advance();
                                            n.to_string()
                                        }
                                        ref k => {
                                            // Accept keywords (view, style, theme, etc.) after '-'
                                            if let Some(name) = keyword_as_ident_name(k) {
                                                self.advance();
                                                name
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
                                    let mut class_name = name;
                                    while self.peek_kind() == TokenKind::Minus {
                                        self.advance(); // consume '-'
                                        let next: String = match self.peek_kind() {
                                            TokenKind::Ident(_) => self.expect_ident("")?,
                                            TokenKind::IntLiteral(n) => {
                                                self.advance();
                                                n.to_string()
                                            }
                                            ref k => {
                                                if let Some(name) = keyword_as_ident_name(k) {
                                                    self.advance();
                                                    name
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
                        // Collect additional space-separated class names: class: btn btn-plus
                        // BUT stop if the next identifier is a known boolean HTML attribute
                        // (e.g. <audio class: media controls> — "controls" is an attribute)
                        let mut classes = vec![first];
                        loop {
                            // Peek — is the next token another class name?
                            let next_is_class = match self.peek_kind() {
                                TokenKind::Ident(_) => true,
                                ref kind => keyword_as_ident_name(kind).is_some(),
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
                                    while self.peek_kind() == TokenKind::Minus {
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
                                        let mut cn = name;
                                        while self.peek_kind() == TokenKind::Minus {
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
    fn parse_bind_prop(&mut self) -> Result<ElementProp, ParseError> {
        self.advance(); // 'bind'
        self.expect_token(TokenKind::Colon, "expected ':'")?;
        let name = self.expect_ident("expected bind property name")?;

        // Parse ={signal_path}
        let signal_path = if self.peek_kind() == TokenKind::Eq {
            self.advance(); // '='
            match self.peek_kind() {
                TokenKind::LBrace => {
                    self.advance();
                    let path = self.expect_ident("expected signal path")?;
                    // Handle nested paths like "store.field"
                    let path = if self.peek_kind() == TokenKind::Dot {
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
            _ => {
                return Err(ParseError {
                    message: format!("Expected @event binding, got {:?}", t.kind),
                    span: t.span,
                })
            }
        };
        let handler = self.parse_event_handler_value()?;
        Ok(ElementProp::Event {
            name: event_name,
            handler,
        })
    }

    /// Parse onEvent={handler} (no @ prefix) — event name is already known.
    fn parse_event_prop_no_at(&mut self, event_name: &str) -> Result<ElementProp, ParseError> {
        // Consume the event name ident token
        self.advance();
        let handler = self.parse_event_handler_value()?;
        Ok(ElementProp::Event {
            name: event_name.to_string(),
            handler,
        })
    }

    /// Parse the ={handler} part of an event binding.
    fn parse_event_handler_value(&mut self) -> Result<EventHandler, ParseError> {
        if self.peek_kind() != TokenKind::Eq {
            return Err(ParseError {
                message: "Expected '=' after event name".into(),
                span: self.peek_span(),
            });
        }
        self.advance(); // '='
        match self.peek_kind() {
            TokenKind::LBrace => {
                self.advance();
                let handler = if self.peek_kind() == TokenKind::LParen {
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
                            let expr_span = crate::parser::expr::span_of(&expr).clone();
                            let body = Block {
                                stmts: vec![Stmt::ExprStmt {
                                    expr,
                                    span: expr_span.clone(),
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

    // ── Directives ─────────────────────────────────────

    fn parse_if_directive(&mut self) -> Result<ViewNode, ParseError> {
        self.advance(); // '@if'
        let condition = self.parse_expr()?;
        // Parse optional mode= "wait|sync" after condition, before {
        let transition_mode = if self.peek_ident_value().map_or(false, |s| s == "mode") {
            self.advance(); // consume 'mode'
            self.expect_token(TokenKind::Eq, "expected '='")?;
            Some(self.expect_string("expected string literal")?)
        } else {
            None
        };
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
            // Handle both `@else @if` and `@else if` patterns
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
            } else if self.peek_kind() == TokenKind::If {
                self.advance(); // 'if' after '@else' (without @ prefix)
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

        let if_span = self.peek_span();
        Ok(ViewNode::IfDirective(IfDirective {
            condition,
            body,
            else_ifs,
            else_body,
            transition_mode,
            span: if_span,
        }))
    }

    fn parse_for_directive(&mut self) -> Result<ViewNode, ParseError> {
        self.advance(); // '@for'
        let item_name = self.expect_ident("expected item variable name")?;
        self.expect_token(TokenKind::In, "expected 'in'")?;
        let iterable = self.parse_expr()?;

        // Optional key={expr} for identity tracking
        let key_expr = if self.peek_ident_value().as_deref() == Some("key") {
            self.advance(); // 'key'
            self.expect_token(TokenKind::Eq, "expected '='")?;
            self.expect_token(TokenKind::LBrace, "expected '{'")?;
            let expr = self.parse_expr()?;
            self.expect_token(TokenKind::RBrace, "expected '}'")?;
            Some(expr)
        } else {
            None
        };

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

        let for_span = self.peek_span();
        Ok(ViewNode::ForDirective(ForDirective {
            item_name,
            iterable,
            body,
            empty_body,
            key_expr,
            span: for_span,
        }))
    }

    // ── Style rule ─────────────────────────────────────

    /// Parse style rules from `style { .class1 { ... } .class2 { ... } }` block.
    /// Returns a list of StyleRule — one per CSS class rule.
    pub fn parse_style_rules(&mut self) -> Result<Vec<StyleRule>, ParseError> {
        self.advance(); // 'style'
        self.expect_token(TokenKind::LBrace, "expected '{' after 'style'")?;

        let mut rules = Vec::new();
        while self.peek_kind() != TokenKind::RBrace && self.peek_kind() != TokenKind::Eof {
            // Each rule: .className { prop: value; ... }
            if self.peek_kind() == TokenKind::Dot {
                self.advance(); // consume '.'
            }
            let mut class_name = self.expect_ident("expected class name")?;
            // Handle hyphenated class names: .app-shell, .btn-add, etc.
            while self.peek_kind() == TokenKind::Minus {
                self.advance(); // consume '-'
                let next = self.expect_ident("expected identifier after '-'")?;
                class_name = format!("{}-{}", class_name, next);
            }
            // Handle pseudo-classes: .className:hover, .className:focus, etc.
            while self.peek_kind() == TokenKind::Colon {
                self.advance(); // consume ':'
                let pseudo = self.expect_ident("expected pseudo-class after ':'")?;
                class_name = format!("{}:{}", class_name, pseudo);
            }
            // Skip unsupported compound selector parts (e.g., "input:checked + span")
            // until we find the opening brace
            while self.peek_kind() != TokenKind::LBrace
                && self.peek_kind() != TokenKind::RBrace
                && self.peek_kind() != TokenKind::Eof
            {
                self.advance();
            }
            self.expect_token(TokenKind::LBrace, "expected '{'")?;

            let mut properties = Vec::new();
            while self.peek_kind() != TokenKind::RBrace && self.peek_kind() != TokenKind::Eof {
                // Parse property name — supports CSS custom properties (--name)
                let mut key = if self.peek_kind() == TokenKind::Minus
                    && self
                        .tokens
                        .get(self.pos + 1)
                        .map_or(false, |t| t.kind == TokenKind::Minus)
                {
                    self.advance(); // eat first '-'
                    self.advance(); // eat second '-'
                    let mut name = String::from("--");
                    loop {
                        match self.peek_kind() {
                            TokenKind::Ident(s) => {
                                name.push_str(&s);
                                self.advance();
                            }
                            TokenKind::Minus => {
                                name.push('-');
                                self.advance();
                            }
                            _ => break,
                        }
                    }
                    name
                } else {
                    self.expect_ident("expected property name")?
                };
                // Handle hyphenated CSS property names: font-family, min-height, etc.
                while self.peek_kind() == TokenKind::Minus {
                    self.advance(); // consume '-'
                    let next = self.expect_ident("expected identifier after '-'")?;
                    key = format!("{}-{}", key, next);
                }
                self.expect_token(TokenKind::Colon, "expected ':'")?;

                let value = {
                    let t = self.advance();
                    match &t.kind {
                        TokenKind::IntLiteral(n) => {
                            let val = *n as f64;
                            try_consume_dimension(self, val)
                        }
                        TokenKind::FloatLiteral(n) => try_consume_dimension(self, *n),
                        TokenKind::StringLiteral(s) => StyleValue::String(s.clone()),
                        TokenKind::Hash => {
                            // Hex color: #fff, #333, #0f0f23, #e4e4e7
                            let mut hex = String::from("#");
                            let next_t = self.advance();
                            match &next_t.kind {
                                TokenKind::IntLiteral(n) => {
                                    hex.push_str(&n.to_string());
                                    // Check for `#0` + `f0f23` pattern
                                    if matches!(self.peek_kind(), TokenKind::Ident(_)) {
                                        let ident_t = self.advance();
                                        if let TokenKind::Ident(s) = &ident_t.kind {
                                            hex.push_str(s);
                                        }
                                    }
                                }
                                TokenKind::Ident(s) => {
                                    hex.push_str(s);
                                }
                                _ => {
                                    return Err(ParseError {
                                        message: format!(
                                            "Unexpected token after '#' in color value: {:?}",
                                            next_t.kind
                                        ),
                                        span: next_t.span,
                                    })
                                }
                            }
                            StyleValue::HexColor(hex)
                        }
                        TokenKind::Ident(s) => {
                            let mut ident = s.clone();
                            // Handle hyphenated idents: space-between, nowrap, etc.
                            while self.peek_kind() == TokenKind::Minus {
                                self.advance(); // consume '-'
                                let next = self.expect_ident("expected identifier after '-'")?;
                                ident = format!("{}-{}", ident, next);
                            }
                            // Handle CSS function calls: rotate(15deg), translateY(-2px), etc.
                            if self.peek_kind() == TokenKind::LParen {
                                ident = consume_css_function_args(self, &ident);
                            }
                            // Check for dimension: 16px, 1.5rem
                            if let Some(unit) = try_parse_dimension(&ident) {
                                StyleValue::Dimension(unit.0, unit.1)
                            } else if ident.starts_with('#') {
                                StyleValue::HexColor(ident)
                            } else {
                                StyleValue::Ident(ident)
                            }
                        }
                        _ => {
                            return Err(ParseError {
                                message: format!("Unexpected style value: {:?}", t.kind),
                                span: t.span,
                            })
                        }
                    }
                };

                // Collect additional value parts for multi-value properties
                // like `margin: 0 auto` or `border: 1 solid #333`
                let mut parts: Vec<StyleValuePart> = Vec::new();
                parts.push(style_value_to_part(&value));
                loop {
                    // Check if next token could be another value part
                    match self.peek_kind() {
                        TokenKind::Semicolon | TokenKind::RBrace => break,
                        ref k
                            if keyword_as_ident_name(k).is_some() && self.peek_is_attr_start() =>
                        {
                            break
                        }
                        _ => {}
                    }
                    // Try to parse another value part
                    let next_part = {
                        let t = self.advance();
                        match &t.kind {
                            TokenKind::IntLiteral(n) => {
                                let val = *n as f64;
                                try_consume_dimension_part(self, val)
                            }
                            TokenKind::FloatLiteral(n) => try_consume_dimension_part(self, *n),
                            TokenKind::Ident(s) => {
                                let mut ident = s.clone();
                                while self.peek_kind() == TokenKind::Minus {
                                    self.advance();
                                    let next =
                                        self.expect_ident("expected identifier after '-'")?;
                                    ident = format!("{}-{}", ident, next);
                                }
                                if self.peek_kind() == TokenKind::LParen {
                                    ident = consume_css_function_args(self, &ident);
                                }
                                if let Some(unit) = try_parse_dimension(&ident) {
                                    StyleValuePart::Dimension(unit.0, unit.1)
                                } else if ident.starts_with('#') {
                                    StyleValuePart::HexColor(ident)
                                } else {
                                    StyleValuePart::Ident(ident)
                                }
                            }
                            TokenKind::Hash => {
                                let mut hex = String::from("#");
                                let next_t = self.advance();
                                match &next_t.kind {
                                    TokenKind::IntLiteral(n) => {
                                        hex.push_str(&n.to_string());
                                        if matches!(self.peek_kind(), TokenKind::Ident(_)) {
                                            let ident_t = self.advance();
                                            if let TokenKind::Ident(s) = &ident_t.kind {
                                                hex.push_str(s);
                                            }
                                        }
                                    }
                                    TokenKind::Ident(s) => {
                                        hex.push_str(s);
                                    }
                                    _ => {
                                        break;
                                    }
                                }
                                StyleValuePart::HexColor(hex)
                            }
                            _ => {
                                break;
                            }
                        }
                    };
                    parts.push(next_part);
                }
                let value = if parts.len() == 1 {
                    value
                } else {
                    StyleValue::List(parts)
                };

                properties.push((key, value));
                if self.peek_kind() == TokenKind::Semicolon {
                    self.advance();
                }
            }

            self.expect_token(TokenKind::RBrace, "expected '}'")?;
            rules.push(StyleRule {
                class_name,
                properties,
            });
        }

        self.expect_token(TokenKind::RBrace, "expected '}'")?;
        Ok(rules)
    }

    /// Parse a single style rule (legacy syntax): `style .className { ... }`
    pub fn parse_style_rule(&mut self) -> Result<StyleRule, ParseError> {
        self.advance(); // 'style'
        if self.peek_kind() == TokenKind::Dot {
            self.advance(); // consume '.'
        }
        let class_name = self.expect_ident("expected class name")?;
        self.expect_token(TokenKind::LBrace, "expected '{'")?;

        let mut properties = Vec::new();
        while self.peek_kind() != TokenKind::RBrace && self.peek_kind() != TokenKind::Eof {
            let mut key = self.expect_ident("expected property name")?;
            // Handle hyphenated CSS property names: font-family, min-height, etc.
            while self.peek_kind() == TokenKind::Minus {
                self.advance(); // consume '-'
                let next = self.expect_ident("expected identifier after '-'")?;
                key = format!("{}-{}", key, next);
            }
            self.expect_token(TokenKind::Colon, "expected ':'")?;

            let value = {
                let t = self.advance();
                match &t.kind {
                    TokenKind::IntLiteral(n) => {
                        let val = *n as f64;
                        try_consume_dimension(self, val)
                    }
                    TokenKind::FloatLiteral(n) => try_consume_dimension(self, *n),
                    TokenKind::StringLiteral(s) => StyleValue::String(s.clone()),
                    TokenKind::Hash => {
                        let mut hex = String::from("#");
                        let next_t = self.advance();
                        match &next_t.kind {
                            TokenKind::IntLiteral(n) => {
                                hex.push_str(&n.to_string());
                                if matches!(self.peek_kind(), TokenKind::Ident(_)) {
                                    let ident_t = self.advance();
                                    if let TokenKind::Ident(s) = &ident_t.kind {
                                        hex.push_str(s);
                                    }
                                }
                            }
                            TokenKind::FloatLiteral(n) => {
                                hex.push_str(&n.to_string());
                                if matches!(self.peek_kind(), TokenKind::Ident(_)) {
                                    let ident_t = self.advance();
                                    if let TokenKind::Ident(s) = &ident_t.kind {
                                        hex.push_str(s);
                                    }
                                }
                            }
                            TokenKind::Ident(s) => {
                                hex.push_str(s);
                            }
                            _ => {
                                return Err(ParseError {
                                    message: format!(
                                        "Unexpected token after '#' in color value: {:?}",
                                        next_t.kind
                                    ),
                                    span: next_t.span,
                                })
                            }
                        }
                        StyleValue::HexColor(hex)
                    }
                    TokenKind::Ident(s) => {
                        let mut ident = s.clone();
                        // Handle hyphenated idents: space-between, nowrap, etc.
                        while self.peek_kind() == TokenKind::Minus {
                            self.advance(); // consume '-'
                            let next = self.expect_ident("expected identifier after '-'")?;
                            ident = format!("{}-{}", ident, next);
                        }
                        // Handle CSS function calls: rotate(15deg), translateY(-2px), etc.
                        if self.peek_kind() == TokenKind::LParen {
                            ident = consume_css_function_args(self, &ident);
                        }
                        // Check for dimension: 16px, 1.5rem
                        if let Some(unit) = try_parse_dimension(&ident) {
                            StyleValue::Dimension(unit.0, unit.1)
                        } else if ident.starts_with('#') {
                            StyleValue::HexColor(ident)
                        } else {
                            StyleValue::Ident(ident)
                        }
                    }
                    _ => {
                        return Err(ParseError {
                            message: format!("Unexpected style value: {:?}", t.kind),
                            span: t.span,
                        })
                    }
                }
            };

            properties.push((key, value));
            if self.peek_kind() == TokenKind::Semicolon {
                self.advance();
            }
        }

        self.expect_token(TokenKind::RBrace, "expected '}'")?;
        Ok(StyleRule {
            class_name,
            properties,
        })
    }

    // ── Motion Trigger Parsing ──────────────────────────────

    /// Parse a motion trigger: consumes `={motionExpr}` after the trigger keyword.
    fn parse_motion_trigger(&mut self, kind: TriggerKind) -> Result<MotionTrigger, ParseError> {
        self.advance(); // consume the @enter/@hover/etc. token
        self.expect_token(TokenKind::Eq, "expected '=' after motion trigger")?;
        self.expect_token(TokenKind::LBrace, "expected '{'")?;
        let motion_expr = self.parse_motion_expr()?;
        self.expect_token(TokenKind::RBrace, "expected '}'")?;
        Ok(MotionTrigger {
            trigger: kind,
            motion_expr,
        })
    }

    /// Parse a property block: { key: expr, ... }
    fn parse_property_block(&mut self) -> Result<Vec<(String, Expr)>, ParseError> {
        self.expect_token(TokenKind::LBrace, "expected '{' for property block")?;
        let mut properties = Vec::new();
        while self.peek_kind() != TokenKind::RBrace && self.peek_kind() != TokenKind::Eof {
            let key = self.expect_ident("expected property name")?;
            self.expect_token(TokenKind::Colon, "expected ':' after property name")?;
            let value = self.parse_expr()?;
            properties.push((key, value));
            if self.peek_kind() == TokenKind::Comma {
                self.advance();
            }
        }
        self.expect_token(TokenKind::RBrace, "expected '}' at end of property block")?;
        Ok(properties)
    }
}

/// Check if a token kind is a keyword that can be used as an HTML attribute name.
fn is_keyword_attr_name(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Type
            | TokenKind::Style
            | TokenKind::Ref
            | TokenKind::For
            | TokenKind::If
            | TokenKind::Class
    )
}

/// Check if an identifier looks like an event handler name (onPress, onClick, etc.)
fn is_event_like(name: &str) -> bool {
    if name.len() < 4 || !name.starts_with("on") {
        return false;
    }
    // Check third char is uppercase: onPress, onClick, onChange, etc.
    name.chars().nth(2).is_some_and(|c| c.is_uppercase())
}

/// Consume CSS function arguments inside parentheses.
/// Given `rotate` and next token `(`, consumes `(15deg)` and returns `rotate(15deg)`.
fn consume_css_function_args(parser: &mut Parser, func_name: &str) -> String {
    parser.advance(); // consume '('
    let mut depth: i32 = 1;
    let mut args = String::new();
    while depth > 0 && parser.peek_kind() != TokenKind::Eof {
        match parser.peek_kind() {
            TokenKind::LParen => {
                depth += 1;
                args.push('(');
                parser.advance();
            }
            TokenKind::RParen => {
                depth -= 1;
                if depth > 0 {
                    args.push(')');
                }
                parser.advance();
            }
            TokenKind::Semicolon | TokenKind::RBrace => break,
            _ => {
                let t = parser.advance();
                args.push_str(&token_text(&t.kind));
            }
        }
    }
    format!("{}({})", func_name, args)
}

/// If the next token is an Ident that forms a valid CSS dimension with `val`,
/// consume it and return StyleValue::Dimension. Otherwise return StyleValue::Number.
fn try_consume_dimension(parser: &mut Parser, val: f64) -> StyleValue {
    // Handle ident-based units: px, rem, fr, etc.
    if let TokenKind::Ident(ref next_ident) = parser.peek_kind() {
        let combined = format!("{}{}", val, next_ident);
        if try_parse_dimension(&combined).is_some() {
            parser.advance(); // consume the unit ident
            return StyleValue::Dimension(val, next_ident.clone());
        }
    }
    // Handle % symbol (tokenized as Percent, not Ident)
    if parser.peek_kind() == TokenKind::Percent {
        parser.advance();
        return StyleValue::Dimension(val, "%".to_string());
    }
    StyleValue::Number(val)
}

/// Same as try_consume_dimension but returns StyleValuePart.
fn try_consume_dimension_part(parser: &mut Parser, val: f64) -> StyleValuePart {
    // Handle ident-based units: px, rem, fr, etc.
    if let TokenKind::Ident(ref next_ident) = parser.peek_kind() {
        let combined = format!("{}{}", val, next_ident);
        if try_parse_dimension(&combined).is_some() {
            parser.advance();
            return StyleValuePart::Dimension(val, next_ident.clone());
        }
    }
    // Handle % symbol (tokenized as Percent, not Ident)
    if parser.peek_kind() == TokenKind::Percent {
        parser.advance();
        return StyleValuePart::Dimension(val, "%".to_string());
    }
    StyleValuePart::Number(val)
}

/// Try to parse a string like "16px" into (16.0, "px").
fn try_parse_dimension(s: &str) -> Option<(f64, String)> {
    let units = [
        "px", "rem", "em", "vh", "vw", "%", "pt", "cm", "mm", // CSS Grid
        "fr", // Time
        "ms", "s", // Angle
        "deg", "rad", "grad", "turn", // Viewport-relative (additional)
        "vmin", "vmax", "svh", "svw", "lvh", "lvw", "dvh", "dvw", // Container queries
        "cqw", "cqh", "cqi", "cqb", "cqmin", "cqmax", // Typography
        "ch", "ex", "ic", "lh", "rlh",
    ];
    for unit in &units {
        if let Some(num_str) = s.strip_suffix(unit) {
            if let Ok(n) = num_str.parse::<f64>() {
                return Some((n, unit.to_string()));
            }
        }
    }
    None
}

/// Check if a token kind represents text content between view elements.
/// Any token that can produce a meaningful string representation is valid text.
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
            | TokenKind::Pipe
            | TokenKind::Dollar
            | TokenKind::Hash
            | TokenKind::StringLiteral(_)
            | TokenKind::Percent
            // Assignment and comparison operators
            | TokenKind::Eq
            | TokenKind::PlusEq | TokenKind::MinusEq
            | TokenKind::StarEq | TokenKind::SlashEq | TokenKind::PercentEq
            | TokenKind::EqEq | TokenKind::NotEq
            | TokenKind::LtEq | TokenKind::GtEq
            | TokenKind::AndAnd | TokenKind::OrOr
            | TokenKind::Arrow | TokenKind::FatArrow
            | TokenKind::DotDotDot | TokenKind::QuestionDot
            | TokenKind::At
            | TokenKind::LBracket | TokenKind::RBracket
            | TokenKind::TemplateString(_)
    ) || is_keyword_attr_name(kind)
        || keyword_as_ident_name(kind).is_some()
}

/// Convert a text token to its string representation.
fn token_text(kind: &TokenKind) -> String {
    use crate::lexer::TokenKind;
    match kind {
        TokenKind::Ident(s) => s.clone(),
        TokenKind::IntLiteral(n) => n.to_string(),
        TokenKind::FloatLiteral(f) => {
            // f64::to_string drops ".0" from integer-valued floats (e.g. 1.0 → "1").
            // This breaks version-like strings (1.0.0) rendered as text in views.
            let s = f.to_string();
            if !s.contains('.') {
                format!("{}.0", s)
            } else {
                s
            }
        }
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
        TokenKind::StringLiteral(s) => format!("\"{}\"", s),
        TokenKind::Pipe => "|".into(),
        TokenKind::Hash => "#".into(),
        TokenKind::Dollar => "$".into(),
        TokenKind::Percent => "%".into(),
        TokenKind::Eq => "=".into(),
        TokenKind::PlusEq => "+=".into(),
        TokenKind::MinusEq => "-=".into(),
        TokenKind::StarEq => "*=".into(),
        TokenKind::SlashEq => "/=".into(),
        TokenKind::PercentEq => "%=".into(),
        TokenKind::EqEq => "==".into(),
        TokenKind::NotEq => "!=".into(),
        TokenKind::LtEq => "<=".into(),
        TokenKind::GtEq => ">=".into(),
        TokenKind::AndAnd => "&&".into(),
        TokenKind::OrOr => "||".into(),
        TokenKind::Arrow => "->".into(),
        TokenKind::FatArrow => "=>".into(),
        TokenKind::DotDotDot => "...".into(),
        TokenKind::QuestionDot => "?.".into(),
        TokenKind::At => "@".into(),
        TokenKind::LBracket => "[".into(),
        TokenKind::RBracket => "]".into(),
        TokenKind::TemplateString(s) => format!("`{}`", s),
        // Fallback: try keyword_as_ident_name for keyword tokens
        other => crate::parser::stmt::keyword_as_ident_name(other).unwrap_or_default(),
    }
}
