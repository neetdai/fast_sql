use crate::{
    ParserError,
    ast::select::SubSelectStatement,
    common::{alias::Alias, expr::Expr, utils::expect_kind},
    keyword::Keyword,
    token::{TokenKind, TokenTable},
};

#[derive(Debug, PartialEq)]
pub enum Table<'a> {
    Name(Alias<'a, Expr<'a>>),
    SubQuery(Alias<'a, SubSelectStatement<'a>>),
}

impl<'a> Table<'a> {
    pub(crate) fn class_name_with_single(
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
    ) -> Result<Self, ParserError> {
        let expr = Expr::class_field(token_table, cursor)?;
        Ok(Self::Name(Alias {
            name: None,
            value: expr,
        }))
    }

    pub(crate) fn build(
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
    ) -> Result<Self, ParserError> {
        if let Some(TokenKind::LeftParen) = token_table.get_kind(*cursor) {
            let alias = Alias::new(token_table, cursor)?;
            Ok(Table::SubQuery(alias))
        } else {
            let alias = Alias::new(token_table, cursor)?;
            Ok(Table::Name(alias))
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum From<'a> {
    Table(Table<'a>),
    CrossJoin {
        left: Table<'a>,
        right: Table<'a>,
    },
    LeftJoin {
        left: Table<'a>,
        right: Table<'a>,
        condition: Expr<'a>,
    },
    RightJoin {
        left: Table<'a>,
        right: Table<'a>,
        condition: Expr<'a>,
    },
    InnerJoin {
        left: Table<'a>,
        right: Table<'a>,
        condition: Expr<'a>,
    },
    FullJoin {
        left: Table<'a>,
        right: Table<'a>,
        condition: Expr<'a>,
    },
}

impl<'a> From<'a> {
    pub(crate) fn parse(
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
    ) -> Result<Self, ParserError> {
        let left = Table::build(token_table, cursor)?;
        Self::parse_joins(token_table, cursor, From::Table(left))
    }

    fn parse_joins(
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
        mut current: From<'a>,
    ) -> Result<Self, ParserError> {
        loop {
            match token_table.get_kind(*cursor) {
                Some(TokenKind::Keyword(Keyword::Join)) => {
                    *cursor += 1;
                    let right = Table::build(token_table, cursor)?;
                    expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::On))?;
                    *cursor += 1;
                    let condition = Expr::build(token_table, cursor)?;
                    current = From::InnerJoin {
                        left: Self::extract_table(current),
                        right,
                        condition,
                    };
                }
                Some(TokenKind::Keyword(Keyword::Inner)) => {
                    *cursor += 1;
                    expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Join))?;
                    *cursor += 1;
                    let right = Table::build(token_table, cursor)?;
                    expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::On))?;
                    *cursor += 1;
                    let condition = Expr::build(token_table, cursor)?;
                    current = From::InnerJoin {
                        left: Self::extract_table(current),
                        right,
                        condition,
                    };
                }
                Some(TokenKind::Keyword(Keyword::Left)) => {
                    *cursor += 1;
                    if token_table.get_kind(*cursor) == Some(&TokenKind::Keyword(Keyword::Outer)) {
                        *cursor += 1;
                    }
                    expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Join))?;
                    *cursor += 1;
                    let right = Table::build(token_table, cursor)?;
                    expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::On))?;
                    *cursor += 1;
                    let condition = Expr::build(token_table, cursor)?;
                    current = From::LeftJoin {
                        left: Self::extract_table(current),
                        right,
                        condition,
                    };
                }
                Some(TokenKind::Keyword(Keyword::Right)) => {
                    *cursor += 1;
                    if token_table.get_kind(*cursor) == Some(&TokenKind::Keyword(Keyword::Outer)) {
                        *cursor += 1;
                    }
                    expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Join))?;
                    *cursor += 1;
                    let right = Table::build(token_table, cursor)?;
                    expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::On))?;
                    *cursor += 1;
                    let condition = Expr::build(token_table, cursor)?;
                    current = From::RightJoin {
                        left: Self::extract_table(current),
                        right,
                        condition,
                    };
                }
                Some(TokenKind::Keyword(Keyword::Full)) => {
                    *cursor += 1;
                    if token_table.get_kind(*cursor) == Some(&TokenKind::Keyword(Keyword::Outer)) {
                        *cursor += 1;
                    }
                    expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Join))?;
                    *cursor += 1;
                    let right = Table::build(token_table, cursor)?;
                    expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::On))?;
                    *cursor += 1;
                    let condition = Expr::build(token_table, cursor)?;
                    current = From::FullJoin {
                        left: Self::extract_table(current),
                        right,
                        condition,
                    };
                }
                Some(TokenKind::Keyword(Keyword::Cross)) => {
                    *cursor += 1;
                    expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Join))?;
                    *cursor += 1;
                    let right = Table::build(token_table, cursor)?;
                    current = From::CrossJoin {
                        left: Self::extract_table(current),
                        right,
                    };
                }
                _ => break,
            }
        }
        Ok(current)
    }

    fn extract_table(from: From<'a>) -> Table<'a> {
        match from {
            From::Table(t) => t,
            From::CrossJoin { left, .. }
            | From::InnerJoin { left, .. }
            | From::LeftJoin { left, .. }
            | From::RightJoin { left, .. }
            | From::FullJoin { left, .. } => left,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::*;
    use crate::common::expr::{BinaryOp, BinaryOperator, Field};
    use crate::token::TokenKind;

    fn make_table<'a>(source: &'a str, tokens: Vec<(TokenKind, usize, usize)>) -> TokenTable<'a> {
        let mut table = TokenTable::with_source(source);
        for (kind, start, end) in tokens {
            table.push(
                kind,
                String::from_utf8_lossy(&source.as_bytes()[start..=end]),
            );
        }
        table
    }

    #[test]
    fn test_simple_table() {
        let tokens = make_table("users", vec![(TokenKind::Identifier, 0, 4)]);
        let mut cursor = 0;
        let result = From::parse(&tokens, &mut cursor).unwrap();
        assert_eq!(cursor, 1);
        assert_eq!(
            result,
            From::Table(Table::Name(Alias {
                name: None,
                value: Expr::Field(Field {
                    prefix: None,
                    name: Cow::Borrowed("users"),
                }),
            }))
        );
    }

    #[test]
    fn test_table_with_alias() {
        let tokens = make_table(
            "users u",
            vec![(TokenKind::Identifier, 0, 4), (TokenKind::Identifier, 6, 6)],
        );
        let mut cursor = 0;
        let result = From::parse(&tokens, &mut cursor).unwrap();
        assert_eq!(cursor, 2);
        assert_eq!(
            result,
            From::Table(Table::Name(Alias {
                name: Some(Cow::Borrowed("u")),
                value: Expr::Field(Field {
                    prefix: None,
                    name: Cow::Borrowed("users"),
                }),
            }))
        );
    }

    #[test]
    fn test_inner_join() {
        let tokens = make_table(
            "users JOIN orders ON user_id = user_id",
            vec![
                (TokenKind::Identifier, 0, 4),
                (TokenKind::Keyword(Keyword::Join), 6, 9),
                (TokenKind::Identifier, 11, 16),
                (TokenKind::Keyword(Keyword::On), 18, 19),
                (TokenKind::Identifier, 21, 27),
                (TokenKind::Equal, 29, 29),
                (TokenKind::Identifier, 31, 37),
            ],
        );
        let mut cursor = 0;
        let result = From::parse(&tokens, &mut cursor).unwrap();

        let expected_condition = Expr::BinaryOp(Box::new(BinaryOp {
            op: BinaryOperator::Equal,
            left: Expr::Field(Field {
                prefix: None,
                name: Cow::Borrowed("user_id"),
            }),
            right: Expr::Field(Field {
                prefix: None,
                name: Cow::Borrowed("user_id"),
            }),
        }));

        assert!(matches!(result, From::InnerJoin { .. }));
        if let From::InnerJoin {
            left,
            right,
            condition,
        } = result
        {
            assert_eq!(
                left,
                Table::Name(Alias {
                    name: None,
                    value: Expr::Field(Field {
                        prefix: None,
                        name: Cow::Borrowed("users")
                    })
                })
            );
            assert_eq!(
                right,
                Table::Name(Alias {
                    name: None,
                    value: Expr::Field(Field {
                        prefix: None,
                        name: Cow::Borrowed("orders")
                    })
                })
            );
            assert_eq!(condition, expected_condition);
        }
    }

    #[test]
    fn test_multiple_joins() {
        let tokens = make_table(
            "users JOIN orders ON user_id = user_id JOIN order_items ON order_id = order_id",
            vec![
                (TokenKind::Identifier, 0, 4),
                (TokenKind::Keyword(Keyword::Join), 6, 9),
                (TokenKind::Identifier, 11, 16),
                (TokenKind::Keyword(Keyword::On), 18, 19),
                (TokenKind::Identifier, 21, 27),
                (TokenKind::Equal, 29, 29),
                (TokenKind::Identifier, 31, 37),
                (TokenKind::Keyword(Keyword::Join), 39, 42),
                (TokenKind::Identifier, 44, 54),
                (TokenKind::Keyword(Keyword::On), 56, 57),
                (TokenKind::Identifier, 59, 66),
                (TokenKind::Equal, 68, 68),
                (TokenKind::Identifier, 70, 77),
            ],
        );
        let mut cursor = 0;
        let result = From::parse(&tokens, &mut cursor).unwrap();
        assert!(matches!(result, From::InnerJoin { .. }));
    }

    #[test]
    fn test_cross_join() {
        let tokens = make_table(
            "u CROSS JOIN o",
            vec![
                (TokenKind::Identifier, 0, 0),
                (TokenKind::Keyword(Keyword::Cross), 2, 6),
                (TokenKind::Keyword(Keyword::Join), 8, 11),
                (TokenKind::Identifier, 13, 13),
            ],
        );
        let mut cursor = 0;
        let result = From::parse(&tokens, &mut cursor).unwrap();
        assert_eq!(
            result,
            From::CrossJoin {
                left: Table::Name(Alias {
                    name: None,
                    value: Expr::Field(Field {
                        prefix: None,
                        name: Cow::Borrowed("u"),
                    }),
                }),
                right: Table::Name(Alias {
                    name: None,
                    value: Expr::Field(Field {
                        prefix: None,
                        name: Cow::Borrowed("o"),
                    }),
                }),
            }
        );
    }

    #[test]
    fn test_table_with_prefix() {
        let tokens = make_table(
            "users.id",
            vec![
                (TokenKind::Identifier, 0, 4),
                (TokenKind::Dot, 5, 5),
                (TokenKind::Identifier, 6, 7),
            ],
        );
        let mut cursor = 0;
        let result = From::parse(&tokens, &mut cursor).unwrap();

        assert_eq!(
            result,
            From::Table(Table::Name(Alias {
                name: None,
                value: Expr::Field(Field {
                    prefix: Some(Cow::Borrowed("users")),
                    name: Cow::Borrowed("id")
                })
            }))
        );
    }
}
