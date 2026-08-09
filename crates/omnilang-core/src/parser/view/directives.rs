//! View directive parsers: @if/@for and motion triggers.

use crate::error::ParseError;
use crate::lexer::TokenKind;
use crate::parser::ast::*;
use crate::parser::Parser;

use super::style::{is_text_token, token_text};

impl<'a> Parser<'a> {
    // ── Directives ─────────────────────────────────────

    pub(super) fn parse_if_directive(&mut self) -> Result<ViewNode, ParseError> {
        self.advance(); // '@if'
        let condition = self.parse_expr()?;
        // Parse optional mode: "fade"|"wait"|"none" after condition, before {
        let transition_mode = if self.peek_ident_value().is_some_and(|s| s == "mode") {
            self.advance(); // 'mode'
            self.expect_token(TokenKind::Colon, "expected ':' after 'mode'")?;
            // Accept both string literals ("fade", "wait") and idents (fade, wait)
            let mode_val = if matches!(*self.peek_kind(), TokenKind::StringLiteral(_)) {
                self.expect_string("expected mode value")?
            } else {
                self.expect_ident("expected mode value (fade, wait, none)")?
            };
            // Validate: only "fade", "wait", "none" are valid
            if !matches!(mode_val.as_str(), "fade" | "wait" | "none") {
                return Err(self.error(format!(
                    "invalid transition_mode '{}' — use 'fade', 'wait', or 'none'",
                    mode_val
                )));
            }
            Some(mode_val)
        } else {
            None
        };
        self.expect_token(TokenKind::LBrace, "expected '{'")?;

        let mut body = Vec::new();
        while *self.peek_kind() != TokenKind::RBrace && *self.peek_kind() != TokenKind::Eof {
            body.push(self.parse_view_node()?);
        }
        self.expect_token(TokenKind::RBrace, "expected '}'")?;

        let mut else_ifs = Vec::new();
        let mut else_body = None;

        while *self.peek_kind() == TokenKind::AtElse {
            self.advance();
            // Handle both `@else @if` and `@else if` patterns
            if *self.peek_kind() == TokenKind::AtIf {
                self.advance(); // '@if' after '@else'
                let cond = self.parse_expr()?;
                self.expect_token(TokenKind::LBrace, "expected '{'")?;
                let mut b = Vec::new();
                while *self.peek_kind() != TokenKind::RBrace && *self.peek_kind() != TokenKind::Eof
                {
                    b.push(self.parse_view_node()?);
                }
                self.expect_token(TokenKind::RBrace, "expected '}'")?;
                else_ifs.push((cond, b));
            } else if *self.peek_kind() == TokenKind::If {
                self.advance(); // 'if' after '@else' (without @ prefix)
                let cond = self.parse_expr()?;
                self.expect_token(TokenKind::LBrace, "expected '{'")?;
                let mut b = Vec::new();
                while *self.peek_kind() != TokenKind::RBrace && *self.peek_kind() != TokenKind::Eof
                {
                    b.push(self.parse_view_node()?);
                }
                self.expect_token(TokenKind::RBrace, "expected '}'")?;
                else_ifs.push((cond, b));
            } else {
                self.expect_token(TokenKind::LBrace, "expected '{'")?;
                let mut b = Vec::new();
                while *self.peek_kind() != TokenKind::RBrace && *self.peek_kind() != TokenKind::Eof
                {
                    b.push(self.parse_view_node()?);
                }
                self.expect_token(TokenKind::RBrace, "expected '}'")?;
                else_body = Some(b);
                break;
            }
        }

        let if_span = self.peek_span();
        Ok(ViewNode::IfDirective(IfDirective {
            condition,
            body,
            else_ifs,
            else_body,
            transition_mode,
            span: if_span,
        }))
    }

