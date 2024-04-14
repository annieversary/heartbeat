use anyhow::Result;
use chrono::NaiveDateTime;
use sqlx::{Executor, Sqlite};

pub struct Beat {
    pub id: i64,
    pub device: i64,
    pub timestamp: NaiveDateTime,
}

impl Beat {
    #[allow(dead_code)]
    pub async fn count<'c, E>(executor: E) -> Result<i32>
    where
        E: Executor<'c, Database = Sqlite>,
    {
        let count = sqlx::query_scalar!("select count(*) from beats")
            .fetch_one(executor)
            .await?;
        Ok(count)
    }

    pub async fn create<'c, E>(mut self, pool: E) -> Result<Self>
    where
        E: Executor<'c, Database = Sqlite>,
    {
        let id = sqlx::query!(
            "insert into beats (device, timestamp) values (?, ?)",
            self.device,
            self.timestamp,
        )
        .execute(pool)
        .await?
        .last_insert_rowid();

        self.id = id;

        Ok(self)
    }

    pub async fn last_beat<'c, E>(executor: E) -> Result<Option<Self>>
    where
        E: Executor<'c, Database = Sqlite>,
    {
        let last_beat = sqlx::query_as!(Self, "select * from beats order by id desc limit 1")
            .fetch_optional(executor)
            .await?;

        Ok(last_beat)
    }
}
