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

    pub fn outcome(&self) -> Outcome {
        if self.failed == 0 {
            Outcome::Passed
        } else {
            Outcome::Failed
        }
    }
}

impl std::fmt::Display for Summary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            planned,
            passed,
            failed,
        } = *self;
        let skipped = self.skipped();
        let skipped = if skipped > 0 {
            format_args!(", {skipped} skipped")
        } else {
            format_args!("")
        };

        if failed == 0 {
            write!(
                f,
                "test suite passed: {passed} of {planned} tests passed{skipped}"
            )
        } else {
            write!(
                f,
                "test suite failed: {failed} of {planned} tests failed, {passed} passed{skipped}"
            )
        }
    }
}
