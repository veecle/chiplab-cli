use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, bail};
use camino::{Utf8Path, Utf8PathBuf};
use chiplab_api::apis::default_api;
use chiplab_api::models::{Board, BoardModel, RunTestSuiteRequest, WaitForJobResponse};

use crate::api;
use crate::api::models::TestSuiteResult;
use crate::api::models::test_suite_result::{Outcome, Summary};
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
    #[arg(short, long, value_name = "MODEL", value_parser = parse_board_model)]
    board: BoardModel,

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
    /// Prints the report as a JSON line, or as plain text.
    fn print(&self, json: bool) -> anyhow::Result<()> {
        if json {
            println!("{}", serde_json::to_string(self)?);
            return Ok(());
        }

        match (&self.summary, &self.error) {
            (Some(summary), _) if self.outcome == Outcome::Passed => {
                println!("{}", summarize(summary));
            }
            (Some(summary), _) => eprintln!("error: {}", summarize(summary)),
            (None, Some(error)) => eprintln!("error: test suite run failed: {error}"),
            (None, None) => eprintln!("error: test suite did not run to completion"),
        }

        Ok(())
    }

    /// The exit status matching the outcome.
    fn exit_code(&self) -> ExitCode {
        match self.outcome {
            Outcome::Passed => ExitCode::SUCCESS,
            Outcome::Failed => ExitCode::from(EXIT_TESTS_FAILED),
            Outcome::Incomplete => ExitCode::from(EXIT_INCOMPLETE),
        }
    }
}

/// Parses a board model by its API name.
///
/// The generated `BoardModel` has no `FromStr`, so this goes through serde to reuse its renames,
/// keeping the accepted names and the "expected one of" error in sync with the spec.
fn parse_board_model(model: &str) -> Result<BoardModel, serde_json::Error> {
    serde_json::from_value(serde_json::Value::String(model.to_owned()))
}

impl Args {
    pub async fn run(self, app: &App) -> anyhow::Result<ExitCode> {
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
            .context("failed to upload the firmware")?;
        let suite_artifact_id = api::upload(&app.api, &project_id, &test_suite)
            .await
            .context("failed to upload the test suite")?;

        let request =
            RunTestSuiteRequest::new(Board::new(board), firmware_artifact_id, suite_artifact_id);
        let job_id = default_api::run_test_suite(&app.api, &project_id, request)
            .await
            .map_err(api::error)
            .context("failed to start the test suite run")?
            .run_test_suite_id;

        eprintln!("waiting for test suite run `{job_id}`");
        let (report, artifacts) = loop {
            let response = default_api::wait_for_job(&app.api, &job_id, None)
                .await
                .map_err(api::error)
                .context("failed to wait for the test suite run")?;

            match response {
                WaitForJobResponse::Running(_) => continue,
                WaitForJobResponse::Succeeded(job) => {
                    let result: TestSuiteResult = serde_json::from_value(job.result)
                        .context("failed to parse the test suite result")?;

                    let outcome = result
                        .summary
                        .as_ref()
                        .map_or(Outcome::Incomplete, Summary::outcome);

                    let report = TestSuiteReport {
                        job_id: job.job_id,
                        outcome,
                        summary: result.summary,
                        error: None,
                    };
                    break (report, result.artifacts);
                }
                WaitForJobResponse::Failed(job) => {
                    let report = TestSuiteReport {
                        job_id: job.job_id,
                        outcome: Outcome::Incomplete,
                        summary: None,
                        error: Some(job.error),
                    };
                    break (report, Vec::default());
                }
            }
        };

        let download =
            maybe_download_artifacts(app, &job_id, download_dir.as_deref(), &artifacts).await;

        report.print(json)?;
        let exit_code = report.exit_code();

        if let Err(error) = download {
            if exit_code == ExitCode::SUCCESS {
                bail!(error);
            }

            eprintln!("error: {error:#}");
        }

        Ok(exit_code)
    }
}

async fn maybe_download_artifacts(
    app: &App,
    job_id: &str,
    dir: Option<&Utf8Path>,
    artifacts: &[String],
) -> anyhow::Result<()> {
    let Some(dir) = dir else {
        return Ok(());
    };

    if artifacts.is_empty() {
        eprintln!("no artifacts to download");
        return Ok(());
    }

    api::download_all(&app.api, job_id, artifacts, dir)
        .await
        .context("failed to download the artifacts")?
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{Outcome, Summary, TestSuiteReport};

    #[test]
    fn serialize_test_suite_report() {
        struct Test {
            name: &'static str,
            input: TestSuiteReport,
            output: serde_json::Value,
        }

        vec![
            Test {
                name: "passed run should include the summary",
                input: TestSuiteReport {
                    job_id: "job_1".to_string(),
                    outcome: Outcome::Passed,
                    summary: Some(Summary {
                        planned: 2,
                        passed: 2,
                        failed: 0,
                    }),
                    error: None,
                },
                output: json!({
                    "job_id": "job_1",
                    "outcome": "passed",
                    "summary": { "planned": 2, "passed": 2, "failed": 0 },
                    "error": null,
                }),
            },
            Test {
                name: "failed run with skipped tests should include the summary",
                input: TestSuiteReport {
                    job_id: "job_1".to_string(),
                    outcome: Outcome::Failed,
                    summary: Some(Summary {
                        planned: 3,
                        passed: 1,
                        failed: 1,
                    }),
                    error: None,
                },
                output: json!({
                    "job_id": "job_1",
                    "outcome": "failed",
                    "summary": { "planned": 3, "passed": 1, "failed": 1 },
                    "error": null,
                }),
            },
            Test {
                name: "incomplete run should have no summary",
                input: TestSuiteReport {
                    job_id: "job_1".to_string(),
                    outcome: Outcome::Incomplete,
                    summary: None,
                    error: None,
                },
                output: json!({
                    "job_id": "job_1",
                    "outcome": "incomplete",
                    "summary": null,
                    "error": null,
                }),
            },
            Test {
                name: "failed job should include the error",
                input: TestSuiteReport {
                    job_id: "job_1".to_string(),
                    outcome: Outcome::Incomplete,
                    summary: None,
                    error: Some("sandbox crashed".to_string()),
                },
                output: json!({
                    "job_id": "job_1",
                    "outcome": "incomplete",
                    "summary": null,
                    "error": "sandbox crashed",
                }),
            },
        ]
        .into_iter()
        .for_each(|test| {
            let got = serde_json::to_value(&test.input).unwrap();
            assert_eq!(got, test.output, "{}", test.name);
        });
    }
}
