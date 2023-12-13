use std::sync::Arc;

use anyhow::Result;
use axum::{extract::State, response::Html};
use chrono::{DateTime, Datelike, Days, Duration, Timelike, Utc};
use maud::{html, PreEscaped};

use crate::{
    errors::AppError,
    helpers::{format_relative, RangeDays},
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
    struct Beat {
        device: i64,
        date: DateTime<Utc>,
        timestamp: f64,
    }

    let beats = sqlx::query!("select device, timestamp from beats order by id desc limit 4000")
        .fetch_all(&state.pool)
        .await?
        .into_iter()
        .map(|b| Beat {
            device: b.device,
            date: b.timestamp.and_utc(),
            timestamp: b.timestamp.and_utc().timestamp() as f64,
        })
        .collect::<Vec<_>>();

    let devices = sqlx::query!("select * from devices")
        .fetch_all(&state.pool)
        .await?;

    let Some(oldest) = beats.last() else {
        return Ok(PreEscaped("Not enough beats".to_string()));
    };

    let now = Utc::now();
    let range = RangeDays::new(oldest.date, now);

    let diff = now.timestamp() as f64 - oldest.timestamp;

    let pos = |timestamp: f64| 100.0 * (timestamp - oldest.timestamp) / diff;

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
                        span.hours style={"left: "(pos(day.timestamp() as f64))"%;"} { (day.format("%m/%d").to_string()) }
                    }

                    // TODO get the ones before the previous day

                    @for day in range {
                        @if oldest.date < day && day < now {
                            span.dots style={"left: "(pos(day.timestamp() as f64))"%;"} { }
                        }

                        @let six = day.with_hour(6).unwrap();
                        @if oldest.date < six && six < now {
                            span.dots style={"left: "(pos(six.timestamp() as f64))"%;"} { }
                        }

                        @let twelve = day.with_hour(12).unwrap();
                        @if oldest.date < twelve && twelve < now {
                            span.dots style={"left: "(pos(twelve.timestamp() as f64))"%;"} { }
                        }

                        @let eight = day.with_hour(18).unwrap();
                        @if oldest.date < eight && eight < now {
                            span.dots style={"left: "(pos(eight.timestamp() as f64))"%;"} { }
                        }
                    }
                }
                @for device in &devices {
                    .line {
                        @for beat in beats.iter().filter(|b| b.device == device.id) {
                            // TODO
                            span.beat style={"left: "(pos(beat.timestamp))"%;"} title=(beat.date.format("%Y/%m/%d %H:%M UTC").to_string())  { }
                        }
                    }
                }
            }
        }
    })
}
