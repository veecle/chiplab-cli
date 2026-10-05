/// The result of a completed test suite run.
#[derive(serde::Deserialize, Debug)]
pub struct TestSuiteResult {
    /// Missing if the suite did not run to completion, e.g. because it did not compile or exited
    /// early.
    pub summary: Option<Summary>,

    /// Names of the artifacts the run produced.
    #[serde(default)]
    pub artifacts: Vec<String>,
}

/// The pass/fail counts of a completed test suite run.
#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct Summary {
    /// Includes skipped tests, which are neither passed nor failed.
    pub planned: u64,
    pub passed: u64,
    pub failed: u64,
}

/// How a test suite run went overall.
#[derive(serde::Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    /// No test failed, though some may have been skipped.
    Passed,

    /// Some tests failed.
    Failed,

    /// The suite did not run to completion.
    Incomplete,
}

impl Summary {
    /// The number of planned tests that neither passed nor failed.
    ///
    /// Saturates rather than panicking if the server reports more passed and failed tests than
    /// were planned, since the counts are only used for display.
    pub fn skipped(&self) -> u64 {
        self.planned
            .saturating_sub(self.passed)
            .saturating_sub(self.failed)
    }

    /// Returns the overall outcome of the run.
    ///
    /// A summary only exists for runs that completed, so this never returns [`Outcome::Incomplete`].
    pub fn outcome(&self) -> Outcome {
        if self.failed == 0 {
            Outcome::Passed
        } else {
            Outcome::Failed
        }
    }
}
