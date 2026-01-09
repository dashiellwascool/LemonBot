use std::sync::Arc;

use poise::{Framework, FrameworkOptions};
use serenity::{
    Client,
    all::{Context, EventHandler, GatewayIntents, Ready},
    async_trait,
};
use songbird::SerenityInit;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::{
    config::{Config, ConfigError},
    database::{make_db_pool, migrate_db, DatabaseKey},
    features::{
        squawk::SquawkListener,
        tts::{self, listener::TTSListener, queue::TTSSenders},
    },
    save_data::SaveData,
};

pub mod config;
pub mod database;
pub mod features;
pub mod save_data;

struct DiscordBot;

pub async fn start_bot(config: Config) -> anyhow::Result<()> {
    let intents = GatewayIntents::non_privileged()
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILDS
        | GatewayIntents::MESSAGE_CONTENT;

    let save_data = Arc::new(RwLock::new(SaveData::load_or_default()?));
    let config = Arc::new(config);

    let mut client = Client::builder(config.token.clone(), intents);

    // we only need these features for tts
    if config.piper_server.is_some() {
        // make sure postgres url is also some
        if config.postgres_url.is_none() {
            error!("POSTGRES_URL must also be set if PIPER_SERVER is set");
            return Err(ConfigError::MissingVar("POSTGRES_URL".to_string()).into());
        }

        // init poise
        let poise_framework = Framework::builder()
            .options(FrameworkOptions {
                commands: vec![
                    tts::commands::join(),
                    tts::commands::leave(),
                    tts::commands::set_nick(),
                    tts::commands::set_model(),
                    tts::commands::set_speaker(),
                    tts::commands::view_models(),
                    tts::commands::view_speakers()
                ],
                ..Default::default()
            })
            .setup(|ctx, _ready, framework| {
                Box::pin(async move {
                    poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                    Ok(())
                })
            })
            .build();

        // add everything to the client
        client = client
            .framework(poise_framework)
            .event_handler(TTSListener)
            .event_handler(tts::commands::view_models::ViewModelsListener)
            .event_handler(tts::commands::view_speakers::ViewSpeakersListener)
            .register_songbird();
    } else {
        warn!("PIPER_SERVER not set. TTS will not be enabled");
    }

    // init client
    let mut client = client
        .event_handler(DiscordBot)
        .event_handler(SquawkListener)
        .await?;

    {
        let mut data = client.data.write().await;
        // we only need this for tts
        if config.piper_server.is_some() {
            // init db
            let db = make_db_pool(&config).await?;
            migrate_db(&db).await?;

            data.insert::<TTSSenders>(Default::default());
            data.insert::<DatabaseKey>(Arc::new(db));
        }

        // everything else
        data.insert::<Config>(config);
        data.insert::<SaveData>(save_data);

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
