use crate::{
    ParserError,
    common::{expr::Expr, utils::expect_kind},
    keyword::Keyword,
    token::{TokenKind, TokenTable},
};

#[cfg(feature = "serde")]
use serde::{ser::SerializeStruct, Serialize, Serializer};

#[derive(Debug, PartialEq)]
pub struct Limit<'a> {
    pub offset: Option<Expr<'a>>,
    pub limit: Expr<'a>,
}

impl<'a> Limit<'a> {
    pub(crate) fn new(token_table: &TokenTable<'a>, cursor: &mut usize) -> Result<Self, ParserError> {
        expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Limit))?;
        *cursor += 1;

        let first = token_table
            .get_kind(*cursor)
            .map(|kind| kind == &TokenKind::Number)
            .unwrap_or(false);
        let comma = token_table
            .get_kind(*cursor + 1)
            .map(|kind| kind == &TokenKind::Comma)
            .unwrap_or(false);
        let offset = token_table
            .get_kind(*cursor + 1)
            .map(|kind| kind == &TokenKind::Keyword(Keyword::Offset))
            .unwrap_or(false);
        let second = token_table
            .get_kind(*cursor + 2)
            .map(|kind| kind == &TokenKind::Number)
            .unwrap_or(false);

        match (first, comma, offset, second) {
            (true, true, false, true) => {
                let offset = Expr::build(token_table, cursor)?;
                *cursor += 1;
                let limit = Expr::build(token_table, cursor)?;
                Ok(Limit {
                    offset: Some(offset),
                    limit,
                })
            }
            (true, false, true, true) => {
                let limit = Expr::build(token_table, cursor)?;
                *cursor += 1;
                let offset = Expr::build(token_table, cursor)?;
                Ok(Limit {
                    offset: Some(offset),
                    limit,
                })
            }
            (true, false, false, false) => {
                let limit = Expr::build(token_table, cursor)?;
                Ok(Limit {
                    offset: None,
                    limit,
                })
            }
            _ => Err(ParserError::SyntaxError(*cursor, *cursor)),
        }
    }
}

#[cfg(feature = "serde")]
impl<'a> Serialize for Limit<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("Limit", 2)?;
        s.serialize_field("offset", &self.offset)?;
        s.serialize_field("limit", &self.limit)?;
        s.end()
    }
}
