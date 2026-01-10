use serenity::all::AutocompleteChoice;
use tracing::error;

use crate::{config::Config, database::{self, DatabaseKey}, features::tts::{get_speakers, make_ephemeral_reply, PoiseContext}};

#[poise::command(slash_command)]
pub async fn set_speaker(
    ctx: PoiseContext<'_>,
    #[autocomplete = "autocomplete_speaker"] speaker: Option<String>,
) -> Result<(), anyhow::Error> {
    let (config, db) = {
        let data = ctx.serenity_context().data.read().await;
        (
            data.get::<Config>().expect("Config is initialized").clone(),
            data.get::<DatabaseKey>()
                .expect("Database is initialized")
                .clone(),
        )
    };

    if let Some(speaker) = &speaker {
        let user = database::get_tts_user(&db, ctx.author().id.get()).await?;
        let Ok(speakers) = get_speakers(
            config.piper_server.as_ref().expect("piper server is set"),
            &config.reqwest_client,
            user.model.clone(),
        )
        .await else {
            error!("failed to get speaker list for model {:?}", user.model);
            ctx.send(make_ephemeral_reply("Failed to get the speaker list. Try resetting your model with `/set_model` ?")).await?;
            return Ok(());
        };

        if !speakers.speakers.contains(speaker) {
            ctx.send(make_ephemeral_reply("That isn't a valid speaker"))
                .await?;
            return Ok(());
        }
    }

    database::set_speaker(&db, ctx.author().id.get(), speaker.clone()).await?;

    if let Some(speaker) = speaker {
        ctx.send(make_ephemeral_reply(&format!(
            ":thumbsup: Your speaker is now {speaker}."
        )))
        .await?;
    } else {
        ctx.send(make_ephemeral_reply(
            ":thumbsup: Your speaker is now the default.",
        ))
        .await?;
    }

    Ok(())
}

async fn autocomplete_speaker(
    ctx: PoiseContext<'_>,
    partial: &str,
) -> impl Iterator<Item = AutocompleteChoice> {
    let (config, db) = {
        let data = ctx.serenity_context().data.read().await;
        (
            data.get::<Config>().expect("Config is initialized").clone(),
            data.get::<DatabaseKey>()
                .expect("Database is initialized")
                .clone(),
        )
    };

    let user = match database::get_tts_user(&db, ctx.author().id.get()).await {
        Ok(u) => u,
        Err(e) => {
            error!("error getting tts user: {e}");
            return Default::default();
        }
    };

    let speakers = get_speakers(
        config.piper_server.as_ref().expect("piper server is set"),
        &config.reqwest_client,
        user.model,
    )
    .await;

    match speakers {
        Err(e) => {
            error!("error getting speakers {e}");
            Default::default()
        }
        Ok(m) => {
            let mut m: Vec<&String> = m.speakers
                .iter()
                .filter(|x| x.to_lowercase().starts_with(partial))
                .collect();
            m.sort();
            m.iter()
                .map(|&x| AutocompleteChoice::new(x, x.clone()))
                .collect::<Vec<AutocompleteChoice>>()
                .into_iter()
        }
    }
}

