use poise::CreateReply;
use serde_json::json;
use serenity::all::Context;
use songbird::{
    id::{ChannelId, GuildId},
    input::{Input, codecs::get_codec_registry},
};
use symphonia::default::get_probe;
use tokio::sync::mpsc;

use crate::{
    config::Config,
    features::tts::queue::{QueuedMessage, TTSSenders},
};

pub mod commands;
pub mod listener;
pub mod queue;
mod vc;

type PoiseContext<'a> = poise::Context<'a, (), anyhow::Error>;

fn make_ephemeral_reply(msg: &str) -> CreateReply {
    CreateReply::default().ephemeral(true).content(msg)
}

async fn get_tts(msg: &str, server: &str) -> Input {
    let client = reqwest::Client::new();

    let body = json!({
        "text": msg,
        "length_scale": 1,
        "noise_scale": 0.666,
        "noise_w_scale": 0.8
    });

    let resp = client
        .post(server)
        .json(&body)
        .send()
        .await
        .expect("frick?");

    let input: Input = resp.bytes().await.expect("response bytes").into();

    input
        .make_playable_async(get_codec_registry(), get_probe())
        .await
        .expect("file should be supported")
}