    /// Data three-state intent: `@load={fnName}` (spec 2026-08-08 §4.1).
    /// Strict shape — `@load` `=` `{` IDENT `}` — mirrors the @if fallback:
    /// on any parse failure the caller restores position and re-collects
    /// the text so `@load` appearing in prose keeps working.
    pub(super) fn parse_load_directive(&mut self) -> Result<ViewNode, ParseError> {
        let span = self.peek_span();
        self.advance(); // '@load'
        self.expect_token(TokenKind::Eq, "'@load' 后缺少 '='")?;
        self.expect_token(TokenKind::LBrace, "'=' 后缺少 '{'")?;
        let fn_name = self.expect_ident("@load={...} 内缺少函数名")?;
        self.expect_token(TokenKind::RBrace, "@load 函数名后缺少 '}'")?;
        Ok(ViewNode::LoadDirective { fn_name, span })
    }

    /// Modal dialog intent: `@modal={openSig}` on a `dialog` element (spec
    /// 2026-08-09 §2.1). Strict shape — `@modal` `=` `{` IDENT `}` —
    /// mirrors the `@load` directive precedent: the value must be a bare
    /// identifier reference (the user-declared boolean open signal), not an
    /// arbitrary expression. Signal existence/type checks are semantic-layer
    /// (V1/V4/V5).
    pub(super) fn parse_modal_directive(&mut self) -> Result<ElementProp, ParseError> {
        self.advance(); // '@modal'
        self.expect_token(TokenKind::Eq, "'@modal' 后缺少 '='")?;
        self.expect_token(TokenKind::LBrace, "'=' 后缺少 '{'")?;
        let signal_span = self.peek_span();
        let signal_name = self.expect_ident("@modal={...} 内缺少信号名")?;
        self.expect_token(TokenKind::RBrace, "@modal 信号名后缺少 '}'")?;
        Ok(ElementProp::Modal {
            open_signal: Expr::VarRef(signal_name, signal_span),
        })
    }

    /// Skeleton placeholder intent: bare `@skeleton` or `@skeleton="shape"`
    /// (spec 2026-08-09 §2.3). The shape value, when present, must be a
    /// string literal at parse time (mirroring `@skeleton="text"`);
    /// shape-name legality (text/line/circle/card) is a semantic-layer
    /// check (V10), as is the hard constraint that `@skeleton` only appears
    /// inside a `@load` loading branch (V9).
    pub(super) fn parse_skeleton_directive(&mut self) -> Result<ElementProp, ParseError> {
        self.advance(); // '@skeleton'
        if !matches!(self.peek_kind(), TokenKind::Eq) {
            return Ok(ElementProp::Skeleton { shape: None });
        }
        self.advance(); // '='
        let t = self.advance();
        match t.kind {
            TokenKind::StringLiteral(shape) => Ok(ElementProp::Skeleton { shape: Some(shape) }),
            other => Err(ParseError {
                kind: None,
                message: format!(
                    "'@skeleton' 的值必须是字符串——合法形状: text/line/circle/card，但得到 {:?}",
                    other
                ),
                span: t.span,
            }),
        }
    }

    /// Render-failure fallback intent: bare `@errorBoundary` (spec
    /// 2026-08-09 §2.4). Marks an element subtree whose render failures
    /// fall back to the body-level `@slot fallback`; a value is not allowed
    /// (same bare-attribute rule as `@form`).
    pub(super) fn parse_error_boundary_directive(&mut self) -> Result<ElementProp, ParseError> {
        let t = self.advance(); // '@errorBoundary'
        if matches!(self.peek_kind(), TokenKind::Eq) {
            return Err(ParseError {
                kind: None,
                message: "'@errorBoundary' 不接受值——使用裸属性标记（spec 2026-08-09 §2.4）".into(),
                span: t.span,
            });
        }
        Ok(ElementProp::ErrorBoundary)
    }

