use std::{collections::HashMap, sync::Arc};

use reqwest::Client;
use serenity::{all::Http, futures::lock::Mutex, prelude::TypeMapKey};
use songbird::{Songbird, id::GuildId};
use sqlx::{Pool, Postgres};
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::{error, info};

use crate::{
    database::{TTSUser, get_tts_user},
    features::tts::get_tts,
};

pub struct TTSSenders;

impl TypeMapKey for TTSSenders {
    type Value = Arc<Mutex<HashMap<GuildId, Sender<TTSMessage>>>>;
}

#[derive(Clone)]
pub struct TTSMessage {
    pub author_id: u64,
    pub guild: u64,
    pub message: String,
}

pub(super) async fn speak_message_queue(
    songbird: Arc<Songbird>,
    client: Arc<Client>,
    mut rx: Receiver<TTSMessage>,
    guild_id: GuildId,
    tts_url: String,
    db: Arc<Pool<Postgres>>,
    http: Arc<Http>,
) {
    info!("Starting message queue for {guild_id}");

    let mut last_author: Option<String> = None;
    loop {
        let Some(message) = rx.recv().await else {
            info!("Closing message queue for {guild_id}");
            return;
        };

        let Some(lock) = songbird.get(guild_id) else {
            error!("could not get songbird for {guild_id}");
            continue;
        };

        // get all the info we need
        let user = match get_tts_user(&db, message.author_id).await {
            Ok(u) => u,
            Err(e) => {
                error!("Database error! {e}");
                TTSUser::default()
            }
        };

        let author_name =
            match get_author_name(&http, &user, message.guild, message.author_id).await {
                Ok(m) => m,
                Err(e) => {
                    error!("failed to get author name for {}: {e}", message.author_id);
                    continue;
                }
            };

        // create the audio
        let mut author_prefix = format!("{} said. ", author_name);
        if let Some(last_author) = &last_author
            && *last_author == author_name
        {
            author_prefix = String::new();
        }
        last_author = Some(author_name);

        let message = format!("{}{}", author_prefix, message.message);
        let tts = match get_tts(&message, user.model, user.speaker, &tts_url, &client).await {
            Ok(i) => i,
            Err(e) => {
                error!("Error getting tts message! {e}");
                continue;
            }
        };

        // play the audio
        let mut handle = lock.lock().await;
        handle.enqueue_input(tts).await;
    }
}

/// Attempts to get the name of a discord account with the following priority:
/// TTS nickname, guild nickname, global nickname, username
async fn get_author_name(
    http: &Http,
    tts_user: &TTSUser,
    guild_id: u64,
    user_id: u64,
) -> anyhow::Result<String> {
    // try TTS nickname
    if let Some(nick) = &tts_user.nick {
        return Ok(nick.to_string());
    }

    // try guild nickname
    let member = http.get_member(guild_id.into(), user_id.into()).await?;
    if let Some(nick) = member.nick {
        return Ok(nick);
    }

    // global nickname
    if let Some(nick) = member.user.global_name {
        return Ok(nick);
    }

    // username
    Ok(member.user.name)
}
