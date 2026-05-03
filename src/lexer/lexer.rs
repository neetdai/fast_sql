use crate::{
    error::ParserError,
    keyword::{Keyword, KeywordMap},
    simd_common::{
        find_consecutive_in_range, is_escaped, longest_consecutive_matching, mixed_match,
        skip_until_match, skip_until_sequence,
    },
    token::{TokenKind, TokenTable},
};
use std::string::String;

// ============================================================================
// 1. 静态查找表 (Lookup Table) - 消除分支预测失败
// ============================================================================

// 字符分类标志位
const C_UNK: u8 = 0; // 未知/非法
const C_WSP: u8 = 1 << 0; // 空白 (Whitespace)
const C_DIG: u8 = 1 << 1; // 数字 (0-9)
const C_ALP: u8 = 1 << 2; // 字母 (a-z, A-Z, _)
const C_QUO: u8 = 1 << 3; // 引号 (' " `)
const C_SYM: u8 = 1 << 4; // 符号 (+ - * / etc)

// 256字节的映射表，将 byte 映射到分类
const fn char_table() -> [u8; 256] {
    let mut t = [C_UNK; 256];

    // 设置空白
    t[b' ' as usize] = C_WSP;
    t[b'\t' as usize] = C_WSP;
    t[b'\n' as usize] = C_WSP;
    t[b'\r' as usize] = C_WSP;

    // 设置数字
    let mut i = b'0';
    while i <= b'9' {
        t[i as usize] |= C_DIG | C_ALP;
        i += 1;
    }

    // 设置字母 (包含下划线)
    let mut i = b'a';
    while i <= b'z' {
        t[i as usize] |= C_ALP;
        i += 1;
    }
    let mut i = b'A';
    while i <= b'Z' {
        t[i as usize] |= C_ALP;
        i += 1;
    }
    t[b'_' as usize] |= C_ALP;

    // 设置引号
    t[b'\'' as usize] = C_QUO;
    t[b'"' as usize] = C_QUO;

    // 设置符号
    let syms = b"+-*/%()<>=,;.\\!&|^~";
    let mut j = 0;
    while j < syms.len() {
        t[syms[j] as usize] = C_SYM;
        j += 1;
    }
    t
}

const CHAR_TABLE: [u8; 256] = char_table();

#[derive(Debug)]
pub(crate) struct Lexer<'a> {
    inner: &'a [u8],
    position: usize,
    keyword_map: &'a KeywordMap,
}

impl<'a> Lexer<'a> {
    pub(crate) fn new(text: &'a str, keyword_map: &'a KeywordMap) -> Result<Self, ParserError> {
        Ok(Self {
            inner: text.as_bytes(),
            position: 0,
            keyword_map,
        })
    }

    // #[inline]
    fn skip_whitespace(&mut self) {
        let (_, end) =
            longest_consecutive_matching(self.inner, [b' ', b'\t', b'\n', b'\r'], self.position);
        self.position = {
            if end == -1 {
                self.position
            } else {
                end.cast_unsigned() + 1
            }
        };
    }

    // #[inline]
    fn scan_number(&mut self) -> Result<(TokenKind, usize, usize), ParserError> {
        let start = self.position;
        if let Some(b'-') = self.inner.get(self.position) {
            self.position += 1;
        }

        let (kind, _, end) = self.scan_digit_number()?;
        self.position = end;

        Ok((kind, start, self.position))
    }

