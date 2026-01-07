use lemonbot::{config::Config, start_bot};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();

    info!("Lemon Bot v{}", env!("CARGO_PKG_VERSION"));
    info!("Made with ❤ by Dashiell");

    _ = dotenvy::dotenv();

    let config = Config::init()?;

    start_bot(config).await?;

    Ok(())
}
