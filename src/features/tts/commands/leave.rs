use poise::command;

use crate::features::tts::{make_ephemeral_reply, vc::leave_vc, PoiseContext};

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
