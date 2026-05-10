use std::{slice::SliceIndex};

use strum::Display;

use crate::keyword::Keyword;

#[derive(Debug, Clone, PartialEq, Eq, Display)]
#[repr(u16)]
pub enum TokenKind {
    Number,
    StringLiteral,
    Identifier,
    Delimiter,
    Dot,
    LeftParen,
    RightParen,
    LeftShift,
    RightShift,
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
    BitXor,
    BitAnd,
    Or,
    Keyword(Keyword),
}

#[derive(Debug)]
pub struct TokenTable<'a> {
    pub tokens: Vec<TokenKind>,
    pub source_ref_list: Vec<&'a str>,
}

impl<'a> TokenTable<'a> {
    pub(crate) fn with_source(source: &'a str) -> Self {
        let cap = source.len() / 4;
        Self {
            tokens: Vec::with_capacity(cap),
            source_ref_list: Vec::with_capacity(cap),
        }
    }

    pub(crate) fn push(&mut self, kind: TokenKind, source_ref: &'a str) {
        self.tokens.push(kind);
        self.source_ref_list.push(source_ref);
    }

    pub(crate) fn source_at(&self, cursor: usize) -> &'a str {
        self.source_ref_list[cursor]
    }

    pub(crate) fn get_kind<I>(&self, index: I) -> Option<&I::Output>
    where
        I: SliceIndex<[TokenKind]>,
    {
        self.tokens.get(index)
    }
}
