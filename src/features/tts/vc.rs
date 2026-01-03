use std::sync::Arc;

use songbird::{error::JoinError, id::{ChannelId, GuildId}, Songbird};
use thiserror::Error;
use serenity::prelude::*;
use tokio::sync::mpsc;

use crate::{config::Config, database::DatabaseKey, features::tts::queue::{self, TTSMessage, TTSSenders}};

pub async fn get_songbird(ctx: &Context) -> Arc<Songbird> {
    songbird::get(ctx).await.expect("Songbird is already initialized")
}

pub async fn join_vc(ctx: &Context, guild: GuildId, channel: ChannelId) -> anyhow::Result<()> {
    let songbird = get_songbird(ctx).await;

    // initialize the queue
    let (senders_lock, config, db) = {
        let data = ctx.data.read().await;
        (
            data.get::<TTSSenders>()
                .expect("TTSSenders has been initialized")
                .clone(),
            data.get::<Config>()
                .expect("Config has been initialized")
                .clone(),
            data.get::<DatabaseKey>()
                .expect("Database has been initialized")
                .clone()
        )
    };

    let (tx, rx) = mpsc::channel::<TTSMessage>(10);
    tokio::task::spawn(queue::speak_message_queue(
        songbird.clone(),
        rx,
        guild,
        config.piper_server.clone(),
        db
    ));

    // put the sender in the senders map
    {
        let mut senders = senders_lock.lock().await;
        senders.insert(guild, tx);
    }

    // join the vc
    songbird.join(guild, channel).await?;

    Ok(())
}

#[derive(Error, Debug)]
pub enum LeaveVCError {
    #[error("No songbird handle exists for the bot in {0}")]
    NoHandle(GuildId),

    #[error("{0}")]
    JoinError(#[from] JoinError)
}

pub async fn leave_vc(ctx: &Context, guild: GuildId) -> Result<(), LeaveVCError> {
    let songbird = get_songbird(ctx).await;

    let handle = if let Some(handle) = songbird.get(guild) {
        handle
    } else {
        return Err(LeaveVCError::NoHandle(guild));
    };

    let mut handle = handle.lock().await;
    handle.leave().await?;
    handle.queue().stop();
    handle.stop();

    // remove sender. this should automatically stop the queue task as well
    let senders_lock = {
        let data = ctx.data.read().await;
        data.get::<TTSSenders>()
            .expect("TTSSenders has been initialized")
            .clone()
    };
    let mut senders = senders_lock.lock().await;
    senders.remove(&guild);

    Ok(())
}

