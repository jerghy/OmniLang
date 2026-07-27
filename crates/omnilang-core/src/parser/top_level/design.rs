use crate::error::ParseError;
use crate::lexer::Span;
use crate::lexer::TokenKind;
use crate::parser::ast::*;
use crate::parser::Parser;

impl<'a> Parser<'a> {
    // ── Design ────────────────────────────────────────────

    /// Parse a `design Name { ... }` block at the top level.
    ///
    /// Consumes the `design` keyword, the name, and the braced body,
    /// delegating the body content to the string-based design parser.
    pub fn parse_design_def(&mut self) -> Result<DesignDef, ParseError> {
        let start = self.peek_span();
        self.advance(); // 'design'
        let name = self.expect_ident("expected design block name")?;

        // Consume the opening '{'
        let lbrace_span = self.peek().span; // capture span BEFORE advance replaces with placeholder
        self.expect_token(TokenKind::LBrace, "expected '{'")?;

        // Scan forward to find the matching '}' (tracking brace depth)
        let mut depth: usize = 1;
        let mut rbrace_pos = None;
        for i in self.pos..self.tokens.len() {
            match &self.tokens[i].kind {
                TokenKind::LBrace => depth += 1,
                TokenKind::RBrace => {
                    depth -= 1;
                    if depth == 0 {
                        rbrace_pos = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }

        let rbrace_pos = rbrace_pos.ok_or_else(|| ParseError {
            message: "unterminated design block — expected '}'".to_string(),
            span: start,
        })?;

        let rbrace = &self.tokens[rbrace_pos];

        // Extract the inner text between '{' and '}'
        let inner_text = &self.source[lbrace_span.end..rbrace.span.start];

        // Delegate to the string-based design body parser
        let mut def =
            crate::parser::design::parse_design_body(inner_text).map_err(|e| ParseError {
                message: e.message,
                span: Span {
                    start: start.start + e.offset,
                    end: start.start + e.offset,
                    ..start
                },
            })?;
        def.name = name;

        // Advance the parser position past the closing '}'
        self.pos = rbrace_pos + 1;

        Ok(def)
    }
}
