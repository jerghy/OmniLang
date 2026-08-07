//! Element property parsing helpers for OmniView components.
//!
//! Extracted from `mod.rs` to keep that file below 800 lines.

use crate::error::{DiagnosticKind, ParseError};
use crate::lexer::{Span, TokenKind};
use crate::parser::ast::*;
use crate::parser::stmt::keyword_as_ident_name;
use crate::parser::Parser;

impl<'a> Parser<'a> {
    pub(super) fn check_closing_tag(&self, tag: &str) -> bool {
        let tokens = &self.tokens;
        let pos = self.pos;
        if pos + 2 >= tokens.len() {
            return false;
        }
        matches!(tokens[pos].kind, TokenKind::Lt)
            && matches!(tokens[pos + 1].kind, TokenKind::Slash)
            && matches!(&tokens[pos + 2].kind, TokenKind::Ident(t) if t == tag)
    }

    /// Check if the token at `pos` looks like the start of an attribute
    /// (identifier followed by '=' or ':'), rather than another class name.
    pub(super) fn peek_is_attr_start(&self) -> bool {
        if self.pos + 1 >= self.tokens.len() {
            return false;
        }
        matches!(
            self.tokens[self.pos + 1].kind,
            TokenKind::Eq | TokenKind::Colon
        )
    }

    /// Check if the next token is a known boolean HTML attribute keyword.
    /// Used to prevent these from being treated as class names in
    /// `class: btn primary disabled` scenarios.
    pub(super) fn peek_is_boolean_attr_keyword(&self) -> bool {
        let name: Option<String> = match self.peek_kind() {
            TokenKind::Ident(s) => Some(s.clone()),
            kind => keyword_as_ident_name(kind).map(|s| s.to_string()),
        };
        matches!(
            name.as_deref(),
            Some("controls")
                | Some("disabled")
                | Some("readonly")
                | Some("required")
                | Some("checked")
                | Some("selected")
                | Some("autofocus")
                | Some("multiple")
                | Some("loop")
                | Some("muted")
                | Some("async")
                | Some("defer")
                | Some("hidden")
                | Some("autocomplete")
                | Some("autoplay")
        )
    }

