use std::sync::Arc;

use anyhow::{anyhow, Result};
use axum::{extract::State, response::Html};
use maud::{html, PreEscaped};
use plotlib::{
    page::Page,
    repr::Plot,
    style::{PointMarker, PointStyle},
    view::ContinuousView,
};

use crate::{errors::AppError, html::base_template, AppState};

pub async fn graph(State(state): State<Arc<AppState>>) -> Result<Html<String>, AppError> {
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

    let content = html! {
        .graph {
            (PreEscaped(svg.to_string()))
        }
    };
    let content = base_template(content);

    Ok(Html(content.0))
}
