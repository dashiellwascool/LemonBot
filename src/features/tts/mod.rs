use std::fmt::Display;

use poise::CreateReply;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use songbird::input::{codecs::get_codec_registry, Input, MakePlayableError};
use symphonia::default::get_probe;
use thiserror::Error;

pub mod commands;
pub mod listener;
pub mod queue;
mod vc;

type PoiseContext<'a> = poise::Context<'a, (), anyhow::Error>;

fn make_ephemeral_reply(msg: &str) -> CreateReply {
    CreateReply::default().ephemeral(true).content(msg)
}

#[derive(Serialize)]
struct TTSBody {
    text: String,
    model: Option<String>,
    speaker: Option<String>,
}

#[derive(Deserialize, Debug, Error)]
struct ErrorResponse {
    error: i32,
    error_code: i32,
    message: String
}

#[derive(Debug, Error)]
enum GetTTSError {
    #[error("piper error: {0}")]
    Piper(#[from] ErrorResponse),
    #[error("client error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("audio error: {0}")]
    Audio(#[from] MakePlayableError)
}

impl Display for ErrorResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("piper server error {}. code={}. message={}", self.error, self.error_code, self.message))
    }
}

async fn get_tts(
    msg: &str,
    model: Option<String>,
    speaker: Option<String>,
    server: &str,
    client: &reqwest::Client,
) -> Result<Input, GetTTSError> {
    let body = TTSBody {
        text: msg.to_string(),
        model,
        speaker,
    };

    let resp = client
        .get(format!("{server}/speak"))
        .json(&body)
        .send()
        .await?;

    if resp.status() != StatusCode::OK {
        return Err(resp.json::<ErrorResponse>().await?.into())
    }

    let input: Input = resp.bytes().await.expect("response bytes").into();

    Ok(input
        .make_playable_async(get_codec_registry(), get_probe())
        .await?)
}

async fn get_models(server: &str, client: &reqwest::Client) -> anyhow::Result<Vec<String>> {
    let resp = client.get(format!("{server}/models")).send().await?;
    let voices: Vec<String> = resp.json().await?;

    Ok(voices)
}

#[derive(Deserialize)]
struct GetSpeakers {
    model: String,
    speakers: Vec<String>
}

async fn get_speakers(
    server: &str,
    client: &reqwest::Client,
    model: Option<String>,
) -> anyhow::Result<GetSpeakers> {
    let url = match model {
        Some(model) => format!("{server}/speakers?model={model}"),
        None => format!("{server}/speakers"),
    };
    let resp = client.get(url).send().await?;
    let voices = resp.json().await?;

    Ok(voices)
}

