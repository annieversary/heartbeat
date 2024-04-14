#![allow(dead_code)]
use std::sync::{atomic::AtomicI64, Arc};

use chrono::Utc;
use sqlx::sqlite::SqlitePoolOptions;

use crate::AppState;

pub async fn init_state() -> Arc<AppState> {
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(":memory:")
        .await
        .expect("failed to connect to db");

    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("couldn't run migrations");

    Arc::new(AppState {
        pool,
        longest_absence: AtomicI64::new(0),
        start_time: Utc::now(),
    })
}
