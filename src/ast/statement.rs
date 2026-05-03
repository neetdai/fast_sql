use super::{insert::InsertStatement, update::UpdateStatement};
use crate::{
    ast::{cte::Cte, delete::DeleteStatement, query::Query},
    error::ParserError,
    keyword::Keyword,
    token::{TokenKind, TokenTable},
};

#[derive(Debug, PartialEq)]
pub enum Statement<'a> {
    Query(Query<'a>),
    Insert(InsertStatement<'a>),
    Update(UpdateStatement<'a>),
    Delete(DeleteStatement<'a>),
}

impl<'a> Statement<'a> {
    pub fn new(token_table: &TokenTable<'a>, cursor: &mut usize) -> Result<Self, ParserError> {
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
            _ => Err(ParserError::SyntaxError(*cursor, *cursor)),
        }
    }
}
