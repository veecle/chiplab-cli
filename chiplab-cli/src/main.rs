use std::process::ExitCode;

use clap::Parser;
use clap::error::ErrorKind;

use crate::app::App;
use crate::command::Command;

mod api;
mod app;
mod command;
mod error;
mod progress;

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
    let args = Args::try_parse().unwrap_or_else(|error| {
        // A bare invocation gets the banner before the usual help; an explicit `--help` stays
        // plain and goes through clap's normal path.
        if error.kind() == ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand {
            eprintln!("{BANNER}");
        }
        error.exit()
    });

    let result: error::Result<()> = async {
        let app = App::new()?;
        args.command.run(&app).await
    }
    .await;

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => error::report(&error),
    }
}

const BANNER: &str = r"
 $$$$$$\  $$\       $$\           $$\           $$\
$$  __$$\ $$ |      \__|          $$ |          $$ |
$$ /  \__|$$$$$$$\  $$\  $$$$$$\  $$ | $$$$$$\  $$$$$$$\
$$ |      $$  __$$\ $$ |$$  __$$\ $$ | \____$$\ $$  __$$\
$$ |      $$ |  $$ |$$ |$$ /  $$ |$$ | $$$$$$$ |$$ |  $$ |
$$ |  $$\ $$ |  $$ |$$ |$$ |  $$ |$$ |$$  __$$ |$$ |  $$ |
\$$$$$$  |$$ |  $$ |$$ |$$$$$$$  |$$ |\$$$$$$$ |$$$$$$$  |
 \______/ \__|  \__|\__|$$  ____/ \__| \_______|\_______/
                        $$ |
                        $$ |
                        \__|
";
