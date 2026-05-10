use aho_corasick::{AhoCorasick, AhoCorasickBuilder, BuildError, MatchKind};
use strum::{Display, VariantArray};

#[derive(
    Debug, PartialEq, Eq, Hash, Clone, Copy, Display, strum::VariantArray, strum::AsRefStr,
)]
#[repr(u16)]
pub enum Keyword {
    Select,
    From,
    Where,
    Insert,
    Into,
    Values,
    Update,
    Set,
    Delete,
    Create,
    Table,
    Drop,
    Alter,
    Add,
    Join,
    On,
    As,
    And,
    Asc,
    Desc,
    Or,
    Not,
    Null,
    Is,
    In,
    Like,
    Order,
    By,
    Group,
    Having,
    Limit,
    Left,
    Right,
    Inner,
    Offset,
    Distinct,
    Union,
    All,
    Exists,
    Between,
    Case,
    When,
    Then,
    Else,
    End,
    Full,
    Outer,
    Cross,
    Intersect,
    Except,
    True,
    False,
    First,
    Last,
    Nulls,
    With,
    Recursive,
    Over,
    Partition,
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
