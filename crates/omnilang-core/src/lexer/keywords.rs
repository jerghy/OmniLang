//! Keyword matching for the lexer.
//!
//! Extracted from `read_ident_or_keyword` and `read_at_keyword` to keep
//! the main lexer file below 800 lines.

use super::TokenKind;

/// Map an identifier (without @ prefix) to a keyword TokenKind, or None if it's a plain identifier.
pub(super) fn ident_to_keyword(ident: &str) -> Option<TokenKind> {
    match ident {
        "fn" => Some(TokenKind::Fn),
        "const" => Some(TokenKind::Const),
        "let" => Some(TokenKind::Let),
        "mut" => Some(TokenKind::Mut),
        "if" => Some(TokenKind::If),
        "else" => Some(TokenKind::Else),
        "while" => Some(TokenKind::While),
        "for" => Some(TokenKind::For),
        "in" => Some(TokenKind::In),
        "return" => Some(TokenKind::Return),
        "break" => Some(TokenKind::Break),
        "continue" => Some(TokenKind::Continue),
        "print" => Some(TokenKind::Print),
        "println" => Some(TokenKind::Println),
        "as" => Some(TokenKind::As),
        "is" => Some(TokenKind::Is),
        "async" => Some(TokenKind::Async),
        "await" => Some(TokenKind::Await),
        "throw" => Some(TokenKind::Throw),
        "try" => Some(TokenKind::Try),
        "catch" => Some(TokenKind::Catch),
        "finally" => Some(TokenKind::Finally),
        "match" => Some(TokenKind::Match),
        "struct" => Some(TokenKind::Struct),
        "class" => Some(TokenKind::Class),
        "interface" => Some(TokenKind::Interface),
        "impl" => Some(TokenKind::Impl),
        "extends" => Some(TokenKind::Extends),
        "implements" => Some(TokenKind::Implements),
        "type" => Some(TokenKind::Type),
        "nominal" => Some(TokenKind::Nominal),
        "static" => Some(TokenKind::Static),
        "private" => Some(TokenKind::Private),
        "pub" => Some(TokenKind::Pub),
        "override" => Some(TokenKind::Override),
        "import" => Some(TokenKind::Import),
        "from" => Some(TokenKind::From),
        "export" => Some(TokenKind::Export),
        "true" => Some(TokenKind::True),
        "false" => Some(TokenKind::False),
        "null" => Some(TokenKind::Null),
        "component" => Some(TokenKind::Component),
        "signal" => Some(TokenKind::Signal),
        "computed" => Some(TokenKind::Computed),
        "prop" => Some(TokenKind::Prop),
        "event" => Some(TokenKind::Event),
        "ref" => Some(TokenKind::Ref),
        "store" => Some(TokenKind::Store),
        "view" => Some(TokenKind::View),
        "style" => Some(TokenKind::Style),
        "action" => Some(TokenKind::Action),
        "getter" => Some(TokenKind::Getter),
        "design" => Some(TokenKind::Design),
        "bind" => Some(TokenKind::Bind),
        "emit" => Some(TokenKind::Emit),
        // Motion keywords (context-sensitive, recognized in @motion blocks)
        "if_mode" => Some(TokenKind::IfMode),
        _ => None,
    }
}

/// Map an @-prefixed keyword string (including the @ sign) to a TokenKind.
pub(super) fn at_keyword_to_token(kw: &str) -> Option<TokenKind> {
    match kw {
        "@store" => Some(TokenKind::AtStore),
        // Navigation intent keywords (spec 2026-08-09 §2.1/§2.4/§2.6)
        "@route" => Some(TokenKind::AtRoute),
        "@layout" => Some(TokenKind::AtLayout),
        "@query" => Some(TokenKind::AtQuery),
        "@if" => Some(TokenKind::AtIf),
        "@else" => Some(TokenKind::AtElse),
        "@for" => Some(TokenKind::AtFor),
        "@empty" => Some(TokenKind::AtEmpty),
        "@slot" => Some(TokenKind::AtSlot),
        // Form/data-three-state intent keywords (spec 2026-08-08 §4.1)
        "@form" => Some(TokenKind::AtForm),
        "@load" => Some(TokenKind::AtLoad),
        // Feedback/modal intent keywords (spec 2026-08-09 §2.1/§2.3/§2.4)
        "@modal" => Some(TokenKind::AtModal),
        "@skeleton" => Some(TokenKind::AtSkeleton),
        "@errorBoundary" => Some(TokenKind::AtErrorBoundary),
        "@mount" => Some(TokenKind::AtMount),
        "@unmount" => Some(TokenKind::AtUnmount),
        "@effect" => Some(TokenKind::AtEffect),
        // Motion @-keywords
        "@motion" => Some(TokenKind::AtMotion),
        "@enter" => Some(TokenKind::AtEnter),
        "@hover" => Some(TokenKind::AtHover),
        "@press" => Some(TokenKind::AtPress),
        "@focus" => Some(TokenKind::AtFocus),
        "@in-view" => Some(TokenKind::AtInView),
        "@scroll" => Some(TokenKind::AtScroll),
        "@drag-horizontal" => Some(TokenKind::AtDragHorizontal),
        "@drag-vertical" => Some(TokenKind::AtDragVertical),
        // @hero support
        "@hero" => Some(TokenKind::AtHero),
        // @if-mode component prop
        "@if-mode" => Some(TokenKind::IfMode),
        // Unknown @-prefixed words are event bindings like @onPress, @onChange
        _ => None,
    }
}
