use minivec::MiniVec;

use crate::{
    ParserError,
    common::{
        expr::Expr,
        from::Table,
        utils::{expect_kind, maybe_kind},
    },
    keyword::Keyword,
    token::{TokenKind, TokenTable},
};

#[derive(Debug, PartialEq)]
pub struct InsertStatement<'a> {
    pub table: Table<'a>,
    pub columns: MiniVec<Expr<'a>>,
    pub values: MiniVec<MiniVec<Expr<'a>>>,
}

impl<'a> InsertStatement<'a> {
    pub(crate) fn new(
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
    ) -> Result<Self, ParserError> {
        Self::build_ast(token_table, cursor)
    }

    fn build_ast(token_table: &TokenTable<'a>, cursor: &mut usize) -> Result<Self, ParserError> {
        expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Insert))?;
        *cursor += 1;

        expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Into))?;
        *cursor += 1;

        let table = Table::class_name_with_single(token_table, cursor)?;

        expect_kind(token_table, cursor, &TokenKind::LeftParen)?;
        *cursor += 1;

        let mut columns = MiniVec::new();

        loop {
            match token_table.get_kind(*cursor) {
                Some(TokenKind::Comma) => {
                    *cursor += 1;
                }
                Some(TokenKind::RightParen) => {
                    break;
                }
                Some(_) => {
                    let column = Expr::build(token_table, cursor)?;
                    columns.push(column);
                }
                None => break,
            }
        }
        expect_kind(token_table, cursor, &TokenKind::RightParen)?;
        *cursor += 1;

        expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Values))?;
        *cursor += 1;

        let mut values = MiniVec::new();
        loop {
            match token_table.get_kind(*cursor) {
                Some(TokenKind::LeftParen) => {
                    *cursor += 1;
                    let mut value_row = MiniVec::new();
                    loop {
                        match token_table.get_kind(*cursor) {
                            Some(TokenKind::Comma) => {
                                *cursor += 1;
                            }
                            Some(TokenKind::RightParen) => {
                                break;
                            }
                            Some(_) => {
                                let value = Expr::build(token_table, cursor)?;
                                value_row.push(value);
                            }
                            None => break,
                        }
                    }
                    expect_kind(token_table, cursor, &TokenKind::RightParen)?;
                    *cursor += 1;
                    values.push(value_row);
                }
                Some(TokenKind::Comma) => {
                    *cursor += 1;
                }
                _ => break,
            }
        }

        if maybe_kind(token_table, cursor, &TokenKind::Eof) {
            *cursor += 1;
        }

        Ok(InsertStatement {
            table,
            columns,
            values,
        })
    }
}
