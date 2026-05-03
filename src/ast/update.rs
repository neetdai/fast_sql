use minivec::MiniVec;

use crate::{
    ParserError,
    common::{
        expr::Expr,
        from::From,
        utils::{expect_kind, maybe_kind},
    },
    keyword::Keyword,
    token::{TokenKind, TokenTable},
};

#[derive(Debug, PartialEq)]
pub struct UpdateStatement<'a> {
    pub table: From<'a>,
    pub assignments: MiniVec<Expr<'a>>,
    pub where_statement: Option<Expr<'a>>,
}

impl<'a> UpdateStatement<'a> {
    pub fn new(token_table: &TokenTable<'a>, cursor: &mut usize) -> Result<Self, ParserError> {
        Self::build_ast(token_table, cursor)
    }

    fn build_ast(token_table: &TokenTable<'a>, cursor: &mut usize) -> Result<Self, ParserError> {
        expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Update))?;
        *cursor += 1;

        let table = From::parse(token_table, cursor)?;

        expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Set))?;
        *cursor += 1;

        let mut assignments = MiniVec::with_capacity(8);
        loop {
            match token_table.get_kind(*cursor) {
                Some(TokenKind::Comma) => {
                    *cursor += 1;
                }
                Some(TokenKind::Keyword(_)) => {
                    break;
                }
                Some(_) => {
                    assignments.push(Expr::build(token_table, cursor)?);
                }
                None => break,
            }
        }

        let where_statement =
            if maybe_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Where)) {
                *cursor += 1;
                Some(Expr::build(token_table, cursor)?)
            } else {
                None
            };

        Ok(Self {
            table,
            assignments,
            where_statement,
        })
    }
}
