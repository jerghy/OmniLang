//! Parser module — recursive descent + Pratt expression parsing.

pub mod ast;

use crate::error::ParseError;
use crate::lexer::{Token, TokenKind};
use ast::Program;

fn eof_token() -> Token {
    Token::new(TokenKind::Eof, 0, 0, 0, 0)
}

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
        self.tokens.get(self.pos).cloned().unwrap_or_else(eof_token)
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

    fn expect(&mut self, msg: &str) -> Result<Token, ParseError> {
        let t = self.peek();
        if matches!(t.kind, TokenKind::Eof) {
            return Err(ParseError {
                message: format!("Unexpected end of file: {}", msg),
                span: t.span.clone(),
            });
        }
        Ok(self.advance())
    }

    fn parse_program(&mut self) -> Result<Program, ParseError> {
        Ok(Program {
            components: vec![],
            stores: vec![],
            themes: vec![],
            functions: vec![],
            structs: vec![],
            tagged_unions: vec![],
            imports: vec![],
        })
    }
}
