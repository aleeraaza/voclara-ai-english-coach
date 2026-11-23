//! Runs the migrations against a real Postgres when `TEST_DATABASE_URL` is set
//! (CI always sets it). Skips quietly otherwise so `cargo test` works offline.

mod common;

use sqlx::{PgPool, Row};

const EXPECTED_TABLES: &[&str] = &[
    "users",
    "refresh_tokens",
    "sessions",
    "session_costs",
    "scores",
    "pronunciation_results",
    "minute_ledger",
    "entitlements",
    "purchases",
    "streaks",
    "push_tokens",
    "jobs",
    "otp_codes",
];

#[tokio::test]
async fn migrations_create_all_tables() {
    let Some(url) = common::test_database_url() else {
        return;
    };
    let db = PgPool::connect(&url).await.unwrap();
    voclara_api::MIGRATOR.run(&db).await.unwrap();

    for table in EXPECTED_TABLES {
        let row = sqlx::query("SELECT to_regclass($1) IS NOT NULL AS present")
            .bind(format!("public.{table}"))
            .fetch_one(&db)
            .await
            .unwrap();
        assert!(row.get::<bool, _>("present"), "table {table} missing");
    }

    // The balance view must exist and return 0 for a user with no ledger rows.
    let user_id: uuid::Uuid =
        sqlx::query_scalar("INSERT INTO users (phone) VALUES ($1) RETURNING id")
            .bind(format!(
                "+92300{}",
                &uuid::Uuid::new_v4().simple().to_string()[..7]
            ))
            .fetch_one(&db)
            .await
            .unwrap();
    let balance: i64 = sqlx::query_scalar(
        "SELECT COALESCE((SELECT balance_secs FROM minute_balances WHERE user_id = $1), 0)",
    )
    .bind(user_id)
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(balance, 0);

    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(&db)
        .await
        .unwrap();
}
