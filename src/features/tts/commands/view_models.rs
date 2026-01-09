use crate::{config::Config, features::tts::{get_models, PoiseContext}};

#[poise::command(slash_command)]
pub async fn view_models(ctx: PoiseContext<'_>) -> Result<(), anyhow::Error> {
    let config = {
        let data = ctx.serenity_context().data.read().await;
        data.get::<Config>().expect("Config is initialized").clone()
    };

    let models = get_models(
        config.piper_server.as_ref().expect("this is initialized"),
        &config.reqwest_client,
    )
    .await?;

    let mut pages: Vec<String> = Vec::new();

    let mut page = String::new();
    for (i, item) in models.iter().enumerate() {
        if i % 5 == 0 && !page.is_empty() {
            pages.push(page.clone());
            page.clear();
        }
        page += "- ";
        page += item;
        page += "\n";
    }
    if !page.is_empty() {
        pages.push(page);
    }

    todo!()
}
