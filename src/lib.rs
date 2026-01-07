use std::sync::Arc;

use poise::{Framework, FrameworkOptions};
use serenity::{
    all::{Context, EventHandler, GatewayIntents, Ready}, async_trait, Client
};
use songbird::SerenityInit;
use tokio::sync::RwLock;
use tracing::info;

use crate::{
    config::Config, database::{make_db_pool, migrate_db, DatabaseKey}, features::{
        squawk::SquawkListener,
        tts::{self, listener::TTSListener, queue::TTSSenders},
    }, save_data::SaveData
};

pub mod config;
pub mod features;
pub mod save_data;
pub mod database;

struct DiscordBot;

pub async fn start_bot(config: Config) -> anyhow::Result<()> {
    let intents =
        GatewayIntents::non_privileged() | GatewayIntents::GUILD_MESSAGES | GatewayIntents::GUILDS | GatewayIntents::MESSAGE_CONTENT;

    let save_data = Arc::new(RwLock::new(SaveData::load_or_default()?));
    let config = Arc::new(config);

    // init database
    let db = make_db_pool(&config).await?;
    migrate_db(&db).await?;

    // init poise
    let poise_framework = Framework::builder()
        .options(FrameworkOptions {
            commands: vec![tts::commands::join(), tts::commands::leave(), tts::commands::set_nick(), tts::commands::set_model(), tts::commands::set_speaker()],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(())
            })
        })
        .build();

    // init client
    let mut client = Client::builder(config.token.clone(), intents)
        .event_handler(DiscordBot)
        .event_handler(SquawkListener)
        .framework(poise_framework)
        .event_handler(TTSListener)
        .register_songbird()
        .await?;

    { // insert everything
        let mut data = client.data.write().await;
        data.insert::<Config>(config);
        data.insert::<SaveData>(save_data);
        data.insert::<TTSSenders>(Default::default());
        data.insert::<DatabaseKey>(Arc::new(db));
    }

    client.start().await?;

    Ok(())
}

#[async_trait]
impl EventHandler for DiscordBot {
    async fn ready(&self, _: Context, _: Ready) {
        info!("Ready!");
    }
}
