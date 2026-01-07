use poise::CreateReply;
use serde::Serialize;
use songbird::input::{Input, codecs::get_codec_registry};
use symphonia::default::get_probe;

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

async fn get_tts(
    msg: &str,
    model: Option<String>,
    speaker: Option<String>,
    server: &str,
    client: &reqwest::Client,
) -> anyhow::Result<Input> {
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

async fn get_speakers(
    server: &str,
    client: &reqwest::Client,
    model: Option<String>,
) -> anyhow::Result<Vec<String>> {
    let url = match model {
        Some(model) => format!("{server}/speakers?model={model}"),
        None => format!("{server}/speakers"),
    };
    let resp = client.get(url).send().await?;
    let voices: Vec<String> = resp.json().await?;

    Ok(voices)
}
