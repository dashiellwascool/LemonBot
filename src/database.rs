use std::{sync::Arc, time::Duration};

use serenity::prelude::TypeMapKey;
use sqlx::{Pool, Postgres, Row, postgres::PgPoolOptions};
use tracing::info;

use crate::config::Config;

#[derive(Default)]
pub struct TTSUser {
    pub nick: Option<String>,
    pub model: Option<String>,
    pub speaker: Option<String>,
}

pub struct DatabaseKey;

impl TypeMapKey for DatabaseKey {
    type Value = Arc<Pool<Postgres>>;
}

pub async fn make_db_pool(config: &Config) -> anyhow::Result<Pool<Postgres>> {
    info!("Making database pool");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(60))
        .connect(&config.postgres_url)
        .await?;

    Ok(pool)
}

pub async fn migrate_db(db: &Pool<Postgres>) -> anyhow::Result<()> {
    info!("Running databse migrations (if any)");
    sqlx::migrate!().run(db).await?;

    Ok(())
}

pub async fn get_tts_user(db: &Pool<Postgres>, user_id: u64) -> anyhow::Result<TTSUser> {
    let response = sqlx::query(
        "
        select * from TTSUsers where discord_id=$1;
        ",
    )
    .bind(user_id as i64)
    .fetch_optional(db)
    .await?;

    Ok(match response {
        Some(row) => TTSUser {
            nick: row.get("nick"),
            model: row.get("model"),
            speaker: row.get("speaker"),
        },
        None => TTSUser::default(),
    })
}

pub async fn set_nick(db: &Pool<Postgres>, user_id: u64, nick: Option<String>) -> anyhow::Result<()> {
    sqlx::query("INSERT INTO TTSUsers (discord_id, nick) VALUES ($1, $2) ON CONFLICT (discord_id) DO UPDATE SET discord_id=$1, nick=$2;")
        .bind(user_id as i64)
        .bind(nick)
        .execute(db)
        .await?;
    Ok(())
}
