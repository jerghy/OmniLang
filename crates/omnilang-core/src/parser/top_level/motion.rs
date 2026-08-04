use crate::error::{DiagnosticKind, ParseError};
use crate::lexer::{Span, TokenKind};
use crate::parser::ast::*;
use crate::parser::stmt::keyword_as_ident_name;
use crate::parser::Parser;

/// Whitelist of animation properties allowed in keyframe stops (semantic DSL).
///
/// This is a semantic set (v2, Task 5.2): every property's animatability has
/// been verified on all three targets — not a list of "what happens to be
/// implemented". `borderRadius`/`fontSize` joined in v2 (WAAPI native,
/// `BorderRadiusTween`/`TextStyle.lerp` on Flutter, runtime CSS transition
/// on OmniView). Properties outside the set are rejected at parse time.
const SEMANTIC_ANIM_PROPS: &[&str] = &[
    "opacity",
    "scale",
    "scaleX",
    "scaleY",
    "translateX",
    "translateY",
    "translateZ",
    "rotate",
    "rotateX",
    "rotateY",
    "rotateZ",
    "skewX",
    "skewY",
    "backgroundColor",
    "color",
    "width",
    "height",
    "borderRadius",
    "fontSize",
];

impl<'a> Parser<'a> {
    // ── Motion / Timeline block parsers ─────────────────

    /// Parse @motion NAME { body }
    pub fn parse_motion_def(&mut self) -> Result<MotionDef, ParseError> {
        let span_start = self.peek_span();
        self.expect_token(TokenKind::AtMotion, "expected '@motion'")?;
        let name = self.expect_ident("expected motion name")?;
        self.expect_token(TokenKind::LBrace, "expected '{'")?;
        let body = self.parse_motion_body()?;
        self.expect_token(TokenKind::RBrace, "expected '}'")?;
        Ok(MotionDef {
            name,
            body,
            span: span_start,
        })
    }

    fn parse_motion_body(&mut self) -> Result<MotionBody, ParseError> {
        // Check percentage tokens FIRST (e.g., "0%") before trying ident-based matching.
        // IntLiteral/FloatLiteral are not idents, so peek_ident_value() returns None for them.
        let is_percentage = matches!(
            self.peek_kind(),
            TokenKind::IntLiteral(_) | TokenKind::FloatLiteral(_)
        );
        if is_percentage {
            return Ok(MotionBody::Keyframe(self.parse_keyframe_body()?));
        }

        // First token determines mode.
        let first = self
            .peek_ident_value()
            .ok_or_else(|| self.error("expected motion body"))?;

        match first.as_str() {
            "from" | "to" => Ok(MotionBody::Keyframe(self.parse_keyframe_body()?)),
            // `paused` in block position (`@motion fadeIn { paused: true }`)
            // must hard-error with the same migration hint as the
            // keyframe-body map form (parse_playback_props) — not the generic
            // expected-from/to error (LOW-4). `paused` was deleted in the
            // semantic DSL (ADR-053); pause/resume via @if conditional
            // rendering instead.
            "paused" => Err(ParseError::with_kind(
                DiagnosticKind::EPausedRemoved,
                "paused 已删除，用 @if 条件渲染替代",
                self.peek_span(),
            )),
            _ => Err(self.error(format!(
                "expected 'from' or 'to' to start keyframe body, got '{}'",
                first
            ))),
        }
    }

    /// Parse keyframe body: from { ... } to { ... } with optional over/easing/playback
    fn parse_keyframe_body(&mut self) -> Result<KeyframeBody, ParseError> {
        let mut stops = Vec::new();

        // Parse first stop
        stops.push(self.parse_keyframe_stop()?);

        // Parse remaining stops while they start with "to", "from", or percentage
        loop {
            let is_percentage = matches!(
                self.peek_kind(),
                TokenKind::IntLiteral(_) | TokenKind::FloatLiteral(_)
            );
            match self.peek_ident_value() {
                Some(ref s) if s == "to" || s == "from" => {
                    stops.push(self.parse_keyframe_stop()?);
                }
                Some(ref s) if s.chars().all(|c| c.is_ascii_digit() || c == '.') => {
                    // Percentage stop: "50" means 50%
                    stops.push(self.parse_keyframe_stop()?);
                }
                _ if is_percentage => {
                    // IntLiteral/FloatLiteral percentage: e.g., "25", "75.5"
                    stops.push(self.parse_keyframe_stop()?);
                }
                _ => break,
            }
        }

        // Parse shared duration/easing
        let mut over = Expr::Literal(
            Literal::Float(0.3),
            Span {
                start: 0,
                end: 0,
                line: 0,
                col: 0,
            },
        );
        let mut easing = None;

        self.parse_shared_timing(&mut over, &mut easing)?;

        let playback = self.parse_playback_props()?;

        Ok(KeyframeBody {
            stops,
            over,
            easing,
            playback,
        })
    }

