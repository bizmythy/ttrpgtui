use clap::Parser;
use cli::Cli;

use crate::app::App;

mod action;
mod app;
mod cli;
mod components;
mod config;
mod dnd_beyond;
mod errors;
mod logging;
mod models;
mod storage;
mod tui;
mod ui;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    crate::errors::init()?;

    let args = Cli::parse();
    crate::logging::init(args.data_dir.as_deref())?;

    let mut app = App::new(args.tick_rate, args.frame_rate, args.data_dir)?;
    app.run().await?;
    Ok(())
}
