pub mod ast;
pub mod common;
mod error;
mod keyword;
mod lexer;
pub mod parser;
mod simd_common;
mod token;
// mod common_tmp;

pub use {error::ParserError, parser::Parser};
// pub use common_tmp::Expr;
pub use ast::{
    insert::InsertStatement, query::Query, select::SelectStatement, statement::Statement,
};

pub(crate) use simd_common::{
    find_consecutive_in_range, longest_consecutive_matching, skip_until_match, skip_until_sequence,
};
