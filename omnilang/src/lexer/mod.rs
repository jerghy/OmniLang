//! Lexer module — hand-written character-by-character tokenizer.
//!
//! Produces a flat Vec<Token> from source text. No regex, no parser generators.

mod keywords;
mod token;
pub use token::*;

/// Convenience function: tokenize a source string and return all tokens.
pub fn tokenize(source: &str) -> Vec<Token> {
    Lexer::new(source).tokenize()
}

/// Hand-written lexer: character-by-character, no regex.
pub struct Lexer {
    source: Vec<char>,
    byte_offsets: Vec<usize>, // char_index -> byte_offset, length = chars.len() + 1
    pos: usize,
    line: u32,
    col: u32,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        let chars: Vec<char> = source.chars().collect();
        let mut byte_offsets = Vec::with_capacity(chars.len() + 1);
        let mut offset = 0;
        for c in &chars {
            byte_offsets.push(offset);
            offset += c.len_utf8();
        }
        byte_offsets.push(offset); // total byte length for EOF position
        Lexer {
            source: chars,
            byte_offsets,
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    /// Compute the byte offset in the original source string for a given character index.
    /// Uses a precomputed table for O(1) lookup.
    fn byte_offset(&self, char_index: usize) -> usize {
        self.byte_offsets[char_index.min(self.byte_offsets.len() - 1)]
    }

    /// Convert a character index (self.pos space) to a byte index in the original &str.
    fn byte_of(&self, char_idx: usize) -> usize {
        self.byte_offset(char_idx)
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

    /// Advance without checking bounds. Only safe when `peek()` has confirmed
    /// a character is available.
    fn advance_unchecked(&mut self) -> char {
        self.advance().expect("peek() confirmed char available")
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
                                    hex.push(self.advance_unchecked());
                                }
                                if self.peek() == Some('}') {
                                    self.advance();
                                }
                                match u32::from_str_radix(&hex, 16) {
                                    Ok(code) => match char::from_u32(code) {
                                        Some(c) => s.push(c),
                                        None => s.push('\u{FFFD}'),
                                    },
                                    Err(_) => {
                                        s.push_str(&format!("\\u{{{}}}", hex));
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
                        hex.push(self.advance_unchecked());
                    }
                    return self.mk_token(
                        match u64::from_str_radix(&hex, 16) {
                            Ok(v) => TokenKind::IntLiteral(v),
                            Err(_) => TokenKind::Error(format!("invalid hex literal: 0x{}", hex)),
                        },
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
                        oct.push(self.advance_unchecked());
                    }
                    return self.mk_token(
                        match u64::from_str_radix(&oct, 8) {
                            Ok(v) => TokenKind::IntLiteral(v),
                            Err(_) => TokenKind::Error(format!("invalid octal literal: 0o{}", oct)),
                        },
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
                        bin.push(self.advance_unchecked());
                    }
                    return self.mk_token(
                        match u64::from_str_radix(&bin, 2) {
                            Ok(v) => TokenKind::IntLiteral(v),
                            Err(_) => {
                                TokenKind::Error(format!("invalid binary literal: 0b{}", bin))
                            }
                        },
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
            num.push(self.advance_unchecked());
        }
        if self.peek() == Some('.') && self.peek_n(1).is_some_and(|c| c.is_ascii_digit()) {
            is_float = true;
            num.push(self.advance_unchecked());
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                num.push(self.advance_unchecked());
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
                num.push(self.advance_unchecked()); // consume e/E
                if self.peek() == Some('+') || self.peek() == Some('-') {
                    num.push(self.advance_unchecked());
                }
                while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                    num.push(self.advance_unchecked());
                }
            }
        }

        if is_float {
            self.mk_token(
                match num.parse() {
                    Ok(v) => TokenKind::FloatLiteral(v),
                    Err(_) => TokenKind::Error(format!("invalid float literal: {}", num)),
                },
                start,
                self.pos,
                line,
                col,
            )
        } else {
            self.mk_token(
                match num.parse() {
                    Ok(v) => TokenKind::IntLiteral(v),
                    Err(_) => TokenKind::Error(format!("invalid integer literal: {}", num)),
                },
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
            c.is_ascii_alphanumeric()
                || c == '_'
                || (!c.is_ascii() && !c.is_whitespace() && !c.is_control())
        }) {
            ident.push(self.advance_unchecked());
        }
        let kind = keywords::ident_to_keyword(&ident).unwrap_or(TokenKind::Ident(ident));
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
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            kw.push(self.advance_unchecked());
        }
        let kind = keywords::at_keyword_to_token(&kw).unwrap_or(TokenKind::Ident(kw));
        self.mk_token(kind, start, self.pos, line, col)
    }
}
