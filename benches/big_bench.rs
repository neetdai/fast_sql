use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use mimalloc::MiMalloc;
use simd_sql::Parser;
use std::hint::black_box;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("sql parser 1");

    let parser = Parser::new().unwrap();

    let sql_1 = "SELECT c.customer_id, c.customer_name FROM customers c WHERE EXISTS ( SELECT 1 FROM orders o WHERE o.customer_id = c.customer_id AND EXISTS ( SELECT 1 FROM order_items oi WHERE oi.order_id = o.order_id AND oi.product_id IN ( SELECT product_id FROM products WHERE category_id = 2 ) ) );";
    let sql_len_1 = sql_1.len();
    group.throughput(Throughput::Elements(sql_1.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("big sql parser 1", sql_len_1),
        sql_1,
        |b, i| {
            b.iter(|| {
                parser.parse(black_box(&i)).unwrap();
            });
        },
    );

    let sql_2 = "WITH top_products AS ( SELECT product_id, SUM(quantity) AS total_sold FROM order_items GROUP BY product_id ORDER BY total_sold DESC LIMIT 10 ), product_details AS ( SELECT p.product_id, p.product_name, p.unit_price FROM products p WHERE p.product_id IN (SELECT product_id FROM top_products) ) SELECT tp.product_id, pd.product_name, tp.total_sold, pd.unit_price FROM top_products tp JOIN product_details pd ON tp.product_id = pd.product_id;";
    let sql_len_2 = sql_2.len();
    group.throughput(Throughput::Elements(sql_len_2 as u64));
    group.bench_with_input(
        BenchmarkId::new("big sql parser 2", sql_len_2),
        sql_2,
        |b, i| {
            b.iter(|| {
                parser.parse(black_box(&i)).unwrap();
            });
        },
    );

    let sql_3 = "SELECT order_id, ROUND(SUM(quantity * unit_price * (1 - discount)) * (1 + tax_rate), 2) AS net_total, LOG(2, ABS(SUM(quantity))) AS log_qty, POWER(SUM(quantity), 2) AS squared_qty FROM order_items GROUP BY order_id;";
    let sql_len_3 = sql_3.len();
    group.throughput(Throughput::Elements(sql_len_3 as u64));
    group.bench_with_input(
        BenchmarkId::new("big sql parser 3", sql_len_3),
        sql_3,
        |b, i| {
            b.iter(|| {
                parser.parse(black_box(&i)).unwrap();
            });
        },
    );

    let sql_4 = "insert into a (user_id, user_name, email) values (123, 'John Doe', 'john@example.com');".repeat(100000);
    let sql_len_4 = sql_4.len();
    group.throughput(Throughput::Elements(sql_len_4 as u64));
    group.bench_with_input(
        BenchmarkId::new("big sql parser 4", sql_len_4),
        &sql_4,
        |b, i| {
            b.iter(|| {
                parser.parse(black_box(&i)).unwrap();
            });
        },
    );
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
