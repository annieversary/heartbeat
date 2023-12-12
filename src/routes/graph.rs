use std::sync::Arc;

use anyhow::{anyhow, Result};
use axum::{extract::State, response::Html};
use chrono::{DateTime, Datelike, Days, Duration, Timelike, Utc};
use maud::{html, PreEscaped};
use plotlib::{
    page::Page,
    repr::Plot,
    style::{PointMarker, PointStyle},
    view::ContinuousView,
};

use crate::{
    errors::AppError,
    helpers::{format_relative, RangeDays},
    html::base_template,
    AppState,
};

pub async fn graph(State(state): State<Arc<AppState>>) -> Result<Html<String>, AppError> {
    let content = html! {
        h1 { "absences" }
        (absences_graph(&state).await?)

        h1 { "recent beats" }
        .recent-beats {
            (recent_beats(&state).await?)
        }
    };
    let content = base_template(content);

    Ok(Html(content.0))
}

async fn absences_graph(state: &AppState) -> Result<PreEscaped<String>, AppError> {
    #[derive(Debug)]
    struct Absence {
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        duration: i64,
    }

    impl Absence {
        fn desc(&self) -> String {
            format!(
                "From {} to {} of {}",
                self.start.format("%Y/%m/%d %H:%M UTC"),
                self.end.format("%Y/%m/%d %H:%M UTC"),
                format_relative(self.duration)
            )
        }
    }

    let absences = sqlx::query!(
        "select * from absences where duration > ? order by id desc limit 1000",
        0 //60 * 60 * 2
    )
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .map(|a| Absence {
        start: (a.timestamp.and_utc() - Duration::seconds(a.duration)),
        end: a.timestamp.and_utc(),
        duration: a.duration,
    })
    .collect::<Vec<_>>();

    let (Some(newest), Some(oldest)) = (absences.first(), absences.last()) else {
        return Ok(PreEscaped("Not enough absences".to_string()));
    };

    let range = RangeDays::new(oldest.start, newest.end);

    fn date_matches(a: DateTime<Utc>, b: DateTime<Utc>) -> bool {
        a.day() == b.day() && a.month() == b.month() && a.year() == b.year()
    }

    // get absences that start or end on this day
    let absences_on = |d: DateTime<Utc>| {
        let mut a = absences
            .iter()
            .filter(|abs| {
                let t1 = abs.start;
                let t2 = abs.end;
                date_matches(t1, d) || date_matches(t2, d)
            })
            .collect::<Vec<_>>();
        a.sort_unstable_by_key(|abs| abs.start);
        a
    };

    fn pos(date: DateTime<Utc>) -> f32 {
        100.0 * (date.hour() as f32 * 60.0 + date.minute() as f32) / (24.0 * 60.0)
    }

    // TODO this graph breaks if there's an absence that spans 3 days
    // well, i think its gonna work fine, but we're not gonna have the middle day filled
    Ok(html! {
        .absences {
            @for date in range.rev() {
                .line {
                    .date {
                        (date.format("%Y/%m/%d").to_string())
                    }

                    .graph {
                        @for i in 0..=24 {
                            @let perc = 100.0 * i as f32 / 24.0;
                            span.dots style={"left: "(perc)"%;"} { }
                        }

                        @for abs in absences_on(date) {
                            // line between
                            @let start = if date_matches(abs.start, date) { abs.start } else { date };
                            @let length = if date_matches(abs.end, date) { abs.end - start } else { date.checked_add_days(Days::new(1)).unwrap() - start }.num_seconds() as f32;
                            @let length = 100.0 * (length / (60.0 * 60.0 * 24.0));
                            span.length style={"left: "(pos(start))"%; width: "(length)"%;"} title=(abs.desc()) { }

                            // start
                            @if date_matches(abs.start, date) {
                                span.start style={"left: "(pos(abs.start))"%;"} title=(abs.start.format("%Y/%m/%d %H:%M UTC").to_string()) { }
                            }

                            // end
                            @if date_matches(abs.end, date) {
                                span.end style={"left: "(pos(abs.end))"%;"} title=(abs.end.format("%Y/%m/%d %H:%M UTC").to_string()) { }
                            }
                        }
                    }
                }
            }
        }
    })
}

async fn recent_beats(state: &AppState) -> Result<PreEscaped<String>, AppError> {
    let beats = sqlx::query!("select * from beats order by id desc limit 1000")
        .fetch_all(&state.pool)
        .await?;

    let devices = sqlx::query!("select * from devices")
        .fetch_all(&state.pool)
        .await?;

    let markers = [PointMarker::Square, PointMarker::Circle, PointMarker::Cross];

    let plots = devices.iter().enumerate().map(|(i, dev)| {
        Plot::new(
            beats
                .iter()
                .filter(|b| b.device == dev.id)
                .map(|b| (b.timestamp.and_utc().timestamp() as f64, dev.id as f64))
                .collect(),
        )
        .point_style(
            PointStyle::new()
                .marker(markers[i % markers.len()])
                .colour("#DD3355"),
        )
    });

    let mut v = ContinuousView::new()
        .x_label("timestamp")
        .y_label("device")
        .y_range(0.0, devices.len() as f64);

    // beats are ordered backwards
    if let (Some(a), Some(b)) = (beats.last(), beats.first()) {
        v = v.x_range(
            a.timestamp.and_utc().timestamp() as f64,
            b.timestamp.and_utc().timestamp() as f64,
        );
    }

    for plot in plots {
        v = v.add(plot);
    }

    let Ok(svg) = Page::single(&v).to_svg() else {
        return Err(anyhow!("Failed to convert graph to svg").into());
    };

    Ok(PreEscaped(svg.to_string()))
}