    pub(super) fn parse_for_directive(&mut self) -> Result<ViewNode, ParseError> {
        self.advance(); // '@for'
        let item_name = self.expect_ident("expected item variable name")?;
        self.expect_token(TokenKind::In, "expected 'in'")?;
        let mut iterable = self.parse_expr()?;

        // Disambiguate `@for item in list [scroll] {` (scroll hint) from
        // `list[scroll]` (index access): parse_expr greedily treats any '['
        // as IndexGet. A bracket separated by whitespace from the iterable
        // whose index is the identifier `scroll` is reinterpreted as the
        // scroll hint (spec §7.2: scroll_hint ::= '[' 'scroll' ']').
        let mut scroll_hint = matches!(
            &iterable,
            Expr::IndexGet { object, index, span }
                if matches!(&**index, Expr::VarRef(name, _) if name == "scroll")
                    && span.start > crate::parser::expr::span_of(object).end
        );
        if scroll_hint {
            iterable = match iterable {
                Expr::IndexGet { object, .. } => *object,
                other => other,
            };
        }

        // Optional key={expr} for identity tracking
        let key_expr = if self.peek_ident_value().as_deref() == Some("key") {
            self.advance(); // 'key'
            self.expect_token(TokenKind::Eq, "expected '='")?;
            self.expect_token(TokenKind::LBrace, "expected '{'")?;
            let expr = self.parse_expr()?;
            self.expect_token(TokenKind::RBrace, "expected '}'")?;
            Some(expr)
        } else {
            None
        };

        // Optional [scroll] hint after key={...} (BNF: key_expr? scroll_hint?)
        if *self.peek_kind() == TokenKind::LBracket {
            self.advance(); // '['
            let hint = self.expect_ident("expected 'scroll'")?;
            if hint != "scroll" {
                return Err(self.error("expected 'scroll' inside brackets, e.g. [scroll]"));
            }
            self.expect_token(TokenKind::RBracket, "expected ']'")?;
            scroll_hint = true;
        }

        self.expect_token(TokenKind::LBrace, "expected '{'")?;

        let mut body = Vec::new();
        while *self.peek_kind() != TokenKind::RBrace && *self.peek_kind() != TokenKind::Eof {
            body.push(self.parse_view_node()?);
        }
        self.expect_token(TokenKind::RBrace, "expected '}'")?;

        let mut empty_body = None;
        if *self.peek_kind() == TokenKind::AtEmpty {
            self.advance();
            self.expect_token(TokenKind::LBrace, "expected '{'")?;
            let mut b = Vec::new();
            while *self.peek_kind() != TokenKind::RBrace && *self.peek_kind() != TokenKind::Eof {
                b.push(self.parse_view_node()?);
            }
            self.expect_token(TokenKind::RBrace, "expected '}'")?;
            empty_body = Some(b);
        }

        let for_span = self.peek_span();
        Ok(ViewNode::ForDirective(ForDirective {
            item_name,
            iterable,
            body,
            empty_body,
            key_expr,
            scroll_hint,
            span: for_span,
        }))
    }

    // ── Motion Trigger Parsing ──────────────────────────────

    /// Parse a motion trigger: `@trigger [sub-params] ={motionExpr}`.
    ///
    /// `@scroll` / `@drag-horizontal` / `@drag-vertical` accept the v1
    /// control-dimension sub-parameters between the trigger keyword and the
    /// `={motion}` binding (v2 BNF):
    ///
    /// ```text
    /// @scroll start: 0 end: 1000 scrub: true ={fadeScroll}
    /// @drag-horizontal constraints: [-100, 100] elastic: 0.5 ={swipeCard}
    /// ```
    ///
    /// Unknown or malformed sub-parameters are hard errors — never parsed
    /// and ignored.
    pub(super) fn parse_motion_trigger(
        &mut self,
        kind: TriggerKind,
    ) -> Result<MotionTrigger, ParseError> {
        self.advance(); // consume the @enter/@hover/etc. token
        let kind = self.parse_trigger_sub_params(kind)?;
        self.expect_token(TokenKind::Eq, "expected '=' after motion trigger")?;
        self.expect_token(TokenKind::LBrace, "expected '{'")?;
        let motion_expr = self.parse_motion_expr()?;
        self.expect_token(TokenKind::RBrace, "expected '}'")?;
        Ok(MotionTrigger {
            trigger: kind,
            motion_expr,
        })
    }

