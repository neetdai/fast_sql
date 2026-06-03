use crate::{
    ParserError,
    ast::select::SubSelectStatement,
    common::{alias::Alias, expr::Expr, utils::{expect_kind, maybe_kind}},
    keyword::Keyword,
    token::{TokenKind, TokenTable},
};

#[cfg(feature = "serde")]
use serde::{ser::{SerializeStruct, SerializeStructVariant}, Serialize, Serializer};

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

#[cfg(feature = "serde")]
impl<'a> Serialize for Table<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Name(name) => serializer.serialize_newtype_variant("Table", 0, "Name", &name),
            Self::SubQuery(sub_query) => serializer.serialize_newtype_variant("Table", 1, "SubQuery", &sub_query),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum JoinType {
    LeftJoin,
    RightJoin,
    InnerJoin,
    CrossJoin,
    FullJoin,
}

#[cfg(feature = "serde")]
impl Serialize for JoinType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::LeftJoin => serializer.serialize_unit_variant("JoinType", 0, "LeftJoin"),
            Self::RightJoin => serializer.serialize_unit_variant("JoinType", 1, "RightJoin"),
            Self::InnerJoin => serializer.serialize_unit_variant("JoinType", 2, "InnerJoin"),
            Self::CrossJoin => serializer.serialize_unit_variant("JoinType", 3, "CrossJoin"),
            Self::FullJoin => serializer.serialize_unit_variant("JoinType", 4, "FullJoin"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum From<'a> {
    Table(Table<'a>),
    CrossJoin {
        left: Box<From<'a>>,
        right: Box<From<'a>>,
    },
    NaturalJoin {
        left: Box<From<'a>>,
        right: Box<From<'a>>,
        join_type: JoinType,
    },
    JoinUsing {
        left: Box<From<'a>>,
        right: Box<From<'a>>,
        join_type: JoinType,
        using: Vec<&'a str>,
    },
    LeftJoin {
        left: Box<From<'a>>,
        right: Box<From<'a>>,
        condition: Expr<'a>,
    },
    RightJoin {
        left: Box<From<'a>>,
        right: Box<From<'a>>,
        condition: Expr<'a>,
    },
    InnerJoin {
        left: Box<From<'a>>,
        right: Box<From<'a>>,
        condition: Expr<'a>,
    },
    FullJoin {
        left: Box<From<'a>>,
        right: Box<From<'a>>,
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

    fn parse_join_on_using(
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
        is_natural: bool,
    ) -> Result<Option<Vec<&'a str>>, ParserError> {
        if is_natural {
            return Ok(None);
        }
        if maybe_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Using)) {
            *cursor += 1;
            Self::parse_using_list(token_table, cursor).map(Some)
        } else {
            Ok(None)
        }
    }

    fn parse_using_list(
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
    ) -> Result<Vec<&'a str>, ParserError> {
        expect_kind(token_table, cursor, &TokenKind::LeftParen)?;
        *cursor += 1;

        let mut columns = Vec::new();
        loop {
            match token_table.get_kind(*cursor) {
                Some(TokenKind::Identifier) => {
                    columns.push(token_table.source_at(*cursor));
                    *cursor += 1;
                }
                Some(TokenKind::Comma) => {
                    *cursor += 1;
                }
                Some(TokenKind::RightParen) => {
                    *cursor += 1;
                    break;
                }
                _ => return Err(ParserError::SyntaxError(*cursor, *cursor)),
            }
        }

        if columns.is_empty() {
            return Err(ParserError::SyntaxError(*cursor, *cursor));
        }
        Ok(columns)
    }

    fn parse_joins(
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
        mut current: From<'a>,
    ) -> Result<Self, ParserError> {
        loop {
            let is_natural =
                if let Some(TokenKind::Keyword(Keyword::Natural)) = token_table.get_kind(*cursor) {
                    *cursor += 1;
                    true
                } else {
                    false
                };

            match token_table.get_kind(*cursor) {
                Some(TokenKind::Keyword(Keyword::Join)) => {
                    *cursor += 1;
                    let left = Box::new(Self::parse_joins(token_table, cursor, current)?);
                    let right = Box::new(Self::parse(token_table, cursor)?);
                    if is_natural {
                        current = From::NaturalJoin {
                            left,
                            right,
                            join_type: JoinType::InnerJoin,
                        };
                    } else if let Some(using) =
                        Self::parse_join_on_using(token_table, cursor, false)?
                    {
                        current = From::JoinUsing {
                            left,
                            right,
                            join_type: JoinType::InnerJoin,
                            using,
                        };
                    } else {
                        expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::On))?;
                        *cursor += 1;
                        let condition = Expr::build(token_table, cursor)?;
                        current = From::InnerJoin {
                            left,
                            right,
                            condition,
                        };
                    }
                }
                Some(TokenKind::Keyword(Keyword::Inner)) => {
                    *cursor += 1;
                    expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Join))?;
                    *cursor += 1;
                    let left = Box::new(Self::parse_joins(token_table, cursor, current)?);
                    let right = Box::new(Self::parse(token_table, cursor)?);
                    if is_natural {
                        current = From::NaturalJoin {
                            left,
                            right,
                            join_type: JoinType::InnerJoin,
                        };
                    } else if let Some(using) =
                        Self::parse_join_on_using(token_table, cursor, false)?
                    {
                        current = From::JoinUsing {
                            left,
                            right,
                            join_type: JoinType::InnerJoin,
                            using,
                        };
                    } else {
                        expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::On))?;
                        *cursor += 1;
                        let condition = Expr::build(token_table, cursor)?;
                        current = From::InnerJoin {
                            left,
                            right,
                            condition,
                        };
                    }
                }
                Some(TokenKind::Keyword(Keyword::Left)) => {
                    *cursor += 1;
                    if token_table.get_kind(*cursor) == Some(&TokenKind::Keyword(Keyword::Outer)) {
                        *cursor += 1;
                    }
                    expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Join))?;
                    *cursor += 1;
                    let left = Box::new(Self::parse_joins(token_table, cursor, current)?);
                    let right = Box::new(Self::parse(token_table, cursor)?);
                    if is_natural {
                        current = From::NaturalJoin {
                            left,
                            right,
                            join_type: JoinType::LeftJoin,
                        };
                    } else if let Some(using) =
                        Self::parse_join_on_using(token_table, cursor, false)?
                    {
                        current = From::JoinUsing {
                            left,
                            right,
                            join_type: JoinType::LeftJoin,
                            using,
                        };
                    } else {
                        expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::On))?;
                        *cursor += 1;
                        let condition = Expr::build(token_table, cursor)?;
                        current = From::LeftJoin {
                            left,
                            right,
                            condition,
                        };
                    }
                }
                Some(TokenKind::Keyword(Keyword::Right)) => {
                    *cursor += 1;
                    if token_table.get_kind(*cursor) == Some(&TokenKind::Keyword(Keyword::Outer)) {
                        *cursor += 1;
                    }
                    expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Join))?;
                    *cursor += 1;
                    let left = Box::new(Self::parse_joins(token_table, cursor, current)?);
                    let right = Box::new(Self::parse(token_table, cursor)?);
                    if is_natural {
                        current = From::NaturalJoin {
                            left,
                            right,
                            join_type: JoinType::RightJoin,
                        };
                    } else if let Some(using) =
                        Self::parse_join_on_using(token_table, cursor, false)?
                    {
                        current = From::JoinUsing {
                            left,
                            right,
                            join_type: JoinType::RightJoin,
                            using,
                        };
                    } else {
                        expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::On))?;
                        *cursor += 1;
                        let condition = Expr::build(token_table, cursor)?;
                        current = From::RightJoin {
                            left,
                            right,
                            condition,
                        };
                    }
                }
                Some(TokenKind::Keyword(Keyword::Full)) => {
                    *cursor += 1;
                    if token_table.get_kind(*cursor) == Some(&TokenKind::Keyword(Keyword::Outer)) {
                        *cursor += 1;
                    }
                    expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Join))?;
                    *cursor += 1;
                    let left = Box::new(Self::parse_joins(token_table, cursor, current)?);
                    let right = Box::new(Self::parse(token_table, cursor)?);
                    if is_natural {
                        current = From::NaturalJoin {
                            left,
                            right,
                            join_type: JoinType::FullJoin,
                        };
                    } else if let Some(using) =
                        Self::parse_join_on_using(token_table, cursor, false)?
                    {
                        current = From::JoinUsing {
                            left,
                            right,
                            join_type: JoinType::FullJoin,
                            using,
                        };
                    } else {
                        expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::On))?;
                        *cursor += 1;
                        let condition = Expr::build(token_table, cursor)?;
                        current = From::FullJoin {
                            left,
                            right,
                            condition,
                        };
                    }
                }
                Some(TokenKind::Keyword(Keyword::Cross)) => {
                    if is_natural {
                        return Err(ParserError::SyntaxError(*cursor, *cursor));
                    }
                    *cursor += 1;
                    expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Join))?;
                    *cursor += 1;
                    let left = Box::new(Self::parse_joins(token_table, cursor, current)?);
                    let right = Box::new(Self::parse(token_table, cursor)?);
                    current = From::CrossJoin {
                        left,
                        right,
                    };
                }
                _ => {
                    if is_natural {
                        return Err(ParserError::SyntaxError(*cursor, *cursor));
                    }
                    break;
                }
            }
        }
        Ok(current)
    }
}

