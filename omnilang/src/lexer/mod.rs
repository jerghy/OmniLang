//! Lexer module — byte-level tokenizer (zero-copy, no Vec<char> allocation).
//!
//! Produces a flat Vec<Token> from source text. No regex, no parser generators.

mod keywords;
mod token;
pub use token::*;

/// Convenience function: tokenize a source string and return all tokens.
pub fn tokenize(source: &str) -> Vec<Token> {
    if source.len() > MAX_INPUT_SIZE {
        return vec![Token::new(
            TokenKind::Error(format!(
                "input size {} exceeds maximum of {} bytes",
                source.len(),
                MAX_INPUT_SIZE
            )),
            0,
            0,
            1,
            1,
        )];
    }
    Lexer::new(source).tokenize()
}

/// Maximum input size: 64 MB. Prevents DoS from pathological inputs.
const MAX_INPUT_SIZE: usize = 64 * 1024 * 1024;

/// Hand-written lexer: byte-level scanning, zero Vec<char> allocation.
///
/// Borrows the source bytes directly for zero-copy tokenization. All positions
/// are byte offsets into the original source — no character-index conversion needed.
pub struct Lexer<'a> {
    source: &'a [u8], // borrowed byte slice — ZERO allocation
    pos: usize,       // byte position
    line: u32,
    col: u32,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Lexer {
            source: source.as_bytes(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    /// Create a token with byte positions directly (no conversion needed).
    fn mk_token(&self, kind: TokenKind, start: usize, end: usize, line: u32, col: u32) -> Token {
        Token::new(kind, start, end, line, col)
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

    // ── Byte-level primitives ──────────────────────────────────────────────

    /// Peek at the current byte (ASCII fast path).
    fn peek_byte(&self) -> Option<u8> {
        self.source.get(self.pos).copied()
    }

    /// Peek at a byte n positions ahead.
    fn peek_n_byte(&self, n: usize) -> Option<u8> {
        self.source.get(self.pos + n).copied()
    }

    /// Peek at the current character (decodes multi-byte UTF-8 as needed).
    /// Uses an ASCII fast path for single-byte chars.
    fn peek_char(&self) -> Option<char> {
        let b = self.source.get(self.pos)?;
        if *b < 0x80 {
            return Some(*b as char);
        }
        // Multi-byte: decode from remaining bytes
        let s = std::str::from_utf8(&self.source[self.pos..]).ok()?;
        s.chars().next()
    }

    /// Advance by one byte and update line/col tracking.
    ///
    /// col increments only on ASCII bytes and the first byte of multi-byte sequences
    /// (continuation bytes are skipped for col counting).
    fn advance(&mut self) -> Option<u8> {
        let b = self.source.get(self.pos).copied()?;
        if b == b'\n' {
            self.line += 1;
            self.col = 1;
        } else if b < 0x80 || (b & 0xC0) != 0x80 {
            // Not a continuation byte — counts as a visible character
            self.col += 1;
        }
        self.pos += 1;
        Some(b)
    }

    /// Advance without checking bounds. Only safe when `peek_byte()` has confirmed
    /// a byte is available.
    fn advance_unchecked(&mut self) -> u8 {
        match self.advance() {
            Some(b) => b,
            None => {
                // This path should never be reached when peek confirmed availability
                b'\0'
            }
        }
    }

    /// Advance by one full character (may be multiple bytes for non-ASCII).
    /// Only use when you need the actual char value (string content, ident chars).
    fn advance_char(&mut self) -> Option<char> {
        let c = self.peek_char()?;
        let n_bytes = c.len_utf8();
        for _ in 0..n_bytes {
            self.advance();
        }
        Some(c)
    }

    /// Advance by one full character, unbounded.
    fn advance_char_unchecked(&mut self) -> char {
        match self.advance_char() {
            Some(c) => c,
            None => '\0',
        }
    }

    // ── Token dispatch ─────────────────────────────────────────────────────

    fn read_token(&mut self) -> Token {
        let start = self.pos;
        let line = self.line;
        let col = self.col;

        match self.peek_byte() {
            None => self.mk_token(TokenKind::Eof, start, self.pos, line, col),
            Some(b'\'') => self.read_single_quoted_string(),
            Some(b'"') => self.read_string(),
            Some(b'`') => self.read_template_string(),
            Some(b'0'..=b'9') => self.read_number(),
            Some(b'@') => self.read_at_keyword(),
            Some(b'.') => {
                self.advance();
                self.check_dot_continuation(start, line, col)
            }
            Some(b'+') => {
                self.advance();
                self.if_next(b'=', TokenKind::PlusEq, TokenKind::Plus, start, line, col)
            }
            Some(b'-') => {
                self.advance();
                if self.peek_byte() == Some(b'>') {
                    self.advance();
                    self.mk_token(TokenKind::Arrow, start, self.pos, line, col)
                } else {
                    self.if_next(b'=', TokenKind::MinusEq, TokenKind::Minus, start, line, col)
                }
            }
            Some(b'*') => {
                self.advance();
                self.if_next(b'=', TokenKind::StarEq, TokenKind::Star, start, line, col)
            }
            Some(b'/') => {
                self.advance();
                self.if_next(b'=', TokenKind::SlashEq, TokenKind::Slash, start, line, col)
            }
            Some(b'%') => {
                self.advance();
                self.if_next(
                    b'=',
                    TokenKind::PercentEq,
                    TokenKind::Percent,
                    start,
                    line,
                    col,
                )
            }
            Some(b'=') => {
                self.advance();
                if self.peek_byte() == Some(b'=') {
                    self.advance();
                    self.mk_token(TokenKind::EqEq, start, self.pos, line, col)
                } else if self.peek_byte() == Some(b'>') {
                    self.advance();
                    self.mk_token(TokenKind::FatArrow, start, self.pos, line, col)
                } else {
                    self.mk_token(TokenKind::Eq, start, self.pos, line, col)
                }
            }
            Some(b'!') => {
                self.advance();
                self.if_next(b'=', TokenKind::NotEq, TokenKind::Not, start, line, col)
            }
            Some(b'<') => {
                self.advance();
                self.if_next(b'=', TokenKind::LtEq, TokenKind::Lt, start, line, col)
            }
            Some(b'>') => {
                self.advance();
                self.if_next(b'=', TokenKind::GtEq, TokenKind::Gt, start, line, col)
            }
            Some(b'&') => {
                self.advance();
                if self.peek_byte() == Some(b'&') {
                    self.advance();
                    self.mk_token(TokenKind::AndAnd, start, self.pos, line, col)
                } else {
                    // Single & in text content (HTML entity, etc.) — treat as ident char.
                    self.mk_token(TokenKind::Ident("&".into()), start, self.pos, line, col)
                }
            }
            Some(b'|') => {
                self.advance();
                if self.peek_byte() == Some(b'|') {
                    self.advance();
                    self.mk_token(TokenKind::OrOr, start, self.pos, line, col)
                } else {
                    self.mk_token(TokenKind::Pipe, start, self.pos, line, col)
                }
            }
            Some(b'$') => {
                self.advance();
                self.mk_token(TokenKind::Dollar, start, self.pos, line, col)
            }
            Some(b'#') => {
                self.advance();
                self.mk_token(TokenKind::Hash, start, self.pos, line, col)
            }
            Some(b'?') => {
                self.advance();
                if self.peek_byte() == Some(b'.') {
                    self.advance();
                    self.mk_token(TokenKind::QuestionDot, start, self.pos, line, col)
                } else if self.peek_byte() == Some(b'?') {
                    self.advance();
                    self.mk_token(TokenKind::QuestionQuestion, start, self.pos, line, col)
                } else {
                    self.mk_token(TokenKind::QuestionMark, start, self.pos, line, col)
                }
            }
            Some(b'(') => {
                self.advance();
                self.mk_token(TokenKind::LParen, start, self.pos, line, col)
            }
            Some(b')') => {
                self.advance();
                self.mk_token(TokenKind::RParen, start, self.pos, line, col)
            }
            Some(b'{') => {
                self.advance();
                self.mk_token(TokenKind::LBrace, start, self.pos, line, col)
            }
            Some(b'}') => {
                self.advance();
                self.mk_token(TokenKind::RBrace, start, self.pos, line, col)
            }
            Some(b'[') => {
                self.advance();
                self.mk_token(TokenKind::LBracket, start, self.pos, line, col)
            }
            Some(b']') => {
                self.advance();
                self.mk_token(TokenKind::RBracket, start, self.pos, line, col)
            }
            Some(b';') => {
                self.advance();
                self.mk_token(TokenKind::Semicolon, start, self.pos, line, col)
            }
            Some(b',') => {
                self.advance();
                self.mk_token(TokenKind::Comma, start, self.pos, line, col)
            }
            Some(b':') => {
                self.advance();
                self.mk_token(TokenKind::Colon, start, self.pos, line, col)
            }
            Some(_) => {
                // Fallback: check for identifier (alphabetic, underscore, or non-ASCII)
                if let Some(c) = self.peek_char() {
                    if c.is_alphabetic()
                        || c == '_'
                        || (!c.is_ascii() && !c.is_whitespace() && !c.is_control())
                    {
                        return self.read_ident_or_keyword();
                    }
                }
                self.advance();
                let c = self.peek_char().unwrap_or('\0');
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

    fn if_next(
        &mut self,
        expected: u8,
        if_match: TokenKind,
        if_not: TokenKind,
        start: usize,
        line: u32,
        col: u32,
    ) -> Token {
        if self.peek_byte() == Some(expected) {
            self.advance();
            self.mk_token(if_match, start, self.pos, line, col)
        } else {
            self.mk_token(if_not, start, self.pos, line, col)
        }
    }

    fn check_dot_continuation(&mut self, start: usize, line: u32, col: u32) -> Token {
        if self.peek_byte() == Some(b'.') {
            self.advance();
            if self.peek_byte() == Some(b'.') {
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
            match self.peek_byte() {
                Some(b' ' | b'\t' | b'\n' | b'\r') => {
                    self.advance();
                }
                Some(b'/') => {
                    if self.peek_n_byte(1) == Some(b'/') {
                        while self.peek_byte().is_some() && self.peek_byte() != Some(b'\n') {
                            self.advance();
                        }
                    } else if self.peek_n_byte(1) == Some(b'*') {
                        self.advance();
                        self.advance();
                        loop {
                            if self.peek_byte() == Some(b'*') && self.peek_n_byte(1) == Some(b'/') {
                                self.advance();
                                self.advance();
                                break;
                            }
                            if self.peek_byte().is_none() {
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

    // ── String readers ─────────────────────────────────────────────────────

    fn read_single_quoted_string(&mut self) -> Token {
        let start = self.pos;
        let line = self.line;
        let col = self.col;
        self.advance(); // opening '
        let remaining = self.source.len().saturating_sub(self.pos);
        let mut s = String::with_capacity(remaining.min(65536));
        loop {
            match self.peek_byte() {
                None => break,
                Some(b'\'') => {
                    self.advance();
                    break;
                }
                Some(b'\\') => {
                    self.advance();
                    if self.peek_byte() == Some(b'\'') {
                        s.push('\'');
                        self.advance();
                    } else if self.peek_byte() == Some(b'\\') {
                        s.push('\\');
                        self.advance();
                    } else {
                        s.push('\\');
                    }
                }
                Some(b'<') => {
                    // Only break if < looks like an HTML tag start (<letter or </)
                    let next = self.peek_n_byte(1);
                    if next.is_some_and(|b| b.is_ascii_alphabetic() || b == b'/') {
                        break;
                    }
                    s.push('<');
                    self.advance();
                }
                Some(b'>') => {
                    s.push('>');
                    self.advance();
                }
                Some(_) => {
                    if let Some(c) = self.advance_char() {
                        s.push(c);
                    } else {
                        break;
                    }
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
        let remaining = self.source.len().saturating_sub(self.pos);
        let mut s = String::with_capacity(remaining.min(65536));
        loop {
            match self.peek_byte() {
                None => break,
                Some(b'"') => {
                    self.advance();
                    break;
                }
                Some(b'<') => {
                    // Only break if < looks like an HTML tag start (<letter or </)
                    // Allows < in strings like regex patterns "<[^>]+>"
                    let next = self.peek_n_byte(1);
                    if next.is_some_and(|b| b.is_ascii_alphabetic() || b == b'/') {
                        break;
                    }
                    s.push('<');
                    self.advance();
                }
                Some(b'>') => {
                    // > inside a string is normal content (comparison, regex, etc.)
                    s.push('>');
                    self.advance();
                }
                Some(b'\\') => {
                    self.advance();
                    match self.peek_byte() {
                        Some(b'"') => {
                            s.push('"');
                            self.advance();
                        }
                        Some(b'\\') => {
                            s.push('\\');
                            self.advance();
                        }
                        Some(b'n') => {
                            s.push('\n');
                            self.advance();
                        }
                        Some(b't') => {
                            s.push('\t');
                            self.advance();
                        }
                        Some(b'r') => {
                            s.push('\r');
                            self.advance();
                        }
                        Some(b'u') => {
                            self.advance();
                            if self.peek_byte() == Some(b'{') {
                                self.advance();
                                let mut hex = String::new();
                                while self.peek_byte().is_some_and(|b| b.is_ascii_hexdigit()) {
                                    hex.push(self.advance_unchecked() as char);
                                }
                                if self.peek_byte() == Some(b'}') {
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
                Some(_) => {
                    if let Some(c) = self.advance_char() {
                        s.push(c);
                    } else {
                        break;
                    }
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
        let remaining = self.source.len().saturating_sub(self.pos);
        let mut s = String::with_capacity(remaining.min(65536));
        loop {
            match self.peek_byte() {
                None => break,
                Some(b'`') => {
                    self.advance();
                    break;
                }
                Some(b'\\') => {
                    self.advance();
                    if self.peek_byte() == Some(b'`') {
                        s.push('`');
                        self.advance();
                    } else if self.peek_byte() == Some(b'\\') {
                        s.push('\\');
                        self.advance();
                    } else {
                        s.push('\\');
                    }
                }
                Some(_) => {
                    if let Some(c) = self.advance_char() {
                        s.push(c);
                    } else {
                        break;
                    }
                }
            }
        }
        self.mk_token(TokenKind::TemplateString(s), start, self.pos, line, col)
    }

    // ── Number reader ──────────────────────────────────────────────────────

    fn read_number(&mut self) -> Token {
        let start = self.pos;
        let line = self.line;
        let col = self.col;

        // Check for hex/octal/binary prefix
        if self.peek_byte() == Some(b'0') {
            match self.peek_n_byte(1) {
                Some(b'x') | Some(b'X') => {
                    self.advance();
                    self.advance();
                    let mut hex = String::new();
                    while self.peek_byte().is_some_and(|b| b.is_ascii_hexdigit()) {
                        hex.push(self.advance_unchecked() as char);
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
                Some(b'o') | Some(b'O') => {
                    self.advance();
                    self.advance();
                    let mut oct = String::new();
                    while self.peek_byte().is_some_and(|b| (b'0'..=b'7').contains(&b)) {
                        oct.push(self.advance_unchecked() as char);
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
                Some(b'b') | Some(b'B') => {
                    self.advance();
                    self.advance();
                    let mut bin = String::new();
                    while self.peek_byte() == Some(b'0') || self.peek_byte() == Some(b'1') {
                        bin.push(self.advance_unchecked() as char);
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
        while self.peek_byte().is_some_and(|b| b.is_ascii_digit()) {
            num.push(self.advance_unchecked() as char);
        }
        if self.peek_byte() == Some(b'.') && self.peek_n_byte(1).is_some_and(|b| b.is_ascii_digit()) {
            is_float = true;
            num.push(self.advance_unchecked() as char);
            while self.peek_byte().is_some_and(|b| b.is_ascii_digit()) {
                num.push(self.advance_unchecked() as char);
            }
        }
        // Peek-ahead for scientific notation: e/E must be followed by at
        // least one digit (optionally after a sign). Otherwise the 'e' is
        // the start of a unit/identifier (e.g. "10ex" CSS unit, "10em").
        if self.peek_byte() == Some(b'e') || self.peek_byte() == Some(b'E') {
            let e_pos = self.pos;
            let mut look = e_pos + 1;
            let mut has_exponent = false;
            if self.peek_n_byte(1) == Some(b'+') || self.peek_n_byte(1) == Some(b'-') {
                look += 1;
            }
            if let Some(b) = self.source.get(look) {
                if b.is_ascii_digit() {
                    has_exponent = true;
                }
            }
            if has_exponent {
                is_float = true;
                num.push(self.advance_unchecked() as char); // consume e/E
                if self.peek_byte() == Some(b'+') || self.peek_byte() == Some(b'-') {
                    num.push(self.advance_unchecked() as char);
                }
                while self.peek_byte().is_some_and(|b| b.is_ascii_digit()) {
                    num.push(self.advance_unchecked() as char);
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

    // ── Identifier and keyword readers ─────────────────────────────────────

    fn read_ident_or_keyword(&mut self) -> Token {
        let start = self.pos;
        let line = self.line;
        let col = self.col;
        let mut ident = String::new();
        while self.peek_char().is_some_and(|c| {
            c.is_ascii_alphanumeric()
                || c == '_'
                || (!c.is_ascii() && !c.is_whitespace() && !c.is_control())
        }) {
            ident.push(self.advance_char_unchecked());
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
            .peek_byte()
            .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        {
            kw.push(self.advance_unchecked() as char);
        }
        let kind = keywords::at_keyword_to_token(&kw).unwrap_or(TokenKind::Ident(kw));
        self.mk_token(kind, start, self.pos, line, col)
    }
}
