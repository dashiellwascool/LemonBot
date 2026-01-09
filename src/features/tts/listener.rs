use serenity::{
    all::{Message, VoiceState},
    async_trait,
    prelude::*,
};
use tracing::info;

use crate::{
    config::Config,
    features::tts::{
        queue::TTSSenders,
        vc::leave_vc,
    },
};

pub struct TTSListener;
#[async_trait]
impl EventHandler for TTSListener {
    async fn message(&self, ctx: Context, message: Message) {
        // do not read tts from bots
        if message.author.bot || message.content.is_empty() {
            return;
        }

        // get the guild the message was in
        let guild = if let Some(g) = message.guild(&ctx.cache) {
            g
        } else {
            return;
        }
        .clone();

        info!("{} {:?}", message.author.name, message.author.global_name);

        // get songbird
        let songbird = songbird::get(&ctx)
            .await
            .expect("Songbird has been initialized");
        let songbird = if let Some(handle) = songbird.get(guild.id) {
            handle
        } else {
            return;
        };

        let bot_channel = {
            let songbird = songbird.lock().await;
            let channel = songbird.current_channel();
            if let Some(channel) = channel {
                channel
            } else {
                return;
            }
        };

        // check if the message was sent in a tts channel
        let config = {
            let data_read = ctx.data.read().await;
            data_read
                .get::<Config>()
                .expect("Config should be present")
                .clone()
        };

        if !config.tts_channels.contains(&message.channel_id.get()) && message.channel_id.get() != bot_channel.0.get() {
            return;
        }

        // check if the user is in a vc
        let user_vc = if let Some(vc) = guild.voice_states.get(&message.author.id) {
            vc.channel_id.expect("we are in a guild")
        } else {
            return;
        };

        // now let's check if we are in the same vc

        if bot_channel.0.get() == user_vc.get() {
            // get the sender and queue the message
            let senders_lock = {
                let data = ctx.data.read().await;
                data.get::<TTSSenders>()
                    .expect("TTSSenders has been initialized")
                    .clone()
            };
            let senders = senders_lock.lock().await;
            if let Some(sender) = senders.get(&message.guild_id.expect("we are in a guild").into())
            {
                _ = sender
                    .send(message)
                    .await;
            }
        }
    }

    async fn voice_state_update(&self, ctx: Context, _old: Option<VoiceState>, new: VoiceState) {
        // Leave the call if we are in one and it is only us in it

        let guild = if let Some(g) = new.guild_id {
            g
        } else {
            return;
        };

        // check if we are in a vc
        let songbird = songbird::get(&ctx)
            .await
            .expect("Songbird has been initialized");
        let songbird = if let Some(handle) = songbird.get(guild) {
            handle
        } else {
            return;
        };
        let channel = {
            let songbird = songbird.lock().await;
            songbird.current_channel()
        };
        if let Some(bot_vc) = channel {
            let bot_vc = bot_vc.0.get();
            let bot_id = ctx.http.get_current_user().await.expect("we are a user").id;

            for (user, state) in &ctx
                .cache
                .guild(guild)
                .expect("we have this guild")
                .voice_states
            {
                if *user != bot_id && state.channel_id.map(|i| i.get()) == Some(bot_vc) {
                    return;
                }
            }

            _ = leave_vc(&ctx, guild.into()).await;
        }
    }
}
