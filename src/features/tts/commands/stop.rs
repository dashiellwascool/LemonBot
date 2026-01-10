use crate::features::tts::{PoiseContext, make_ephemeral_reply};

#[derive(Debug, poise::ChoiceParameter, Default)]
enum Choices {
    #[name = "Current message"]
    #[default]
    Current,

    #[name = "All messages"]
    All,
}

#[poise::command(slash_command)]
pub async fn stop(ctx: PoiseContext<'_>, messages: Option<Choices>) -> anyhow::Result<()> {
    let messages = messages.unwrap_or_default();

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
    {
        let handle = handle.lock().await;
        if let Some(bot_vc) = handle.current_channel()
            && bot_vc.0.get() == vc.get()
        {
            ctx.send(make_ephemeral_reply(":thumbsup:")).await?;
            match messages {
                Choices::Current => {
                    _ = handle.queue().skip();
                }
                Choices::All => handle.queue().stop(),
            }
            return Ok(());
        }
    }

    ctx.send(make_ephemeral_reply("we are not in the same vc"))
        .await?;
    Ok(())
}
