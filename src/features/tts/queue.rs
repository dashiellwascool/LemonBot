use std::{collections::HashMap, sync::Arc};

use reqwest::Client;
use serenity::{futures::lock::Mutex, prelude::TypeMapKey};
use songbird::{Songbird, id::GuildId};
use sqlx::{Pool, Postgres};
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::{error, info};

use crate::{database::{get_tts_user, TTSUser}, features::tts::get_tts};

pub struct TTSSenders;

impl TypeMapKey for TTSSenders {
    type Value = Arc<Mutex<HashMap<GuildId, Sender<TTSMessage>>>>;
}

#[derive(Clone)]
pub struct TTSMessage {
    pub author_id: u64,
    pub author: String,
    pub message: String,
}

pub(super) async fn speak_message_queue(
    songbird: Arc<Songbird>,
    client: Arc<Client>,
    mut rx: Receiver<TTSMessage>,
    guild_id: GuildId,
    tts_url: String,
    db: Arc<Pool<Postgres>>
) {
    info!("Starting message queue for {guild_id}");

    let mut last_author: Option<String> = None;
    loop {
        match rx.recv().await {
            Some(mut message) => {
                if let Some(lock) = songbird.get(guild_id) {
                    // assume we should play the audio if we are recieving these messages
                    let user = match get_tts_user(&db, message.author_id).await {
                        Ok(u) => u,
                        Err(e) => {
                            error!("Database error! {e}");
                            TTSUser::default()
                        },
                    };

                    if let Some(nick) = user.nick {
                        message.author = nick;
                    }

                    // create the audio
                    let mut author_prefix = format!("{} said. ", message.author);
                    if let Some(last_author) = &last_author
                        && *last_author == message.author
                    {
                        author_prefix = String::new();
                    }
                    last_author = Some(message.author);

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
            None => {
                info!("Closing message queue for {guild_id}");
                return;
            }
        }
    }
}
