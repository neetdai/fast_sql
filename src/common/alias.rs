
use crate::{
    ParserError,
    keyword::Keyword,
    token::{TokenKind, TokenTable},
};

#[cfg(feature = "serde")]
use serde::{ser::SerializeStruct, Serialize, Serializer};

pub trait Aliasable<'a>: Sized {
    fn aliasable(token_table: &TokenTable<'a>, cursor: &mut usize) -> Result<Self, ParserError>;
}

#[derive(Debug, PartialEq)]
pub struct Alias<'a, T> {
    pub name: Option<&'a str>,
    pub value: T,
}

impl<'a, T> Alias<'a, T>
where
    T: Aliasable<'a>,
{
    pub(crate) fn new(
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
    ) -> Result<Self, ParserError> {
        let value = T::aliasable(token_table, cursor)?;
        match token_table.get_kind(*cursor) {
            Some(TokenKind::Keyword(Keyword::As)) => {
                *cursor += 1;
                if let Some(TokenKind::Identifier) = token_table.get_kind(*cursor) {
                    let name = token_table.source_at(*cursor);
                    *cursor += 1;
                    Ok(Alias {
                        name: Some(name),
                        value,
                    })
                } else {
                    Err(ParserError::SyntaxError(*cursor, *cursor))
                }
            }
            Some(TokenKind::Identifier) => {
                let name = token_table.source_at(*cursor);
                *cursor += 1;
                Ok(Alias {
                    name: Some(name),
                    value,
                })
            }
            _ => Ok(Alias { name: None, value }),
        }
    }
}

#[cfg(feature = "serde")]
impl<'a, T: Serialize> Serialize for Alias<'a, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("Alias", 2)?;
        s.serialize_field("name", &self.name)?;
        s.serialize_field("value", &self.value)?;
        s.end()
    }
}