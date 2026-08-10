//! Component-level navigation declarations: `@route` / `@layout` / `@query`
//! (spec 2026-08-09 §2.1/§2.4/§2.6). These are component-top-level directives
//! on the same level as `prop`/`signal`/`event`/`fn` — a component declares
//! itself a route page, a layout, and/or query-parameter props.
//!
//! Parse-layer validation only (V1/V2/V5 from spec §3 — 解析错误). Semantic
//! validation (V3-V13, W1) lives in `semantic::validate_route_navigation`.

use crate::error::ParseError;
use crate::lexer::{Span, TokenKind};
use crate::parser::ast::*;
use crate::parser::Parser;

impl<'a> Parser<'a> {
    /// Parse `@route "/path" (guard: IDENT | lazy: BOOL | redirect: STRING)* ;`
    /// (spec §2.1/§2.5/§2.8). Consumes the `@route` token itself (the
    /// component dispatch loop dispatches on the peeked kind without
    /// advancing); this parses through the trailing semicolon.
    pub(super) fn parse_route_decl(&mut self) -> Result<RouteDecl, ParseError> {
        let start = self.peek_span();
        self.advance(); // '@route'
        let path = match self.advance().kind {
            TokenKind::StringLiteral(s) => s,
            other => {
                return Err(ParseError {
                    kind: None,
                    message: format!("@route 路径必须是字符串字面量，got {:?}", other),
                    span: self.peek_span(),
                })
            }
        };
        // V2: path must be non-empty and start with '/'.
        if path.is_empty() || !path.starts_with('/') {
            return Err(ParseError {
                kind: None,
                message: format!("路由路径必须以 `/` 开头且非空：'{path}'"),
                span: start,
            });
        }
        let params = parse_path_params(&path, start)?;

        // Attribute extension points: `guard:` / `lazy:` / `redirect:`.
        let mut guard = None;
        let mut lazy = None;
        let mut redirect = None;
        while let Some(key) = self.try_parse_ident() {
            let key_span = self.peek_span();
            match key.as_str() {
                "guard" => {
                    self.expect_token(TokenKind::Colon, "expected ':' after 'guard'")?;
                    guard = Some(self.expect_ident("expected guard fn name")?);
                }
                "lazy" => {
                    self.expect_token(TokenKind::Colon, "expected ':' after 'lazy'")?;
                    lazy = Some(match self.advance().kind {
                        TokenKind::True => true,
                        TokenKind::False => false,
                        other => {
                            return Err(ParseError {
                                kind: None,
                                message: format!("lazy 属性必须是 true/false，got {:?}", other),
                                span: key_span,
                            })
                        }
                    });
                }
                "redirect" => {
                    self.expect_token(TokenKind::Colon, "expected ':' after 'redirect'")?;
                    redirect = Some(match self.advance().kind {
                        TokenKind::StringLiteral(s) => s,
                        other => {
                            return Err(ParseError {
                                kind: None,
                                message: format!(
                                    "redirect 属性必须是字符串字面量，got {:?}",
                                    other
                                ),
                                span: key_span,
                            })
                        }
                    });
                }
                other => {
                    return Err(ParseError {
                        kind: None,
                        message: format!(
                            "未知 @route 属性 \"{other}\"——合法属性: guard/lazy/redirect"
                        ),
                        span: key_span,
                    })
                }
            }
        }
        if *self.peek_kind() == TokenKind::Semicolon {
            self.advance();
        }
        Ok(RouteDecl {
            path,
            params,
            guard,
            lazy,
            redirect,
            span: Span {
                start: start.start,
                end: self.peek().span.start,
                line: start.line,
                col: start.col,
            },
        })
    }

