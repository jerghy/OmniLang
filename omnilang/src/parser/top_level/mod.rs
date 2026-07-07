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
                    TokenKind::Theme => {
                        module.themes.push(self.parse_theme_def()?);
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
                    TokenKind::AtTimeline => {
                        module.timelines.push(self.parse_timeline_def()?);
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
                            module.tagged_unions.push(self.parse_tagged_union_def()?);
                        } else {
                            // Advance past 'type' Name '='
                            self.advance(); // 'type'
                            self.advance(); // name
                            self.advance(); // '='

                            // Check if next token starts a variant: Ident { ... } or Ident | ...
                            let is_tagged = match self.peek_kind() {
                                TokenKind::Ident(_) => {
                                    self.tokens.get(self.pos + 1).is_some_and(|t| {
                                        t.kind == TokenKind::LBrace || t.kind == TokenKind::Pipe
                                    })
                                }
                                _ => false,
                            };

                            self.pos = saved; // reset to 'type'

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

/// Parse a position value string (from `start: "..."` syntax).
pub fn parse_position_string(raw: &str) -> PositionValue {
    match raw {
        "withPrev" => return PositionValue::WithPrev,
        "after" => return PositionValue::After,
        _ => {}
    }
    if raw.starts_with("overlap(") && raw.ends_with(')') {
        let n = raw
            .trim_start_matches("overlap(")
            .trim_end_matches(')')
            .to_string();
        return PositionValue::Overlap(n);
    }
    if raw.starts_with("afterPrev(") && raw.ends_with(')') {
        let n = raw
            .trim_start_matches("afterPrev(")
            .trim_end_matches(')')
            .to_string();
        return PositionValue::AfterPrev(n);
    }
    if raw.starts_with("withOffset(") && raw.ends_with(')') {
        let n = raw
            .trim_start_matches("withOffset(")
            .trim_end_matches(')')
            .to_string();
        return PositionValue::WithOffset(n);
    }
    if raw.starts_with("absolute(") && raw.ends_with(')') {
        let n = raw
            .trim_start_matches("absolute(")
            .trim_end_matches(')')
            .to_string();
        return PositionValue::Absolute(n);
    }
    PositionValue::Absolute(raw.to_string())
}
