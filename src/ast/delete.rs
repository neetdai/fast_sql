use minivec::MiniVec;

use crate::{
    ParserError,
    common::{
        expr::Expr,
        from::{From, Table},
        utils::{expect_kind, maybe_kind},
    },
    keyword::Keyword,
    token::{TokenKind, TokenTable},
};

/// Represents a DELETE statement.
///
/// Supports three syntactic forms:
///
/// 1. Simple single-table delete:
///    `DELETE FROM users WHERE id = 1`
///
/// 2. DELETE with JOIN for filtering (MySQL / SQL Server style):
///    `DELETE u FROM users u JOIN orders o ON u.id = o.user_id WHERE o.status = 'done'`
///
/// 3. Multi-table delete (MySQL style):
///    `DELETE u, o FROM users u JOIN orders o ON u.id = o.user_id`
///
/// The `delete_tables` field lists explicit target aliases when present.
/// When `None`, the entire FROM clause is the implicit delete target.
#[derive(Debug, PartialEq)]
pub struct DeleteStatement<'a> {
    /// Optional explicit list of table aliases to delete from.
    /// Present in MySQL-style `DELETE t1, t2 FROM ...` syntax.
    pub delete_tables: Option<MiniVec<Table<'a>>>,
    /// The FROM clause — a single table or tree of joined tables.
    pub from: From<'a>,
    /// Optional WHERE condition.
    pub conditions: Option<Expr<'a>>,
}

impl<'a> DeleteStatement<'a> {
    pub(crate) fn new(
        token_table: &TokenTable<'a>,
        cursor: &mut usize,
    ) -> Result<Self, ParserError> {
        Self::build_ast(token_table, cursor)
    }

    fn build_ast(token_table: &TokenTable<'a>, cursor: &mut usize) -> Result<Self, ParserError> {
        expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Delete))?;
        *cursor += 1;

        // ── detect MySQL-style multi-table delete: DELETE t1, t2 FROM ... ──
        // If the token immediately after DELETE is an Identifier (not FROM),
        // collect comma-separated table aliases as explicit delete targets.
        let delete_tables = match token_table.get_kind(*cursor) {
            Some(TokenKind::Identifier) => {
                let mut tables = MiniVec::new();
                loop {
                    match token_table.get_kind(*cursor) {
                        Some(TokenKind::Comma) => {
                            *cursor += 1;
                        }
                        Some(TokenKind::Identifier) => {
                            tables.push(Table::class_name_with_single(token_table, cursor)?);
                        }
                        _ => break,
                    }
                }
                if tables.is_empty() {
                    None
                } else {
                    Some(tables)
                }
            }
            _ => None,
        };

        expect_kind(token_table, cursor, &TokenKind::Keyword(Keyword::From))?;
        *cursor += 1;

        // ── parse FROM clause (supports JOINs via From::parse) ──
        let from = From::parse(token_table, cursor)?;

        // ── optional WHERE condition ──
        let conditions = if maybe_kind(token_table, cursor, &TokenKind::Keyword(Keyword::Where)) {
            *cursor += 1;
            Some(Expr::build(token_table, cursor)?)
        } else {
            None
        };

        Ok(Self {
            delete_tables,
            from,
            conditions,
        })
    }
}
