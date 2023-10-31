use anyhow::Result;
mod cli;
mod api;
mod artifacts;
mod interactor;

#[tokio::main]
async fn main() -> Result<()> {
    cli::Cli::run().await
}

