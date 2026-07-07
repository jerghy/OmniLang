//! Parser module — recursive descent + Pratt expression parsing.

pub mod ast;
pub mod design;
pub mod expr;
pub mod stmt;
pub mod top_level;
pub mod view;

use rustc_hash::FxHashMap;
use crate::error::ParseError;
use crate::lexer::{Token, TokenKind};
use crate::parser::stmt::keyword_as_ident_name;
use ast::Program;
#[allow(unused_imports)]
use ast::Visibility;

/// Parse a token stream into an AST Program.
/// Returns `Ok(program)` on success, or `Err(ParseError)` if any parse errors were
/// collected during recovery. The first collected error is returned.
pub fn parse(tokens: Vec<Token>, source: &str) -> Result<Program, ParseError> {
    let mut parser = Parser::new(tokens, source);
    let module = parser.parse_module()?;
    // Return the first collected parse error (if any) so callers know the AST
    // may be incomplete. The remaining errors are discarded at this level.
    if let Some(err) = parser.errors.first() {
        return Err(ParseError {
            message: format!(
                "{} (and {} more error(s))",
                err.message,
                parser.errors.len() - 1
            ),
            span: err.span,
        });
    }
    let mut modules = FxHashMap::default();
    modules.insert("input.omni".to_string(), module);
    Ok(Program { modules })
}

/// Maximum recursion depth for the parser to prevent stack overflow.
const MAX_RECURSION_DEPTH: usize = 256;

