use std::{collections::HashMap, sync::Arc};

use linkify::LinkFinder;
use regex::{Regex, RegexBuilder};
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
    database::{self, TTSUser, get_tts_user},
    features::tts::{GetTTSError, get_tts},
};

pub struct TTSSenders;

impl TypeMapKey for TTSSenders {
    type Value = Arc<Mutex<HashMap<GuildId, Sender<Message>>>>;
}

pub struct TTSReplacements {
    replacements: [(Regex, &'static str); 3],
    emoji_capture: Regex,
    unicode_emoji: Regex,
}

impl TTSReplacements {
    pub fn new() -> anyhow::Result<Self> {
        Ok(TTSReplacements {
            replacements: [
                (
                    RegexBuilder::new(r"```(.*?)```")
                        .dot_matches_new_line(true)
                        .build()?,
                    " |code block| ",
                ),
                (
                    RegexBuilder::new(r"`(.*?)`")
                        .dot_matches_new_line(true)
                        .build()?,
                    " |code block|",
                ),
                (
                    RegexBuilder::new(r"\|\|(.*?)\|\|")
                        .dot_matches_new_line(true)
                        .build()?,
                    " |spoilers| ",
                ),
            ],
            emoji_capture: Regex::new(r"<(a?):(.*?):\d+>")?,
            unicode_emoji: Regex::new(r"\p{Emoji}")?,
        })
    }
}

impl TypeMapKey for TTSReplacements {
    type Value = Arc<TTSReplacements>;
}

pub struct QueueContext {
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

#[allow(clippy::too_many_arguments)]
pub(super) async fn speak_message_queue(
    songbird: Arc<Songbird>,
    client: Arc<Client>,
    mut rx: Receiver<Message>,
    guild_id: GuildId,
    tts_url: String,
    db: Arc<Pool<Postgres>>,
    http: Arc<Http>,
    cache: Arc<Cache>,
    replacements: Arc<TTSReplacements>,
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

        // process and clean the message
        // clean code blocks
        let mut pre_link_content = message.content.to_string();
        for (regex, str) in replacements.replacements.iter() {
            pre_link_content = regex.replace_all(&pre_link_content, *str).into();
        }

        // clean discord emojis
        pre_link_content = replacements
            .emoji_capture
            .replace_all(&pre_link_content, |capture: &regex::Captures<'_>| {
                let is_animated = capture.get(1).expect("there are two groups").as_str();
                let emoji_name = capture.get(2).expect("there are two groups").as_str();

                info!("{:?}", capture);

                let emoji_type = if is_animated.is_empty() {
                    "emoji"
                } else {
                    "animated emoji"
                };

                format!(" {emoji_name} {emoji_type} ")
            })
            .into();

        // clean unicode emojis
        pre_link_content = replacements
            .unicode_emoji
            .replace_all(&pre_link_content, |capture: &regex::Captures<'_>| {
                let emoji = capture.get(0).expect("there is one group").as_str();

                if let Some(emoji) = emojis::get(emoji) {
                    format!(" {} emoji ", emoji.name())
                } else {
                    String::from(" emoji ")
                }
            })
            .into();

        // clean links
        let content: String = LinkFinder::new()
            .spans(&pre_link_content)
            .filter(|s| s.kind().is_none())
            .map(|s| s.as_str())
            .collect();

        let has_link = content != pre_link_content;

        let content = content_safe(
            &cache,
            content,
            &ContentSafeOptions::new()
                .display_as_member_from(message.guild_id.expect("we are in a guild")),
            &[],
        );

        // get all the info we need
        let user = match get_tts_user(&db, message.author.id.get()).await {
            Ok(u) => u,
            Err(e) => {
                error!("Database error! {e}");
                TTSUser::default()
            }
        };

        let author_name = get_author_name(&cache, &http, &user, &message).await;

        // check for an image or file
        let mut has_image = false;
        let mut has_file = false;
        for attachment in message.attachments {
            info!("{:?}", attachment.content_type);
        }

        // create the audio
        let text = {
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
                    "{}{}{}{}{}",
                    author_prefix,
                    content,
                    if has_link { " and sent a link" } else { "" },
                    if has_image { " and sent an image" } else { "" },
                    if has_file { " and sent a file" } else { "" }
                )
            }
        };
        let tts = match get_tts_with_fallback(
            &text,
            user.model,
            user.speaker,
            &tts_url,
            &client,
            message.author.id.get(),
            &db,
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

async fn clean_replacements(ctx: &QueueContext, message: String) -> String {
    let mut message = message.to_string();
    for (regex, str) in ctx.replacements.replacements.iter() {
        message = regex.replace_all(&message, *str).into();
    }
    message
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
