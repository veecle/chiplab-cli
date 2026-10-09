use std::io::Write;
use std::path::PathBuf;

use camino::{Utf8Path, Utf8PathBuf};
use chiplab_api::apis::default_api;
use chiplab_api::models::{Board, BoardModel, RunTestSuiteRequest, WaitForJobResponse};

use crate::api;
use crate::api::models::TestSuiteResult;
use crate::api::models::test_suite_result::Summary;
use crate::app::App;
use crate::error::{self, Result, ResultExt, Silent};

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

    /// The board model to simulate, e.g. `stm32f4_discovery`, or the path to a JSON file with the
    /// full board definition.
    ///
    /// See <https://chiplab.veecle.ai/mcp/resources/docs/boards.md> for the available boards and
    /// the board definition format.
    #[arg(short, long, value_name = "MODEL|FILE", value_parser = parse_board)]
    board: Board,

    /// Directory to download the run's artifacts to. If missing, no artifacts are downloaded.
    ///
    /// The given directory is created if it does not already exist.
    /// Existing files with the same names are overwritten.
    #[arg(long, value_name = "DIR")]
    download_dir: Option<Utf8PathBuf>,

    /// Print the test suite result as a single JSON line.
    #[arg(long)]
    json: bool,
}

/// Exit status for when the test suite failed to run to completion.
const EXIT_INCOMPLETE: u8 = 3;

/// Exit status for when the test suite ran to completion but some test cases failed.
const EXIT_TESTS_FAILED: u8 = 4;

/// How a test suite run went overall.
#[derive(serde::Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Outcome {
    /// No test failed, though some may have been skipped.
    Passed,

    /// Some tests failed.
    Failed,

    /// The suite did not run to completion.
    Incomplete,
}

/// The result printed with `--json`.
#[derive(serde::Serialize, Debug)]
struct TestSuiteReport {
    /// The ID of the test suite run job.
    job_id: String,
    /// `passed`, `failed`, or `incomplete`.
    outcome: Outcome,
    /// Missing if the suite did not run to completion.
    summary: Option<Summary>,
    /// The job error, present only if the job itself failed rather than the suite.
    error: Option<String>,
}

impl TestSuiteReport {
    /// Builds a report, deriving the outcome from the summary.
    fn new(job_id: String, summary: Option<Summary>, error: Option<String>) -> Self {
        let outcome = match &summary {
            None => Outcome::Incomplete,
            Some(summary) if summary.failed == 0 => Outcome::Passed,
            Some(_) => Outcome::Failed,
        };

        Self {
            job_id,
            outcome,
            summary,
            error,
        }
    }

    /// Prints the report as a JSON line, or as plain text.
    fn print(&self, json: bool) -> Result<()> {
        if json {
            let mut stdout = std::io::stdout().lock();
            serde_json::to_writer(&mut stdout, self)?;
            writeln!(stdout)?;

            return Ok(());
        }

        if let Some(summary) = &self.summary
            && self.outcome == Outcome::Passed
        {
            println!("{}", summarize(summary));
        }

        Ok(())
    }

    /// The exit status matching the outcome.
    fn exit_code(&self) -> u8 {
        match self.outcome {
            Outcome::Passed => 0,
            Outcome::Failed => EXIT_TESTS_FAILED,
            Outcome::Incomplete => EXIT_INCOMPLETE,
        }
    }

    /// The error matching the outcome, carrying its exit status.
    ///
    /// With `json` the error is [`Silent`], as the printed report already describes it.
    fn result(&self, json: bool) -> Result<()> {
        if self.outcome == Outcome::Passed {
            return Ok(());
        }

        let code = self.exit_code();

        if json {
            return Err(Silent(code).into());
        }

        let message = match (&self.summary, &self.error) {
            (Some(summary), _) => summarize(summary).to_string(),
            (None, Some(error)) => format!("test suite run failed: {error}"),
            (None, None) => "test suite did not run to completion".to_owned(),
        };

        Ok(Err(message).with_exit_code(code)?)
    }
}

/// Parses a board model by its API name.
///
/// The generated `BoardModel` has no `FromStr`, so this goes through serde to reuse its renames,
/// keeping the accepted names and the "expected one of" error in sync with the spec.
fn parse_board_model(model: &str) -> Result<BoardModel, serde_json::Error> {
    serde_json::from_value(serde_json::Value::String(model.to_owned()))
}

