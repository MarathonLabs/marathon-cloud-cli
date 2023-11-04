use anyhow::Result;
mod cli;
mod api;
mod artifacts;
mod interactor;
mod errors;

#[tokio::main]
async fn main() -> Result<()> {
    cli::Cli::run().await
}