    fn scan_digit_number(&mut self) -> Result<(TokenKind, usize, usize), ParserError> {
        let start = self.position;
        match self.scan_unsigned_number()? {
            Some((_kind, _, end)) => self.position = end,
            None => return Err(ParserError::InvalidToken(start, start)),
        }

        // 检测十六进制/八进制/二进制前缀 (仅前导 '0')
        if start == self.position && self.inner.get(self.position) == Some(&b'0') {
            match self.inner.get(self.position + 1) {
                Some(b'x') | Some(b'X') => {
                    self.position += 2;
                    let hex_start = self.position;
                    let (_, end) = mixed_match(
                        self.inner,
                        [(b'0', b'9'), (b'a', b'f'), (b'A', b'F')],
                        [b'_'],
                        hex_start,
                    );
                    if end == -1 {
                        return Err(ParserError::InvalidToken(hex_start, hex_start));
                    }
                    self.position = end.cast_unsigned();
                    return Ok((TokenKind::Number, start, self.position));
                }
                Some(b'o') | Some(b'O') => {
                    self.position += 2;
                    let oct_start = self.position;
                    let (_, end) = mixed_match(self.inner, [(b'0', b'7')], [b'_'], oct_start);
                    if end == -1 {
                        return Err(ParserError::InvalidToken(oct_start, oct_start));
                    }
                    self.position = end.cast_unsigned();
                    return Ok((TokenKind::Number, start, self.position));
                }
                Some(b'b') | Some(b'B') => {
                    self.position += 2;
                    let bin_start = self.position;
                    let (_, end) = mixed_match(self.inner, [(b'0', b'1')], [b'_'], bin_start);
                    if end == -1 {
                        return Err(ParserError::InvalidToken(bin_start, bin_start));
                    }
                    self.position = end.cast_unsigned();
                    return Ok((TokenKind::Number, start, self.position));
                }
                _ => {}
            }
        }

        let mut exists_dot = false;
        let mut exists_log = false;

        loop {
            match self.inner.get(self.position + 1) {
                Some(b'.') if !exists_dot => {
                    let next = self.position + 2;
                    match self.inner.get(next) {
                        Some(n) if CHAR_TABLE[*n as usize] & C_DIG != 0 => {
                            self.position = next;
                            match self.scan_unsigned_number()? {
                                Some((_kind, _, end)) => self.position = end,
                                None => return Err(ParserError::InvalidToken(start, start)),
                            }
                            exists_dot = true;
                        }
                        _ => return Err(ParserError::InvalidToken(self.position, self.position)),
                    }
                }
                Some(b'.') if exists_dot => {
                    return Err(ParserError::InvalidToken(self.position, self.position));
                }
                Some(b'_') => {
                    let next = self.position + 2;
                    match self.inner.get(next) {
                        Some(n) if CHAR_TABLE[*n as usize] & C_DIG != 0 => {
                            self.position = next;
                            match self.scan_unsigned_number()? {
                                Some((_kind, _, end)) => self.position = end,
                                None => return Err(ParserError::InvalidToken(start, start)),
                            }
                        }
                        _ => return Err(ParserError::InvalidToken(self.position, self.position)),
                    }
                }
                Some(b'E') | Some(b'e') if !exists_log => {
                    let next = self.position + 2;

                    match self.inner.get(next) {
                        Some(n) if CHAR_TABLE[*n as usize] & C_DIG != 0 => {
                            self.position = next;
                            match self.scan_unsigned_number()? {
                                Some((_kind, _, end)) => self.position = end,
                                None => return Err(ParserError::InvalidToken(start, start)),
                            }
                            exists_log = true;
                        }
                        _ => return Err(ParserError::InvalidToken(self.position, self.position)),
                    }
                }
                Some(b'E') | Some(b'e') if exists_log => {
                    return Err(ParserError::InvalidToken(self.position, self.position));
                }
                Some(n) if CHAR_TABLE[*n as usize] & C_ALP != 0 => {
                    return Err(ParserError::InvalidToken(self.position, self.position));
                }
                _ => break,
            }
        }

        Ok((TokenKind::Number, start, self.position))
    }

    fn scan_unsigned_number(&mut self) -> Result<Option<(TokenKind, usize, usize)>, ParserError> {
        let (start, end) = find_consecutive_in_range(self.inner, (b'0', b'9'), self.position);
        if end == -1 {
            Ok(None)
        } else {
            self.position = end.cast_unsigned();
            Ok(Some((TokenKind::Number, start, self.position)))
        }
    }

    // #[inline]
    fn scan_identify(&mut self) -> Result<(TokenKind, usize, usize), ParserError> {
        let start = self.position;
        let _pos = self.position;
        let _length = self.inner.len();

        let (_, end) = mixed_match(
            self.inner,
            [(b'a', b'z'), (b'A', b'Z'), (b'0', b'9')],
            [b'_'],
            start,
        );
        self.position = if end == -1 {
            start
        } else {
            end.cast_unsigned()
        };
        let end = self.position;

        let source = match self.inner.get(start..=end) {
            Some(s) => s,
            None => return Err(ParserError::InvalidToken(start, end)),
        };

        if let Some(keyword) = self.maybe_keyword(source) {
            Ok((TokenKind::Keyword(keyword), start, end))
        } else {
            Ok((TokenKind::Identifier, start, end))
        }
    }

