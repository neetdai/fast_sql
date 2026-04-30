//! 通用的 Pratt Parser 实现
//! 
//! 该模块提供了一个灵活的 Pratt Parser 框架，可以用于解析具有运算符优先级的表达式。
//! 
//! # 示例
//! 
//! ```ignore
//! use crate::common::pratt::{PrattParser, InfixOp, PrefixOp};
//! 
//! // 定义表达式类型
//! enum Expr {
//!     Literal(i32),
//!     BinaryOp { op: BinaryOperator, left: Box<Expr>, right: Box<Expr> },
//!     UnaryOp { op: UnaryOperator, operand: Box<Expr> },
//! }
//! 
//! // 实现必要 trait
//! impl InfixOp for BinaryOperator {
//!     fn precedence(&self) -> u8 {
//!         match self {
//!             BinaryOperator::Add => 1,
//!             BinaryOperator::Multiply => 2,
//!         }
//!     }
//! 
//!     fn is_left_associative(&self) -> bool {
//!         true
//!     }
//! }
//! 
//! impl PrefixOp for UnaryOperator {
//!     fn precedence(&self) -> u8 {
//!         3
//!     }
//! }
//! 
//! // 使用 Pratt Parser
//! let parser = PrattParser::new();
//! let expr = parser.parse_expression(&token_table, &mut cursor)?;
//! ```

use crate::{ParserError, token::{TokenKind, TokenTable}};

/// 中缀运算符 trait
/// 
/// 实现此 trait 的类型可以作为 Pratt Parser 中的中缀运算符使用。
pub trait InfixOp {
    /// 获取运算符的优先级
    /// 
    /// 数值越大，优先级越高。例如，乘法通常比加法优先级高。
    fn precedence(&self) -> u8;

    /// 判断运算符是否是左结合的
    /// 
    /// 大多数二元运算符（如 +、-、*、/）都是左结合的。
    /// 右结合的运算符例子包括赋值运算符和指数运算符。
    fn is_left_associative(&self) -> bool;
}

/// 前缀运算符 trait
/// 
/// 实现此 trait 的类型可以作为 Pratt Parser 中的前缀运算符使用。
pub trait PrefixOp {
    /// 获取运算符的优先级
    /// 
    /// 前缀运算符的优先级通常高于中缀运算符。
    fn precedence(&self) -> u8;
}

/// 运算符解析器 trait
/// 
/// 定义如何从 token 流中解析运算符。
pub trait OpParser<Op> {
    /// 尝试从当前 token 解析运算符
    /// 
    /// 如果当前 token 不是该运算符，返回 None。
    fn try_parse_op(token_table: &TokenTable, cursor: &usize) -> Option<Op>;
}

/// 表达式解析器 trait
/// 
/// 定义如何从 token 流中解析表达式。
pub trait ExprParser<Expr> {
    /// 解析原子表达式（最基础的表达式）
    /// 
    /// 原子表达式包括：
    /// - 字面量（数字、字符串等）
    /// - 标识符
    /// - 括号表达式
    /// - 其他不需要运算符的基本表达式
    fn parse_primary(token_table: &TokenTable, cursor: &mut usize) -> Result<Expr, ParserError>;
}

/// Pratt Parser 结构体
/// 
/// 通用的 Pratt Parser 实现，可以用于解析具有运算符优先级的表达式。
/// 
/// # 类型参数
/// 
/// - `Expr`: 表达式类型
/// - `Infix`: 中缀运算符类型
/// - `Prefix`: 前缀运算符类型
pub struct PrattParser<Expr, Infix, Prefix> {
    _phantom: std::marker::PhantomData<(Expr, Infix, Prefix)>,
}

