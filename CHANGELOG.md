# Changelog

<!--
Each section is published as the release notes on GitHub, which renders every newline as a line break.
Unlike other Markdown files, write each paragraph and list item on a single line, not one sentence per line.
-->

## Next

## v0.2.0

- **Shows progress while running.** `run-test-suite` reports each step as it happens. In a terminal, the current step is shown with a spinner or progress bar. In CI logs, each step is printed as a line once it's done.
- **Keeps downloading artifacts when one fails.** Each artifact is reported as soon as it's downloaded. A failed download no longer stops the remaining artifacts from being downloaded; all failures are reported before the run exits with an error.
- **Downloads the artifacts of runs that didn't complete.** When the run itself fails rather than the tests, the output captured up to the failure is downloaded too, which is where to look for the cause.
- **Is MIT licensed.** The license is included in the release archives.

## v0.1.0

This is the first release of `chiplab-cli`, a command-line client for [Chiplab](https://chiplab.veecle.ai). It lets you run Chiplab firmware tests from your CI pipeline, the same way your agent runs them during development.

### What it does

- **Runs test suites in CI.** It runs your TypeScript test suite against a firmware build on a simulated board and reports whether the tests passed.
- **Fits into pipelines.** Each outcome (passed, test failures, or a run that didn't complete) has its own exit code. There's optional JSON output for other tools, and the run's artifacts can be downloaded for your CI to keep.
- **Runs almost anywhere.** It's a single prebuilt binary for Linux (x86_64 and aarch64) and macOS (Apple Silicon). You can also build it with `cargo` or run it with Nix.

### Getting started

Ask your agent to add Chiplab to your CI. Through the Chiplab MCP server, it can find out how to install the CLI, write the pipeline step, and set up the access token.