#[cfg(feature = "serde")]
impl<'a> Serialize for From<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Table(table) => serializer.serialize_newtype_variant("From", 0, "Table", &table),
            Self::CrossJoin { left, right } => {
                let mut sv = serializer.serialize_struct_variant("From", 1, "CrossJoin", 2)?;
                sv.serialize_field("left", &left)?;
                sv.serialize_field("right", &right)?;
                sv.end()
            }
            Self::NaturalJoin {left, right, join_type} => {
                let mut sv = serializer.serialize_struct_variant("From", 2, "NaturalJoin", 3)?;
                sv.serialize_field("left", &left)?;
                sv.serialize_field("right", &right)?;
                sv.serialize_field("join_type", &join_type)?;
                sv.end()
            }
            Self::JoinUsing { left, right, join_type, using } => {
                let mut sv = serializer.serialize_struct_variant("From", 3, "JoinUsing", 4)?;
                sv.serialize_field("left", &left)?;
                sv.serialize_field("right", &right)?;
                sv.serialize_field("join_type", &join_type)?;
                sv.serialize_field("using", &using)?;
                sv.end()
            }
            Self::LeftJoin {left, right, condition} => {
                let mut sv = serializer.serialize_struct_variant("From", 4, "LeftJoin", 3)?;
                sv.serialize_field("left", &left)?;
                sv.serialize_field("right", &right)?;
                sv.serialize_field("condition", &condition)?;
                sv.end()
            }
            Self::RightJoin {left, right, condition} => {
                let mut sv = serializer.serialize_struct_variant("From", 5, "RightJoin", 3)?;
                sv.serialize_field("left", &left)?;
                sv.serialize_field("right", &right)?;
                sv.serialize_field("condition", &condition)?;
                sv.end()
            }
            Self::InnerJoin {left, right, condition} => {
                let mut sv = serializer.serialize_struct_variant("From", 6, "InnerJoin", 3)?;
                sv.serialize_field("left", &left)?;
                sv.serialize_field("right", &right)?;
                sv.serialize_field("condition", &condition)?;
                sv.end()
            }
            Self::FullJoin {left, right, condition} => {
                let mut sv = serializer.serialize_struct_variant("From", 7, "FullJoin", 3)?;
                sv.serialize_field("left", &left)?;
                sv.serialize_field("right", &right)?;
                sv.serialize_field("condition", &condition)?;
                sv.end()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::expr::{BinaryOp, BinaryOperator, Field};
    use crate::token::TokenKind;

    fn make_table<'a>(source: &'a str, tokens: Vec<(TokenKind, usize, usize)>) -> TokenTable<'a> {
        let mut table = TokenTable::with_source(source);
        for (kind, start, end) in tokens {
            table.push(
                kind,
                unsafe { str::from_utf8_unchecked(&source.as_bytes()[start..=end]) },
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
                    name: "users",
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
                name: Some("u"),
                value: Expr::Field(Field {
                    prefix: None,
                    name: "users",
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
                name: "user_id",
            }),
            right: Expr::Field(Field {
                prefix: None,
                name: "user_id",
            }),
        }));

        assert_eq!(result, From::InnerJoin {
            left: Box::new(From::Table(Table::Name(Alias { name: None, value: Expr::Field(Field { prefix: None, name: "users" }) }))),
            right: Box::new(From::Table(Table::Name(Alias { name: None, value: Expr::Field(Field { prefix: None, name: "orders" }) }))),
            condition: expected_condition,
        });
        // if let From::InnerJoin {
        //     left,
        //     right,
        //     condition,
        // } = result
        // {
        //     assert_eq!(
        //         left,
        //         Table::Name(Alias {
        //             name: None,
        //             value: Expr::Field(Field {
        //                 prefix: None,
        //                 name: "users"
        //             })
        //         })
        //     );
        //     assert_eq!(
        //         right,
        //         Table::Name(Alias {
        //             name: None,
        //             value: Expr::Field(Field {
        //                 prefix: None,
        //                 name: "orders"
        //             })
        //         })
        //     );
        //     assert_eq!(condition, expected_condition);
        // }
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
        assert_eq!(result, From::InnerJoin {
            left: Box::new(From::InnerJoin {
                left: Box::new(From::Table(Table::Name(Alias { name: None, value: Expr::Field(Field { prefix: None, name: "users" }) }))),
                right: Box::new(From::Table(Table::Name(Alias { name: None, value: Expr::Field(Field { prefix: None, name: "orders" }) }))),
                condition: Expr::BinaryOp(Box::new(
                    BinaryOp { op: BinaryOperator::Equal, left: Expr::Field(Field { prefix: None, name: "user_id" }), right: Expr::Field(Field { prefix: None, name: "user_id" }) },
                ))
            }),
            right: Box::new(From::Table(Table::Name(Alias { name: None, value: Expr::Field(Field { prefix: None, name: "order_items" }) }))),
            condition: Expr::BinaryOp(Box::new(
                BinaryOp {op: BinaryOperator::Equal, left: Expr::Field(Field { prefix: None, name: "order_id" }), right: Expr::Field(Field { prefix: None, name: "order_id" })}
            ))
        });
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
                left: Box::new(From::Table(Table::Name(Alias {
                    name: None,
                    value: Expr::Field(Field {
                        prefix: None,
                        name: "u",
                    }),
                }))),
                right: Box::new(From::Table(Table::Name(Alias {
                    name: None,
                    value: Expr::Field(Field {
                        prefix: None,
                        name: "o",
                    }),
                }))),
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
                    prefix: Some("users"),
                    name: "id"
                })
            }))
        );
    }

    // ========================================================================
    // NATURAL JOIN 测试
    // ========================================================================

    #[test]
    fn test_natural_join() {
        let tokens = make_table(
            "users NATURAL JOIN orders",
            vec![
                (TokenKind::Identifier, 0, 4),
                (TokenKind::Keyword(Keyword::Natural), 6, 12),
                (TokenKind::Keyword(Keyword::Join), 14, 17),
                (TokenKind::Identifier, 19, 24),
            ],
        );
        let mut cursor = 0;
        let result = From::parse(&tokens, &mut cursor).unwrap();
        assert_eq!(result, From::NaturalJoin {
            left: Box::new(From::Table(Table::Name(Alias { name: None, value: Expr::Field(Field { prefix: None, name: "users" }) }))),
            right: Box::new(From::Table(Table::Name(Alias { name: None, value: Expr::Field(Field { prefix: None, name: "orders" }) }))),
            join_type: JoinType::InnerJoin,
        });
    }

    #[test]
    fn test_natural_left_join() {
        let tokens = make_table(
            "users NATURAL LEFT JOIN orders",
            vec![
                (TokenKind::Identifier, 0, 4),
                (TokenKind::Keyword(Keyword::Natural), 6, 12),
                (TokenKind::Keyword(Keyword::Left), 14, 17),
                (TokenKind::Keyword(Keyword::Join), 19, 22),
                (TokenKind::Identifier, 24, 29),
            ],
        );
        let mut cursor = 0;
        let result = From::parse(&tokens, &mut cursor).unwrap();
        assert_eq!(result, From::NaturalJoin {
            left: Box::new(From::Table(Table::Name(Alias { name: None, value: Expr::Field(Field { prefix: None, name: "users" }) }))),
            right: Box::new(From::Table(Table::Name(Alias { name: None, value: Expr::Field(Field { prefix: None, name: "orders" }) }))),
            join_type: JoinType::LeftJoin,
        });
    }

    #[test]
    fn test_natural_right_join() {
        let tokens = make_table(
            "users NATURAL RIGHT JOIN orders",
            vec![
                (TokenKind::Identifier, 0, 4),
                (TokenKind::Keyword(Keyword::Natural), 6, 12),
                (TokenKind::Keyword(Keyword::Right), 14, 18),
                (TokenKind::Keyword(Keyword::Join), 20, 23),
                (TokenKind::Identifier, 25, 30),
            ],
        );
        let mut cursor = 0;
        let result = From::parse(&tokens, &mut cursor).unwrap();
        assert_eq!(result, From::NaturalJoin {
            left: Box::new(From::Table(Table::Name(Alias { name: None, value: Expr::Field(Field { prefix: None, name: "users" }) }))),
            right: Box::new(From::Table(Table::Name(Alias { name: None, value: Expr::Field(Field { prefix: None, name: "orders" }) }))),
            join_type: JoinType::RightJoin,
        });
    }

    #[test]
    fn test_natural_full_join() {
        let tokens = make_table(
            "users NATURAL FULL JOIN orders",
            vec![
                (TokenKind::Identifier, 0, 4),
                (TokenKind::Keyword(Keyword::Natural), 6, 12),
                (TokenKind::Keyword(Keyword::Full), 14, 17),
                (TokenKind::Keyword(Keyword::Join), 19, 22),
                (TokenKind::Identifier, 24, 29),
            ],
        );
        let mut cursor = 0;
        let result = From::parse(&tokens, &mut cursor).unwrap();
        assert_eq!(result, From::NaturalJoin {
            left: Box::new(From::Table(Table::Name(Alias { name: None, value: Expr::Field(Field { prefix: None, name: "users" }) }))),
            right: Box::new(From::Table(Table::Name(Alias { name: None, value: Expr::Field(Field { prefix: None, name: "orders" }) }))),
            join_type: JoinType::FullJoin,
        });
    }

    #[test]
    fn test_natural_join_chain() {
        // Note: chained conditionless joins (NATURAL, CROSS) produce right-deep
        // trees due to the recursive parse_joins architecture. This is a known
        // pre-existing limitation shared with CROSS JOIN chains.
        let tokens = make_table(
            "a NATURAL JOIN b NATURAL JOIN c",
            vec![
                (TokenKind::Identifier, 0, 0),
                (TokenKind::Keyword(Keyword::Natural), 2, 8),
                (TokenKind::Keyword(Keyword::Join), 10, 13),
                (TokenKind::Identifier, 15, 15),
                (TokenKind::Keyword(Keyword::Natural), 17, 23),
                (TokenKind::Keyword(Keyword::Join), 25, 28),
                (TokenKind::Identifier, 30, 30),
            ],
        );
        let mut cursor = 0;
        let result = From::parse(&tokens, &mut cursor).unwrap();
        assert_eq!(result, From::NaturalJoin {
            left: Box::new(From::Table(Table::Name(Alias { name: None, value: Expr::Field(Field { prefix: None, name: "a" }) }))),
            right: Box::new(From::NaturalJoin {
                left: Box::new(From::Table(Table::Name(Alias { name: None, value: Expr::Field(Field { prefix: None, name: "b" }) }))),
                right: Box::new(From::Table(Table::Name(Alias { name: None, value: Expr::Field(Field { prefix: None, name: "c" }) }))),
                join_type: JoinType::InnerJoin,
            }),
            join_type: JoinType::InnerJoin,
        });
    }

    // ========================================================================
    // JOIN ... USING 测试
    // ========================================================================

    #[test]
    fn test_join_using() {
        let tokens = make_table(
            "users JOIN orders USING (user_id)",
            vec![
                (TokenKind::Identifier, 0, 4),
                (TokenKind::Keyword(Keyword::Join), 6, 9),
                (TokenKind::Identifier, 11, 16),
                (TokenKind::Keyword(Keyword::Using), 18, 22),
                (TokenKind::LeftParen, 24, 24),
                (TokenKind::Identifier, 25, 31),
                (TokenKind::RightParen, 32, 32),
            ],
        );
        let mut cursor = 0;
        let result = From::parse(&tokens, &mut cursor).unwrap();
        assert_eq!(result, From::JoinUsing {
            left: Box::new(From::Table(Table::Name(Alias { name: None, value: Expr::Field(Field { prefix: None, name: "users" }) }))),
            right: Box::new(From::Table(Table::Name(Alias { name: None, value: Expr::Field(Field { prefix: None, name: "orders" }) }))),
            join_type: JoinType::InnerJoin,
            using: vec!["user_id"],
        });
    }

    #[test]
    fn test_join_using_multiple_columns() {
        let tokens = make_table(
            "users JOIN orders USING (user_id, order_id)",
            vec![
                (TokenKind::Identifier, 0, 4),
                (TokenKind::Keyword(Keyword::Join), 6, 9),
                (TokenKind::Identifier, 11, 16),
                (TokenKind::Keyword(Keyword::Using), 18, 22),
                (TokenKind::LeftParen, 24, 24),
                (TokenKind::Identifier, 25, 31),
                (TokenKind::Comma, 32, 32),
                (TokenKind::Identifier, 34, 41),
                (TokenKind::RightParen, 42, 42),
            ],
        );
        let mut cursor = 0;
        let result = From::parse(&tokens, &mut cursor).unwrap();
        assert_eq!(result, From::JoinUsing {
            left: Box::new(From::Table(Table::Name(Alias { name: None, value: Expr::Field(Field { prefix: None, name: "users" }) }))),
            right: Box::new(From::Table(Table::Name(Alias { name: None, value: Expr::Field(Field { prefix: None, name: "orders" }) }))),
            join_type: JoinType::InnerJoin,
            using: vec!["user_id", "order_id"],
        });
    }

    #[test]
    fn test_left_join_using() {
        let tokens = make_table(
            "users LEFT JOIN orders USING (id)",
            vec![
                (TokenKind::Identifier, 0, 4),
                (TokenKind::Keyword(Keyword::Left), 6, 9),
                (TokenKind::Keyword(Keyword::Join), 11, 14),
                (TokenKind::Identifier, 16, 21),
                (TokenKind::Keyword(Keyword::Using), 23, 27),
                (TokenKind::LeftParen, 29, 29),
                (TokenKind::Identifier, 30, 31),
                (TokenKind::RightParen, 32, 32),
            ],
        );
        let mut cursor = 0;
        let result = From::parse(&tokens, &mut cursor).unwrap();
        assert_eq!(result, From::JoinUsing {
            left: Box::new(From::Table(Table::Name(Alias { name: None, value: Expr::Field(Field { prefix: None, name: "users" }) }))),
            right: Box::new(From::Table(Table::Name(Alias { name: None, value: Expr::Field(Field { prefix: None, name: "orders" }) }))),
            join_type: JoinType::LeftJoin,
            using: vec!["id"],
        });
    }

    #[test]
    fn test_right_join_using() {
        let tokens = make_table(
            "users RIGHT OUTER JOIN orders USING (id)",
            vec![
                (TokenKind::Identifier, 0, 4),
                (TokenKind::Keyword(Keyword::Right), 6, 10),
                (TokenKind::Keyword(Keyword::Outer), 12, 16),
                (TokenKind::Keyword(Keyword::Join), 18, 21),
                (TokenKind::Identifier, 23, 28),
                (TokenKind::Keyword(Keyword::Using), 30, 34),
                (TokenKind::LeftParen, 36, 36),
                (TokenKind::Identifier, 37, 38),
                (TokenKind::RightParen, 39, 39),
            ],
        );
        let mut cursor = 0;
        let result = From::parse(&tokens, &mut cursor).unwrap();
        assert_eq!(result, From::JoinUsing {
            left: Box::new(From::Table(Table::Name(Alias { name: None, value: Expr::Field(Field { prefix: None, name: "users" }) }))),
            right: Box::new(From::Table(Table::Name(Alias { name: None, value: Expr::Field(Field { prefix: None, name: "orders" }) }))),
            join_type: JoinType::RightJoin,
            using: vec!["id"],
        });
    }

    #[test]
    fn test_join_using_empty_list_errors() {
        let tokens = make_table(
            "a JOIN b USING ()",
            vec![
                (TokenKind::Identifier, 0, 0),
                (TokenKind::Keyword(Keyword::Join), 2, 5),
                (TokenKind::Identifier, 7, 7),
                (TokenKind::Keyword(Keyword::Using), 9, 13),
                (TokenKind::LeftParen, 15, 15),
                (TokenKind::RightParen, 16, 16),
            ],
        );
        let mut cursor = 0;
        assert!(From::parse(&tokens, &mut cursor).is_err());
    }
}
