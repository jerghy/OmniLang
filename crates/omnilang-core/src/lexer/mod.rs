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
        self.advance().unwrap_or(b'\0')
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
        self.advance_char().unwrap_or('\0')
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
        if self.peek_byte() == Some(b'.') && self.peek_n_byte(1).is_some_and(|b| b.is_ascii_digit())
        {
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── Test helpers ─────────────────────────────────────

    fn tokenize_all(source: &str) -> Vec<TokenKind> {
        tokenize(source).into_iter().map(|t| t.kind).collect()
    }

    fn assert_token(source: &str, expected: TokenKind) {
        let tokens = tokenize(source);
        if tokens.is_empty() {
            panic!("no tokens produced for {source:?}");
        }
        assert_eq!(tokens[0].kind, expected, "source: {source:?}");
    }

    fn assert_tokens(source: &str, expected: &[TokenKind]) {
        let kinds = tokenize_all(source);
        // Strip trailing Eof
        let kinds: Vec<&TokenKind> = kinds.iter().filter(|k| **k != TokenKind::Eof).collect();
        assert_eq!(
            kinds.len(),
            expected.len(),
            "token count mismatch for {source:?}\ngot:      {kinds:?}\nexpected: {expected:?}"
        );
        for (i, (got, exp)) in kinds.iter().zip(expected.iter()).enumerate() {
            assert_eq!(*got, exp, "token {i} mismatch for {source:?}");
        }
    }

    // ── Operators & Punctuation ──────────────────────────

    #[test]
    fn test_op_plus() {
        assert_token("+", TokenKind::Plus);
    }
    #[test]
    fn test_op_minus() {
        assert_token("-", TokenKind::Minus);
    }
    #[test]
    fn test_op_star() {
        assert_token("*", TokenKind::Star);
    }
    #[test]
    fn test_op_slash() {
        assert_token("/", TokenKind::Slash);
    }
    #[test]
    fn test_op_percent() {
        assert_token("%", TokenKind::Percent);
    }
    #[test]
    fn test_op_eq() {
        assert_token("=", TokenKind::Eq);
    }
    #[test]
    fn test_op_eq_eq() {
        assert_token("==", TokenKind::EqEq);
    }
    #[test]
    fn test_op_not_eq() {
        assert_token("!=", TokenKind::NotEq);
    }
    #[test]
    fn test_op_lt() {
        assert_token("<", TokenKind::Lt);
    }
    #[test]
    fn test_op_gt() {
        assert_token(">", TokenKind::Gt);
    }
    #[test]
    fn test_op_lteq() {
        assert_token("<=", TokenKind::LtEq);
    }
    #[test]
    fn test_op_gteq() {
        assert_token(">=", TokenKind::GtEq);
    }
    #[test]
    fn test_op_andand() {
        assert_token("&&", TokenKind::AndAnd);
    }
    #[test]
    fn test_op_oror() {
        assert_token("||", TokenKind::OrOr);
    }
    #[test]
    fn test_op_not() {
        assert_token("!", TokenKind::Not);
    }
    #[test]
    fn test_op_arrow() {
        assert_token("->", TokenKind::Arrow);
    }
    #[test]
    fn test_op_fat_arrow() {
        assert_token("=>", TokenKind::FatArrow);
    }
    #[test]
    fn test_op_dot() {
        assert_token(".", TokenKind::Dot);
    }
    #[test]
    fn test_op_dotdotdot() {
        assert_token("...", TokenKind::DotDotDot);
    }
    #[test]
    fn test_op_question_mark() {
        assert_token("?", TokenKind::QuestionMark);
    }
    #[test]
    fn test_op_question_question() {
        assert_token("??", TokenKind::QuestionQuestion);
    }
    #[test]
    fn test_op_question_dot() {
        assert_token("?.", TokenKind::QuestionDot);
    }
    #[test]
    fn test_op_plus_eq() {
        assert_token("+=", TokenKind::PlusEq);
    }
    #[test]
    fn test_op_minus_eq() {
        assert_token("-=", TokenKind::MinusEq);
    }
    #[test]
    fn test_op_star_eq() {
        assert_token("*=", TokenKind::StarEq);
    }
    #[test]
    fn test_op_slash_eq() {
        assert_token("/=", TokenKind::SlashEq);
    }
    #[test]
    fn test_op_percent_eq() {
        assert_token("%=", TokenKind::PercentEq);
    }
    #[test]
    fn test_op_pipe() {
        assert_token("|", TokenKind::Pipe);
    }
    #[test]
    fn test_op_dollar() {
        assert_token("$", TokenKind::Dollar);
    }
    #[test]
    fn test_op_hash() {
        assert_token("#", TokenKind::Hash);
    }
    #[test]
    fn test_op_lparen() {
        assert_token("(", TokenKind::LParen);
    }
    #[test]
    fn test_op_rparen() {
        assert_token(")", TokenKind::RParen);
    }
    #[test]
    fn test_op_lbrace() {
        assert_token("{", TokenKind::LBrace);
    }
    #[test]
    fn test_op_rbrace() {
        assert_token("}", TokenKind::RBrace);
    }
    #[test]
    fn test_op_lbracket() {
        assert_token("[", TokenKind::LBracket);
    }
    #[test]
    fn test_op_rbracket() {
        assert_token("]", TokenKind::RBracket);
    }
    #[test]
    fn test_op_semicolon() {
        assert_token(";", TokenKind::Semicolon);
    }
    #[test]
    fn test_op_comma() {
        assert_token(",", TokenKind::Comma);
    }
    #[test]
    fn test_op_colon() {
        assert_token(":", TokenKind::Colon);
    }
    #[test]
    fn test_op_at() {
        assert_token("@", TokenKind::Ident("@".into()));
    }

    // ── Keywords (omnilang) ──────────────────────────────

    #[test]
    fn test_keyword_fn() {
        assert_token("fn", TokenKind::Fn);
    }
    #[test]
    fn test_keyword_const() {
        assert_token("const", TokenKind::Const);
    }
    #[test]
    fn test_keyword_let() {
        assert_token("let", TokenKind::Let);
    }
    #[test]
    fn test_keyword_mut() {
        assert_token("mut", TokenKind::Mut);
    }
    #[test]
    fn test_keyword_if() {
        assert_token("if", TokenKind::If);
    }
    #[test]
    fn test_keyword_else() {
        assert_token("else", TokenKind::Else);
    }
    #[test]
    fn test_keyword_while() {
        assert_token("while", TokenKind::While);
    }
    #[test]
    fn test_keyword_for() {
        assert_token("for", TokenKind::For);
    }
    #[test]
    fn test_keyword_in() {
        assert_token("in", TokenKind::In);
    }
    #[test]
    fn test_keyword_return() {
        assert_token("return", TokenKind::Return);
    }
    #[test]
    fn test_keyword_break() {
        assert_token("break", TokenKind::Break);
    }
    #[test]
    fn test_keyword_continue() {
        assert_token("continue", TokenKind::Continue);
    }
    #[test]
    fn test_keyword_print() {
        assert_token("print", TokenKind::Print);
    }
    #[test]
    fn test_keyword_println() {
        assert_token("println", TokenKind::Println);
    }
    #[test]
    fn test_keyword_async() {
        assert_token("async", TokenKind::Async);
    }
    #[test]
    fn test_keyword_await() {
        assert_token("await", TokenKind::Await);
    }
    #[test]
    fn test_keyword_as() {
        assert_token("as", TokenKind::As);
    }
    #[test]
    fn test_keyword_throw() {
        assert_token("throw", TokenKind::Throw);
    }
    #[test]
    fn test_keyword_try() {
        assert_token("try", TokenKind::Try);
    }
    #[test]
    fn test_keyword_catch() {
        assert_token("catch", TokenKind::Catch);
    }
    #[test]
    fn test_keyword_finally() {
        assert_token("finally", TokenKind::Finally);
    }
    #[test]
    fn test_keyword_match() {
        assert_token("match", TokenKind::Match);
    }
    #[test]
    fn test_keyword_struct() {
        assert_token("struct", TokenKind::Struct);
    }
    #[test]
    fn test_keyword_class() {
        assert_token("class", TokenKind::Class);
    }
    #[test]
    fn test_keyword_interface() {
        assert_token("interface", TokenKind::Interface);
    }
    #[test]
    fn test_keyword_impl() {
        assert_token("impl", TokenKind::Impl);
    }
    #[test]
    fn test_keyword_extends() {
        assert_token("extends", TokenKind::Extends);
    }
    #[test]
    fn test_keyword_implements() {
        assert_token("implements", TokenKind::Implements);
    }
    #[test]
    fn test_keyword_type() {
        assert_token("type", TokenKind::Type);
    }
    #[test]
    fn test_keyword_nominal() {
        assert_token("nominal", TokenKind::Nominal);
    }
    #[test]
    fn test_keyword_static() {
        assert_token("static", TokenKind::Static);
    }
    #[test]
    fn test_keyword_private() {
        assert_token("private", TokenKind::Private);
    }
    #[test]
    fn test_keyword_pub() {
        assert_token("pub", TokenKind::Pub);
    }
    #[test]
    fn test_keyword_override() {
        assert_token("override", TokenKind::Override);
    }
    #[test]
    fn test_keyword_import() {
        assert_token("import", TokenKind::Import);
    }
    #[test]
    fn test_keyword_from() {
        assert_token("from", TokenKind::From);
    }
    #[test]
    fn test_keyword_export() {
        assert_token("export", TokenKind::Export);
    }
    #[test]
    fn test_keyword_true() {
        assert_token("true", TokenKind::True);
    }
    #[test]
    fn test_keyword_false() {
        assert_token("false", TokenKind::False);
    }
    #[test]
    fn test_keyword_null() {
        assert_token("null", TokenKind::Null);
    }

    // ── Keywords (omniview) ──────────────────────────────

    #[test]
    fn test_keyword_emit() {
        assert_token("emit", TokenKind::Emit);
    }
    #[test]
    fn test_keyword_component() {
        assert_token("component", TokenKind::Component);
    }
    #[test]
    fn test_keyword_signal() {
        assert_token("signal", TokenKind::Signal);
    }
    #[test]
    fn test_keyword_computed() {
        assert_token("computed", TokenKind::Computed);
    }
    #[test]
    fn test_keyword_prop() {
        assert_token("prop", TokenKind::Prop);
    }
    #[test]
    fn test_keyword_event() {
        assert_token("event", TokenKind::Event);
    }
    #[test]
    fn test_keyword_ref() {
        assert_token("ref", TokenKind::Ref);
    }
    #[test]
    fn test_keyword_store() {
        assert_token("store", TokenKind::Store);
    }
    #[test]
    fn test_keyword_view() {
        assert_token("view", TokenKind::View);
    }
    #[test]
    fn test_keyword_style() {
        assert_token("style", TokenKind::Style);
    }
    #[test]
    fn test_keyword_action() {
        assert_token("action", TokenKind::Action);
    }
    #[test]
    fn test_keyword_getter() {
        assert_token("getter", TokenKind::Getter);
    }
    #[test]
    fn test_keyword_design() {
        assert_token("design", TokenKind::Design);
    }

    // ── @-keywords ───────────────────────────────────────

    #[test]
    fn test_at_store() {
        assert_token("@store", TokenKind::AtStore);
    }
    #[test]
    fn test_at_if() {
        assert_token("@if", TokenKind::AtIf);
    }
    #[test]
    fn test_at_else() {
        assert_token("@else", TokenKind::AtElse);
    }
    #[test]
    fn test_at_for() {
        assert_token("@for", TokenKind::AtFor);
    }
    #[test]
    fn test_at_empty() {
        assert_token("@empty", TokenKind::AtEmpty);
    }
    #[test]
    fn test_at_slot() {
        assert_token("@slot", TokenKind::AtSlot);
    }
    #[test]
    fn test_at_native() {
        assert_token("@native", TokenKind::AtNative);
    }
    #[test]
    fn test_at_fallback() {
        assert_token("@fallback", TokenKind::AtFallback);
    }
    #[test]
    fn test_at_error() {
        assert_token("@error", TokenKind::AtError);
    }
    #[test]
    fn test_at_mount() {
        assert_token("@mount", TokenKind::AtMount);
    }
    #[test]
    fn test_at_unmount() {
        assert_token("@unmount", TokenKind::AtUnmount);
    }
    #[test]
    fn test_at_effect() {
        assert_token("@effect", TokenKind::AtEffect);
    }
    #[test]
    fn test_at_motion() {
        assert_token("@motion", TokenKind::AtMotion);
    }
    #[test]
    fn test_at_timeline() {
        assert_token("@timeline", TokenKind::AtTimeline);
    }
    #[test]
    fn test_at_enter() {
        assert_token("@enter", TokenKind::AtEnter);
    }
    #[test]
    fn test_at_leave() {
        assert_token("@leave", TokenKind::AtLeave);
    }
    #[test]
    fn test_at_hover() {
        assert_token("@hover", TokenKind::AtHover);
    }
    #[test]
    fn test_at_unhover() {
        assert_token("@unhover", TokenKind::AtUnhover);
    }
    #[test]
    fn test_at_press() {
        assert_token("@press", TokenKind::AtPress);
    }
    #[test]
    fn test_at_focus() {
        assert_token("@focus", TokenKind::AtFocus);
    }
    #[test]
    fn test_at_animate() {
        assert_token("@animate", TokenKind::AtAnimate);
    }
    #[test]
    fn test_at_inview() {
        assert_token("@inView", TokenKind::AtInView);
    }
    #[test]
    fn test_at_scroll() {
        assert_token("@scroll", TokenKind::AtScroll);
    }
    #[test]
    fn test_at_drag_horizontal() {
        assert_token("@dragHorizontal", TokenKind::AtDragHorizontal);
    }
    #[test]
    fn test_at_drag_vertical() {
        assert_token("@dragVertical", TokenKind::AtDragVertical);
    }
    #[test]
    fn test_at_driven_by() {
        assert_token("@driven-by", TokenKind::AtDrivenBy);
    }
    #[test]
    fn test_at_paused() {
        assert_token("@paused", TokenKind::AtPaused);
    }
    #[test]
    fn test_at_hover_reverse() {
        assert_token("@hover-reverse", TokenKind::AtHoverReverse);
    }
    #[test]
    fn test_at_inview_once() {
        assert_token("@in-view-once", TokenKind::AtInViewOnce);
    }
    #[test]
    fn test_at_inview_repeat() {
        assert_token("@in-view-repeat", TokenKind::AtInViewRepeat);
    }
    #[test]
    fn test_at_inview_threshold() {
        assert_token("@in-view-threshold", TokenKind::AtInViewThreshold);
    }
    #[test]
    fn test_at_scroll_start() {
        assert_token("@scroll-start", TokenKind::AtScrollStart);
    }
    #[test]
    fn test_at_scroll_end() {
        assert_token("@scroll-end", TokenKind::AtScrollEnd);
    }
    #[test]
    fn test_at_scroll_scrub() {
        assert_token("@scroll-scrub", TokenKind::AtScrollScrub);
    }
    #[test]
    fn test_at_hero() {
        assert_token("@hero", TokenKind::AtHero);
    }
    #[test]
    fn test_at_hero_transition() {
        assert_token("@hero-transition", TokenKind::AtHeroTransition);
    }
    #[test]
    fn test_keyword_if_mode() {
        assert_token("if_mode", TokenKind::IfMode);
    }
    #[test]
    fn test_keyword_bind() {
        assert_token("bind", TokenKind::Bind);
    }

    // ── String literals ──────────────────────────────────

    #[test]
    fn test_string_double_empty() {
        assert_token(r#""""#, TokenKind::StringLiteral(String::new()));
    }
    #[test]
    fn test_string_double_hello() {
        assert_token(r#""hello""#, TokenKind::StringLiteral("hello".into()));
    }
    #[test]
    fn test_string_double_escape_quote() {
        // Source string: "hello\"world" — the \" inside is an escape producing '"'
        assert_token(
            "\"hello\\\"world\"",
            TokenKind::StringLiteral("hello\"world".into()),
        );
    }
    #[test]
    fn test_string_double_escape_backslash() {
        assert_token(r#""a\\b""#, TokenKind::StringLiteral("a\\b".into()));
    }
    #[test]
    fn test_string_double_escape_newline() {
        assert_token(
            r#""line1\nline2""#,
            TokenKind::StringLiteral("line1\nline2".into()),
        );
    }
    #[test]
    fn test_string_double_escape_tab() {
        assert_token(
            r#""col1\tcol2""#,
            TokenKind::StringLiteral("col1\tcol2".into()),
        );
    }
    #[test]
    fn test_string_double_escape_carriage_return() {
        assert_token(r#""abc\rdef""#, TokenKind::StringLiteral("abc\rdef".into()));
    }
    #[test]
    fn test_string_double_escape_unicode() {
        assert_token(r#""\u{263A}""#, TokenKind::StringLiteral("☺".into()));
    }
    #[test]
    fn test_string_double_unicode_direct() {
        // Direct unicode in string (not escape)
        assert_token(
            "\"hello ☺ world\"",
            TokenKind::StringLiteral("hello ☺ world".into()),
        );
    }
    #[test]
    fn test_string_single_empty() {
        assert_token("''", TokenKind::StringLiteral(String::new()));
    }
    #[test]
    fn test_string_single_hello() {
        assert_token("'hello'", TokenKind::StringLiteral("hello".into()));
    }
    #[test]
    fn test_string_single_escape_quote() {
        assert_token(r#"'it\'s'"#, TokenKind::StringLiteral("it's".into()));
    }
    #[test]
    fn test_string_single_escape_backslash() {
        assert_token(r#"'a\\b'"#, TokenKind::StringLiteral("a\\b".into()));
    }
    #[test]
    fn test_template_string() {
        assert_token(
            r"`hello ${name}`",
            TokenKind::TemplateString("hello ${name}".into()),
        );
    }
    #[test]
    fn test_template_string_escape_backtick() {
        assert_token(
            r"`hello\`world`",
            TokenKind::TemplateString("hello`world".into()),
        );
    }

    // ── Number literals ──────────────────────────────────

    #[test]
    fn test_int_zero() {
        assert_token("0", TokenKind::IntLiteral(0));
    }
    #[test]
    fn test_int_simple() {
        assert_token("42", TokenKind::IntLiteral(42));
    }
    #[test]
    fn test_int_large() {
        assert_token("999999", TokenKind::IntLiteral(999999));
    }
    #[test]
    fn test_float_simple() {
        assert_token("3.14", TokenKind::FloatLiteral(3.14));
    }
    #[test]
    fn test_float_no_fraction() {
        // 42. is not a float — the lexer requires a digit after '.'
        assert_tokens("42.", &[TokenKind::IntLiteral(42), TokenKind::Dot]);
    }
    #[test]
    fn test_float_exponent() {
        assert_token("1.5e3", TokenKind::FloatLiteral(1500.0));
    }
    #[test]
    fn test_float_exponent_positive() {
        assert_token("1e+2", TokenKind::FloatLiteral(100.0));
    }
    #[test]
    fn test_float_exponent_negative() {
        assert_token("1e-2", TokenKind::FloatLiteral(0.01));
    }
    #[test]
    fn test_float_exponent_uppercase() {
        assert_token("2E10", TokenKind::FloatLiteral(20000000000.0));
    }
    #[test]
    fn test_hex_literal() {
        assert_token("0xFF", TokenKind::IntLiteral(255));
    }
    #[test]
    fn test_hex_literal_lowercase_x() {
        assert_token("0x1a", TokenKind::IntLiteral(26));
    }
    #[test]
    fn test_octal_literal() {
        assert_token("0o77", TokenKind::IntLiteral(63));
    }
    #[test]
    fn test_octal_literal_uppercase() {
        assert_token("0O10", TokenKind::IntLiteral(8));
    }
    #[test]
    fn test_binary_literal() {
        assert_token("0b1010", TokenKind::IntLiteral(10));
    }
    #[test]
    fn test_binary_literal_uppercase() {
        assert_token("0B1100", TokenKind::IntLiteral(12));
    }

    // ── Comments ─────────────────────────────────────────

    #[test]
    fn test_line_comment() {
        assert_tokens("// this is a comment\n42", &[TokenKind::IntLiteral(42)]);
    }
    #[test]
    fn test_line_comment_eof() {
        assert_tokens("// just a comment", &[]);
    }
    #[test]
    fn test_block_comment() {
        assert_tokens("/* block */42", &[TokenKind::IntLiteral(42)]);
    }
    #[test]
    fn test_block_comment_multiline() {
        assert_tokens("/* multi\nline */42", &[TokenKind::IntLiteral(42)]);
    }
    #[test]
    fn test_block_comment_unclosed() {
        let kinds = tokenize_all("/* unclosed");
        assert_eq!(kinds, vec![TokenKind::Eof]);
    }

    // ── Compound expressions ─────────────────────────────

    #[test]
    fn test_simple_addition() {
        assert_tokens(
            "1 + 2",
            &[
                TokenKind::IntLiteral(1),
                TokenKind::Plus,
                TokenKind::IntLiteral(2),
            ],
        );
    }
    #[test]
    fn test_if_else_expression() {
        assert_tokens(
            "if a == b { x } else { y }",
            &[
                TokenKind::If,
                TokenKind::Ident("a".into()),
                TokenKind::EqEq,
                TokenKind::Ident("b".into()),
                TokenKind::LBrace,
                TokenKind::Ident("x".into()),
                TokenKind::RBrace,
                TokenKind::Else,
                TokenKind::LBrace,
                TokenKind::Ident("y".into()),
                TokenKind::RBrace,
            ],
        );
    }
    #[test]
    fn test_function_call() {
        assert_tokens(
            "foo(bar, baz)",
            &[
                TokenKind::Ident("foo".into()),
                TokenKind::LParen,
                TokenKind::Ident("bar".into()),
                TokenKind::Comma,
                TokenKind::Ident("baz".into()),
                TokenKind::RParen,
            ],
        );
    }
    #[test]
    fn test_array_index() {
        assert_tokens(
            "arr[0]",
            &[
                TokenKind::Ident("arr".into()),
                TokenKind::LBracket,
                TokenKind::IntLiteral(0),
                TokenKind::RBracket,
            ],
        );
    }
    #[test]
    fn test_struct_literal() {
        assert_tokens(
            "Point { x: 1, y: 2 }",
            &[
                TokenKind::Ident("Point".into()),
                TokenKind::LBrace,
                TokenKind::Ident("x".into()),
                TokenKind::Colon,
                TokenKind::IntLiteral(1),
                TokenKind::Comma,
                TokenKind::Ident("y".into()),
                TokenKind::Colon,
                TokenKind::IntLiteral(2),
                TokenKind::RBrace,
            ],
        );
    }
    #[test]
    fn test_dot_access() {
        assert_tokens(
            "obj.field",
            &[
                TokenKind::Ident("obj".into()),
                TokenKind::Dot,
                TokenKind::Ident("field".into()),
            ],
        );
    }
    #[test]
    fn test_optional_chain() {
        assert_tokens(
            "obj?.field",
            &[
                TokenKind::Ident("obj".into()),
                TokenKind::QuestionDot,
                TokenKind::Ident("field".into()),
            ],
        );
    }
    #[test]
    fn test_nullish_coalescing() {
        assert_tokens(
            "a ?? b",
            &[
                TokenKind::Ident("a".into()),
                TokenKind::QuestionQuestion,
                TokenKind::Ident("b".into()),
            ],
        );
    }
    #[test]
    fn test_arrow_function() {
        assert_tokens(
            "x => x + 1",
            &[
                TokenKind::Ident("x".into()),
                TokenKind::FatArrow,
                TokenKind::Ident("x".into()),
                TokenKind::Plus,
                TokenKind::IntLiteral(1),
            ],
        );
    }
    #[test]
    fn test_let_statement() {
        assert_tokens(
            "let x = 42;",
            &[
                TokenKind::Let,
                TokenKind::Ident("x".into()),
                TokenKind::Eq,
                TokenKind::IntLiteral(42),
                TokenKind::Semicolon,
            ],
        );
    }

    // ── Edge cases ───────────────────────────────────────

    #[test]
    fn test_empty_source() {
        let kinds = tokenize_all("");
        assert_eq!(kinds, vec![TokenKind::Eof]);
    }
    #[test]
    fn test_only_whitespace() {
        let kinds = tokenize_all("   \t\n  \r\n  ");
        assert_eq!(kinds, vec![TokenKind::Eof]);
    }
    #[test]
    fn test_unicode_identifiers() {
        assert_tokens("你好", &[TokenKind::Ident("你好".into())]);
    }
    #[test]
    fn test_unicode_identifiers_mixed() {
        assert_tokens("_hello_世界", &[TokenKind::Ident("_hello_世界".into())]);
    }
    #[test]
    fn test_underscore_identifier() {
        assert_token("_myVar", TokenKind::Ident("_myVar".into()));
    }
    #[test]
    fn test_mixed_underscored_ident() {
        assert_token("my_var_1", TokenKind::Ident("my_var_1".into()));
    }
    #[test]
    fn test_multiple_statements() {
        assert_tokens(
            "a; b; c;",
            &[
                TokenKind::Ident("a".into()),
                TokenKind::Semicolon,
                TokenKind::Ident("b".into()),
                TokenKind::Semicolon,
                TokenKind::Ident("c".into()),
                TokenKind::Semicolon,
            ],
        );
    }
    #[test]
    fn test_nested_braces() {
        assert_tokens(
            "{ { { } } }",
            &[
                TokenKind::LBrace,
                TokenKind::LBrace,
                TokenKind::LBrace,
                TokenKind::RBrace,
                TokenKind::RBrace,
                TokenKind::RBrace,
            ],
        );
    }
    #[test]
    fn test_logical_operators_in_expression() {
        assert_tokens(
            "a && b || !c",
            &[
                TokenKind::Ident("a".into()),
                TokenKind::AndAnd,
                TokenKind::Ident("b".into()),
                TokenKind::OrOr,
                TokenKind::Not,
                TokenKind::Ident("c".into()),
            ],
        );
    }
    #[test]
    fn test_comparison_chain() {
        assert_tokens(
            "a < b && c >= d",
            &[
                TokenKind::Ident("a".into()),
                TokenKind::Lt,
                TokenKind::Ident("b".into()),
                TokenKind::AndAnd,
                TokenKind::Ident("c".into()),
                TokenKind::GtEq,
                TokenKind::Ident("d".into()),
            ],
        );
    }
    #[test]
    fn test_math_expression() {
        assert_tokens(
            "1 + 2 * 3 - 4 / 5",
            &[
                TokenKind::IntLiteral(1),
                TokenKind::Plus,
                TokenKind::IntLiteral(2),
                TokenKind::Star,
                TokenKind::IntLiteral(3),
                TokenKind::Minus,
                TokenKind::IntLiteral(4),
                TokenKind::Slash,
                TokenKind::IntLiteral(5),
            ],
        );
    }
}
