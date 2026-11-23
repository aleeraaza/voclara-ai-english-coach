//! Minute wallet against a real Postgres (skipped when TEST_DATABASE_URL is unset).
#![allow(clippy::unwrap_used)]

mod common;

use sqlx::PgPool;
use uuid::Uuid;
use voclara_api::{
    domain::minutes::{self, GrantReason},
    error::AppError,
};

async fn new_user(db: &PgPool) -> Uuid {
    sqlx::query_scalar("INSERT INTO users (phone) VALUES ($1) RETURNING id")
        .bind(common::random_phone())
        .fetch_one(db)
        .await
        .unwrap()
}

async fn new_session(db: &PgPool, user: Uuid) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO sessions (user_id, mode, provider, prompt_version, max_secs)
         VALUES ($1, 'talk', 'fake', 'tutor-v1', 600) RETURNING id",
    )
    .bind(user)
    .fetch_one(db)
    .await
    .unwrap()
}

#[tokio::test]
async fn free_grant_happens_only_once() {
    let Some(db) = common::test_db().await else {
        return;
    };
    let user = new_user(&db).await;
    let mut conn = db.acquire().await.unwrap();
    assert!(minutes::grant_free(&mut conn, user).await.unwrap());
    assert!(!minutes::grant_free(&mut conn, user).await.unwrap());
    assert_eq!(minutes::balance(&db, user).await.unwrap(), 600);
}

#[tokio::test]
async fn debit_reduces_balance_and_refuses_overdraft() {
    let Some(db) = common::test_db().await else {
        return;
    };
    let user = new_user(&db).await;
    let mut conn = db.acquire().await.unwrap();
    minutes::grant_free(&mut conn, user).await.unwrap();
    minutes::grant(&mut conn, user, 120, GrantReason::Purchase, None)
        .await
        .unwrap();

    let s1 = new_session(&db, user).await;
    assert_eq!(
        minutes::debit_seconds(&db, user, s1, 700).await.unwrap(),
        20
    );

    let s2 = new_session(&db, user).await;
    let err = minutes::debit_seconds(&db, user, s2, 21).await.unwrap_err();
    assert!(matches!(err, AppError::InsufficientMinutes));
    assert_eq!(
        minutes::balance(&db, user).await.unwrap(),
        20,
        "failed debit changes nothing"
    );
}

#[tokio::test]
async fn a_session_is_charged_only_once() {
    let Some(db) = common::test_db().await else {
        return;
    };
    let user = new_user(&db).await;
    minutes::grant_free(&mut db.acquire().await.unwrap(), user)
        .await
        .unwrap();
    let s = new_session(&db, user).await;
    minutes::debit_seconds(&db, user, s, 100).await.unwrap();
    let err = minutes::debit_seconds(&db, user, s, 100).await.unwrap_err();
    assert!(matches!(err, AppError::Conflict(_)));
    assert_eq!(minutes::balance(&db, user).await.unwrap(), 500);
}

#[tokio::test]
async fn non_positive_amounts_are_rejected() {
    let Some(db) = common::test_db().await else {
        return;
    };
    let user = new_user(&db).await;
    let s = new_session(&db, user).await;
    assert!(matches!(
        minutes::debit_seconds(&db, user, s, 0).await,
        Err(AppError::BadRequest(_))
    ));
    let mut conn = db.acquire().await.unwrap();
    assert!(matches!(
        minutes::grant(&mut conn, user, -5, GrantReason::Adjustment, None).await,
        Err(AppError::BadRequest(_))
    ));
}

/// The race that matters: many sessions ending at the same moment must never
/// spend more than the wallet holds. 10 debits of 100 s against 600 s means
/// exactly 6 succeed and the balance ends at 0, never below.
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn concurrent_debits_never_overdraw() {
    let Some(db) = common::test_db().await else {
        return;
    };
    let user = new_user(&db).await;
    minutes::grant_free(&mut db.acquire().await.unwrap(), user)
        .await
        .unwrap();

    let mut sessions = Vec::new();
    for _ in 0..10 {
        sessions.push(new_session(&db, user).await);
    }

    // Start all debits at once. `db.clone()` is cheap (the pool is an Arc
    // inside); each task needs its own owned handle because a spawned task
    // must be 'static and cannot borrow `db` from this function.
    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(sessions.len()));
    let mut handles = Vec::new();
    for s in sessions {
        let db = db.clone();
        let barrier = barrier.clone();
        handles.push(tokio::spawn(async move {
            barrier.wait().await;
            minutes::debit_seconds(&db, user, s, 100).await
        }));
    }

    let (mut ok, mut refused) = (0, 0);
    for h in handles {
        match h.await.unwrap() {
            Ok(_) => ok += 1,
            Err(AppError::InsufficientMinutes) => refused += 1,
            Err(other) => panic!("unexpected error: {other:?}"),
        }
    }
    assert_eq!((ok, refused), (6, 4));
    assert_eq!(minutes::balance(&db, user).await.unwrap(), 0);
}

#[tokio::test]
async fn drill_allowance_counts_todays_drills_only() {
    let Some(db) = common::test_db().await else {
        return;
    };
    let user = new_user(&db).await;
    assert_eq!(
        minutes::drill_secs_left_today(&db, user).await.unwrap(),
        120
    );

    sqlx::query(
        "INSERT INTO sessions (user_id, mode, provider, prompt_version, max_secs, seconds_used, started_at)
         VALUES ($1, 'drill', 'fake', 'drill-v1', 120, 45, now()),
                ($1, 'drill', 'fake', 'drill-v1', 120, 90, now() - interval '2 days'),
                ($1, 'talk',  'fake', 'tutor-v1', 600, 300, now())",
    )
    .bind(user)
    .execute(&db)
    .await
    .unwrap();
    assert_eq!(minutes::drill_secs_left_today(&db, user).await.unwrap(), 75);
}
