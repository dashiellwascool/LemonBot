use std::{collections::HashMap, sync::Arc};

use serenity::{futures::lock::Mutex, prelude::TypeMapKey};
use songbird::{Songbird, id::GuildId};
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::{error, info};

use crate::features::tts::get_tts;

pub struct TTSSenders;

impl TypeMapKey for TTSSenders {
    type Value = Arc<Mutex<HashMap<GuildId, Sender<TTSMessage>>>>;
}

#[derive(Clone)]
pub struct TTSMessage {
    pub author: String,
    pub message: String,
}

pub(super) async fn speak_message_queue(
    songbird: Arc<Songbird>,
    mut rx: Receiver<TTSMessage>,
    guild_id: GuildId,
    tts_url: String,
) {
    info!("Starting message queue for {guild_id}");

    let mut last_author: Option<String> = None;
    loop {
        match rx.recv().await {
            Some(message) => {
                if let Some(lock) = songbird.get(guild_id) {
                    // assume we should play the audio if we are recieving these messages

                    // create the audio
                    let mut author_prefix = format!("{} said. ", message.author);
                    if let Some(last_author) = &last_author
                        && *last_author == message.author
                    {
                        author_prefix = String::new();
                    }
                    last_author = Some(message.author);

                    let message = format!("{}{}", author_prefix, message.message);
                    let tts = match get_tts(&message, &tts_url, &reqwest::Client::new()).await {
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
