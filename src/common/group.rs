use minivec::MiniVec;

use crate::{
    ParserError,
    common::{expr::Expr, utils::expect_kind},
    keyword::Keyword,
    token::{TokenKind, TokenTable},
};

#[derive(Debug, PartialEq)]
pub struct Group<'a> {
    pub columns: MiniVec<Expr<'a>>,
}

impl<'a> Group<'a> {
    pub(crate) fn build(
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
    ) -> Result<Self, ParserError> {
        expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Group))?;
        *cursor += 1;
        expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::By))?;
        *cursor += 1;

        let mut columns = MiniVec::new();
        loop {
            let is_clause_kw = matches!(
                token_table.get_kind(*cursor),
                Some(TokenKind::Keyword(
                    Keyword::Where
                        | Keyword::Group
                        | Keyword::Having
                        | Keyword::Order
                        | Keyword::Limit
                        | Keyword::From
                )) | Some(TokenKind::RightParen | TokenKind::Delimiter)
                | None
            );
            if is_clause_kw {
                break;
            }
            match token_table.get_kind(*cursor) {
                Some(TokenKind::Comma) => {
                    *cursor += 1;
                }
                Some(_) => {
                    let expr = Expr::build(token_table, cursor)?;
                    columns.push(expr);
                }
                _ => {
                    break;
                }
            }
        }

        Ok(Self { columns })
    }
}
