use poise::CreateReply;

pub mod commands;
pub mod listener;
pub mod queue;
mod vc;
mod piper;

type PoiseContext<'a> = poise::Context<'a, (), anyhow::Error>;

fn make_ephemeral_reply(msg: &str) -> CreateReply {
    CreateReply::default().ephemeral(true).content(msg)
}

