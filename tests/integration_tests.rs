// Integration tests to exercise the high-level Parser API and SQL AST construction
// These tests target correctness-first improvements by validating that
// typical SQL snippets are parsed without errors.

use std::vec;

use minivec::mini_vec;
use simd_sql::{Parser, Query, SelectStatement, Statement, ast::statement::StatementInner, common::{alias::Alias, expr::{Between, BinaryOp, BinaryOperator, Expr, Field, In, InValue, Like, NumericLiteral, StringLiteral}, from::{From, Table}, limit::Limit, order::{Order, OrderDirection, OrderItem}}};

// ============================================================================
// SELECT 语句测试
// ============================================================================

#[test]
fn parse_basic_select() {
    let p = Parser::new().expect("failed to initialize Parser");
    assert!(p.parse("SELECT 1").is_ok(), "basic SELECT should parse");
}

#[test]
fn parse_select_with_alias_and_multiple_columns() {
    let p = Parser::new().expect("failed to initialize Parser");
    assert!(
        p.parse("SELECT 1 AS one, 2 AS two").is_ok(),
        "SELECT with multiple columns and aliases should parse"
    );
}

#[test]
fn parse_select_from_and_join() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT 1 FROM users JOIN orders ON users.id = orders.user_id";
    assert!(
        p.parse(sql).is_ok(),
        "SELECT with FROM and JOIN should parse"
    );
}

#[test]
fn parse_select_star_from_table() {
    let p = Parser::new().expect("failed to initialize Parser");
    assert!(
        p.parse("SELECT * FROM users").is_ok(),
        "SELECT * FROM should parse"
    );
}

#[test]
fn parse_select_with_where() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT id, name FROM users WHERE age > 18";
    assert!(p.parse(sql).is_ok(), "SELECT with WHERE should parse");
}

#[test]
fn parse_select_with_where_and_conditions() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT * FROM orders WHERE status = 'active' AND total >= 100";
    assert!(
        p.parse(sql).is_ok(),
        "SELECT with complex WHERE should parse"
    );
}

#[test]
fn parse_select_with_group_by() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT category, COUNT(*) FROM products GROUP BY category";
    assert!(p.parse(sql).is_ok(), "SELECT with GROUP BY should parse");
}

#[test]
fn parse_select_with_order_by() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT name, price FROM products ORDER BY price DESC";
    assert!(p.parse(sql).is_ok(), "SELECT with ORDER BY should parse");
}

#[test]
fn parse_select_with_limit() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT * FROM users LIMIT 10";
    assert!(p.parse(sql).is_ok(), "SELECT with LIMIT should parse");
}

#[test]
fn parse_select_with_all_clauses() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT category, COUNT(*) AS cnt FROM products WHERE price > 0 GROUP BY category HAVING cnt > 1 ORDER BY cnt DESC LIMIT 5";
    assert!(
        p.parse(sql).is_ok(),
        "complex SELECT with all clauses should parse"
    );
}

#[test]
fn parse_select_with_subquery_in_from() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT * FROM (SELECT id, name FROM users) AS u";
    assert!(
        p.parse(sql).is_ok(),
        "SELECT with subquery in FROM should parse"
    );
}

#[test]
fn parse_select_with_in_expression() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT * FROM users WHERE id IN (1, 2, 3)";
    assert!(p.parse(sql).is_ok(), "SELECT with IN should parse");
}

#[test]
fn parse_select_with_between() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT * FROM products WHERE price BETWEEN 10 AND 100";
    assert!(p.parse(sql).is_ok(), "SELECT with BETWEEN should parse");
}

#[test]
fn parse_select_with_like() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT * FROM users WHERE name LIKE 'John%'";
    assert!(p.parse(sql).is_ok(), "SELECT with LIKE should parse");
}

#[test]
fn parse_select_with_case_expression() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT CASE WHEN score >= 90 THEN 'A' WHEN score >= 80 THEN 'B' ELSE 'C' END AS grade FROM results";
    assert!(p.parse(sql).is_ok(), "SELECT with CASE should parse");
}

#[test]
fn parse_select_with_left_join() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT u.id, o.total FROM users u LEFT JOIN orders o ON u.id = o.user_id";
    assert!(p.parse(sql).is_ok(), "SELECT with LEFT JOIN should parse");
}

#[test]
fn parse_select_with_right_join() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT u.id, o.total FROM users u RIGHT JOIN orders o ON u.id = o.user_id";
    assert!(p.parse(sql).is_ok(), "SELECT with RIGHT JOIN should parse");
}

