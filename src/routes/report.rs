use std::sync::Arc;

use anyhow::Result;
use axum::{extract::State, response::Html};
use chrono::Duration;
use maud::html;

use crate::{errors::AppError, helpers::format_relative, html::base_template, AppState};

pub async fn report(State(state): State<Arc<AppState>>) -> Result<Html<String>, AppError> {
    struct Absence {
        start: String,
        end: String,
        length: String,
    }

    let absences: Vec<Absence> = sqlx::query!("select * from absences order by id desc limit 1000")
        .fetch_all(&state.pool)
        .await?
        .into_iter()
        .map(|a| Absence {
            end: a
                .timestamp
                .and_utc()
                .format("%Y/%m/%d %H:%M UTC")
                .to_string(),
            start: (a.timestamp.and_utc() - Duration::seconds(a.duration))
                .format("%Y/%m/%d %H:%M UTC")
                .to_string(),
            length: format_relative(a.duration),
        })
        .collect();

    let content = html! {
        ul {
            @for a in &absences {
                li {
                    "Absence from "(a.start)" to "(a.end)" of "(a.length)
                }
            }
        }
    };
    let content = base_template(content);

    Ok(Html(content.0))
}
