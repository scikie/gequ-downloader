mod config;
mod database;
mod models;
mod crawlers;
mod cli;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    cli::run().await
}