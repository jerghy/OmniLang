use crate::error::ParseError;
use crate::lexer::{Span, TokenKind};
use crate::parser::ast::*;
use crate::parser::Parser;

impl<'a> Parser<'a> {
    // ── Import ─────────────────────────────────────────

    pub fn parse_import(&mut self) -> Result<ImportStmt, ParseError> {
        let start = self.peek_span();
        self.advance(); // 'import'

        match self.peek_kind() {
            TokenKind::Type => {
                // import type { A, B } from "./path.omni";
                self.advance(); // 'type'
                let names = self.parse_import_name_list()?;
                self.expect_token(TokenKind::From, "expected 'from' after 'import type'")?;
                let path = self.expect_string("expected module path string")?;
                self.consume_semicolon();
                let end = self.peek_span();
                Ok(ImportStmt::TypeOnly(TypeImport {
                    names,
                    path,
                    span: Span {
                        start: start.start,
                        end: end.end,
                        line: start.line,
                        col: start.col,
                    },
                }))
            }
            TokenKind::LBrace => {
                // import { A, B } from "./path.omni";
                let names = self.parse_import_name_list()?;
                self.expect_token(TokenKind::From, "expected 'from'")?;
                let path = self.expect_string("expected module path string")?;
                self.consume_semicolon();
                let end = self.peek_span();
                Ok(ImportStmt::Named(NamedImport {
                    names,
                    path,
                    span: Span {
                        start: start.start,
                        end: end.end,
                        line: start.line,
                        col: start.col,
                    },
                }))
            }
            TokenKind::StringLiteral(_) => {
                // import "./path.omni" as name;
                let path = self.expect_string("expected module path string")?;
                self.expect_token(TokenKind::As, "expected 'as' after path")?;
                let alias = self.expect_ident("expected alias name")?;
                self.consume_semicolon();
                let end = self.peek_span();
                Ok(ImportStmt::Namespace(NamespaceImport {
                    path,
                    alias,
                    span: Span {
                        start: start.start,
                        end: end.end,
                        line: start.line,
                        col: start.col,
                    },
                }))
            }
            _ => {
                // import omniview; — bare import (lib marker)
                let name = self.expect_ident("expected import name, '{', or string")?;
                self.consume_semicolon();
                Ok(ImportStmt::Lib(name))
            }
        }
    }

    /// Parse `{ Name1, Name2 }` import/export name list.
    fn parse_import_name_list(&mut self) -> Result<Vec<String>, ParseError> {
        self.expect_token(TokenKind::LBrace, "expected '{'")?;
        let mut names = Vec::new();
        if *self.peek_kind() != TokenKind::RBrace {
            names.push(self.expect_ident("expected import name")?);
            while *self.peek_kind() == TokenKind::Comma {
                self.advance();
                if *self.peek_kind() == TokenKind::RBrace {
                    break; // trailing comma
                }
                names.push(self.expect_ident("expected import name")?);
            }
        }
        self.expect_token(TokenKind::RBrace, "expected '}'")?;
        Ok(names)
    }

    // ── Export ─────────────────────────────────────────

    pub fn parse_export(&mut self) -> Result<ExportFrom, ParseError> {
        let start = self.peek_span();
        self.advance(); // 'export'
        let names = self.parse_import_name_list()?;
        self.expect_token(TokenKind::From, "expected 'from'")?;
        let path = self.expect_string("expected module path string")?;
        self.consume_semicolon();
        let end = self.peek_span();
        Ok(ExportFrom {
            names,
            path,
            span: Span {
                start: start.start,
                end: end.end,
                line: start.line,
                col: start.col,
            },
        })
    }
}
