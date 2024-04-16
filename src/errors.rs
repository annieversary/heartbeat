use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};

pub enum AppError {
    Anyhow(anyhow::Error),
    Html(Html<String>),
}

impl AppError {
    pub fn html_from_str(s: impl ToString) -> Self {
        Self::Html(Html(s.to_string()))
    }
}

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::Anyhow(error) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("something went wrong: {}", error),
            )
                .into_response(),

            AppError::Html(html) => html.into_response(),
        }
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self::Anyhow(err.into())
    }
}
