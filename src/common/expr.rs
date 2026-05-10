use std::borrow::Cow;

use minivec::MiniVec;

use crate::{
    ParserError, SelectStatement,
    ast::select::SubSelectStatement,
    common::{
        alias::Aliasable,
        order::Order,
        pratt_parser::{Flow, PrattOutput, PrattParser, PrattParserTrait, PrecedenceTrait},
        utils::{expect_kind, maybe_kind},
    },
    keyword::Keyword,
    token::{TokenKind, TokenTable},
};

#[derive(Debug, PartialEq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Mod,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    BitAnd,
    BitXor,
    LeftShift,
    RightShift,
    And,
    Or,
    Between,
    In,
    Like,
    Not,
}

impl BinaryOperator {
    pub(crate) fn from_token_kind(kind: &TokenKind) -> Option<Self> {
        match kind {
            TokenKind::Plus => Some(BinaryOperator::Add),
            TokenKind::Subtract => Some(BinaryOperator::Subtract),
            TokenKind::Multiply => Some(BinaryOperator::Multiply),
            TokenKind::Divide => Some(BinaryOperator::Divide),
            TokenKind::Mod => Some(BinaryOperator::Mod),
            TokenKind::Equal => Some(BinaryOperator::Equal),
            TokenKind::NotEqual => Some(BinaryOperator::NotEqual),
            TokenKind::Less => Some(BinaryOperator::Less),
            TokenKind::LessEqual => Some(BinaryOperator::LessEqual),
            TokenKind::Greater => Some(BinaryOperator::Greater),
            TokenKind::GreaterEqual => Some(BinaryOperator::GreaterEqual),
            TokenKind::BitAnd => Some(BinaryOperator::BitAnd),
            TokenKind::BitXor => Some(BinaryOperator::BitXor),
            TokenKind::LeftShift => Some(BinaryOperator::LeftShift),
            TokenKind::RightShift => Some(BinaryOperator::RightShift),
            TokenKind::Keyword(Keyword::And) => Some(BinaryOperator::And),
            TokenKind::Keyword(Keyword::Or) => Some(BinaryOperator::Or),
            TokenKind::Keyword(Keyword::Between) => Some(BinaryOperator::Between),
            TokenKind::Keyword(Keyword::In) => Some(BinaryOperator::In),
            TokenKind::Keyword(Keyword::Like) => Some(BinaryOperator::Like),
            _ => None,
        }
    }
}

impl PrecedenceTrait for BinaryOperator {
    fn precedence(&self) -> usize {
        match self {
            BinaryOperator::Or => 1,
            BinaryOperator::And => 2,
            BinaryOperator::Equal | BinaryOperator::NotEqual => 3,
            BinaryOperator::Not
            | BinaryOperator::Between
            | BinaryOperator::In
            | BinaryOperator::Like => 4,
            BinaryOperator::Less
            | BinaryOperator::LessEqual
            | BinaryOperator::Greater
            | BinaryOperator::GreaterEqual => 5,
            BinaryOperator::LeftShift
            | BinaryOperator::RightShift
            | BinaryOperator::BitAnd
            | BinaryOperator::BitXor => 6,
            BinaryOperator::Add | BinaryOperator::Subtract => 7,
            BinaryOperator::Multiply | BinaryOperator::Divide | BinaryOperator::Mod => 8,
        }
    }

    fn is_left_associative(&self) -> bool {
        true
    }

    fn min_precedence() -> usize {
        0
    }
}

#[derive(Debug, PartialEq)]
pub struct BinaryOp<'a> {
    pub op: BinaryOperator,
    pub left: Expr<'a>,
    pub right: Expr<'a>,
}