    /// Parse `@layout "/prefix" ;` (spec §2.6). The `@layout` token has been
    /// consumed by the component dispatch loop.
    pub(super) fn parse_layout_decl(&mut self) -> Result<LayoutDecl, ParseError> {
        let start = self.peek_span();
        self.advance(); // '@layout'
        let prefix = match self.advance().kind {
            TokenKind::StringLiteral(s) => s,
            other => {
                return Err(ParseError {
                    kind: None,
                    message: format!("@layout 路径必须是字符串字面量，got {:?}", other),
                    span: self.peek_span(),
                })
            }
        };
        // V2: prefix must be non-empty and start with '/'.
        if prefix.is_empty() || !prefix.starts_with('/') {
            return Err(ParseError {
                kind: None,
                message: format!("路由路径必须以 `/` 开头且非空：'{prefix}'"),
                span: start,
            });
        }
        // Layout prefixes may contain `:param` segments too — validate them
        // with the same V5 rule (a layout with params matches its children).
        parse_path_params(&prefix, start)?;
        if *self.peek_kind() == TokenKind::Semicolon {
            self.advance();
        }
        Ok(LayoutDecl {
            prefix,
            span: Span {
                start: start.start,
                end: self.peek().span.start,
                line: start.line,
                col: start.col,
            },
        })
    }

    /// Parse `@query IDENT : TYPE (= expr)? ;` (spec §2.4). The `@query` token
    /// has been consumed by the component dispatch loop.
    pub(super) fn parse_query_decl(&mut self) -> Result<QueryDecl, ParseError> {
        let start = self.peek_span();
        self.advance(); // '@query'
        let name = self.expect_ident("expected query parameter name")?;
        self.expect_token(TokenKind::Colon, "expected ':' after query parameter name")?;
        let type_ann = self.parse_type_annotation()?;
        let default = if *self.peek_kind() == TokenKind::Eq {
            self.advance();
            Some(self.parse_expr()?)
        } else {
            None
        };
        if *self.peek_kind() == TokenKind::Semicolon {
            self.advance();
        }
        Ok(QueryDecl {
            name,
            type_ann,
            default,
            span: Span {
                start: start.start,
                end: self.peek().span.start,
                line: start.line,
                col: start.col,
            },
        })
    }
}

