use std::process::ExitCode;

use clap::Parser;

use crate::app::App;
use crate::command::Command;

mod app;
mod command;

#[derive(Parser, Debug)]
#[command(
    version,
    about,
    after_help = "Authenticate by setting `CHIPLAB_TOKEN` to a Chiplab personal access token (`veecle_pat_…`)."
)]
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
