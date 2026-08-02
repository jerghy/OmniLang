use crate::error::ParseError;
use crate::lexer::{Span, TokenKind};
use crate::parser::ast::*;
use crate::parser::Parser;

impl<'a> Parser<'a> {
    // ── Store ──────────────────────────────────────────

    pub fn parse_store_def(&mut self) -> Result<StoreDef, ParseError> {
        let start = self.peek_span();
        self.advance(); // 'store'
        let name = self.expect_ident("expected store name")?;
        self.expect_token(TokenKind::LBrace, "expected '{'")?;

        let mut signals = Vec::new();
        let mut getters = Vec::new();
        let mut actions = Vec::new();

        while *self.peek_kind() != TokenKind::RBrace && *self.peek_kind() != TokenKind::Eof {
            match self.peek_kind() {
                TokenKind::Signal => signals.push(self.parse_signal_decl()?),
                TokenKind::Getter => getters.push(self.parse_getter_decl()?),
                TokenKind::Action => actions.push(self.parse_action_def()?),
                // Task 3.5 (域 5 BNF v2): 顶层与内联 store 成员集统一为
                // signal|getter|action——`fn` 成员两层都报错（设计文档
                // 诊断表原文：「用 getter 或 action」）。
                TokenKind::Fn => {
                    return Err(ParseError {
                        kind: None,
                        message: "store 内 fn 成员不允许；用 getter 或 action".into(),
                        span: self.peek_span(),
                    });
                }
                _ => {
                    let t = self.advance();
                    return Err(ParseError {
                        kind: None,
                        message: format!("Unexpected token in store: {:?}", t.kind),
                        span: t.span,
                    });
                }
            }
        }

        self.expect_token(TokenKind::RBrace, "expected '}'")?;

        Ok(StoreDef {
            name,
            signals,
            getters,
            actions,
            span: Span {
                start: start.start,
                end: start.end,
                line: start.line,
                col: start.col,
            },
        })
    }

    pub(super) fn parse_getter_decl(&mut self) -> Result<GetterDecl, ParseError> {
        self.advance(); // 'getter'
        let name = self.expect_ident("expected getter name")?;
        self.expect_token(TokenKind::Colon, "expected ':'")?;
        let type_ann = self.parse_type_annotation()?;
        self.expect_token(TokenKind::Eq, "expected '='")?;
        let body = self.parse_expr()?;
        if *self.peek_kind() == TokenKind::Semicolon {
            self.advance();
        }
        Ok(GetterDecl {
            name,
            type_ann,
            body,
        })
    }
}
