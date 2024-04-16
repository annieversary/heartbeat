use std::sync::{atomic::Ordering, Arc};

use anyhow::Result;
use axum::{extract::State, response::Html};
use chrono::Utc;
use maud::html;

use crate::{
    beat::Beat, errors::AppError, helpers::format_relative, html::base_template, AppState,
};

pub async fn home(State(state): State<Arc<AppState>>) -> Result<Html<String>, AppError> {
    let first_beat = Beat::first_beat(&state.pool)
        .await?
        .ok_or_else(|| AppError::html_from_str("there are no heartbeats yet :3"))?;
    let last_beat = Beat::last_beat(&state.pool)
        .await?
        .ok_or_else(|| AppError::html_from_str("there are no heartbeats yet :3"))?;

    let total_beats = Beat::count(&state.pool).await?;

    let last_beat_time = last_beat.timestamp.and_utc();
    let first_beat_time = first_beat.timestamp.and_utc();
    let now = Utc::now();

    let dur = (now - last_beat_time).num_seconds();
    state.longest_absence.fetch_max(dur, Ordering::Relaxed);

    let active = dur < 60 * 10; // 10 mins

    let content = html! {
        p {
            "this is "
            a href="https://versary.town" target="_blank" {"my"}
            " heartbeat service :3" br;
            "this page displays the last time that i have unlocked/used any of my devices"
        }
        ul {
            h4 {
                "status: "
                @if active {
                    span.active {
                        "active"
                    }
                } @else {
                    span.inactive {
                        "inactive"
                    }
                }
            }
            li {
                "last beat: "
                    strong {
                        (last_beat_time.format("%Y/%m/%d %H:%M UTC").to_string())
                    }
            }
            li {
                "time since last beat: "
                    strong {
                        (format_relative(dur))
                    }
            }

            h4 { "stats" }
            li title="longest absence since the server restarted" {
                "longest absence: "
                    strong {
                        (format_relative(state.longest_absence.load(Ordering::Relaxed)))
                    }
            }
            li {
                "total beats: "
                    strong {
                        (total_beats)
                    }
            }
            li {
                "first beat: "
                    strong {
                        (first_beat_time.format("%Y/%m/%d %H:%M UTC").to_string())
                    }
            }
            li {
                "server uptime: "
                    strong {
                        (format_relative((now - state.start_time).num_seconds()))
                    }
            }
        }

        @if active {
            p.small {
                "im active right now! if i'm not replying to your messages,"
                br;
                "i'm probably busy doing other things"
                br;
                "and i will get back to you once i can dedicate my full attention to you :3"
            }
        } @else if dur > 60 * 60 * 4 {
            p.small {
                "i've been inactive for more than 4 hours, which probably means im asleep,"
                br;
                "even if it's a weird time for my current timezone."
                br;
                "i have a "
                a href="https://en.wikipedia.org/wiki/Non-24-hour_sleep%E2%80%93wake_disorder" target="_blank" {
                    "sleep disorder"
                }
            }
        }
    };

    let content = base_template(content);

    Ok(Html(content.0))
}

#[cfg(test)]
mod tests {
    use crate::{device::Device, testing::init_state};

    use super::*;
    use ::axum_test::TestServer;
    use assertables::*;
    use axum::{routing::post, Router};
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
            .route("/", post(home))
            .with_state(state.clone());
        let server = TestServer::new(app).unwrap();

        (server, state)
    }

    #[tokio::test]
    async fn doesnt_panic_with_no_pings() -> Result<()> {
        let (server, _state) = base().await;

        let response = server.post("/").await;

        response.assert_status_ok();
        assert_contains!(response.text(), "no heartbeats yet");

        Ok(())
    }

    #[tokio::test]
    async fn works() -> Result<()> {
        for num in [1, 3, 5, 200] {
            let (server, state) = base().await;

            for i in 0..num {
                Beat {
                    id: 0,
                    device: 1,
                    timestamp: (Utc::now() - TimeDelta::days(i)).naive_utc(),
                }
                .create(&state.pool)
                .await?;
            }

            let response = server.post("/").await;

            response.assert_status_ok();
            assert_contains!(
                response.text(),
                &format!("total beats: <strong>{num}</strong>")
            );
        }

        Ok(())
    }

    #[tokio::test]
    async fn is_inactive() -> Result<()> {
        let (server, state) = base().await;

        Beat {
            id: 0,
            device: 1,
            timestamp: (Utc::now() - TimeDelta::minutes(11)).naive_utc(),
        }
        .create(&state.pool)
        .await?;

        let response = server.post("/").await;

        response.assert_status_ok();
        assert_contains!(
            response.text(),
            "status: <span class=\"inactive\">inactive</span>"
        );

        Ok(())
    }

    #[tokio::test]
    async fn is_active() -> Result<()> {
        let (server, state) = base().await;

        Beat {
            id: 0,
            device: 1,
            timestamp: (Utc::now() - TimeDelta::minutes(9)).naive_utc(),
        }
        .create(&state.pool)
        .await?;

        let response = server.post("/").await;

        response.assert_status_ok();
        assert_contains!(
            response.text(),
            "status: <span class=\"active\">active</span>"
        );

        Ok(())
    }
}
