use serenity::{all::{Context, CreateMessage, EventHandler, GatewayIntents, Message, MessageFlags, Ready}, async_trait, Client};
use tracing::{error, info};

use crate::config::Config;

pub mod config;
pub mod save_data;

struct DiscordBot {
    config: Config
}

pub async fn start_bot(config: Config) -> anyhow::Result<()> {
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::GUILDS;

    let mut client = Client::builder(config.token.clone(), intents)
        .event_handler(DiscordBot {
            config
        }).await?;

    client.start().await?;
    Ok(())
}

#[async_trait]
impl EventHandler for DiscordBot {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("Ready!");
    }

    async fn message(&self, ctx: Context, message: Message) {
        if message.author.bot {
            return;
        }

        match message.mentions_me(&ctx.http).await {
            Ok(mentions_me) => {
                if mentions_me {
                    info!("squawking because i was mentioned");
                    let reply = get_squawk_message(&self.config).reference_message(&message);
                    if let Err(e) = message.channel_id.send_message(ctx.http, reply).await {
                        error!("failed to reply to message: {e}");
                    }
                    return;
                }
            },
            Err(e) => error!("failed mentions me check on new message: {e}")
        }

        if rand::random_range(0.0..=1.0) < self.config.squawk_response_chance {
            info!("squawking because random squawk chance");
            let reply = get_squawk_message(&self.config).reference_message(&message);
            if let Err(e) = message.channel_id.send_message(ctx.http, reply).await {
                error!("failed to reply to message: {e}");
            }
        }
    }
}

fn get_squawk_message(config: &Config) -> CreateMessage {
    let msg = if rand::random_range(0.0..=1.0) < config.fuk_u_chance {
        "fuk u"
    } else {
        "squawk"
    };
    CreateMessage::new().content(msg).flags(MessageFlags::SUPPRESS_NOTIFICATIONS)
}