    /// Parse a static prop: name="value" or name={expr}
    /// Also handles hyphenated names like margin-left, background-color
    pub(super) fn parse_element_prop(&mut self) -> Result<ElementProp, ParseError> {
        let mut name = self.expect_ident("expected prop name")?;

        // Handle hyphenated attribute names: margin-left, background-color, etc.
        while self.at(&TokenKind::Minus) {
            self.advance(); // consume '-'
            let next = self.expect_ident("expected identifier after '-'")?;
            name = format!("{}-{}", name, next);
        }

        // Check for semantic binding attributes (focus, scroll-x, scroll-y).
        // These are compiler-recognized ref bindings, not generic static
        // props. Per the semantic-level DSL spec they bind a bare signal:
        // name={sig}. The string-literal form (value="text") and bare
        // boolean form stay generic Static props, and so does a computed
        // expression (value={age.to_string()}) — only a bare VarRef is a
        // semantic binding.
        // v4 compiler HIGH-4: `value` is no longer an unconditional semantic
        // binding — only when the element also declares @ref (v1 双向形状)
        // does it reclassify; otherwise value={expr} is a plain one-way prop
        // (design contract D2's value_prop production). "value" was removed
        // from SEMANTIC_BINDINGS; the @ref discriminator runs as a
        // post-processing pass after element props finish parsing
        // (parse_element in view/mod.rs).
        const SEMANTIC_BINDINGS: &[&str] = &["focus", "scroll-x", "scroll-y"];
        if SEMANTIC_BINDINGS.contains(&name.as_str())
            && self.at(&TokenKind::Eq)
            && matches!(
                self.tokens.get(self.pos + 1).map(|t| &t.kind),
                Some(TokenKind::LBrace)
            )
        {
            self.advance(); // '='
            self.advance(); // '{'
                            // A slot named like a semantic binding (e.g. <Modal value={...}/>)
                            // with view content is a slot projection, not a binding — the `<`
                            // discriminator applies here too.
            if self.at(&TokenKind::Lt) {
                let content = self.parse_slot_content_braces()?;
                return Ok(ElementProp::SlotContent { name, content });
            }
            let signal = self.parse_expr()?;
            self.expect_token(TokenKind::RBrace, "expected '}'")?;
            // Only a bare signal reference ({sig}) or a qualified store signal
            // ({Store.sig}) is a compiler-recognized semantic binding —
            // store signals are referenced qualified and bare store signal
            // names are not in component scope. Computed expressions stay
            // generic Static props (equivalent to the {expr} arm of the
            // match below).
            let is_bare_signal = matches!(&signal, Expr::VarRef(..));
            let is_qualified_store_signal = matches!(
                &signal,
                Expr::FieldAccess { object, .. } if matches!(object.as_ref(), Expr::VarRef(..))
            );
            if is_bare_signal || is_qualified_store_signal {
                return Ok(ElementProp::RefBinding {
                    binding: name,
                    signal,
                });
            }
            return Ok(ElementProp::Static {
                name,
                value: signal,
            });
        }

        let kind = self.peek_kind().clone();
        match kind {
            TokenKind::Eq => {
                self.advance();
                let kind = self.peek_kind().clone();
                match kind {
                    TokenKind::StringLiteral(_) => {
                        let t = self.advance();
                        if let TokenKind::StringLiteral(s) = &t.kind {
                            // D4 (v2): `slot="name"` consumption is deleted —
                            // the only consumer form is the named-props
                            // projection `name={<.../>}` (design doc 域 4
                            // diagnostic table). Hard-error instead of
                            // silently projecting at codegen time.
                            if name == "slot" {
                                return Err(ParseError::with_kind(
                                    crate::error::DiagnosticKind::ESlotRemoved,
                                    "改用命名 props：name={...}",
                                    t.span,
                                ));
                            }
                            Ok(ElementProp::Static {
                                name,
                                value: Expr::Literal(Literal::String(s.clone()), t.span),
                            })
                        } else {
                            // Should not reach here — outer match confirmed StringLiteral
                            Err(ParseError {
                                kind: None,
                                message: "expected string literal after '='".into(),
                                span: t.span,
                            })
                        }
                    }
                    TokenKind::LBrace => {
                        self.advance();
                        // Slot content projection (ADR-053 D5): name={<div/>}.
                        // The `<` after `{` discriminates slot content from a
                        // regular {expr} prop value — `<` is not a valid
                        // expression prefix, so there is no ambiguity.
                        if self.at(&TokenKind::Lt) {
                            let content = self.parse_slot_content_braces()?;
                            Ok(ElementProp::SlotContent { name, content })
                        } else {
                            let expr = self.parse_expr()?;
                            self.expect_token(TokenKind::RBrace, "expected '}'")?;
                            Ok(ElementProp::Static { name, value: expr })
                        }
                    }
                    _ => {
                        let t = self.advance();
                        Err(ParseError {
                            kind: None,
                            message: format!(
                                "Expected string or {{expr}} for prop value, got {:?}",
                                t.kind
                            ),
                            span: t.span,
                        })
                    }
                }
            }
            TokenKind::Colon => {
                // ref: name — element reference (not a class name)
                if name == "ref" {
                    self.advance(); // consume ':'
                    let ref_name = self.expect_ident("expected ref name")?;
                    return Ok(ElementProp::Ref(ref_name));
                }
                // states: { state1: { ... }, state2: { ... }, ... } — the
                // multi-state animation map was removed with the semantic DSL
                // (ADR-053): motion intent is now @motion + triggers. This
                // used to swallow the body and emit a "__states_removed"
                // sentinel trigger nobody consumed — a silent parse-and-
                // ignore. Hard error with the migration hint instead (never
                // silently drop intent).
                if name == "states" {
                    let colon_span = self.peek_span();
                    return Err(ParseError::with_kind(
                        crate::error::DiagnosticKind::EStatesRemoved,
                        "states: 已删除（ADR-053），用 @motion + 触发器替代",
                        colon_span,
                    ));
                }
                // on: { signal } — the signal-driven states binding (removed
                // with `states:` above). Same silent-sentinel history: hard
                // error with the @onXxx={...} migration hint. (Without this,
                // `on: { e => ... }` misreported as "箭头仅用于事件内联".)
                if name == "on" {
                    let colon_span = self.peek_span();
                    return Err(ParseError::with_kind(
                        crate::error::DiagnosticKind::EOnRemoved,
                        "on: 已删除，用 @onXxx={...}",
                        colon_span,
                    ));
                }
                // a11y: { label: string, role: string, hidden: expr,
                //        live: "polite"|"assertive", alt: expr }
                // Semantic accessibility block (design doc 域 10) — an
                // element attribute block mirroring the style: { ... } shape.
                if name == "a11y" {
                    self.advance(); // consume ':'
                    self.expect_token(TokenKind::LBrace, "expected '{' after 'a11y:'")?;
                    let mut spec = A11ySpec::default();
                    if self.at(&TokenKind::RBrace) {
                        let t = self.peek();
                        return Err(ParseError {
                            kind: None,
                            message: "a11y 块不能为空".into(),
                            span: t.span,
                        });
                    }
                    while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
                        let key = self.expect_ident("expected a11y key")?;
                        self.expect_token(TokenKind::Colon, "expected ':' after a11y key")?;
                        match key.as_str() {
                            // BNF: 'label' ':' string — string literals only.
                            // (The web targets emit them as static attributes;
                            // dynamic accessible names are future work.)
                            "label" | "role" | "live" => {
                                let t = self.advance();
                                let value = match &t.kind {
                                    TokenKind::StringLiteral(s) => s.clone(),
                                    _ => {
                                        return Err(ParseError {
                                            kind: None,
                                            message: format!(
                                                "a11y 键 '{key}' 需要字符串字面量，得到 {:?}",
                                                t.kind
                                            ),
                                            span: t.span,
                                        })
                                    }
                                };
                                match key.as_str() {
                                    "label" => spec.label = Some(value),
                                    "role" => spec.role = Some(value),
                                    _ => spec.live = Some(value),
                                }
                            }
                            // BNF: 'hidden' ':' expr / 'alt' ':' expr — dynamic.
                            "hidden" => {
                                spec.hidden = Some(self.parse_expr()?);
                            }
                            "alt" => {
                                spec.alt = Some(self.parse_expr()?);
                            }
                            other => {
                                let t = self.peek();
                                return Err(ParseError {
                                    kind: None,
                                    message: format!(
                                        "未知 a11y 键：'{other}'（支持 label/role/hidden/live/alt）"
                                    ),
                                    span: t.span,
                                });
                            }
                        }
                        if self.at(&TokenKind::Comma) {
                            self.advance();
                        }
                    }
                    self.expect_token(TokenKind::RBrace, "expected '}'")?;
                    return Ok(ElementProp::A11y(spec));
                }
                // variant: DesignName.VariantName(dim=val, ...)
                if name == "variant" {
                    self.advance(); // consume ':'
                    let design_name = self.expect_ident("expected design name")?;
                    self.expect_token(TokenKind::Dot, "expected '.' after design name")?;
                    let variant_name = self.expect_ident("expected variant name")?;

                    // Parse optional dimension value pairs: (dim=val, dim=val, ...)
                    let mut dimension_values = Vec::new();
                    if self.at(&TokenKind::LParen) {
                        self.advance(); // consume '('
                        while !self.at(&TokenKind::RParen) && !self.at(&TokenKind::Eof) {
                            let dim = self.expect_ident("expected dimension name")?;
                            self.expect_token(TokenKind::Eq, "expected '='")?;
                            let val = self.expect_ident("expected dimension value")?;
                            dimension_values.push((dim, val));
                            if self.at(&TokenKind::Comma) {
                                self.advance();
                            }
                        }
                        self.expect_token(TokenKind::RParen, "expected ')'")?;
                    }

                    return Ok(ElementProp::Variant {
                        design_name,
                        variant_name,
                        dimension_values,
                    });
                }
                // style: entries or class: name
                self.advance();
                match self.peek_kind() {
                    TokenKind::LBrace if name == "class" => {
                        // class: { key: expr, ... } — dynamic class map
                        self.advance(); // consume '{'
                        let mut entries = Vec::new();
                        if !self.at(&TokenKind::RBrace) {
                            let key = self.expect_ident("expected class key")?;
                            self.expect_token(TokenKind::Colon, "expected ':'")?;
                            let val = self.parse_expr()?;
                            entries.push((key, val));
                            while self.at(&TokenKind::Comma) {
                                self.advance();
                                let key = self.expect_ident("expected class key")?;
                                self.expect_token(TokenKind::Colon, "expected ':'")?;
                                let val = self.parse_expr()?;
                                entries.push((key, val));
                            }
                        }
                        self.expect_token(TokenKind::RBrace, "expected '}'")?;
                        Ok(ElementProp::Class {
                            value: ClassValue::ClassMap(entries),
                        })
                    }
                    TokenKind::LBrace => {
                        self.advance();
                        let mut entries = Vec::new();
                        if !self.at(&TokenKind::RBrace) {
                            let key = self.parse_css_property_name("expected style key")?;
                            self.expect_token(TokenKind::Colon, "expected ':'")?;
                            let val = self.parse_expr()?;
                            entries.push((key, val));
                            while self.at(&TokenKind::Comma) {
                                self.advance();
                                let key = self.parse_css_property_name("expected style key")?;
                                self.expect_token(TokenKind::Colon, "expected ':'")?;
                                let val = self.parse_expr()?;
                                entries.push((key, val));
                            }
                        }
                        self.expect_token(TokenKind::RBrace, "expected '}'")?;
                        Ok(ElementProp::Style { entries })
                    }
                    _ => {
                        let t = self.advance();
                        let first = match &t.kind {
                            TokenKind::StringLiteral(s) => s.clone(),
                            TokenKind::Ident(s) => {
                                let mut class_name = s.clone();
                                // Handle hyphenated class names: theme-bar, btn-add, demo1-r-0, etc.
                                // Also accept IntLiteral after '-' (CSS allows numbers after hyphens: -0, -1, etc.)
                                while self.at(&TokenKind::Minus) {
                                    self.advance(); // consume '-'
                                    let next: String = match self.peek_kind().clone() {
                                        TokenKind::Ident(_) => self.expect_ident("")?,
                                        TokenKind::IntLiteral(n) => {
                                            self.advance();
                                            n.to_string()
                                        }
                                        ref k => {
                                            // Accept keywords (view, style, theme, etc.) after '-'
                                            if let Some(name) = keyword_as_ident_name(k) {
                                                self.advance();
                                                name.to_string()
                                            } else {
                                                return Err(ParseError { kind: None,
                                                    message: "expected identifier or number after '-' in class name".into(),
                                                    span: self.peek_span(),
                                                });
                                            }
                                        }
                                    };
                                    class_name = format!("{}-{}", class_name, next);
                                }
                                class_name
                            }
                            ref k => {
                                // Accept keywords that can be used as identifiers (e.g., theme, view, style)
                                if let Some(name) = keyword_as_ident_name(k) {
                                    let mut class_name = name.to_string();
                                    while self.at(&TokenKind::Minus) {
                                        self.advance(); // consume '-'
                                        let kind = self.peek_kind().clone();
                                        let next: String = match kind {
                                            TokenKind::Ident(_) => self.expect_ident("")?,
                                            TokenKind::IntLiteral(n) => {
                                                self.advance();
                                                n.to_string()
                                            }
                                            ref k => {
                                                if let Some(name) = keyword_as_ident_name(k) {
                                                    self.advance();
                                                    name.to_string()
                                                } else {
                                                    return Err(ParseError { kind: None,
                                                        message: "expected identifier or number after '-' in class name".into(),
                                                        span: self.peek_span(),
                                                    });
                                                }
                                            }
                                        };
                                        class_name = format!("{}-{}", class_name, next);
                                    }
                                    class_name
                                } else {
                                    return Err(ParseError {
                                        kind: None,
                                        message: format!("Expected class name, got {:?}", k),
                                        span: t.span,
                                    });
                                }
                            }
                        };
                        // Check for design class reference: DesignName.ClassName
                        // Detect dot-notation in the first class name
                        if self.at(&TokenKind::Dot) {
                            self.advance(); // consume '.'
                            let class_name = self.expect_ident("expected class name after '.'")?;
                            return Ok(ElementProp::Class {
                                value: ClassValue::DesignClass {
                                    design_name: first,
                                    class_name,
                                },
                            });
                        }
                        // Collect additional space-separated class names: class: btn btn-plus
                        // BUT stop if the next identifier is a known boolean HTML attribute
                        // (e.g. <audio class: media controls> — "controls" is an attribute)
                        let mut classes = vec![first];
                        loop {
                            // Peek — is the next token another class name?
                            let next_is_class = match self.peek_kind() {
                                TokenKind::Ident(_) => true,
                                kind => keyword_as_ident_name(kind).is_some(),
                            };
                            if !next_is_class {
                                break;
                            }
                            // Check: is this actually the start of the next attribute?
                            // An ident followed by '=' or ':' is an attribute, not a class name
                            if self.peek_is_attr_start() {
                                break;
                            }
                            // Check: is this a known boolean attribute keyword?
                            // (e.g., controls, disabled, readonly, loop, muted, etc.)
                            if self.peek_is_boolean_attr_keyword() {
                                break;
                            }
                            let next_t = self.advance();
                            match &next_t.kind {
                                TokenKind::Ident(s) => {
                                    let mut cn = s.clone();
                                    while self.at(&TokenKind::Minus) {
                                        self.advance();
                                        let n = self.expect_ident(
                                            "expected identifier after '-' in class name",
                                        )?;
                                        cn = format!("{}-{}", cn, n);
                                    }
                                    classes.push(cn);
                                }
                                k => {
                                    if let Some(name) = keyword_as_ident_name(k) {
                                        let mut cn = name.to_string();
                                        while self.at(&TokenKind::Minus) {
                                            self.advance();
                                            let n = self.expect_ident(
                                                "expected identifier after '-' in class name",
                                            )?;
                                            cn = format!("{}-{}", cn, n);
                                        }
                                        classes.push(cn);
                                    } else {
                                        // Shouldn't happen — we already checked
                                        break;
                                    }
                                }
                            }
                        }
                        // Check for dynamic class expression: class: name { expr }
                        if self.at(&TokenKind::LBrace) {
                            // Parse class: name(s) { conditional_expr }
                            // e.g., class: tab-btn {if activeDemo == "position" { "active" } else { "" }}
                            // Combine static names with dynamic expression result
                            let span = self.peek_span();
                            let static_part =
                                Expr::Literal(Literal::String(classes.join(" ")), span);
                            self.advance(); // consume '{'
                            let dyn_expr = self.parse_expr()?;
                            self.expect_token(TokenKind::RBrace, "expected '}'")?;
                            let combined = Expr::BinaryOp {
                                left: Box::new(static_part),
                                op: BinaryOpKind::Add,
                                right: Box::new(dyn_expr),
                                span,
                            };
                            // Emit as a Static className prop so codegen handles it naturally
                            return Ok(ElementProp::Static {
                                name: "className".into(),
                                value: combined,
                            });
                        }

                        Ok(ElementProp::Class {
                            value: ClassValue::StyleRef(classes.join(" ")),
                        })
                    }
                }
            }
            _ => {
                // Boolean attribute: just the name, e.g., <input disabled>
                Ok(ElementProp::Static {
                    name,
                    value: Expr::Literal(
                        Literal::Bool(true),
                        Span {
                            start: 0,
                            end: 0,
                            line: 0,
                            col: 0,
                        },
                    ),
                })
            }
        }
    }

    /// Parse slot projection content inside `{ ... }` (ADR-053 D5 consumer
    /// named props): one or more view nodes (elements, directives,
    /// interpolations) until the matching `}`. The caller has already
    /// consumed the opening `{`; `<` at the call site discriminates slot
    /// content from a regular `{expr}` prop value.
    pub(super) fn parse_slot_content_braces(&mut self) -> Result<Vec<ViewNode>, ParseError> {
        let mut content = Vec::new();
        while !self.at(&TokenKind::RBrace) && !self.at(&TokenKind::Eof) {
            content.push(self.parse_view_node()?);
        }
        if content.is_empty() {
            let t = self.peek();
            return Err(ParseError {
                kind: None,
                message: format!(
                    "expected slot content (e.g. '<div>...</div>') after '{{', got {:?}",
                    t.kind
                ),
                span: t.span,
            });
        }
        self.expect_token(TokenKind::RBrace, "expected '}'")?;
        Ok(content)
    }

    /// Parse bind:property={signal} for two-way binding.
    pub(super) fn parse_bind_prop(&mut self) -> Result<ElementProp, ParseError> {
        self.advance(); // 'bind'
        self.expect_token(TokenKind::Colon, "expected ':'")?;
        let name = self.expect_ident("expected bind property name")?;

        // Parse ={signal_path}
        let signal_path = if self.at(&TokenKind::Eq) {
            self.advance(); // '='
            let kind = self.peek_kind().clone();
            match kind {
                TokenKind::LBrace => {
                    self.advance();
                    let path = self.expect_ident("expected signal path")?;
                    // Handle nested paths like "store.field"
                    let path = if self.at(&TokenKind::Dot) {
                        self.advance();
                        let field = self.expect_ident("expected field name")?;
                        format!("{}.{}", path, field)
                    } else {
                        path
                    };
                    self.expect_token(TokenKind::RBrace, "expected '}'")?;
                    path
                }
                TokenKind::Ident(id) => {
                    self.advance();
                    id.clone()
                }
                _ => {
                    return Err(ParseError {
                        kind: None,
                        message: "Expected {signalPath} or signalName for bind value".into(),
                        span: self.peek_span(),
                    })
                }
            }
        } else {
            return Err(ParseError {
                kind: None,
                message: "Expected '=' after bind:property".into(),
                span: self.peek_span(),
            });
        };

        Ok(ElementProp::Bind { name, signal_path })
    }

    /// Parse @eventName={handler} for event binding.
    pub(super) fn parse_event_prop(&mut self) -> Result<ElementProp, ParseError> {
        let t = self.advance();
        let event_name = match &t.kind {
            TokenKind::At => {
                // Traditional: @ is separate token
                self.expect_ident("expected event name")?
            }
            TokenKind::Ident(name) if name.starts_with('@') => {
                // Lexer combined: @onPress as single Ident token
                name[1..].to_string() // strip '@' prefix
            }
            _ => {
                return Err(ParseError {
                    kind: None,
                    message: format!("Expected @event binding, got {:?}", t.kind),
                    span: t.span,
                })
            }
        };
        // @ref={name} is a ref binding, not an event
        if event_name == "ref" {
            let handler = self.parse_event_handler_value()?;
            let ref_name = match &handler {
                EventHandler::FnRef(n) => n.clone(),
                _ => {
                    return Err(ParseError {
                        kind: None,
                        message: format!("Expected ref name, got {:?}", handler),
                        span: t.span,
                    })
                }
            };
            return Ok(ElementProp::Ref(ref_name));
        }
        // @paused was deleted in the semantic DSL (ADR-053): it used to parse
        // as a generic event binding with no runtime effect. v2 (Task 5.3)
        // hard-errors with a migration hint — pause/resume via @if conditional
        // rendering instead (design doc 域 2 诊断表).
        if event_name == "paused" {
            return Err(ParseError::with_kind(
                DiagnosticKind::EPausedRemoved,
                "paused 已删除，用 @if 条件渲染替代",
                t.span,
            ));
        }

        // @drag / @dragEnd — draggable-intent directive (ADR-073). Name-based
        // dispatch BEFORE the generic event path: the directive is NOT a DOM
        // event — it is an intent the compiler wires per target
        // (capturePointer is an internal detail, ADR-071 decision 3). The
        // value must be a bare component-function name. @dragEnd alone
        // (without @drag) hard-errors in the caller's merge step
        // (`push_element_prop`), which keeps the two attributes
        // order-independent on the same element.
        if event_name == "drag" || event_name == "dragEnd" {
            let handler = self.parse_event_handler_value()?;
            let handler_name = match &handler {
                EventHandler::FnRef(n) => n.clone(),
                _ => {
                    return Err(ParseError {
                        kind: None,
                        message: "@drag/@dragEnd 值必须是组件函数名（如 @drag={onDragMove}）"
                            .into(),
                        span: t.span,
                    })
                }
            };
            let is_drag = event_name == "drag";
            return Ok(ElementProp::Drag {
                on_drag: if is_drag {
                    handler_name.clone()
                } else {
                    String::new()
                },
                on_drag_end: if is_drag { None } else { Some(handler_name) },
            });
        }

        let handler = self.parse_event_handler_value()?;
        Ok(ElementProp::Event {
            name: event_name,
            handler,
        })
    }

    /// Parse onEvent={handler} (no @ prefix) — event name is already known.
    pub(super) fn parse_event_prop_no_at(
        &mut self,
        event_name: &str,
    ) -> Result<ElementProp, ParseError> {
        // Consume the event name ident token
        self.advance();
        let handler = self.parse_event_handler_value()?;
        Ok(ElementProp::Event {
            name: event_name.to_string(),
            handler,
        })
    }

    /// Push an element prop, merging `@drag`/`@dragEnd` pairs (ADR-073).
    ///
    /// The two draggable-intent attributes are order-independent on the same
    /// element: `parse_event_prop` emits `@dragEnd` with an empty `on_drag`
    /// placeholder, and the merge here backfills the already-seen `@drag`
    /// (or errors if `@dragEnd` stands alone, or either attribute repeats).
    pub(super) fn push_element_prop(
        &mut self,
        props: &mut Vec<ElementProp>,
        prop: ElementProp,
    ) -> Result<(), ParseError> {
        let ElementProp::Drag {
            on_drag,
            on_drag_end,
        } = &prop
        else {
            props.push(prop);
            return Ok(());
        };
        let Some(existing) = props
            .iter_mut()
            .find(|p| matches!(p, ElementProp::Drag { .. }))
        else {
            // First @drag/@dragEnd of the element — push verbatim. A lone
            // @dragEnd placeholder (empty on_drag) is rejected at the end of
            // the prop loop (finalize in mod.rs), because @drag may
            // legitimately come later on the element.
            props.push(prop);
            return Ok(());
        };
        let ElementProp::Drag {
            on_drag: existing_on_drag,
            on_drag_end: existing_end,
        } = existing
        else {
            unreachable!("find matched ElementProp::Drag");
        };
        if on_drag.is_empty() {
            // New prop is @dragEnd — the @drag part must already be present
            // (a placeholder existing alongside means two @dragEnds).
            if existing_on_drag.is_empty() || existing_end.is_some() {
                return Err(ParseError {
                    kind: None,
                    message: "duplicate @dragEnd on the same element".into(),
                    span: self.peek_span(),
                });
            }
            *existing_end = on_drag_end.clone();
        } else {
            // New prop is @drag — backfill into the @dragEnd placeholder
            // (order-independent pair, ADR-073).
            if !existing_on_drag.is_empty() {
                return Err(ParseError {
                    kind: None,
                    message: "duplicate @drag on the same element".into(),
                    span: self.peek_span(),
                });
            }
            *existing_on_drag = on_drag.clone();
        }
        Ok(())
    }

    /// Parse the ={handler} part of an event binding.
    pub(super) fn parse_event_handler_value(&mut self) -> Result<EventHandler, ParseError> {
        if !self.at(&TokenKind::Eq) {
            return Err(ParseError {
                kind: None,
                message: "Expected '=' after event name".into(),
                span: self.peek_span(),
            });
        }
        self.advance(); // '='
        match self.peek_kind() {
            TokenKind::LBrace => {
                self.advance();
                let handler = if self.at(&TokenKind::LParen) {
                    // Inline fn: (params) => { body }
                    self.advance();
                    let params = self.parse_params()?;
                    self.expect_token(TokenKind::RParen, "expected ')'")?;
                    self.expect_token(TokenKind::FatArrow, "expected '=>'")?;
                    let body = self.parse_block()?;
                    EventHandler::InlineFn { params, body }
                } else {
                    // Handler expression: fnName, fnName(args), store.action,
                    // assignment (dragStatus = "dragging"), or any expression.
                    // Parse as expression; wrap non-simple cases in inline fn.
                    let expr = self.parse_expr()?;
                    match &expr {
                        Expr::VarRef(name, _) => EventHandler::FnRef(name.clone()),
                        _ => {
                            // Wrap any other expression (fn call, assignment, etc.)
                            // in an inline fn body: () => { expr; }
                            let expr_span = *crate::parser::expr::span_of(&expr);
                            let body = Block {
                                stmts: vec![Stmt::ExprStmt {
                                    expr,
                                    span: expr_span,
                                }],
                                trailing_expr: None,
                                span: expr_span,
                            };
                            EventHandler::InlineFn {
                                params: vec![],
                                body,
                            }
                        }
                    }
                };
                self.expect_token(TokenKind::RBrace, "expected '}'")?;
                Ok(handler)
            }
            _ => {
                let t = self.advance();
                Err(ParseError {
                    kind: None,
                    message: format!("Expected {{handler}} after '=', got {:?}", t.kind),
                    span: t.span,
                })
            }
        }
    }
}

