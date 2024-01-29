use anyhow::Result;
mod api;
mod artifacts;
mod cli;
mod errors;
mod filtering;
mod interactor;

#[tokio::main]
async fn main() -> Result<()> {
    cli::Cli::run().await
}