#[derive(Debug, PartialEq)]
pub enum Expr<'a> {
    Field(Field<'a>),
    Star(Star<'a>),
    FunctionCall(FunctionCall<'a>),
    StringLiteral(StringLiteral<'a>),
    NumericLiteral(NumericLiteral<'a>),
    BinaryOp(Box<BinaryOp<'a>>),
    Between(Between<'a>),
    In(In<'a>),
    Case(CaseExpr<'a>),
    Like(Like<'a>),
    IsNull(IsNull<'a>),
    Exists(Box<ExistsExpr<'a>>),
    BoolLiteral(bool),
    NullLiteral,
    WindowFunction(Box<WindowFunction<'a>>),
}

impl<'a> Expr<'a> {
    pub(crate) fn class_field(
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
    ) -> Result<Self, ParserError> {
        Field::from_token(token_table, cursor).map(Expr::Field)
    }

    pub(crate) fn class_function_call(
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
    ) -> Result<Self, ParserError> {
        FunctionCall::from_token(token_table, cursor).map(Expr::FunctionCall)
    }

    pub(crate) fn class_string_literal(
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
    ) -> Result<Self, ParserError> {
        StringLiteral::from_token(token_table, cursor).map(Expr::StringLiteral)
    }

    pub(crate) fn class_number_literal(
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
    ) -> Result<Self, ParserError> {
        NumericLiteral::from_token(token_table, cursor).map(Expr::NumericLiteral)
    }

    pub(crate) fn class_star(
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
    ) -> Result<Self, ParserError> {
        Star::from_token(token_table, cursor).map(Expr::Star)
    }

    pub(crate) fn parse_expression(
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
    ) -> Result<Self, ParserError> {
        PrattParser::parse_expression::<Self>(token_table, cursor)
    }

    pub(crate) fn build(
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
    ) -> Result<Self, ParserError> {
        Self::parse_expression(token_table, cursor)
    }
}

impl<'a> Aliasable<'a> for Expr<'a> {
    fn aliasable(token_table: &TokenTable<'a>, cursor: &mut usize) -> Result<Self, ParserError> {
        Self::build(token_table, cursor)
    }
}

impl<'a> PrattOutput<BinaryOperator> for Expr<'a> {
    fn apply(op: BinaryOperator, left: Self, right: Self) -> Self {
        Expr::BinaryOp(Box::new(BinaryOp { op, left, right }))
    }
}

impl<'a> PrattParserTrait<'a> for Expr<'a> {
    type Item = BinaryOperator;
    type Output = Self;

    fn parse_primary(
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
    ) -> Result<Self::Output, ParserError> {
        match token_table.get_kind(*cursor) {
            Some(TokenKind::Number) => Self::class_number_literal(token_table, cursor),
            Some(TokenKind::StringLiteral) => Self::class_string_literal(token_table, cursor),
            Some(TokenKind::Identifier) => {
                if let Some(TokenKind::LeftParen) = token_table.get_kind(*cursor + 1) {
                    Self::class_function_call(token_table, cursor)
                } else if let Ok(star) = Self::class_star(token_table, cursor) {
                    Ok(star)
                } else {
                    Self::class_field(token_table, cursor)
                }
            }
            Some(TokenKind::Multiply) => Self::class_star(token_table, cursor),
            Some(TokenKind::LeftParen) => {
                *cursor += 1;
                let expr = Self::parse_expression(token_table, cursor)?;
                expect_kind(token_table, cursor, &TokenKind::RightParen)?;
                *cursor += 1;
                Ok(expr)
            }
            Some(TokenKind::Keyword(Keyword::Case)) => Self::class_case(token_table, cursor),
            Some(TokenKind::Keyword(Keyword::True)) => {
                *cursor += 1;
                Ok(Expr::BoolLiteral(true))
            }
            Some(TokenKind::Keyword(Keyword::False)) => {
                *cursor += 1;
                Ok(Expr::BoolLiteral(false))
            }
            Some(TokenKind::Keyword(Keyword::Null)) => {
                *cursor += 1;
                Ok(Expr::NullLiteral)
            }
            Some(TokenKind::Keyword(Keyword::Exists)) => {
                *cursor += 1;
                expect_kind(token_table, cursor, &TokenKind::LeftParen)?;
                *cursor += 1;
                let select_stmt = SelectStatement::new(token_table, cursor)?;
                expect_kind(token_table, cursor, &TokenKind::RightParen)?;
                *cursor += 1;
                Ok(Expr::Exists(Box::new(ExistsExpr {
                    is_not: false,
                    subquery: Box::new(select_stmt),
                })))
            }
            _ => Err(ParserError::SyntaxError(*cursor, *cursor)),
        }
    }

    fn match_item(token_table: &TokenTable, cursor: &mut usize) -> Option<Self::Item> {
        match token_table.get_kind(*cursor) {
            Some(TokenKind::Plus) => {
                *cursor += 1;
                Some(BinaryOperator::Add)
            }
            Some(TokenKind::Subtract) => {
                *cursor += 1;
                Some(BinaryOperator::Subtract)
            }
            Some(TokenKind::Multiply) => {
                *cursor += 1;
                Some(BinaryOperator::Multiply)
            }
            Some(TokenKind::Divide) => {
                *cursor += 1;
                Some(BinaryOperator::Divide)
            }
            Some(TokenKind::Mod) => {
                *cursor += 1;
                Some(BinaryOperator::Mod)
            }
            Some(TokenKind::Equal) => {
                *cursor += 1;
                Some(BinaryOperator::Equal)
            }
            Some(TokenKind::NotEqual) => {
                *cursor += 1;
                Some(BinaryOperator::NotEqual)
            }
            Some(TokenKind::Less) => {
                *cursor += 1;
                Some(BinaryOperator::Less)
            }
            Some(TokenKind::LessEqual) => {
                *cursor += 1;
                Some(BinaryOperator::LessEqual)
            }
            Some(TokenKind::Greater) => {
                *cursor += 1;
                Some(BinaryOperator::Greater)
            }
            Some(TokenKind::GreaterEqual) => {
                *cursor += 1;
                Some(BinaryOperator::GreaterEqual)
            }
            Some(TokenKind::Keyword(Keyword::And)) => {
                *cursor += 1;
                Some(BinaryOperator::And)
            }
            Some(TokenKind::Keyword(Keyword::Or)) => {
                *cursor += 1;
                Some(BinaryOperator::Or)
            }
            _ => None,
        }
    }

    fn parse_postfix(
        left: Self::Output,
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
    ) -> Result<(Self::Output, Flow), ParserError> {
        match token_table.get_kind(*cursor) {
            Some(&TokenKind::Keyword(Keyword::Not)) => {
                *cursor += 1;
                match token_table.get_kind(*cursor) {
                    Some(&TokenKind::Keyword(Keyword::Between)) => {
                        let between = Between::build(true, Box::new(left), token_table, cursor);
                        between.map(|e| (Expr::Between(e), Flow::Continue))
                    }
                    Some(&TokenKind::Keyword(Keyword::In)) => {
                        let in_expr = In::build(true, Box::new(left), token_table, cursor);
                        in_expr.map(|e| (Expr::In(e), Flow::Continue))
                    }
                    Some(&TokenKind::Keyword(Keyword::Like)) => {
                        let like = Like::build(true, Box::new(left), token_table, cursor);
                        like.map(|e| (Expr::Like(e), Flow::Continue))
                    }
                    Some(&TokenKind::Keyword(Keyword::Exists)) => {
                        let exists = ExistsExpr::build(true, token_table, cursor);
                        exists.map(|e| (Expr::Exists(Box::new(e)), Flow::Continue))
                    }
                    _ => Err(ParserError::SyntaxError(*cursor, *cursor)),
                }
            }
            Some(&TokenKind::Keyword(Keyword::Is)) => {
                *cursor += 1;
                let is_not = if maybe_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Not)) {
                    *cursor += 1;
                    true
                } else {
                    false
                };
                expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Null))?;
                *cursor += 1;
                Ok((
                    Expr::IsNull(IsNull {
                        is_not,
                        field: Box::new(left),
                    }),
                    Flow::Continue,
                ))
            }
            Some(&TokenKind::Keyword(Keyword::Between)) => {
                let between = Between::build(false, Box::new(left), token_table, cursor);
                between.map(|e| (Expr::Between(e), Flow::Continue))
            }
            Some(&TokenKind::Keyword(Keyword::In)) => {
                let in_ = In::build(false, Box::new(left), token_table, cursor);
                in_.map(|e| (Expr::In(e), Flow::Continue))
            }
            Some(&TokenKind::Keyword(Keyword::Like)) => {
                let like = Like::build(false, Box::new(left), token_table, cursor);
                like.map(|e| (Expr::Like(e), Flow::Continue))
            }
            Some(&TokenKind::Keyword(Keyword::Over)) => {
                *cursor += 1;
                let window_spec = WindowSpec::build(token_table, cursor)?;
                let function = match left {
                    Expr::FunctionCall(fc) => fc,
                    _ => return Err(ParserError::SyntaxError(*cursor, *cursor)),
                };
                Ok((
                    Expr::WindowFunction(Box::new(WindowFunction {
                        function,
                        window_spec,
                    })),
                    Flow::Continue,
                ))
            }
            _ => Ok((left, Flow::Run)),
        }
    }
}

impl<'a> Expr<'a> {
    pub(crate) fn class_between(
        is_not: bool,
        field: Box<Expr<'a>>,
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
    ) -> Result<Self, ParserError> {
        Between::build(is_not, field, token_table, cursor).map(Expr::Between)
    }

    pub(crate) fn class_in(
        is_not: bool,
        field: Box<Expr<'a>>,
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
    ) -> Result<Self, ParserError> {
        In::build(is_not, field, token_table, cursor).map(Expr::In)
    }

    pub(crate) fn class_case(
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
    ) -> Result<Self, ParserError> {
        CaseExpr::build(token_table, cursor).map(Expr::Case)
    }
}

#[derive(Debug, PartialEq)]
pub struct Field<'a> {
    pub prefix: Option<&'a str>,
    pub name: &'a str,
}

impl<'a> Field<'a> {
    pub(crate) fn from_token(
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
    ) -> Result<Self, ParserError> {
        let first = token_table
            .get_kind(*cursor)
            .map(|kind| kind == &TokenKind::Identifier)
            .unwrap_or(false);
        let dot = token_table
            .get_kind(*cursor + 1)
            .map(|kind| kind == &TokenKind::Dot)
            .unwrap_or(false);
        let second = token_table
            .get_kind(*cursor + 2)
            .map(|kind| kind == &TokenKind::Identifier)
            .unwrap_or(false);

        let (prefix, name, sum) = match (first, dot, second) {
            (true, false, _) => {
                let name = token_table.source_at(*cursor);
                (None, name, 1)
            }
            (true, true, true) => {
                let prefix = token_table.source_at(*cursor);
                let name = token_table.source_at(*cursor + 2);
                (Some(prefix), name, 3)
            }
            _ => return Err(ParserError::SyntaxError(*cursor, *cursor)),
        };

        *cursor += sum;

        Ok(Self { prefix, name })
    }
}

#[derive(Debug, PartialEq)]
pub struct Star<'a> {
    pub prefix: Option<&'a str>,
}

impl<'a> Star<'a> {
    pub(crate) fn from_token(
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
    ) -> Result<Self, ParserError> {
        let first = token_table
            .get_kind(*cursor)
            .map(|kind| kind == &TokenKind::Identifier)
            .unwrap_or(false);
        let first_star = token_table
            .get_kind(*cursor)
            .map(|kind| kind == &TokenKind::Multiply)
            .unwrap_or(false);
        let dot = token_table
            .get_kind(*cursor + 1)
            .map(|kind| kind == &TokenKind::Dot)
            .unwrap_or(false);
        let second = token_table
            .get_kind(*cursor + 2)
            .map(|kind| kind == &TokenKind::Multiply)
            .unwrap_or(false);

        if first_star {
            *cursor += 1;
            Ok(Self { prefix: None })
        } else if first && dot && second {
            let prefix = token_table.source_at(*cursor);
            *cursor += 3;
            Ok(Self {
                prefix: Some(prefix),
            })
        } else {
            Err(ParserError::SyntaxError(*cursor, *cursor))
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct FunctionCall<'a> {
    pub name: &'a str,
    pub args: MiniVec<Expr<'a>>,
    pub distinct: bool,
}

impl<'a> FunctionCall<'a> {
    pub(crate) fn from_token(
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
    ) -> Result<Self, ParserError> {
        let first = token_table
            .get_kind(*cursor)
            .map(|kind| kind == &TokenKind::Identifier)
            .unwrap_or(false);
        let second = token_table
            .get_kind(*cursor + 1)
            .map(|kind| kind == &TokenKind::LeftParen)
            .unwrap_or(false);

        if !(first && second) {
            return Err(ParserError::SyntaxError(*cursor, *cursor));
        }

        let name = token_table.source_at(*cursor);
        *cursor += 2;

        let distinct =
            if let Some(TokenKind::Keyword(Keyword::Distinct)) = token_table.get_kind(*cursor) {
                *cursor += 1;
                true
            } else {
                false
            };

        let mut args = MiniVec::with_capacity(8);
        let mut is_comma = false;
        loop {
            match token_table.get_kind(*cursor) {
                Some(TokenKind::Comma) => {
                    *cursor += 1;
                    is_comma = true;
                    continue;
                }
                Some(TokenKind::RightParen) => {
                    if is_comma {
                        return Err(ParserError::SyntaxError(*cursor, *cursor));
                    }
                    *cursor += 1;
                    break;
                }
                Some(_) => {
                    let expr = Expr::build(token_table, cursor)?;
                    args.push(expr);
                    is_comma = false;
                }
                _ => {
                    return Err(ParserError::SyntaxError(*cursor, *cursor));
                }
            }
        }

        Ok(Self {
            name,
            args,
            distinct,
        })
    }
}

#[derive(Debug, PartialEq)]
pub struct StringLiteral<'a> {
    pub value: &'a str,
}

impl<'a> StringLiteral<'a> {
    pub(crate) fn from_token(
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
    ) -> Result<Self, ParserError> {
        if let Some(TokenKind::StringLiteral) = token_table.get_kind(*cursor) {
            let value = token_table.source_at(*cursor);
            *cursor += 1;
            Ok(Self { value })
        } else {
            Err(ParserError::SyntaxError(*cursor, *cursor))
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct NumericLiteral<'a> {
    pub value: &'a str,
}

impl<'a> NumericLiteral<'a> {
    pub(crate) fn from_token(
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
    ) -> Result<Self, ParserError> {
        if let Some(TokenKind::Number) = token_table.get_kind(*cursor) {
            let value = token_table.source_at(*cursor);
            *cursor += 1;
            Ok(Self { value })
        } else {
            Err(ParserError::SyntaxError(*cursor, *cursor))
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Between<'a> {
    pub is_not: bool,
    pub field: Box<Expr<'a>>,
    pub lower: Box<Expr<'a>>,
    pub upper: Box<Expr<'a>>,
}

impl<'a> Between<'a> {
    pub(crate) fn build(
        is_not: bool,
        field: Box<Expr<'a>>,
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
    ) -> Result<Self, ParserError> {
        expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Between))?;
        *cursor += 1;

        let lower = Box::new(Expr::parse_primary(token_table, cursor)?);

        expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::And))?;
        *cursor += 1;

        let upper = Box::new(Expr::parse_primary(token_table, cursor)?);

        Ok(Self {
            is_not,
            field,
            lower,
            upper,
        })
    }
}

#[derive(Debug, PartialEq)]
pub struct In<'a> {
    pub is_not: bool,
    pub field: Box<Expr<'a>>,
    pub in_value: InValue<'a>,
}

#[derive(Debug, PartialEq)]
pub enum InValue<'a> {
    List(MiniVec<Expr<'a>>),
    Subquery(SubSelectStatement<'a>),
}

impl<'a> In<'a> {
    pub(crate) fn build(
        is_not: bool,
        field: Box<Expr<'a>>,
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
    ) -> Result<Self, ParserError> {
        expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::In))?;
        *cursor += 1;

        expect_kind(token_table, cursor, &TokenKind::LeftParen)?;
        *cursor += 1;

        let in_value = if maybe_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Select)) {
            InValue::Subquery(Box::new(SelectStatement::new(token_table, cursor)?))
        } else {
            let mut values = MiniVec::with_capacity(8);
            loop {
                match token_table.get_kind(*cursor) {
                    Some(TokenKind::Comma) => {
                        *cursor += 1;
                        continue;
                    }
                    Some(TokenKind::RightParen) => {
                        break;
                    }
                    Some(TokenKind::Keyword(_)) => {
                        break;
                    }
                    Some(_) => {
                        let value = Expr::parse_primary(token_table, cursor)?;
                        values.push(value);
                    }
                    _ => {
                        return Err(ParserError::SyntaxError(*cursor, *cursor));
                    }
                }
            }
            if values.is_empty() {
                return Err(ParserError::SyntaxError(*cursor, *cursor));
            }
            InValue::List(values)
        };

        expect_kind(token_table, cursor, &TokenKind::RightParen)?;
        *cursor += 1;

        Ok(Self {
            is_not,
            field,
            in_value,
        })
    }
}

#[derive(Debug, PartialEq)]
pub struct Like<'a> {
    pub is_not: bool,
    pub field: Box<Expr<'a>>,
    pub pattern: Box<Expr<'a>>,
}

