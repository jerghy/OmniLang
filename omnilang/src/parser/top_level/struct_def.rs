use crate::error::ParseError;
use crate::lexer::{Span, TokenKind};
use crate::parser::ast::*;
use crate::parser::Parser;

impl<'a> Parser<'a> {
    // ── Struct ─────────────────────────────────────────

    pub fn parse_struct_def(&mut self, visibility: Visibility) -> Result<StructDef, ParseError> {
        let start = self.peek_span();
        self.advance(); // 'struct'
        let is_nominal = if self.peek_kind() == TokenKind::Nominal {
            self.advance();
            true
        } else {
            false
        };
        let name = self.expect_ident("expected struct name")?;
        self.expect_token(TokenKind::LBrace, "expected '{'")?;

        let mut fields = Vec::new();
        while self.peek_kind() != TokenKind::RBrace && self.peek_kind() != TokenKind::Eof {
            let field_name = self.expect_ident("expected field name")?;
            self.expect_token(TokenKind::Colon, "expected ':'")?;
            let type_ann = self.parse_type_annotation()?;
            fields.push(StructField {
                name: field_name,
                type_ann,
            });
            if self.peek_kind() == TokenKind::Comma {
                self.advance();
            }
        }
        self.expect_token(TokenKind::RBrace, "expected '}'")?;

        Ok(StructDef {
            visibility,
            name,
            is_nominal,
            fields,
            span: Span {
                start: start.start,
                end: start.end,
                line: start.line,
                col: start.col,
            },
        })
    }

    // ── Tagged Union ───────────────────────────────────

    pub fn parse_tagged_union_def(&mut self) -> Result<TaggedUnionDef, ParseError> {
        let start = self.peek_span();
        self.advance(); // 'type'
        let name = self.expect_ident("expected type name")?;
        self.expect_token(TokenKind::Eq, "expected '='")?;

        let mut variants = Vec::new();
        // Variant list: Variant1 { fields } | Variant2 { fields }
        loop {
            let variant_name = self.expect_ident("expected variant name")?;
            let fields = if self.peek_kind() == TokenKind::LBrace {
                self.advance();
                let mut f = Vec::new();
                while self.peek_kind() != TokenKind::RBrace && self.peek_kind() != TokenKind::Eof {
                    let fname = self.expect_ident("expected field name")?;
                    self.expect_token(TokenKind::Colon, "expected ':'")?;
                    let ft = self.parse_type_annotation()?;
                    f.push(StructField {
                        name: fname,
                        type_ann: ft,
                    });
                    if self.peek_kind() == TokenKind::Comma {
                        self.advance();
                    }
                }
                self.expect_token(TokenKind::RBrace, "expected '}'")?;
                f
            } else {
                vec![]
            };
            variants.push(Variant {
                name: variant_name,
                fields,
            });
            if self.peek_kind() != TokenKind::Pipe {
                break;
            }
            self.advance(); // '|'
        }

        Ok(TaggedUnionDef {
            name,
            variants,
            span: Span {
                start: start.start,
                end: start.end,
                line: start.line,
                col: start.col,
            },
        })
    }

    // ── Type Alias ─────────────────────────────────────

    pub fn parse_type_alias(&mut self, visibility: Visibility) -> Result<TypeAlias, ParseError> {
        let start = self.peek_span();
        self.advance(); // 'type'
        let name = self.expect_ident("expected type name")?;
        self.expect_token(TokenKind::Eq, "expected '='")?;
        let type_ann = self.parse_type_annotation()?;
        self.consume_semicolon();
        let end = self.peek_span();
        Ok(TypeAlias {
            visibility,
            name,
            type_ann,
            span: Span {
                start: start.start,
                end: end.end,
                line: start.line,
                col: start.col,
            },
        })
    }
}
