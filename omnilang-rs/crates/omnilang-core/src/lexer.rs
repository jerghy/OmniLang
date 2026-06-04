use crate::span::Span;

/// All token kinds in OmniLang v0.2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    // Keywords
    Fn,
    Let,
    Mut,
    Void,
    If,
    Else,
    While,
    For,
    In,
    Break,
    Continue,
    Return,
    True,
    False,
    Print,
    Println,
    Match,
    Struct,
    Class,
    Interface,
    Impl,
    Type,
    Import,
    Export,
    Async,
    Await,
    Nominal,
    As,
    From,
    Static,
    Override,
    New,
    Super,
    This,
    Get,
    Set,
    Private,
    Is,

    // Literals
    Number(String),          // raw number string: "42", "0xFF", "3.14"
    StringLit(String),       // the string content (without quotes, unescaped)
    TemplateString(String),  // raw content between backticks

    // Identifiers
    Ident(String),

    // Operators and assignment
    Plus,     // +
    Minus,    // -
    Star,     // *
    Slash,    // /
    Percent,  // %
    Eq,       // =
    EqEq,     // ==
    Ne,       // !=
    Lt,       // <
    Gt,       // >
    Le,       // <=
    Ge,       // >=
    AndAnd,   // &&
    OrOr,     // ||
    QuesQues, // ??
    Bang,     // !
    Dot,      // .
    DotDot,   // ..
    DotDotDot, // ...
    Arrow,    // ->
    Pipe,     // |
    FatArrow, // =>
    Question, // ?
    At,       // @
    Hash,     // #
    Underscore, // _

    // Compound assignment
    PlusEq,    // +=
    MinusEq,   // -=
    StarEq,    // *=
    SlashEq,   // /=
    PercentEq, // %=

    // Delimiters
    LParen,    // (
    RParen,    // )
    LBrace,    // {
    RBrace,    // }
    LBracket,  // [
    RBracket,  // ]
    Comma,     // ,
    Semicolon, // ;
    Colon,     // :
    ColonColon, // ::

    // End of file
    Eof,
}

impl TokenKind {
    /// Check if this token is a keyword.
    pub fn from_ident(name: &str) -> TokenKind {
        match name {
            "fn" => TokenKind::Fn,
            "let" => TokenKind::Let,
            "mut" => TokenKind::Mut,
            "void" => TokenKind::Void,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "return" => TokenKind::Return,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "print" => TokenKind::Print,
            "println" => TokenKind::Println,
            "match" => TokenKind::Match,
            "struct" => TokenKind::Struct,
            "class" => TokenKind::Class,
            "interface" => TokenKind::Interface,
            "impl" => TokenKind::Impl,
            "type" => TokenKind::Type,
            "import" => TokenKind::Import,
            "export" => TokenKind::Export,
            "async" => TokenKind::Async,
            "await" => TokenKind::Await,
            "nominal" => TokenKind::Nominal,
            "as" => TokenKind::As,
            "from" => TokenKind::From,
            "static" => TokenKind::Static,
            "override" => TokenKind::Override,
            "new" => TokenKind::New,
            "super" => TokenKind::Super,
            "this" => TokenKind::This,
            "get" => TokenKind::Get,
            "set" => TokenKind::Set,
            "private" => TokenKind::Private,
            "is" => TokenKind::Is,
            _ => TokenKind::Ident(name.to_string()),
        }
    }

    pub fn is_keyword(name: &str) -> bool {
        !matches!(Self::from_ident(name), TokenKind::Ident(_))
    }
}

/// A token with its source location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, file: impl Into<String>, line: u32, col: u32, end_line: u32, end_col: u32) -> Self {
        Self {
            kind,
            span: Span::new(file, line, col, end_line, end_col),
        }
    }
}

/// Lexer/Scanner for OmniLang source code.
/// Converts source text into a stream of tokens.
pub struct Lexer {
    source: Vec<char>,
    pos: usize,
    line: u32,
    col: u32,
    filename: String,
    errors: Vec<String>,
}

impl Lexer {
    pub fn new(source: &str, filename: impl Into<String>) -> Self {
        Self {
            source: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
            filename: filename.into(),
            errors: Vec::new(),
        }
    }

    fn current(&self) -> Option<char> {
        self.source.get(self.pos).copied()
    }

