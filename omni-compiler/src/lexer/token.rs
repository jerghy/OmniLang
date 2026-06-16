//! Token types and Span for the lexer.

use serde::{Deserialize, Serialize};

/// All token kinds produced by the lexer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TokenKind {
    // Keywords (omnilang)
    Fn, Let, Mut, If, Else, While, For, In, Return, Break, Continue,
    Print, Println, Async, Await, Match,
    Struct, Class, Interface, Impl, Extends, Implements,
    Type, Nominal, Static, Private, Override,
    Import, From, Export,
    True, False, Null,

    // Keywords (omniview)
    Component, Signal, Computed, Prop, Event, Ref, Store, View, Style,
    Action, Getter, Theme,
    AtStore, AtIf, AtElse, AtFor, AtEmpty, AtSlot,
    AtNative, AtFallback, AtError,
    AtMount, AtUnmount, AtEffect,
    Bind,

    // Literals
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    TemplateString(String),

    // Identifiers
    Ident(String),

    // Operators & Punctuation
    Dot, DotDotDot, QuestionMark, QuestionDot,
    Plus, Minus, Star, Slash, Percent,
    PlusEq, MinusEq, StarEq, SlashEq, PercentEq,
    Eq, EqEq, NotEq, Lt, Gt, LtEq, GtEq,
    AndAnd, OrOr, Not,
    Arrow, FatArrow, Pipe,
    LParen, RParen, LBrace, RBrace, LBracket, RBracket,
    Semicolon, Comma, Colon, At,

    // Special
    Eof,
    Error(String),
}

/// Source location span (byte offsets + line/col).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: u32,
    pub col: u32,
}

/// A single token with its kind and source location.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, start: usize, end: usize, line: u32, col: u32) -> Self {
        Token {
            kind,
            span: Span {
                start,
                end,
                line,
                col,
            },
        }
    }
}
