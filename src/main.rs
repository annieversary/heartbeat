use std::net::SocketAddr;
use std::sync::{atomic::AtomicI64, Arc};
use tokio::net::TcpListener;

use axum::{
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Utc};
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};

mod absence;
mod beat;
mod device;
mod errors;
mod helpers;
mod html;
mod routes;
mod testing;

#[tokio::main]
async fn main() {
    // we only care if the error is a line parse
    if let Err(err @ dotenv::Error::LineParse(..)) = dotenv::dotenv() {
        panic!("{:?}", err);
    }

    let db_connection_str = std::env::var("DATABASE_URL").expect("failed to get db url");
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_connection_str)
        .await
        .expect("failed to connect to db");

    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("couldn't run migrations");

    let longest_absence = sqlx::query_scalar!("select MAX(duration) from absences")
        .fetch_optional(&pool)
        .await
        .unwrap_or_default()
        .unwrap_or_default()
        .unwrap_or_default();

    let app = Router::new()
        .route("/", get(routes::home::home))
        .route("/graph", get(routes::graph::graph))
        .route("/report", get(routes::report::report))
        .route("/api/beat", post(routes::beat::beat))
        .route("/api/batch", post(routes::batch::batch))
        .with_state(Arc::new(AppState {
            pool,
            longest_absence: AtomicI64::new(longest_absence as i64),
            start_time: Utc::now(),
        }));

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|a| a.parse().ok())
        .unwrap_or(3000);

    #[cfg(debug_assertions)]
    println!("listening on http://localhost:{port}");

    // run it with hyper on localhost:3000
    let listener = TcpListener::bind(SocketAddr::new([0, 0, 0, 0].into(), port))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

pub struct AppState {
    pool: SqlitePool,
    /// server start time, to keep track of uptime
    start_time: DateTime<Utc>,
    /// Longest absence in seconds
    longest_absence: AtomicI64,
}
