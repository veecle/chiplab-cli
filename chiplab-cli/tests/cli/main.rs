//! Fixture tests running the `chiplab-cli` binary.
//!
//! Each directory in `tests/cases/` containing an `args` file is one case, named by its path
//! relative to `tests/cases/`.
//!
//! The case layout and the workflows are documented in the "Testing" section of `CONTRIBUTING.md`;
//! keep it up to date when changing them.

use std::fmt::Write;
use std::ops::Not;
use std::path::{Path, PathBuf};

mod proxy;

const CASES: &str = "tests/cases";
const FIXTURES: &str = "tests/fixtures";

/// Stands in for the path of the case's temporary directory in `args` and `expected.txt`.
const TEMP: &str = "[TEMP]";

/// Stands in for the proxy's URL in `expected.txt`.
const CHIPLAB_URL: &str = "[CHIPLAB_URL]";

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
    let case_directory = args_path
        .parent()
        .expect("`args` is always inside a case directory");

    let temp = tempfile::tempdir()?;
    let temp_path = temp.path().to_str().expect("temporary paths are UTF-8");
    let seed = case_directory.join("temp");
    if seed.exists() {
        copy_files(&seed, temp.path())?;
    }

    let args: Vec<String> = std::fs::read_to_string(args_path)?
        .lines()
        .map(str::to_owned)
        .collect();

    let case_directory = args_path
        .parent()
        .expect("`args` is always inside a case directory");

    let proxy = case_directory
        .join("no-proxy")
        .exists()
        .not()
        .then(|| {
            proxy::Proxy::start(
                proxy::Mode::from_env()?,
                &case_directory.join("exchanges.json"),
            )
        })
        .transpose()?;

    let forwarded_environment = [
        // HTTP client requires CA certs, nix-sandbox (and potentially other systems) locate
        // them via these environment variables.
        "SSL_CERT_FILE",
        "SSL_CERT_DIR",
    ]
    .into_iter()
    .filter_map(|key| std::env::var_os(key).map(|value| (key, value)));

    let mut command = snapbox::cmd::Command::new(snapbox::cmd::cargo_bin!("chiplab-cli"))
        // A clear environment keeps a developer's `.env` from leaking into the results.
        .env_clear()
        .envs(forwarded_environment)
        .current_dir(FIXTURES)
        .args(args.iter().map(|arg| arg.replace(TEMP, temp_path)));

    if let Some(proxy) = &proxy {
        command = command
            .env("CHIPLAB_URL", proxy.base_url())
            // The proxy authenticates to the real instance itself, so the CLI gets a dummy token.
            .env("CHIPLAB_TOKEN", "veecle_pat_test");
    }

    let output = command.output()?;

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
    push_files(&mut rendered, temp.path(), temp.path())?;
    let mut rendered = rendered.replace(temp_path, TEMP);
    if let Some(proxy) = &proxy {
        // The proxy's port differs between runs.
        rendered = rendered.replace(proxy.base_url(), CHIPLAB_URL);
    }

    if let Some(proxy) = proxy
        && let Err(error) = proxy.finish()
    {
        anyhow::bail!("{error:#}\n\n{rendered}");
    }

    Ok(rendered)
}

/// Copies every directory and file under `from` into `to`.
fn copy_files(from: &Path, to: &Path) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            std::fs::create_dir(&target)?;
            copy_files(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }

    Ok(())
}

/// Appends every directory and file under `directory` with its contents, sorted by path.
fn push_files(rendered: &mut String, root: &Path, directory: &Path) -> anyhow::Result<()> {
    let mut entries = std::fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        let relative = path
            .strip_prefix(root)?
            .display()
            .to_string()
            .replace('\\', "/");
        if entry.file_type()?.is_dir() {
            writeln!(rendered, "--- {TEMP}/{relative}/")?;
            push_files(rendered, root, &path)?;
        } else {
            writeln!(rendered, "--- {TEMP}/{relative}")?;
            push_trimmed(rendered, &std::fs::read(&path)?);
        }
    }

    Ok(())
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
