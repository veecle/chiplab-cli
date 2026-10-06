use std::path::PathBuf;
use std::process::ExitCode;

use crate::app::App;

/// Run a TypeScript test suite against a firmware binary on a simulated board.
///
/// The firmware and test suite are uploaded to the project, then run in a sandboxed simulation of
/// the selected board.
///
/// Exits with status 3 if the test suite failed to run to completion, or status 4 if it ran to
/// completion but one or more test cases failed.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// The Chiplab project to upload the files to and run the test suite in.
    #[arg(short, long = "project", value_name = "PROJECT_ID")]
    project_id: String,

    /// The firmware ELF to run on the board.
    #[arg(short, long, value_name = "FILE")]
    firmware: PathBuf,

    /// The TypeScript test suite to run against the firmware.
    ///
    /// See <https://chiplab.veecle.ai/mcp/resources/docs/testing.md> for more info on writing a
    /// test suite.
    #[arg(short, long, value_name = "FILE")]
    test_suite: PathBuf,

    /// The board model to simulate, e.g. `stm32f4_discovery`.
    ///
    /// See <https://chiplab.veecle.ai/mcp/resources/docs/boards.md> for the available boards.
    #[arg(short, long, value_name = "MODEL")]
    board: String,
}

impl Args {
    pub async fn run(self, _app: &App) -> anyhow::Result<ExitCode> {
        let Self {
            project_id: _,
            firmware: _,
            test_suite: _,
            board: _,
        } = self;

        anyhow::bail!("todo")
    }
}
