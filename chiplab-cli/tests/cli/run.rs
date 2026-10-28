//! Runs the binary for a case and records what it did.

use std::fmt;
use std::ops::Not;
use std::path::Path;
use std::process::ExitStatus;

use crate::error::{Chain, Result};
use crate::{TEMP, proxy, push_trimmed};

const FIXTURES: &str = "tests/fixtures";

/// Stands in for the proxy's URL in `expected.txt`.
const CHIPLAB_URL: &str = "[CHIPLAB_URL]";

/// What a run of the binary did.
#[derive(Debug, Clone)]
pub struct Run {
    /// The command line the binary was run with.
    invocation: String,

    /// The exit status the binary finished with.
    exit_status: ExitStatus,

    /// What the binary wrote to stdout, as whole lines each ending in a newline.
    stdout: String,

    /// What the binary wrote to stderr, as whole lines each ending in a newline.
    stderr: String,

    /// The directories and files left in the temporary directory, sorted by path.
    files: Vec<Entry>,
}

/// A directory or file left in the temporary directory.
#[derive(Debug, Clone)]
pub enum Entry {
    /// A directory, at `path` relative to the temporary directory.
    Directory { path: String },

    /// A file, at `path` relative to the temporary directory, with `contents` as whole lines each
    /// ending in a newline.
    File { path: String, contents: String },
}

/// Renders the entry as text, its contents following a `--- <path>` line.
impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Directory { path } => writeln!(f, "--- {TEMP}/{path}/"),
            Self::File { path, contents } => write!(f, "--- {TEMP}/{path}\n{contents}"),
        }
    }
}

impl Entry {
    /// Replaces every occurrence of `from` with `to`.
    fn replace(&mut self, from: &str, to: &str) {
        match self {
            Self::Directory { path } => *path = path.replace(from, to),
            Self::File { path, contents } => {
                *path = path.replace(from, to);
                *contents = contents.replace(from, to);
            }
        }
    }
}

/// Renders the run as text, each output following a `--- <name>` line.
impl fmt::Display for Run {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            invocation,
            exit_status,
            stdout,
            stderr,
            files,
        } = self;

        writeln!(f, "$ {invocation}")?;
        // `ExitStatus`'s `Display` is platform-specific, render the code explicitly instead.
        match exit_status.code() {
            Some(code) => writeln!(f, "exit status: {code}")?,
            None => writeln!(f, "exit status: none ({exit_status})")?,
        }
        write!(f, "--- stdout\n{stdout}--- stderr\n{stderr}")?;
        for entry in files {
            write!(f, "{entry}")?;
        }
        Ok(())
    }
}

impl Run {
    /// Replaces every occurrence of `from` with `to`.
    fn replace(&mut self, from: &str, to: &str) {
        let Self {
            invocation,
            exit_status: _,
            stdout,
            stderr,
            files,
        } = self;

        for text in [invocation, stdout, stderr] {
            *text = text.replace(from, to);
        }
        for entry in files {
            entry.replace(from, to);
        }
    }
}

/// Runs the binary with `args` for the case in `case_directory`, recording what it did.
pub fn run(case_directory: &Path, args: &[String]) -> Result<Run> {
    let temp = tempfile::tempdir()?;
    let temp_path = temp.path().to_str().expect("temporary paths are UTF-8");
    let seed = case_directory.join("temp");
    if seed.exists() {
        copy_files(&seed, temp.path())?;
    }

    let proxy = case_directory
        .join("no-proxy")
        .exists()
        .not()
        .then(|| {
            // Hand-written exchanges can't be reproduced by a real instance, so always replay them.
            let mode = if case_directory.join("replay-only").exists() {
                proxy::Mode::Replay
            } else {
                proxy::Mode::from_env()?
            };
            proxy::Proxy::start(mode, &case_directory.join("exchanges.json"))
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

    let invocation = std::iter::once("chiplab-cli")
        .chain(args.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(" ");

    let mut stdout = String::new();
    push_trimmed(&mut stdout, &output.stdout);

    let mut stderr = String::new();
    push_trimmed(&mut stderr, &output.stderr);

    let mut files = Vec::new();
    collect_files(&mut files, temp.path(), temp.path())?;

    let mut run = Run {
        invocation,
        exit_status: output.status,
        stdout,
        stderr,
        files,
    };
    run.replace(temp_path, TEMP);
    if let Some(proxy) = &proxy {
        // The proxy's port differs between runs.
        run.replace(proxy.base_url(), CHIPLAB_URL);
    }

    if let Some(proxy) = proxy
        && let Err(error) = proxy.finish()
    {
        return Err(format!("{}\n\n{}", Chain(&*error), run).into());
    }

    Ok(run)
}

/// Copies every directory and file under `from` into `to`.
///
/// The copies are created with default permissions rather than the source's, so they stay
/// writable when `from` is read-only (e.g. in the Nix store).
fn copy_files(from: &Path, to: &Path) -> Result<()> {
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            std::fs::create_dir(&target)?;
            copy_files(&entry.path(), &target)?;
        } else {
            std::fs::write(target, std::fs::read(entry.path())?)?;
        }
    }

    Ok(())
}

/// Collects every directory and file under `directory` with its contents, sorted by path.
fn collect_files(files: &mut Vec<Entry>, root: &Path, directory: &Path) -> Result<()> {
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
            files.push(Entry::Directory { path: relative });
            collect_files(files, root, &path)?;
        } else {
            let mut contents = String::new();
            push_trimmed(&mut contents, &std::fs::read(&path)?);
            files.push(Entry::File {
                path: relative,
                contents,
            });
        }
    }

    Ok(())
}
