use poise::command;

use crate::features::tts::{
    make_ephemeral_reply, vc::{get_songbird, join_vc, leave_vc}, PoiseContext
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
    }.into();
    let guild = ctx.guild().expect("we are in a guild").id.into();

    let songbird = get_songbird(ctx.serenity_context()).await;
    {
        if let Some(lock) = songbird.get(guild) {
            let handle = lock.lock().await;
            if let Some(id) = handle.current_channel() {
                if id != vc {
                    ctx.send(make_ephemeral_reply("I'm currently in a VC")).await?;
                    return Ok(());
                } else {
                    ctx.send(make_ephemeral_reply("I'm already in that VC")).await?;
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

    let handle = handle.lock().await;
    // make sure they are the same
    let same_vc = if let Some(bot_vc) = handle.current_channel()
        && bot_vc.0.get() == vc.get()
    {
        true
    } else {
        false
    };

    if same_vc {
        leave_vc(ctx.serenity_context(), guild_id.into()).await?;
    }

    ctx.send(make_ephemeral_reply(
        "You are not in the same VC as the bot",
    ))
    .await?;

    Ok(())
}
