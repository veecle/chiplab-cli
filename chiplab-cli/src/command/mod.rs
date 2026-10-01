use std::process::ExitCode;

use crate::app::App;

mod run_test_suite;

#[derive(clap::Subcommand, Debug)]
pub enum Command {
    RunTestSuite(run_test_suite::Args),
}

impl Command {
    pub async fn run(self, app: &App) -> anyhow::Result<ExitCode> {
        match self {
            Self::RunTestSuite(args) => args.run(app).await,
        }
    }
}
