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
pub struct DeleteStatement<'a> {
    pub table: Table<'a>,
    pub conditions: Option<Expr<'a>>,
}

impl<'a> DeleteStatement<'a> {
    pub(crate) fn new(
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
    ) -> Result<Self, ParserError> {
        Self::build_ast(token_table, cursor)
    }

    fn build_ast(token_table: &TokenTable<'a>, cursor: &mut usize) -> Result<Self, ParserError> {
        expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Delete))?;
        *cursor += 1;

        expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::From))?;
        *cursor += 1;

        let table = Table::class_name_with_single(token_table, cursor)?;

        let conditions = if maybe_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Where)) {
            *cursor += 1;
            Some(Expr::build(token_table, cursor)?)
        } else {
            None
        };

        Ok(Self { table, conditions })
    }
}
