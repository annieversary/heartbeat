use anyhow::Result;
use chrono::NaiveDateTime;
use sqlx::{Executor, Sqlite};

pub struct Absence {
    pub id: i64,
    pub timestamp: NaiveDateTime,
    pub duration: i64,
    pub begin_beat: i64,
    pub end_beat: i64,
}

impl Absence {
    #[allow(dead_code)]
    pub async fn count<'c, E>(executor: E) -> Result<i32>
    where
        E: Executor<'c, Database = Sqlite>,
    {
        let count = sqlx::query_scalar!("select count(*) from absences")
            .fetch_one(executor)
            .await?;
        Ok(count)
    }

    pub async fn create<'c, E>(mut self, executor: E) -> Result<Self>
    where
        E: Executor<'c, Database = Sqlite>,
    {
        let id = sqlx::query!(
            "insert into absences (timestamp, duration, begin_beat, end_beat) values (?, ?, ?, ?)",
            self.timestamp,
            self.duration,
            self.begin_beat,
            self.end_beat,
        )
        .execute(executor)
        .await?
        .last_insert_rowid();

        self.id = id;

        Ok(self)
    }
}