    fn peek(&self, offset: usize) -> Option<char> {
        self.source.get(self.pos + offset).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.current();
        if let Some(c) = ch {
            self.pos += 1;
            if c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
        ch
    }

    fn make_span(&self, start_line: u32, start_col: u32) -> Span {
        Span::new(
            self.filename.clone(),
            start_line,
            start_col,
            self.line,
            self.col,
        )
    }

    /// Tokenize the entire source and return tokens (or fatal errors).
    pub fn tokenize(&mut self) -> Result<Vec<Token>, Vec<String>> {
        let mut tokens = Vec::new();

        loop {
            let start_line = self.line;
            let start_col = self.col;

            let ch = match self.current() {
                None => {
                    tokens.push(Token::new(
                        TokenKind::Eof,
                        self.filename.clone(),
                        start_line, start_col, start_line, start_col,
                    ));
                    break;
                }
                Some(c) => c,
            };

            let kind = match ch {
                // Whitespace
                ' ' | '\t' | '\r' | '\n' => {
                    self.advance();
                    continue;
                }

                // Comments
                '/' if self.peek(1) == Some('/') => {
                    // Single-line comment
                    self.advance(); // skip first /
                    self.advance(); // skip second /
                    while let Some(c) = self.current() {
                        if c == '\n' {
                            break;
                        }
                        self.advance();
                    }
                    continue;
                }
                '/' if self.peek(1) == Some('*') => {
                    // Block comment
                    self.advance(); // skip /
                    self.advance(); // skip *
                    let mut depth = 1;
                    while depth > 0 {
                        match self.current() {
                            None => {
                                self.errors.push(format!(
                                    "Unterminated block comment at {}:{}",
                                    start_line, start_col
                                ));
                                break;
                            }
                            Some('*') if self.peek(1) == Some('/') => {
                                self.advance();
                                self.advance();
                                depth -= 1;
                            }
                            Some('/') if self.peek(1) == Some('*') => {
                                self.advance();
                                self.advance();
                                depth += 1;
                            }
                            _ => {
                                self.advance();
                            }
                        }
                    }
                    continue;
                }

                // Operators and compound symbols
                '=' => {
                    self.advance();
                    if self.current() == Some('=') {
                        self.advance();
                        TokenKind::EqEq
                    } else if self.current() == Some('>') {
                        self.advance();
                        TokenKind::FatArrow
                    } else {
                        TokenKind::Eq
                    }
                }
                '+' => {
                    self.advance();
                    if self.current() == Some('=') {
                        self.advance();
                        TokenKind::PlusEq
                    } else {
                        TokenKind::Plus
                    }
                }
                '-' => {
                    self.advance();
                    if self.current() == Some('=') {
                        self.advance();
                        TokenKind::MinusEq
                    } else if self.current() == Some('>') {
                        self.advance();
                        TokenKind::Arrow
                    } else {
                        TokenKind::Minus
                    }
                }
                '*' => {
                    self.advance();
                    if self.current() == Some('=') {
                        self.advance();
                        TokenKind::StarEq
                    } else {
                        TokenKind::Star
                    }
                }
                '/' => {
                    self.advance();
                    if self.current() == Some('=') {
                        self.advance();
                        TokenKind::SlashEq
                    } else {
                        TokenKind::Slash
                    }
                }
                '%' => {
                    self.advance();
                    if self.current() == Some('=') {
                        self.advance();
                        TokenKind::PercentEq
                    } else {
                        TokenKind::Percent
                    }
                }
                '<' => {
                    self.advance();
                    if self.current() == Some('=') {
                        self.advance();
                        TokenKind::Le
                    } else {
                        TokenKind::Lt
                    }
                }
                '>' => {
                    self.advance();
                    if self.current() == Some('=') {
                        self.advance();
                        TokenKind::Ge
                    } else {
                        TokenKind::Gt
                    }
                }
                '!' => {
                    self.advance();
                    if self.current() == Some('=') {
                        self.advance();
                        TokenKind::Ne
                    } else {
                        TokenKind::Bang
                    }
                }
                '&' => {
                    self.advance();
                    if self.current() == Some('&') {
                        self.advance();
                        TokenKind::AndAnd
                    } else {
                        // Single & is not a valid token in OmniLang
                        self.errors.push(format!(
                            "Unexpected character '&' at {}:{}",
                            start_line, start_col
                        ));
                        self.advance();
                        continue;
                    }
                }
                '|' => {
                    self.advance();
                    if self.current() == Some('|') {
                        self.advance();
                        TokenKind::OrOr
                    } else {
                        TokenKind::Pipe
                    }
                }
                '?' => {
                    self.advance();
                    if self.current() == Some('?') {
                        self.advance();
                        TokenKind::QuesQues
                    } else if self.current() == Some('.') {
                        // ?. is handled by the parser as a context-sensitive token
                        // We emit them separately
                        TokenKind::Question
                    } else {
                        TokenKind::Question
                    }
                }
                '.' => {
                    self.advance();
                    if self.current() == Some('.') {
                        self.advance();
                        if self.current() == Some('.') {
                            self.advance();
                            TokenKind::DotDotDot
                        } else {
                            TokenKind::DotDot
                        }
                    } else {
                        TokenKind::Dot
                    }
                }
                ':' => {
                    self.advance();
                    if self.current() == Some(':') {
                        self.advance();
                        TokenKind::ColonColon
                    } else {
                        TokenKind::Colon
                    }
                }
                '@' => {
                    self.advance();
                    TokenKind::At
                }
                '#' => {
                    self.advance();
                    TokenKind::Hash
                }
                '_' => {
                    self.advance();
                    // Could be underscore identifier or standalone wildcard
                    if self.current().map_or(false, |c| c.is_alphanumeric() || c == '_') {
                        let name = self.read_identifier("_");
                        TokenKind::from_ident(&format!("_{}", name))
                    } else {
                        TokenKind::Underscore
                    }
                }
                '(' => { self.advance(); TokenKind::LParen }
                ')' => { self.advance(); TokenKind::RParen }
                '{' => { self.advance(); TokenKind::LBrace }
                '}' => { self.advance(); TokenKind::RBrace }
                '[' => { self.advance(); TokenKind::LBracket }
                ']' => { self.advance(); TokenKind::RBracket }
                ',' => { self.advance(); TokenKind::Comma }
                ';' => { self.advance(); TokenKind::Semicolon }

                // String literal
                '"' => {
                    self.advance(); // skip opening "
                    let s = self.read_string('"');
                    TokenKind::StringLit(s)
                }

                // Template string
                '`' => {
                    self.advance(); // skip opening `
                    let s = self.read_template_string();
                    TokenKind::TemplateString(s)
                }

                // Numbers
                '0'..='9' => {
                    let num = self.read_number();
                    TokenKind::Number(num)
                }

                // Identifiers
                c if c.is_alphabetic() => {
                    let name = self.read_identifier("");
                    TokenKind::from_ident(&name)
                }

                _ => {
                    self.errors.push(format!(
                        "Unexpected character '{}' at {}:{}",
                        ch, start_line, start_col
                    ));
                    self.advance();
                    continue;
                }
            };

            tokens.push(Token::new(kind, self.filename.clone(), start_line, start_col, self.line, self.col));
        }

        if !self.errors.is_empty() {
            Err(self.errors.clone())
        } else {
            Ok(tokens)
        }
    }

    fn read_identifier(&mut self, prefix: &str) -> String {
        let mut name = prefix.to_string();
        while let Some(c) = self.current() {
            if c.is_alphanumeric() || c == '_' {
                name.push(c);
                self.advance();
            } else {
                break;
            }
        }
        name
    }

    fn read_string(&mut self, delimiter: char) -> String {
        let mut s = String::new();
        while let Some(c) = self.current() {
            if c == delimiter {
                self.advance(); // skip closing quote
                break;
            } else if c == '\\' {
                self.advance(); // skip backslash
                match self.current() {
                    Some('n') => { s.push('\n'); self.advance(); }
                    Some('t') => { s.push('\t'); self.advance(); }
                    Some('\\') => { s.push('\\'); self.advance(); }
                    Some('"') => { s.push('"'); self.advance(); }
                    Some('\'') => { s.push('\''); self.advance(); }
                    Some('r') => { s.push('\r'); self.advance(); }
                    Some('0') => { s.push('\0'); self.advance(); }
                    Some(c2) => {
                        // Unknown escape, keep as-is
                        s.push('\\');
                        s.push(c2);
                        self.advance();
                    }
                    None => break,
                }
            } else {
                s.push(c);
                self.advance();
            }
        }
        s
    }

    fn read_template_string(&mut self) -> String {
        let mut s = String::new();
        while let Some(c) = self.current() {
            if c == '`' {
                self.advance(); // skip closing backtick
                break;
            } else if c == '\\' {
                self.advance();
                if let Some(c2) = self.current() {
                    match c2 {
                        '`' => { s.push('`'); self.advance(); }
                        '$' => { s.push('$'); self.advance(); }
                        '\\' => { s.push('\\'); self.advance(); }
                        'n' => { s.push('\n'); self.advance(); }
                        't' => { s.push('\t'); self.advance(); }
                        _ => {
                            s.push('\\');
                            s.push(c2);
                            self.advance();
                        }
                    }
                }
            } else {
                s.push(c);
                self.advance();
            }
        }
        s
    }

    fn read_number(&mut self) -> String {
        let mut num = String::new();

        // Check for hex/octal/binary prefix
        if self.current() == Some('0') {
            num.push('0');
            self.advance();
            match self.current() {
                Some('x') | Some('X') => {
                    num.push('x');
                    self.advance();
                    while let Some(c) = self.current() {
                        if c.is_ascii_hexdigit() || c == '_' {
                            if c != '_' { num.push(c); }
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    return num;
                }
                Some('o') | Some('O') => {
                    num.push('o');
                    self.advance();
                    while let Some(c) = self.current() {
                        if ('0'..='7').contains(&c) || c == '_' {
                            if c != '_' { num.push(c); }
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    return num;
                }
                Some('b') | Some('B') => {
                    num.push('b');
                    self.advance();
                    while let Some(c) = self.current() {
                        if c == '0' || c == '1' || c == '_' {
                            if c != '_' { num.push(c); }
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    return num;
                }
                _ => {}
            }
        }

        // Decimal integer part
        while let Some(c) = self.current() {
            if c.is_ascii_digit() {
                num.push(c);
                self.advance();
            } else {
                break;
            }
        }

        // Fractional part
        if self.current() == Some('.') && self.peek(1).map_or(false, |c| c.is_ascii_digit()) {
            num.push('.');
            self.advance();
            while let Some(c) = self.current() {
                if c.is_ascii_digit() {
                    num.push(c);
                    self.advance();
                } else {
                    break;
                }
            }
        }

        // Exponent
        if self.current() == Some('e') || self.current() == Some('E') {
            num.push('e');
            self.advance();
            if self.current() == Some('-') || self.current() == Some('+') {
                num.push(self.current().unwrap());
                self.advance();
            }
            while let Some(c) = self.current() {
                if c.is_ascii_digit() {
                    num.push(c);
                    self.advance();
                } else {
                    break;
                }
            }
        }

        num
    }

    pub fn get_errors(&self) -> &[String] {
        &self.errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokenize(source: &str) -> Vec<Token> {
        let mut lexer = Lexer::new(source, "<test>");
        lexer.tokenize().unwrap()
    }

    fn kind_str(tokens: &[Token]) -> Vec<String> {
        tokens.iter().map(|t| format!("{:?}", t.kind)).collect()
    }

    #[test]
    fn test_keywords() {
        let tokens = tokenize("fn let mut void if else while for in break continue return");
        let kinds = kind_str(&tokens);
        assert!(kinds.contains(&"Fn".to_string()));
        assert!(kinds.contains(&"Let".to_string()));
        assert!(kinds.contains(&"Mut".to_string()));
    }

    #[test]
    fn test_string_literal() {
        let tokens = tokenize(r#""hello\nworld""#);
        match &tokens[0].kind {
            TokenKind::StringLit(s) => assert_eq!(s, "hello\nworld"),
            _ => panic!("Expected StringLit"),
        }
    }

    #[test]
    fn test_number_literals() {
        let tokens = tokenize("42 0xFF 3.14 1e9");
        let kinds = kind_str(&tokens);
        assert!(kinds[0] == r#"Number("42")"#);
        assert!(kinds[1] == r#"Number("0xFF")"#);
        assert!(kinds[2] == r#"Number("3.14")"#);
        assert!(kinds[3] == r#"Number("1e9")"#);
    }

    #[test]
    fn test_operators() {
        let tokens = tokenize("+ - * / % = == != < > <= >= && || ?? !");
        let kinds: Vec<String> = tokens.iter().filter(|t| t.kind != TokenKind::Eof).map(|t| format!("{:?}", t.kind)).collect();
        assert_eq!(kinds, vec![
            "Plus", "Minus", "Star", "Slash", "Percent",
            "Eq", "EqEq", "Ne", "Lt", "Gt", "Le", "Ge",
            "AndAnd", "OrOr", "QuesQues", "Bang",
        ]);
    }

    #[test]
    fn test_comments() {
        // Single-line comment
        let tokens = tokenize("// this is a comment\nfn");
        let kinds = kind_str(&tokens);
        assert!(kinds[0].contains("Fn"));

        // Block comment
        let tokens = tokenize("let /* inline */ x");
        let kinds = kind_str(&tokens);
        assert!(kinds[0].contains("Let"));
        assert!(kinds[1].contains("Ident"));
    }

    #[test]
    fn test_template_string() {
        let tokens = tokenize("`hello ${name}`");
        match &tokens[0].kind {
            TokenKind::TemplateString(s) => assert_eq!(s, "hello ${name}"),
            _ => panic!("Expected TemplateString"),
        }
    }
}
