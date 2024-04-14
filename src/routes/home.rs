use std::sync::{atomic::Ordering, Arc};

use anyhow::{anyhow, Result};
use axum::{extract::State, response::Html};
use chrono::Utc;
use maud::html;

use crate::{
    beat::Beat, errors::AppError, helpers::format_relative, html::base_template, AppState,
};

pub async fn home(State(state): State<Arc<AppState>>) -> Result<Html<String>, AppError> {
    let Some(last_beat) = Beat::last_beat(&state.pool).await? else {
        return Err(anyhow!("there are no heartbeats yet :3").into());
    };

    let Some(first_beat) = Beat::first_beat(&state.pool).await? else {
        return Err(anyhow!("there are no heartbeats yet :3").into());
    };

    let last_beat_time = last_beat.timestamp.and_utc();
    let first_beat_time = first_beat.timestamp.and_utc();
    let now = Utc::now();

    let dur = (now - last_beat_time).num_seconds();
    state.longest_absence.fetch_max(dur, Ordering::Relaxed);

    let active = dur < 60 * 10; // 10 mins
    let since_last_beat = format_relative(dur);
    let longest_absence = format_relative(state.longest_absence.load(Ordering::Relaxed));

    let total_beats = Beat::count(&state.pool).await?;

    let uptime = format_relative((now - state.start_time).num_seconds());

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
                        (since_last_beat)
                    }
            }

            h4 { "stats" }
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
                "first beat: "
                    strong {
                        (first_beat_time.format("%Y/%m/%d %H:%M UTC").to_string())
                    }
            }
            li {
                "server uptime: "
                    strong {
                        (uptime)
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
