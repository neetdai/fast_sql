mod common;
mod func;
mod sse;

pub(crate) use common::{SimdTrait, is_escaped};
pub(crate) use func::{
    find_consecutive_in_range, longest_consecutive_matching, mixed_match, skip_until_match,
    skip_until_sequence,
};
pub(crate) use sse::Sse;
