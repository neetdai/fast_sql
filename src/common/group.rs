use minivec::MiniVec;

use crate::{
    ParserError,
    common::{expr::Expr, utils::expect_kind},
    keyword::Keyword,
    token::{TokenKind, TokenTable},
};

#[cfg(feature = "serde")]
use serde::{
    ser::{Serialize, Serializer, SerializeStruct},
};

#[derive(Debug, PartialEq)]
pub enum GroupByExpr<'a> {
    Simple(Expr<'a>),
    GroupingSets(Vec<Vec<Expr<'a>>>),
    Cube(Vec<Expr<'a>>),
    Rollup(Vec<Expr<'a>>),
}

#[cfg(feature = "serde")]
impl<'a> serde::Serialize for GroupByExpr<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Simple(simple) => serializer.serialize_newtype_variant("GroupByExpr", 0, "Simple", simple),
            Self::GroupingSets(grouping_sets) => serializer.serialize_newtype_variant("GroupByExpr", 1, "GroupingSets", grouping_sets),
            Self::Cube(cube) => serializer.serialize_newtype_variant("GroupByExpr", 2, "Cube", cube),
            Self::Rollup(rollup) => serializer.serialize_newtype_variant("GroupByExpr", 3, "Rollup", rollup),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Group<'a> {
    pub columns: MiniVec<GroupByExpr<'a>>,
}

impl<'a> Group<'a> {
    fn parse_rollup_or_cube(
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
    ) -> Result<Vec<Expr<'a>>, ParserError> {
        expect_kind(token_table, cursor, &TokenKind::LeftParen)?;
        *cursor += 1;

        let mut exprs = Vec::new();
        loop {
            match token_table.get_kind(*cursor) {
                Some(TokenKind::RightParen) => {
                    *cursor += 1;
                    break;
                }
                Some(TokenKind::Comma) => {
                    *cursor += 1;
                }
                Some(_) => {
                    let expr = Expr::build(token_table, cursor)?;
                    exprs.push(expr);
                }
                None => return Err(ParserError::SyntaxError(*cursor, *cursor)),
            }
        }

        if exprs.is_empty() {
            return Err(ParserError::SyntaxError(*cursor, *cursor));
        }
        Ok(exprs)
    }

    fn parse_grouping_sets(
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
    ) -> Result<Vec<Vec<Expr<'a>>>, ParserError> {
        expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Sets))?;
        *cursor += 1;

        expect_kind(token_table, cursor, &TokenKind::LeftParen)?;
        *cursor += 1;

        let mut sets = Vec::new();
        loop {
            match token_table.get_kind(*cursor) {
                Some(TokenKind::RightParen) => {
                    *cursor += 1;
                    break;
                }
                Some(TokenKind::Comma) => {
                    *cursor += 1;
                }
                Some(TokenKind::LeftParen) => {
                    *cursor += 1;
                    let mut exprs = Vec::new();
                    loop {
                        match token_table.get_kind(*cursor) {
                            Some(TokenKind::RightParen) => {
                                *cursor += 1;
                                break;
                            }
                            Some(TokenKind::Comma) => {
                                *cursor += 1;
                            }
                            Some(_) => {
                                let expr = Expr::build(token_table, cursor)?;
                                exprs.push(expr);
                            }
                            None => return Err(ParserError::SyntaxError(*cursor, *cursor)),
                        }
                    }
                    sets.push(exprs);
                }
                Some(_) => {
                    let expr = Expr::build(token_table, cursor)?;
                    sets.push(vec![expr]);
                }
                None => return Err(ParserError::SyntaxError(*cursor, *cursor)),
            }
        }

        if sets.is_empty() {
            return Err(ParserError::SyntaxError(*cursor, *cursor));
        }
        Ok(sets)
    }

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
                Some(TokenKind::Keyword(Keyword::Grouping)) => {
                    *cursor += 1;
                    let sets = Self::parse_grouping_sets(token_table, cursor)?;
                    columns.push(GroupByExpr::GroupingSets(sets));
                }
                Some(TokenKind::Keyword(Keyword::Cube)) => {
                    *cursor += 1;
                    let exprs = Self::parse_rollup_or_cube(token_table, cursor)?;
                    columns.push(GroupByExpr::Cube(exprs));
                }
                Some(TokenKind::Keyword(Keyword::Rollup)) => {
                    *cursor += 1;
                    let exprs = Self::parse_rollup_or_cube(token_table, cursor)?;
                    columns.push(GroupByExpr::Rollup(exprs));
                }
                Some(_) => {
                    let expr = Expr::build(token_table, cursor)?;
                    columns.push(GroupByExpr::Simple(expr));
                }
                _ => break,
            }
        }

        Ok(Self { columns })
    }
}

