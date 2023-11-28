use std::sync::{atomic::Ordering, Arc};

use anyhow::{anyhow, Result};
use axum::{extract::State, response::Html};
use chrono::Utc;
use maud::html;

use crate::{errors::AppError, helpers::format_relative, AppState};

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

pub async fn home(State(state): State<Arc<AppState>>) -> Result<Html<String>, AppError> {
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
