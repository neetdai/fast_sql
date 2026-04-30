use std::fmt::Debug;

use crate::{ParserError, token::TokenTable};

pub(crate) trait PrattOutput<I>
where
    I: PrecedenceTrait,
{
    fn apply(op: I, left: Self, right: Self) -> Self;
}

pub(crate) trait PrecedenceTrait {
    /// 获取运算符的优先级，数值越大优先级越高
    fn precedence(&self) -> usize;

    /// 判断是否是左结合的运算符
    fn is_left_associative(&self) -> bool;

    fn min_precedence() -> usize;
}

pub(crate) trait PrattParserTrait {
    type Item: PrecedenceTrait;
    type Output: PrattOutput<Self::Item> + Debug;

    fn parse_primary(
        token_table: &TokenTable,
        cursor: &mut usize,
    ) -> Result<Self::Output, ParserError>;

    fn match_item(token_table: &TokenTable, cursor: &mut usize) -> Option<Self::Item>;

    fn parse_postfix(
        _left: Self::Output,
        _token_table: &TokenTable,
        cursor: &mut usize,
    ) -> Result<(Self::Output, Flow), ParserError> {
        Err(ParserError::SyntaxError(*cursor, *cursor))
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum Flow {
    Continue,
    Run,
    Break,
}

#[derive(Debug)]
pub(crate) struct PrattParser;

impl PrattParser {
    /// 使用 Pratt Parser 解析表达式
    pub(crate) fn parse_expression<P>(
        token_table: &TokenTable,
        cursor: &mut usize,
    ) -> Result<P::Output, ParserError>
    where
        P: PrattParserTrait + Debug,
    {
        Self::parse_expression_with_min_precedence::<P>(
            token_table,
            cursor,
            P::Item::min_precedence(),
        )
    }

    // /// 使用 Pratt Parser 解析表达式，支持指定最小优先级
    // fn parse_expression_with_min_precedence<P>(
    //     token_table: &TokenTable,
    //     cursor: &mut usize,
    //     min_precedence: usize,
    // ) -> Result<P::Output, ParserError>
    // where
    //     P: PrattParserTrait  + Debug,
    // {
    //     // 解析左侧表达式（原子表达式）
    //     let left = P::parse_primary(token_table, cursor)?;

    //     let mut left_tmp = Some(left);
    //     // 循环处理二元运算符
    //     loop {
    //         // dbg!(&cursor);
    //         if let Some(left) = left_tmp.take() {
    //             let (left, flow) = P::parse_postfix(left, token_table, cursor)?;
    //             left_tmp = Some(left);
    //             match flow {
    //                 Flow::Continue => continue,
    //                 Flow::Run => {},
    //                 Flow::Break => break,
    //             }
    //             // dbg!(&cursor);

    //             // 检查当前 token 是否是二元运算符
    //             let op = match token_table.get_kind(*cursor).and_then(P::match_item) {
    //                 Some(op) => op,
    //                 None => break, // 如果不是二元运算符，则退出循环
    //             };

    //             if op.precedence() < min_precedence {
    //                 break; // 如果运算符优先级低于最小优先级，则退出循环
    //             }

    //             *cursor += 1; // 消耗运算符 token
    //             if token_table.get_kind(*cursor).is_none() {
    //                 break;
    //             }
    //             // dbg!(&cursor);

    //             let next_min_precedence = if op.is_left_associative() {
    //                 op.precedence() + 1
    //             } else {
    //                 op.precedence()
    //             };

    //             let right = Self::parse_expression_with_min_precedence::<P>(
    //                 token_table,
    //                 cursor,
    //                 next_min_precedence,
    //             )?;

    //             if let Some(left) = left_tmp.take() {
    //                 let left = P::Output::apply(op, left, right);
    //                 left_tmp = Some(left);
    //             } else {
    //                 break;
    //             }
    //         } else {
    //             break;
    //         }
    //     }

    //     // dbg!(&left_tmp);
    //     if let Some(left) = left_tmp {
    //         Ok(left)
    //     } else {
    //         Err(ParserError::SyntaxError(*cursor, *cursor))
    //     }
    // }

    fn parse_expression_with_min_precedence<P>(
        token_table: &TokenTable,
        cursor: &mut usize,
        _initial_min_precedence: usize, // 迭代版通常从全局最小优先级开始
    ) -> Result<P::Output, ParserError>
    where
        P: PrattParserTrait + Debug,
    {
        // 1. 解析第一个原子表达式
        let mut current_left = P::parse_primary(token_table, cursor)?;

        // 2. 准备一个栈来存储 (运算符, 左操作数)
        // 栈里的运算符优先级是严格递增的
        let mut stack: Vec<(P::Item, P::Output)> = Vec::new();

        loop {
            // 处理后缀运算符（如函数调用 () 或 成员访问 .）
            let (new_left, flow) = P::parse_postfix(current_left, token_table, cursor)?;
            current_left = new_left;
            // dbg!(&flow);
            match flow {
                Flow::Continue => continue,
                Flow::Run => {}
                Flow::Break => break,
            }
            // // 3. 尝试匹配当前的二元运算符
            // let op = match token_table.get_kind(*cursor).and_then(P::match_item) {
            //     Some(op) => op,
            //     None => break, // 没有更多运算符，准备结束
            // };
            let op = match P::match_item(token_table, cursor) {
                Some(op) => op,
                None => break, // 没有更多运算符，准备结束
            };

            let op_precedence = op.precedence();

            // 4. 优先级判定与归约 (Reduce)
            // 如果栈顶运算符的优先级 >= 当前运算符的优先级，则进行结合
            while let Some((stack_op, _)) = stack.last() {
                let stack_prec = stack_op.precedence();

                // 如果是左结合：栈顶优先级 >= 当前优先级，则归约
                // 如果是右结合：栈顶优先级 >  当前优先级，则归约
                let should_reduce = if op.is_left_associative() {
                    stack_prec >= op_precedence
                } else {
                    stack_prec > op_precedence
                };

                if should_reduce {
                    let (top_op, top_left) = stack.pop().unwrap();
                    current_left = P::Output::apply(top_op, top_left, current_left);
                } else {
                    break;
                }
            }

            // 5. 将当前运算符和当前的“左侧结果”入栈，继续寻找右侧
            stack.push((op, current_left));
            // *cursor += 1; // 消耗运算符

            // 6. 解析运算符右侧的原子表达式，作为下一次循环的新 current_left
            current_left = P::parse_primary(token_table, cursor)?;
        }

        // 7. 循环结束后，处理栈中剩余的所有运算符（从后往前归约）
        while let Some((op, left)) = stack.pop() {
            current_left = P::Output::apply(op, left, current_left);
        }

        Ok(current_left)
    }
}