impl<Expr, Infix, Prefix> Default for PrattParser<Expr, Infix, Prefix> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Expr, Infix, Prefix> PrattParser<Expr, Infix, Prefix> {
    /// 创建新的 Pratt Parser 实例
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<Expr, Infix, Prefix> PrattParser<Expr, Infix, Prefix>
where
    Infix: InfixOp + Clone,
    Prefix: PrefixOp + Clone,
    Expr: Clone,
{
    /// 解析表达式
    /// 
    /// 使用默认的最小优先级（0）开始解析表达式。
    pub fn parse_expression(
        &self,
        token_table: &TokenTable,
        cursor: &mut usize,
        parse_primary: impl Fn(&TokenTable, &mut usize) -> Result<Expr, ParserError>,
        try_parse_infix: impl Fn(&TokenTable, &usize) -> Option<Infix>,
        try_parse_prefix: impl Fn(&TokenTable, &usize) -> Option<Prefix>,
        build_infix: impl Fn(Infix, Expr, Expr) -> Expr,
        build_prefix: impl Fn(Prefix, Expr) -> Expr,
    ) -> Result<Expr, ParserError> {
        self.parse_expression_with_min_precedence(
            token_table,
            cursor,
            0,
            &parse_primary,
            &try_parse_infix,
            &try_parse_prefix,
            &build_infix,
            &build_prefix,
        )
    }

    /// 解析表达式，支持指定最小优先级
    /// 
    /// # 参数
    /// 
    /// - `token_table`: token 表
    /// - `cursor`: 当前 token 位置
    /// - `min_precedence`: 最小优先级，低于此优先级的运算符将停止解析
    /// - `parse_primary`: 解析原子表达式的函数
    /// - `try_parse_infix`: 尝试解析中缀运算符的函数
    /// - `try_parse_prefix`: 尝试解析前缀运算符的函数
    /// - `build_infix`: 构建中缀运算表达式的函数
    /// - `build_prefix`: 构建前缀运算表达式的函数
    fn parse_expression_with_min_precedence(
        &self,
        token_table: &TokenTable,
        cursor: &mut usize,
        min_precedence: u8,
        parse_primary: &impl Fn(&TokenTable, &mut usize) -> Result<Expr, ParserError>,
        try_parse_infix: &impl Fn(&TokenTable, &usize) -> Option<Infix>,
        try_parse_prefix: &impl Fn(&TokenTable, &usize) -> Option<Prefix>,
        build_infix: &impl Fn(Infix, Expr, Expr) -> Expr,
        build_prefix: &impl Fn(Prefix, Expr) -> Expr,
    ) -> Result<Expr, ParserError> {
        // 解析左侧表达式
        let mut left = self.parse_prefix_or_primary(
            token_table,
            cursor,
            parse_primary,
            try_parse_prefix,
            build_prefix,
        )?;

        // 循环处理中缀运算符
        loop {
            // 检查当前 token 是否是中缀运算符
            let op = match try_parse_infix(token_table, cursor) {
                Some(op) => op,
                None => break,
            };

            // 如果运算符优先级低于最小优先级，停止解析
            if op.precedence() < min_precedence {
                break;
            }

            // 消耗运算符 token
            *cursor += 1;

            // 计算下一个表达式的最小优先级
            let next_min_precedence = if op.is_left_associative() {
                op.precedence() + 1
            } else {
                op.precedence()
            };

            // 递归解析右侧表达式
            let right = self.parse_expression_with_min_precedence(
                token_table,
                cursor,
                next_min_precedence,
                parse_primary,
                try_parse_infix,
                try_parse_prefix,
                build_infix,
                build_prefix,
            )?;

            // 构建中缀运算表达式
            left = build_infix(op, left, right);
        }

        Ok(left)
    }

    /// 解析前缀运算符或原子表达式
    fn parse_prefix_or_primary(
        &self,
        token_table: &TokenTable,
        cursor: &mut usize,
        parse_primary: &impl Fn(&TokenTable, &mut usize) -> Result<Expr, ParserError>,
        try_parse_prefix: &impl Fn(&TokenTable, &usize) -> Option<Prefix>,
        build_prefix: &impl Fn(Prefix, Expr) -> Expr,
    ) -> Result<Expr, ParserError> {
        // 尝试解析前缀运算符
        if let Some(op) = try_parse_prefix(token_table, cursor) {
            *cursor += 1;
            let operand = self.parse_prefix_or_primary(
                token_table,
                cursor,
                parse_primary,
                try_parse_prefix,
                build_prefix,
            )?;
            return Ok(build_prefix(op, operand));
        }

        // 解析原子表达式
        parse_primary(token_table, cursor)
    }
}
