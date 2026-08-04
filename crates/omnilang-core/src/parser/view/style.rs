//! CSS style rule parser for OmniView components.

use crate::error::ParseError;
use crate::lexer::TokenKind;
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
    // ── Style rule ─────────────────────────────────────

    /// Parse a CSS property name, supporting CSS custom properties (`--name`).
    /// For `--` prefixed names, consumes the `--` prefix then reads ident parts
    /// separated by hyphens. For regular names, uses expect_ident.
    pub(crate) fn parse_css_property_name(&mut self, msg: &str) -> Result<String, ParseError> {
        if *self.peek_kind() == TokenKind::Minus
            && self
                .tokens
                .get(self.pos + 1)
                .is_some_and(|t| t.kind == TokenKind::Minus)
        {
            self.advance(); // eat first '-'
            self.advance(); // eat second '-'
            let mut name = String::from("--");
            loop {
                match self.peek_kind() {
                    TokenKind::Ident(s) => {
                        name.push_str(s);
                        self.advance();
                    }
                    TokenKind::Minus => {
                        name.push('-');
                        self.advance();
                    }
                    kind => {
                        if let Some(kw) = keyword_as_ident_name(kind) {
                            name.push_str(kw);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
            }
            Ok(name)
        } else {
            let key = self.expect_ident(msg)?;
            let mut result = key;
            // Handle hyphenated CSS property names: font-family, min-height, etc.
            while *self.peek_kind() == TokenKind::Minus {
                self.advance(); // consume '-'
                let next = self.expect_ident("expected identifier after '-'")?;
                result = format!("{}-{}", result, next);
            }
            Ok(result)
        }
    }

    /// Parse style rules from `style { .class1 { ... } .class2 { ... } }` block.
    /// Returns a list of StyleRule — one per CSS class rule.
    pub fn parse_style_rules(&mut self) -> Result<Vec<StyleRule>, ParseError> {
        self.enter_recursion()?;
        self.advance(); // 'style'
        self.expect_token(TokenKind::LBrace, "expected '{' after 'style'")?;

        let mut rules = Vec::new();
        while *self.peek_kind() != TokenKind::RBrace && *self.peek_kind() != TokenKind::Eof {
            // Each rule: .className { prop: value; ... }
            // or: tag { prop: value; ... }
            let is_tag_selector = *self.peek_kind() != TokenKind::Dot;
            if *self.peek_kind() == TokenKind::Dot {
                self.advance(); // consume '.'
            }
            let mut class_name = self.expect_ident("expected class name")?;
            // Handle hyphenated class names: .app-shell, .btn-add, etc.
            while *self.peek_kind() == TokenKind::Minus {
                self.advance(); // consume '-'
                let next = self.expect_ident("expected identifier after '-'")?;
                class_name = format!("{}-{}", class_name, next);
            }
            // Handle pseudo-classes: .className:hover, .className:focus, etc.
            while *self.peek_kind() == TokenKind::Colon {
                self.advance(); // consume ':'
                let pseudo = self.expect_ident("expected pseudo-class after ':'")?;
                class_name = format!("{}:{}", class_name, pseudo);
            }
            // Handle compound class selectors: .var-card.alt, .btn.active.dark, etc.
            // Only when the '.' directly abuts the previous ident — with
            // whitespace it starts a descendant segment instead (".a .b").
            while *self.peek_kind() == TokenKind::Dot
                && self.pos > 0
                && self.tokens[self.pos - 1].span.end == self.tokens[self.pos].span.start
            {
                self.advance(); // consume '.'
                let extra = self.expect_ident("expected class name after '.'")?;
                class_name = format!("{}.{}", class_name, extra);
                // Handle hyphenated names in compound part: .var-card.deep-dark
                while *self.peek_kind() == TokenKind::Minus {
                    self.advance(); // consume '-'
                    let next = self.expect_ident("expected identifier after '-'")?;
                    class_name = format!("{}-{}", class_name, next);
                }
                // Handle pseudo-classes on compound part: .var-card.alt:hover
                while *self.peek_kind() == TokenKind::Colon {
                    self.advance(); // consume ':'
                    let pseudo = self.expect_ident("expected pseudo-class after ':'")?;
                    class_name = format!("{}:{}", class_name, pseudo);
                }
            }
            // Descendant selectors: ".note-list li", ".note-list .item",
            // ".child-box strong". The lexer drops whitespace, so a compound
            // (".a.b") is distinguished from a descendant (".a .b") by token
            // spans: whitespace means prev.span.end < next.span.start.
            loop {
                let has_space = self.pos > 0
                    && self.tokens[self.pos - 1].span.end < self.tokens[self.pos].span.start;
                if !has_space {
                    break;
                }
                let mut segment = String::new();
                let starts_class = *self.peek_kind() == TokenKind::Dot;
                if starts_class {
                    self.advance(); // consume '.'
                    segment.push('.');
                    let ident = self.expect_ident("expected class name after '.'")?;
                    segment.push_str(&ident);
                    // Hyphenated names in the descendant segment: .note-list .app-shell
                    while *self.peek_kind() == TokenKind::Minus {
                        self.advance();
                        let next = self.expect_ident("expected identifier after '-'")?;
                        segment.push('-');
                        segment.push_str(&next);
                    }
                } else if matches!(self.peek_kind(), TokenKind::Ident(_)) {
                    // Tag segment: "li", "strong", "p", ...
                    let ident = self.expect_ident("expected tag name")?;
                    segment.push_str(&ident);
                } else {
                    break;
                }
                // Pseudo-classes on the descendant segment: li:hover
                while *self.peek_kind() == TokenKind::Colon {
                    self.advance();
                    let pseudo = self.expect_ident("expected pseudo-class after ':'")?;
                    segment.push(':');
                    segment.push_str(&pseudo);
                }
                class_name.push(' ');
                class_name.push_str(&segment);
            }
            // Skip unsupported remaining selector parts (e.g., "input:checked + span")
            // until we find the opening brace
            while *self.peek_kind() != TokenKind::LBrace
                && *self.peek_kind() != TokenKind::RBrace
                && *self.peek_kind() != TokenKind::Eof
            {
                self.advance();
            }
            self.expect_token(TokenKind::LBrace, "expected '{'")?;

            let mut properties = Vec::new();
            while *self.peek_kind() != TokenKind::RBrace && *self.peek_kind() != TokenKind::Eof {
                // Parse property name — supports CSS custom properties (--name)
                let key = self.parse_css_property_name("expected property name")?;
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
                                TokenKind::Ident(s) => {
                                    hex.push_str(s);
                                }
                                TokenKind::FloatLiteral(_) => {
                                    // Clamp to valid UTF-8 char boundaries.
                                    // The tokenizer may produce byte positions in the
                                    // middle of multi-byte characters; walk backward from
                                    // the token start and forward from the token end to
                                    // find valid char boundaries.
                                    let raw_start = (0..=next_t.span.start)
                                        .rev()
                                        .find(|&i| self.source.is_char_boundary(i))
                                        .unwrap_or(next_t.span.start);
                                    let raw_end = (next_t.span.end..self.source.len())
                                        .find(|&i| self.source.is_char_boundary(i))
                                        .unwrap_or(next_t.span.end);
                                    let raw = &self.source[raw_start..raw_end];
                                    hex.push_str(raw);
                                    if matches!(self.peek_kind(), TokenKind::Ident(_)) {
                                        let ident_t = self.advance();
                                        if let TokenKind::Ident(s) = &ident_t.kind {
                                            hex.push_str(s);
                                        }
                                    }
                                }
                                _ => {
                                    return Err(ParseError {
                                        kind: None,
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
                            while *self.peek_kind() == TokenKind::Minus {
                                self.advance(); // consume '-'
                                let next = self.expect_ident("expected identifier after '-'")?;
                                ident = format!("{}-{}", ident, next);
                            }
                            // Handle CSS function calls: rotate(15deg), translateY(-2px), etc.
                            if *self.peek_kind() == TokenKind::LParen {
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
                                kind: None,
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
                        k if keyword_as_ident_name(k).is_some() && self.peek_is_attr_start() => {
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
                                while *self.peek_kind() == TokenKind::Minus {
                                    self.advance();
                                    let next =
                                        self.expect_ident("expected identifier after '-'")?;
                                    ident = format!("{}-{}", ident, next);
                                }
                                if *self.peek_kind() == TokenKind::LParen {
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
                                    TokenKind::FloatLiteral(_) => {
                                        let raw = &self.source[next_t.span.start..next_t.span.end];
                                        hex.push_str(raw);
                                        if matches!(self.peek_kind(), TokenKind::Ident(_)) {
                                            let ident_t = self.advance();
                                            if let TokenKind::Ident(s) = &ident_t.kind {
                                                hex.push_str(s);
                                            }
                                        }
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
                if *self.peek_kind() == TokenKind::Semicolon {
                    self.advance();
                }
            }

            self.expect_token(TokenKind::RBrace, "expected '}'")?;
            rules.push(StyleRule {
                class_name,
                properties,
                is_tag_selector,
            });
        }

        self.expect_token(TokenKind::RBrace, "expected '}'")?;
        self.leave_recursion();
        Ok(rules)
    }

    /// Parse a single style rule (legacy syntax): `style .className { ... }`
    pub fn parse_style_rule(&mut self) -> Result<StyleRule, ParseError> {
        self.enter_recursion()?;
        self.advance(); // 'style'
        let is_tag_selector = *self.peek_kind() != TokenKind::Dot;
        if *self.peek_kind() == TokenKind::Dot {
            self.advance(); // consume '.'
        }
        let class_name = self.expect_ident("expected class name")?;
        self.expect_token(TokenKind::LBrace, "expected '{'")?;

        let mut properties = Vec::new();
        while *self.peek_kind() != TokenKind::RBrace && *self.peek_kind() != TokenKind::Eof {
            let mut key = self.expect_ident("expected property name")?;
            // Handle hyphenated CSS property names: font-family, min-height, etc.
            while *self.peek_kind() == TokenKind::Minus {
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
                                    kind: None,
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
                        while *self.peek_kind() == TokenKind::Minus {
                            self.advance(); // consume '-'
                            let next = self.expect_ident("expected identifier after '-'")?;
                            ident = format!("{}-{}", ident, next);
                        }
                        // Handle CSS function calls: rotate(15deg), translateY(-2px), etc.
                        if *self.peek_kind() == TokenKind::LParen {
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
                            kind: None,
                            message: format!("Unexpected style value: {:?}", t.kind),
                            span: t.span,
                        })
                    }
                }
            };

            properties.push((key, value));
            if *self.peek_kind() == TokenKind::Semicolon {
                self.advance();
            }
        }

        self.expect_token(TokenKind::RBrace, "expected '}'")?;
        self.leave_recursion();
        Ok(StyleRule {
            class_name,
            properties,
            is_tag_selector,
        })
    }
}

/// Check if a token kind is a keyword that can be used as an HTML attribute name.
pub(super) fn is_keyword_attr_name(kind: &TokenKind) -> bool {
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
pub(super) fn is_event_like(name: &str) -> bool {
    if name.len() < 4 || !name.starts_with("on") {
        return false;
    }
    // Check third char is uppercase: onPress, onClick, onChange, etc.
    name.chars().nth(2).is_some_and(|c| c.is_uppercase())
}

/// Consume CSS function arguments inside parentheses.
/// Given `rotate` and next token `(`, consumes `(15deg)` and returns `rotate(15deg)`.
pub(super) fn consume_css_function_args(parser: &mut Parser, func_name: &str) -> String {
    parser.advance(); // consume '('
    let mut depth: i32 = 1;
    let mut args = String::new();
    while depth > 0 && *parser.peek_kind() != TokenKind::Eof {
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
pub(super) fn try_consume_dimension(parser: &mut Parser, val: f64) -> StyleValue {
    // Handle ident-based units: px, rem, fr, etc.
    let kind = parser.peek_kind().clone();
    if let TokenKind::Ident(ref next_ident) = kind {
        let combined = format!("{}{}", val, next_ident);
        if try_parse_dimension(&combined).is_some() {
            parser.advance(); // consume the unit ident
            return StyleValue::Dimension(val, next_ident.clone());
        }
    }
    // Handle % symbol (tokenized as Percent, not Ident)
    if *parser.peek_kind() == TokenKind::Percent {
        parser.advance();
        return StyleValue::Dimension(val, "%".to_string());
    }
    StyleValue::Number(val)
}

/// Same as try_consume_dimension but returns StyleValuePart.
pub(super) fn try_consume_dimension_part(parser: &mut Parser, val: f64) -> StyleValuePart {
    // Handle ident-based units: px, rem, fr, etc.
    let kind = parser.peek_kind().clone();
    if let TokenKind::Ident(ref next_ident) = kind {
        let combined = format!("{}{}", val, next_ident);
        if try_parse_dimension(&combined).is_some() {
            parser.advance();
            return StyleValuePart::Dimension(val, next_ident.clone());
        }
    }
    // Handle % symbol (tokenized as Percent, not Ident)
    if *parser.peek_kind() == TokenKind::Percent {
        parser.advance();
        return StyleValuePart::Dimension(val, "%".to_string());
    }
    StyleValuePart::Number(val)
}

/// Try to parse a string like "16px" into (16.0, "px").
///
/// Returns `None` when the string does not match any known CSS unit suffix,
/// or when the numeric portion cannot be parsed as an `f64`. Callers that
/// receive `None` should fall back to treating the value as a raw CSS literal
/// (e.g. `var(--x)`, `calc(...)`, or an unrecognized custom function) rather
/// than silently skipping the style declaration.
pub(super) fn try_parse_dimension(s: &str) -> Option<(f64, String)> {
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
pub(super) fn is_text_token(kind: &TokenKind) -> bool {
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
            // @-keyword tokens that may appear in text content
            | TokenKind::AtEnter | TokenKind::AtHover
            | TokenKind::AtPress | TokenKind::AtFocus | TokenKind::AtInView
            | TokenKind::AtHero
            | TokenKind::AtScroll
            | TokenKind::AtDragHorizontal | TokenKind::AtDragVertical
            | TokenKind::AtStore | TokenKind::AtMotion
            | TokenKind::AtMount
            | TokenKind::AtUnmount | TokenKind::AtEffect
            // Directive continuation keywords are safe as literal text too:
            // "@else"/"@empty" inside element content must not truncate children
            // (directive parsing consumes them internally, before text paths).
            | TokenKind::AtElse | TokenKind::AtEmpty
            | TokenKind::IfMode
            | TokenKind::Percent
            // Assignment and comparison operators
            | TokenKind::Eq
            | TokenKind::PlusEq | TokenKind::MinusEq
            | TokenKind::StarEq | TokenKind::SlashEq | TokenKind::PercentEq
            | TokenKind::EqEq | TokenKind::NotEq
            | TokenKind::LtEq | TokenKind::GtEq
            | TokenKind::AndAnd | TokenKind::OrOr
            | TokenKind::Arrow | TokenKind::FatArrow
            | TokenKind::DotDotDot | TokenKind::QuestionDot | TokenKind::QuestionQuestion
            | TokenKind::At
            | TokenKind::LBracket | TokenKind::RBracket
            | TokenKind::Semicolon
            | TokenKind::TemplateString(_)
    ) || is_keyword_attr_name(kind)
        || keyword_as_ident_name(kind).is_some()
}

/// Convert a text token to its string representation.
pub(super) fn token_text(kind: &TokenKind) -> String {
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
        // IfMode lexes from both "if_mode" (motion-block keyword, keywords.rs)
        // and "@if-mode" (the documented view spelling, keywords.rs). View text
        // uses the @-form, so render "@if-mode"; the bare "if_mode" ident form
        // only appears in @motion expressions, which never reach token_text.
        TokenKind::IfMode => "@if-mode".into(),
        TokenKind::Semicolon => ";".into(),
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
        TokenKind::QuestionQuestion => "??".into(),
        TokenKind::At => "@".into(),
        // Literal spellings of @-keyword tokens accepted by is_text_token —
        // without these arms the fallback below would silently drop the word
        // from text content (e.g. "on @hover over" → "on  over").
        TokenKind::AtEnter => "@enter".into(),
        TokenKind::AtHover => "@hover".into(),
        TokenKind::AtPress => "@press".into(),
        TokenKind::AtFocus => "@focus".into(),
        TokenKind::AtInView => "@in-view".into(),
        TokenKind::AtScroll => "@scroll".into(),
        TokenKind::AtHero => "@hero".into(),
        TokenKind::AtDragHorizontal => "@drag-horizontal".into(),
        TokenKind::AtDragVertical => "@drag-vertical".into(),
        TokenKind::AtStore => "@store".into(),
        TokenKind::AtMotion => "@motion".into(),
        TokenKind::AtMount => "@mount".into(),
        TokenKind::AtUnmount => "@unmount".into(),
        TokenKind::AtEffect => "@effect".into(),
        TokenKind::AtElse => "@else".into(),
        TokenKind::AtEmpty => "@empty".into(),
        TokenKind::LBracket => "[".into(),
        TokenKind::RBracket => "]".into(),
        TokenKind::TemplateString(s) => format!("`{}`", s),
        // Fallback: try keyword_as_ident_name for keyword tokens
        other => crate::parser::stmt::keyword_as_ident_name(other)
            .unwrap_or_default()
            .to_string(),
    }
}
