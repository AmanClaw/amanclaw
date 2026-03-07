use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("amanclaw=info")
        .init();

    tracing::info!("AmanClaw starting...");
    Ok(())
}