#[cfg(test)]
mod legacy_states_on_tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;

    /// Parse a source and return the collected parse diagnostics.
    fn parse_and_diag(source: &str) -> Vec<ParseError> {
        let tokens = tokenize(source);
        let (_program, errors) = parse(tokens, source);
        errors
    }

    #[test]
    fn legacy_states_syntax_hard_errors() {
        // domain HIGH-1: `states:` used to swallow its body and emit the
        // "__states_removed" sentinel (a fake MotionTrigger nobody consumed)
        // — zero diagnostics. It must now hard-error with a migration hint.
        let src = r#"component App { view { <div states: { hover: { opacity: 0.5 } } /> } }"#;
        let diags = parse_and_diag(src);
        let d = diags
            .iter()
            .find(|d| d.message.contains("states"))
            .expect("must diagnose");
        assert!(
            d.message.contains("迁移") || d.message.contains("替代"),
            "missing migration hint: {}",
            d.message
        );
    }

    #[test]
    fn legacy_on_block_hard_errors() {
        // domain HIGH-1: `on:` used to be silently dropped (or misreported as
        // "箭头仅用于事件内联" when the body contained an arrow). It must now
        // hard-error with the @onXxx={...} migration hint.
        let src = r#"component App { view { <div on: { x => y } /> } }"#;
        let diags = parse_and_diag(src);
        assert!(
            diags
                .iter()
                .any(|d| d.message.contains("on:") || d.message.contains("@onXxx")),
            "must diagnose on:"
        );
    }
}

