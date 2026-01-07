use poise::command;
use serenity::all::AutocompleteChoice;
use tracing::error;

use crate::{
    config::Config,
    database::{self, DatabaseKey},
    features::tts::{
        PoiseContext, get_models, get_speakers, make_ephemeral_reply,
        vc::{get_songbird, join_vc, leave_vc},
    },
};

#[command(slash_command, guild_only)]
pub async fn join(ctx: PoiseContext<'_>) -> Result<(), anyhow::Error> {
    // check that the user is a vc
    let vc = if let Some(vc) = ctx
        .guild()
        .expect("this is a guild only command")
        .voice_states
        .get(&ctx.author().id)
    {
        vc.channel_id.expect("we are in a guild")
    } else {
        ctx.send(make_ephemeral_reply("You are not in a VC"))
            .await?;
        return Ok(());
    }
    .into();
    let guild = ctx.guild().expect("we are in a guild").id.into();

    let songbird = get_songbird(ctx.serenity_context()).await;
    {
        if let Some(lock) = songbird.get(guild) {
            let handle = lock.lock().await;
            if let Some(id) = handle.current_channel() {
                if id != vc {
                    ctx.send(make_ephemeral_reply("I'm currently in a VC"))
                        .await?;
                    return Ok(());
                } else {
                    ctx.send(make_ephemeral_reply("I'm already in that VC"))
                        .await?;
                    return Ok(());
                }
            }
        }
    }

    join_vc(ctx.serenity_context(), guild, vc).await?;

    ctx.send(make_ephemeral_reply(":thumbsup:")).await?;

    Ok(())
}

#[command(slash_command, guild_only)]
pub async fn leave(ctx: PoiseContext<'_>) -> Result<(), anyhow::Error> {
    let songbird = songbird::get(ctx.serenity_context())
        .await
        .expect("songbird is already initialized");

    // check if we are in a vc
    let handle = if let Some(handle) = songbird.get(ctx.guild_id().expect("guild only command")) {
        handle
    } else {
        ctx.send(make_ephemeral_reply("I'm not in a vc")).await?;
        return Ok(());
    };

    let guild_id = ctx.guild().expect("we are in a guild").id;

    // get the vc the user is in (if any)
    let vc = if let Some(vc) = ctx
        .guild()
        .expect("we are in a guild")
        .voice_states
        .get(&ctx.author().id)
    {
        vc.channel_id.expect("we are in a guild")
    } else {
        ctx.send(make_ephemeral_reply("You are not in the same VC as me"))
            .await?;
        return Ok(());
    };

    // make sure they are the same
    let same_vc = {
        let handle = handle.lock().await;
        if let Some(bot_vc) = handle.current_channel()
            && bot_vc.0.get() == vc.get()
        {
            true
        } else {
            false
        }
    };

    if same_vc {
        leave_vc(ctx.serenity_context(), guild_id.into()).await?;
        ctx.send(make_ephemeral_reply(":thumbsup:")).await?;
        return Ok(());
    }

    ctx.send(make_ephemeral_reply(
        "You are not in the same VC as the bot",
    ))
    .await?;

    Ok(())
}

#[command(slash_command)]
pub async fn set_nick(ctx: PoiseContext<'_>, nick: Option<String>) -> Result<(), anyhow::Error> {
    let db = {
        let data = ctx.serenity_context().data.read().await;
        data.get::<DatabaseKey>()
            .expect("Database is initialized")
            .clone()
    };
    database::set_nick(&db, ctx.author().id.get(), nick).await?;

    ctx.send(make_ephemeral_reply(":thumpsup:")).await?;

    Ok(())
}

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
        let models = get_models(&config.piper_server, &config.reqwest_client).await?;
        if !models.contains(model) {
            ctx.send(make_ephemeral_reply("That isn't a valid model"))
                .await?;
            return Ok(());
        }
    }

    database::set_model(&db, ctx.author().id.get(), model).await?;

    ctx.send(make_ephemeral_reply(":thumbsup:")).await?;
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

    let models = get_models(&config.piper_server, &config.reqwest_client).await;

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

#[poise::command(slash_command)]
pub async fn set_speaker(
    ctx: PoiseContext<'_>,
    #[autocomplete = "autocomplete_speaker"]
    speaker: Option<String>
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
        let speakers =
            get_speakers(&config.piper_server, &config.reqwest_client, user.model).await?;

        if !speakers.contains(speaker) {
            ctx.send(make_ephemeral_reply("That isn't a valid speaker"))
                .await?;
            return Ok(());
        }
    }

    database::set_speaker(&db, ctx.author().id.get(), speaker).await?;

    ctx.send(make_ephemeral_reply(":thumbsup:")).await?;
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

    let speakers = get_speakers(&config.piper_server, &config.reqwest_client, user.model).await;

    match speakers {
        Err(e) => {
            error!("error getting speakers {e}");
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
