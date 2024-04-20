use anyhow::Result;
use chrono::{DateTime, NaiveDateTime, Utc};
use sqlx::{Executor, QueryBuilder, Row, Sqlite};

#[derive(Debug)]
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

    /// Gets the last 4000 most recent beats
    pub async fn get_recent<'c, E>(executor: E) -> Result<Vec<Self>>
    where
        E: Executor<'c, Database = Sqlite>,
    {
        let beats = sqlx::query_as!(
            Self,
            "select id, device, timestamp from beats order by timestamp desc limit 4000"
        )
        .fetch_all(executor)
        .await?;
        Ok(beats)
    }

    pub async fn get_all_before<'c, E>(timestamp: &NaiveDateTime, executor: E) -> Result<Vec<Self>>
    where
        E: Executor<'c, Database = Sqlite>,
    {
        let beats = sqlx::query_as!(
            Self,
            "select id, device, timestamp from beats where timestamp >= ? order by timestamp asc",
            timestamp
        )
        .fetch_all(executor)
        .await?;
        Ok(beats)
    }

    #[allow(dead_code)]
    pub async fn get_by_ids<'c, E>(ids: &[i64], executor: E) -> Result<Vec<Self>>
    where
        E: Executor<'c, Database = Sqlite>,
    {
        // https://github.com/launchbadge/sqlx/issues/294
        let mut query_builder: QueryBuilder<Sqlite> =
            QueryBuilder::new("select id, device, timestamp from beats where id in (");

        let mut separated = query_builder.separated(", ");
        for id in ids {
            separated.push_bind(id);
        }
        separated.push_unseparated(")");

        let beats = query_builder.build().fetch_all(executor).await?;
        let beats = beats
            .into_iter()
            .filter_map(|row| {
                Some(Beat {
                    id: row.try_get(0).ok()?,
                    device: row.try_get(1).ok()?,
                    timestamp: row.try_get(2).ok()?,
                })
            })
            .collect();

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

    pub async fn create_many<'c, E>(
        device_id: i64,
        timestamps: &[NaiveDateTime],
        executor: E,
    ) -> Result<Vec<i64>>
    where
        E: Executor<'c, Database = Sqlite>,
    {
        // https://github.com/launchbadge/sqlx/issues/294
        let mut query_builder: QueryBuilder<Sqlite> =
            QueryBuilder::new("insert into beats (device, timestamp) ");

        query_builder.push_values(timestamps.iter(), |mut b, timestamp| {
            b.push_bind(device_id).push_bind(timestamp);
        });
        query_builder.push("returning id");

        let ids = query_builder.build().fetch_all(executor).await?;
        let ids = ids
            .into_iter()
            .filter_map(|row| row.try_get(0).ok())
            .collect();

        Ok(ids)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{device::Device, testing::init_state};

    use anyhow::Result;
    use chrono::TimeDelta;

    #[tokio::test]
    async fn can_create_many() -> Result<()> {
        let state = init_state().await;

        Device {
            id: 1,
            name: "test device".to_string(),
            token: "my_token".to_string(),
            beat_count: 0,
        }
        .create(&state.pool)
        .await
        .unwrap();

        let ids = Beat::create_many(
            1,
            &[
                (Utc::now() - TimeDelta::days(10)).naive_utc(),
                (Utc::now() - TimeDelta::days(9)).naive_utc(),
                (Utc::now() - TimeDelta::days(8)).naive_utc(),
            ],
            &state.pool,
        )
        .await?;

        assert_eq!(&[1, 2, 3], &ids.as_ref());

        Ok(())
    }

    #[tokio::test]
    async fn can_get_by_ids() -> Result<()> {
        let state = init_state().await;

        Device {
            id: 1,
            name: "test device".to_string(),
            token: "my_token".to_string(),
            beat_count: 0,
        }
        .create(&state.pool)
        .await
        .unwrap();

        for i in 0..10 {
            Beat {
                id: 0,
                device: 1,
                timestamp: (Utc::now() - TimeDelta::days(i)).naive_utc(),
            }
            .create(&state.pool)
            .await?;
        }

        let beats = Beat::get_by_ids(&[1, 2, 3, 5, 7], &state.pool).await?;

        assert_eq!(5, beats.len());
        assert_eq!(7, beats[4].id);

        Ok(())
    }
}
