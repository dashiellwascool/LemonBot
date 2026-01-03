use std::{collections::HashMap, sync::Arc};

use serenity::{futures::lock::Mutex, prelude::TypeMapKey};
use songbird::{Songbird, id::GuildId};
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::info;

use crate::features::tts::get_tts;

pub struct TTSSenders;

impl TypeMapKey for TTSSenders {
    type Value = Arc<Mutex<HashMap<GuildId, Sender<QueuedMessage>>>>;
}

#[derive(Clone)]
pub struct QueuedMessage {
    pub author: String,
    pub message: String,
}

pub(super) async fn speak_message_queue(
    songbird: Arc<Songbird>,
    mut rx: Receiver<QueuedMessage>,
    guild_id: GuildId,
    tts_url: String,
) {
    info!("Starting message queue for {guild_id}");
    loop {
        match rx.recv().await {
            Some(message) => {
                if let Some(lock) = songbird.get(guild_id) {
                    // assume we should play the audio if we are recieving these messages

                    // create the audio
                    let message = format!("{} said. {}", message.author, message.message);
                    let tts = get_tts(&message, &tts_url).await;

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