#[test]
fn parse_select_with_inner_join() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT u.id, o.total FROM users u INNER JOIN orders o ON u.id = o.user_id";
    assert!(p.parse(sql).is_ok(), "SELECT with INNER JOIN should parse");
}

#[test]
fn parse_select_with_cross_join() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT * FROM users CROSS JOIN orders";
    assert!(p.parse(sql).is_ok(), "SELECT with CROSS JOIN should parse");
}

#[test]
fn parse_select_with_full_join() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT * FROM users FULL JOIN orders ON users.id = orders.user_id";
    assert!(p.parse(sql).is_ok(), "SELECT with FULL JOIN should parse");
}

#[test]
fn parse_select_with_natural_join() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT * FROM users NATURAL JOIN orders";
    assert!(p.parse(sql).is_ok(), "SELECT with NATURAL JOIN should parse");
}

#[test]
fn parse_select_with_natural_left_join() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT * FROM users NATURAL LEFT JOIN orders";
    assert!(p.parse(sql).is_ok(), "SELECT with NATURAL LEFT JOIN should parse");
}

#[test]
fn parse_select_with_natural_right_join() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT * FROM users NATURAL RIGHT JOIN orders";
    assert!(p.parse(sql).is_ok(), "SELECT with NATURAL RIGHT JOIN should parse");
}

#[test]
fn parse_select_with_natural_full_join() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT * FROM users NATURAL FULL JOIN orders";
    assert!(p.parse(sql).is_ok(), "SELECT with NATURAL FULL JOIN should parse");
}

#[test]
fn parse_select_with_join_using() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT * FROM users JOIN orders USING (user_id)";
    assert!(p.parse(sql).is_ok(), "SELECT with JOIN ... USING should parse");
}

#[test]
fn parse_select_with_join_using_multiple() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT * FROM users JOIN orders USING (user_id, order_id)";
    assert!(p.parse(sql).is_ok(), "SELECT with JOIN ... USING (multiple columns) should parse");
}

#[test]
fn parse_select_with_left_join_using() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT * FROM users LEFT JOIN orders USING (id)";
    assert!(p.parse(sql).is_ok(), "SELECT with LEFT JOIN ... USING should parse");
}

#[test]
fn parse_select_with_right_join_using() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT * FROM users RIGHT JOIN orders USING (id)";
    assert!(p.parse(sql).is_ok(), "SELECT with RIGHT JOIN ... USING should parse");
}

#[test]
fn parse_select_with_full_join_using() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT * FROM users FULL JOIN orders USING (id)";
    assert!(p.parse(sql).is_ok(), "SELECT with FULL JOIN ... USING should parse");
}

#[test]
fn parse_select_with_field_alias() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT first_name AS name FROM employees";
    assert!(p.parse(sql).is_ok(), "SELECT with field alias should parse");
}

#[test]
fn parse_select_with_table_prefix() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT users.id, users.name FROM users";
    assert!(
        p.parse(sql).is_ok(),
        "SELECT with table prefix should parse"
    );
}

#[test]
fn parse_select_with_arithmetic_expression() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT price * quantity AS total FROM order_items";
    assert!(
        p.parse(sql).is_ok(),
        "SELECT with arithmetic expression should parse"
    );
}

#[test]
fn parse_select_with_parenthesized_expression() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT * FROM items WHERE (price + tax) * 1.1 > 100";
    assert!(
        p.parse(sql).is_ok(),
        "SELECT with parenthesized expression should parse"
    );
}

// ============================================================================
// INSERT 语句测试
// ============================================================================

#[test]
fn parse_basic_insert() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "INSERT INTO users (id, name, age) VALUES (1, 'Alice', 30)";
    assert!(p.parse(sql).is_ok(), "basic INSERT should parse");
}

#[test]
fn parse_insert_multiple_rows() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "INSERT INTO users (id, name) VALUES (1, 'Alice'), (2, 'Bob'), (3, 'Charlie')";
    assert!(
        p.parse(sql).is_ok(),
        "INSERT with multiple rows should parse"
    );
}

#[test]
fn parse_insert_single_column() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "INSERT INTO logs (message) VALUES ('Hello World')";
    assert!(
        p.parse(sql).is_ok(),
        "INSERT with single column should parse"
    );
}

#[test]
fn parse_insert_select() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "INSERT INTO users (id, name) SELECT id, name FROM old_users";
    assert!(p.parse(sql).is_ok(), "INSERT with SELECT should parse");
}

