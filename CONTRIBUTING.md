# Contributing to chiplab-cli

## Development environment

The primary supported development environment is the included nix devshell.

[`nix-direnv`](https://github.com/nix-community/nix-direnv) is recommended for easy setup.
After running `direnv allow` once, the devshell activates automatically when you `cd` into the repository, and `.env` is read for any local customization needed.
Otherwise you can manually enter the devshell and source `.env` each time you enter the repo.

```sh
nix develop
source .env
cargo run -- --help
```

The required dependencies should be minimal, and the toolchain itself is pinned via `rust-toolchain.toml`, so a standard dev environment with a plain `rustup` install should also be usable.
Some additional dev tools (e.g. formatters and linters used by the checks) may be needed; see the devshell and checks in [`flake.nix`](flake.nix) and [`nix/`](nix) for details.

If you use [`jj`](https://jj-vcs.github.io/jj/), `nix run .#setup-jj-fix` configures `jj fix` in the repository with the same formatters the checks enforce.

## Testing

PRs that change behaviour should include tests covering that change.
Bug fixes should include a regression test, ideally one that fails without the fix.

The main test approach is fixture tests in `chiplab-cli/tests/cases/`, which run the real binary and snapshot its output.
Each case is a directory, optionally nested in subdirectories to group related cases, containing:

- `args`: the command-line arguments, one per line.
  `[TEMP]` is replaced with the path of a fresh temporary directory, for cases that write files.
- `temp/`: files copied into the temporary directory before running, if present.
- `expected.txt`: the expected exit status, stdout and stderr, followed by the files in the temporary directory.
  The proxy's URL is shown as `[CHIPLAB_URL]`.
- `exchanges.json`: the API exchanges the case makes, replayed by a local proxy standing in for Chiplab.
  Cases making no API requests have none.
- `no-proxy`: an empty marker file; if present, the binary runs without the proxy and without any Chiplab configuration in its environment, for testing how the CLI handles that.
- `replay-only`: a marker file whose contents explain why the case's `exchanges.json` is hand-written; if present, the exchanges are always replayed, whatever the proxy mode.
  Use this for scenarios that can't be reproduced against a real Chiplab instance.

The binary runs within `chiplab-cli/tests/fixtures/`, so `args` can refer to the input files shared between cases stored there.

To add a case, create a directory with an `args` file, then generate its `expected.txt` by re-snapshotting.
When the output changes intentionally, re-snapshot and review the diff before committing:

```sh
SNAPSHOTS=overwrite cargo test --test cli
```

In `expected.txt`, `[..]` matches any text within a line and `...` on a line by itself matches any number of lines.
See [`snapbox::Assert`](https://docs.rs/snapbox/latest/snapbox/assert/struct.Assert.html) for the full wildcard details.

### Recording API exchanges

By default the proxy replays `exchanges.json`, failing the case on any request that doesn't match a remaining recorded one, and on any recorded exchange that isn't requested.
Requests may come in any order, as the CLI makes some concurrently; identical requests are answered in their recorded order.
To record the exchanges against a real Chiplab instance instead, run with `TEST_CHIPLAB_PROXY=record`, configured by `CHIPLAB_URL` and `CHIPLAB_TOKEN` as for the CLI itself.
Recording overwrites the exchanges of every case run, so limit it to the case being recorded to avoid re-recording unrelated cases:

```sh
TEST_CHIPLAB_PROXY=record cargo test --test cli -- --exact <case name>
```

To run the cases against a real Chiplab instance without replaying or recording the exchanges, run with `TEST_CHIPLAB_PROXY=bypass`, configured as for recording.

Cases marked `replay-only` are replayed in every mode, so their exchanges are never overwritten and must be kept up to date by hand.

TypeIDs are replaced with fake ones in recordings, and temporary directories that Chiplab creates on its side (such as `./tmp/foo-QHpLw0` in test suite artifacts) with `[CHIPLAB_TEMP_DIR]`.
Cases refer to the project to run in as `project_00000000000000000000000001`; set the real project to record or bypass in with `TEST_CHIPLAB_PROJECT_ID`.

Request headers are not recorded, so no credentials end up in the recordings.
Absolute URLs to hosts other than `CHIPLAB_URL` in responses are rewritten to point at the proxy, so requests to them (such as uploads) are recorded and replayed too.

## Generated API client

The `chiplab-api/` crate is generated from the OpenAPI spec in `chiplab-api.json`, using the templates in `openapi-templates/`.
Neither `chiplab-api/` nor `chiplab-api.json` should be edited by hand:

- `chiplab-api.json` is the source of truth for the API and is exported from the Chiplab service.
- `chiplab-api/` must match a fresh generation; the `generated-chiplab-api` check rejects any difference.

If a change needs something from the API that is not available in the generated client, raise it in an issue instead.

To update the spec, download the current version from the Chiplab service:

```sh
curl -fsSL https://chiplab.veecle.ai/api/stable/openapi.json | jq . > chiplab-api.json
```

After changing `chiplab-api.json` or `openapi-templates/`, regenerate the crate:

```sh
nix run .#update-chiplab-api
```

`chiplab.d.ts` types the test suite fixtures and, like `chiplab-api.json`, comes from the Chiplab service.
To update it:

```sh
nix run .#update-chiplab-d-ts
```

## Releasing

Releases are published by the [`release`](.github/workflows/release.yaml) workflow when a `v<version>` tag is pushed.
It builds the binaries, attaches them to a GitHub release, and generates the release notes from the merged PRs.

1. Bump `version` in the root `Cargo.toml` following [semver](https://semver.org), run `cargo update --workspace` to update `Cargo.lock`, and merge the PR.
2. Tag the merged commit and push the tag:

   ```sh
   git tag -a v0.2.0 -m "Release version v0.2.0" <commit>
   git push origin v0.2.0
   ```

If the workflow fails before publishing (e.g. a flaky runner), use "Re-run failed jobs" on the workflow run.