#[cfg(feature = "serde")]
impl<'a> Serialize for Group<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("Group", 1)?;
        s.serialize_field("columns", &self.columns)?;
        s.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::TokenKind;

    fn make_table<'a>(source: &'a str, tokens: Vec<(TokenKind, usize, usize)>) -> TokenTable<'a> {
        let mut table = TokenTable::with_source(source);
        for (kind, start, end) in tokens {
            table.push(
                kind,
                unsafe { std::str::from_utf8_unchecked(&source.as_bytes()[start..=end]) },
            );
        }
        table
    }

    #[test]
    fn test_simple_group_by() {
        let tokens = make_table(
            "GROUP BY a",
            vec![
                (TokenKind::Keyword(Keyword::Group), 0, 4),
                (TokenKind::Keyword(Keyword::By), 6, 7),
                (TokenKind::Identifier, 9, 9),
            ],
        );
        let mut cursor = 0;
        let result = Group::build(&tokens, &mut cursor).unwrap();
        assert_eq!(result.columns.len(), 1);
        assert_eq!(
            result.columns[0],
            GroupByExpr::Simple(Expr::Field(crate::common::expr::Field {
                prefix: None,
                name: "a"
            }))
        );
    }

    #[test]
    fn test_group_by_multiple() {
        let tokens = make_table(
            "GROUP BY a, b",
            vec![
                (TokenKind::Keyword(Keyword::Group), 0, 4),
                (TokenKind::Keyword(Keyword::By), 6, 7),
                (TokenKind::Identifier, 9, 9),
                (TokenKind::Comma, 10, 10),
                (TokenKind::Identifier, 12, 12),
            ],
        );
        let mut cursor = 0;
        let result = Group::build(&tokens, &mut cursor).unwrap();
        assert_eq!(result.columns.len(), 2);
        assert_eq!(
            result.columns[0],
            GroupByExpr::Simple(Expr::Field(crate::common::expr::Field {
                prefix: None,
                name: "a"
            }))
        );
        assert_eq!(
            result.columns[1],
            GroupByExpr::Simple(Expr::Field(crate::common::expr::Field {
                prefix: None,
                name: "b"
            }))
        );
    }

    // ========================================================================
    // ROLLUP 测试
    // ========================================================================

    #[test]
    fn test_group_by_rollup() {
        let tokens = make_table(
            "GROUP BY ROLLUP (a, b)",
            vec![
                (TokenKind::Keyword(Keyword::Group), 0, 4),
                (TokenKind::Keyword(Keyword::By), 6, 7),
                (TokenKind::Keyword(Keyword::Rollup), 9, 14),
                (TokenKind::LeftParen, 16, 16),
                (TokenKind::Identifier, 17, 17),
                (TokenKind::Comma, 18, 18),
                (TokenKind::Identifier, 20, 20),
                (TokenKind::RightParen, 21, 21),
            ],
        );
        let mut cursor = 0;
        let result = Group::build(&tokens, &mut cursor).unwrap();
        assert_eq!(result.columns.len(), 1);
        assert_eq!(
            result.columns[0],
            GroupByExpr::Rollup(vec![
                Expr::Field(crate::common::expr::Field {
                    prefix: None,
                    name: "a"
                }),
                Expr::Field(crate::common::expr::Field {
                    prefix: None,
                    name: "b"
                }),
            ])
        );
    }

    // ========================================================================
    // CUBE 测试
    // ========================================================================

    #[test]
    fn test_group_by_cube() {
        let tokens = make_table(
            "GROUP BY CUBE (a, b, c)",
            vec![
                (TokenKind::Keyword(Keyword::Group), 0, 4),
                (TokenKind::Keyword(Keyword::By), 6, 7),
                (TokenKind::Keyword(Keyword::Cube), 9, 12),
                (TokenKind::LeftParen, 14, 14),
                (TokenKind::Identifier, 15, 15),
                (TokenKind::Comma, 16, 16),
                (TokenKind::Identifier, 18, 18),
                (TokenKind::Comma, 19, 19),
                (TokenKind::Identifier, 21, 21),
                (TokenKind::RightParen, 22, 22),
            ],
        );
        let mut cursor = 0;
        let result = Group::build(&tokens, &mut cursor).unwrap();
        assert_eq!(result.columns.len(), 1);
        let expected_exprs = vec!["a", "b", "c"]
            .into_iter()
            .map(|name| {
                Expr::Field(crate::common::expr::Field {
                    prefix: None,
                    name,
                })
            })
            .collect::<Vec<_>>();
        assert_eq!(result.columns[0], GroupByExpr::Cube(expected_exprs));
    }

    // ========================================================================
    // GROUPING SETS 测试
    // ========================================================================

    #[test]
    fn test_group_by_grouping_sets() {
        let tokens = make_table(
            "GROUP BY GROUPING SETS ((a, b), c)",
            vec![
                (TokenKind::Keyword(Keyword::Group), 0, 4),
                (TokenKind::Keyword(Keyword::By), 6, 7),
                (TokenKind::Keyword(Keyword::Grouping), 9, 16),
                (TokenKind::Keyword(Keyword::Sets), 18, 21),
                (TokenKind::LeftParen, 23, 23),
                (TokenKind::LeftParen, 24, 24),
                (TokenKind::Identifier, 25, 25),
                (TokenKind::Comma, 26, 26),
                (TokenKind::Identifier, 28, 28),
                (TokenKind::RightParen, 29, 29),
                (TokenKind::Comma, 30, 30),
                (TokenKind::Identifier, 32, 32),
                (TokenKind::RightParen, 33, 33),
            ],
        );
        let mut cursor = 0;
        let result = Group::build(&tokens, &mut cursor).unwrap();
        assert_eq!(result.columns.len(), 1);
        match &result.columns[0] {
            GroupByExpr::GroupingSets(sets) => {
                assert_eq!(sets.len(), 2);
                assert_eq!(sets[0].len(), 2);
                assert_eq!(
                    sets[0][0],
                    Expr::Field(crate::common::expr::Field {
                        prefix: None,
                        name: "a"
                    })
                );
                assert_eq!(
                    sets[0][1],
                    Expr::Field(crate::common::expr::Field {
                        prefix: None,
                        name: "b"
                    })
                );
                assert_eq!(sets[1].len(), 1);
                assert_eq!(
                    sets[1][0],
                    Expr::Field(crate::common::expr::Field {
                        prefix: None,
                        name: "c"
                    })
                );
            }
            _ => panic!("expected GroupingSets"),
        }
    }

    #[test]
    fn test_group_by_grouping_sets_multi_column_groups() {
        let tokens = make_table(
            "GROUP BY GROUPING SETS ((a, b), (c, d))",
            vec![
                (TokenKind::Keyword(Keyword::Group), 0, 4),
                (TokenKind::Keyword(Keyword::By), 6, 7),
                (TokenKind::Keyword(Keyword::Grouping), 9, 16),
                (TokenKind::Keyword(Keyword::Sets), 18, 21),
                (TokenKind::LeftParen, 23, 23),
                (TokenKind::LeftParen, 24, 24),
                (TokenKind::Identifier, 25, 25),
                (TokenKind::Comma, 26, 26),
                (TokenKind::Identifier, 28, 28),
                (TokenKind::RightParen, 29, 29),
                (TokenKind::Comma, 30, 30),
                (TokenKind::LeftParen, 32, 32),
                (TokenKind::Identifier, 33, 33),
                (TokenKind::Comma, 34, 34),
                (TokenKind::Identifier, 36, 36),
                (TokenKind::RightParen, 37, 37),
                (TokenKind::RightParen, 38, 38),
            ],
        );
        let mut cursor = 0;
        let result = Group::build(&tokens, &mut cursor).unwrap();
        assert_eq!(result.columns.len(), 1);
        match &result.columns[0] {
            GroupByExpr::GroupingSets(sets) => {
                assert_eq!(sets.len(), 2);
                assert_eq!(sets[0].len(), 2);
                assert_eq!(sets[1].len(), 2);
            }
            _ => panic!("expected GroupingSets"),
        }
    }

    // ========================================================================
    // 混合 GROUP BY 测试
    // ========================================================================

    #[test]
    fn test_group_by_mixed_rollup_and_simple() {
        let tokens = make_table(
            "GROUP BY a, ROLLUP (b, c)",
            vec![
                (TokenKind::Keyword(Keyword::Group), 0, 4),
                (TokenKind::Keyword(Keyword::By), 6, 7),
                (TokenKind::Identifier, 9, 9),
                (TokenKind::Comma, 10, 10),
                (TokenKind::Keyword(Keyword::Rollup), 12, 17),
                (TokenKind::LeftParen, 19, 19),
                (TokenKind::Identifier, 20, 20),
                (TokenKind::Comma, 21, 21),
                (TokenKind::Identifier, 23, 23),
                (TokenKind::RightParen, 24, 24),
            ],
        );
        let mut cursor = 0;
        let result = Group::build(&tokens, &mut cursor).unwrap();
        assert_eq!(result.columns.len(), 2);
        assert!(matches!(result.columns[0], GroupByExpr::Simple(_)));
        assert!(matches!(result.columns[1], GroupByExpr::Rollup(_)));
    }

    // ========================================================================
    // 错误路径测试
    // ========================================================================

    #[test]
    fn test_rollup_empty_parens_errors() {
        let tokens = make_table(
            "GROUP BY ROLLUP ()",
            vec![
                (TokenKind::Keyword(Keyword::Group), 0, 4),
                (TokenKind::Keyword(Keyword::By), 6, 7),
                (TokenKind::Keyword(Keyword::Rollup), 9, 14),
                (TokenKind::LeftParen, 16, 16),
                (TokenKind::RightParen, 17, 17),
            ],
        );
        let mut cursor = 0;
        assert!(Group::build(&tokens, &mut cursor).is_err());
    }

    #[test]
    fn test_grouping_sets_empty_errors() {
        let tokens = make_table(
            "GROUP BY GROUPING SETS ()",
            vec![
                (TokenKind::Keyword(Keyword::Group), 0, 4),
                (TokenKind::Keyword(Keyword::By), 6, 7),
                (TokenKind::Keyword(Keyword::Grouping), 9, 16),
                (TokenKind::Keyword(Keyword::Sets), 18, 21),
                (TokenKind::LeftParen, 23, 23),
                (TokenKind::RightParen, 24, 24),
            ],
        );
        let mut cursor = 0;
        assert!(Group::build(&tokens, &mut cursor).is_err());
    }
}