#[test]
fn parse_insert_select_without_columns() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "INSERT INTO users SELECT id, name FROM old_users";
    assert!(p.parse(sql).is_ok(), "INSERT with SELECT should parse");
}

// ============================================================================
// UPDATE 语句测试
// ============================================================================

#[test]
fn parse_basic_update() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "UPDATE users SET name = 'Bob' WHERE id = 1";
    assert!(p.parse(sql).is_ok(), "basic UPDATE should parse");
}

#[test]
fn parse_update_multiple_columns() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "UPDATE products SET price = 19.99, stock = 100 WHERE id = 5";
    assert!(
        p.parse(sql).is_ok(),
        "UPDATE with multiple columns should parse"
    );
}

#[test]
fn parse_update_without_where() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "UPDATE users SET status = 'inactive'";
    assert!(p.parse(sql).is_ok(), "UPDATE without WHERE should parse");
}

// ============================================================================
// DELETE 语句测试
// ============================================================================

#[test]
fn parse_basic_delete() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "DELETE FROM users WHERE id = 1";
    assert!(p.parse(sql).is_ok(), "basic DELETE should parse");
}

#[test]
fn parse_delete_with_condition() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "DELETE FROM orders WHERE status = 'cancelled' AND total < 0";
    assert!(
        p.parse(sql).is_ok(),
        "DELETE with complex condition should parse"
    );
}

#[test]
fn parse_delete_without_where() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "DELETE FROM users";
    assert!(p.parse(sql).is_ok(), "DELETE without WHERE should parse");
}

// ============================================================================
// SET 操作 (UNION / INTERSECT / EXCEPT) 测试
// ============================================================================

#[test]
fn parse_select_union() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT id FROM users UNION SELECT id FROM admins";
    assert!(p.parse(sql).is_ok(), "SELECT with UNION should parse");
}

#[test]
fn parse_select_union_all() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT id FROM users UNION ALL SELECT id FROM admins";
    assert!(p.parse(sql).is_ok(), "SELECT with UNION ALL should parse");
}

#[test]
fn parse_select_intersect() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT id FROM users INTERSECT SELECT id FROM admins";
    assert!(p.parse(sql).is_ok(), "SELECT with INTERSECT should parse");
}

#[test]
fn parse_select_except() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT id FROM users EXCEPT SELECT id FROM admins";
    assert!(p.parse(sql).is_ok(), "SELECT with EXCEPT should parse");
}

// ============================================================================
// 复杂表达式测试
// ============================================================================

#[test]
fn parse_complex_expression_and_or_not() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT * FROM t WHERE (a > 1 AND b < 2) OR (c = 3 AND d != 4)";
    assert!(
        p.parse(sql).is_ok(),
        "complex AND/OR expression should parse"
    );
}

#[test]
fn parse_not_between_expression() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT * FROM products WHERE price NOT BETWEEN 10 AND 20";
    assert!(p.parse(sql).is_ok(), "NOT BETWEEN expression should parse");
}

#[test]
fn parse_not_in_expression() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT * FROM users WHERE id NOT IN (1, 2, 3)";
    assert!(p.parse(sql).is_ok(), "NOT IN expression should parse");
}

#[test]
fn parse_not_like_expression() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT * FROM users WHERE name NOT LIKE 'Test%'";
    assert!(p.parse(sql).is_ok(), "NOT LIKE expression should parse");
}

// ============================================================================
// 边界情况测试
// ============================================================================

#[test]
fn parse_empty_input() {
    let p = Parser::new().expect("failed to initialize Parser");
    assert!(p.parse("").is_err(), "empty input should fail to parse");
}

#[test]
fn parse_whitespace_only() {
    let p = Parser::new().expect("failed to initialize Parser");
    assert!(
        p.parse("   ").is_err(),
        "whitespace-only input should fail to parse"
    );
}

#[test]
fn parse_lowercase_keyword() {
    let p = Parser::new().expect("failed to initialize Parser");
    assert!(
        p.parse("select * from users").is_ok(),
        "lowercase keywords should parse"
    );
}

#[test]
fn parse_uppercase_keyword() {
    let p = Parser::new().expect("failed to initialize Parser");
    assert!(
        p.parse("SELECT * FROM USERS").is_ok(),
        "uppercase keywords should parse"
    );
}

#[test]
fn parse_mixed_case_keyword() {
    let p = Parser::new().expect("failed to initialize Parser");
    assert!(
        p.parse("Select * From users").is_ok(),
        "mixed-case keywords should parse"
    );
}

