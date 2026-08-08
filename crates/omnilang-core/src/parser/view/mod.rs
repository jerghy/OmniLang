//! View block and element parser for OmniView components.

use crate::error::ParseError;
use crate::lexer::{Span, TokenKind};
use crate::parser::ast::*;
use crate::parser::stmt::keyword_as_ident_name;
use crate::parser::Parser;

mod directives;
mod element;
mod style;

use self::style::{is_event_like, is_keyword_attr_name, is_text_token, token_text};

impl<'a> Parser<'a> {
    /// Parse a view block: view { <Element>* }
    pub fn parse_view_block(&mut self) -> Result<ViewBlock, ParseError> {
        self.advance(); // 'view'
        self.expect_token(TokenKind::LBrace, "expected '{'")?;

        let mut children = Vec::new();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
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
            TokenKind::AtLoad => {
                // Try to parse @load as a real directive; fall back to text on
                // failure. This handles @load in text content (e.g. a heading
                // mentioning "@load"), mirroring the @if/@for fallback.
                let saved_pos = self.pos;
                let saved_depth = self.depth;
                match self.parse_load_directive() {
                    Ok(node) => Ok(node),
                    Err(_) => {
                        self.pos = saved_pos;
                        self.depth = saved_depth;
                        Ok(self.collect_at_text("@load"))
                    }
                }
            }
            TokenKind::AtSlot => {
                let slot = self.parse_slot_directive_common(false)?;
                Ok(ViewNode::SlotDirective {
                    name: slot.name,
                    default_content: slot.default_content,
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
                    let mut sub_parser = Parser::new(sub_tokens, inner);
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
                if self.at(&TokenKind::RBrace) {
                    self.advance(); // consume '}'
                    Ok(ViewNode::Text(String::new())) // Empty braces → empty text
                } else {
                    // Try to parse as interpolation expression.
                    // Fall back to raw text if expression is not immediately closed by '}'.
                    let saved_pos = self.pos;
                    let saved_depth = self.depth;
                    let parse_result = self.parse_expr();
                    match parse_result {
                        Ok(expr) if self.at(&TokenKind::RBrace) => {
                            self.expect_token(TokenKind::RBrace, "expected '}'")?;
                            Ok(ViewNode::Interpolation(expr))
                        }
                        _ => {
                            // Not a valid/complete interpolation — treat as literal text
                            self.pos = saved_pos;
                            self.depth = saved_depth;
                            let mut text = String::from("{");
                            while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
                                match self.peek_kind().clone() {
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
                                        if let Some(name) =
                                            crate::parser::stmt::keyword_as_ident_name(
                                                self.peek_kind(),
                                            )
                                        {
                                            text.push_str(name);
                                        }
                                        self.advance();
                                    }
                                }
                            }
                            if self.at(&TokenKind::RBrace) {
                                text.push('}');
                                self.advance();
                            }
                            Ok(ViewNode::Text(text))
                        }
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
        let start_span = self.peek().span;
        let mut end_span = start_span;

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
                | TokenKind::AtLoad
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

    /// Parse the shared body of a `@slot` directive: the optional name
    /// (`@slot name`, or `@slot default` / bare for the default slot) and
    /// optional `{ fallback }` view content. Used by both view-level
    /// directives (`ViewNode::SlotDirective`) and component body
    /// declarations (`SlotDecl`), ADR-053 D5.
    ///
    /// v2 (design D4): the legacy `@slot(name)` directive form and the
    /// `@slot name: Type` type constraint are deleted — both hard-error
    /// with a migration hint (spec 域 4 diagnostic table).
    ///
    /// `in_body` selects the component-body parsing context: there, the
    /// `view`/`style` keywords after a bare `@slot` start the component's
    /// view/style blocks (`@slot default` declarations precede
    /// `view { ... }`), so they are never consumed as slot names. Inside a
    /// view there is no such ambiguity — any keyword remains a valid slot
    /// name, matching the pre-D5 behavior.
    ///
    /// The literal name `default` is the default/unnamed slot (per the
    /// syntax spec §5.2), so it maps to `None` in both contexts.
    pub(super) fn parse_slot_directive_common(
        &mut self,
        in_body: bool,
    ) -> Result<SlotDecl, ParseError> {
        let start = self.peek_span();
        self.advance(); // '@slot'
                        // D4: `@slot(name)` directive form is deleted — the body-level
                        // declaration is `@slot name { ... }` (no parens).
        if self.at(&TokenKind::LParen) {
            return Err(ParseError::with_kind(
                crate::error::DiagnosticKind::ESlotRemoved,
                "改用 body 级声明 @slot name=... ",
                self.peek_span(),
            ));
        }
        let name = if matches!(self.peek_kind(), TokenKind::Ident(_))
            || keyword_as_ident_name(self.peek_kind())
                .is_some_and(|kw| !(in_body && (kw == "view" || kw == "style")))
        {
            let n = self.expect_ident("expected slot name")?;
            if n == "default" {
                None
            } else {
                Some(n)
            }
        } else {
            None
        };
        // D4: `@slot name: Type` type constraint + data_param are deleted —
        // slot = parent-provided content only.
        if self.at(&TokenKind::Colon) {
            return Err(ParseError::with_kind(
                crate::error::DiagnosticKind::ESlotRemoved,
                "类型约束与数据参数已删除；slot = 父传内容",
                self.peek_span(),
            ));
        }
        // Parse optional default content body: @slot name { ... }
        let default_content = if self.at(&TokenKind::LBrace) {
            self.advance(); // consume '{'
            let mut children = Vec::new();
            while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
                children.push(self.parse_view_node()?);
            }
            self.expect_token(TokenKind::RBrace, "expected '}'")?;
            Some(children)
        } else {
            None
        };
        let end = self.peek_span();
        Ok(SlotDecl {
            name,
            default_content,
            span: Span {
                start: start.start,
                end: end.end,
                line: start.line,
                col: start.col,
            },
        })
    }

    /// Check if current position starts a component call: Name(...
    /// Only PascalCase names start component calls; lowercase/camelCase is text.
    fn is_component_call_start(&self) -> bool {
        let kind = self.peek_kind();
        // Extract the identifier name, checking for PascalCase
        let name: Option<String> = match kind {
            TokenKind::Ident(s) => Some(s.clone()),
            _ => keyword_as_ident_name(kind).map(|s| s.to_string()),
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
            other => keyword_as_ident_name(other)
                .unwrap_or("unknown")
                .to_string(),
        };

        let mut props = Vec::new();

        // Parse optional props: (name: value, ...)
        if self.at(&TokenKind::LParen) {
            self.advance(); // '('
            while !self.at(&TokenKind::RParen) && !self.at(&TokenKind::Eof) {
                let prop_name = self.expect_ident("expected prop name")?;
                self.expect_token(TokenKind::Colon, "expected ':'")?;
                let prop_value = self.parse_expr()?;
                props.push(ElementProp::Static {
                    name: prop_name,
                    value: prop_value,
                });
                if self.at(&TokenKind::Comma) {
                    self.advance();
                }
            }
            self.expect_token(TokenKind::RParen, "expected ')'")?;
        }

        // Parse optional children block: { ... }
        let mut children = Vec::new();
        let self_closing = if self.at(&TokenKind::LBrace) {
            self.advance();
            while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
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
        if self.at(&TokenKind::Not) {
            self.advance(); // '!'
            self.expect_token(TokenKind::Minus, "expected '-' in HTML comment")?;
            self.expect_token(TokenKind::Minus, "expected '-' in HTML comment")?;
            // Skip until --> (tokenized as Minus + Arrow because lexer
            // merges `->` into a single Arrow token)
            loop {
                if self.at(&TokenKind::Minus)
                    && self.tokens.get(self.pos + 1).map(|t| &t.kind) == Some(&TokenKind::Arrow)
                {
                    self.advance(); // '-'
                    self.advance(); // '->' (Arrow)
                    break;
                }
                if self.at(&TokenKind::Eof) {
                    return Err(ParseError {
                        kind: None,
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
            let kind = self.peek_kind().clone();
            match kind {
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
                    // @drag/@dragEnd merge (ADR-073) — order-independent pair
                    self.push_element_prop(&mut props, prop)?;
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
                    // @drag/@dragEnd merge (ADR-073) — order-independent pair
                    self.push_element_prop(&mut props, prop)?;
                }
                // ── Motion trigger attributes @enter={...}, @hover={...} ──
                TokenKind::AtEnter => {
                    let trigger = self.parse_motion_trigger(TriggerKind::Enter)?;
                    props.push(ElementProp::MotionTrigger(trigger));
                }
                TokenKind::AtHover => {
                    let trigger = self.parse_motion_trigger(TriggerKind::Hover)?;
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
                TokenKind::AtInView => {
                    let trigger = self.parse_motion_trigger(TriggerKind::InView)?;
                    props.push(ElementProp::MotionTrigger(trigger));
                }
                TokenKind::AtScroll => {
                    let trigger = self.parse_motion_trigger(TriggerKind::Scroll {
                        start: None,
                        end: None,
                        scrub: None,
                    })?;
                    props.push(ElementProp::MotionTrigger(trigger));
                }
                TokenKind::AtDragHorizontal => {
                    let trigger = self.parse_motion_trigger(TriggerKind::DragHorizontal {
                        constraints: None,
                        elastic: None,
                        threshold: None,
                    })?;
                    props.push(ElementProp::MotionTrigger(trigger));
                }
                TokenKind::AtDragVertical => {
                    let trigger = self.parse_motion_trigger(TriggerKind::DragVertical {
                        constraints: None,
                        elastic: None,
                        threshold: None,
                    })?;
                    props.push(ElementProp::MotionTrigger(trigger));
                }
                // Form intent: bare `@form` attribute on an element (spec
                // 2026-08-08 §4.1). Marks the form container for validation/
                // error-signal/a11y wiring; a value is not allowed.
                TokenKind::AtForm => {
                    let t = self.advance();
                    if matches!(self.peek_kind(), TokenKind::Eq) {
                        return Err(ParseError {
                            kind: None,
                            message: "'@form' takes no value — use the bare attribute (spec 2026-08-08 §4.1)"
                                .into(),
                            span: t.span,
                        });
                    }
                    props.push(ElementProp::Form);
                }
                // @hero is dead syntax: hero transitions were removed from the
                // feature set in the semantic DSL (ADR-032 scoping / ADR-053
                // syntax). Report a clear diagnostic instead of the generic
                // "Unexpected token in element props" error.
                TokenKind::AtHero => {
                    let t = self.advance();
                    return Err(ParseError { kind: None,
                        message: "hero transitions are not supported — '@hero' was removed in the semantic DSL (ADR-032/053); use @enter/@hover/@press/@focus/@in-view/@scroll/@drag-* triggers instead"
                            .into(),
                        span: t.span,
                    });
                }
                // Sub-attribute tokens (@paused, @in-view-once, @scroll-start,
                // @drag-constraints, etc.) were removed per semantic-level DSL
                // spec §3.2 — they are re-parsed as regular ident-based attrs in
                // later tasks. Unknown @-prefixed tokens fall through to
                // parse_event_prop via the Ident("@...") arm above.
                _ => {
                    let t = self.advance();
                    return Err(ParseError {
                        kind: None,
                        message: format!("Unexpected token in element props: {:?}", t.kind),
                        span: t.span,
                    });
                }
            }
        }

        // v4: value={sig} 判别器——带 @ref 的元素上保持 RefBinding
        // （v1 双向形状 → EValueBindingRemoved）；无 @ref 时还原为
        // Static（普通单向 prop）。
        let has_ref = props.iter().any(|p| matches!(p, ElementProp::Ref { .. }));
        for prop in &mut props {
            match prop {
                // 带 @ref 的 value={sig}（裸 signal 或限定 store signal）
                // 是 v1 双向形状——提升为 RefBinding 交给语义层报错；
                // 计算表达式保持普通单向 Static prop。
                ElementProp::Static { name, value } if name == "value" && has_ref => {
                    let is_bare_signal = matches!(value, Expr::VarRef(..));
                    let is_qualified_store_signal = matches!(
                        value,
                        Expr::FieldAccess { object, .. }
                            if matches!(object.as_ref(), Expr::VarRef(..))
                    );
                    if is_bare_signal || is_qualified_store_signal {
                        *prop = ElementProp::RefBinding {
                            binding: name.clone(),
                            signal: value.clone(),
                        };
                    }
                }
                // 无 @ref 的 value RefBinding → 普通单向 Static prop
                // （防御分支——"value" 已离开 SEMANTIC_BINDINGS，经
                // parse_element_prop 不会再产生该形状）。
                ElementProp::RefBinding { binding, signal } if binding == "value" && !has_ref => {
                    *prop = ElementProp::Static {
                        name: binding.clone(),
                        value: signal.clone(),
                    };
                }
                _ => {}
            }
        }

        // @dragEnd without @drag — hard error (ADR-073). Detected here, at
        // the end of the prop loop, because the pair is order-independent:
        // an @dragEnd-first placeholder is only illegal once no @drag showed
        // up at all.
        if props
            .iter()
            .any(|p| matches!(p, ElementProp::Drag { on_drag, .. } if on_drag.is_empty()))
        {
            return Err(ParseError {
                kind: None,
                message: "@dragEnd requires @drag on the same element".into(),
                span: start,
            });
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
                match self.peek_kind().clone() {
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
                    TokenKind::AtIf | TokenKind::AtFor | TokenKind::AtSlot | TokenKind::AtLoad => {
                        children.push(self.parse_view_node()?);
                    }
                    TokenKind::StringLiteral(_) | TokenKind::LBrace => {
                        children.push(self.parse_view_node()?);
                    }
                    // Collect consecutive text tokens (Idents, colons, etc.) into one Text node.
                    // Reconstruct whitespace between tokens by checking span byte gaps: if token
                    // N+1 starts after token N ends, the lexer skipped whitespace — insert a space.
                    _ if is_text_token(self.peek_kind()) => {
                        let mut text = String::new();
                        let mut prev_end: Option<usize> = None;
                        while is_text_token(self.peek_kind()) {
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
}