impl<'a> Like<'a> {
    pub(crate) fn build(
        is_not: bool,
        field: Box<Expr<'a>>,
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
    ) -> Result<Self, ParserError> {
        expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Like))?;
        *cursor += 1;

        let pattern = Box::new(Expr::parse_primary(token_table, cursor)?);

        Ok(Self {
            is_not,
            field,
            pattern,
        })
    }
}

#[derive(Debug, PartialEq)]
pub struct IsNull<'a> {
    pub is_not: bool,
    pub field: Box<Expr<'a>>,
}

#[derive(Debug, PartialEq)]
pub struct ExistsExpr<'a> {
    pub is_not: bool,
    pub subquery: SubSelectStatement<'a>,
}

impl<'a> ExistsExpr<'a> {
    pub(crate) fn build(
        is_not: bool,
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
    ) -> Result<Self, ParserError> {
        expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Exists))?;
        *cursor += 1;
        expect_kind(token_table, cursor, &TokenKind::LeftParen)?;
        *cursor += 1;
        let select_stmt = SelectStatement::new(token_table, cursor)?;
        expect_kind(token_table, cursor, &TokenKind::RightParen)?;
        *cursor += 1;
        Ok(Self {
            is_not,
            subquery: Box::new(select_stmt),
        })
    }
}