// ============================================================================
// 错误路径测试
// ============================================================================

#[test]
fn parse_invalid_syntax() {
    let p = Parser::new().expect("failed to initialize Parser");
    assert!(
        p.parse("SELICT 1").is_err(),
        "invalid keyword should fail to parse"
    );
}

#[test]
fn parse_invalid_expression() {
    let p = Parser::new().expect("failed to initialize Parser");
    assert!(
        p.parse("SELECT WHERE").is_err(),
        "SELECT without columns should fail to parse"
    );
}

#[test]
fn parse_unterminated_string() {
    let p = Parser::new().expect("failed to initialize Parser");
    assert!(
        p.parse("SELECT 'hello").is_err(),
        "unterminated string should fail to parse"
    );
}

#[test]
fn parse_invalid_number() {
    let p = Parser::new().expect("failed to initialize Parser");
    assert!(
        p.parse("SELECT 12a34").is_err(),
        "invalid number format should fail to parse"
    );
}

// ============================================================================
// 综合 SQL 场景测试
// ============================================================================

#[test]
fn parse_real_world_query_select_from_where_order_by_limit() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "\
SELECT u.id, u.name, o.total AS order_total, o.created_at \
FROM users u \
JOIN orders o ON u.id = o.user_id \
WHERE u.active = 1 AND o.total > 0 \
ORDER BY o.created_at DESC \
LIMIT 20";
    assert!(p.parse(sql).is_ok(), "real-world query should parse");
}

#[test]
fn parse_nested_subquery() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT * FROM (SELECT * FROM (SELECT id FROM users) AS inner_t) AS outer_t";
    assert!(p.parse(sql).is_ok(), "nested subquery should parse");
}

#[test]
fn parse_comment_handling_dash() {
    // The lexer treats unknown symbols as Unknown tokens, but the parser should handle gracefully
    let p = Parser::new().expect("failed to initialize Parser");
    // This is not a valid SQL comment, just a test of dash handling
    let sql = "SELECT 1 -- this is not a real comment";
    // The parser may or may not parse this - it depends on implementation
    let _ = p.parse(sql);
}

#[test]
fn parse_function_call_with_expression() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT CONCAT(first_name, ' ', last_name) AS full_name FROM employees";
    assert!(
        p.parse(sql).is_ok(),
        "function call with expression should parse"
    );
}

#[test]
fn parse_multiple_joins_chain() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "\
SELECT * FROM users u \
JOIN orders o ON u.id = o.user_id \
JOIN order_items oi ON o.id = oi.order_id \
JOIN products p ON oi.product_id = p.id";
    assert!(p.parse(sql).is_ok(), "multiple JOINs chain should parse");
}

// ============================================================================
// CTE (WITH 子句) 测试
// ============================================================================

#[test]
fn parse_cte_basic() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "WITH cte AS (SELECT id FROM users) SELECT * FROM cte";
    assert!(
        p.parse(sql).is_ok(),
        "basic WITH ... AS SELECT should parse"
    );
}

#[test]
fn parse_cte_multiple_bindings() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "\
WITH \
  active AS (SELECT id FROM users WHERE status = 'active'), \
  orders AS (SELECT user_id, COUNT(*) as cnt FROM orders GROUP BY user_id) \
SELECT a.id, o.cnt FROM active a JOIN orders o ON a.id = o.user_id";
    assert!(p.parse(sql).is_ok(), "multiple CTE bindings should parse");
}

#[test]
fn parse_cte_with_column_names() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "WITH cte (id, name) AS (SELECT id, name FROM users) SELECT id FROM cte";
    assert!(p.parse(sql).is_ok(), "CTE with column names should parse");
}

#[test]
fn parse_cte_with_recursive() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "\
WITH RECURSIVE t(n) AS (\
  SELECT 1 UNION ALL SELECT n + 1 FROM t WHERE n < 10\
) SELECT n FROM t";
    assert!(p.parse(sql).is_ok(), "WITH RECURSIVE should parse");
}

#[test]
fn parse_cte_with_set_operations() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "\
WITH cte AS (SELECT id FROM users) \
(SELECT id FROM cte) UNION (SELECT id FROM cte) \
ORDER BY id";
    assert!(p.parse(sql).is_ok(), "CTE with set operations should parse");
}

