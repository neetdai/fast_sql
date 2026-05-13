use aho_corasick::{AhoCorasick, AhoCorasickBuilder, BuildError, MatchKind};
use strum::{Display, VariantArray};

#[derive(
    Debug, PartialEq, Eq, Hash, Clone, Copy, Display, strum::VariantArray, strum::AsRefStr,
)]
#[repr(u16)]
pub enum Keyword {
    Add,
    All,
    Alter,
    And,
    Asc,
    As,
    Between,
    By,
    Cascade,
    Case,
    Check,
    Column,
    Constraint,
    Create,
    Cross,
    Default,
    Delete,
    Desc,
    Distinct,
    Drop,
    Else,
    End,
    Except,
    Exists,
    False,
    First,
    From,
    Full,
    Group,
    Having,
    If,
    In,
    Inner,
    Insert,
    Intersect,
    Into,
    Is,
    Join,
    Key,
    Last,
    Left,
    Like,
    Limit,
    Not,
    Null,
    Nulls,
    Natural,
    Offset,
    On,
    Or,
    Order,
    Outer,
    Over,
    Partition,
    Primary,
    Recursive,
    References,
    Rename,
    Restrict,
    Right,
    Schema,
    Select,
    Set,
    Table,
    Then,
    To,
    True,
    Union,
    Unique,
    Update,
    Using,
    Values,
    When,
    Where,
    With,
}

#[derive(Debug)]
pub(crate) struct KeywordMap {
    // inner: [MiniVec<Keyword>; 8]
    inner: AhoCorasick,
}

impl KeywordMap {

    pub fn new() -> Result<Self, BuildError> {
        let inner = AhoCorasickBuilder::new()
            .ascii_case_insensitive(true)
            .match_kind(MatchKind::LeftmostLongest)
            .build(Keyword::VARIANTS.iter().map(|v| v.as_ref().to_string()))?;
        Ok(Self { inner })
    }

    // pub fn get(&self, len: usize) -> Option<&MiniVec<Keyword>> {
    //     self.inner.get(len)
    // }
    pub fn match_keyword(&self, source: &str) -> Option<Keyword> {
        self.inner.find(source).and_then(|m| {
            let match_keyword = Keyword::VARIANTS[m.pattern()];
            if match_keyword.as_ref().len() == source.len() {
                Some(match_keyword)
            } else {
                None
            }
        })
    }
}
