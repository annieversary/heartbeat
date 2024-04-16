use anyhow::Result;
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use sqlx::{Executor, Sqlite};

use crate::helpers::{date_matches, format_relative, RangeDays};

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

pub struct LongAbsences {
    absences: Vec<Absence>,
}

impl LongAbsences {
    pub async fn get<'c, E>(executor: E) -> Result<Self>
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
        Ok(Self { absences })
    }

    /// get a range of days that encompasses all the absences
    pub fn range(&self) -> Option<RangeDays> {
        let newest = self.absences.first()?;
        let oldest = self.absences.last()?;

        Some(RangeDays::new(oldest.start(), newest.end()))
    }

    /// get absences that start or end on this day
    pub fn absences_on(&self, d: DateTime<Utc>) -> Vec<&Absence> {
        let mut a = self
            .absences
            .iter()
            .filter(|abs| {
                let t1 = abs.start();
                let t2 = abs.end();
                date_matches(t1, d) || date_matches(t2, d)
            })
            .collect::<Vec<_>>();
        a.sort_unstable_by_key(|abs| abs.start());
        a
    }
}
