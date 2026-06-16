//! Parser module — recursive descent + Pratt expression parsing.

pub mod ast;
pub mod expr;
pub mod stmt;
pub mod top_level;
pub mod view;

use crate::error::ParseError;
use crate::lexer::{Token, TokenKind};
use ast::{Program, Visibility};

/// Parse a token stream into an AST Program.
pub fn parse(tokens: &[Token]) -> Result<Program, ParseError> {
    let mut parser = Parser::new(tokens);
    parser.parse_program()
}

/// The recursive descent parser.
pub struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [Token]) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> Token {
        self.tokens
            .get(self.pos)
            .cloned()
            .unwrap_or_else(|| Token::new(TokenKind::Eof, 0, 0, 0, 0))
    }

    fn peek_kind(&self) -> TokenKind {
        self.tokens
            .get(self.pos)
            .map(|t| t.kind.clone())
            .unwrap_or(TokenKind::Eof)
    }

    fn advance(&mut self) -> Token {
        let t = self.peek();
        if t.kind != TokenKind::Eof {
            self.pos += 1;
        }
        t
    }

    /// Parse a complete OmniLang program.
    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut components = Vec::new();
        let mut stores = Vec::new();
        let mut themes = Vec::new();
        let mut functions = Vec::new();
        let mut structs = Vec::new();
        let mut tagged_unions = Vec::new();
        let mut type_aliases = Vec::new();
        let mut imports = Vec::new();
        let mut exports = Vec::new();

        while self.peek_kind() != TokenKind::Eof {
            match self.peek_kind() {
                TokenKind::Component => {
                    components.push(self.parse_component_def()?);
                }
                TokenKind::Store => {
                    stores.push(self.parse_store_def()?);
                }
                TokenKind::Theme => {
                    themes.push(self.parse_theme_def()?);
                }
                TokenKind::Import => {
                    imports.push(self.parse_import()?);
                }
                TokenKind::Export => {
                    exports.push(self.parse_export()?);
                }
                TokenKind::Fn => {
                    functions.push(self.parse_function_def(Visibility::Private)?);
                }
                TokenKind::Struct => {
                    structs.push(self.parse_struct_def(Visibility::Private)?);
                }
                TokenKind::Pub => {
                    self.advance(); // 'pub'
                    match self.peek_kind() {
                        TokenKind::Fn => {
                            functions.push(self.parse_function_def(Visibility::Public)?);
                        }
                        TokenKind::Struct => {
                            structs.push(self.parse_struct_def(Visibility::Public)?);
                        }
                        TokenKind::Type => {
                            type_aliases.push(self.parse_type_alias(Visibility::Public)?);
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
                        tagged_unions.push(self.parse_tagged_union_def()?);
                    } else {
                        // Advance past 'type' Name '='
                        self.advance(); // 'type'
                        self.advance(); // name
                        self.advance(); // '='

                        // Check if next token starts a variant: Ident { ... } or Ident | ...
                        let is_tagged = match self.peek_kind() {
                            TokenKind::Ident(_) => self
                                .tokens
                                .get(self.pos + 1)
                                .is_some_and(|t| t.kind == TokenKind::LBrace || t.kind == TokenKind::Pipe),
                            _ => false,
                        };

                        self.pos = saved; // reset to 'type'

                        if is_tagged {
                            tagged_unions.push(self.parse_tagged_union_def()?);
                        } else {
                            type_aliases.push(self.parse_type_alias(Visibility::Private)?);
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

        Ok(Program {
            components,
            stores,
            themes,
            functions,
            structs,
            type_aliases,
            tagged_unions,
            imports,
            exports,
        })
    }

    // ── Helpers ────────────────────────────────────────

    /// Check if the next token is a left brace `{`.
    pub fn peek_is_brace(&self) -> bool {
        self.peek_kind() == TokenKind::LBrace
    }

    /// Consume a semicolon if present (optional semicolon).
    pub fn consume_semicolon(&mut self) {
        if self.peek_kind() == TokenKind::Semicolon {
            self.advance();
        }
    }

    /// Expect and return a string literal token.
    pub fn expect_string(&mut self, msg: &str) -> Result<String, ParseError> {
        let t = self.advance();
        match &t.kind {
            TokenKind::StringLiteral(s) => Ok(s.clone()),
            _ => Err(ParseError {
                message: format!("{} — found {:?}", msg, t.kind),
                span: t.span,
            }),
        }
    }

    /// Construct a ParseError at the current peek position.
    pub fn error(&self, msg: impl Into<String>) -> ParseError {
        ParseError {
            message: msg.into(),
            span: self.peek_span(),
        }
    }
}
