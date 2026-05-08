use minivec::MiniVec;

use crate::{
    ParserError, SelectStatement, common::{
        expr::Expr,
        from::Table,
        utils::{expect_kind, maybe_kind},
    }, keyword::Keyword, token::{TokenKind, TokenTable}
};

#[derive(Debug, PartialEq)]
pub enum InsertValue<'a> {
    AllSelect {
        select: SelectStatement<'a>,
    },
    PartOfSelect {
        select: SelectStatement<'a>,
        columns: MiniVec<Expr<'a>>,
    },
    Values {
        columns: MiniVec<Expr<'a>>,
        values: MiniVec<MiniVec<Expr<'a>>>,
    }
}

impl<'a> InsertValue<'a> {
    pub(crate) fn build(token_table: &TokenTable<'a>, cursor: &mut usize) -> Result<Self, ParserError> {
        match token_table.get_kind(*cursor) {
            Some(TokenKind::Keyword(Keyword::Select)) => {
                let select = SelectStatement::new(token_table, cursor)?;
                Ok(Self::AllSelect { select })
            },
            Some(TokenKind::LeftParen) => {
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

                match token_table.get_kind(*cursor) {
                    Some(TokenKind::Keyword(Keyword::Select)) => {
                        let select = SelectStatement::new(token_table, cursor)?;
                        Ok(Self::PartOfSelect { select, columns })
                    },
                    Some(TokenKind::Keyword(Keyword::Values)) => {
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

                        Ok(Self::Values { columns, values })
                    }
                    _ => Err(ParserError::SyntaxError(*cursor, *cursor))
                }
            },
            _ => Err(ParserError::SyntaxError(*cursor, *cursor))
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct InsertStatement<'a> {
    pub table: Table<'a>,
    pub insert_value: InsertValue<'a>,
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

        let insert_value = InsertValue::build(token_table, cursor)?;

        Ok(InsertStatement {
            table,
            insert_value,
        })
    }
}
