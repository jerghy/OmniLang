//! Top-level declaration parsers.

pub mod component;
pub mod design;
pub mod function;
pub mod module;
pub mod motion;
pub mod store;
pub mod struct_def;
pub mod timeline;

use crate::error::ParseError;
use crate::lexer::TokenKind;
use crate::parser::ast::*;
use crate::parser::Parser;

impl<'a> Parser<'a> {
    // ── Top-Level Module ───────────────────────────────

    /// Parse the top-level of a single source module.
    /// Returns a `Module` with all top-level declarations populated.
    pub fn parse_module(&mut self) -> Result<Module, ParseError> {
        let mut module = Module {
            name: "input".to_string(),
            ..Default::default()
        };

        while *self.peek_kind() != TokenKind::Eof {
            let result = (|| -> Result<(), ParseError> {
                match self.peek_kind() {
                    TokenKind::Component => {
                        module.components.push(self.parse_component_def()?);
                    }
                    TokenKind::Store => {
                        module.stores.push(self.parse_store_def()?);
                    }
                    TokenKind::Design => {
                        module.designs.push(self.parse_design_def()?);
                    }
                    TokenKind::Import => {
                        module.imports.push(self.parse_import()?);
                    }
                    TokenKind::Export => {
                        module.exports.push(self.parse_export()?);
                    }
                    TokenKind::AtMotion => {
                        module.motions.push(self.parse_motion_def()?);
                    }
                    TokenKind::Async => {
                        self.advance(); // 'async'
                        self.expect_token(TokenKind::Fn, "expected 'fn' after 'async'")?;
                        module.functions.push(
                            self.parse_function_def_after_keyword(Visibility::Private, true)?,
                        );
                    }
                    TokenKind::Fn => {
                        module
                            .functions
                            .push(self.parse_function_def(Visibility::Private)?);
                    }
                    TokenKind::Struct => {
                        module
                            .structs
                            .push(self.parse_struct_def(Visibility::Private)?);
                    }
                    TokenKind::Pub => {
                        self.advance(); // 'pub'
                        match self.peek_kind() {
                            TokenKind::Async => {
                                self.advance(); // 'async'
                                self.expect_token(TokenKind::Fn, "expected 'fn' after 'async'")?;
                                module.functions.push(
                                    self.parse_function_def_after_keyword(
                                        Visibility::Public,
                                        true,
                                    )?,
                                );
                            }
                            TokenKind::Fn => {
                                module
                                    .functions
                                    .push(self.parse_function_def(Visibility::Public)?);
                            }
                            TokenKind::Struct => {
                                module
                                    .structs
                                    .push(self.parse_struct_def(Visibility::Public)?);
                            }
                            TokenKind::Type => {
                                module
                                    .type_aliases
                                    .push(self.parse_type_alias(Visibility::Public)?);
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
                        // Look ahead to determine if this is a type alias or tagged union.
                        // Both have the form: type Name = ...
                        // After '=': tagged unions have `Ident {` or `Ident |`,
                        //            type aliases have anything else.
                        let after_type = self.tokens.get(self.pos + 1);
                        let after_name = self.tokens.get(self.pos + 2);
                        let looks_like_type_alias = matches!(
                            (after_type.map(|t| &t.kind), after_name.map(|t| &t.kind)),
                            (Some(TokenKind::Ident(_)), Some(TokenKind::Eq))
                        );

                        if !looks_like_type_alias {
                            // Cannot be a type alias or standard tagged union — try tagged union
                            module.tagged_unions.push(self.parse_tagged_union_def()?);
                        } else {
                            // Has `type Name =`. Check if the part after = is a variant list.
                            let after_eq = self.tokens.get(self.pos + 3);
                            let after_variant = self.tokens.get(self.pos + 4);
                            let is_tagged =
                                matches!(after_eq.map(|t| &t.kind), Some(TokenKind::Ident(_)))
                                    && matches!(
                                        after_variant.map(|t| &t.kind),
                                        Some(TokenKind::LBrace | TokenKind::Pipe)
                                    );

                            if is_tagged {
                                module.tagged_unions.push(self.parse_tagged_union_def()?);
                            } else {
                                module
                                    .type_aliases
                                    .push(self.parse_type_alias(Visibility::Private)?);
                            }
                        }
                    }
                    TokenKind::Const => {
                        // Parse and discard module-level const declarations.
                        // const values (e.g., theme constants) are handled via imports.
                        self.advance();
                        if *self.peek_kind() == TokenKind::Dollar {
                            self.advance();
                            self.expect_ident("expected identifier after $")?;
                        } else {
                            self.expect_ident("expected constant name")?;
                        }
                        self.expect_token(TokenKind::Eq, "expected '='")?;
                        // Consume the value expression (literal or array)
                        if *self.peek_kind() == TokenKind::LBracket {
                            self.parse_expr()?;
                        } else {
                            self.advance();
                        }
                        if *self.peek_kind() == TokenKind::Semicolon {
                            self.advance();
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
                Ok(())
            })();
            if let Err(e) = result {
                self.emit_error(e);
                self.skip_to_top_level();
            }
        }

        Ok(module)
    }
}
