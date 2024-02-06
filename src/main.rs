use anyhow::Result;
use marathon_cloud::cli;

#[tokio::main]
async fn main() -> Result<()> {
    cli::Cli::run().await
}
