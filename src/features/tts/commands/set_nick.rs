use poise::command;

use crate::{database::{self, DatabaseKey}, features::tts::{make_ephemeral_reply, PoiseContext}};

#[command(slash_command)]
pub async fn set_nick(ctx: PoiseContext<'_>, nick: Option<String>) -> Result<(), anyhow::Error> {
    let db = {
        let data = ctx.serenity_context().data.read().await;
        data.get::<DatabaseKey>()
            .expect("Database is initialized")
            .clone()
    };
    database::set_nick(&db, ctx.author().id.get(), nick).await?;

    ctx.send(make_ephemeral_reply(":thumbsup:")).await?;

    Ok(())
}
