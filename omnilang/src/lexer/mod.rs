//! Lexer module — hand-written character-by-character tokenizer.
//!
//! Produces a flat Vec<Token> from source text. No regex, no parser generators.

mod token;
pub use token::*;

/// Convenience function: tokenize a source string and return all tokens.
pub fn tokenize(source: &str) -> Vec<Token> {
    Lexer::new(source).tokenize()
}

/// Hand-written lexer: character-by-character, no regex.
pub struct Lexer {
    source: Vec<char>,
    /// Byte position of each character index (char_index → byte_offset in original str).
    /// Always one entry longer than `source` so that `byte_pos[source.len()]` is valid.
    byte_pos: Vec<usize>,
    pos: usize,
    line: u32,
    col: u32,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        let chars: Vec<char> = source.chars().collect();
        // Precompute byte position for each character index.
        let mut byte_pos = Vec::with_capacity(chars.len() + 1);
        let mut bp = 0usize;
        for c in &chars {
            byte_pos.push(bp);
            bp += c.len_utf8();
        }
        byte_pos.push(bp); // sentinel for one-past-the-end
        Lexer {
            source: chars,
            byte_pos,
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    /// Convert a character index (self.pos space) to a byte index in the original &str.
    fn byte_of(&self, char_idx: usize) -> usize {
        self.byte_pos.get(char_idx).copied().unwrap_or(self.byte_pos.last().copied().unwrap_or(0))
    }

    /// Create a token whose span is in byte positions (converted from character indices).
    fn mk_token(&self, kind: TokenKind, start: usize, end: usize, line: u32, col: u32) -> Token {
        Token::new(kind, self.byte_of(start), self.byte_of(end), line, col)
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        while self.pos < self.source.len() {
            self.skip_whitespace_and_comments();
            if self.pos >= self.source.len() {
                break;
            }
            let tok = self.read_token();
            match &tok.kind {
                TokenKind::Eof => break,
                TokenKind::Error(_) => {
                    tokens.push(tok);
                    break;
                }
                _ => tokens.push(tok),
            }
        }
        tokens.push(Token::new(
            TokenKind::Eof,
            self.byte_of(self.pos),
            self.byte_of(self.pos),
            self.line,
            self.col,
        ));
        tokens
    }

    fn read_token(&mut self) -> Token {
        let start = self.pos;
        let line = self.line;
        let col = self.col;

        match self.peek() {
            None => self.mk_token(TokenKind::Eof, start, self.pos, line, col),
            Some('\'') => self.read_single_quoted_string(),
            Some('"') => self.read_string(),
            Some('`') => self.read_template_string(),
            Some(c) if c.is_ascii_digit() => self.read_number(),
            Some(c)
                if c.is_alphabetic()
                    || c == '_'
                    || (!c.is_ascii() && !c.is_whitespace() && !c.is_control()) =>
            {
                self.read_ident_or_keyword()
            }
            Some('@') => self.read_at_keyword(),
            Some('.') => {
                self.advance();
                self.check_dot_continuation(start, line, col)
            }
            Some('+') => {
                self.advance();
                self.if_next('=', TokenKind::PlusEq, TokenKind::Plus, start, line, col)
            }
            Some('-') => {
                self.advance();
                if self.peek() == Some('>') {
                    self.advance();
                    self.mk_token(TokenKind::Arrow, start, self.pos, line, col)
                } else {
                    self.if_next('=', TokenKind::MinusEq, TokenKind::Minus, start, line, col)
                }
            }
            Some('*') => {
                self.advance();
                self.if_next('=', TokenKind::StarEq, TokenKind::Star, start, line, col)
            }
            Some('/') => {
                self.advance();
                self.if_next('=', TokenKind::SlashEq, TokenKind::Slash, start, line, col)
            }
            Some('%') => {
                self.advance();
                self.if_next(
                    '=',
                    TokenKind::PercentEq,
                    TokenKind::Percent,
                    start,
                    line,
                    col,
                )
            }
            Some('=') => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    self.mk_token(TokenKind::EqEq, start, self.pos, line, col)
                } else if self.peek() == Some('>') {
                    self.advance();
                    self.mk_token(TokenKind::FatArrow, start, self.pos, line, col)
                } else {
                    self.mk_token(TokenKind::Eq, start, self.pos, line, col)
                }
            }
            Some('!') => {
                self.advance();
                self.if_next('=', TokenKind::NotEq, TokenKind::Not, start, line, col)
            }
            Some('<') => {
                self.advance();
                self.if_next('=', TokenKind::LtEq, TokenKind::Lt, start, line, col)
            }
            Some('>') => {
                self.advance();
                self.if_next('=', TokenKind::GtEq, TokenKind::Gt, start, line, col)
            }
            Some('&') => {
                self.advance();
                if self.peek() == Some('&') {
                    self.advance();
                    self.mk_token(TokenKind::AndAnd, start, self.pos, line, col)
                } else {
                    // Single & in text content (HTML entity, etc.) — treat as ident char.
                    self.mk_token(TokenKind::Ident("&".into()), start, self.pos, line, col)
                }
            }
            Some('|') => {
                self.advance();
                if self.peek() == Some('|') {
                    self.advance();
                    self.mk_token(TokenKind::OrOr, start, self.pos, line, col)
                } else {
                    self.mk_token(TokenKind::Pipe, start, self.pos, line, col)
                }
            }
            Some('$') => {
                self.advance();
                self.mk_token(TokenKind::Dollar, start, self.pos, line, col)
            }
            Some('#') => {
                self.advance();
                self.mk_token(TokenKind::Hash, start, self.pos, line, col)
            }
            Some('?') => {
                self.advance();
                if self.peek() == Some('.') {
                    self.advance();
                    self.mk_token(TokenKind::QuestionDot, start, self.pos, line, col)
                } else if self.peek() == Some('?') {
                    self.advance();
                    self.mk_token(TokenKind::QuestionQuestion, start, self.pos, line, col)
                } else {
                    self.mk_token(TokenKind::QuestionMark, start, self.pos, line, col)
                }
            }
            Some('(') => {
                self.advance();
                self.mk_token(TokenKind::LParen, start, self.pos, line, col)
            }
            Some(')') => {
                self.advance();
                self.mk_token(TokenKind::RParen, start, self.pos, line, col)
            }
            Some('{') => {
                self.advance();
                self.mk_token(TokenKind::LBrace, start, self.pos, line, col)
            }
            Some('}') => {
                self.advance();
                self.mk_token(TokenKind::RBrace, start, self.pos, line, col)
            }
            Some('[') => {
                self.advance();
                self.mk_token(TokenKind::LBracket, start, self.pos, line, col)
            }
            Some(']') => {
                self.advance();
                self.mk_token(TokenKind::RBracket, start, self.pos, line, col)
            }
            Some(';') => {
                self.advance();
                self.mk_token(TokenKind::Semicolon, start, self.pos, line, col)
            }
            Some(',') => {
                self.advance();
                self.mk_token(TokenKind::Comma, start, self.pos, line, col)
            }
            Some(':') => {
                self.advance();
                self.mk_token(TokenKind::Colon, start, self.pos, line, col)
            }
            Some(c) => {
                self.advance();
                self.mk_token(
                    TokenKind::Error(format!("Unexpected character: '{}'", c)),
                    start,
                    self.pos,
                    line,
                    col,
                )
            }
        }
    }

    fn advance(&mut self) -> Option<char> {
        if self.pos >= self.source.len() {
            return None;
        }
        let c = self.source[self.pos];
        self.pos += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(c)
    }

    fn peek(&self) -> Option<char> {
        self.source.get(self.pos).copied()
    }

    fn peek_n(&self, n: usize) -> Option<char> {
        self.source.get(self.pos + n).copied()
    }

    fn if_next(
        &mut self,
        expected: char,
        if_match: TokenKind,
        if_not: TokenKind,
        start: usize,
        line: u32,
        col: u32,
    ) -> Token {
        if self.peek() == Some(expected) {
            self.advance();
            self.mk_token(if_match, start, self.pos, line, col)
        } else {
            self.mk_token(if_not, start, self.pos, line, col)
        }
    }

    fn expect(
        &mut self,
        expected: char,
        kind: TokenKind,
        err_msg: &str,
        start: usize,
        line: u32,
        col: u32,
    ) -> Token {
        if self.peek() == Some(expected) {
            self.advance();
            self.mk_token(kind, start, self.pos, line, col)
        } else {
            self.mk_token(
                TokenKind::Error(err_msg.to_string()),
                start,
                self.pos,
                line,
                col,
            )
        }
    }

    fn check_dot_continuation(&mut self, start: usize, line: u32, col: u32) -> Token {
        if self.peek() == Some('.') {
            self.advance();
            if self.peek() == Some('.') {
                self.advance();
                return self.mk_token(TokenKind::DotDotDot, start, self.pos, line, col);
            }
            self.mk_token(
                TokenKind::Error("expected ...".into()),
                start,
                self.pos,
                line,
                col,
            )
        } else {
            self.mk_token(TokenKind::Dot, start, self.pos, line, col)
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek() {
                Some(' ') | Some('\t') | Some('\n') | Some('\r') => {
                    self.advance();
                }
                Some('/') => {
                    if self.peek_n(1) == Some('/') {
                        while self.peek().is_some() && self.peek() != Some('\n') {
                            self.advance();
                        }
                    } else if self.peek_n(1) == Some('*') {
                        self.advance();
                        self.advance();
                        loop {
                            if self.peek() == Some('*') && self.peek_n(1) == Some('/') {
                                self.advance();
                                self.advance();
                                break;
                            }
                            if self.peek().is_none() {
                                break;
                            }
                            self.advance();
                        }
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }
    }

    fn read_single_quoted_string(&mut self) -> Token {
        let start = self.pos;
        let line = self.line;
        let col = self.col;
        self.advance(); // opening '
        let mut s = String::new();
        loop {
            match self.peek() {
                None => break,
                Some('\'') => {
                    self.advance();
                    break;
                }
                Some('\\') => {
                    self.advance();
                    if self.peek() == Some('\'') {
                        s.push('\'');
                        self.advance();
                    } else if self.peek() == Some('\\') {
                        s.push('\\');
                        self.advance();
                    } else {
                        s.push('\\');
                    }
                }
                Some('<') => {
                    let next = self.peek_n(1);
                    if next.is_some_and(|c| c.is_alphabetic() || c == '/') {
                        break;
                    }
                    s.push('<');
                    self.advance();
                }
                Some('>') => {
                    s.push('>');
                    self.advance();
                }
                Some(c) => {
                    s.push(c);
                    self.advance();
                }
            }
        }
        self.mk_token(TokenKind::StringLiteral(s), start, self.pos, line, col)
    }

    fn read_string(&mut self) -> Token {
        let start = self.pos;
        let line = self.line;
        let col = self.col;
        self.advance(); // opening "
        let mut s = String::new();
        loop {
            match self.peek() {
                None => break,
                Some('"') => {
                    self.advance();
                    break;
                }
                Some('<') => {
                    // Only break if < looks like an HTML tag start (<letter or </)
                    // Allows < in strings like regex patterns "<[^>]+>"
                    let next = self.peek_n(1);
                    if next.is_some_and(|c| c.is_alphabetic() || c == '/') {
                        break;
                    }
                    s.push('<');
                    self.advance();
                }
                Some('>') => {
                    // > inside a string is normal content (comparison, regex, etc.)
                    s.push('>');
                    self.advance();
                }
                Some('\\') => {
                    self.advance();
                    match self.peek() {
                        Some('"') => {
                            s.push('"');
                            self.advance();
                        }
                        Some('\\') => {
                            s.push('\\');
                            self.advance();
                        }
                        Some('n') => {
                            s.push('\n');
                            self.advance();
                        }
                        Some('t') => {
                            s.push('\t');
                            self.advance();
                        }
                        Some('r') => {
                            s.push('\r');
                            self.advance();
                        }
                        Some('u') => {
                            self.advance();
                            if self.peek() == Some('{') {
                                self.advance();
                                let mut hex = String::new();
                                while self.peek().is_some_and(|c| c.is_ascii_hexdigit()) {
                                    hex.push(self.advance().unwrap());
                                }
                                if self.peek() == Some('}') {
                                    self.advance();
                                }
                                if let Ok(code) = u32::from_str_radix(&hex, 16) {
                                    if let Some(c) = char::from_u32(code) {
                                        s.push(c);
                                    }
                                }
                            }
                        }
                        _ => {
                            s.push('\\');
                        }
                    }
                }
                Some(c) => {
                    s.push(c);
                    self.advance();
                }
            }
        }
        self.mk_token(TokenKind::StringLiteral(s), start, self.pos, line, col)
    }

    fn read_template_string(&mut self) -> Token {
        let start = self.pos;
        let line = self.line;
        let col = self.col;
        self.advance(); // opening `
        let mut s = String::new();
        loop {
            match self.peek() {
                None => break,
                Some('`') => {
                    self.advance();
                    break;
                }
                Some('\\') => {
                    self.advance();
                    if self.peek() == Some('`') {
                        s.push('`');
                        self.advance();
                    } else if self.peek() == Some('\\') {
                        s.push('\\');
                        self.advance();
                    } else {
                        s.push('\\');
                    }
                }
                Some(c) => {
                    s.push(c);
                    self.advance();
                }
            }
        }
        self.mk_token(TokenKind::TemplateString(s), start, self.pos, line, col)
    }

    fn read_number(&mut self) -> Token {
        let start = self.pos;
        let line = self.line;
        let col = self.col;

        // Check for hex/octal/binary prefix
        if self.peek() == Some('0') {
            match self.peek_n(1) {
                Some('x') | Some('X') => {
                    self.advance();
                    self.advance();
                    let mut hex = String::new();
                    while self.peek().is_some_and(|c| c.is_ascii_hexdigit()) {
                        hex.push(self.advance().unwrap());
                    }
                    return self.mk_token(
                        TokenKind::IntLiteral(i64::from_str_radix(&hex, 16).unwrap_or(0)),
                        start,
                        self.pos,
                        line,
                        col,
                    );
                }
                Some('o') | Some('O') => {
                    self.advance();
                    self.advance();
                    let mut oct = String::new();
                    while self.peek().is_some_and(|c| ('0'..='7').contains(&c)) {
                        oct.push(self.advance().unwrap());
                    }
                    return self.mk_token(
                        TokenKind::IntLiteral(i64::from_str_radix(&oct, 8).unwrap_or(0)),
                        start,
                        self.pos,
                        line,
                        col,
                    );
                }
                Some('b') | Some('B') => {
                    self.advance();
                    self.advance();
                    let mut bin = String::new();
                    while self.peek() == Some('0') || self.peek() == Some('1') {
                        bin.push(self.advance().unwrap());
                    }
                    return self.mk_token(
                        TokenKind::IntLiteral(i64::from_str_radix(&bin, 2).unwrap_or(0)),
                        start,
                        self.pos,
                        line,
                        col,
                    );
                }
                _ => {}
            }
        }

        let mut num = String::new();
        let mut is_float = false;
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            num.push(self.advance().unwrap());
        }
        if self.peek() == Some('.') && self.peek_n(1).is_some_and(|c| c.is_ascii_digit()) {
            is_float = true;
            num.push(self.advance().unwrap());
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                num.push(self.advance().unwrap());
            }
        }
        // Peek-ahead for scientific notation: e/E must be followed by at
        // least one digit (optionally after a sign). Otherwise the 'e' is
        // the start of a unit/identifier (e.g. "10ex" CSS unit, "10em").
        if self.peek() == Some('e') || self.peek() == Some('E') {
            let e_pos = self.pos;
            let mut look = e_pos + 1;
            let mut has_exponent = false;
            if self.peek_n(1) == Some('+') || self.peek_n(1) == Some('-') {
                look += 1;
            }
            if let Some(c) = self.source.get(look) {
                if c.is_ascii_digit() {
                    has_exponent = true;
                }
            }
            if has_exponent {
                is_float = true;
                num.push(self.advance().unwrap()); // consume e/E
                if self.peek() == Some('+') || self.peek() == Some('-') {
                    num.push(self.advance().unwrap());
                }
                while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                    num.push(self.advance().unwrap());
                }
            }
        }

        if is_float {
            self.mk_token(
                TokenKind::FloatLiteral(num.parse().unwrap_or(0.0)),
                start,
                self.pos,
                line,
                col,
            )
        } else {
            self.mk_token(
                TokenKind::IntLiteral(num.parse().unwrap_or(0)),
                start,
                self.pos,
                line,
                col,
            )
        }
    }

    fn read_ident_or_keyword(&mut self) -> Token {
        let start = self.pos;
        let line = self.line;
        let col = self.col;
        let mut ident = String::new();
        while self.peek().is_some_and(|c| {
            c.is_alphanumeric()
                || c == '_'
                || (!c.is_ascii() && !c.is_whitespace() && !c.is_control())
        }) {
            ident.push(self.advance().unwrap());
        }
        let kind = match ident.as_str() {
            "fn" => TokenKind::Fn,
            "const" => TokenKind::Const,
            "let" => TokenKind::Let,
            "mut" => TokenKind::Mut,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "return" => TokenKind::Return,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "print" => TokenKind::Print,
            "println" => TokenKind::Println,
            "as" => TokenKind::As,
            "async" => TokenKind::Async,
            "await" => TokenKind::Await,
            "throw" => TokenKind::Throw,
            "try" => TokenKind::Try,
            "catch" => TokenKind::Catch,
            "finally" => TokenKind::Finally,
            "match" => TokenKind::Match,
            "struct" => TokenKind::Struct,
            "class" => TokenKind::Class,
            "interface" => TokenKind::Interface,
            "impl" => TokenKind::Impl,
            "extends" => TokenKind::Extends,
            "implements" => TokenKind::Implements,
            "type" => TokenKind::Type,
            "nominal" => TokenKind::Nominal,
            "static" => TokenKind::Static,
            "private" => TokenKind::Private,
            "pub" => TokenKind::Pub,
            "override" => TokenKind::Override,
            "import" => TokenKind::Import,
            "from" => TokenKind::From,
            "export" => TokenKind::Export,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "null" => TokenKind::Null,
            "component" => TokenKind::Component,
            "signal" => TokenKind::Signal,
            "computed" => TokenKind::Computed,
            "prop" => TokenKind::Prop,
            "event" => TokenKind::Event,
            "ref" => TokenKind::Ref,
            "store" => TokenKind::Store,
            "view" => TokenKind::View,
            "style" => TokenKind::Style,
            "action" => TokenKind::Action,
            "getter" => TokenKind::Getter,
            "theme" => TokenKind::Theme,
            "bind" => TokenKind::Bind,
            "emit" => TokenKind::Emit,
            // Motion keywords (context-sensitive, recognized in @motion blocks)
            "if_mode" => TokenKind::IfMode,
            "drag_constraints" => TokenKind::DragConstraints,
            "drag_elastic" => TokenKind::DragElastic,
            "drag_scrub" => TokenKind::DragScrub,
            "drag_momentum" => TokenKind::DragMomentum,
            "drag_threshold" => TokenKind::DragThreshold,
            "drag_velocity_max" => TokenKind::DragVelocityMax,
            _ => TokenKind::Ident(ident),
        };
        self.mk_token(kind, start, self.pos, line, col)
    }

    fn read_at_keyword(&mut self) -> Token {
        let start = self.pos;
        let line = self.line;
        let col = self.col;
        self.advance(); // @
        let mut kw = String::from("@");
        while self
            .peek()
            .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            kw.push(self.advance().unwrap());
        }
        let kind = match kw.as_str() {
            "@store" => TokenKind::AtStore,
            "@if" => TokenKind::AtIf,
            "@else" => TokenKind::AtElse,
            "@for" => TokenKind::AtFor,
            "@empty" => TokenKind::AtEmpty,
            "@slot" => TokenKind::AtSlot,
            "@native" => TokenKind::AtNative,
            "@fallback" => TokenKind::AtFallback,
            "@error" => TokenKind::AtError,
            "@mount" => TokenKind::AtMount,
            "@unmount" => TokenKind::AtUnmount,
            "@effect" => TokenKind::AtEffect,
            // Motion @-keywords
            "@motion" => TokenKind::AtMotion,
            "@timeline" => TokenKind::AtTimeline,
            "@enter" => TokenKind::AtEnter,
            "@leave" => TokenKind::AtLeave,
            "@hover" => TokenKind::AtHover,
            "@unhover" => TokenKind::AtUnhover,
            "@press" => TokenKind::AtPress,
            "@focus" => TokenKind::AtFocus,
            "@animate" => TokenKind::AtAnimate,
            "@in-view" => TokenKind::AtInView,
            "@scroll" => TokenKind::AtScroll,
            "@drag-horizontal" | "@dragHorizontal" => TokenKind::AtDragHorizontal,
            "@drag-vertical" | "@dragVertical" => TokenKind::AtDragVertical,
            "@driven-by" => TokenKind::AtDrivenBy,
            "@paused" => TokenKind::AtPaused,
            "@hover-reverse" => TokenKind::AtHoverReverse,
            "@in-view-once" => TokenKind::AtInViewOnce,
            "@in-view-repeat" => TokenKind::AtInViewRepeat,
            "@in-view-threshold" => TokenKind::AtInViewThreshold,
            "@scroll-start" => TokenKind::AtScrollStart,
            "@scroll-end" => TokenKind::AtScrollEnd,
            "@scroll-scrub" => TokenKind::AtScrollScrub,
            // Drag sub-attributes
            "@drag-constraints" => TokenKind::DragConstraints,
            "@drag-elastic" => TokenKind::DragElastic,
            "@drag-scrub" => TokenKind::DragScrub,
            "@drag-momentum" => TokenKind::DragMomentum,
            "@drag-threshold" => TokenKind::DragThreshold,
            "@drag-velocity-max" => TokenKind::DragVelocityMax,
            // @hero support
            "@hero" => TokenKind::AtHero,
            "@hero-transition" => TokenKind::AtHeroTransition,
            // @if-mode component prop
            "@if-mode" => TokenKind::IfMode,
            // Unknown @-prefixed words are event bindings like @onPress, @onChange
            _ => TokenKind::Ident(kw),
        };
        self.mk_token(kind, start, self.pos, line, col)
    }
}
