//! Fixture tests running the `chiplab-cli` binary.
//!
//! Each directory in `tests/cases/` containing an `args` file is one case, named by its path
//! relative to `tests/cases/`.
//!
//! The case layout and the workflows are documented in the "Testing" section of `CONTRIBUTING.md`;
//! keep it up to date when changing them.

use std::path::{Path, PathBuf};

// Shared with the binary. Only part of it is used here, and its unit tests don't run under this
// harness.
#[allow(dead_code, unused_imports)]
#[path = "../../src/error.rs"]
mod error;
mod proxy;
mod run;
mod temp_dir;
mod terminal;
mod terminal_svg;

use crate::error::Result;
use crate::run::{Stderr, run};

const CASES: &str = "tests/cases";

/// Stands in for the path of the case's temporary directory in `args` and `expected.txt`.
const TEMP: &str = "[TEMP]";

/// The expected output when stderr is a terminal, checked for the cases that have this file.
const TERMINAL_EXPECTED: &str = "expected.term.svg";

fn main() {
    tryfn::Harness::new(CASES, setup, test)
        .select(["args"])
        .test();
}

fn setup(args_path: PathBuf) -> tryfn::Case {
    let case_directory = args_path
        .parent()
        .expect("`args` is always inside a case directory");

    let expected = tryfn::Data::read_from(&case_directory.join("expected.txt"), None);

    tryfn::Case {
        name: case_name(case_directory),
        fixture: args_path,
        expected,
    }
}

fn case_name(case_directory: &Path) -> String {
    case_directory
        .strip_prefix(CASES)
        .expect("cases are found within the cases directory")
        .display()
        .to_string()
        // Keep case names, and so `--exact` filters, the same across platforms.
        .replace('\\', "/")
}

fn test(args_path: &Path) -> Result<String> {
    let case_directory = args_path
        .parent()
        .expect("`args` is always inside a case directory");

    let args: Vec<String> = std::fs::read_to_string(args_path)?
        .lines()
        .map(str::to_owned)
        .collect();

    // Hand-written exchanges can't be reproduced by a real instance, so always replay them.
    let mode = if case_directory.join("replay-only").exists() {
        proxy::Mode::Replay
    } else {
        proxy::Mode::from_env()?
    };

    let rendered = run(case_directory, &args, mode, Stderr::Pipe)?.to_string();

    let terminal_expected = case_directory.join(TERMINAL_EXPECTED);
    if terminal_expected.exists() {
        // Only how the output is shown differs, so replaying the exchanges just used, or recorded,
        // is enough.
        let terminal_run = run(case_directory, &args, proxy::Mode::Replay, Stderr::Terminal)?;
        snapbox::Assert::new()
            .action_env(snapbox::assert::DEFAULT_ACTION_ENV)
            .try_eq(
                Some(&format!(
                    "{} ({TERMINAL_EXPECTED})",
                    case_name(case_directory)
                )),
                snapbox::Data::text(terminal_svg::render(&terminal_run))
                    .is(snapbox::data::DataFormat::TermSvg),
                snapbox::Data::read_from(&terminal_expected, None),
            )
            .map_err(|error| error.to_string())?;
    }

    Ok(rendered)
}

/// Appends `output` with trailing whitespace stripped from each line.
///
/// `clap` indents the blank lines between paragraphs of multi-paragraph help, leaving trailing
/// whitespace that editors and formatters would otherwise strip from the expected files.
fn push_trimmed(rendered: &mut String, output: &[u8]) {
    for line in String::from_utf8_lossy(output).lines() {
        rendered.push_str(line.trim_end());
        rendered.push('\n');
    }
}