    // 可能是关键词
    fn maybe_keyword(&self, source: &[u8]) -> Option<Keyword> {
        self.keyword_map
            .match_keyword(unsafe { std::str::from_utf8_unchecked(source) })
    }

    fn skip_block_comment(&mut self) -> Result<(), ParserError> {
        let (_, end) = skip_until_sequence(self.inner, [b'*', b'/'], self.position + 2);
        if end == -1 {
            return Err(ParserError::InvalidToken(self.position, self.position));
        }
        self.position = end as usize + 2;
        Ok(())
    }

    fn skip_line_comment(&mut self) {
        let (_, end) = skip_until_match(self.inner, [b'\n'], self.position + 2);
        if end == -1 {
            self.position = self.inner.len();
        } else {
            self.position = end as usize + 1;
        }
    }

    // 匹配字符串
    // #[inline]
    fn scan_string(&mut self, terminator: u8) -> Result<(TokenKind, usize, usize), ParserError> {
        let start = self.position;
        let mut pos = self.position + 1;

        loop {
            let (_, next) = skip_until_match(self.inner, [terminator], pos);
            if next == -1 {
                return Err(ParserError::InvalidToken(start, self.position));
            }

            let candidate = next as usize;
            if !is_escaped(self.inner, candidate, start) {
                self.position = candidate;
                return Ok((TokenKind::StringLiteral, start, self.position));
            }
            pos = candidate + 1;
        }
    }

    fn scan_symbol(&mut self, table: &mut TokenTable<'a>) -> Result<(), ParserError> {
        let start = self.position;
        let end = self.position;
        match self.inner.get(self.position) {
            Some(b'(') => {
                table.push(
                    TokenKind::LeftParen,
                    unsafe {str::from_utf8_unchecked(&self.inner[start..=end])},
                );
                self.position += 1;
            }
            Some(b')') => {
                table.push(
                    TokenKind::RightParen,
                    unsafe {str::from_utf8_unchecked(&self.inner[start..=end])},
                );
                self.position += 1;
            }
            Some(b'<') => match self.inner.get(self.position + 1) {
                Some(b'=') => {
                    table.push(
                        TokenKind::LessEqual,
                        unsafe {str::from_utf8_unchecked(&self.inner[self.position..=self.position + 1])},
                    );
                    self.position += 2;
                }
                Some(b'>') => {
                    table.push(
                        TokenKind::NotEqual,
                        unsafe {str::from_utf8_unchecked(&self.inner[self.position..=self.position + 1])},
                    );
                    self.position += 2;
                }
                _ => {
                    table.push(
                        TokenKind::Less,
                        unsafe {str::from_utf8_unchecked(&self.inner[start..=end])},
                    );
                    self.position += 1;
                }
            },
            Some(b'>') => match self.inner.get(self.position + 1) {
                Some(b'=') => {
                    table.push(
                        TokenKind::GreaterEqual,
                        unsafe {str::from_utf8_unchecked(&self.inner[self.position..=self.position + 1])},
                    );
                    self.position += 2;
                }
                _ => {
                    table.push(
                        TokenKind::Greater,
                        unsafe {str::from_utf8_unchecked(&self.inner[start..=end])},
                    );
                    self.position += 1;
                }
            },
            Some(b'=') => {
                table.push(
                    TokenKind::Equal,
                    unsafe {str::from_utf8_unchecked(&self.inner[start..=end])},
                );
                self.position += 1;
            }
            Some(b'.') => {
                table.push(
                    TokenKind::Dot,
                    unsafe {str::from_utf8_unchecked(&self.inner[start..=end])},
                );
                self.position += 1;
            }
            Some(b',') => {
                table.push(
                    TokenKind::Comma,
                    unsafe {str::from_utf8_unchecked(&self.inner[start..=end])},
                );
                self.position += 1;
            }
            Some(b'+') => {
                table.push(
                    TokenKind::Plus,
                    unsafe {str::from_utf8_unchecked(&self.inner[start..=end])},
                );
                self.position += 1;
            }
            Some(b'-') => match self.inner.get(self.position + 1) {
                Some(b'-') => {
                    self.skip_line_comment();
                }
                Some(b'0'..=b'9') => {
                    let start = self.position;
                    self.position += 1;
                    let (kind, _, end) = self.scan_number()?;
                    table.push(kind, unsafe {str::from_utf8_unchecked(&self.inner[start..=end])});
                    self.position += 1;
                }
                _ => {
                    table.push(
                        TokenKind::Subtract,
                        unsafe {str::from_utf8_unchecked(&self.inner[start..=end])},
                    );
                    self.position += 1;
                }
            },
            Some(b'*') => {
                table.push(
                    TokenKind::Multiply,
                    unsafe {str::from_utf8_unchecked(&self.inner[start..=end])},
                );
                self.position += 1;
            }
            Some(b'/') => match self.inner.get(self.position + 1) {
                Some(b'*') => {
                    self.skip_block_comment()?;
                }
                Some(b'/') => {
                    self.skip_line_comment();
                }
                _ => {
                    table.push(
                        TokenKind::Divide,
                        unsafe {str::from_utf8_unchecked(&self.inner[start..=end])},
                    );
                    self.position += 1;
                }
            },
            Some(b'%') => {
                table.push(
                    TokenKind::Mod,
                    unsafe {str::from_utf8_unchecked(&self.inner[start..=end])},
                );
                self.position += 1;
            }
            Some(b';') => {
                table.push(
                    TokenKind::Eof,
                    unsafe {str::from_utf8_unchecked(&self.inner[start..=end])},
                );
                self.position += 1;
            }
            Some(b'&') => {
                table.push(
                    TokenKind::And,
                    unsafe {str::from_utf8_unchecked(&self.inner[start..=end])},
                );
                self.position += 1;
            }
            Some(b'|') => {
                table.push(
                    TokenKind::Or,
                    unsafe {str::from_utf8_unchecked(&self.inner[start..=end])},
                );
                self.position += 1;
            }
            Some(b'^') => {
                table.push(
                    TokenKind::Xor,
                    unsafe {str::from_utf8_unchecked(&self.inner[start..=end]) },
                );
                self.position += 1;
            }
            Some(b'!') => match self.inner.get(self.position + 1) {
                Some(b'=') => {
                    table.push(
                        TokenKind::NotEqual,
                        unsafe {str::from_utf8_unchecked(&self.inner[self.position..=self.position + 1])},
                    );
                    self.position += 2;
                }
                _ => {
                    return Err(ParserError::InvalidToken(start, end));
                }
            },
            _ => return Err(ParserError::InvalidToken(start, end)),
        };
        Ok(())
    }

