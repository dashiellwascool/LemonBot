use std::sync::Arc;

use serenity::{all::{Context, EventHandler, GatewayIntents, Ready}, async_trait, Client};
use tokio::sync::RwLock;
use tracing::info;

use crate::{config::Config, features::squawk::SquawkListener, save_data::SaveData};

pub mod config;
pub mod save_data;
pub mod features;

struct DiscordBot;

pub async fn start_bot(config: Config) -> anyhow::Result<()> {
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::GUILDS;

    let save_data = Arc::new(RwLock::new(SaveData::load_or_default()?));
    let config = Arc::new(config);

    // read save data
    let mut client = Client::builder(config.token.clone(), intents)
        .event_handler(DiscordBot)
        .event_handler(SquawkListener)
        .await?;

    {
        let mut data = client.data.write().await;
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
