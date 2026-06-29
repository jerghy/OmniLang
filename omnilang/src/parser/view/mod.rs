//! View block and element parser for OmniView components.

use crate::error::ParseError;
use crate::lexer::{Span, TokenKind};
use crate::parser::ast::*;
use crate::parser::stmt::keyword_as_ident_name;
use crate::parser::Parser;

mod directives;
mod style;

use self::style::{is_event_like, is_keyword_attr_name, is_text_token, token_text};

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
            TokenKind::AtIf => {
                // Try to parse @if as a real directive; fall back to text on failure
                let saved_pos = self.pos;
                let saved_depth = self.depth;
                match self.parse_if_directive() {
                    Ok(node) => Ok(node),
                    Err(_) => {
                        self.pos = saved_pos;
                        self.depth = saved_depth;
                        Ok(self.collect_at_text("@if"))
                    }
                }
            }
            TokenKind::AtFor => {
                // Try to parse @for as a real directive; fall back to text on failure.
                // This handles @for in text content (e.g. "@for key={expr} Demo" in a heading).
                let saved_pos = self.pos;
                let saved_depth = self.depth;
                match self.parse_for_directive() {
                    Ok(node) => Ok(node),
                    Err(_) => {
                        self.pos = saved_pos;
                        self.depth = saved_depth;
                        Ok(self.collect_at_text("@for"))
                    }
                }
            }
            TokenKind::AtSlot => {
                self.advance();
                let name = if self.peek_kind() == TokenKind::LParen {
                    self.advance();
                    let n = self.expect_ident("expected slot name")?;
                    self.expect_token(TokenKind::RParen, "expected ')'")?;
                    Some(n)
                } else if matches!(self.peek_kind(), TokenKind::Ident(_))
                    || keyword_as_ident_name(&self.peek_kind()).is_some()
                {
                    let n = self.expect_ident("expected slot name")?;
                    Some(n)
                } else {
                    None
                };
                // Parse optional default content body: @slot name { ... }
                let default_content = if self.peek_kind() == TokenKind::LBrace {
                    self.advance(); // consume '{'
                    let mut children = Vec::new();
                    while self.peek_kind() != TokenKind::RBrace
                        && self.peek_kind() != TokenKind::Eof
                    {
                        children.push(self.parse_view_node()?);
                    }
                    self.expect_token(TokenKind::RBrace, "expected '}'")?;
                    Some(children)
                } else {
                    None
                };
                Ok(ViewNode::SlotDirective {
                    name,
                    default_content,
                })
            }
            TokenKind::StringLiteral(s) => {
                let text = s.clone();
                self.advance();
                // Detect interpolation inside string literal: "{expr}"
                // The lexer's read_string() consumes everything between " and ",
                // including {expr} markers. Check if the content starts with { and
                // ends with }, then parse it as an interpolation expression.
                if text.starts_with('{') && text.ends_with('}') && text.len() > 2 {
                    let inner = &text[1..text.len() - 1];
                    // Create a temporary lexer + parser for the expression
                    use crate::lexer::tokenize;
                    let sub_tokens = tokenize(inner);
                    let mut sub_parser = Parser::new(&sub_tokens, inner);
                    match sub_parser.parse_expr() {
                        Ok(expr) => Ok(ViewNode::Interpolation(expr)),
                        Err(_) => Ok(ViewNode::Text(text)),
                    }
                } else {
                    Ok(ViewNode::Text(text))
                }
            }
            TokenKind::LBrace => {
                // Interpolation: { expr } or empty braces {} (JSX comments stripped)
                self.advance();
                if self.peek_kind() == TokenKind::RBrace {
                    self.advance(); // consume '}'
                    Ok(ViewNode::Text(String::new())) // Empty braces → empty text
                } else {
                    // Try to parse as interpolation expression.
                    // Fall back to raw text if expression is not immediately closed by '}'.
                    let saved_pos = self.pos;
                    let saved_depth = self.depth;
                    let parse_result = self.parse_expr();
                    let is_valid_interpolation =
                        parse_result.is_ok() && self.peek_kind() == TokenKind::RBrace;
                    if is_valid_interpolation {
                        self.expect_token(TokenKind::RBrace, "expected '}'")?;
                        Ok(ViewNode::Interpolation(
                            parse_result.expect("parse_result is Ok as verified above"),
                        ))
                    } else {
                        // Not a valid/complete interpolation — treat as literal text
                        self.pos = saved_pos;
                        self.depth = saved_depth;
                        let mut text = String::from("{");
                        while self.peek_kind() != TokenKind::RBrace
                            && self.peek_kind() != TokenKind::Eof
                        {
                            match self.peek_kind() {
                                TokenKind::StringLiteral(s) => {
                                    text.push_str(&s);
                                    self.advance();
                                }
                                TokenKind::IntLiteral(n) => {
                                    text.push_str(&n.to_string());
                                    self.advance();
                                }
                                TokenKind::FloatLiteral(n) => {
                                    text.push_str(&n.to_string());
                                    self.advance();
                                }
                                TokenKind::Semicolon => {
                                    text.push(';');
                                    self.advance();
                                }
                                TokenKind::Comma => {
                                    text.push(',');
                                    self.advance();
                                }
                                TokenKind::Dot => {
                                    text.push('.');
                                    self.advance();
                                }
                                TokenKind::LParen => {
                                    text.push('(');
                                    self.advance();
                                }
                                TokenKind::RParen => {
                                    text.push(')');
                                    self.advance();
                                }
                                TokenKind::LBrace => {
                                    text.push('{');
                                    self.advance();
                                }
                                TokenKind::Eq => {
                                    text.push('=');
                                    self.advance();
                                }
                                TokenKind::Colon => {
                                    text.push(':');
                                    self.advance();
                                }
                                TokenKind::Plus => {
                                    text.push('+');
                                    self.advance();
                                }
                                TokenKind::Minus => {
                                    text.push('-');
                                    self.advance();
                                }
                                TokenKind::Star => {
                                    text.push('*');
                                    self.advance();
                                }
                                TokenKind::Slash => {
                                    text.push('/');
                                    self.advance();
                                }
                                _ => {
                                    if let Some(name) = crate::parser::stmt::keyword_as_ident_name(
                                        &self.peek_kind(),
                                    ) {
                                        text.push_str(&name);
                                    }
                                    self.advance();
                                }
                            }
                        }
                        if self.peek_kind() == TokenKind::RBrace {
                            text.push('}');
                            self.advance();
                        }
                        Ok(ViewNode::Text(text))
                    }
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
        // Use char-safe boundaries for non-ASCII content.
        let text = if start_span.start < self.source.len()
            && end_span.end <= self.source.len()
            && start_span.start <= end_span.end
        {
            let start = self
                .source
                .char_indices()
                .find(|&(i, _)| i >= start_span.start)
                .map(|(i, _)| i)
                .unwrap_or(start_span.start);
            let end = self.source[start..]
                .char_indices()
                .find(|&(i, _)| i + start >= end_span.end)
                .map(|(i, c)| i + start + c.len_utf8())
                .unwrap_or(end_span.end);
            self.source[start..end].to_string()
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
                    let expr = self.parse_expr()?;
                    self.expect_token(TokenKind::RBrace, "expected '}'")?;
                    props.push(ElementProp::PausedOverride(expr));
                }
                TokenKind::AtHoverReverse => {
                    self.advance();
                    self.expect_token(TokenKind::Eq, "expected '='")?;
                    let mode = if self.peek_kind() == TokenKind::LBrace {
                        self.advance();
                        let m = self.expect_ident("expected hover reverse mode")?;
                        self.expect_token(TokenKind::RBrace, "expected '}'")?;
                        m
                    } else {
                        self.expect_string("expected string value")?
                    };
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

        // Auto-close void HTML elements (input, br, hr, img, etc.)
        let void_tags = [
            "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param",
            "source", "track", "wbr",
        ];
        if !self_closing && void_tags.contains(&tag.as_str()) {
            self_closing = true;
            // If source has <input></input> with explicit closing tag,
            // consume the orphaned </tagname> that follows.
            if self.check_closing_tag(&tag) {
                self.advance(); // '<'
                self.advance(); // '/'
                self.expect_ident("expected closing tag")?;
                self.expect_token(TokenKind::Gt, "expected '>'")?;
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
                            let key = self.parse_css_property_name("expected style key")?;
                            self.expect_token(TokenKind::Colon, "expected ':'")?;
                            let val = self.parse_expr()?;
                            entries.push((key, val));
                            while self.peek_kind() == TokenKind::Comma {
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
                        // Check for dynamic class expression: class: name { expr }
                        if self.peek_kind() == TokenKind::LBrace {
                            // Parse class: name(s) { conditional_expr }
                            // e.g., class: tab-btn {if activeDemo == "position" { "active" } else { "" }}
                            // Combine static names with dynamic expression result
                            let span = self.peek_span();
                            let static_part =
                                Expr::Literal(Literal::String(classes.join(" ")), span.clone());
                            self.advance(); // consume '{'
                            let dyn_expr = self.parse_expr()?;
                            self.expect_token(TokenKind::RBrace, "expected '}'")?;
                            let combined = Expr::BinaryOp {
                                left: Box::new(static_part),
                                op: BinaryOpKind::Add,
                                right: Box::new(dyn_expr),
                                span: span.clone(),
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
}
