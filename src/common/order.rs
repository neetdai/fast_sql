use minivec::MiniVec;

use crate::{
    ParserError,
    common::{
        expr::Expr,
        utils::{expect_kind, maybe_kind},
    },
    keyword::Keyword,
    token::{TokenKind, TokenTable},
};

#[derive(Debug, PartialEq)]
pub enum OrderDirection {
    ASC,
    DESC,
}

#[derive(Debug, PartialEq)]
pub enum NullsOrder {
    First,
    Last,
}

#[derive(Debug, PartialEq)]
pub struct OrderItem {
    pub expr: Expr,
    pub direction: OrderDirection,
    pub nulls_order: Option<NullsOrder>,
}

#[derive(Debug, PartialEq)]
pub struct Order {
    pub columns: MiniVec<OrderItem>,
}

impl Order {
    pub(crate) fn build(token_table: &TokenTable, cursor: &mut usize) -> Result<Self, ParserError> {
        expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Order))?;
        *cursor += 1;
        expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::By))?;
        *cursor += 1;

        let mut columns = MiniVec::new();
        loop {
            match token_table.get_kind(*cursor) {
                Some(TokenKind::Identifier) => {
                    let expr = Expr::build(token_table, cursor)?;
                    let direction =
                        if maybe_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Asc)) {
                            *cursor += 1;
                            OrderDirection::ASC
                        } else if maybe_kind(
                            token_table,
                            cursor,
                            &TokenKind::Keyword(Keyword::Desc),
                        ) {
                            *cursor += 1;
                            OrderDirection::DESC
                        } else {
                            OrderDirection::ASC
                        };
                    let nulls_order =
                        if maybe_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Nulls)) {
                            *cursor += 1;
                            if maybe_kind(token_table, cursor, &TokenKind::Keyword(Keyword::First))
                            {
                                *cursor += 1;
                                Some(NullsOrder::First)
                            } else if maybe_kind(
                                token_table,
                                cursor,
                                &TokenKind::Keyword(Keyword::Last),
                            ) {
                                *cursor += 1;
                                Some(NullsOrder::Last)
                            } else {
                                return Err(ParserError::SyntaxError(*cursor, *cursor));
                            }
                        } else {
                            None
                        };
                    columns.push(OrderItem {
                        expr,
                        direction,
                        nulls_order,
                    });
                }
                Some(TokenKind::Comma) => {
                    *cursor += 1;
                }
                _ => {
                    break;
                }
            }
        }

        Ok(Order { columns })
    }
}