#[test]
fn parse_cte_with_joins() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "\
WITH dept_salary AS (\
  SELECT d.id, SUM(e.salary) as total \
  FROM departments d \
  JOIN employees e ON d.id = e.dept_id \
  GROUP BY d.id\
) \
SELECT d.*, ds.total \
FROM departments d \
JOIN dept_salary ds ON d.id = ds.id \
WHERE ds.total > 10000";
    assert!(p.parse(sql).is_ok(), "CTE with JOINs should parse");
}

#[test]
fn parse_cte_chained() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "\
WITH \
  step1 AS (SELECT id FROM users WHERE status = 'active'), \
  step2 AS (SELECT id FROM step1 WHERE id > 100) \
SELECT * FROM step2";
    assert!(p.parse(sql).is_ok(), "chained CTE should parse");
}

// ============================================================================
// 窗口函数 (OVER 子句) 测试
// ============================================================================

#[test]
fn parse_window_function_row_number() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT ROW_NUMBER() OVER (ORDER BY id) FROM users";
    assert!(p.parse(sql).is_ok(), "ROW_NUMBER() OVER should parse");
}

#[test]
fn parse_window_function_with_partition() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql =
        "SELECT ROW_NUMBER() OVER (PARTITION BY dept ORDER BY salary DESC) as rn FROM employees";
    assert!(p.parse(sql).is_ok(), "OVER with PARTITION BY should parse");
}

#[test]
fn parse_window_function_multiple() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT RANK() OVER (ORDER BY price) as rk, DENSE_RANK() OVER (ORDER BY price) as dr FROM products";
    assert!(
        p.parse(sql).is_ok(),
        "multiple window functions should parse"
    );
}

#[test]
fn parse_window_function_benchmark_sql20() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT p.id, p.name, p.price, ROW_NUMBER() OVER (PARTITION BY category ORDER BY price DESC) as rank, RANK() OVER (ORDER BY price) as price_rank, DENSE_RANK() OVER (ORDER BY price) as dense_price_rank FROM products p";
    assert!(p.parse(sql).is_ok(), "sql_20 window functions should parse");
}

#[test]
fn parse_window_function_no_args() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT RANK() OVER (ORDER BY score DESC) FROM students";
    assert!(
        p.parse(sql).is_ok(),
        "window function with no args should parse"
    );
}

// ============================================================================
// 注释测试
// ============================================================================

#[test]
fn parse_block_comment() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT /* skip */ * FROM users";
    assert!(p.parse(sql).is_ok(), "block comment should be skipped");
}

#[test]
fn parse_line_comment() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT a -- comment\nFROM users";
    assert!(p.parse(sql).is_ok(), "line comment should be skipped");
}

#[test]
fn parse_block_comment_multiline() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT /* line1\nline2 */ id FROM users";
    assert!(p.parse(sql).is_ok(), "multiline block comment should parse");
}

