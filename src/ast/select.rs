use minivec::MiniVec;

use crate::{
    ParserError,
    common::{
        alias::{Alias, Aliasable},
        expr::Expr,
        from::From,
        group::Group,
        limit::Limit,
        order::Order,
        utils::{expect_kind, maybe_kind},
    },
    keyword::Keyword,
    token::{TokenKind, TokenTable},
};

#[derive(Debug, PartialEq)]
pub struct SelectStatement<'a> {
    pub distinct: bool,
    pub columns: Vec<Alias<'a, Expr<'a>>>,
    pub from: Option<MiniVec<From<'a>>>,
    pub where_statement: Option<Expr<'a>>,
    pub group_by: Option<Group<'a>>,
    pub having_statement: Option<Expr<'a>>,
    pub order_by: Option<Order<'a>>,
    pub limit: Option<Limit<'a>>,
}

impl<'a> SelectStatement<'a> {
    pub fn new(token_table: &TokenTable<'a>, cursor: &mut usize) -> Result<Self, ParserError> {
        Self::build_ast(token_table, cursor)
    }

    fn build_ast(token_table: &TokenTable<'a>, cursor: &mut usize) -> Result<Self, ParserError> {
        expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Select))?;
        *cursor += 1;

        let distinct =
            if let Some(TokenKind::Keyword(Keyword::Distinct)) = token_table.get_kind(*cursor) {
                *cursor += 1;
                true
            } else {
                false
            };

        let mut columns = Vec::new();
        loop {
            match token_table.get_kind(*cursor) {
                Some(TokenKind::Comma) => {
                    *cursor += 1;
                }
                Some(TokenKind::Keyword(Keyword::Case)) => {
                    let expr = Alias::new(token_table, cursor)?;
                    columns.push(expr);
                }
                Some(TokenKind::Keyword(_)) => break,
                Some(_) => {
                    let expr = Alias::new(token_table, cursor)?;
                    columns.push(expr);
                }
                None => break,
            }
        }

        let from = if maybe_kind(token_table, cursor, &TokenKind::Keyword(Keyword::From)) {
            *cursor += 1;

            let mut list = MiniVec::new();
            loop {
                match token_table.get_kind(*cursor) {
                    Some(TokenKind::Comma) => {
                        *cursor += 1;
                    }
                    Some(TokenKind::RightParen)
                    | Some(TokenKind::Keyword(_))
                    | Some(TokenKind::Eof) => break,
                    Some(_) => {
                        list.push(From::parse(token_table, cursor)?);
                    }
                    None => break,
                }
            }
            Some(list)
        } else {
            None
        };

        let where_statement =
            if maybe_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Where)) {
                *cursor += 1;
                Some(Expr::build(token_table, cursor)?)
            } else {
                None
            };

        let group_by = if maybe_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Group)) {
            Some(Group::build(token_table, cursor)?)
        } else {
            None
        };

        let having_statement =
            if maybe_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Having)) {
                *cursor += 1;
                Some(Expr::build(token_table, cursor)?)
            } else {
                None
            };

        let order_by = if maybe_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Order)) {
            Some(Order::build(token_table, cursor)?)
        } else {
            None
        };

        let limit = if maybe_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Limit)) {
            Some(Limit::new(token_table, cursor)?)
        } else {
            None
        };

        Ok(Self {
            columns,
            from,
            where_statement,
            group_by,
            having_statement,
            order_by,
            limit,
            distinct,
        })
    }
}

pub type SubSelectStatement<'a> = Box<SelectStatement<'a>>;

impl<'a> Aliasable<'a> for SubSelectStatement<'a> {
    fn aliasable(token_table: &TokenTable<'a>, cursor: &mut usize) -> Result<Self, ParserError> {
        expect_kind(token_table, cursor, &TokenKind::LeftParen)?;
        *cursor += 1;
        let select_stmt = SelectStatement::new(token_table, cursor)?;
        expect_kind(token_table, cursor, &TokenKind::RightParen)?;
        *cursor += 1;
        Ok(Box::new(select_stmt))
    }
}
