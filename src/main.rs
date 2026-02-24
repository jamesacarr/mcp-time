mod server;

use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    // TODO: Wire up TimeServer with stdio transport in Wave 2
    // TimeServer::new().serve(stdio()).await?.waiting().await?;

    Ok(())
}
