use std::process::ExitCode;

use clap::Parser;

use crate::app::App;
use crate::command::Command;

mod api;
mod app;
mod command;
mod error;

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
async fn main() -> ExitCode {
    let args = Args::parse();

    let result: error::Result<ExitCode> = async {
        let app = App::new()?;
        Ok(args.command.run(&app).await?)
    }
    .await;

    result.unwrap_or_else(|error| error::report(&error))
}
