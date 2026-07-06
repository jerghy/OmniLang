//! View directive parsers: @if/@for and motion triggers.

use crate::error::ParseError;
use crate::lexer::TokenKind;
use crate::parser::ast::*;
use crate::parser::Parser;

use super::style::{is_text_token, token_text};

impl<'a> Parser<'a> {
    // ── Directives ─────────────────────────────────────

    pub(super) fn parse_if_directive(&mut self) -> Result<ViewNode, ParseError> {
        self.advance(); // '@if'
        let condition = self.parse_expr()?;
        // Parse optional mode= "wait|sync" after condition, before {
        let transition_mode = if self.peek_ident_value().is_some_and(|s| s == "mode") {
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

    pub(super) fn parse_for_directive(&mut self) -> Result<ViewNode, ParseError> {
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

    // ── Motion Trigger Parsing ──────────────────────────────

    /// Parse a motion trigger: consumes `={motionExpr}` after the trigger keyword.
    pub(super) fn parse_motion_trigger(
        &mut self,
        kind: TriggerKind,
    ) -> Result<MotionTrigger, ParseError> {
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
    pub(super) fn parse_property_block(&mut self) -> Result<Vec<(String, Expr)>, ParseError> {
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

    /// Collect an @-keyword that failed to parse as a real directive into a text node.
    /// Used when @for/@if appear in element text content rather than as directives.
    pub(super) fn collect_at_text(&mut self, keyword: &str) -> ViewNode {
        let mut text = String::from(keyword);
        self.advance(); // consume the @-keyword token
        while self.pos < self.tokens.len() {
            let kind = self.peek_kind();
            match kind {
                TokenKind::Lt | TokenKind::Gt | TokenKind::Eof => break,
                TokenKind::LBrace => {
                    text.push('{');
                    self.advance();
                }
                TokenKind::RBrace => {
                    text.push('}');
                    self.advance();
                }
                _ if is_text_token(&kind) => {
                    // Insert space if there's a gap indicating whitespace
                    text.push_str(&token_text(&kind));
                    self.advance();
                }
                _ => break,
            }
        }
        ViewNode::Text(text)
    }
}
