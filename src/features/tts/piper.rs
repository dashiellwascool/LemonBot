use std::{fmt::Display, time::Duration};

use reqwest::{Response, StatusCode};
use serde::{Deserialize, Serialize};
use songbird::input::{Input, MakePlayableError, codecs::get_codec_registry};
use symphonia::default::get_probe;
use tokio::time::sleep;

#[derive(Serialize)]
struct TTSBody {
    text: String,
    model: Option<String>,
    speaker: Option<String>,
}

#[derive(Deserialize, Debug, thiserror::Error)]
pub struct ErrorResponse {
    pub error: i32,
    pub error_code: i32, // TODO: lets move this into an enum
    pub message: String,
}

#[derive(Debug, thiserror::Error)]
pub enum GetTTSError {
    #[error("piper error: {0}")]
    Piper(#[from] ErrorResponse),
    #[error("client error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("audio error: {0}")]
    Audio(#[from] MakePlayableError),
}

impl Display for ErrorResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!(
            "piper server error {}. code={}. message={}",
            self.error, self.error_code, self.message
        ))
    }
}

pub async fn get_tts(
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
    let resp = get_with_retry_json(client, &format!("{server}/speak"), &body).await?;

    if resp.status() != StatusCode::OK {
        return Err(resp.json::<ErrorResponse>().await?.into());
    }

    let input: Input = resp.bytes().await.expect("response bytes").into();

    Ok(input
        .make_playable_async(get_codec_registry(), get_probe())
        .await?)
}

pub async fn get_models(server: &str, client: &reqwest::Client) -> anyhow::Result<Vec<String>> {
    let resp = get_with_retry(client, &format!("{server}/models")).await?;
    let voices: Vec<String> = resp.json().await?;

    Ok(voices)
}

#[derive(Deserialize)]
pub struct GetSpeakers {
    pub model: String,
    pub speakers: Vec<String>,
}

pub async fn get_speakers(
    server: &str,
    client: &reqwest::Client,
    model: Option<String>,
) -> anyhow::Result<GetSpeakers> {
    let url = match model {
        Some(model) => format!("{server}/speakers?model={model}"),
        None => format!("{server}/speakers"),
    };
    let resp = get_with_retry(client, &url).await?;
    let voices = resp.json().await?;

    Ok(voices)
}

async fn get_with_retry_json<T: Serialize>(
    client: &reqwest::Client,
    link: &str,
    body: T,
) -> Result<Response, reqwest::Error> {
    let mut attempts = 0;
    loop {
        match client.get(link).json(&body).send().await {
            Ok(r) => return Ok(r),
            Err(e) => {
                if attempts >= 3 {
                    return Err(e);
                }
                attempts += 1;
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

async fn get_with_retry(client: &reqwest::Client, link: &str) -> Result<Response, reqwest::Error> {
    let mut attempts = 0;
    loop {
        match client.get(link).send().await {
            Ok(r) => return Ok(r),
            Err(e) => {
                if attempts >= 3 {
                    return Err(e);
                }
                attempts += 1;
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

