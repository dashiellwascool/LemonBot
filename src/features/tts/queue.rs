use std::{collections::HashMap, sync::Arc};

use linkify::LinkFinder;
use reqwest::Client;
use serde_json::error::Category;
use serenity::{
    all::{Cache, ContentSafeOptions, Http, content_safe},
    futures::lock::Mutex,
    prelude::TypeMapKey,
};
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

#[allow(clippy::too_many_arguments)]
pub(super) async fn speak_message_queue(
    songbird: Arc<Songbird>,
    client: Arc<Client>,
    mut rx: Receiver<TTSMessage>,
    guild_id: GuildId,
    tts_url: String,
    db: Arc<Pool<Postgres>>,
    http: Arc<Http>,
    cache: Arc<Cache>,
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

        // proces sand clean the message
        let content: String = LinkFinder::new()
            .spans(&message.message)
            .filter(|s| s.kind().is_none())
            .map(|s| s.as_str())
            .collect();

        let has_link = content != message.message;

        let content = content_safe(
            &cache,
            content,
            &ContentSafeOptions::new().display_as_member_from(message.guild),
            &[],
        );

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
        let message = {
            if content.is_empty() && has_link {
                format!("{} sent a link.", author_name)
            } else {
                let mut author_prefix = format!("{} said. ", author_name);
                if let Some(last_author) = &last_author
                    && *last_author == author_name
                {
                    author_prefix = String::new();
                }
                last_author = Some(author_name);

                format!(
                    "{}{}{}",
                    author_prefix,
                    content,
                    if has_link { " and sent a link." } else { "" }
                )
            }
        };
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
