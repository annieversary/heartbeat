use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use axum::{extract::State, http::HeaderMap};
use chrono::Utc;

use crate::{errors::AppError, AppState};

pub async fn beat(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
) -> Result<String, AppError> {
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