    /// Parse a single keyframe stop: "from { ... }", "to { ... }", or "50% { ... }"
    fn parse_keyframe_stop(&mut self) -> Result<KeyframeStop, ParseError> {
        let position = match self.peek_ident_value() {
            Some(ref s) if s == "from" => {
                self.advance();
                StopPosition::From
            }
            Some(ref s) if s == "to" => {
                self.advance();
                StopPosition::To
            }
            Some(ref s) if s.chars().all(|c| c.is_ascii_digit() || c == '.') => {
                let val: f64 = s.parse().unwrap_or(50.0);
                self.advance();
                StopPosition::Percent(val)
            }
            _ => {
                // Also accept IntLiteral/FloatLiteral for percentage (e.g., "0%", "50%")
                match self.peek_kind() {
                    TokenKind::IntLiteral(n) => {
                        let val = *n as f64;
                        self.advance();
                        // Consume optional % token after number (e.g., "0%")
                        while *self.peek_kind() == TokenKind::Percent {
                            self.advance();
                        }
                        StopPosition::Percent(val)
                    }
                    TokenKind::FloatLiteral(f) => {
                        let val = *f;
                        self.advance();
                        // Consume optional % token after number
                        while *self.peek_kind() == TokenKind::Percent {
                            self.advance();
                        }
                        StopPosition::Percent(val)
                    }
                    _ => return Err(self.error("expected 'from', 'to', or percentage")),
                }
            }
        };

        self.expect_token(TokenKind::LBrace, "expected '{'")?;
        let mut properties = Vec::new();
        while *self.peek_kind() != TokenKind::RBrace && *self.peek_kind() != TokenKind::Eof {
            let key = self.parse_semantic_anim_prop()?;
            self.expect_token(TokenKind::Colon, "expected ':'")?;
            let value = self.parse_expr()?;
            properties.push((key, value));
            if *self.peek_kind() == TokenKind::Comma {
                self.advance();
            }
        }
        self.expect_token(TokenKind::RBrace, "expected '}'")?;

        // Optional per-stop ease
        let ease = if self.peek_ident_value().is_some_and(|s| s == "ease") {
            self.advance();
            if *self.peek_kind() == TokenKind::Colon {
                self.advance();
            }
            Some(self.parse_easing()?)
        } else {
            None
        };

        Ok(KeyframeStop {
            position,
            properties,
            ease,
        })
    }

