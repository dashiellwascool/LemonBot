use std::{collections::HashMap, sync::Arc};

use linkify::LinkFinder;
use reqwest::Client;
use serenity::{
    all::{Cache, ContentSafeOptions, Http, Message, content_safe},
    futures::lock::Mutex,
    prelude::TypeMapKey,
};
use songbird::{Songbird, id::GuildId, input::Input};
use sqlx::{Pool, Postgres};
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::{error, info};

use crate::{
    database::{self, get_tts_user, TTSUser},
    features::tts::{piper::{get_tts, GetTTSError}, queue::replacements::TTSReplacements},
};

pub mod replacements;

pub struct TTSSenders;

impl TypeMapKey for TTSSenders {
    type Value = Arc<Mutex<HashMap<GuildId, Sender<Message>>>>;
}

pub struct SpeakQueue {
    pub songbird: Arc<Songbird>,
    pub client: Arc<Client>,
    pub rx: Receiver<Message>,
    pub guild_id: GuildId,
    pub tts_url: String,
    pub db: Arc<Pool<Postgres>>,
    pub http: Arc<Http>,
    pub cache: Arc<Cache>,
    pub replacements: Arc<TTSReplacements>,
}

impl SpeakQueue {
    pub(super) async fn run(mut self) {
        let mut last_author: Option<String> = None;
        info!("Starting message queue for {}", self.guild_id);
        loop {
            let Some(message) = self.rx.recv().await else {
                info!("Closing message queue for {}", self.guild_id);
                return;
            };

            let Some(lock) = self.songbird.get(self.guild_id) else {
                error!("could not get songbird for {}", self.guild_id);
                continue;
            };

            // process & clean the content
            let mut content = message.content.clone();
            self.replacements.process_string(&mut content);
            let num_links = clean_links(&mut content);
            content = content_safe(
                &self.cache,
                content,
                &ContentSafeOptions::new()
                    .display_as_member_from(message.guild_id.expect("we are in a guild")),
                &[],
            );

            // get database user
            let user = match get_tts_user(&self.db, message.author.id.get()).await {
                Ok(u) => u,
                Err(e) => {
                    error!("Database error! {e}");
                    TTSUser::default()
                }
            };

            let author_name = get_author_name(&self.cache, &self.http, &user, &message).await;

            let attachment_text = make_attachment_text(&message, num_links);

            if content.is_empty() {
                if attachment_text.is_empty() {
                    continue;
                } else {
                    content = format!("{author_name} sent {attachment_text}");
                }
            } else if attachment_text.is_empty() {
                if let Some(last_author) = &last_author
                    && *last_author == author_name
                {
                } else {
                    content = format!("{author_name} said {content}");
                }
            } else {
                content = format!("{author_name} said {content} and sent {attachment_text}")
            }

            last_author = Some(author_name);

            let tts = match get_tts_with_fallback(
                &content,
                user.model,
                user.speaker,
                &self.tts_url,
                &self.client,
                message.author.id.get(),
                &self.db,
            )
            .await
            {
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
}

async fn get_tts_with_fallback(
    msg: &str,
    model: Option<String>,
    speaker: Option<String>,
    server: &str,
    client: &reqwest::Client,
    user_id: u64,
    db: &Pool<Postgres>,
) -> anyhow::Result<Input> {
    match get_tts(msg, model, speaker, server, client).await {
        Ok(i) => Ok(i),
        Err(e) => match e {
            GetTTSError::Piper(e) => {
                if e.error_code == 1 {
                    // bad model
                    // reset the user's model
                    database::set_model(db, user_id, None).await?;
                    return Ok(get_tts(msg, None, None, server, client).await?);
                }
                Err(e.into())
            }
            e => Err(e.into()),
        },
    }
}

/// Returns the number of links cleaned
fn clean_links(text: &mut String) -> u32 {
    let mut num_links = 0;

    *text = LinkFinder::new()
        .spans(text)
        .filter(|s| {
            if s.kind().is_some() {
                num_links += 1;
                return false;
            }
            true
        })
        .map(|s| s.as_str())
        .collect();

    num_links
}

/// Attempts to get the name of a discord account with the following priority:
/// TTS nickname, guild nickname, global nickname, username
async fn get_author_name(
    cache: &Arc<Cache>,
    http: &Http,
    tts_user: &TTSUser,
    message: &Message,
) -> String {
    // try TTS nickname
    if let Some(nick) = &tts_user.nick {
        return nick.to_string();
    }

    // try guild nickname
    if let Ok(m) = message.member((cache, http)).await
        && let Some(nick) = m.nick
    {
        return nick;
    }

    // global nickname
    if let Some(nick) = &message.author.global_name {
        return nick.clone();
    }

    // username
    message.author.name.clone()
}

fn make_attachment_text(message: &Message, num_links: u32) -> String {
    let mut num_files = 0;
    let mut num_images = 0;
    let mut num_audio = 0;
    let mut num_video = 0;
    for attachment in &message.attachments {
        if let Some(content_type) = &attachment.content_type {
            if content_type.starts_with("image/") {
                num_images += 1;
                continue;
            } else if content_type.starts_with("video/") {
                num_video += 1;
                continue;
            } else if content_type.starts_with("audio/") {
                num_audio += 1;
                continue;
            }
        }
        num_files += 1;
    }

    let mut texts: Vec<String> = Vec::new();
    if num_images > 0 {
        texts.push(make_single_attachment_text(num_images, "image", true));
    }
    if num_video > 0 {
        texts.push(make_single_attachment_text(num_video, "video", false));
    }
    if num_audio > 0 {
        texts.push(make_single_attachment_text(num_audio, "audio file", true));
    }
    if num_files > 0 {
        texts.push(make_single_attachment_text(num_files, "file", false));
    }
    if num_links > 0 {
        texts.push(make_single_attachment_text(num_links, "link", false));
    }
    if message.poll.is_some() {
        texts.push(String::from("a poll"));
    }
    if message.message_reference.is_some() {
        texts.push(String::from("a forwarded message"));
    }

    if !texts.is_empty() {
        let mut result = String::new();

        for text in &texts[0..texts.len() - 1] {
            result += text;
            result += ". ";
        }

        if texts.len() > 1 {
            result += " and ";
        }
        result += texts.last().expect("texts is not empty");

        return result;
    }

    String::new()
}

fn make_single_attachment_text(num: u32, name: &str, use_an: bool) -> String {
    if num > 1 {
        format!("{num} {name}s")
    } else if num == 1 {
        if use_an {
            format!("an {name}")
        } else {
            format!("a {name}")
        }
    } else {
        String::new()
    }
}