/// Split a route path into its `:param` segments, validating each segment
/// name (V5 — 解析错误): a parameter name must start with a letter or
/// underscore and contain only letters/digits/underscores. `:123`, `::id`
/// and `:id-2` are all illegal. Static segments are not validated (the
/// route library's path grammar owns their legality).
fn parse_path_params(path: &str, span: Span) -> Result<Vec<String>, ParseError> {
    let mut params = Vec::new();
    // path starts with '/', so the first split item is the empty pre-slash
    // part; every following item is one path segment.
    for seg in path.split('/').skip(1) {
        if let Some(name) = seg.strip_prefix(':') {
            let valid = !name.is_empty()
                && name.starts_with(|c: char| c.is_alphabetic() || c == '_')
                && name.chars().all(|c| c.is_alphanumeric() || c == '_');
            if !valid {
                return Err(ParseError {
                    kind: None,
                    message: format!(
                        "路径参数段 ':{name}' 非法——参数名为字母/下划线开头，如 `:id`"
                    ),
                    span,
                });
            }
            params.push(name.to_string());
        }
    }
    Ok(params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;

    fn parse_route_decl_src(src: &str) -> Result<RouteDecl, ParseError> {
        let tokens = tokenize(src);
        let mut parser = Parser::new(tokens, src);
        parser.parse_route_decl() // consumes '@route' itself
    }

    fn parse_layout_decl_src(src: &str) -> Result<LayoutDecl, ParseError> {
        let tokens = tokenize(src);
        let mut parser = Parser::new(tokens, src);
        parser.parse_layout_decl() // consumes '@layout' itself
    }

    fn parse_query_decl_src(src: &str) -> Result<QueryDecl, ParseError> {
        let tokens = tokenize(src);
        let mut parser = Parser::new(tokens, src);
        parser.parse_query_decl() // consumes '@query' itself
    }

    #[test]
    fn test_parse_route_decl_static_path() {
        let decl = parse_route_decl_src(r#"@route "/todos";"#).unwrap();
        assert_eq!(decl.path, "/todos");
        assert!(decl.params.is_empty());
        assert!(decl.guard.is_none());
        assert!(decl.lazy.is_none());
        assert!(decl.redirect.is_none());
    }

    #[test]
    fn test_parse_route_decl_params_and_attrs() {
        let decl = parse_route_decl_src(
            r#"@route "/todos/:id" guard: isAdmin lazy: true redirect: "/login";"#,
        )
        .unwrap();
        assert_eq!(decl.path, "/todos/:id");
        assert_eq!(decl.params, vec!["id"]);
        assert_eq!(decl.guard.as_deref(), Some("isAdmin"));
        assert_eq!(decl.lazy, Some(true));
        assert_eq!(decl.redirect.as_deref(), Some("/login"));
    }

    #[test]
    fn test_parse_route_decl_multiple_params() {
        let decl = parse_route_decl_src(r#"@route "/users/:org/repos/:repo";"#).unwrap();
        assert_eq!(decl.params, vec!["org", "repo"]);
    }

    #[test]
    fn test_parse_layout_decl() {
        let decl = parse_layout_decl_src(r#"@layout "/admin";"#).unwrap();
        assert_eq!(decl.prefix, "/admin");
    }

    #[test]
    fn test_parse_query_decl_with_default() {
        let decl = parse_query_decl_src(r#"@query filter: String = "all";"#).unwrap();
        assert_eq!(decl.name, "filter");
        assert!(matches!(
            decl.type_ann,
            TypeAnnotation::Named { ref name, .. } if name == "String"
        ));
        assert!(decl.default.is_some());
    }

    #[test]
    fn test_parse_query_decl_no_default() {
        let decl = parse_query_decl_src(r#"@query q: Int;"#).unwrap();
        assert_eq!(decl.name, "q");
        assert!(decl.default.is_none());
    }

    #[test]
    fn test_parse_route_bad_path() {
        // V2: empty or non-'/' path
        let err = parse_route_decl_src(r#"@route "todos";"#).unwrap_err();
        assert!(
            err.message.contains("必须以 `/` 开头且非空"),
            "got: {}",
            err.message
        );
        let err = parse_route_decl_src(r#"@route "";"#).unwrap_err();
        assert!(
            err.message.contains("必须以 `/` 开头且非空"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn test_parse_route_bad_param_segment() {
        // V5: illegal param segment names
        for bad in [":123", "::id", ":id-2"] {
            let src = format!(r#"@route "/a/{bad}";"#);
            let err = parse_route_decl_src(&src).unwrap_err();
            assert!(
                err.message.contains("参数段") && err.message.contains("非法"),
                "path {bad:?} must fail V5, got: {}",
                err.message
            );
        }
    }

    #[test]
    fn test_parse_route_unknown_attr() {
        let err = parse_route_decl_src(r#"@route "/a" bogus: true;"#).unwrap_err();
        assert!(
            err.message.contains("未知 @route 属性"),
            "got: {}",
            err.message
        );
    }

    #[test]
    fn test_parse_route_bad_lazy_value() {
        let err = parse_route_decl_src(r#"@route "/a" lazy: 42;"#).unwrap_err();
        assert!(
            err.message.contains("lazy 属性必须是 true/false"),
            "got: {}",
            err.message
        );
    }

    /// 组件顶层解析：`@route` 在组件块内被接受（V1 位置合法面）。
    #[test]
    fn test_parse_route_in_component() {
        let src = r#"component App {
  @route "/todos";
  view {
    <div></div>
  }
}
"#;
        let (program, errors) = parse(tokenize(src), src);
        assert!(errors.is_empty(), "parse errors: {:?}", errors);
        let module = program.modules.get("input.omni").unwrap();
        assert_eq!(module.components.len(), 1);
        let comp = &module.components[0];
        assert_eq!(comp.route_decls.len(), 1);
        assert_eq!(comp.route_decls[0].path, "/todos");
    }

    /// V1 负例：`@route` 出现在文件顶层 → 解析错误（只能声明在组件顶层块内）。
    #[test]
    fn test_parse_route_outside_component_top_level() {
        let src = r#"@route "/todos";
"#;
        let (_, errors) = parse(tokenize(src), src);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("只能声明在组件顶层块内")),
            "got: {:?}",
            errors
        );
    }

    /// V1 负例：`@route` 出现在视图内 → 解析错误。
    #[test]
    fn test_parse_route_outside_component_in_view() {
        let src = r#"component App {
  view {
    @route "/todos";
    <div></div>
  }
}
"#;
        let (_, errors) = parse(tokenize(src), src);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("只能声明在组件顶层块内")),
            "got: {:?}",
            errors
        );
    }
}