/// Parser tests for the `@drag` / `@dragEnd` draggable-intent directive
/// (ADR-073).
#[cfg(test)]
mod drag_directive_tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;

    /// Parse a component source and return its view children.
    fn view_children(source: &str) -> Vec<ViewNode> {
        let tokens = tokenize(source);
        let (program, errors) = parse(tokens, source);
        assert!(errors.is_empty(), "unexpected parse errors: {:?}", errors);
        program.modules.get("input.omni").unwrap().components[0]
            .view
            .as_ref()
            .expect("component should have view")
            .children
            .clone()
    }

    /// Find the Drag prop of the first element child.
    fn drag_prop(nodes: &[ViewNode]) -> Option<&ElementProp> {
        nodes.iter().find_map(|n| match n {
            ViewNode::Element(el) => el
                .props
                .iter()
                .find(|p| matches!(p, ElementProp::Drag { .. })),
            _ => None,
        })
    }

    #[test]
    fn parses_drag_directive() {
        let src = r#"
component Drag {
  ref box: Element
  fn move(e) { }
  fn end(e) { }
  view { <div @ref={box} @drag={move} @dragEnd={end} /> }
}
"#;
        let children = view_children(src);
        let prop = drag_prop(&children).expect("element must carry a Drag prop");
        match prop {
            ElementProp::Drag {
                on_drag,
                on_drag_end,
            } => {
                assert_eq!(on_drag, "move");
                assert_eq!(on_drag_end.as_deref(), Some("end"));
            }
            other => panic!("expected ElementProp::Drag, got: {:?}", other),
        }
    }

    #[test]
    fn drag_end_before_drag_is_order_independent() {
        // @dragEnd may appear before @drag on the same element — the merge
        // happens after the prop loop (ADR-073).
        let src = "component Drag { ref box: Element; fn move(e) { } fn end(e) { } \
                   view { <div @dragEnd={end} @drag={move} @ref={box} /> } }";
        let children = view_children(src);
        let prop = drag_prop(&children).expect("element must carry a Drag prop");
        match prop {
            ElementProp::Drag {
                on_drag,
                on_drag_end,
            } => {
                assert_eq!(on_drag, "move");
                assert_eq!(on_drag_end.as_deref(), Some("end"));
            }
            other => panic!("expected ElementProp::Drag, got: {:?}", other),
        }
    }

    #[test]
    fn drag_end_without_drag_errors() {
        let src = "component Drag { fn end(e) { } view { <div @dragEnd={end} /> } }";
        let tokens = tokenize(src);
        let (_program, errors) = parse(tokens, src);
        assert!(
            errors
                .iter()
                .any(|d| d.message.contains("@dragEnd requires @drag")),
            "expected '@dragEnd requires @drag' error, got: {:?}",
            errors
        );
    }

    #[test]
    fn duplicate_drag_errors() {
        let src = "component Drag { fn move(e) { } view { <div @drag={move} @drag={move} /> } }";
        let tokens = tokenize(src);
        let (_program, errors) = parse(tokens, src);
        assert!(
            errors.iter().any(|d| d.message.contains("duplicate @drag")),
            "expected duplicate @drag error, got: {:?}",
            errors
        );
    }

    #[test]
    fn drag_value_must_be_bare_function_name() {
        // Inline arrows / calls are rejected — @drag takes a bare fn name
        // (the compiler wires the call, ADR-073).
        let src = "component Drag { fn move(e) { } view { <div @drag={move(e)} /> } }";
        let tokens = tokenize(src);
        let (_program, errors) = parse(tokens, src);
        assert!(
            errors
                .iter()
                .any(|d| d.message.contains("必须是组件函数名")),
            "expected bare-fn-name error, got: {:?}",
            errors
        );
    }

    #[test]
    fn drag_does_not_collide_with_motion_drag_triggers() {
        // @drag-horizontal / @drag-vertical are motion triggers — @drag must
        // not shadow them (lexer keeps them as dedicated tokens).
        let src = "@motion fade { from { opacity: 0 } to { opacity: 1 } } \
                   component App { view { <div @drag-horizontal ={fade} @drag={move} /> } \
                   fn move(e) { } }";
        let tokens = tokenize(src);
        let (_program, errors) = parse(tokens, src);
        assert!(
            errors.is_empty(),
            "motion drag triggers + @drag must coexist, got: {:?}",
            errors
        );
    }
}