#[derive(Debug, PartialEq)]
pub struct WindowSpec<'a> {
    pub partition_by: Option<MiniVec<Expr<'a>>>,
    pub order_by: Option<Order<'a>>,
}

impl<'a> WindowSpec<'a> {
    pub(crate) fn build(
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
    ) -> Result<Self, ParserError> {
        expect_kind(token_table, cursor, &TokenKind::LeftParen)?;
        *cursor += 1;

        let partition_by =
            if maybe_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Partition)) {
                *cursor += 1;
                expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::By))?;
                *cursor += 1;
                let mut cols = MiniVec::new();
                loop {
                    match token_table.get_kind(*cursor) {
                        Some(TokenKind::Comma) => {
                            *cursor += 1;
                        }
                        Some(TokenKind::Keyword(Keyword::Order)) | Some(TokenKind::RightParen) => {
                            break;
                        }
                        Some(TokenKind::Identifier)
                        | Some(TokenKind::Number)
                        | Some(TokenKind::StringLiteral)
                        | Some(TokenKind::Multiply) => {
                            cols.push(Expr::build(token_table, cursor)?);
                        }
                        _ => break,
                    }
                }
                Some(cols)
            } else {
                None
            };

        let order_by = if maybe_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Order)) {
            Some(Order::build(token_table, cursor)?)
        } else {
            None
        };

        expect_kind(token_table, cursor, &TokenKind::RightParen)?;
        *cursor += 1;
        Ok(Self {
            partition_by,
            order_by,
        })
    }
}

