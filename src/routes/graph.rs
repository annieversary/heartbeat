use std::sync::Arc;

use anyhow::Result;
use axum::{extract::State, response::Html};
use chrono::{DateTime, Days, Timelike, Utc};
use maud::{html, PreEscaped};

use crate::{
    absence::LongAbsences,
    beat::Beat,
    device::Device,
    errors::AppError,
    helpers::{date_matches, RangeDays},
    html::base_template,
    AppState,
};

pub async fn graph(State(state): State<Arc<AppState>>) -> Result<Html<String>, AppError> {
    let content = html! {
        h1 { "recent beats" }
        (recent_beats(&state).await?)

        h1 { "absences" }
        (absences_graph(&state).await?)
    };
    let content = base_template(content);

    Ok(Html(content.0))
}

async fn absences_graph(state: &AppState) -> Result<PreEscaped<String>, AppError> {
    let absences = LongAbsences::get(&state.pool).await?;

    let range = absences
        .range()
        .ok_or_else(|| AppError::html_from_str("not enough absences :3"))?;

    fn pos(date: DateTime<Utc>) -> f32 {
        100.0 * (date.hour() as f32 * 60.0 + date.minute() as f32) / (24.0 * 60.0)
    }

    // TODO this graph breaks if there's an absence that spans 3 days
    // well, i think its gonna work fine, but we're not gonna have the middle day filled
    Ok(html! {
        .absences {
            .left {
                .line style="color: transparent;" {""}
                @for date in range.clone().rev() {
                    .line {
                        (date.format("%Y/%m/%d").to_string())
                    }
                }
            }
            .right {
                .line {
                    @for i in 0..24 {
                        @let perc = 100.0 * i as f32 / 24.0;
                        span.hours style={"left: "(perc)"%;"} { (i) }
                    }
                }
                @for date in range.rev() {
                    .line {
                        @for i in 0..=24 {
                            @let perc = 100.0 * i as f32 / 24.0;
                            span.dots style={"left: "(perc)"%;"} { }
                        }

                        @for abs in absences.absences_on(date) {
                            // line between
                            @let start = if date_matches(abs.start(), date) { abs.start() } else { date };
                            @let length = if date_matches(abs.end(), date) { abs.end() - start } else { date.checked_add_days(Days::new(1)).unwrap() - start }.num_seconds() as f32;
                            @let length = 100.0 * (length / (60.0 * 60.0 * 24.0));
                            span.length style={"left: "(pos(start))"%; width: "(length)"%;"} title=(abs.desc()) { }

                            // start
                            @if date_matches(abs.start(), date) {
                                span.start style={"left: "(pos(abs.start()))"%;"} title=(abs.start().format("%Y/%m/%d %H:%M UTC").to_string()) { }
                            }

                            // end
                            @if date_matches(abs.end(), date) {
                                span.end style={"left: "(pos(abs.end()))"%;"} title=(abs.end().format("%Y/%m/%d %H:%M UTC").to_string()) { }
                            }
                        }
                    }
                }
            }
        }
    })
}

async fn recent_beats(state: &AppState) -> Result<PreEscaped<String>, AppError> {
    let beats = Beat::get_recent(&state.pool).await?;

    let devices = Device::get_all(&state.pool).await?;

    let oldest = beats
        .last()
        .ok_or_else(|| AppError::html_from_str("not enough beats :3"))?;

    let now = Utc::now();
    let range = RangeDays::new(oldest.date(), now);

    let max_diff = now.timestamp() - oldest.unix_timestamp();

    let pos =
        |timestamp: i64| 100.0 * (timestamp - oldest.unix_timestamp()) as f64 / max_diff as f64;

    Ok(html! {
        .recent-beats {
            .left {
                .line style="color: transparent;" {""}
                @for device in &devices {
                    .line {
                        (device.name)
                    }
                }
            }
            .right {
                .line {
                    @for day in range.clone() {
                        span.hours style={"left: "(pos(day.timestamp()))"%;"} { (day.format("%m/%d").to_string()) }
                    }

                    @for day in range {
                        @if oldest.date() < day && day < now {
                            span.dots style={"left: "(pos(day.timestamp()))"%;"} { }
                        }

                        @let six = day.with_hour(6).unwrap();
                        @if oldest.date() < six && six < now {
                            span.dots style={"left: "(pos(six.timestamp()))"%;"} { }
                        }

                        @let twelve = day.with_hour(12).unwrap();
                        @if oldest.date() < twelve && twelve < now {
                            span.dots style={"left: "(pos(twelve.timestamp()))"%;"} { }
                        }

                        @let eight = day.with_hour(18).unwrap();
                        @if oldest.date() < eight && eight < now {
                            span.dots style={"left: "(pos(eight.timestamp()))"%;"} { }
                        }
                    }
                }
                @for device in &devices {
                    .line {
                        @for beat in beats.iter().filter(|b| b.device == device.id) {
                            // TODO
                            span.beat style={"left: "(pos(beat.unix_timestamp()))"%;"} title=(beat.date().format("%Y/%m/%d %H:%M UTC").to_string())  { }
                        }
                    }
                }
            }
        }
    })
}
