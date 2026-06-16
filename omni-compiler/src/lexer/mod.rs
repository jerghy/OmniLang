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
    pos: usize,
    line: u32,
    col: u32,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Lexer {
            source: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
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
            self.pos,
            self.pos,
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
            None => Token::new(TokenKind::Eof, start, self.pos, line, col),
            Some('"') => self.read_string(),
            Some('`') => self.read_template_string(),
            Some(c) if c.is_ascii_digit() => self.read_number(),
            Some(c) if c.is_alphabetic() || c == '_' || (!c.is_ascii() && !c.is_whitespace() && !c.is_control()) => self.read_ident_or_keyword(),
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
                    Token::new(TokenKind::Arrow, start, self.pos, line, col)
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
                    Token::new(TokenKind::EqEq, start, self.pos, line, col)
                } else if self.peek() == Some('>') {
                    self.advance();
                    Token::new(TokenKind::FatArrow, start, self.pos, line, col)
                } else {
                    Token::new(TokenKind::Eq, start, self.pos, line, col)
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
                self.expect('&', TokenKind::AndAnd, "expected &&", start, line, col)
            }
            Some('|') => {
                self.advance();
                if self.peek() == Some('|') {
                    self.advance();
                    Token::new(TokenKind::OrOr, start, self.pos, line, col)
                } else {
                    Token::new(TokenKind::Pipe, start, self.pos, line, col)
                }
            }
            Some('$') => {
                self.advance();
                Token::new(TokenKind::Dollar, start, self.pos, line, col)
            }
            Some('#') => {
                self.advance();
                Token::new(TokenKind::Hash, start, self.pos, line, col)
            }
            Some('?') => {
                self.advance();
                if self.peek() == Some('.') {
                    self.advance();
                    Token::new(TokenKind::QuestionDot, start, self.pos, line, col)
                } else {
                    Token::new(TokenKind::QuestionMark, start, self.pos, line, col)
                }
            }
            Some('(') => {
                self.advance();
                Token::new(TokenKind::LParen, start, self.pos, line, col)
            }
            Some(')') => {
                self.advance();
                Token::new(TokenKind::RParen, start, self.pos, line, col)
            }
            Some('{') => {
                self.advance();
                Token::new(TokenKind::LBrace, start, self.pos, line, col)
            }
            Some('}') => {
                self.advance();
                Token::new(TokenKind::RBrace, start, self.pos, line, col)
            }
            Some('[') => {
                self.advance();
                Token::new(TokenKind::LBracket, start, self.pos, line, col)
            }
            Some(']') => {
                self.advance();
                Token::new(TokenKind::RBracket, start, self.pos, line, col)
            }
            Some(';') => {
                self.advance();
                Token::new(TokenKind::Semicolon, start, self.pos, line, col)
            }
            Some(',') => {
                self.advance();
                Token::new(TokenKind::Comma, start, self.pos, line, col)
            }
            Some(':') => {
                self.advance();
                Token::new(TokenKind::Colon, start, self.pos, line, col)
            }
            Some(c) => {
                self.advance();
                Token::new(
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
            Token::new(if_match, start, self.pos, line, col)
        } else {
            Token::new(if_not, start, self.pos, line, col)
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
            Token::new(kind, start, self.pos, line, col)
        } else {
            Token::new(
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
                return Token::new(TokenKind::DotDotDot, start, self.pos, line, col);
            }
            Token::new(
                TokenKind::Error("expected ...".into()),
                start,
                self.pos,
                line,
                col,
            )
        } else {
            Token::new(TokenKind::Dot, start, self.pos, line, col)
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
                Some('<') | Some('>') => {
                    // HTML boundary in view text content — treat as end of string
                    // The unmatched " will be handled as text by the parser
                    break;
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
        Token::new(TokenKind::StringLiteral(s), start, self.pos, line, col)
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
        Token::new(TokenKind::TemplateString(s), start, self.pos, line, col)
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
                    return Token::new(
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
                    return Token::new(
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
                    return Token::new(
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
        if self.peek() == Some('e') || self.peek() == Some('E') {
            is_float = true;
            num.push(self.advance().unwrap());
            if self.peek() == Some('+') || self.peek() == Some('-') {
                num.push(self.advance().unwrap());
            }
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                num.push(self.advance().unwrap());
            }
        }

        if is_float {
            Token::new(
                TokenKind::FloatLiteral(num.parse().unwrap_or(0.0)),
                start,
                self.pos,
                line,
                col,
            )
        } else {
            Token::new(
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
        while self
            .peek()
            .is_some_and(|c| c.is_alphanumeric() || c == '_' || (!c.is_ascii() && !c.is_whitespace() && !c.is_control()))
        {
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
            "async" => TokenKind::Async,
            "await" => TokenKind::Await,
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
            _ => TokenKind::Ident(ident),
        };
        Token::new(kind, start, self.pos, line, col)
    }

    fn read_at_keyword(&mut self) -> Token {
        let start = self.pos;
        let line = self.line;
        let col = self.col;
        self.advance(); // @
        let mut kw = String::from("@");
        while self
            .peek()
            .is_some_and(|c| c.is_alphanumeric() || c == '_')
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
            // Unknown @-prefixed words are event bindings like @onPress, @onChange
            _ => TokenKind::Ident(kw),
        };
        Token::new(kind, start, self.pos, line, col)
    }
}