    pub(crate) fn tokenize(&mut self, table: &mut TokenTable<'a>) -> Result<(), ParserError> {
        loop {
            self.skip_whitespace();

            let c = match self.inner.get(self.position) {
                Some(c) => *c,
                None => break,
            };

            let char_class = CHAR_TABLE[c as usize];

            if (char_class & C_ALP) != 0 {
                if (char_class & C_DIG) != 0 {
                    let (kind, start, end) = self.scan_number()?;
                    table.push(kind, unsafe {str::from_utf8_unchecked(&self.inner[start..=end])});
                    self.position += 1;
                } else {
                    let (kind, start, end) = self.scan_identify()?;
                    table.push(kind, unsafe {str::from_utf8_unchecked(&self.inner[start..=end])});
                    self.position += 1;
                }
            } else if (char_class & C_SYM) != 0 {
                self.scan_symbol(&mut *table)?;
            } else if (char_class & C_QUO) != 0 {
                let (kind, start, end) = self.scan_string(c)?;
                table.push(kind, unsafe {str::from_utf8_unchecked(&self.inner[start..=end])});
                self.position += 1;
            } else {
                table.push(
                    TokenKind::Unknown,
                    unsafe {str::from_utf8_unchecked(&self.inner[self.position..=self.position])},
                );
                self.position += 1;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::*;
    use crate::token::TokenKind;

    fn tokenize<'a>(
        keyword_map: &'a KeywordMap,
        sql: &'a str,
    ) -> Result<(Vec<TokenKind>, Vec<&'a str>), ParserError> {
        let mut table = TokenTable::with_source(sql);
        let mut lexer = Lexer::new(sql, &keyword_map).unwrap();
        lexer.tokenize(&mut table)?;
        Ok((table.tokens, table.source_ref_list))
    }

    fn tokenize_err(sql: &str) -> bool {
        let keyword_map = KeywordMap::new().unwrap();
        let mut table = TokenTable::with_source(sql);
        let mut lexer = Lexer::new(sql, &keyword_map).unwrap();
        lexer.tokenize(&mut table).is_err()
    }

    #[test]
    fn test_skip_whitespace() {
        let keyword_map = KeywordMap::new().unwrap();
        assert_eq!(
            tokenize(
                &keyword_map,
                r#"                 
                "#
            )
            .unwrap(),
            (vec![], vec![])
        );
    }

    #[test]
    fn test_match_number() {
        let keyword_map = KeywordMap::new().unwrap();
        assert_eq!(
            tokenize(&keyword_map, "1234567890").unwrap(),
            (vec![TokenKind::Number], vec!["1234567890"])
        );
        assert_eq!(
            tokenize(
                &keyword_map,
                "123451111111111111111111111111111111111111 2222222222222222222222222222"
            )
            .unwrap(),
            (
                vec![TokenKind::Number, TokenKind::Number],
                vec![
                    "123451111111111111111111111111111111111111",
                    "2222222222222222222222222222"
                ]
            )
        );
        assert_eq!(
            tokenize(&keyword_map, "-123").unwrap(),
            (vec![TokenKind::Number], vec!["-123"])
        );
        assert_eq!(
            tokenize(&keyword_map, "-123.456").unwrap(),
            (vec![TokenKind::Number], vec!["-123.456"])
        );
        assert_eq!(
            tokenize(&keyword_map, "123_456_7890").unwrap(),
            (vec![TokenKind::Number], vec!["123_456_7890"])
        );
        assert_eq!(
            tokenize(&keyword_map, "-123.456E10").unwrap(),
            (vec![TokenKind::Number], vec!["-123.456E10"])
        );
        assert_eq!(
            tokenize(&keyword_map, "-123.456_789E10").unwrap(),
            (
                vec![TokenKind::Number],
                vec!["-123.456_789E10"]
            )
        );
        assert_eq!(
            tokenize(&keyword_map, "1").unwrap(),
            (vec![TokenKind::Number], vec!["1"])
        );
        assert_eq!(
            tokenize(&keyword_map, "0xFF").unwrap(),
            (vec![TokenKind::Number], vec!["0xFF"])
        );
        assert_eq!(
            tokenize(&keyword_map, "0x1A2b").unwrap(),
            (vec![TokenKind::Number], vec!["0x1A2b"])
        );
        assert_eq!(
            tokenize(&keyword_map, "0xff_ee").unwrap(),
            (vec![TokenKind::Number], vec!["0xff_ee"])
        );
        assert_eq!(
            tokenize(&keyword_map, "-0xDEAD").unwrap(),
            (vec![TokenKind::Number], vec!["-0xDEAD"])
        );
        assert_eq!(
            tokenize(&keyword_map, "0o777").unwrap(),
            (vec![TokenKind::Number], vec!["0o777"])
        );
        assert_eq!(
            tokenize(&keyword_map, "0b1010").unwrap(),
            (vec![TokenKind::Number], vec!["0b1010"])
        );
        assert_eq!(
            tokenize(&keyword_map, "0b1111_0000").unwrap(),
            (vec![TokenKind::Number], vec!["0b1111_0000"])
        );
        assert!(tokenize_err("0x"));
        assert_eq!(
            tokenize(&keyword_map, "0").unwrap(),
            (vec![TokenKind::Number], vec!["0"])
        );
        assert_eq!(
            tokenize(&keyword_map, "0123").unwrap(),
            (vec![TokenKind::Number], vec!["0123"])
        );
    }

    #[test]
    fn test_tokenize_cmp() {
        let keyword_map = KeywordMap::new().unwrap();
        let (tokens, positions) = tokenize(&keyword_map, "a > b >= c < d <= e <> f = g").unwrap();
        assert_eq!(
            tokens,
            vec![
                TokenKind::Identifier,
                TokenKind::Greater,
                TokenKind::Identifier,
                TokenKind::GreaterEqual,
                TokenKind::Identifier,
                TokenKind::Less,
                TokenKind::Identifier,
                TokenKind::LessEqual,
                TokenKind::Identifier,
                TokenKind::NotEqual,
                TokenKind::Identifier,
                TokenKind::Equal,
                TokenKind::Identifier
            ]
        );
        assert_eq!(
            positions,
            vec![
                "a",
                ">",
                "b",
                ">=",
                "c",
                "<",
                "d",
                "<=",
                "e",
                "<>",
                "f",
                "=",
                "g"
            ]
        );
    }

    #[test]
    fn test_match_string() {
        let keyword_map = KeywordMap::new().unwrap();
        assert_eq!(
            tokenize(&keyword_map, "''").unwrap(),
            (vec![TokenKind::StringLiteral], vec!["''"])
        );
        assert_eq!(
            tokenize(&keyword_map, "'helloWorld'").unwrap(),
            (
                vec![TokenKind::StringLiteral],
                vec!["'helloWorld'"]
            )
        );
        assert_eq!(
            tokenize(&keyword_map, r#"'hello\\'"#).unwrap(),
            (
                vec![TokenKind::StringLiteral],
                vec![r#"'hello\\'"#]
            )
        );
        assert_eq!(
            tokenize(
                &keyword_map,
                "'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'"
            )
            .unwrap(),
            (
                vec![TokenKind::StringLiteral],
                vec![
                    "'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'"
                ]
            )
        );
        assert_eq!(
            tokenize(
                &keyword_map,
                "\'aaaaaaaaaaaaa\\'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\'"
            )
            .unwrap(),
            (
                vec![TokenKind::StringLiteral],
                vec![
                    "\'aaaaaaaaaaaaa\\'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\'"
                ]
            )
        );
    }

    #[test]
    fn test_match_indentify() {
        let keyword_map = KeywordMap::new().unwrap();
        assert_eq!(
            tokenize(&keyword_map, "asdfghjk").unwrap(),
            (vec![TokenKind::Identifier], vec!["asdfghjk"])
        );
        assert_eq!(
            tokenize(
                &keyword_map,
                "qwertyuiopASDFGHJKL1234567890_zxcvbnm 1234567890"
            )
            .unwrap(),
            (
                vec![TokenKind::Identifier, TokenKind::Number],
                vec![
                    "qwertyuiopASDFGHJKL1234567890_zxcvbnm",
                    "1234567890"
                ]
            )
        );
    }

    #[test]
    fn test_keyword() {
        let keyword_map = KeywordMap::new().unwrap();
        let (tokens, positions) = tokenize(&keyword_map, "select from").unwrap();
        assert_eq!(
            tokens,
            vec![
                TokenKind::Keyword(Keyword::Select),
                TokenKind::Keyword(Keyword::From)
            ]
        );
        assert_eq!(
            positions,
            vec!["select", "from"]
        );
    }

    #[test]
    fn test_sql() {
        let keyword_map = KeywordMap::new().unwrap();
        let (tokens, positions) = tokenize(&keyword_map, "select * from a").unwrap();
        assert_eq!(
            tokens,
            vec![
                TokenKind::Keyword(Keyword::Select),
                TokenKind::Multiply,
                TokenKind::Keyword(Keyword::From),
                TokenKind::Identifier,
            ]
        );
        assert_eq!(
            positions,
            vec![
                "select",
                "*",
                "from",
                "a"
            ]
        );
    }

    #[test]
    fn test_sql2() {
        let keyword_map = KeywordMap::new().unwrap();
        let (tokens, positions) =
            tokenize(&keyword_map, "select * from a where b in (1,2,3) and c = 1").unwrap();
        assert_eq!(
            tokens,
            vec![
                TokenKind::Keyword(Keyword::Select),
                TokenKind::Multiply,
                TokenKind::Keyword(Keyword::From),
                TokenKind::Identifier,
                TokenKind::Keyword(Keyword::Where),
                TokenKind::Identifier,
                TokenKind::Keyword(Keyword::In),
                TokenKind::LeftParen,
                TokenKind::Number,
                TokenKind::Comma,
                TokenKind::Number,
                TokenKind::Comma,
                TokenKind::Number,
                TokenKind::RightParen,
                TokenKind::Keyword(Keyword::And),
                TokenKind::Identifier,
                TokenKind::Equal,
                TokenKind::Number
            ]
        );
        assert_eq!(
            positions,
            vec![
                "select",
                "*",
                "from",
                "a",
                "where",
                "b",
                "in",
                "(",
                "1",
                ",",
                "2",
                ",",
                "3",
                ")",
                "and",
                "c",
                "=",
                "1",
            ]
        );
    }
}