#[derive(Debug, PartialEq)]
pub struct WindowFunction<'a> {
    pub function: FunctionCall<'a>,
    pub window_spec: WindowSpec<'a>,
}

#[derive(Debug, PartialEq)]
pub struct CaseExpr<'a> {
    pub condition: Option<Box<Expr<'a>>>,
    pub when_clauses: MiniVec<WhenClause<'a>>,
    pub else_result: Option<Box<Expr<'a>>>,
}

#[derive(Debug, PartialEq)]
pub struct WhenClause<'a> {
    pub condition: Box<Expr<'a>>,
    pub result: Box<Expr<'a>>,
}

impl<'a> CaseExpr<'a> {
    pub(crate) fn build(
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
    ) -> Result<Self, ParserError> {
        expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Case))?;
        *cursor += 1;

        let condition = if !maybe_kind(token_table, cursor, &TokenKind::Keyword(Keyword::When)) {
            Some(Box::new(Expr::build(token_table, cursor)?))
        } else {
            None
        };

        let mut when_clauses = MiniVec::new();
        loop {
            if !maybe_kind(token_table, cursor, &TokenKind::Keyword(Keyword::When)) {
                break;
            }
            *cursor += 1;

            let cond = Expr::build(token_table, cursor)?;

            expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Then))?;
            *cursor += 1;

            let result = Expr::build(token_table, cursor)?;

            when_clauses.push(WhenClause {
                condition: Box::new(cond),
                result: Box::new(result),
            });
        }

        let else_result = if maybe_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Else)) {
            *cursor += 1;
            Some(Box::new(Expr::build(token_table, cursor)?))
        } else {
            None
        };

        expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::End))?;
        *cursor += 1;

        Ok(Self {
            condition,
            when_clauses,
            else_result,
        })
    }
}

#[cfg(test)]
mod test {

    use minivec::mini_vec;

    use crate::{
        ParserError,
        common::{
            alias::Alias,
            expr::{
                BinaryOp, BinaryOperator, Expr, Field, FunctionCall, IsNull, NumericLiteral, Star,
                StringLiteral,
            },
        },
        keyword::Keyword,
        token::{TokenKind, TokenTable},
    };

