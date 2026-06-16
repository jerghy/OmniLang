use omni_compiler::lexer::{tokenize, TokenKind};

#[test]
fn test_empty_source() {
    let tokens = tokenize("");
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].kind, TokenKind::Eof);
}

#[test]
fn test_keywords() {
    let tokens = tokenize("fn let mut if else while for in return");
    let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
    assert!(kinds.contains(&&TokenKind::Fn));
    assert!(kinds.contains(&&TokenKind::Let));
    assert!(kinds.contains(&&TokenKind::Mut));
    assert!(kinds.contains(&&TokenKind::If));
    assert!(kinds.contains(&&TokenKind::Return));
}

#[test]
fn test_identifiers() {
    let tokens = tokenize("hello world_123");
    assert_eq!(tokens[0].kind, TokenKind::Ident("hello".into()));
    assert_eq!(tokens[1].kind, TokenKind::Ident("world_123".into()));
}

#[test]
fn test_integers() {
    let tokens = tokenize("42 0xFF 0o77 0b1010");
    assert_eq!(tokens[0].kind, TokenKind::IntLiteral(42));
    assert_eq!(tokens[1].kind, TokenKind::IntLiteral(255)); // 0xFF
    assert_eq!(tokens[2].kind, TokenKind::IntLiteral(63));  // 0o77
    assert_eq!(tokens[3].kind, TokenKind::IntLiteral(10));  // 0b1010
}

#[test]
fn test_floats() {
    let tokens = tokenize("3.14 1.5e10 2.0E-3");
    assert!(matches!(tokens[0].kind, TokenKind::FloatLiteral(f) if (f - 3.14).abs() < 0.001));
    assert!(matches!(tokens[1].kind, TokenKind::FloatLiteral(_)));
    assert!(matches!(tokens[2].kind, TokenKind::FloatLiteral(_)));
}

#[test]
fn test_strings() {
    let tokens = tokenize(r#""hello" "world\n""#);
    assert_eq!(tokens[0].kind, TokenKind::StringLiteral("hello".into()));
    assert_eq!(tokens[1].kind, TokenKind::StringLiteral("world\n".into()));
}

#[test]
fn test_operators() {
    let tokens = tokenize("+ - * / % = == != < > <= >= && || !");
    let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
    assert!(kinds.contains(&&TokenKind::Plus));
    assert!(kinds.contains(&&TokenKind::Minus));
    assert!(kinds.contains(&&TokenKind::Star));
    assert!(kinds.contains(&&TokenKind::EqEq));
    assert!(kinds.contains(&&TokenKind::AndAnd));
    assert!(kinds.contains(&&TokenKind::OrOr));
    assert!(kinds.contains(&&TokenKind::Not));
}

#[test]
fn test_punctuation() {
    let tokens = tokenize("() {} [] ; , : . ...");
    let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
    assert!(kinds.contains(&&TokenKind::LParen));
    assert!(kinds.contains(&&TokenKind::RParen));
    assert!(kinds.contains(&&TokenKind::LBrace));
    assert!(kinds.contains(&&TokenKind::RBrace));
    assert!(kinds.contains(&&TokenKind::DotDotDot));
}

#[test]
fn test_at_directives() {
    let tokens = tokenize("@if @for @else @mount @effect @store @slot");
    let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
    assert!(kinds.contains(&&TokenKind::AtIf));
    assert!(kinds.contains(&&TokenKind::AtFor));
    assert!(kinds.contains(&&TokenKind::AtMount));
    assert!(kinds.contains(&&TokenKind::AtEffect));
}

#[test]
fn test_omni_keywords() {
    let tokens = tokenize("component signal computed prop event store view style action getter bind");
    let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
    assert!(kinds.contains(&&TokenKind::Component));
    assert!(kinds.contains(&&TokenKind::Signal));
    assert!(kinds.contains(&&TokenKind::Computed));
    assert!(kinds.contains(&&TokenKind::View));
    assert!(kinds.contains(&&TokenKind::Bind));
}

#[test]
fn test_comments_are_skipped() {
    let tokens = tokenize("// single line comment\nfn\n/* block comment */\nlet");
    // Should have: fn, let, eof
    assert_eq!(tokens[0].kind, TokenKind::Fn);
    assert_eq!(tokens[1].kind, TokenKind::Let);
    assert_eq!(tokens[2].kind, TokenKind::Eof);
}

#[test]
fn test_compound_assign() {
    let tokens = tokenize("+= -= *= /= %=");
    assert_eq!(tokens[0].kind, TokenKind::PlusEq);
    assert_eq!(tokens[1].kind, TokenKind::MinusEq);
    assert_eq!(tokens[2].kind, TokenKind::StarEq);
    assert_eq!(tokens[3].kind, TokenKind::SlashEq);
    assert_eq!(tokens[4].kind, TokenKind::PercentEq);
    assert_eq!(tokens[5].kind, TokenKind::Eof);
}
