use poise::command;

use crate::features::tts::{make_ephemeral_reply, vc::{get_songbird, join_vc}, PoiseContext};

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
