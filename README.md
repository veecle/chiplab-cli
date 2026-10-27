# chiplab-cli

Command-line client for [Chiplab](https://chiplab.veecle.ai).

Chiplab by Veecle is an MCP platform that lets AI agents compile, simulate, and validate firmware on the actual target chip without physical hardware.
`chiplab-cli` lets you run Chiplab firmware tests from your CI pipeline, the same way your agent runs them during development.

## What it does

- **Runs test suites in CI.**
  It runs your TypeScript test suite against a firmware build on a simulated board and reports whether the tests passed.
- **Fits into pipelines.**
  Each outcome (passed, test failures, or a run that didn't complete) has its own exit code.
  There's optional JSON output for other tools, and the run's artifacts can be downloaded for your CI to keep.
- **Runs almost anywhere.**
  It's a single prebuilt binary for Linux (x86_64 and aarch64) and macOS (Apple Silicon).
  You can also build it with `cargo` or run it with Nix.

## Getting started

Ask your agent to add Chiplab to your CI.
Through the Chiplab MCP server, it can find out how to install the CLI, write the pipeline step, and set up the access token.

To explore the CLI yourself, run `chiplab-cli --help`.

## Installation

Each [release](https://github.com/veecle/chiplab-cli/releases) provides prebuilt binaries for Linux (x86_64 and aarch64, statically linked) and macOS (Apple Silicon):

- `chiplab-cli-x86_64-unknown-linux-musl.tar.gz`
- `chiplab-cli-aarch64-unknown-linux-musl.tar.gz`
- `chiplab-cli-aarch64-apple-darwin.tar.gz`

```sh
curl -fsSL https://github.com/veecle/chiplab-cli/releases/latest/download/chiplab-cli-x86_64-unknown-linux-musl.tar.gz | tar -xz
./chiplab-cli --help
```

Replace `latest/download` with `download/<tag>` (e.g. `download/v0.1.0`) to pin a version.
To verify the download, fetch the release's `sha256sums.txt` next to it and run `sha256sum --check --ignore-missing sha256sums.txt`.

The macOS binary is not signed.
When it is downloaded through a browser, macOS blocks it; remove the quarantine attribute with `xattr -d com.apple.quarantine chiplab-cli`.

It can also be built from source with `cargo`:

```sh
cargo install --locked --git https://github.com/veecle/chiplab-cli --tag v0.1.0 chiplab-cli
```

Or run with Nix:

```sh
nix run github:veecle/chiplab-cli/v0.1.0 -- --help
```

## Development

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for setting up a development environment and other contribution guidelines.

## License

This project is licensed under the [MIT License](LICENSE).