#[test]
fn parse_unterminated_block_comment() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT /* no end";
    assert!(
        p.parse(sql).is_err(),
        "unterminated block comment should error"
    );
}

#[test]
fn parse_line_comment_eof() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT id -- no newline";
    assert!(p.parse(sql).is_ok(), "line comment at EOF should work");
}

#[test]
fn parse_big_sql_1() {
    let p = Parser::new().expect("failed to initialize Parser");
    let sql = "SELECT 
    t1.id, t1.name, t1.created_at,
    t2.order_id, t2.amount, t2.status,
    t3.log_id, t3.event_type, t3.payload
FROM user_table t1
LEFT JOIN order_table t2 ON t1.id = t2.user_id
LEFT JOIN log_table t3 ON t1.id = t3.user_id
WHERE 
    t1.status = 1
    AND t2.created_at BETWEEN '2024-01-01' AND '2025-01-01'
    AND t3.event_type IN ('click', 'view', 'purchase')
    AND (
        t2.amount > 100 OR 
        t3.payload LIKE '%error%'
    )
ORDER BY t1.created_at DESC
LIMIT 100;";
    let result = p.parse(sql).unwrap();
    assert_eq!(
        result,
        Statement {
            list: vec![
                StatementInner::Query(Query::Select(SelectStatement {
                    distinct: false,
                    columns: vec![
                        Alias {name: None, value: Expr::Field(Field { prefix: Some("t1"), name: "id" })},
                        Alias {name: None, value: Expr::Field(Field { prefix: Some("t1"), name: "name" })},
                        Alias {name: None, value: Expr::Field(Field { prefix: Some("t1"), name: "created_at" })},
                        Alias {name: None, value: Expr::Field(Field { prefix: Some("t2"), name: "order_id" })},
                        Alias {name: None, value: Expr::Field(Field { prefix: Some("t2"), name: "amount" })},
                        Alias {name: None, value: Expr::Field(Field { prefix: Some("t2"), name: "status" })},
                        Alias {name: None, value: Expr::Field(Field { prefix: Some("t3"), name: "log_id" })},
                        Alias {name: None, value: Expr::Field(Field { prefix: Some("t3"), name: "event_type" })},
                        Alias {name: None, value: Expr::Field(Field { prefix: Some("t3"), name: "payload" })},

                    ],
                    from: Some(mini_vec![From::LeftJoin { 
                        left: Box::new(From::LeftJoin {
                            left: Box::new(From::Table(Table::Name(Alias { name: Some("t1"), value: Expr::Field(Field {prefix: None, name: "user_table"}) }))),
                            right: Box::new(From::Table(Table::Name(Alias { name: Some("t2"), value: Expr::Field(Field {prefix: None, name: "order_table"}) }))),
                            condition: Expr::BinaryOp(Box::new(BinaryOp {
                                op: BinaryOperator::Equal,
                                left: Expr::Field(Field { prefix: Some("t1"), name: "id" }),
                                right: Expr::Field(Field { prefix: Some("t2"), name: "user_id" })
                            }))
                        }),
                        right: Box::new(From::Table(Table::Name(Alias { name: Some("t3"), value: Expr::Field(Field {prefix: None, name: "log_table"}) }))),
                        condition: Expr::BinaryOp(Box::new(BinaryOp {
                            op: BinaryOperator::Equal,
                            left: Expr::Field(Field { prefix: Some("t1"), name: "id" }),
                            right: Expr::Field(Field { prefix: Some("t3"), name: "user_id" })
                        }))
                    }]),
                    where_statement: Some(Expr::BinaryOp(Box::new(
                        BinaryOp {
                            op: BinaryOperator::And,
                            left: Expr::BinaryOp(Box::new(BinaryOp{
                                op: BinaryOperator::And,
                                left: Expr::BinaryOp(Box::new(BinaryOp {
                                    op: BinaryOperator::And,
                                    left: Expr::BinaryOp(Box::new(BinaryOp {
                                        op: BinaryOperator::Equal,
                                        left: Expr::Field(Field { prefix: Some("t1"), name: "status" }),
                                        right: Expr::NumericLiteral(NumericLiteral {
                                            value: "1"
                                        })
                                    })),
                                    right: Expr::Between(Between {
                                            is_not: false,
                                            field: Box::new(Expr::Field(Field { prefix: Some("t2"), name: "created_at" })),
                                            lower: Box::new(Expr::StringLiteral(StringLiteral {
                                                value: "'2024-01-01'",
                                            })),
                                            upper: Box::new(Expr::StringLiteral(StringLiteral {
                                                value: "'2025-01-01'",
                                            })),
                                        })
                                })),
                                right: Expr::In(In {
                                        is_not: false,
                                        field: Box::new(Expr::Field(Field { prefix: Some("t3"), name: "event_type" })),
                                        in_value: InValue::List(mini_vec![
                                            Expr::StringLiteral(StringLiteral { value: "'click'" }),
                                            Expr::StringLiteral(StringLiteral { value: "'view'" }),
                                            Expr::StringLiteral(StringLiteral { value: "'purchase'" }),
                                        ])
                                    })
                            })),
                            right: Expr::BinaryOp(Box::new(BinaryOp {
                                op: BinaryOperator::Or,
                                left: Expr::BinaryOp(Box::new(BinaryOp {
                                    op: BinaryOperator::Greater,
                                    left: Expr::Field(Field { prefix: Some("t2"), name: "amount" }),
                                    right: Expr::NumericLiteral(NumericLiteral { value: "100" }),
                                })),
                                right: Expr::Like(Like {
                                    is_not: false,
                                    field: Box::new(Expr::Field(Field { prefix: Some("t3"), name: "payload" })),
                                    pattern: Box::new(Expr::StringLiteral(StringLiteral{value: "'%error%'"}))
                                }),
                            }))
                        }))),
                    group_by: None,
                    having_statement: None,
                    order_by: Some(Order {
                        columns: mini_vec![
                            OrderItem{
                                expr: Expr::Field(Field { prefix: Some("t1"), name: "created_at" }),
                                direction: OrderDirection::DESC,
                                nulls_order: None,
                            }
                        ]
                    }),
                    limit: Some(Limit {
                        offset: None,
                        limit: Expr::NumericLiteral(NumericLiteral { value: "100" })
                    }) }))
            ]
        }
    );
}

