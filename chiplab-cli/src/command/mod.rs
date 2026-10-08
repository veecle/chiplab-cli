use crate::app::App;
use crate::error::Result;

mod run_test_suite;

#[derive(clap::Subcommand, Debug)]
pub enum Command {
    RunTestSuite(run_test_suite::Args),
}

impl Command {
    pub async fn run(self, app: &App) -> Result<()> {
        match self {
            Self::RunTestSuite(args) => args.run(app).await,
        }
    }
}
