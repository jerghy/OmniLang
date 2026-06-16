//! Parser module — recursive descent + Pratt expression parsing.

pub mod ast;
pub mod expr;
pub mod stmt;
pub mod top_level;
pub mod view;

use crate::error::ParseError;
use crate::lexer::{Token, TokenKind};
use ast::Program;

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
        let mut imports = Vec::new();

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
                TokenKind::Fn => {
                    functions.push(self.parse_function_def()?);
                }
                TokenKind::Struct => {
                    structs.push(self.parse_struct_def()?);
                }
                TokenKind::Type => {
                    tagged_unions.push(self.parse_tagged_union_def()?);
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
            tagged_unions,
            imports,
        })
    }
}
