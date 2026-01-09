use serenity::all::AutocompleteChoice;
use tracing::error;

use crate::{config::Config, database::{self, DatabaseKey}, features::tts::{get_models, make_ephemeral_reply, PoiseContext}};

#[poise::command(slash_command)]
pub async fn set_model(
    ctx: PoiseContext<'_>,
    #[autocomplete = "autocomplete_model"] model: Option<String>,
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

    // get model list
    if let Some(model) = &model {
        let models = get_models(
            config.piper_server.as_ref().expect("piper server is set"),
            &config.reqwest_client,
        )
        .await?;
        if !models.contains(model) {
            ctx.send(make_ephemeral_reply("That isn't a valid model"))
                .await?;
            return Ok(());
        }
    }

    database::set_model(&db, ctx.author().id.get(), model.clone()).await?;

    if let Some(model) = model {
        ctx.send(make_ephemeral_reply(&format!(
            ":thumbsup: Your model is now {model}."
        )))
        .await?;
    } else {
        ctx.send(make_ephemeral_reply(
            ":thumbsup: Your model is now the default.",
        ))
        .await?;
    }

    Ok(())
}

async fn autocomplete_model(
    ctx: PoiseContext<'_>,
    partial: &str,
) -> impl Iterator<Item = AutocompleteChoice> {
    let config = {
        let data = ctx.serenity_context().data.read().await;
        data.get::<Config>()
            .expect("Database is initialized")
            .clone()
    };

    let models = get_models(
        config.piper_server.as_ref().expect("piper server is set"),
        &config.reqwest_client,
    )
    .await;

    match models {
        Err(e) => {
            error!("error getting models {e}");
            Default::default()
        }
        Ok(m) => {
            let mut m: Vec<&String> = m
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

