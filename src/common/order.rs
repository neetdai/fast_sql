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

#[cfg(feature = "serde")]
use serde::{
    ser::{Serialize, Serializer, SerializeStruct},
};

#[derive(Debug, PartialEq)]
pub enum OrderDirection {
    ASC,
    DESC,
}

#[cfg(feature = "serde")]
impl Serialize for OrderDirection {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            OrderDirection::ASC => serializer.serialize_unit_variant("OrderDirection", 0, "ASC"),
            OrderDirection::DESC => serializer.serialize_unit_variant("OrderDirection", 1, "DESC"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum NullsOrder {
    First,
    Last,
}

#[cfg(feature = "serde")]
impl Serialize for NullsOrder {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            NullsOrder::First => serializer.serialize_unit_variant("NullsOrder", 0, "First"),
            NullsOrder::Last => serializer.serialize_unit_variant("NullsOrder", 1, "Last"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct OrderItem<'a> {
    pub expr: Expr<'a>,
    pub direction: OrderDirection,
    pub nulls_order: Option<NullsOrder>,
}

#[cfg(feature = "serde")]
impl<'a> Serialize for OrderItem<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("OrderItem", 3)?;
        s.serialize_field("expr", &self.expr)?;
        s.serialize_field("direction", &self.direction)?;
        s.serialize_field("nulls_order", &self.nulls_order)?;
        s.end()
    }
}

#[derive(Debug, PartialEq)]
pub struct Order<'a> {
    pub columns: MiniVec<OrderItem<'a>>,
}

impl<'a> Order<'a> {
    pub(crate) fn build(
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
    ) -> Result<Self, ParserError> {
        expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Order))?;
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
                        | Keyword::Grouping
                        | Keyword::Having
                        | Keyword::Order
                        | Keyword::Limit
                        | Keyword::From
                        | Keyword::Cube
                        | Keyword::Rollup
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
                _ => {
                    break;
                }
            }
        }

        if columns.is_empty() {
            return Err(ParserError::SyntaxError(*cursor, *cursor));
        }
        Ok(Order { columns })
    }
}

#[cfg(feature = "serde")]
impl<'a> Serialize for Order<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("Order", 1)?;
        s.serialize_field("columns", &self.columns)?;
        s.end()
    }
}