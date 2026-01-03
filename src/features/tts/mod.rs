use poise::CreateReply;
use serde_json::json;
use songbird::input::{Input, codecs::get_codec_registry};
use symphonia::default::get_probe;

pub mod commands;
pub mod listener;
pub mod queue;
mod vc;

type PoiseContext<'a> = poise::Context<'a, (), anyhow::Error>;

//#[derive(Deserialize, Debug)]
//struct VoicesResponse {
//    speaker_id_map: HashMap<String, i32>,
//}

fn make_ephemeral_reply(msg: &str) -> CreateReply {
    CreateReply::default().ephemeral(true).content(msg)
}

async fn get_tts(msg: &str, server: &str, client: &reqwest::Client) -> anyhow::Result<Input> {
    let body = json!({
        "text": msg,
        "length_scale": 1,
        "noise_scale": 0.666,
        "noise_w_scale": 0.8
    });

    let resp = client.post(server).json(&body).send().await?;

    let input: Input = resp.bytes().await.expect("response bytes").into();

    Ok(input
        .make_playable_async(get_codec_registry(), get_probe())
        .await?)
}

//async fn get_voices(
//    server: &str,
//    client: &reqwest::Client,
//) -> anyhow::Result<HashMap<String, Vec<String>>> {
//    let mut voices = HashMap::new();
//
//    let resp = client.get(format!("{server}/voices")).send().await?;
//    let response_voices: HashMap<String, VoicesResponse> = resp.json().await?;
//
//    for (model, voice) in response_voices {
//        if !voice.speaker_id_map.is_empty() {
//            for (speaker, _) in voice.speaker_id_map {
//                let voice = Voice {
//                    model: model.clone(),
//                    speaker: Some(speaker),
//                };
//                voices.insert(voice.get_name(), voice);
//            }
//        } else {
//            let voice = Voice {
//                model: model.clone(),
//                speaker: None,
//            };
//            voices.insert(voice.get_name(), voice);
//        }
//    }
//
//    Ok(voices)
//}
