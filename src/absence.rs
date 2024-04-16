use anyhow::Result;
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use sqlx::{Executor, Sqlite};

use crate::helpers::format_relative;

pub struct Absence {
    pub id: i64,
    pub timestamp: NaiveDateTime,
    pub duration: i64,
    pub begin_beat: i64,
    pub end_beat: i64,
}

impl Absence {
    pub fn start(&self) -> DateTime<Utc> {
        self.timestamp.and_utc() - Duration::seconds(self.duration)
    }
    pub fn end(&self) -> DateTime<Utc> {
        self.timestamp.and_utc()
    }

    pub fn desc(&self) -> String {
        format!(
            "From {} to {} of {}",
            self.start().format("%Y/%m/%d %H:%M UTC"),
            self.end().format("%Y/%m/%d %H:%M UTC"),
            format_relative(self.duration)
        )
    }

    pub async fn long_absences<'c, E>(executor: E) -> Result<Vec<Self>>
    where
        E: Executor<'c, Database = Sqlite>,
    {
        let absences = sqlx::query_as!(
            Absence,
            "select * from absences where duration > ? order by id desc",
            60 * 60 // 1h
        )
        .fetch_all(executor)
        .await?;
        Ok(absences)
    }

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
