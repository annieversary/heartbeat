use std::sync::Arc;

use anyhow::Result;
use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use sqlx::{Executor, Sqlite};

use crate::AppState;

pub struct Device {
    pub id: i64,
    pub name: String,
    pub token: String,
    pub beat_count: i64,
}

#[async_trait]
impl FromRequestParts<Arc<AppState>> for Device {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let Some(auth) = parts.headers.get("Authorization") else {
            return Err((StatusCode::BAD_REQUEST, "authorization header is missing"));
        };

        let Ok(auth) = auth.to_str() else {
            return Err((
                StatusCode::BAD_REQUEST,
                "failed to read Authorization header as string",
            ));
        };

        let Ok(Some(device)) = Device::get_by_auth(auth, &state.pool).await else {
            return Err((
                StatusCode::UNAUTHORIZED,
                ("no device found with this token"),
            ));
        };

        Ok(device)
    }
}

impl Device {
    pub async fn get_by_auth<'c, E>(auth: &str, executor: E) -> Result<Option<Self>>
    where
        E: Executor<'c, Database = Sqlite>,
    {
        let device = sqlx::query_as!(
            Device,
            "select id as \"id!\", name as \"name!\", token, beat_count from devices where token = ?",
            auth
        )
            .fetch_optional(executor)
            .await?;

        Ok(device)
    }

    #[allow(dead_code)]
    pub async fn create<'c, E>(self, executor: E) -> Result<()>
    where
        E: Executor<'c, Database = Sqlite>,
    {
        sqlx::query!(
            "insert into devices (id, name, token, beat_count) values (?, ?, ?, ?)",
            self.id,
            self.name,
            self.token,
            self.beat_count
        )
        .execute(executor)
        .await?;
        Ok(())
    }

    pub async fn increase_beat_count<'c, E>(&self, executor: E) -> Result<()>
    where
        E: Executor<'c, Database = Sqlite>,
    {
        sqlx::query!(
            "update devices set beat_count = beat_count + 1 where id = ?",
            self.id,
        )
        .execute(executor)
        .await?;

        Ok(())
    }
}
