use crate::{Statement, error::ParserError, keyword::KeywordMap, lexer::Lexer};
use simdutf8::basic::from_utf8;

#[derive(Debug)]
pub struct Parser {
    keyword_map: KeywordMap,
}

impl Parser {
    pub fn new() -> Result<Self, ParserError> {
        Ok(Self {
            keyword_map: KeywordMap::new()
                .map_err(|err| ParserError::AhoCorasickBuild(err.to_string()))?,
        })
    }

    pub fn parse(&self, text: &str) -> Result<Statement, ParserError> {
        let text = from_utf8(text.as_bytes())?;
        let tokentable = {
            let mut lexer = Lexer::new(text, &self.keyword_map)?;
            lexer.tokenize()?
        };
        let mut cursor = 0;
        // let select = SelectStatement::new(&tokentable, &mut cursor)?;
        let statement = Statement::new(&tokentable, &mut cursor)?;
        // dbg!(&select);
        // dbg!(&statement);
        // dbg!(&select.where_statement);
        Ok(statement)
    }
}