/// The recursive descent parser.
pub struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    depth: usize,
    source: &'a str,
    /// Collected parse errors — parser continues past recoverable errors.
    errors: Vec<ParseError>,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: Vec<Token>, source: &'a str) -> Self {
        Parser {
            tokens,
            pos: 0,
            depth: 0,
            source,
            errors: Vec::new(),
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

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or_else(|| {
            // Static fallback — initialized once
            static EOF: std::sync::OnceLock<Token> = std::sync::OnceLock::new();
            EOF.get_or_init(|| Token::new(TokenKind::Eof, 0, 0, 0, 0))
        })
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    fn advance(&mut self) -> Token {
        if self.pos >= self.tokens.len() {
            return Token::new(TokenKind::Eof, 0, 0, 0, 0);
        }
        // Move the real token out, leave a cheap placeholder in its place.
        // The placeholder is never accessed because positions < pos are retired.
        let placeholder = Token::placeholder();
        let token = std::mem::replace(&mut self.tokens[self.pos], placeholder);
        self.pos += 1;
        token
    }

    /// Zero-clone variant check. Returns true if current token matches `kind`.
    fn at(&self, kind: &TokenKind) -> bool {
        self.tokens.get(self.pos).is_some_and(|t| &t.kind == kind)
    }

    /// Check-and-consume. Returns true and advances if current token matches `kind`.
    fn advance_if(&mut self, kind: &TokenKind) -> bool {
        if self.at(kind) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    // ── Helpers ────────────────────────────────────────

    /// Peek at the string value of the next identifier without consuming it.
    /// Handles both TokenKind::Ident(name) and keyword tokens via keyword_as_ident_name().
    pub fn peek_ident_value(&self) -> Option<String> {
        let tok = self.peek();
        match &tok.kind {
            TokenKind::Ident(name) => Some(name.clone()),
            other => keyword_as_ident_name(other).map(|s| s.to_string()),
        }
    }

    /// Check if the next token is a left brace `{`.
    pub fn peek_is_brace(&self) -> bool {
        self.at(&TokenKind::LBrace)
    }

    /// Consume a semicolon if present (optional semicolon).
    pub fn consume_semicolon(&mut self) {
        self.advance_if(&TokenKind::Semicolon);
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

    // ── Error recovery helpers ──────────────────────────
    // These are intentionally unused until error recovery logic is activated.

    /// Record a parse error (non-fatal — parsing continues).
    fn emit_error(&mut self, err: ParseError) {
        self.errors.push(err);
    }

    /// Skip tokens until one of `expected` is found (or EOF).
    /// Returns number of tokens skipped. Does NOT consume the found token.
    fn skip_to(&mut self, expected: &[TokenKind]) -> usize {
        let start = self.pos;
        loop {
            let kind = self.peek_kind();
            if expected.contains(kind) || *kind == TokenKind::Eof {
                break;
            }
            self.advance();
        }
        self.pos - start
    }

    /// Skip to a statement boundary: ';' (consumed), '}', or EOF.
    fn skip_to_stmt_boundary(&mut self) {
        loop {
            match self.peek_kind() {
                TokenKind::Semicolon => {
                    self.advance();
                    return;
                }
                TokenKind::RBrace | TokenKind::Eof => return,
                _ => {
                    self.advance();
                }
            }
        }
    }

    /// Recover from a parse error during statement parsing.
    /// Fixes the recursion depth (leave_recursion was skipped) and skips
    /// to the next statement boundary so parsing can continue.
    fn recover_from_error(&mut self) {
        self.depth = self.depth.saturating_sub(1);
        // Only skip if we're not already at a safe restart position
        // (statement boundary, statement-start keyword, or EOF).
        // This prevents overshooting when the failed parser already consumed
        // the boundary token (e.g. parse_prefix() advances before erroring).
        if !self.at_stmt_restart() {
            self.skip_to_stmt_boundary();
        }
    }

    /// Returns true if the current token is a safe position to resume
    /// statement parsing: either a boundary token, a keyword that starts
    /// a new statement, or a token that can start an expression statement.
    fn at_stmt_restart(&self) -> bool {
        matches!(
            self.peek_kind(),
            TokenKind::Semicolon
                | TokenKind::RBrace
                | TokenKind::Eof
                | TokenKind::Let
                | TokenKind::If
                | TokenKind::While
                | TokenKind::For
                | TokenKind::Return
                | TokenKind::Break
                | TokenKind::Continue
                | TokenKind::Print
                | TokenKind::Println
                | TokenKind::Throw
                | TokenKind::Try
                | TokenKind::Emit
                | TokenKind::LBrace
                | TokenKind::Ident(_)
                | TokenKind::IntLiteral(_)
                | TokenKind::FloatLiteral(_)
                | TokenKind::StringLiteral(_)
                | TokenKind::TemplateString(_)
                | TokenKind::True
                | TokenKind::False
                | TokenKind::Null
                | TokenKind::LParen
                | TokenKind::Minus
                | TokenKind::Not
                | TokenKind::Await
                | TokenKind::Match
                | TokenKind::Mut
        )
    }

    /// Skip to next top-level declaration keyword.
    fn skip_to_top_level(&mut self) {
        const TOP_LEVEL: &[TokenKind] = &[
            TokenKind::Fn,
            TokenKind::Pub,
            TokenKind::Component,
            TokenKind::Store,
            TokenKind::Theme,
            TokenKind::Design,
            TokenKind::Struct,
            TokenKind::Type,
            TokenKind::Const,
            TokenKind::Import,
            TokenKind::Export,
            TokenKind::AtMotion,
            TokenKind::AtTimeline,
            TokenKind::Async,
        ];
        self.skip_to(TOP_LEVEL);
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
        let program = parse(tokens, source).unwrap();
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
        let program = parse(tokens, source).unwrap();
        let module = program.modules.get("input.omni").unwrap();
        assert_eq!(module.structs.len(), 1);
        assert_eq!(module.structs[0].name, "Point");
        assert_eq!(module.structs[0].fields.len(), 2);
    }

    #[test]
    fn roundtrip_enum_def() {
        let source = "type Color = Red | Green | Blue";
        let tokens = tokenize(source);
        let program = parse(tokens, source).unwrap();
        let module = program.modules.get("input.omni").unwrap();
        assert_eq!(module.tagged_unions.len(), 1);
    }

    #[test]
    fn roundtrip_type_alias() {
        let source = "type UserId = i32;";
        let tokens = tokenize(source);
        let program = parse(tokens, source).unwrap();
        let module = program.modules.get("input.omni").unwrap();
        assert_eq!(module.type_aliases.len(), 1);
        assert_eq!(module.type_aliases[0].name, "UserId");
    }

    #[test]
    fn roundtrip_import_export() {
        let source = "import { add } from \"./utils/math.omni\";\nexport { main } from \"./utils/math.omni\";";
        let tokens = tokenize(source);
        let program = parse(tokens, source).unwrap();
        let module = program.modules.get("input.omni").unwrap();
        assert_eq!(module.imports.len(), 1);
        assert_eq!(module.exports.len(), 1);
    }

    #[test]
    fn test_reject_invalid_hex() {
        let source = "let x = 0xFFG;";
        let tokens = tokenize(source);
        let result = parse(tokens, source);
        assert!(
            result.is_err(),
            "Invalid hex '0xFFG' should produce parse error"
        );
    }

    #[test]
    fn test_reject_invalid_float() {
        let source = "let x = 1.2.3;";
        let tokens = tokenize(source);
        let result = parse(tokens, source);
        assert!(
            result.is_err(),
            "Invalid float '1.2.3' should produce parse error"
        );
    }

    // ── Error recovery tests ────────────────────────────

    #[test]
    fn error_recovery_mid_function() {
        // Syntax error mid-function should not prevent the rest from parsing.
        // `let x = ;` fails at expression parsing; recovery skips to `;`
        // boundary so `let y = 42;` and `return y;` are still parsed.
        let source = "pub fn main() { let x = ; let y = 42; return y; }";
        let tokens = tokenize(source);
        let mut parser = Parser::new(tokens, source);
        let module = parser.parse_module().unwrap();
        assert_eq!(
            parser.errors.len(),
            1,
            "exactly one parse error should be collected"
        );
        assert_eq!(module.functions.len(), 1, "function should still parse");
        assert_eq!(
            module.functions[0].body.stmts.len(),
            2,
            "function body should contain 2 valid statements after recovery"
        );
    }

    #[test]
    fn error_recovery_garbage_between_functions() {
        // Garbage between top-level declarations should be skipped,
        // allowing both adjacent functions to parse.
        let source = "pub fn foo() { return 1; } quux pub fn bar() { return 2; }";
        let tokens = tokenize(source);
        let mut parser = Parser::new(tokens, source);
        let module = parser.parse_module().unwrap();
        assert_eq!(
            parser.errors.len(),
            1,
            "exactly one parse error should be collected for the garbage ident"
        );
        assert_eq!(
            module.functions.len(),
            2,
            "both functions should parse despite intervening garbage"
        );
        assert_eq!(module.functions[0].name, "foo");
        assert_eq!(module.functions[1].name, "bar");
    }
}
