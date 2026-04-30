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

// impl std::str::FromStr for Keyword {
//     type Err = ();

//     fn from_str(s: &str) -> Result<Self, Self::Err> {
//         match s.to_ascii_uppercase().as_str() {
//             "SELECT" => Ok(Keyword::Select),
//             "FROM" => Ok(Keyword::From),
//             "WHERE" => Ok(Keyword::Where),
//             "INSERT" => Ok(Keyword::Insert),
//             "INTO" => Ok(Keyword::Into),
//             "VALUES" => Ok(Keyword::Values),
//             "UPDATE" => Ok(Keyword::Update),
//             "SET" => Ok(Keyword::Set),
//             "DELETE" => Ok(Keyword::Delete),
//             "CREATE" => Ok(Keyword::Create),
//             "TABLE" => Ok(Keyword::Table),
//             "DROP" => Ok(Keyword::Drop),
//             "ALTER" => Ok(Keyword::Alter),
//             "ADD" => Ok(Keyword::Add),
//             "JOIN" => Ok(Keyword::Join),
//             "ON" => Ok(Keyword::On),
//             "AS" => Ok(Keyword::As),
//             "AND" => Ok(Keyword::And),
//             "OR" => Ok(Keyword::Or),
//             "NOT" => Ok(Keyword::Not),
//             "NULL" => Ok(Keyword::Null),
//             "IS" => Ok(Keyword::Is),
//             "IN" => Ok(Keyword::In),
//             "LIKE" => Ok(Keyword::Like),
//             "LEFT" => Ok(Keyword::Left),
//             "ORDER" => Ok(Keyword::Order),
//             "BY" => Ok(Keyword::By),
//             "GROUP" => Ok(Keyword::Group),
//             "HAVING" => Ok(Keyword::Having),
//             "LIMIT" => Ok(Keyword::Limit),
//             "OFFSET" => Ok(Keyword::Offset),
//             "DISTINCT" => Ok(Keyword::Distinct),
//             "UNION" => Ok(Keyword::Union),
//             "ALL" => Ok(Keyword::All),
//             "EXISTS" => Ok(Keyword::Exists),
//             "BETWEEN" => Ok(Keyword::Between),
//             "CASE" => Ok(Keyword::Case),
//             "WHEN" => Ok(Keyword::When),
//             "THEN" => Ok(Keyword::Then),
//             "ELSE" => Ok(Keyword::Else),
//             "END" => Ok(Keyword::End),
//             "FULL" => Ok(Keyword::Full),
//             "OUTER" => Ok(Keyword::Outer),
//             "CROSS" => Ok(Keyword::Cross),
//             "INNER" => Ok(Keyword::Inner),
//             _ => Err(()),
//         }
//     }
// }

#[derive(Debug)]
pub(crate) struct KeywordMap {
    // inner: [MiniVec<Keyword>; 8]
    inner: AhoCorasick,
}

impl KeywordMap {
    // pub fn new() -> Self {
    //     let inner = [
    //         mini_vec![],
    //         mini_vec![],
    //         mini_vec![
    //             Keyword::As,
    //             Keyword::By,
    //             Keyword::In,
    //             Keyword::Is,
    //             Keyword::Or,
    //             Keyword::On,
    //         ],
    //         mini_vec![
    //             Keyword::Add,
    //             Keyword::Not,
    //             Keyword::End,
    //             Keyword::All,
    //             Keyword::Set,
    //             Keyword::And,
    //             Keyword::End,
    //             Keyword::Asc,
    //         ],
    //         mini_vec![
    //             Keyword::Join,
    //             Keyword::Like,
    //             Keyword::Null,
    //             Keyword::Drop,
    //             Keyword::From,
    //             Keyword::Into,
    //             Keyword::Full,
    //             Keyword::Case,
    //             Keyword::Then,
    //             Keyword::Else,
    //             Keyword::When,
    //             Keyword::Desc,
    //         ],
    //         mini_vec![
    //             Keyword::Alter,
    //             Keyword::Create,
    //             Keyword::Group,
    //             Keyword::Order,
    //             Keyword::Table,
    //             Keyword::Union,
    //             Keyword::Where,
    //             Keyword::Outer,
    //             Keyword::Limit,
    //         ],
    //         mini_vec![
    //             Keyword::Select,
    //             Keyword::Delete,
    //             Keyword::Update,
    //             Keyword::Values,
    //             Keyword::Exists,
    //             Keyword::Having,
    //             Keyword::Offset,
    //         ],
    //         mini_vec![Keyword::Between, Keyword::Case, Keyword::Distinct,],
    //     ];
    //     Self { inner }
    // }
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
