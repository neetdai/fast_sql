use minivec::MiniVec;

use crate::{
    ParserError,
    ast::query::Query,
    common::utils::{expect_kind, maybe_kind},
    keyword::Keyword,
    token::{TokenKind, TokenTable},
};

#[derive(Debug, PartialEq)]
pub struct CteBinding {
    pub name: usize,
    pub columns: Option<MiniVec<usize>>,
    pub query: Box<Query>,
}

#[derive(Debug, PartialEq)]
pub struct Cte {
    pub recursive: bool,
    pub bindings: MiniVec<CteBinding>,
}

impl CteBinding {
    fn new(token_table: &TokenTable, cursor: &mut usize) -> Result<Self, ParserError> {
        let name = match token_table.get_kind(*cursor) {
            Some(TokenKind::Identifier) => *cursor,
            _ => return Err(ParserError::SyntaxError(*cursor, *cursor)),
        };
        *cursor += 1;

        let columns = if maybe_kind(token_table, cursor, &TokenKind::LeftParen) {
            *cursor += 1;
            let mut cols = MiniVec::new();
            loop {
                match token_table.get_kind(*cursor) {
                    Some(TokenKind::Comma) => {
                        *cursor += 1;
                    }
                    Some(TokenKind::RightParen) => {
                        *cursor += 1;
                        break;
                    }
                    Some(TokenKind::Identifier) => {
                        cols.push(*cursor);
                        *cursor += 1;
                    }
                    _ => return Err(ParserError::SyntaxError(*cursor, *cursor)),
                }
            }
            Some(cols)
        } else {
            None
        };

        expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::As))?;
        *cursor += 1;

        // (SELECT ...) — 支持 UNION 等集合操作
        expect_kind(token_table, cursor, &TokenKind::LeftParen)?;
        *cursor += 1;
        let query = Box::new(Query::build(token_table, cursor)?);
        expect_kind(token_table, cursor, &TokenKind::RightParen)?;
        *cursor += 1;

        Ok(CteBinding {
            name,
            columns,
            query,
        })
    }
}

impl Cte {
    pub(crate) fn build(token_table: &TokenTable, cursor: &mut usize) -> Result<Self, ParserError> {
        expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::With))?;
        *cursor += 1;

        let recursive = if maybe_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Recursive))
        {
            *cursor += 1;
            true
        } else {
            false
        };

        let mut bindings = MiniVec::new();
        loop {
            match token_table.get_kind(*cursor) {
                Some(TokenKind::Comma) => {
                    *cursor += 1;
                }
                Some(TokenKind::Identifier) => {
                    bindings.push(CteBinding::new(token_table, cursor)?);
                }
                _ => break,
            }
        }

        if bindings.is_empty() {
            return Err(ParserError::SyntaxError(*cursor, *cursor));
        }

        Ok(Cte {
            recursive,
            bindings,
        })
    }
}