    /// Parse the colon-form control dimensions of `@scroll` / `@drag-*`
    /// triggers (v1 syntax, restored by the v2 BNF). No sub-parameters are
    /// allowed on the other five triggers. Unknown keys and malformed values
    /// are hard parse errors (no parse-and-ignore).
    fn parse_trigger_sub_params(&mut self, kind: TriggerKind) -> Result<TriggerKind, ParseError> {
        let is_scroll = matches!(kind, TriggerKind::Scroll { .. });
        let is_drag = matches!(
            kind,
            TriggerKind::DragHorizontal { .. } | TriggerKind::DragVertical { .. }
        );
        if !is_scroll && !is_drag {
            return Ok(kind);
        }
        let (mut start, mut end, mut scrub) = (None, None, None);
        let (mut constraints, mut elastic, mut threshold) = (None, None, None);
        let (mut horizontal, mut vertical) = (false, false);
        match kind {
            TriggerKind::Scroll {
                start: s,
                end: e,
                scrub: sc,
            } => {
                start = s;
                end = e;
                scrub = sc;
            }
            TriggerKind::DragHorizontal {
                constraints: c,
                elastic: el,
                threshold: th,
            } => {
                constraints = c;
                elastic = el;
                threshold = th;
                horizontal = true;
            }
            TriggerKind::DragVertical {
                constraints: c,
                elastic: el,
                threshold: th,
            } => {
                constraints = c;
                elastic = el;
                threshold = th;
                vertical = true;
            }
            _ => return Ok(kind),
        }
        let allowed: &[&str] = if is_scroll {
            &["start", "end", "scrub"]
        } else {
            &["constraints", "elastic", "threshold"]
        };
        loop {
            match self.peek_ident_value() {
                Some(ref s) if is_scroll && s == "start" => {
                    self.advance();
                    self.expect_token(TokenKind::Colon, "expected ':' after 'start'")?;
                    start = Some(self.parse_expr_no_assign()?);
                }
                Some(ref s) if is_scroll && s == "end" => {
                    self.advance();
                    self.expect_token(TokenKind::Colon, "expected ':' after 'end'")?;
                    end = Some(self.parse_expr_no_assign()?);
                }
                Some(ref s) if is_scroll && s == "scrub" => {
                    self.advance();
                    self.expect_token(TokenKind::Colon, "expected ':' after 'scrub'")?;
                    scrub = Some(self.parse_expr_no_assign()?);
                }
                Some(ref s) if is_drag && s == "constraints" => {
                    self.advance();
                    self.expect_token(TokenKind::Colon, "expected ':' after 'constraints'")?;
                    self.expect_token(TokenKind::LBracket, "expected '[' for constraints range")?;
                    let min = self.parse_expr_no_assign()?;
                    self.expect_token(TokenKind::Comma, "expected ',' between constraints bounds")?;
                    let max = self.parse_expr_no_assign()?;
                    self.expect_token(TokenKind::RBracket, "expected ']' after constraints range")?;
                    constraints = Some((min, max));
                }
                Some(ref s) if is_drag && s == "elastic" => {
                    self.advance();
                    self.expect_token(TokenKind::Colon, "expected ':' after 'elastic'")?;
                    elastic = Some(self.parse_expr_no_assign()?);
                }
                Some(ref s) if is_drag && s == "threshold" => {
                    self.advance();
                    self.expect_token(TokenKind::Colon, "expected ':' after 'threshold'")?;
                    threshold = Some(self.parse_expr_no_assign()?);
                }
                Some(other) => {
                    return Err(self.error(format!(
                        "unknown parameter '{}' on @scroll/@drag trigger; expected {}",
                        other,
                        allowed.join(" | ")
                    )))
                }
                None => break,
            }
        }
        Ok(if is_scroll {
            TriggerKind::Scroll { start, end, scrub }
        } else if horizontal {
            TriggerKind::DragHorizontal {
                constraints,
                elastic,
                threshold,
            }
        } else {
            debug_assert!(vertical);
            TriggerKind::DragVertical {
                constraints,
                elastic,
                threshold,
            }
        })
    }

