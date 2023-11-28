use std::sync::{
    atomic::{AtomicI64, Ordering},
    Arc,
};

use anyhow::{anyhow, Context, Result};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Utc};
use maud::html;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};

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
        .route("/", get(home))
        .route("/api/beat", post(beat))
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
    axum::Server::bind(&([0, 0, 0, 0], port).into())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

struct AppState {
    pool: SqlitePool,
    /// Longest absence in seconds
    longest_absence: AtomicI64,
    start_time: DateTime<Utc>,
}

const CSS: &str = r#"
html {
    font-family: 'Chivo Mono', monospace;
    font-weight: 300;
    background-color: #ffd1dc;
}

li {
    list-style: none;
}

.small {
font-size: 0.7rem;
}
"#;

async fn home(State(state): State<Arc<AppState>>) -> Result<Html<String>, AppError> {
    let Some(last_beat) = sqlx::query!("select * from beats order by id desc limit 1")
        .fetch_optional(&state.pool)
        .await?
    else {
        return Err(anyhow!("there are no heartbeats yet :3").into());
    };

    let last_beat_time = last_beat.timestamp.and_utc();
    let now = Utc::now();

    let dur = (now - last_beat_time).num_seconds();
    state.longest_absence.fetch_max(dur, Ordering::Relaxed);

    let since_last_beat = format_relative(dur);
    let longest_absence = format_relative(state.longest_absence.load(Ordering::Relaxed));

    let total_beats = sqlx::query_scalar!("select count(*) from beats")
        .fetch_one(&state.pool)
        .await?;

    let uptime = format_relative((now - state.start_time).num_seconds());

    let content = html! {
        html {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";

                title {"annie's heartbeat"}

                // TODO og meta tags

                link rel="preconnect" href="https://fonts.googleapis.com";
                link rel="preconnect" href="https://fonts.gstatic.com" crossorigin;
                link href="https://fonts.googleapis.com/css2?family=Chivo+Mono:ital,wght@0,200;0,300;0,400;0,700;1,200;1,300;1,400;1,700&display=swap" rel="stylesheet";
                link href="https://fonts.googleapis.com/css2?family=Inconsolata&display=swap" rel="stylesheet";
                style {(CSS)}
            }
            body {
                p {
                    "this is "
                    a href="https://versary.town" target="_blank" {"my"}
                    " heartbeat service :3" br;
                    "this page displays the last time that i have unlocked/used any of my devices"
                }
                ul {
                    li {
                        "last beat time: "
                            strong {
                                (last_beat_time.format("%Y/%m/%d %H:%M UTC").to_string())
                            }
                    }
                    li {
                        "time since last beat: "
                            strong {
                                (since_last_beat)
                            }
                    }
                    li title="longest absence since the server restarted" {
                        "longest absence: "
                            strong {
                                (longest_absence)
                            }
                    }
                    li {
                        "total beats: "
                            strong {
                                (total_beats)
                            }
                    }
                    li {
                        "uptime: "
                            strong {
                                (uptime)
                            }
                    }
                }

                p.small {
                    "if this website shows me as active but i'm not replying to your messages,"
                    br;
                    "i'm probably busy doing other things"
                    br;
                    "and i will get back to you once i can dedicate my full attention :3"
                }
            }
        }
    };

    Ok(Html(content.0))
}

async fn beat(headers: HeaderMap, State(state): State<Arc<AppState>>) -> Result<String, AppError> {
    let auth = headers
        .get("Authorization")
        .ok_or_else(|| anyhow!("authorization header is missing"))?
        .to_str()
        .context("failed to read Authorization header as string")?;

    let Some(_device) = sqlx::query!("select * from devices where token = ?", auth)
        .fetch_optional(&state.pool)
        .await?
    else {
        return Err(anyhow!("no device found with this token").into());
    };

    let last_beat = sqlx::query!("select * from beats order by id desc limit 1")
        .fetch_optional(&state.pool)
        .await?;

    let mut tx = state.pool.begin().await?;

    let now = Utc::now();
    let new_beat = sqlx::query!(
        "insert into beats (device, timestamp) values (?, ?)",
        _device.id,
        now,
    )
    .execute(&mut *tx)
    .await?
    .last_insert_rowid();

    sqlx::query!(
        "update devices set beat_count = beat_count + 1 where id = ?",
        _device.id,
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    // if the absence was longer than 1h, log it
    if let Some(last_beat) = last_beat {
        let diff = now - last_beat.timestamp.and_utc();
        if diff.num_hours() >= 1 {
            let duration = diff.num_seconds();
            sqlx::query!(
                "insert into absences (timestamp, duration, begin_beat, end_beat) values (?, ?, ?, ?)",
                now,
                duration,
                last_beat.id,
                new_beat
            )
                .execute(&state.pool)
                .await?;
        }
    }

    Ok(now.timestamp().to_string())
}

pub fn format_relative(secs: i64) -> String {
    if secs == 0 {
        return "just now".into();
    }

    // numbers taken from https://docs.rs/humantime/latest/src/humantime/duration.rs.html#297

    let mut s = String::new();

    let years = secs / 31_557_600; // 365.25d
    let ydays = secs % 31_557_600;
    let months = ydays / 2_630_016; // 30.44d
    let mdays = ydays % 2_630_016;
    let days = mdays / 86400;
    let day_secs = mdays % 86400;
    let hours = day_secs / 3600;
    let minutes = day_secs % 3600 / 60;
    let seconds = day_secs % 60;

    macro_rules! bweh {
        ($name:expr, $dis:literal, $plural:expr) => {
            if $name > 0 {
                s.push_str(&$name.to_string());
                if $plural {
                    s.push(' ');
                }
                s.push_str($dis);
                if $name > 1 && $plural {
                    s.push('s');
                }
                s.push(' ');
            }
        };
    }

    bweh!(years, "year", true);
    bweh!(months, "month", true);
    bweh!(days, "day", true);
    bweh!(hours, "h", false);
    bweh!(minutes, "m", false);
    bweh!(seconds, "s", false);

    s
}

struct AppError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("something went wrong: {}", self.0),
        )
            .into_response()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format() {
        let r = format_relative(10000);
        assert_eq!(r, "2h 46m 40s ");

        let r = format_relative(20000021);
        assert_eq!(r, "7 months 18 days 9h 38m 29s ");

        let r = format_relative(40000021);
        assert_eq!(r, "1 year 3 months 6 days 9h 26m 13s ");

        let r = format_relative(1000000000);
        assert_eq!(r, "31 years 8 months 7 days 19h 17m 52s ");
    }
}