    /// Parse an animation property name against the semantic whitelist.
    ///
    /// CSS custom properties (`--*`) are allowed as passthrough for CSS-based
    /// targets only (ADR-053 D3).
    fn parse_semantic_anim_prop(&mut self) -> Result<String, ParseError> {
        if *self.peek_kind() == TokenKind::Minus
            && self
                .tokens
                .get(self.pos + 1)
                .is_some_and(|t| t.kind == TokenKind::Minus)
        {
            self.advance(); // eat first '-'
            self.advance(); // eat second '-'
            let mut name = String::from("--");
            loop {
                match self.peek_kind() {
                    TokenKind::Ident(s) => {
                        name.push_str(s);
                        self.advance();
                    }
                    TokenKind::Minus => {
                        name.push('-');
                        self.advance();
                    }
                    kind => {
                        if let Some(kw) = keyword_as_ident_name(kind) {
                            name.push_str(kw);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
            }
            return Ok(name);
        }

        let name = self.expect_ident("expected animation property name")?;
        if SEMANTIC_ANIM_PROPS.contains(&name.as_str()) {
            Ok(name)
        } else {
            Err(self.error(format!(
                "'{}' is not a recognized animation property. Use one of: {}",
                name,
                SEMANTIC_ANIM_PROPS.join(", ")
            )))
        }
    }

    /// Parse shared timing after keyframe stops:
    /// `over: 300ms`, `easing: ease-out` (colon separator, comma optional).
    fn parse_shared_timing(
        &mut self,
        over: &mut Expr,
        easing: &mut Option<EasingExpr>,
    ) -> Result<(), ParseError> {
        loop {
            match self.peek_ident_value() {
                Some(ref s) if s == "over" => {
                    self.advance(); // 'over'
                    self.expect_token(TokenKind::Colon, "expected ':' after 'over'")?;
                    *over = self.parse_duration_expr()?;
                }
                Some(ref s) if s == "easing" => {
                    self.advance(); // 'easing'
                    self.expect_token(TokenKind::Colon, "expected ':' after 'easing'")?;
                    *easing = Some(self.parse_easing()?);
                }
                _ => break,
            }
            if *self.peek_kind() == TokenKind::Comma {
                self.advance();
            }
        }
        Ok(())
    }

    /// Parse a duration value: "300ms", "1.5s", or a bare number (seconds default).
    fn parse_duration_expr(&mut self) -> Result<Expr, ParseError> {
        let t = self.advance();
        match &t.kind {
            TokenKind::IntLiteral(n) => {
                let val_str = format!("{}", n);
                if self
                    .peek_ident_value()
                    .is_some_and(|s| s == "ms" || s == "s")
                {
                    let unit = self.expect_ident("expected duration unit")?;
                    Ok(Expr::Literal(
                        Literal::String(format!("{}{}", val_str, unit)),
                        t.span,
                    ))
                } else {
                    Ok(Expr::Literal(Literal::Int(*n), t.span))
                }
            }
            TokenKind::FloatLiteral(f) => {
                let val_str = format!("{}", f);
                if self
                    .peek_ident_value()
                    .is_some_and(|s| s == "ms" || s == "s")
                {
                    let unit = self.expect_ident("expected duration unit")?;
                    Ok(Expr::Literal(
                        Literal::String(format!("{}{}", val_str, unit)),
                        t.span,
                    ))
                } else {
                    Ok(Expr::Literal(Literal::Float(*f), t.span))
                }
            }
            _ => Err(self.error("expected duration value (e.g. 300ms, 1.5s)")),
        }
    }

    /// Parse playback properties (loop, direction, fill, delay)
    fn parse_playback_props(&mut self) -> Result<Vec<PlaybackProp>, ParseError> {
        let mut props = Vec::new();
        loop {
            match self.peek_ident_value() {
                Some(ref s) if s == "loop" => {
                    self.advance();
                    let value = if matches!(
                        self.peek_kind(),
                        TokenKind::IntLiteral(_) | TokenKind::FloatLiteral(_)
                    ) {
                        let t = self.advance();
                        match &t.kind {
                            TokenKind::IntLiteral(n) => LoopValue::Count(*n as u32),
                            _ => LoopValue::Infinite,
                        }
                    } else {
                        LoopValue::Infinite
                    };
                    props.push(PlaybackProp::Loop(value));
                }
                Some(ref s) if s == "direction" => {
                    self.advance();
                    let dir = match self.expect_ident("expected direction")?.as_str() {
                        "normal" => DirectionValue::Normal,
                        "reverse" => DirectionValue::Reverse,
                        "alternate" => DirectionValue::Alternate,
                        "alternateReverse" => DirectionValue::AlternateReverse,
                        other => {
                            return Err(self.error(format!(
                                "expected normal|reverse|alternate|alternateReverse, got '{}'",
                                other
                            )))
                        }
                    };
                    props.push(PlaybackProp::Direction(dir));
                }
                Some(ref s) if s == "fill" => {
                    self.advance();
                    let fill = match self.expect_ident("expected fill value")?.as_str() {
                        "none" => FillValue::None,
                        "forwards" => FillValue::Forwards,
                        "backwards" => FillValue::Backwards,
                        "both" => FillValue::Both,
                        other => {
                            return Err(self.error(format!(
                                "expected none|forwards|backwards|both, got '{}'",
                                other
                            )))
                        }
                    };
                    props.push(PlaybackProp::Fill(fill));
                }
                Some(ref s) if s == "delay" => {
                    self.advance();
                    props.push(PlaybackProp::Delay(self.parse_expr()?));
                }
                Some(ref s) if s == "paused" => {
                    // `paused` was deleted in the semantic DSL (ADR-053): it
                    // used to be parsed for syntax compatibility and silently
                    // ignored at runtime. v2 (Task 5.3) hard-errors with a
                    // migration hint — pause/resume via @if conditional
                    // rendering instead.
                    return Err(ParseError::with_kind(
                        DiagnosticKind::EPausedRemoved,
                        "paused 已删除，用 @if 条件渲染替代",
                        self.peek_span(),
                    ));
                }
                _ => break,
            }
        }
        Ok(props)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;

    /// Parse a @motion source string and return the first MotionDef.
    fn parse_motion(source: &str) -> MotionDef {
        let tokens = tokenize(source);
        let (program, errors) = parse(tokens, source);
        assert!(
            errors.is_empty(),
            "unexpected parse errors: {:?}",
            errors.iter().map(|e| e.message.clone()).collect::<Vec<_>>()
        );
        let module = program.modules.get("input.omni").unwrap();
        module.motions[0].clone()
    }

    fn keyframe_body(source: &str) -> KeyframeBody {
        match parse_motion(source).body {
            MotionBody::Keyframe(kf) => kf,
        }
    }

    #[test]
    fn motion_new_syntax_colon_timing() {
        let kf = keyframe_body(
            "@motion fadeIn { from { opacity: 0 } to { opacity: 1 } over: 300ms easing: ease-out }",
        );
        assert!(matches!(kf.over, Expr::Literal(Literal::String(ref s), _) if s == "300ms"));
        assert!(matches!(kf.easing, Some(EasingExpr::Named(ref n)) if n == "ease-out"));
    }

    #[test]
    fn motion_duration_units() {
        // ms unit
        let kf = keyframe_body("@motion a { from { opacity: 0 } to { opacity: 1 } over: 1.5s }");
        assert!(matches!(kf.over, Expr::Literal(Literal::String(ref s), _) if s == "1.5s"));
        // bare number (seconds default)
        let kf = keyframe_body("@motion b { from { opacity: 0 } to { opacity: 1 } over: 0.5 }");
        assert!(matches!(kf.over, Expr::Literal(Literal::Float(f), _) if f == 0.5));
    }

    #[test]
    fn motion_comma_separated_timing() {
        let kf = keyframe_body(
            "@motion fadeIn { from { opacity: 0 } to { opacity: 1 } over: 300ms, easing: linear, loop }",
        );
        assert!(matches!(kf.easing, Some(EasingExpr::Named(ref n)) if n == "linear"));
        assert!(matches!(
            kf.playback[0],
            PlaybackProp::Loop(LoopValue::Infinite)
        ));
    }

    #[test]
    fn motion_old_timing_syntax_rejected() {
        // Old syntax (no colon) must fail with a clear error.
        let source = "@motion fadeIn { from { opacity: 0 } to { opacity: 1 } over 300 }";
        let tokens = tokenize(source);
        let (_program, errors) = parse(tokens, source);
        assert!(
            errors.iter().any(|e| e.message.contains("expected ':'")),
            "expected colon error, got: {:?}",
            errors.iter().map(|e| e.message.clone()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn motion_whitelist_accepts_semantic_props() {
        let kf = keyframe_body(
            "@motion a { from { opacity: 0, scaleX: 0.5, translateY: -20, rotate: 45 } to { opacity: 1, scaleX: 1, translateY: 0, rotate: 0 } }",
        );
        assert_eq!(kf.stops[0].properties.len(), 4);
    }

    #[test]
    fn motion_paused_playback_keyword_rejected() {
        // `paused` was deleted in the semantic DSL (ADR-053): it used to be
        // parsed for syntax compatibility and silently ignored at runtime.
        // v2 (Task 5.3) hard-errors with a migration hint.
        let source = "@motion a { from { opacity: 0 } to { opacity: 1 } paused }";
        let tokens = tokenize(source);
        let (_program, errors) = parse(tokens, source);
        assert!(
            errors.iter().any(|e| {
                e.kind == Some(DiagnosticKind::EPausedRemoved)
                    && e.message.contains("paused 已删除，用 @if 条件渲染替代")
            }),
            "expected E_PAUSED_REMOVED error with migration hint, got: {:?}",
            errors.iter().map(|e| e.message.clone()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn motion_paused_colon_true_rejected() {
        // The colon form (`paused: true`) must produce the same hard error,
        // not a generic "expected '}'" from the trailing ':'.
        let source = "@motion a { from { opacity: 0 } to { opacity: 1 } paused: true }";
        let tokens = tokenize(source);
        let (_program, errors) = parse(tokens, source);
        assert!(
            errors.iter().any(|e| {
                e.kind == Some(DiagnosticKind::EPausedRemoved)
                    && e.message.contains("paused 已删除，用 @if 条件渲染替代")
            }),
            "expected E_PAUSED_REMOVED error with migration hint, got: {:?}",
            errors.iter().map(|e| e.message.clone()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn motion_paused_block_form_rejected() {
        // LOW-4: the BLOCK form (`@motion fadeIn { paused: true }` — `paused`
        // as the body's first token) used to fall through to the generic
        // "expected 'from' or 'to'" error; it must hard-error with the same
        // E_PAUSED_REMOVED migration hint as the keyframe-body map form.
        let source = "@motion fadeIn { paused: true }";
        let tokens = tokenize(source);
        let (_program, errors) = parse(tokens, source);
        assert!(
            errors.iter().any(|e| {
                e.kind == Some(DiagnosticKind::EPausedRemoved)
                    && e.message.contains("paused 已删除，用 @if 条件渲染替代")
            }),
            "expected E_PAUSED_REMOVED block-form error with migration hint, got: {:?}",
            errors.iter().map(|e| e.message.clone()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn motion_whitelist_accepts_border_radius_and_font_size() {
        // v2 semantic set (Task 5.2): borderRadius/fontSize join the 19-item
        // whitelist — both are animatable on all three targets.
        let kf = keyframe_body(
            "@motion soften { from { borderRadius: 8 } to { borderRadius: 16, fontSize: 12 } over: 300ms }",
        );
        assert_eq!(kf.stops[0].properties[0].0, "borderRadius");
        assert_eq!(kf.stops[1].properties[0].0, "borderRadius");
        assert_eq!(kf.stops[1].properties[1].0, "fontSize");

        let kf = keyframe_body("@motion grow { from { fontSize: 12 } to { fontSize: 16 } }");
        assert_eq!(kf.stops[0].properties[0].0, "fontSize");
    }

    #[test]
    fn motion_whitelist_rejects_unknown_prop() {
        let source =
            "@motion a { from { opacity: 0 } to { opacity: 1, boxShadow: \"0 0 4px #000\" } }";
        let tokens = tokenize(source);
        let (_program, errors) = parse(tokens, source);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("not a recognized animation property")),
            "expected whitelist error, got: {:?}",
            errors.iter().map(|e| e.message.clone()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn motion_whitelist_allows_custom_properties() {
        // CSS custom properties are allowed as passthrough (ADR-053 D3).
        let kf = keyframe_body(
            "@motion a { from { --accent-h: 221 } to { --accent-h: 350 } over: 600ms }",
        );
        assert_eq!(kf.stops[0].properties[0].0, "--accent-h");
    }

    #[test]
    fn motion_percent_stops() {
        let kf = keyframe_body(
            "@motion a { 0% { opacity: 0 } 50% { opacity: 0.5 } 100% { opacity: 1 } }",
        );
        assert_eq!(kf.stops.len(), 3);
        assert!(matches!(kf.stops[1].position, StopPosition::Percent(p) if p == 50.0));
    }

    #[test]
    fn motion_steps_easing_positional() {
        let kf = keyframe_body(
            "@motion a { from { opacity: 0 } to { opacity: 1 } easing: steps(6, end) }",
        );
        assert!(
            matches!(kf.easing, Some(EasingExpr::Steps { count: 6, ref direction }) if direction == "end")
        );
    }

    #[test]
    fn motion_easing_camelcase_normalized() {
        // Transition period: camelCase accepted and normalized to kebab-case.
        let kf =
            keyframe_body("@motion a { from { opacity: 0 } to { opacity: 1 } easing: easeOut }");
        assert!(matches!(kf.easing, Some(EasingExpr::Named(ref n)) if n == "ease-out"));

        // power2Out maps to the closest semantic equivalent.
        let kf =
            keyframe_body("@motion b { from { opacity: 0 } to { opacity: 1 } easing: power2Out }");
        assert!(matches!(kf.easing, Some(EasingExpr::Named(ref n)) if n == "ease-out"));
    }

    #[test]
    fn motion_easing_back_directions() {
        let kf = keyframe_body(
            "@motion a { from { opacity: 0 } to { opacity: 1 } easing: back-in-out }",
        );
        assert!(
            matches!(kf.easing, Some(EasingExpr::Back { ref direction }) if direction == "inout")
        );
        let kf =
            keyframe_body("@motion b { from { opacity: 0 } to { opacity: 1 } easing: backOut }");
        assert!(
            matches!(kf.easing, Some(EasingExpr::Back { ref direction }) if direction == "out")
        );
    }

    #[test]
    fn motion_unknown_easing_rejected() {
        let source = "@motion a { from { opacity: 0 } to { opacity: 1 } easing: bogus }";
        let tokens = tokenize(source);
        let (_program, errors) = parse(tokens, source);
        assert!(
            errors.iter().any(|e| e.message.contains("unknown easing")),
            "expected unknown easing error, got: {:?}",
            errors.iter().map(|e| e.message.clone()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn motion_bounce_kebab_accepted() {
        let kf = keyframe_body(
            "@motion a { from { opacity: 0 } to { opacity: 1 } easing: bounce-in-out }",
        );
        assert!(matches!(kf.easing, Some(EasingExpr::Named(ref n)) if n == "bounce-in-out"));
    }
}
