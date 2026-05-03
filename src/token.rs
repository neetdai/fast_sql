use std::{borrow::Cow, fmt::Display, slice::SliceIndex};

use crate::keyword::Keyword;

#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(u16)]
pub enum TokenKind {
    Number,
    StringLiteral,
    Identifier,
    Eof,
    Dot,
    LeftParen,
    RightParen,
    Comma,
    Unknown,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Equal,
    NotEqual,
    Plus,
    Subtract,
    Multiply,
    Divide,
    Mod,
    Xor,
    And,
    Or,
    Keyword(Keyword),
}

impl Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenKind::Number => write!(f, "Number"),
            TokenKind::StringLiteral => write!(f, "StringLiteral"),
            TokenKind::Identifier => write!(f, "Identifier"),
            TokenKind::Eof => write!(f, "Eof"),
            TokenKind::Dot => write!(f, "Dot"),
            TokenKind::LeftParen => write!(f, "LeftParen"),
            TokenKind::RightParen => write!(f, "RightParen"),
            TokenKind::Comma => write!(f, "Comma"),
            TokenKind::Unknown => write!(f, "Unknown"),
            TokenKind::Less => write!(f, "Less"),
            TokenKind::LessEqual => write!(f, "LessEqual"),
            TokenKind::Greater => write!(f, "Greater"),
            TokenKind::GreaterEqual => write!(f, "GreaterEqual"),
            TokenKind::Equal => write!(f, "Equal"),
            TokenKind::NotEqual => write!(f, "NotEqual"),
            TokenKind::Plus => write!(f, "Plus"),
            TokenKind::Subtract => write!(f, "Subtract"),
            TokenKind::Multiply => write!(f, "Multiply"),
            TokenKind::Divide => write!(f, "Divide"),
            TokenKind::Mod => write!(f, "Mod"),
            TokenKind::Xor => write!(f, "Xor"),
            TokenKind::And => write!(f, "And"),
            TokenKind::Or => write!(f, "Or"),
            TokenKind::Keyword(kw) => kw.fmt(f),
        }
    }
}

#[derive(Debug)]
pub struct TokenTable<'a> {
    pub tokens: Vec<TokenKind>,
    pub source_ref_list: Vec<Cow<'a, str>>,
}

impl<'a> TokenTable<'a> {
    pub(crate) fn with_source(source: &'a str) -> Self {
        let cap = source.len() / 4;
        Self {
            tokens: Vec::with_capacity(cap),
            source_ref_list: Vec::with_capacity(cap),
        }
    }

    pub(crate) fn push(&mut self, kind: TokenKind, source_ref: Cow<'a, str>) {
        self.tokens.push(kind);
        self.source_ref_list.push(source_ref);
    }

    pub(crate) fn source_at(&self, cursor: usize) -> Cow<'a, str> {
        self.source_ref_list[cursor].clone()
    }

    pub(crate) fn get_kind<I>(&self, index: I) -> Option<&I::Output>
    where
        I: SliceIndex<[TokenKind]>,
    {
        self.tokens.get(index)
    }
}
