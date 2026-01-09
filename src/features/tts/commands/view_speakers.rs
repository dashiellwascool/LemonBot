use poise::CreateReply;
use serde::{Deserialize, Serialize};
use serenity::{
    all::{
        Context, CreateActionRow, CreateButton, CreateEmbed, CreateInteractionResponse,
        CreateInteractionResponseMessage, EventHandler, Interaction,
    },
    async_trait,
};
use tracing::error;

use crate::{
    config::Config,
    database::{self, DatabaseKey},
    features::tts::{PoiseContext, get_speakers},
};

pub struct ViewSpeakersListener;

#[derive(Serialize, Deserialize)]
struct ButtonId {
    page: i32,
}

#[async_trait]
impl EventHandler for ViewSpeakersListener {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let Some(interaction) = interaction.message_component() else {
            return;
        };
        let id = &interaction.data.custom_id;
        if !id.starts_with("view_speakers") {
            return;
        }

        let Ok(button_id) = serde_json::from_str::<ButtonId>(&id[13..]) else {
            error!("failed to parse view_speakers button id");
            return;
        };

        let Ok((embed, components)) =
            make_embed(&ctx, interaction.user.id.get(), button_id.page).await
        else {
            error!("failed to make view_speakers embed");
            return;
        };

        if interaction
            .create_response(
                ctx.http,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .embed(embed)
                        .components(vec![components])
                        .ephemeral(true),
                ),
            )
            .await
            .is_err()
        {
            error!("view_models interaction failed to update");
            return;
        };
    }
}

#[poise::command(slash_command)]
pub async fn view_speakers(ctx: PoiseContext<'_>) -> Result<(), anyhow::Error> {
    let (embed, components) = make_embed(ctx.serenity_context(), ctx.author().id.get(), 0).await?;
    ctx.send(
        CreateReply::default()
            .embed(embed)
            .components(vec![components])
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

const PAGE_SIZE: usize = 10;

async fn make_embed(
    ctx: &Context,
    user_id: u64,
    page: i32,
) -> Result<(CreateEmbed, CreateActionRow), anyhow::Error> {
    let (config, db) = {
        let data = ctx.data.read().await;
        (
            data.get::<Config>().expect("Config is initialized").clone(),
            data.get::<DatabaseKey>()
                .expect("Database is initialized")
                .clone(),
        )
    };

    let user = database::get_tts_user(&db, user_id).await?;

    let mut speakers = get_speakers(
        config.piper_server.as_ref().expect("piper server is set"),
        &config.reqwest_client,
        user.model,
    )
    .await?;

    let mut page_text = String::new();
    let mut total_pages = 0;
    let mut real_page = 0;
    if !speakers.speakers.is_empty() {
        total_pages = speakers.speakers.len().div_ceil(PAGE_SIZE);
        real_page = page.rem_euclid(total_pages as i32) as usize;

        speakers.speakers.sort();
        let iter = speakers.speakers.iter();
        for speaker in iter.skip(real_page * PAGE_SIZE).take(PAGE_SIZE) {
            page_text += "- ";
            page_text += speaker;
            page_text += "\n";
        }
    } else {
        page_text = "There are no speakers for this model.".to_string();
    }

    let next_button_id = serde_json::to_string(&ButtonId { page: real_page as i32 + 1 })?;
    let prev_button_id = serde_json::to_string(&ButtonId { page: real_page as i32 - 1 })?;

    let components = CreateActionRow::Buttons(vec![
        CreateButton::new(format!("view_speakers {}", prev_button_id)).emoji('◀'),
        CreateButton::new(format!("view_speakers {}", next_button_id)).emoji('▶')
    ]);

    Ok((
        CreateEmbed::new()
            .title(format!("Speaker List for {} ({}/{})", speakers.model, (real_page + 1).min(total_pages), total_pages))
            .description(page_text)
            .color(15844367),
        components,
    ))
}