    /// Collect an @-keyword that failed to parse as a real directive into a text node.
    /// Used when @for/@if appear in element text content rather than as directives.
    pub(super) fn collect_at_text(&mut self, keyword: &str) -> ViewNode {
        let mut text = String::from(keyword);
        self.advance(); // consume the @-keyword token
        while self.pos < self.tokens.len() {
            let kind = self.peek_kind();
            match kind {
                TokenKind::Lt | TokenKind::Gt | TokenKind::Eof => break,
                TokenKind::LBrace => {
                    text.push('{');
                    self.advance();
                }
                TokenKind::RBrace => {
                    text.push('}');
                    self.advance();
                }
                _ if is_text_token(kind) => {
                    // Insert space if there's a gap indicating whitespace
                    text.push_str(&token_text(kind));
                    self.advance();
                }
                _ => break,
            }
        }
        ViewNode::Text(text)
    }
}

#[cfg(test)]
mod trigger_params_tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;

    /// Parse a view source with one element carrying motion triggers and
    /// return the element's MotionTrigger props.
    fn parse_triggers(source: &str) -> Vec<MotionTrigger> {
        let tokens = tokenize(source);
        let (program, errors) = parse(tokens, source);
        assert!(
            errors.is_empty(),
            "unexpected parse errors: {:?}",
            errors.iter().map(|e| e.message.clone()).collect::<Vec<_>>()
        );
        let module = program.modules.get("input.omni").unwrap();
        let comp = &module.components[0];
        let view = comp.view.as_ref().expect("component has view");
        let mut out = Vec::new();
        for node in &view.children {
            if let ViewNode::Element(el) = node {
                for prop in &el.props {
                    if let ElementProp::MotionTrigger(mt) = prop {
                        out.push(mt.clone());
                    }
                }
            }
        }
        assert!(!out.is_empty(), "no motion triggers found in: {}", source);
        out
    }

    #[test]
    fn scroll_trigger_with_v1_sub_params_parses() {
        // v1 design doc §3.6 syntax, restored by the v2 BNF:
        //   scroll_params ::= 'start:' expr 'end:' expr 'scrub:' bool
        let triggers = parse_triggers(
            "@motion fadeScroll { from { opacity: 0 } to { opacity: 1 } }
component C {
  view { <div @scroll start: 0 end: 1000 scrub: true ={fadeScroll}>x</div> }
}",
        );
        let TriggerKind::Scroll { start, end, scrub } = &triggers[0].trigger else {
            panic!("expected Scroll trigger");
        };
        assert!(matches!(start, Some(Expr::Literal(Literal::Int(0), _))));
        assert!(matches!(end, Some(Expr::Literal(Literal::Int(1000), _))));
        assert!(matches!(scrub, Some(Expr::Literal(Literal::Bool(true), _))));
        assert!(matches!(
            triggers[0].motion_expr,
            Expr::VarRef(ref name, _) if name == "fadeScroll"
        ));
    }

    #[test]
    fn scroll_trigger_without_sub_params_still_parses() {
        let triggers = parse_triggers(
            "@motion fadeScroll { from { opacity: 0 } to { opacity: 1 } }
component C {
  view { <div @scroll ={fadeScroll}>x</div> }
}",
        );
        let TriggerKind::Scroll { start, end, scrub } = &triggers[0].trigger else {
            panic!("expected Scroll trigger");
        };
        assert!(start.is_none() && end.is_none() && scrub.is_none());
    }

    #[test]
    fn drag_trigger_with_v1_sub_params_parses() {
        // v1 design doc §3.7 syntax, restored by the v2 BNF:
        //   drag_params ::= 'constraints:' '[' expr ',' expr ']' 'elastic:' expr
        //                   'threshold:' expr
        let triggers = parse_triggers(
            "@motion swipeCard { from { rotate: 0 } to { rotate: 30 } }
component C {
  view { <div @drag-horizontal constraints: [-100, 100] elastic: 0.5 threshold: 0.3 ={swipeCard}>x</div> }
}",
        );
        let TriggerKind::DragHorizontal {
            constraints,
            elastic,
            threshold,
        } = &triggers[0].trigger
        else {
            panic!("expected DragHorizontal trigger");
        };
        let (min, max) = constraints.as_ref().expect("constraints parsed");
        assert!(matches!(min, Expr::UnaryOp { .. }));
        assert!(matches!(max, Expr::Literal(Literal::Int(100), _)));
        assert!(matches!(
            elastic,
            Some(Expr::Literal(Literal::Float(f), _)) if *f == 0.5
        ));
        assert!(matches!(
            threshold,
            Some(Expr::Literal(Literal::Float(f), _)) if *f == 0.3
        ));
    }

    #[test]
    fn drag_vertical_sub_params_parse() {
        let triggers = parse_triggers(
            "@motion m { from { opacity: 0 } to { opacity: 1 } }
component C {
  view { <div @drag-vertical constraints: [0, 300] ={m}>x</div> }
}",
        );
        let TriggerKind::DragVertical { constraints, .. } = &triggers[0].trigger else {
            panic!("expected DragVertical trigger");
        };
        assert!(constraints.is_some());
    }

    #[test]
    fn unknown_trigger_sub_param_is_hard_error() {
        // No parse-and-ignore: an unknown sub-parameter is a hard parse error.
        let source = "@motion m { from { opacity: 0 } to { opacity: 1 } }
component C {
  view { <div @scroll start: 0 bogus: 1 ={m}>x</div> }
}";
        let tokens = tokenize(source);
        let (_program, errors) = parse(tokens, source);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("unknown parameter 'bogus'")),
            "expected unknown-parameter error, got: {:?}",
            errors.iter().map(|e| e.message.clone()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn velocity_max_is_removed_syntax() {
        // v4 pragmatic MEDIUM-1: velocity_max 零消费者全链路实现 + React
        // slice 特例排除——移除后书写它必须产生「未知 drag 参数」诊断
        // （迁移提示指向删除事实）。
        let source = "@motion m { from { opacity: 0 } to { opacity: 1 } }
component C {
  view { <div @drag-horizontal velocity_max: 100 ={m}>x</div> }
}";
        let tokens = tokenize(source);
        let (_program, errors) = parse(tokens, source);
        assert!(
            errors.iter().any(|e| e.message.contains("velocity_max")),
            "velocity_max must be a removed-syntax diagnostic (unknown drag parameter), got: {:?}",
            errors.iter().map(|e| e.message.clone()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn scroll_key_on_drag_trigger_is_hard_error() {
        // `start` belongs to @scroll, not @drag-* — wrong-kind keys error.
        let source = "@motion m { from { opacity: 0 } to { opacity: 1 } }
component C {
  view { <div @drag-horizontal start: 0 ={m}>x</div> }
}";
        let tokens = tokenize(source);
        let (_program, errors) = parse(tokens, source);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("unknown parameter 'start'")),
            "expected wrong-kind parameter error, got: {:?}",
            errors.iter().map(|e| e.message.clone()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn malformed_constraints_bracket_is_hard_error() {
        // constraints: [1, 2, 3] — trailing element is a hard error.
        let source = "@motion m { from { opacity: 0 } to { opacity: 1 } }
component C {
  view { <div @drag-horizontal constraints: [1, 2, 3] ={m}>x</div> }
}";
        let tokens = tokenize(source);
        let (_program, errors) = parse(tokens, source);
        assert!(
            !errors.is_empty(),
            "malformed constraints must not parse, got no errors"
        );
    }
}

#[cfg(test)]
mod batch1_feedback_modal_tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;

    /// Parse a one-component view and return the element props of the first
    /// (non-text) element child.
    fn parse_first_element_props(source: &str) -> Vec<ElementProp> {
        let tokens = tokenize(source);
        let (program, errors) = parse(tokens, source);
        assert!(
            errors.is_empty(),
            "unexpected parse errors: {:?}",
            errors.iter().map(|e| e.message.clone()).collect::<Vec<_>>()
        );
        let module = program.modules.get("input.omni").unwrap();
        let comp = &module.components[0];
        let view = comp.view.as_ref().expect("component has view");
        for node in &view.children {
            if let ViewNode::Element(el) = node {
                return el.props.clone();
            }
        }
        panic!("no element found in: {}", source);
    }

    /// Parse a one-component module and return the expression-statement
    /// exprs of the first function body (showToast call sites).
    fn parse_first_fn_expr_stmts(source: &str) -> Vec<Expr> {
        let tokens = tokenize(source);
        let (program, errors) = parse(tokens, source);
        assert!(
            errors.is_empty(),
            "unexpected parse errors: {:?}",
            errors.iter().map(|e| e.message.clone()).collect::<Vec<_>>()
        );
        let module = program.modules.get("input.omni").unwrap();
        let comp = &module.components[0];
        let mut out = Vec::new();
        for stmt in &comp.actions[0].body.stmts {
            if let Stmt::ExprStmt { expr, .. } = stmt {
                out.push(expr.clone());
            }
        }
        // The final statement of a block is promoted to `trailing_expr`
        // (stmt.rs parse_block) — include it so the last showToast call
        // site is collected too.
        if let Some(ref e) = comp.actions[0].body.trailing_expr {
            out.push(e.as_ref().clone());
        }
        assert!(!out.is_empty(), "no expr statements found in: {}", source);
        out
    }

    /// Parse a one-component module and return the collected parse-error
    /// messages (negative-path tests — hard errors must be surfaced by the
    /// parser, not silently recovered).
    fn parse_error_messages(source: &str) -> Vec<String> {
        let tokens = tokenize(source);
        let (_program, errors) = parse(tokens, source);
        errors.into_iter().map(|e| e.message).collect()
    }

    #[test]
    fn error_boundary_with_value_is_hard_error() {
        let msgs = parse_error_messages(
            "component C {
  view { <div @errorBoundary={x} /> }
}",
        );
        assert!(
            msgs.iter().any(|m| m.contains("@errorBoundary' 不接受值")),
            "expected @errorBoundary value rejection, got: {:?}",
            msgs
        );
    }

    #[test]
    fn skeleton_non_string_value_is_hard_error() {
        let msgs = parse_error_messages(
            "component C {
  view { <div @skeleton=42 /> }
}",
        );
        assert!(
            msgs.iter()
                .any(|m| m.contains("@skeleton' 的值必须是字符串")),
            "expected @skeleton non-string rejection, got: {:?}",
            msgs
        );
    }

    #[test]
    fn modal_missing_shape_parts_are_hard_errors() {
        for (src, expected) in [
            (
                "component C {
  view { <div @modal isOpen /> }
}",
                "'@modal' 后缺少 '='",
            ),
            (
                "component C {
  view { <dialog @modal={} /> }
}",
                "缺少信号名",
            ),
            (
                "component C {
  view { <dialog @modal={isOpen /> }
}",
                "缺少 '}'",
            ),
        ] {
            let msgs = parse_error_messages(src);
            assert!(
                msgs.iter().any(|m| m.contains(expected)),
                "expected {:?} for {:?}, got: {:?}",
                expected,
                src,
                msgs
            );
        }
    }

    #[test]
    fn show_toast_unknown_and_duplicate_option_keys_are_hard_errors() {
        for (src, expected) in [
            (
                "component C {
  action save() {
    showToast(\"x\", { tone: \"error\" })
  }
  view { <div>x</div> }
}",
                "未知 showToast 选项 \"tone\"",
            ),
            (
                "component C {
  action save() {
    showToast(\"x\", { kind: \"error\", kind: \"info\" })
  }
  view { <div>x</div> }
}",
                "重复声明",
            ),
        ] {
            let msgs = parse_error_messages(src);
            assert!(
                msgs.iter().any(|m| m.contains(expected)),
                "expected {:?} for {:?}, got: {:?}",
                expected,
                src,
                msgs
            );
        }
    }

    #[test]
    fn show_toast_unclosed_options_is_hard_error() {
        // Options `{` opened but never closed — EOF and ')' fallbacks must
        // report the missing '}', not a bogus option-key error.
        for src in [
            // True EOF fallback: the source ends while the options object
            // is still open (no trailing '}' anywhere).
            "component C {
  action save() {
    showToast(\"x\", { kind: \"error\"",
            // ')' fallback: options open, then the call's closing paren
            // arrives without the '}' — the options object is unclosed.
            "component C {
  action save() {
    showToast(\"x\", { kind: \"error\" )
  }
  view { <div>x</div> }
}",
        ] {
            let msgs = parse_error_messages(src);
            assert!(
                msgs.iter().any(|m| m.contains("选项对象未闭合")),
                "expected unclosed-options error for {:?}, got: {:?}",
                src,
                msgs
            );
        }
    }

    #[test]
    fn parse_modal_dialog_shape() {
        // spec 2026-08-09 §2.1: ModalDirective := '@modal' '=' '{' Ident '}'
        let props = parse_first_element_props(
            "component C {
  view { <dialog @modal={isOpen}><div>x</div></dialog> }
}",
        );
        let modal = props
            .iter()
            .find_map(|p| match p {
                ElementProp::Modal { open_signal } => Some(open_signal),
                _ => None,
            })
            .expect("expected @modal prop");
        assert!(
            matches!(modal, Expr::VarRef(ref name, _) if name == "isOpen"),
            "open_signal must be the identifier ref isOpen, got: {:?}",
            modal
        );
    }

    #[test]
    fn parse_skeleton_directive_shapes() {
        // spec 2026-08-09 §2.3: SkeletonDirective := '@skeleton'
        // | '@skeleton' '=' StringLiteral
        let props = parse_first_element_props(
            "component C {
  view { <div @skeleton /> }
}",
        );
        assert!(
            props
                .iter()
                .any(|p| matches!(p, ElementProp::Skeleton { shape: None })),
            "bare @skeleton must parse with no shape, got: {:?}",
            props
        );

        let props = parse_first_element_props(
            "component C {
  view { <div @skeleton=\"card\" /> }
}",
        );
        assert!(
            props.iter().any(|p| matches!(
                p,
                ElementProp::Skeleton {
                    shape: Some(ref s)
                } if s == "card"
            )),
            "@skeleton=\"card\" must parse shape Some(\"card\"), got: {:?}",
            props
        );
    }

    #[test]
    fn parse_error_boundary_directive() {
        // spec 2026-08-09 §2.4: ErrorBoundaryDirective := '@errorBoundary'
        // (bare only); fallback slot flows through the view-level @slot.
        let props = parse_first_element_props(
            "component C {
  view {
    <div @errorBoundary>
      @slot fallback { <Text>加载失败</Text> }
      <UserProfile />
    </div>
  }
}",
        );
        assert!(
            props
                .iter()
                .any(|p| matches!(p, ElementProp::ErrorBoundary)),
            "expected @errorBoundary prop, got: {:?}",
            props
        );
    }

    #[test]
    fn parse_show_toast_builtin_call() {
        // spec 2026-08-09 §2.2: ShowToastCall := 'showToast' '(' Expr ','?
        // ToastOptions? ')' — parse layer captures structurally only.
        let exprs = parse_first_fn_expr_stmts(
            "component C {
  action save() {
    showToast(\"保存成功\")
    showToast(\"网络不可用\", { kind: \"error\", duration: 0 })
  }
  view { <div>x</div> }
}",
        );
        let Expr::BuiltinToastCall {
            msg,
            kind,
            duration,
            ..
        } = &exprs[0]
        else {
            panic!("expected BuiltinToastCall, got: {:?}", exprs[0]);
        };
        assert!(matches!(
            msg.as_ref(),
            Expr::Literal(Literal::String(ref s), _) if s == "保存成功"
        ));
        assert!(kind.is_none() && duration.is_none(), "default options");

        let Expr::BuiltinToastCall {
            msg,
            kind,
            duration,
            ..
        } = &exprs[1]
        else {
            panic!("expected BuiltinToastCall, got: {:?}", exprs[1]);
        };
        assert!(matches!(
            msg.as_ref(),
            Expr::Literal(Literal::String(ref s), _) if s == "网络不可用"
        ));
        assert!(matches!(
            kind.as_deref(),
            Some(Expr::Literal(Literal::String(ref k), _)) if k == "error"
        ));
        assert!(matches!(
            duration.as_deref(),
            Some(Expr::Literal(Literal::Int(0), _))
        ));
    }
}
