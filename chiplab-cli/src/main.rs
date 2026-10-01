use std::process::ExitCode;

use clap::Parser;

use crate::app::App;
use crate::command::Command;

mod api;
mod app;
mod command;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<ExitCode> {
    let args = Args::parse();
    let app = App::new()?;

    args.command.run(&app).await
}
