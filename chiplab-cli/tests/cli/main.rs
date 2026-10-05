//! Fixture tests running the `chiplab-cli` binary.
//!
//! Each directory in `tests/cases/` containing an `args` file is one case, named by its path
//! relative to `tests/cases/`:
//!
//! - `args`: the command-line arguments, one per line.
//! - `expected.txt`: the rendered exit status, stdout and stderr.
//!
//! Run with `SNAPSHOTS=overwrite cargo test --test cli` to update the expected files.

use std::fmt::Write;
use std::path::{Path, PathBuf};

const CASES: &str = "tests/cases";

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

fn test(args_path: &Path) -> anyhow::Result<String> {
    let args: Vec<String> = std::fs::read_to_string(args_path)?
        .lines()
        .map(str::to_owned)
        .collect();

    let output = snapbox::cmd::Command::new(snapbox::cmd::cargo_bin!("chiplab-cli"))
        // A clear environment keeps a developer's `.env` from leaking into the results.
        .env_clear()
        .args(&args)
        .output()?;

    let mut rendered = String::new();
    writeln!(rendered, "$ chiplab-cli {}", args.join(" "))?;
    // `ExitStatus`'s `Display` is platform-specific, render the code explicitly instead.
    match output.status.code() {
        Some(code) => writeln!(rendered, "exit status: {code}")?,
        None => writeln!(rendered, "exit status: none ({})", output.status)?,
    }
    writeln!(rendered, "--- stdout")?;
    push_trimmed(&mut rendered, &output.stdout);
    writeln!(rendered, "--- stderr")?;
    push_trimmed(&mut rendered, &output.stderr);
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
