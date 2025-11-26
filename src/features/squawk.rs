use std::sync::Arc;

use chrono::{Duration, Utc};
use serenity::{all::{ChannelId, Context, CreateAllowedMentions, CreateMessage, EventHandler, Http, Message, MessageFlags, Ready}, async_trait};
use tokio::{sync::RwLock, time::sleep};
use tracing::{error, info, warn};

use crate::{config::Config, save_data::SaveData};

pub struct SquawkListener;

const SQUAWK: &str = "Squawk!";
const FUK_U: &str = "frick u";

#[async_trait]
impl EventHandler for SquawkListener {

    async fn ready(&self, ctx: Context, _: Ready) {
        let (config,data) = {
            let data = ctx.data.read().await;
            let config = data.get::<Config>().expect("config should be present").clone();
            let save = data.get::<SaveData>().expect("save data should be present").clone();

            (config, save)
        };
        tokio::task::spawn(self_squawk_task(config, data, ctx.http.clone()));
    }

    async fn message(&self, ctx: Context, message: Message) {
        if message.author.bot {
            return;
        }

        match message.mentions_me(&ctx.http).await {
            Ok(mentions_me) => {
                if mentions_me {
                    let config = {
                        let data_read = ctx.data.read().await;
                        data_read.get::<Config>().expect("Config should be present").clone()
                    };

                    if config.squawk_blacklist_channels.contains(&message.channel_id.get()) {
                        return;
                    }

                    info!("squawking because i was mentioned");
                    let reply = get_squawk_message(&config).reference_message(&message);
                    if let Err(e) = message.channel_id.send_message(ctx.http, reply).await {
                        error!("failed to reply to message: {e}");
                    }
                    return;
                }
            },
            Err(e) => error!("failed mentions me check on new message: {e}")
        }

        let save_data = {
            let data_read = ctx.data.read().await;
            data_read.get::<SaveData>().expect("Save Data should be present").clone()
        };

        {
            let save_data = save_data.read().await;
            if Utc::now() < save_data.squawk_cooldown {
                return;
            }
        }

        let config = {
            let data_read = ctx.data.read().await;
            data_read.get::<Config>().expect("Config should be present").clone()
        };

        if config.random_squawk_channels.contains(&message.channel_id.get()) && rand::random_range(0.0..=1.0) < config.squawk_response_chance {
            {
                let mut save_data = save_data.write().await;
                save_data.squawk_cooldown = Utc::now() + Duration::seconds(config.squawk_cooldown);
                if let Err(e) = save_data.save() {
                    error!("failed to save data! {e}");
                }
            }

            info!("squawking because random squawk chance");
            let reply = get_squawk_message(&config).reference_message(&message);
            if let Err(e) = message.channel_id.send_message(ctx.http, reply).await {
                error!("failed to reply to message: {e}");
            }
        }
    }
}

pub async fn self_squawk_task(config: Arc<Config>, data: Arc<RwLock<SaveData>>, discord: Arc<Http>) {
    info!("Starting self squawk");
    if config.random_squawk_channels.is_empty() {
        warn!("No random squawk channels specified. Disabling random squawk");
        return;
    }

    loop {
        let next_squawk = {
            data.read().await.next_random_squawk
        };

        let now = Utc::now();

        if now < next_squawk {
            info!("Next self squawk is on {}", next_squawk.naive_local());
            sleep((next_squawk - now).to_std().expect("next squawk will be in the future")).await;
        }

        let cooldown = {
            data.read().await.squawk_cooldown
        };

        if Utc::now() > cooldown && Utc::now() < next_squawk + Duration::seconds(config.squawk_cooldown) {
            info!("Self squawking now");
            // send squawk
            let channel = config.random_squawk_channels[rand::random_range(0..config.random_squawk_channels.len())];
            if let Err(e) = ChannelId::new(channel).send_message(&discord, CreateMessage::new().content(SQUAWK).flags(MessageFlags::SUPPRESS_NOTIFICATIONS)).await {
                error!("Failed to send random squawk {e}");
            }

        } else {
            info!("Skipping self squawk because of cooldown");
        }
        let next_squawk_offset = rand::random_range(config.squawk_cooldown..=config.max_random_squawk_time);

        let mut data = data.write().await;
        data.next_random_squawk = Utc::now() + Duration::seconds(next_squawk_offset);
        data.squawk_cooldown = Utc::now() + Duration::seconds(config.squawk_cooldown);
        if let Err(e) = data.save() {
            error!("Failed to save data {e}");
        }
    }
}

fn get_squawk_message(config: &Config) -> CreateMessage {
    let msg = if rand::random_range(0.0..=1.0) < config.fuk_u_chance {
        FUK_U
    } else {
        SQUAWK
    };
    CreateMessage::new().content(msg).flags(MessageFlags::SUPPRESS_NOTIFICATIONS).allowed_mentions(CreateAllowedMentions::new().all_users(false))
}

