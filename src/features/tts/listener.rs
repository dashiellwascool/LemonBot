use serenity::{
    all::{Message, VoiceState},
    async_trait,
    prelude::*,
};

use crate::{
    config::Config,
    features::tts::{
        queue::{TTSMessage, TTSSenders},
        vc::leave_vc,
    },
};

pub struct TTSListener;
#[async_trait]
impl EventHandler for TTSListener {
    async fn message(&self, ctx: Context, message: Message) {
        // get the guild the message was in
        let guild = if let Some(g) = message.guild(&ctx.cache) {
            g
        } else {
            return;
        }
        .clone();

        // check if the message was sent in a tts channel
        let config = {
            let data_read = ctx.data.read().await;
            data_read
                .get::<Config>()
                .expect("Config should be present")
                .clone()
        };

        if !config.tts_channels.contains(&message.channel_id.get()) {
            return;
        }

        // check if the user is in a vc
        let user_vc = if let Some(vc) = guild.voice_states.get(&message.author.id) {
            vc.channel_id.expect("we are in a guild")
        } else {
            return;
        };

        // now let's check if we are in the same vc
        let songbird = songbird::get(&ctx)
            .await
            .expect("Songbird has been initialized");
        let songbird = if let Some(handle) = songbird.get(guild.id) {
            handle
        } else {
            return;
        };
        let same_vc = {
            let songbird = songbird.lock().await;
            if let Some(bot_vc) = songbird.current_channel()
                && bot_vc.0.get() == user_vc.get()
            {
                true
            } else {
                false
            }
        };

        if same_vc {
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
                // get the name

                // >_<
                let author_member = ctx.http
                        .get_guild(message.guild_id.expect("we are in a guld"))
                        .await
                        .expect("we are in a guild")
                        .member(&ctx.http, message.author.id)
                        .await
                        .expect("the member sent a message");

                let name = if let Some(nick) = author_member.nick {
                    nick
                } else if let Some(global_nick) = &message.author.global_name {
                    global_nick.clone()
                } else {
                    message.author.name.clone()
                };

                _ = sender
                    .send(TTSMessage {
                        author: name,
                        message: message.content,
                    })
                    .await;
            }
        }
    }

    async fn voice_state_update(&self, ctx: Context, _old: Option<VoiceState>, new: VoiceState) {
        // Leave the call if we are in one and it is only us in it

        if new.channel_id.is_some() {
            // we only need to check if someone just left the VC
            return;
        }
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
