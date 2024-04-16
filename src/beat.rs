use anyhow::Result;
use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::{Executor, Sqlite};

pub struct Beat {
    pub id: i64,
    pub device: i64,
    pub timestamp: NaiveDateTime,
}

impl Beat {
    pub fn date(&self) -> DateTime<Utc> {
        self.timestamp.and_utc()
    }

    pub fn unix_timestamp(&self) -> i64 {
        self.timestamp.and_utc().timestamp()
    }

    pub async fn count<'c, E>(executor: E) -> Result<i32>
    where
        E: Executor<'c, Database = Sqlite>,
    {
        let count = sqlx::query_scalar!("select count(*) from beats")
            .fetch_one(executor)
            .await?;
        Ok(count)
    }

    pub async fn get_recent<'c, E>(executor: E) -> Result<Vec<Self>>
    where
        E: Executor<'c, Database = Sqlite>,
    {
        let beats = sqlx::query_as!(
            Self,
            "select id, device, timestamp from beats order by id desc limit 4000"
        )
        .fetch_all(executor)
        .await?;
        Ok(beats)
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

    pub async fn first_beat<'c, E>(executor: E) -> Result<Option<Self>>
    where
        E: Executor<'c, Database = Sqlite>,
    {
        let last_beat = sqlx::query_as!(Self, "select * from beats order by timestamp asc limit 1")
            .fetch_optional(executor)
            .await?;

        Ok(last_beat)
    }

    pub async fn last_beat<'c, E>(executor: E) -> Result<Option<Self>>
    where
        E: Executor<'c, Database = Sqlite>,
    {
        let last_beat =
            sqlx::query_as!(Self, "select * from beats order by timestamp desc limit 1")
                .fetch_optional(executor)
                .await?;

        Ok(last_beat)
    }
}
