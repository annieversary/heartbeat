use std::sync::{atomic::Ordering, Arc};

use anyhow::Result;
use axum::extract::State;
use chrono::Utc;

use crate::{absence::Absence, beat::Beat, device::Device, errors::AppError, AppState};

pub async fn beat(State(state): State<Arc<AppState>>, device: Device) -> Result<String, AppError> {
    let last_beat = Beat::last_beat(&state.pool).await?;

    let mut tx = state.pool.begin().await?;

    let now = Utc::now();
    let beat = Beat {
        id: 0, // id is ignored on create
        device: device.id,
        timestamp: now.naive_utc(),
    }
    .create(&mut *tx)
    .await?;

    device.increase_beat_count(1, &mut *tx).await?;

    tx.commit().await?;

    // update longest absence
    if let Some(last_beat) = last_beat {
        let diff = now - last_beat.timestamp.and_utc();
        let duration = diff.num_seconds();

        // update longest absence in state
        state.longest_absence.fetch_max(duration, Ordering::Relaxed);

        // if the absence was longer than 1h, log it
        if diff.num_hours() >= 1 {
            Absence {
                id: 0,
                timestamp: now.naive_utc(),
                duration,
                begin_beat: last_beat.id,
                end_beat: beat.id,
            }
            .create(&state.pool)
            .await?;
        }
    }

    Ok(now.timestamp().to_string())
}

#[cfg(test)]
mod tests {
    use crate::{beat::Beat, device::Device, testing::init_state};

    use super::*;
    use ::axum_test::TestServer;
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
            .route("/api/beat", post(beat))
            .with_state(state.clone());
        let server = TestServer::new(app).unwrap();

        (server, state)
    }

    async fn request(server: &TestServer) -> Result<TestResponse> {
        let response = server
            .post("/api/beat")
            .add_header(
                HeaderName::from_bytes(b"Authorization")?,
                HeaderValue::from_str("my_token")?,
            )
            .await;
        Ok(response)
    }

    #[tokio::test]
    async fn can_create_beats() -> Result<()> {
        let (server, state) = base().await;

        assert_eq!(0, Beat::count(&state.pool).await?);
        assert_eq!(
            0,
            Device::get_by_auth("my_token", &state.pool)
                .await?
                .unwrap()
                .beat_count
        );

        let response = request(&server).await?;

        response.assert_status_ok();

        assert_eq!(1, Beat::count(&state.pool).await?);
        assert_eq!(
            1,
            Device::get_by_auth("my_token", &state.pool)
                .await?
                .unwrap()
                .beat_count
        );
        assert_eq!(0, Absence::count(&state.pool).await?);

        Ok(())
    }

    #[tokio::test]
    async fn doesnt_create_an_absence_if_under_1h() -> Result<()> {
        let (server, state) = base().await;

        // make a beat from 10 seconds ago. since this is the most recent one, we'll use this and not create an absence
        let time = Utc::now() + TimeDelta::new(-10, 0).unwrap();
        Beat {
            id: 0,
            device: 1,
            timestamp: time.naive_utc(),
        }
        .create(&state.pool)
        .await?;

        // make a beat from 1 day ago
        let time = Utc::now() + TimeDelta::days(-1);
        Beat {
            id: 0,
            device: 1,
            timestamp: time.naive_utc(),
        }
        .create(&state.pool)
        .await?;

        assert_eq!(0, Absence::count(&state.pool).await?);

        let response = request(&server).await?;

        response.assert_status_ok();

        assert_eq!(0, Absence::count(&state.pool).await?);

        Ok(())
    }

    #[tokio::test]
    async fn creates_an_absence_if_over_1h() -> Result<()> {
        let (server, state) = base().await;

        // make a beat from a day ago
        let time = Utc::now() + TimeDelta::days(-1);
        Beat {
            id: 0,
            device: 1,
            timestamp: time.naive_utc(),
        }
        .create(&state.pool)
        .await?;

        assert_eq!(0, Absence::count(&state.pool).await?);

        let response = request(&server).await?;

        response.assert_status_ok();

        assert_eq!(1, Absence::count(&state.pool).await?);
        assert_eq!(86400, state.longest_absence.load(Ordering::Relaxed));

        Ok(())
    }
}
