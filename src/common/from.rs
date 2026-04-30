use crate::{
    ParserError,
    ast::select::SubSelectStatement,
    common::{alias::Alias, expr::Expr, utils::expect_kind},
    keyword::Keyword,
    token::{TokenKind, TokenTable},
};

#[derive(Debug, PartialEq)]
pub enum Table {
    Name(Alias<Expr>),
    SubQuery(Alias<SubSelectStatement>),
}

impl Table {
    pub(crate) fn class_name_with_single(
        token_table: &TokenTable,
        cursor: &mut usize,
    ) -> Result<Self, ParserError> {
        let expr = Expr::class_field(token_table, cursor)?;
        Ok(Self::Name(Alias {
            name: None,
            value: expr,
        }))
    }

    pub(crate) fn build(token_table: &TokenTable, cursor: &mut usize) -> Result<Self, ParserError> {
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
pub enum From {
    Table(Table),
    CrossJoin {
        left: Table,
        right: Table,
    },
    LeftJoin {
        left: Table,
        right: Table,
        condition: Expr,
    },
    RightJoin {
        left: Table,
        right: Table,
        condition: Expr,
    },
    InnerJoin {
        left: Table,
        right: Table,
        condition: Expr,
    },
    FullJoin {
        left: Table,
        right: Table,
        condition: Expr,
    },
}

impl From {
    pub(crate) fn class_table(
        token_table: &TokenTable,
        cursor: &mut usize,
    ) -> Result<Self, ParserError> {
        let table = Table::build(token_table, cursor)?;
        Ok(From::Table(table))
    }

    pub(crate) fn parse(token_table: &TokenTable, cursor: &mut usize) -> Result<Self, ParserError> {
        let left = Table::build(token_table, cursor)?;
        Self::parse_joins(token_table, cursor, From::Table(left))
    }

    fn parse_joins(
        token_table: &TokenTable,
        cursor: &mut usize,
        mut current: From,
    ) -> Result<Self, ParserError> {
        // dbg!(token_table.get_kind(*cursor));
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

    fn extract_table(from: From) -> Table {
        match from {
            From::Table(t) => t,
            From::CrossJoin { left, .. } => left,
            From::InnerJoin { left, .. } => left,
            From::LeftJoin { left, .. } => left,
            From::RightJoin { left, .. } => left,
            From::FullJoin { left, .. } => left,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::expr::{BinaryOp, BinaryOperator, Field};

    fn tokenize(tokens: Vec<(TokenKind, usize, usize)>) -> TokenTable {
        let mut table = TokenTable::new();
        for (kind, start, end) in tokens {
            table.push(kind, start, end);
        }
        table
    }

    #[test]
    fn test_simple_table() {
        let tokens = tokenize(vec![
            (TokenKind::Identifier, 0, 3), // "users"
        ]);
        let mut cursor = 0;
        let result = From::parse(&tokens, &mut cursor).unwrap();
        assert_eq!(cursor, 1);
        assert_eq!(
            result,
            From::Table(Table::Name(Alias {
                name: None,
                value: Expr::Field(Field {
                    prefix: None,
                    value: 0,
                }),
            }))
        );
    }

    #[test]
    fn test_table_with_alias() {
        let tokens = tokenize(vec![
            (TokenKind::Identifier, 0, 3), // "users"
            (TokenKind::Identifier, 5, 5), // "u"
        ]);
        let mut cursor = 0;
        let result = From::parse(&tokens, &mut cursor).unwrap();
        assert_eq!(cursor, 2);
        assert_eq!(
            result,
            From::Table(Table::Name(Alias {
                name: Some(1),
                value: Expr::Field(Field {
                    prefix: None,
                    value: 0,
                }),
            }))
        );
    }

    #[test]
    fn test_inner_join() {
        let tokens = tokenize(vec![
            (TokenKind::Identifier, 0, 3),             // "users"     - idx 0
            (TokenKind::Keyword(Keyword::Join), 5, 8), // "JOIN"      - idx 1
            (TokenKind::Identifier, 10, 14),           // "orders"    - idx 2
            (TokenKind::Keyword(Keyword::On), 16, 17), // "ON"        - idx 3
            (TokenKind::Identifier, 19, 22),           // "user_id"   - idx 4
            (TokenKind::Equal, 23, 23),                // "="         - idx 5
            (TokenKind::Identifier, 25, 30),           // "user_id"   - idx 6
        ]);
        let mut cursor = 0;
        let result = From::parse(&tokens, &mut cursor).unwrap();

        let expected_condition = Expr::BinaryOp(Box::new(BinaryOp {
            op: BinaryOperator::Equal,
            left: Expr::Field(Field {
                prefix: None,
                value: 4,
            }),
            right: Expr::Field(Field {
                prefix: None,
                value: 6,
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
                        value: 0
                    })
                })
            );
            assert_eq!(
                right,
                Table::Name(Alias {
                    name: None,
                    value: Expr::Field(Field {
                        prefix: None,
                        value: 2
                    })
                })
            );
            assert_eq!(condition, expected_condition);
        }
    }

    #[test]
    fn test_left_join() {
        let tokens = tokenize(vec![
            (TokenKind::Identifier, 0, 3),               // "users"
            (TokenKind::Keyword(Keyword::Left), 5, 8),   // "LEFT"
            (TokenKind::Keyword(Keyword::Join), 10, 13), // "JOIN"
            (TokenKind::Identifier, 15, 20),             // "orders"
            (TokenKind::Keyword(Keyword::On), 22, 23),   // "ON"
            (TokenKind::Identifier, 25, 31),             // "user_id"
            (TokenKind::Equal, 32, 32),                  // "="
            (TokenKind::Identifier, 34, 39),             // "user_id"
        ]);
        let mut cursor = 0;
        let result = From::parse(&tokens, &mut cursor).unwrap();
        assert!(matches!(result, From::LeftJoin { .. }));
    }

    #[test]
    fn test_right_join() {
        let tokens = tokenize(vec![
            (TokenKind::Identifier, 0, 3),               // "users"
            (TokenKind::Keyword(Keyword::Right), 5, 9),  // "RIGHT"
            (TokenKind::Keyword(Keyword::Join), 11, 14), // "JOIN"
            (TokenKind::Identifier, 16, 21),             // "orders"
            (TokenKind::Keyword(Keyword::On), 23, 24),   // "ON"
            (TokenKind::Identifier, 26, 32),             // "user_id"
            (TokenKind::Equal, 33, 33),                  // "="
            (TokenKind::Identifier, 35, 40),             // "user_id"
        ]);
        let mut cursor = 0;
        let result = From::parse(&tokens, &mut cursor).unwrap();
        assert!(matches!(result, From::RightJoin { .. }));
    }

    #[test]
    fn test_full_join() {
        let tokens = tokenize(vec![
            (TokenKind::Identifier, 0, 3),               // "users"
            (TokenKind::Keyword(Keyword::Full), 5, 8),   // "FULL"
            (TokenKind::Keyword(Keyword::Join), 10, 13), // "JOIN"
            (TokenKind::Identifier, 15, 20),             // "orders"
            (TokenKind::Keyword(Keyword::On), 22, 23),   // "ON"
            (TokenKind::Identifier, 25, 31),             // "user_id"
            (TokenKind::Equal, 32, 32),                  // "="
            (TokenKind::Identifier, 34, 39),             // "user_id"
        ]);
        let mut cursor = 0;
        let result = From::parse(&tokens, &mut cursor).unwrap();
        assert!(matches!(result, From::FullJoin { .. }));
    }

    #[test]
    fn test_inner_join_with_inner_keyword() {
        let tokens = tokenize(vec![
            (TokenKind::Identifier, 0, 3),               // "users"
            (TokenKind::Keyword(Keyword::Inner), 5, 9),  // "INNER"
            (TokenKind::Keyword(Keyword::Join), 11, 14), // "JOIN"
            (TokenKind::Identifier, 16, 21),             // "orders"
            (TokenKind::Keyword(Keyword::On), 23, 24),   // "ON"
            (TokenKind::Identifier, 26, 32),             // "user_id"
            (TokenKind::Equal, 33, 33),                  // "="
            (TokenKind::Identifier, 35, 40),             // "user_id"
        ]);
        let mut cursor = 0;
        let result = From::parse(&tokens, &mut cursor).unwrap();
        assert!(matches!(result, From::InnerJoin { .. }));
    }

    #[test]
    fn test_cross_join() {
        let tokens = tokenize(vec![
            (TokenKind::Identifier, 0, 3),
            (TokenKind::Keyword(Keyword::Cross), 4, 8),
            (TokenKind::Keyword(Keyword::Join), 10, 13),
            (TokenKind::Identifier, 15, 20), // "users"
        ]);
        let mut cursor = 0;
        let result = From::parse(&tokens, &mut cursor).unwrap();
        assert_eq!(
            result,
            From::CrossJoin {
                left: Table::Name(Alias {
                    name: None,
                    value: Expr::Field(Field {
                        prefix: None,
                        value: 0,
                    }),
                }),
                right: Table::Name(Alias {
                    name: None,
                    value: Expr::Field(Field {
                        prefix: None,
                        value: 3,
                    }),
                }),
            }
        );
    }

    #[test]
    fn test_multiple_joins() {
        let tokens = tokenize(vec![
            (TokenKind::Identifier, 0, 3),               // "users"
            (TokenKind::Keyword(Keyword::Join), 5, 8),   // "JOIN"
            (TokenKind::Identifier, 10, 14),             // "orders"
            (TokenKind::Keyword(Keyword::On), 16, 17),   // "ON"
            (TokenKind::Identifier, 19, 25),             // "user_id"
            (TokenKind::Equal, 26, 26),                  // "="
            (TokenKind::Identifier, 28, 33),             // "user_id"
            (TokenKind::Keyword(Keyword::Join), 35, 38), // "JOIN"
            (TokenKind::Identifier, 40, 49),             // "order_items"
            (TokenKind::Keyword(Keyword::On), 51, 52),   // "ON"
            (TokenKind::Identifier, 54, 60),             // "order_id"
            (TokenKind::Equal, 61, 61),                  // "="
            (TokenKind::Identifier, 63, 70),             // "order_id"
        ]);
        let mut cursor = 0;
        let result = From::parse(&tokens, &mut cursor).unwrap();
        // 嵌套的 JOIN
        assert!(matches!(result, From::InnerJoin { .. }));
    }

    #[test]
    fn test_left_join_outer_keyword() {
        let tokens = tokenize(vec![
            (TokenKind::Identifier, 0, 3),                // "users"
            (TokenKind::Keyword(Keyword::Left), 5, 8),    // "LEFT"
            (TokenKind::Keyword(Keyword::Outer), 10, 14), // "OUTER"
            (TokenKind::Keyword(Keyword::Join), 16, 19),  // "JOIN"
            (TokenKind::Identifier, 21, 26),              // "orders"
            (TokenKind::Keyword(Keyword::On), 28, 29),    // "ON"
            (TokenKind::Identifier, 31, 37),              // "user_id"
            (TokenKind::Equal, 38, 38),                   // "="
            (TokenKind::Identifier, 40, 45),              // "user_id"
        ]);
        let mut cursor = 0;
        let result = From::parse(&tokens, &mut cursor).unwrap();
        assert!(matches!(result, From::LeftJoin { .. }));
    }

    #[test]
    fn test_table_with_prefix() {
        let tokens = tokenize(vec![
            (TokenKind::Identifier, 0, 3), // "users"
            (TokenKind::Dot, 4, 4),        // "."
            (TokenKind::Identifier, 6, 8), // "id"
        ]);
        let mut cursor = 0;
        let result = From::parse(&tokens, &mut cursor).unwrap();

        assert_eq!(
            result,
            From::Table(Table::Name(Alias {
                name: None,
                value: Expr::Field(Field {
                    prefix: Some(0),
                    value: 2
                })
            }))
        );
    }
}