    fn make_table<'a>(source: &'a str, entries: Vec<(TokenKind, usize, usize)>) -> TokenTable<'a> {
        let mut table = TokenTable::with_source(source);
        for (kind, start, end) in entries {
            table.push(
                kind,
                unsafe { str::from_utf8_unchecked(&source.as_bytes()[start..=end])},
            );
        }
        table
    }

    #[test]
    fn test_column() {
        let source = "ab.cd";
        let token_table = make_table(
            source,
            vec![
                (TokenKind::Identifier, 0, 1),
                (TokenKind::Dot, 2, 2),
                (TokenKind::Identifier, 3, 4),
            ],
        );
        let mut cursor = 0;
        let expr = Expr::class_field(&token_table, &mut cursor).unwrap();
        assert_eq!(
            expr,
            Expr::Field(Field {
                prefix: Some("ab"),
                name: "cd",
            })
        );
        assert_eq!(cursor, 3);

        let source2 = "ab.cd as e";
        let token_table2 = make_table(
            source2,
            vec![
                (TokenKind::Identifier, 0, 1),
                (TokenKind::Dot, 2, 2),
                (TokenKind::Identifier, 3, 4),
                (TokenKind::Keyword(Keyword::As), 6, 7),
                (TokenKind::Identifier, 9, 9),
            ],
        );
        let mut cursor = 0;
        let alias = Alias::new(&token_table2, &mut cursor).unwrap();
        assert_eq!(
            alias,
            Alias {
                name: Some("e"),
                value: Expr::Field(Field {
                    prefix: Some("ab"),
                    name: "cd",
                })
            }
        );
        assert_eq!(cursor, 5);

        let source3 = "ab.cd e";
        let token_table3 = make_table(
            source3,
            vec![
                (TokenKind::Identifier, 0, 1),
                (TokenKind::Dot, 2, 2),
                (TokenKind::Identifier, 3, 4),
                (TokenKind::Identifier, 6, 6),
            ],
        );
        let mut cursor = 0;
        let alias2 = Alias::new(&token_table3, &mut cursor).unwrap();
        assert_eq!(
            alias2,
            Alias {
                name: Some("e"),
                value: Expr::Field(Field {
                    prefix: Some("ab"),
                    name: "cd",
                })
            }
        );
        assert_eq!(cursor, 4);
    }

    #[test]
    fn test_function() {
        let source = "foo('hello')";
        let token_table = make_table(
            source,
            vec![
                (TokenKind::Identifier, 0, 2),
                (TokenKind::LeftParen, 3, 3),
                (TokenKind::StringLiteral, 4, 10),
                (TokenKind::RightParen, 11, 11),
            ],
        );
        let mut cursor = 0;
        let expr = Expr::class_function_call(&token_table, &mut cursor).unwrap();
        assert_eq!(
            expr,
            Expr::FunctionCall(FunctionCall {
                distinct: false,
                name: "foo",
                args: mini_vec![Expr::StringLiteral(StringLiteral {
                    value: "'hello'",
                })]
            })
        );
        assert_eq!(cursor, 4);
    }

    #[test]
    fn test_function_1() {
        let source = "bar('x', 'y')";
        let token_table = make_table(
            source,
            vec![
                (TokenKind::Identifier, 0, 2),
                (TokenKind::LeftParen, 3, 3),
                (TokenKind::StringLiteral, 4, 6),
                (TokenKind::Comma, 7, 7),
                (TokenKind::StringLiteral, 9, 11),
                (TokenKind::RightParen, 12, 12),
            ],
        );
        let mut cursor = 0;
        let expr = Expr::class_function_call(&token_table, &mut cursor).unwrap();
        assert_eq!(
            expr,
            Expr::FunctionCall(FunctionCall {
                name: "bar",
                args: mini_vec![
                    Expr::StringLiteral(StringLiteral {
                        value: "'x'"
                    }),
                    Expr::StringLiteral(StringLiteral {
                        value: "'y'"
                    }),
                ],
                distinct: false,
            })
        );
        assert_eq!(cursor, 6);
    }

    #[test]
    fn test_function_should_panic_1() {
        let source = "f('x', )";
        let token_table = make_table(
            source,
            vec![
                (TokenKind::Identifier, 0, 0),
                (TokenKind::LeftParen, 1, 1),
                (TokenKind::StringLiteral, 2, 4),
                (TokenKind::Comma, 5, 5),
                (TokenKind::RightParen, 6, 6),
            ],
        );
        let mut cursor = 0;
        let expr = Expr::class_function_call(&token_table, &mut cursor);
        assert_eq!(expr, Err(ParserError::SyntaxError(4, 4)));
    }

    #[test]
    fn test_binary_operator_1() {
        let source = "1 + 2 * 3";
        let token_table = make_table(
            source,
            vec![
                (TokenKind::Number, 0, 0),
                (TokenKind::Plus, 2, 2),
                (TokenKind::Number, 4, 4),
                (TokenKind::Multiply, 6, 6),
                (TokenKind::Number, 8, 8),
            ],
        );
        let mut cursor = 0;
        let expr = Expr::build(&token_table, &mut cursor).unwrap();
        assert_eq!(
            expr,
            Expr::BinaryOp(Box::new(BinaryOp {
                op: BinaryOperator::Add,
                left: Expr::NumericLiteral(NumericLiteral {
                    value: "1"
                }),
                right: Expr::BinaryOp(Box::new(BinaryOp {
                    op: BinaryOperator::Multiply,
                    left: Expr::NumericLiteral(NumericLiteral {
                        value: "2"
                    }),
                    right: Expr::NumericLiteral(NumericLiteral {
                        value: "3"
                    }),
                })),
            }))
        );
        assert_eq!(cursor, 5);
    }

    #[test]
    fn test_build_binary_op_2() {
        let source = "1 * 2 + 3";
        let token_table = make_table(
            source,
            vec![
                (TokenKind::Number, 0, 0),
                (TokenKind::Multiply, 2, 2),
                (TokenKind::Number, 4, 4),
                (TokenKind::Plus, 6, 6),
                (TokenKind::Number, 8, 8),
            ],
        );
        let mut cursor = 0;
        let expr = Expr::build(&token_table, &mut cursor).unwrap();
        assert_eq!(
            expr,
            Expr::BinaryOp(Box::new(BinaryOp {
                op: BinaryOperator::Add,
                left: Expr::BinaryOp(Box::new(BinaryOp {
                    op: BinaryOperator::Multiply,
                    left: Expr::NumericLiteral(NumericLiteral {
                        value: "1"
                    }),
                    right: Expr::NumericLiteral(NumericLiteral {
                        value: "2"
                    }),
                })),
                right: Expr::NumericLiteral(NumericLiteral {
                    value: "3"
                }),
            }))
        );
        assert_eq!(cursor, 5);
    }

    #[test]
    fn test_build_binary_op_3() {
        let source = "(1 + 2) * 3";
        let token_table = make_table(
            source,
            vec![
                (TokenKind::LeftParen, 0, 0),
                (TokenKind::Number, 1, 1),
                (TokenKind::Plus, 3, 3),
                (TokenKind::Number, 5, 5),
                (TokenKind::RightParen, 6, 6),
                (TokenKind::Multiply, 8, 8),
                (TokenKind::Number, 10, 10),
            ],
        );
        let mut cursor = 0;
        let expr = Expr::build(&token_table, &mut cursor).unwrap();
        assert_eq!(
            expr,
            Expr::BinaryOp(Box::new(BinaryOp {
                op: BinaryOperator::Multiply,
                left: Expr::BinaryOp(Box::new(BinaryOp {
                    op: BinaryOperator::Add,
                    left: Expr::NumericLiteral(NumericLiteral {
                        value: "1"
                    }),
                    right: Expr::NumericLiteral(NumericLiteral {
                        value: "2"
                    }),
                })),
                right: Expr::NumericLiteral(NumericLiteral {
                    value: "3"
                }),
            }))
        );
        assert_eq!(cursor, 7);
    }

    #[test]
    fn test_binary_operator_all_arithmetic() {
        let source = "1 / 2";
        let token_table = make_table(
            source,
            vec![
                (TokenKind::Number, 0, 0),
                (TokenKind::Divide, 2, 2),
                (TokenKind::Number, 4, 4),
            ],
        );
        let mut cursor = 0;
        let expr = Expr::build(&token_table, &mut cursor).unwrap();
        assert_eq!(
            expr,
            Expr::BinaryOp(Box::new(BinaryOp {
                op: BinaryOperator::Divide,
                left: Expr::NumericLiteral(NumericLiteral {
                    value: "1"
                }),
                right: Expr::NumericLiteral(NumericLiteral {
                    value: "2"
                }),
            }))
        );
        assert_eq!(cursor, 3);

        let source2 = "1 % 2";
        let token_table2 = make_table(
            source2,
            vec![
                (TokenKind::Number, 0, 0),
                (TokenKind::Mod, 2, 2),
                (TokenKind::Number, 4, 4),
            ],
        );
        let mut cursor = 0;
        let expr = Expr::build(&token_table2, &mut cursor).unwrap();
        assert_eq!(
            expr,
            Expr::BinaryOp(Box::new(BinaryOp {
                op: BinaryOperator::Mod,
                left: Expr::NumericLiteral(NumericLiteral {
                    value: "1"
                }),
                right: Expr::NumericLiteral(NumericLiteral {
                    value: "2"
                }),
            }))
        );
    }

    #[test]
    fn test_binary_operator_comparison() {
        let source_eq = "id = 1";
        let token_table = make_table(
            source_eq,
            vec![
                (TokenKind::Identifier, 0, 1),
                (TokenKind::Equal, 3, 3),
                (TokenKind::Number, 5, 5),
            ],
        );
        let mut cursor = 0;
        let expr = Expr::build(&token_table, &mut cursor).unwrap();
        assert_eq!(
            expr,
            Expr::BinaryOp(Box::new(BinaryOp {
                op: BinaryOperator::Equal,
                left: Expr::Field(Field {
                    prefix: None,
                    name: "id"
                }),
                right: Expr::NumericLiteral(NumericLiteral {
                    value: "1"
                }),
            }))
        );
        assert_eq!(cursor, 3);

        let source_ne = "id <> 'x'";
        let token_table2 = make_table(
            source_ne,
            vec![
                (TokenKind::Identifier, 0, 1),
                (TokenKind::NotEqual, 3, 4),
                (TokenKind::StringLiteral, 6, 8),
            ],
        );
        let mut cursor = 0;
        let expr = Expr::build(&token_table2, &mut cursor).unwrap();
        assert_eq!(
            expr,
            Expr::BinaryOp(Box::new(BinaryOp {
                op: BinaryOperator::NotEqual,
                left: Expr::Field(Field {
                    prefix: None,
                    name: "id"
                }),
                right: Expr::StringLiteral(StringLiteral {
                    value: "'x'"
                }),
            }))
        );
    }

    #[test]
    fn test_binary_operator_logical() {
        let source = "a AND b OR c";
        let token_table = make_table(
            source,
            vec![
                (TokenKind::Identifier, 0, 0),
                (TokenKind::Keyword(Keyword::And), 2, 4),
                (TokenKind::Identifier, 6, 6),
                (TokenKind::Keyword(Keyword::Or), 8, 9),
                (TokenKind::Identifier, 11, 11),
            ],
        );
        let mut cursor = 0;
        let expr = Expr::build(&token_table, &mut cursor).unwrap();
        assert_eq!(
            expr,
            Expr::BinaryOp(Box::new(BinaryOp {
                op: BinaryOperator::Or,
                left: Expr::BinaryOp(Box::new(BinaryOp {
                    op: BinaryOperator::And,
                    left: Expr::Field(Field {
                        prefix: None,
                        name: "a"
                    }),
                    right: Expr::Field(Field {
                        prefix: None,
                        name: "b"
                    }),
                })),
                right: Expr::Field(Field {
                    prefix: None,
                    name: "c"
                }),
            }))
        );
        assert_eq!(cursor, 5);
    }

    #[test]
    fn test_string_literal() {
        let source = "'hello'";
        let token_table = make_table(source, vec![(TokenKind::StringLiteral, 0, 6)]);
        let mut cursor = 0;
        let expr = Expr::class_string_literal(&token_table, &mut cursor).unwrap();
        assert_eq!(
            expr,
            Expr::StringLiteral(StringLiteral {
                value: "'hello'"
            })
        );
        assert_eq!(cursor, 1);
    }

    #[test]
    fn test_number_literal() {
        let source = "12345";
        let token_table = make_table(source, vec![(TokenKind::Number, 0, 4)]);
        let mut cursor = 0;
        let expr = Expr::class_number_literal(&token_table, &mut cursor).unwrap();
        assert_eq!(
            expr,
            Expr::NumericLiteral(NumericLiteral {
                value: "12345"
            })
        );
        assert_eq!(cursor, 1);
    }

    #[test]
    fn test_star() {
        let source = "*";
        let token_table = make_table(source, vec![(TokenKind::Multiply, 0, 0)]);
        let mut cursor = 0;
        let expr = Expr::class_star(&token_table, &mut cursor).unwrap();
        assert_eq!(expr, Expr::Star(Star { prefix: None }));
        assert_eq!(cursor, 1);

        let source2 = "t.*";
        let token_table2 = make_table(
            source2,
            vec![
                (TokenKind::Identifier, 0, 0),
                (TokenKind::Dot, 1, 1),
                (TokenKind::Multiply, 2, 2),
            ],
        );
        let mut cursor = 0;
        let expr = Expr::class_star(&token_table2, &mut cursor).unwrap();
        assert_eq!(
            expr,
            Expr::Star(Star {
                prefix: Some("t")
            })
        );
        assert_eq!(cursor, 3);
    }

    #[test]
    fn test_field_simple() {
        let source = "col1";
        let token_table = make_table(source, vec![(TokenKind::Identifier, 0, 3)]);
        let mut cursor = 0;
        let expr = Expr::class_field(&token_table, &mut cursor).unwrap();
        assert_eq!(
            expr,
            Expr::Field(Field {
                prefix: None,
                name: "col1"
            })
        );
        assert_eq!(cursor, 1);
    }

    #[test]
    fn test_field_with_prefix() {
        let source = "usr.id";
        let token_table = make_table(
            source,
            vec![
                (TokenKind::Identifier, 0, 2),
                (TokenKind::Dot, 3, 3),
                (TokenKind::Identifier, 4, 5),
            ],
        );
        let mut cursor = 0;
        let expr = Expr::class_field(&token_table, &mut cursor).unwrap();
        assert_eq!(
            expr,
            Expr::Field(Field {
                prefix: Some("usr"),
                name: "id"
            })
        );
        assert_eq!(cursor, 3);
    }

    #[test]
    fn test_field_invalid() {
        let source = "5 x";
        let token_table = make_table(
            source,
            vec![(TokenKind::Number, 0, 0), (TokenKind::Identifier, 2, 2)],
        );
        let mut cursor = 0;
        let result = Expr::class_field(&token_table, &mut cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_function_distinct() {
        let source = "c(DISTINCT 3)";
        let token_table = make_table(
            source,
            vec![
                (TokenKind::Identifier, 0, 0),
                (TokenKind::LeftParen, 1, 1),
                (TokenKind::Keyword(Keyword::Distinct), 2, 2),
                (TokenKind::Number, 11, 11),
                (TokenKind::RightParen, 12, 12),
            ],
        );
        let mut cursor = 0;
        let expr = Expr::class_function_call(&token_table, &mut cursor).unwrap();
        assert_eq!(
            expr,
            Expr::FunctionCall(FunctionCall {
                distinct: true,
                name: "c",
                args: mini_vec![Expr::NumericLiteral(NumericLiteral {
                    value: "3"
                })]
            })
        );
    }
}
