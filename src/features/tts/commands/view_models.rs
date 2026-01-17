use poise::CreateReply;
use serde::{Deserialize, Serialize};
use serenity::{all::{Context, CreateActionRow, CreateButton, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage, EventHandler, Interaction}, async_trait};
use tracing::error;

use crate::{
    config::Config,
    features::tts::{piper::get_models, PoiseContext},
};

pub struct ViewModelsListener;

#[derive(Deserialize, Serialize)]
struct ButtonId {
    page: i32
}

#[async_trait]
impl EventHandler for ViewModelsListener {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let Some(interaction) = interaction.message_component() else { return; };
        let id = &interaction.data.custom_id;
        if !id.starts_with("view_models") {
            return;
        }
        let Ok(button_id) = serde_json::from_str::<ButtonId>(&id[12..]) else {
            error!("failed to parse view_models button id");
            return;
        };

        let Ok((embed, components)) = make_embed(&ctx, button_id.page).await else {
            error!("failed to make view_models embed");
            return;
        };

        if interaction.create_response(ctx.http, CreateInteractionResponse::UpdateMessage(
            CreateInteractionResponseMessage::new().embed(embed).components(vec![components]).ephemeral(true)
        )).await.is_err() {
            error!("view_models interaction failed to update");
            return;
        };
    }
}

#[poise::command(slash_command)]
pub async fn view_models(ctx: PoiseContext<'_>) -> Result<(), anyhow::Error> {
    let (embed, components) = make_embed(ctx.serenity_context(), 0).await?;
    ctx.send(CreateReply::default().embed(embed).components(vec![components]).ephemeral(true)).await?;

    Ok(())
}

const PAGE_SIZE: usize = 10;

async fn make_embed(ctx: &Context, page: i32) -> Result<(CreateEmbed, CreateActionRow), anyhow::Error> {
    let config = {
        let data = ctx.data.read().await;
        data.get::<Config>().expect("Config is initialized").clone()
    };

    let mut models = get_models(
        config.piper_server.as_ref().expect("this is initialized"),
        &config.reqwest_client,
    )
    .await?;

    let total_pages = models.len().div_ceil(PAGE_SIZE);
    let page = page.rem_euclid(total_pages as i32) as usize;

    models.sort();

    let mut page_text = String::from("[You can hear demos of the models here!](https://rhasspy.github.io/piper-samples/) If you want a model added, tell my owner!\n\n");
    let iter = models.iter();
    for model in iter.skip(page * PAGE_SIZE).take(PAGE_SIZE) {
        page_text += "- ";
        page_text += model;
        page_text += "\n";
    }

    let next_button_id = serde_json::to_string(&ButtonId { page: page as i32 + 1 })?;
    let prev_button_id = serde_json::to_string(&ButtonId { page: page as i32 - 1 })?;

    let components = CreateActionRow::Buttons(vec![
        CreateButton::new(format!("view_models {}", prev_button_id)).emoji('◀'),
        CreateButton::new(format!("view_models {}", next_button_id)).emoji('▶')
    ]);

    Ok((CreateEmbed::new().title(format!("Model List ({}/{})", page + 1, total_pages)).description(page_text).color(15844367), components))
}
