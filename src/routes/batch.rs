use std::sync::{atomic::Ordering, Arc};

use anyhow::{anyhow, Result};
use axum::{extract::State, Json};
use chrono::NaiveDateTime;

use crate::{absence::Absence, beat::Beat, device::Device, errors::AppError, AppState};

#[derive(serde::Deserialize, serde::Serialize)]
pub struct BeatBatch {
    timestamps: Vec<NaiveDateTime>,
}

pub async fn batch(
    State(state): State<Arc<AppState>>,
    device: Device,
    Json(BeatBatch { timestamps }): Json<BeatBatch>,
) -> Result<String, AppError> {
    if timestamps.is_empty() {
        return Err(anyhow!("no timestamps provided").into());
    }

    let mut tx = state.pool.begin().await?;

    let ids = Beat::create_many(device.id, &timestamps, &mut *tx).await?;
    device
        .increase_beat_count(ids.len() as i64, &mut *tx)
        .await?;

    let first_timestamp = timestamps.iter().min().unwrap();

    let beats = Beat::get_all_before(first_timestamp, &mut *tx).await?;
    let mut absences = Absence::get_all_before(first_timestamp, &mut *tx).await?;

    let mut idx = 0;
    'out: while idx < absences.len() {
        for timestamp in &timestamps {
            let absence = &absences[idx];
            if absence.contains(&timestamp.and_utc()) {
                absence.delete(&mut *tx).await?;
                absences.remove(idx);

                // TODO what do we do with longest_absence here if this absence was the longest?
                // we dont have the previous longest

                continue 'out;
            }
        }

        idx += 1;
    }

    for window in beats.windows(2) {
        let [last_beat, beat] = window else {
            continue;
        };

        // if there's already an absence between these two beats, skip
        if absences
            .iter()
            .any(|abs| abs.begin_beat == last_beat.id && abs.end_beat == beat.id)
        {
            continue;
        }

        let diff = beat.timestamp.and_utc() - last_beat.timestamp.and_utc();

        // update longest absence in state
        state
            .longest_absence
            .fetch_max(diff.num_seconds(), Ordering::Relaxed);

        // if the absence was longer than 1h, log it
        if diff.num_hours() >= 1 {
            Absence {
                id: 0,
                timestamp: beat.timestamp,
                duration: diff.num_seconds(),
                begin_beat: last_beat.id,
                end_beat: beat.id,
            }
            .create(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;

    Ok(ids.len().to_string())
}

#[cfg(test)]
mod tests {
    use crate::{beat::Beat, device::Device, testing::init_state};

    use super::*;
    use ::axum_test::TestServer;
    use anyhow::Result;
    use axum::{
        http::{HeaderName, HeaderValue},
        routing::post,
        Router,
    };
    use axum_test::TestResponse;
    use chrono::TimeDelta;

    async fn base() -> (TestServer, Arc<AppState>) {
        let state = init_state().await;

        Device {
            id: 1,
            name: "test device".to_string(),
            token: "my_token".to_string(),
            beat_count: 0,
        }
        .create(&state.pool)
        .await
        .unwrap();

        let app = Router::new()
            .route("/api/batch", post(batch))
            .with_state(state.clone());
        let server = TestServer::new(app).unwrap();

        (server, state)
    }

    async fn request(server: &TestServer, timestamps: Vec<NaiveDateTime>) -> Result<TestResponse> {
        let response = server
            .post("/api/batch")
            .add_header(
                HeaderName::from_bytes(b"Authorization")?,
                HeaderValue::from_str("my_token")?,
            )
            .json(&BeatBatch { timestamps })
            .await;
        Ok(response)
    }

    #[tokio::test]
    async fn can_create_in_batch() -> Result<()> {
        let (server, state) = base().await;

        let response = request(
            &server,
            vec![
                (Utc::now() - TimeDelta::days(10)).naive_utc(),
                (Utc::now() - TimeDelta::days(9)).naive_utc(),
                (Utc::now() - TimeDelta::days(8)).naive_utc(),
            ],
        )
        .await?;

        response.assert_status_ok();
        assert_eq!(3, Beat::count(&state.pool).await?);

        Ok(())
    }

    #[tokio::test]
    async fn creates_absences() -> Result<()> {
        let (server, state) = base().await;

        let response = request(
            &server,
            vec![
                (Utc::now() - TimeDelta::days(10)).naive_utc(),
                (Utc::now() - TimeDelta::days(9)).naive_utc(),
                (Utc::now() - TimeDelta::days(8)).naive_utc(),
            ],
        )
        .await?;

        response.assert_status_ok();
        assert_eq!(2, Absence::count(&state.pool).await?);

        Ok(())
    }

    #[tokio::test]
    async fn doesnt_create_duplicated_absences() -> Result<()> {
        let (server, state) = base().await;

        Beat {
            id: 0,
            device: 1,
            timestamp: (Utc::now() - TimeDelta::days(5)).naive_utc(),
        }
        .create(&state.pool)
        .await?;
        Beat {
            id: 0,
            device: 1,
            timestamp: (Utc::now() - TimeDelta::days(3)).naive_utc(),
        }
        .create(&state.pool)
        .await?;
        Absence {
            id: 0,
            timestamp: (Utc::now() - TimeDelta::days(3)).naive_utc(),
            duration: 5000,
            begin_beat: 1,
            end_beat: 2,
        }
        .create(&state.pool)
        .await?;

        let response = request(
            &server,
            vec![
                (Utc::now() - TimeDelta::days(10)).naive_utc(),
                (Utc::now() - TimeDelta::days(9)).naive_utc(),
            ],
        )
        .await?;

        response.assert_status_ok();
        // there should be an absence between 10 and 9, 9 and 5, 5 and 3
        assert_eq!(3, Absence::count(&state.pool).await?);

        Ok(())
    }

    #[tokio::test]
    async fn deletes_interrupted_absences() -> Result<()> {
        let (server, state) = base().await;

        Beat {
            id: 0,
            device: 1,
            timestamp: (Utc::now() - TimeDelta::seconds(5000)).naive_utc(),
        }
        .create(&state.pool)
        .await?;
        Beat {
            id: 0,
            device: 1,
            timestamp: Utc::now().naive_utc(),
        }
        .create(&state.pool)
        .await?;
        Absence {
            id: 0,
            timestamp: Utc::now().naive_utc(),
            duration: 5000,
            begin_beat: 1,
            end_beat: 2,
        }
        .create(&state.pool)
        .await?;

        let response = request(
            &server,
            vec![(Utc::now() - TimeDelta::seconds(2500)).naive_utc()],
        )
        .await?;

        response.assert_status_ok();
        assert_eq!(0, Absence::count(&state.pool).await?);

        Ok(())
    }

    #[tokio::test]
    async fn doesnt_delete_uninterrupted_absences() -> Result<()> {
        let (server, state) = base().await;

        Beat {
            id: 0,
            device: 1,
            timestamp: (Utc::now() - TimeDelta::seconds(5000)).naive_utc(),
        }
        .create(&state.pool)
        .await?;
        Beat {
            id: 0,
            device: 1,
            timestamp: Utc::now().naive_utc(),
        }
        .create(&state.pool)
        .await?;
        Absence {
            id: 0,
            timestamp: Utc::now().naive_utc(),
            duration: 5000,
            begin_beat: 1,
            end_beat: 2,
        }
        .create(&state.pool)
        .await?;

        let response = request(
            &server,
            vec![(Utc::now() - TimeDelta::seconds(5020)).naive_utc()],
        )
        .await?;

        response.assert_status_ok();
        assert_eq!(1, Absence::count(&state.pool).await?);

        Ok(())
    }
}