/// Parses a board either by its model's API name, or from a JSON file with the full definition.
fn parse_board(value: &str) -> Result<Board, String> {
    let model_error = match parse_board_model(value) {
        Ok(model) => return Ok(Board::new(model)),
        Err(error) => error,
    };

    let path = Utf8Path::new(value);
    let contents = match std::fs::read(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(format!(
                "{model_error}\nor a path to a board definition file, but failed to read `{path}`:\n{error}"
            ));
        }
        Err(error) => return Err(format!("failed to read `{path}`:\n{error}")),
    };

    let mut deserializer = serde_json::Deserializer::from_slice(&contents);

    let board = serde_ignored::deserialize(
        &mut deserializer,
        |key| eprintln!("warning: dropping unknown key `{key}` in `{path}`, this may be an error in future versions"),
    )
    .and_then(|board| deserializer.end().map(|()| board))
    .map_err(|error| format!("failed to parse `{path}`:\n{error}"))?;

    Ok(board)
}

impl Args {
    pub async fn run(self, app: &App) -> Result<()> {
        let Self {
            project_id,
            firmware,
            test_suite,
            board,
            download_dir,
            json,
        } = self;

        let firmware_artifact_id = api::upload(&app.api, &project_id, &firmware)
            .await
            .wrap_error("failed to upload the firmware")?;
        let suite_artifact_id = api::upload(&app.api, &project_id, &test_suite)
            .await
            .wrap_error("failed to upload the test suite")?;

        let request = RunTestSuiteRequest::new(board, firmware_artifact_id, suite_artifact_id);
        let job_id = default_api::run_test_suite(&app.api, &project_id, request)
            .await
            .map_err(api::error)
            .wrap_error("failed to start the test suite run")?
            .run_test_suite_id;

        eprintln!("waiting for test suite run `{job_id}`");
        let (report, artifacts) = loop {
            let response = default_api::wait_for_job(&app.api, &job_id, None)
                .await
                .map_err(api::error)
                .wrap_error("failed to wait for the test suite run")?;

            match response {
                WaitForJobResponse::Running(_) => continue,
                WaitForJobResponse::Succeeded(job) => {
                    let result: TestSuiteResult = serde_json::from_value(job.result)
                        .wrap_error("failed to parse the test suite result")?;

                    let report = TestSuiteReport::new(job.job_id, result.summary, None);
                    break (report, result.artifacts);
                }
                WaitForJobResponse::Failed(job) => {
                    let report = TestSuiteReport::new(job.job_id, None, Some(job.error));
                    break (report, Vec::default());
                }
            }
        };

        let download =
            maybe_download_artifacts(app, &job_id, download_dir.as_deref(), &artifacts).await;

        report.print(json)?;

        match (report.result(json), download) {
            (result, Ok(())) => result,
            (Ok(()), Err(error)) => Err(error),
            // Report the outcome before the download error, but exit with the outcome's status.
            (Err(outcome), Err(error)) => {
                error::report(&outcome);
                error::report(&error);
                Err(Silent(report.exit_code()).into())
            }
        }
    }
}

async fn maybe_download_artifacts(
    app: &App,
    job_id: &str,
    dir: Option<&Utf8Path>,
    artifacts: &[String],
) -> Result<()> {
    let Some(dir) = dir else {
        return Ok(());
    };

    if artifacts.is_empty() {
        eprintln!("no artifacts to download");
        return Ok(());
    }

    api::download_all(&app.api, job_id, artifacts, dir)
        .await
        .wrap_error("failed to download the artifacts")?
        .iter()
        .for_each(|path| eprintln!("downloaded `{path}`"));

    Ok(())
}

fn summarize(summary: &Summary) -> impl std::fmt::Display {
    let Summary {
        planned,
        passed,
        failed,
    } = *summary;
    let skipped = summary.skipped();

    std::fmt::from_fn(move |f| {
        let skipped = if skipped > 0 {
            format_args!(", {skipped} skipped")
        } else {
            format_args!("")
        };

        if failed == 0 {
            write!(
                f,
                "test suite passed: {passed} of {planned} tests passed{skipped}"
            )?;
        } else {
            write!(
                f,
                "test suite failed: {failed} of {planned} tests failed, {passed} passed{skipped}"
            )?;
        }

        Ok(())
    })
}
