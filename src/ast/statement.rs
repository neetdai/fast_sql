use super::{ddl::DdlStatement, insert::InsertStatement, update::UpdateStatement};
use crate::{
    ast::{cte::Cte, delete::DeleteStatement, query::Query},
    error::ParserError,
    keyword::Keyword,
    token::{TokenKind, TokenTable},
};

#[cfg(feature = "serde")]
use serde::{ser::{SerializeStruct, SerializeSeq}, Serialize, Serializer};

#[derive(Debug, PartialEq)]
pub struct Statement<'a> {
    pub list: Vec<StatementInner<'a>>,
}

impl<'a> Statement<'a> {
    pub(crate) fn new(token_table: &TokenTable<'a>, cursor: &mut usize) -> Result<Self, ParserError> {
        let mut list = Vec::new();
        loop {
            match token_table.get_kind(*cursor) {
                Some(TokenKind::Delimiter) => {
                    *cursor += 1;
                    continue;
                } 
                None => {
                    break;
                }
                _ => {
                    let inner = StatementInner::new(token_table, cursor)?;
                    list.push(inner);
                }
            }
        }

        if list.is_empty() {
            return Err(ParserError::SyntaxError(*cursor, *cursor));
        }

        Ok(Self { list })
    }
}

#[cfg(feature = "serde")]
impl<'a> Serialize for Statement<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let len = self.list.len();
        let mut seq = serializer.serialize_seq(Some(len))?;
        for item in &self.list {
            seq.serialize_element(item)?;
        }
        seq.end()
    }
}

#[derive(Debug, PartialEq)]
pub enum StatementInner<'a> {
    Query(Query<'a>),
    Insert(InsertStatement<'a>),
    Update(UpdateStatement<'a>),
    Delete(DeleteStatement<'a>),
    Ddl(DdlStatement<'a>),
}

impl<'a> StatementInner<'a> {
    pub(crate) fn new(token_table: &TokenTable<'a>, cursor: &mut usize) -> Result<Self, ParserError> {
        Self::match_statement(token_table, cursor)
    }

    fn match_statement(
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
    ) -> Result<Self, ParserError> {
        match token_table.get_kind(*cursor) {
            Some(TokenKind::Keyword(Keyword::With)) => {
                let cte = Cte::build(token_table, cursor)?;
                let query = Query::build(token_table, cursor)?;
                Ok(Self::Query(Query::Cte {
                    ctes: cte.bindings,
                    query: Box::new(query),
                }))
            }
            Some(TokenKind::Keyword(Keyword::Select)) => {
                Query::build(token_table, cursor).map(Self::Query)
            }
            Some(TokenKind::Keyword(Keyword::Insert)) => {
                Ok(Self::Insert(InsertStatement::new(token_table, cursor)?))
            }
            Some(TokenKind::Keyword(Keyword::Update)) => {
                Ok(Self::Update(UpdateStatement::new(token_table, cursor)?))
            }
            Some(TokenKind::Keyword(Keyword::Delete)) => {
                Ok(Self::Delete(DeleteStatement::new(token_table, cursor)?))
            }
            Some(TokenKind::Keyword(Keyword::Create))
            | Some(TokenKind::Keyword(Keyword::Drop))
            | Some(TokenKind::Keyword(Keyword::Alter)) => {
                DdlStatement::build(token_table, cursor).map(Self::Ddl)
            }
            _ => Err(ParserError::SyntaxError(*cursor, *cursor)),
        }
    }
}

#[cfg(feature = "serde")]
impl<'a> Serialize for StatementInner<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Query(query) => serializer.serialize_newtype_variant("StatementInner", 0, "Query", query),
            Self::Insert(insert) => serializer.serialize_newtype_variant("StatementInner", 1, "Insert", insert),
            Self::Update(update) => serializer.serialize_newtype_variant("StatementInner", 2, "Update", update),
            Self::Delete(delete) => serializer.serialize_newtype_variant("StatementInner", 3, "Delete", delete),
            Self::Ddl(ddl) => serializer.serialize_newtype_variant("StatementInner", 4, "Ddl", ddl),
        }
    }
}