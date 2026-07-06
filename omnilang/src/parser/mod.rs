//! Parser module — recursive descent + Pratt expression parsing.

pub mod ast;
pub mod expr;
pub mod stmt;
pub mod top_level;
pub mod view;

use std::collections::HashMap;

use crate::error::ParseError;
use crate::lexer::{Token, TokenKind};
use crate::parser::stmt::keyword_as_ident_name;
use ast::Program;
#[allow(unused_imports)]
use ast::Visibility;

/// Parse a token stream into an AST Program.
pub fn parse(tokens: &[Token], source: &str) -> Result<Program, ParseError> {
    let mut parser = Parser::new(tokens, source);
    let module = parser.parse_module()?;
    let mut modules = HashMap::new();
    modules.insert("input.omni".to_string(), module);
    Ok(Program { modules })
}

/// Maximum recursion depth for the parser to prevent stack overflow.
const MAX_RECURSION_DEPTH: usize = 256;

/// The recursive descent parser.
pub struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
    depth: usize,
    source: &'a str,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a [Token], source: &'a str) -> Self {
        Parser {
            tokens,
            pos: 0,
            depth: 0,
            source,
        }
    }

    /// Increment recursion depth, returning an error if the limit is exceeded.
    fn enter_recursion(&mut self) -> Result<(), ParseError> {
        self.depth += 1;
        if self.depth > MAX_RECURSION_DEPTH {
            return Err(ParseError {
                message: format!(
                    "maximum recursion depth ({}) exceeded — input may be too deeply nested",
                    MAX_RECURSION_DEPTH
                ),
                span: self.peek_span(),
            });
        }
        Ok(())
    }

    /// Decrement recursion depth on exit.
    fn leave_recursion(&mut self) {
        self.depth = self.depth.saturating_sub(1);
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

    // ── Helpers ────────────────────────────────────────

    /// Peek at the string value of the next identifier without consuming it.
    /// Handles both TokenKind::Ident(name) and keyword tokens via keyword_as_ident_name().
    pub fn peek_ident_value(&self) -> Option<String> {
        let tok = self.peek();
        match &tok.kind {
            TokenKind::Ident(name) => Some(name.clone()),
            other => keyword_as_ident_name(other),
        }
    }

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

#[cfg(test)]
mod roundtrip_tests {
    use super::*;
    use crate::lexer::tokenize;

    #[test]
    fn roundtrip_function_def() {
        let source = "pub fn main() { return 42; }";
        let tokens = tokenize(source);
        let program = parse(&tokens, source).unwrap();
        assert_eq!(program.modules.len(), 1);
        let module = program.modules.get("input.omni").unwrap();
        assert_eq!(module.functions.len(), 1);
        let func = &module.functions[0];
        assert_eq!(func.name, "main");
        assert!(matches!(func.visibility, Visibility::Public));
    }

    #[test]
    fn roundtrip_struct_def() {
        let source = "struct Point { x: i32, y: i32 }";
        let tokens = tokenize(source);
        let program = parse(&tokens, source).unwrap();
        let module = program.modules.get("input.omni").unwrap();
        assert_eq!(module.structs.len(), 1);
        assert_eq!(module.structs[0].name, "Point");
        assert_eq!(module.structs[0].fields.len(), 2);
    }

    #[test]
    fn roundtrip_enum_def() {
        let source = "type Color = Red | Green | Blue";
        let tokens = tokenize(source);
        let program = parse(&tokens, source).unwrap();
        let module = program.modules.get("input.omni").unwrap();
        assert_eq!(module.tagged_unions.len(), 1);
    }

    #[test]
    fn roundtrip_type_alias() {
        let source = "type UserId = i32;";
        let tokens = tokenize(source);
        let program = parse(&tokens, source).unwrap();
        let module = program.modules.get("input.omni").unwrap();
        assert_eq!(module.type_aliases.len(), 1);
        assert_eq!(module.type_aliases[0].name, "UserId");
    }

    #[test]
    fn roundtrip_import_export() {
        let source = "import { add } from \"./utils/math.omni\";\nexport { main } from \"./utils/math.omni\";";
        let tokens = tokenize(source);
        let program = parse(&tokens, source).unwrap();
        let module = program.modules.get("input.omni").unwrap();
        assert_eq!(module.imports.len(), 1);
        assert_eq!(module.exports.len(), 1);
    }

    #[test]
    fn test_reject_invalid_hex() {
        let source = "let x = 0xFFG;";
        let tokens = tokenize(source);
        let result = parse(&tokens, source);
        assert!(
            result.is_err(),
            "Invalid hex '0xFFG' should produce parse error"
        );
    }

    #[test]
    fn test_reject_invalid_float() {
        let source = "let x = 1.2.3;";
        let tokens = tokenize(source);
        let result = parse(&tokens, source);
        assert!(
            result.is_err(),
            "Invalid float '1.2.3' should produce parse error"
        );
    }
}
